//! Achievement system core data model and pure logic
//! (`src/module/achievements/achievement.c`,
//! `src/module/achievements/achievement.h` in the legacy C server).
//!
//! This module ports the *data model and stat-driven award logic* of the
//! legacy achievement system as a leaf module with no access to
//! `World`/`PlayerRuntime`/networking, matching the pattern used by
//! `crate::quest` before it was wired into live call sites. What is ported
//! here: the full 127-entry `AchievementType` enum and `achievement_defs`
//! table (`achievement.c:44-326`, copied digit for digit including Steam
//! API id strings, display names/descriptions, categories and progress
//! targets), the `Achievement`/`AccountAchievements`/`AchievementStats`
//! storage structs (`achievement.h:217-276`), and every stat-update /
//! award-check function (`achievement_add_flowers` .. `achievement_add_
//! play_time`, `achievement_check_login_streak`, `achievement_check_level`,
//! `achievement_check_skill`, `achievement_check_profession`,
//! `achievement_check_exploration`, `achievement_get_stat_progress`,
//! `achievement_area_to_pent_index`, `achievement_clear_all`) as pure
//! functions returning the list of newly-unlocked achievements for the
//! caller to route through logging/Steam-sync/DB "first unlock" side
//! effects it can't reach from here.
//!
//! NOT ported yet (left for the caller-side wiring task, tracked as
//! REMAINING on the "Achievements" P3 `PORTING_TODO.md` entry):
//! `achievement_send_to_client`/`achievement_sync_all` (the `SV_ACH_*`
//! mod-packet wire format from `mod_achievements.h` - no Rust protocol
//! definitions exist for it yet), `db_achievement_record_unlock`/the
//! "first player globally" DB tracking and cross-server `server_chat`
//! announcement (`database_achievement.c`), `achievement_list`/
//! `achievement_show_stats`/`achievement_fix_all` (text-formatting
//! functions that belong in `ugaris-server`'s command layer once the
//! `/achievements`/`/achstats`/`/achfix`/`/achclear`/`/achsync`/`/achgive`
//! commands - currently help-text-only stubs in `commands_player.rs` - are
//! wired up), and persistence (no PPD/DB column exists yet for
//! `AccountAchievements`/`AchievementStats`; `crate::player`'s existing
//! `AchievementState` is a small pre-existing ad hoc subset - chests +
//! transport exploration markers only - and is left untouched by this
//! change to avoid an unrelated refactor). No call site anywhere in the
//! Rust tree constructs or mutates `AccountAchievements`/`AchievementStats`
//! yet; this module is data-and-logic-only until that wiring lands.

/// C `#define MAX_ACHIEVEMENTS 128` (`achievement.h:18`).
pub const MAX_ACHIEVEMENTS: usize = 128;

/// C `ACHIEVEMENT_TYPE_COUNT` (`achievement.h:211`): 127 real achievement
/// types precede the trailing count marker in the C enum.
pub const ACHIEVEMENT_TYPE_COUNT: usize = 127;

/// C `PentArea` (`achievement.h:215`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PentArea {
    Earth = 0,
    Fire = 1,
    Ice = 2,
    Hell = 3,
}

/// C `PENT_AREA_COUNT`.
pub const PENT_AREA_COUNT: usize = 4;

/// C `ServerAchCat` (`achievement.h:23-34`); numeric values must match the
/// protocol's `AchievementCategory` (`mod_achievements.h:52-62`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AchCategory {
    Progression = 0,
    Combat = 1,
    Exploration = 2,
    Quests = 3,
    Crafting = 4,
    Social = 5,
    Pentagram = 6,
    Collection = 7,
    Special = 8,
}

/// C `AchievementType` (`achievement.h:38-212`), copied in exact
/// declaration order so `as usize` matches the C enum's numeric value used
/// to index `achievement_defs`/`AccountAchievements::achievements`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AchievementType {
    StartedUgaris = 0,
    RisingBeginner = 1,
    ExperiencedHero = 2,
    UgarisVeteran = 3,
    LegendaryAdventurer = 4,
    DemonSlayer = 5,
    MasterOfHell = 6,
    MasterOfUgaris = 7,
    FirstBlood = 8,
    Ladykiller = 9,
    ArenaCombatant = 10,
    SlayerOfWizards = 11,
    Dragonsbane = 12,
    DemonLordsDemise = 13,
    SlayerOfDemonLords = 14,
    FiendFighter = 15,
    Demonbane = 16,
    DreadDestroyer = 17,
    DemonicExterminator = 18,
    UgarisPathfinder = 19,
    TravellerOfAstonia = 20,
    UndergroundExplorer = 21,
    ExplorerOfAstonia = 22,
    GreatExplorer = 23,
    AHelpingHand = 24,
    Quester = 25,
    GreenThumb = 26,
    BotanyEnthusiast = 27,
    NaturesFriend = 28,
    Herbalist = 29,
    MasterHerbalist = 30,
    MushroomHunter = 31,
    FungusFinder = 32,
    SporeSeeker = 33,
    MushroomMaster = 34,
    Mycologist = 35,
    BerryPicker = 36,
    FruitForager = 37,
    BerryEnthusiast = 38,
    HarvestHero = 39,
    MasterGatherer = 40,
    Alchemist = 41,
    JourneymanBrewer = 42,
    ArcaneAlchemist = 43,
    GrandmasterBrewer = 44,
    GoldLooter = 45,
    WellPaidGatherer = 46,
    Solved = 47,
    FullOfSolves = 48,
    RuneMaster = 49,
    GrandmasterPentagram = 50,
    EarthboundNovice = 51,
    FlameInitiate = 52,
    FightingTheFrost = 53,
    ThroughGatesOfHell = 54,
    HappyGoLucky = 55,
    FavoredByFortune = 56,
    FiveInARow = 57,
    TreasureHunter = 58,
    EarthRocks = 59,
    FireRocks = 60,
    IceRocks = 61,
    WeaponNovice = 62,
    MasterOfArms = 63,
    ApprenticeMagic = 64,
    IntermediateMagic = 65,
    MasterOfMagic = 66,
    ApprenticeFighting = 67,
    IntermediateFighting = 68,
    MasterOfFighting = 69,
    ClanMember = 70,
    ClanMaster = 71,
    ClubMember = 72,
    ClubMaster = 73,
    TrustButVerify = 74,
    HardcoreHero = 75,
    HardcoreLegend = 76,
    MilitaryVeteran = 77,
    TunnelRat = 78,
    MasterTrader = 79,
    MasterAlchemist = 80,
    MasterHerbalistProf = 81,
    MasterMiner = 82,
    MasterAthlete = 83,
    MasterAssassin = 84,
    MasterThief = 85,
    MasterLightWarrior = 86,
    MasterDarkWarrior = 87,
    MasterMercenary = 88,
    MasterClanWarrior = 89,
    SilverNovice = 90,
    SilverCollector = 91,
    SilverHoarder = 92,
    SilverBaron = 93,
    SilverTycoon = 94,
    SilverMagnate = 95,
    SilverLegend = 96,
    GoldNovice = 97,
    GoldCollector = 98,
    GoldHoarder = 99,
    GoldBaron = 100,
    GoldTycoon = 101,
    GoldMagnate = 102,
    GoldLegend = 103,
    CoinCollector = 104,
    WealthyAdventurer = 105,
    RichNoble = 106,
    Millionaire = 107,
    Recruit = 108,
    Soldier = 109,
    Commander = 110,
    General = 111,
    WarLegend = 112,
    TunnelExplorer = 113,
    TunnelRunner = 114,
    TunnelVeteran = 115,
    Looter = 116,
    TreasureMaster = 117,
    LegendaryLooter = 118,
    PotionMaster = 119,
    LegendaryBrewer = 120,
    DedicatedPlayer = 121,
    VeteranPlayer = 122,
    UgarisLifer = 123,
    Regular = 124,
    Committed = 125,
    Devoted = 126,
}

impl AchievementType {
    /// All 127 achievement types in C declaration/index order.
    pub const ALL: [AchievementType; ACHIEVEMENT_TYPE_COUNT] = [
        AchievementType::StartedUgaris,
        AchievementType::RisingBeginner,
        AchievementType::ExperiencedHero,
        AchievementType::UgarisVeteran,
        AchievementType::LegendaryAdventurer,
        AchievementType::DemonSlayer,
        AchievementType::MasterOfHell,
        AchievementType::MasterOfUgaris,
        AchievementType::FirstBlood,
        AchievementType::Ladykiller,
        AchievementType::ArenaCombatant,
        AchievementType::SlayerOfWizards,
        AchievementType::Dragonsbane,
        AchievementType::DemonLordsDemise,
        AchievementType::SlayerOfDemonLords,
        AchievementType::FiendFighter,
        AchievementType::Demonbane,
        AchievementType::DreadDestroyer,
        AchievementType::DemonicExterminator,
        AchievementType::UgarisPathfinder,
        AchievementType::TravellerOfAstonia,
        AchievementType::UndergroundExplorer,
        AchievementType::ExplorerOfAstonia,
        AchievementType::GreatExplorer,
        AchievementType::AHelpingHand,
        AchievementType::Quester,
        AchievementType::GreenThumb,
        AchievementType::BotanyEnthusiast,
        AchievementType::NaturesFriend,
        AchievementType::Herbalist,
        AchievementType::MasterHerbalist,
        AchievementType::MushroomHunter,
        AchievementType::FungusFinder,
        AchievementType::SporeSeeker,
        AchievementType::MushroomMaster,
        AchievementType::Mycologist,
        AchievementType::BerryPicker,
        AchievementType::FruitForager,
        AchievementType::BerryEnthusiast,
        AchievementType::HarvestHero,
        AchievementType::MasterGatherer,
        AchievementType::Alchemist,
        AchievementType::JourneymanBrewer,
        AchievementType::ArcaneAlchemist,
        AchievementType::GrandmasterBrewer,
        AchievementType::GoldLooter,
        AchievementType::WellPaidGatherer,
        AchievementType::Solved,
        AchievementType::FullOfSolves,
        AchievementType::RuneMaster,
        AchievementType::GrandmasterPentagram,
        AchievementType::EarthboundNovice,
        AchievementType::FlameInitiate,
        AchievementType::FightingTheFrost,
        AchievementType::ThroughGatesOfHell,
        AchievementType::HappyGoLucky,
        AchievementType::FavoredByFortune,
        AchievementType::FiveInARow,
        AchievementType::TreasureHunter,
        AchievementType::EarthRocks,
        AchievementType::FireRocks,
        AchievementType::IceRocks,
        AchievementType::WeaponNovice,
        AchievementType::MasterOfArms,
        AchievementType::ApprenticeMagic,
        AchievementType::IntermediateMagic,
        AchievementType::MasterOfMagic,
        AchievementType::ApprenticeFighting,
        AchievementType::IntermediateFighting,
        AchievementType::MasterOfFighting,
        AchievementType::ClanMember,
        AchievementType::ClanMaster,
        AchievementType::ClubMember,
        AchievementType::ClubMaster,
        AchievementType::TrustButVerify,
        AchievementType::HardcoreHero,
        AchievementType::HardcoreLegend,
        AchievementType::MilitaryVeteran,
        AchievementType::TunnelRat,
        AchievementType::MasterTrader,
        AchievementType::MasterAlchemist,
        AchievementType::MasterHerbalistProf,
        AchievementType::MasterMiner,
        AchievementType::MasterAthlete,
        AchievementType::MasterAssassin,
        AchievementType::MasterThief,
        AchievementType::MasterLightWarrior,
        AchievementType::MasterDarkWarrior,
        AchievementType::MasterMercenary,
        AchievementType::MasterClanWarrior,
        AchievementType::SilverNovice,
        AchievementType::SilverCollector,
        AchievementType::SilverHoarder,
        AchievementType::SilverBaron,
        AchievementType::SilverTycoon,
        AchievementType::SilverMagnate,
        AchievementType::SilverLegend,
        AchievementType::GoldNovice,
        AchievementType::GoldCollector,
        AchievementType::GoldHoarder,
        AchievementType::GoldBaron,
        AchievementType::GoldTycoon,
        AchievementType::GoldMagnate,
        AchievementType::GoldLegend,
        AchievementType::CoinCollector,
        AchievementType::WealthyAdventurer,
        AchievementType::RichNoble,
        AchievementType::Millionaire,
        AchievementType::Recruit,
        AchievementType::Soldier,
        AchievementType::Commander,
        AchievementType::General,
        AchievementType::WarLegend,
        AchievementType::TunnelExplorer,
        AchievementType::TunnelRunner,
        AchievementType::TunnelVeteran,
        AchievementType::Looter,
        AchievementType::TreasureMaster,
        AchievementType::LegendaryLooter,
        AchievementType::PotionMaster,
        AchievementType::LegendaryBrewer,
        AchievementType::DedicatedPlayer,
        AchievementType::VeteranPlayer,
        AchievementType::UgarisLifer,
        AchievementType::Regular,
        AchievementType::Committed,
        AchievementType::Devoted,
    ];
}

/// C `AchievementDef` (`achievement.h:279-286`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AchievementDef {
    pub ty: AchievementType,
    /// Steam API achievement id (`STEAM_ACH_*` in `mod_achievements.h`).
    pub steam_id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: AchCategory,
    /// Target for progress-based achievements (`0` = instant/one-time).
    pub target: u32,
    pub hidden: bool,
}

/// C `static const AchievementDef achievement_defs[ACHIEVEMENT_TYPE_COUNT]`
/// (`achievement.c:44-326`), copied digit for digit (Steam ids, display
/// names, descriptions, categories, targets - all C table entries have
/// `hidden = 0`, so none are hidden achievements today).
pub const ACHIEVEMENT_DEFS: [AchievementDef; ACHIEVEMENT_TYPE_COUNT] = [
    AchievementDef {
        ty: AchievementType::StartedUgaris,
        steam_id: "STARTED_UGARIS",
        name: "Started Ugaris",
        description: "Launch Ugaris through Steam",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::RisingBeginner,
        steam_id: "RISING_BEGINNER",
        name: "Rising Beginner",
        description: "Reach level 10",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ExperiencedHero,
        steam_id: "EXPERIENCED_HERO",
        name: "Experienced Hero",
        description: "Reach level 20",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::UgarisVeteran,
        steam_id: "UGARIS_VETERAN",
        name: "Ugaris Veteran",
        description: "Reach level 50",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::LegendaryAdventurer,
        steam_id: "LEGENDARY_ADVENTURER",
        name: "Legendary Adventurer",
        description: "Reach level 75",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::DemonSlayer,
        steam_id: "DEMON_SLAYER",
        name: "Demon Slayer",
        description: "Reach level 100",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterOfHell,
        steam_id: "MASTER_OF_HELL",
        name: "Master of Hell",
        description: "Reach level 150",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterOfUgaris,
        steam_id: "MASTER_OF_UGARIS",
        name: "Master of Ugaris",
        description: "Reach level 200",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FirstBlood,
        steam_id: "FIRST_BLOOD",
        name: "First Blood",
        description: "Defeat your first enemy",
        category: AchCategory::Combat,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Ladykiller,
        steam_id: "LADYKILLER",
        name: "Ladykiller",
        description: "Defeat Islena",
        category: AchCategory::Combat,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ArenaCombatant,
        steam_id: "ARENA_COMBATANT",
        name: "Arena Combatant",
        description: "Defeat another player in PvP",
        category: AchCategory::Combat,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SlayerOfWizards,
        steam_id: "SLAYER_OF_WIZARDS",
        name: "Slayer of Wizards",
        description: "Kill Yendor",
        category: AchCategory::Combat,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Dragonsbane,
        steam_id: "DRAGONSBANE",
        name: "Dragonsbane",
        description: "Complete Carlos' Dragons quest",
        category: AchCategory::Combat,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::DemonLordsDemise,
        steam_id: "DEMON_LORDS_DEMISE",
        name: "Demon Lord's Demise",
        description: "Kill a demon lord",
        category: AchCategory::Combat,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SlayerOfDemonLords,
        steam_id: "SLAYER_OF_DEMON_LORDS",
        name: "Slayer of Demon Lords",
        description: "Kill 20 unique demon lords",
        category: AchCategory::Combat,
        target: 20,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FiendFighter,
        steam_id: "FIEND_FIGHTER",
        name: "Fiend Fighter",
        description: "Defeat 100 demons",
        category: AchCategory::Combat,
        target: 100,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Demonbane,
        steam_id: "DEMONBANE",
        name: "Demonbane",
        description: "Defeat 2,500 demons",
        category: AchCategory::Combat,
        target: 2500,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::DreadDestroyer,
        steam_id: "DREAD_DESTROYER",
        name: "Dread Destroyer",
        description: "Defeat 15,000 demons",
        category: AchCategory::Combat,
        target: 15000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::DemonicExterminator,
        steam_id: "DEMONIC_EXTERMINATOR",
        name: "Demonic Exterminator",
        description: "Defeat 250,000 demons",
        category: AchCategory::Combat,
        target: 250000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::UgarisPathfinder,
        steam_id: "UGARIS_PATHFINDER",
        name: "Ugaris Pathfinder",
        description: "Discover and travel to Aston",
        category: AchCategory::Exploration,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::TravellerOfAstonia,
        steam_id: "TRAVELLER_OF_ASTONIA",
        name: "Traveller of Astonia",
        description: "Visit all major cities",
        category: AchCategory::Exploration,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::UndergroundExplorer,
        steam_id: "UNDERGROUND_EXPLORER",
        name: "Underground Explorer",
        description: "Activate all Earth Underground teleports",
        category: AchCategory::Exploration,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ExplorerOfAstonia,
        steam_id: "EXPLORER_OF_ASTONIA",
        name: "Explorer of Astonia",
        description: "Activate all Rodney's map teleports",
        category: AchCategory::Exploration,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GreatExplorer,
        steam_id: "GREAT_EXPLORER",
        name: "Great Explorer",
        description: "Complete Finding Arkhata quest",
        category: AchCategory::Exploration,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::AHelpingHand,
        steam_id: "A_HELPING_HAND",
        name: "A Helping Hand",
        description: "Complete Lydia's potion quest",
        category: AchCategory::Quests,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Quester,
        steam_id: "QUESTER",
        name: "Quester",
        description: "Use the repeat option on any quest",
        category: AchCategory::Quests,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GreenThumb,
        steam_id: "GREEN_THUMB",
        name: "Green Thumb",
        description: "Gather 10 flowers",
        category: AchCategory::Crafting,
        target: 10,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::BotanyEnthusiast,
        steam_id: "BOTANY_ENTHUSIAST",
        name: "Botany Enthusiast",
        description: "Gather 50 flowers",
        category: AchCategory::Crafting,
        target: 50,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::NaturesFriend,
        steam_id: "NATURES_FRIEND",
        name: "Nature's Friend",
        description: "Gather 200 flowers",
        category: AchCategory::Crafting,
        target: 200,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Herbalist,
        steam_id: "HERBALIST",
        name: "Herbalist",
        description: "Gather 500 flowers",
        category: AchCategory::Crafting,
        target: 500,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterHerbalist,
        steam_id: "MASTER_HERBALIST",
        name: "Master Herbalist",
        description: "Gather 1000 flowers",
        category: AchCategory::Crafting,
        target: 1000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MushroomHunter,
        steam_id: "MUSHROOM_HUNTER",
        name: "Mushroom Hunter",
        description: "Gather 10 mushrooms",
        category: AchCategory::Crafting,
        target: 10,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FungusFinder,
        steam_id: "FUNGUS_FINDER",
        name: "Fungus Finder",
        description: "Gather 50 mushrooms",
        category: AchCategory::Crafting,
        target: 50,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SporeSeeker,
        steam_id: "SPORE_SEEKER",
        name: "Spore Seeker",
        description: "Gather 200 mushrooms",
        category: AchCategory::Crafting,
        target: 200,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MushroomMaster,
        steam_id: "MUSHROOM_MASTER",
        name: "Mushroom Master",
        description: "Gather 500 mushrooms",
        category: AchCategory::Crafting,
        target: 500,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Mycologist,
        steam_id: "MYCOLOGIST",
        name: "Mycologist",
        description: "Gather 1000 mushrooms",
        category: AchCategory::Crafting,
        target: 1000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::BerryPicker,
        steam_id: "BERRY_PICKER",
        name: "Berry Picker",
        description: "Gather 10 berries",
        category: AchCategory::Crafting,
        target: 10,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FruitForager,
        steam_id: "FRUIT_FORAGER",
        name: "Fruit Forager",
        description: "Gather 50 berries",
        category: AchCategory::Crafting,
        target: 50,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::BerryEnthusiast,
        steam_id: "BERRY_ENTHUSIAST",
        name: "Berry Enthusiast",
        description: "Gather 200 berries",
        category: AchCategory::Crafting,
        target: 200,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::HarvestHero,
        steam_id: "HARVEST_HERO",
        name: "Harvest Hero",
        description: "Gather 500 berries",
        category: AchCategory::Crafting,
        target: 500,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterGatherer,
        steam_id: "MASTER_GATHERER",
        name: "Master Gatherer",
        description: "Gather 1000 berries",
        category: AchCategory::Crafting,
        target: 1000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Alchemist,
        steam_id: "ALCHEMIST",
        name: "Alchemist",
        description: "Brew 10 potions",
        category: AchCategory::Crafting,
        target: 10,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::JourneymanBrewer,
        steam_id: "JOURNEYMAN_BREWER",
        name: "Journeyman Brewer",
        description: "Brew 50 potions",
        category: AchCategory::Crafting,
        target: 50,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ArcaneAlchemist,
        steam_id: "ARCANE_ALCHEMIST",
        name: "Arcane Alchemist",
        description: "Brew 100 potions",
        category: AchCategory::Crafting,
        target: 100,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GrandmasterBrewer,
        steam_id: "GRANDMASTER_BREWER",
        name: "Grandmaster Brewer",
        description: "Brew 200 potions",
        category: AchCategory::Crafting,
        target: 200,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GoldLooter,
        steam_id: "GOLD_LOOTER",
        name: "Gold Looter",
        description: "Find and open the 80s mine gold room chest",
        category: AchCategory::Crafting,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::WellPaidGatherer,
        steam_id: "WELL_PAID_GATHERER",
        name: "Well-Paid Gatherer",
        description: "Hand in every alchemy item to Reskin",
        category: AchCategory::Crafting,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Solved,
        steam_id: "SOLVED",
        name: "Solved!",
        description: "Solve one pentagram quest",
        category: AchCategory::Pentagram,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FullOfSolves,
        steam_id: "FULL_OF_SOLVES",
        name: "Full of Solves",
        description: "Solve 20 pentagram quests",
        category: AchCategory::Pentagram,
        target: 20,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::RuneMaster,
        steam_id: "RUNE_MASTER",
        name: "Rune Master",
        description: "Solve 100 pentagram quests",
        category: AchCategory::Pentagram,
        target: 100,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GrandmasterPentagram,
        steam_id: "GRANDMASTER_PENTAGRAM",
        name: "Grandmaster of the Pentagram",
        description: "Solve 500 pentagram quests",
        category: AchCategory::Pentagram,
        target: 500,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::EarthboundNovice,
        steam_id: "EARTHBOUND_NOVICE",
        name: "Earthbound Novice",
        description: "Solve a pentagram in Earth Pents",
        category: AchCategory::Pentagram,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FlameInitiate,
        steam_id: "FLAME_INITIATE",
        name: "Flame Initiate",
        description: "Solve a pentagram in Fire Pents",
        category: AchCategory::Pentagram,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FightingTheFrost,
        steam_id: "FIGHTING_THE_FROST",
        name: "Fighting the Frost",
        description: "Solve a pentagram in Ice Pents",
        category: AchCategory::Pentagram,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ThroughGatesOfHell,
        steam_id: "THROUGH_GATES_OF_HELL",
        name: "Through the Gates of Hell",
        description: "Solve a pentagram in Hell Pents",
        category: AchCategory::Pentagram,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::HappyGoLucky,
        steam_id: "HAPPY_GO_LUCKY",
        name: "Happy Go Lucky",
        description: "Hit a lucky pentagram",
        category: AchCategory::Pentagram,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FavoredByFortune,
        steam_id: "FAVORED_BY_FORTUNE",
        name: "Favored by Fortune",
        description: "Two lucky pents on the same solve",
        category: AchCategory::Pentagram,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FiveInARow,
        steam_id: "FIVE_IN_A_ROW",
        name: "5 in a Row",
        description: "Get 5 pents the same color",
        category: AchCategory::Pentagram,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::TreasureHunter,
        steam_id: "TREASURE_HUNTER",
        name: "Treasure Hunter",
        description: "Open 50 chests",
        category: AchCategory::Collection,
        target: 50,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::EarthRocks,
        steam_id: "EARTH_ROCKS",
        name: "Earth Rocks!",
        description: "Collect 50 Earth stones",
        category: AchCategory::Collection,
        target: 50,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::FireRocks,
        steam_id: "FIRE_ROCKS",
        name: "Fire Rocks!",
        description: "Collect 100 Fire stones",
        category: AchCategory::Collection,
        target: 100,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::IceRocks,
        steam_id: "ICE_ROCKS",
        name: "Ice Rocks!",
        description: "Collect 1000 Ice stones",
        category: AchCategory::Collection,
        target: 1000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::WeaponNovice,
        steam_id: "WEAPON_NOVICE",
        name: "Weapon Novice",
        description: "Level a weapon skill to 10",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterOfArms,
        steam_id: "MASTER_OF_ARMS",
        name: "Master of Arms",
        description: "Level a weapon skill to 110",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ApprenticeMagic,
        steam_id: "APPRENTICE_MAGIC",
        name: "Apprentice of Magic",
        description: "Level Fire or Lightning magic to 10",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::IntermediateMagic,
        steam_id: "INTERMEDIATE_MAGIC",
        name: "Intermediate of Magic",
        description: "Level Fire or Lightning magic to 50",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterOfMagic,
        steam_id: "MASTER_OF_MAGIC",
        name: "Master of Magic",
        description: "Level Fire or Lightning magic to 110",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ApprenticeFighting,
        steam_id: "APPRENTICE_FIGHTING",
        name: "Apprentice of Fighting",
        description: "Level Attack or Parry to 10",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::IntermediateFighting,
        steam_id: "INTERMEDIATE_FIGHTING",
        name: "Intermediate of Fighting",
        description: "Level Attack or Parry to 50",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterOfFighting,
        steam_id: "MASTER_OF_FIGHTING",
        name: "Master of Fighting",
        description: "Level Attack or Parry to 110",
        category: AchCategory::Progression,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ClanMember,
        steam_id: "CLAN_MEMBER",
        name: "Clan Member",
        description: "Join a Clan",
        category: AchCategory::Social,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ClanMaster,
        steam_id: "CLAN_MASTER",
        name: "Clan Master",
        description: "Found your own clan",
        category: AchCategory::Social,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ClubMember,
        steam_id: "CLUB_MEMBER",
        name: "Club Member",
        description: "Join a club",
        category: AchCategory::Social,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::ClubMaster,
        steam_id: "CLUB_MASTER",
        name: "Club Master",
        description: "Found a club",
        category: AchCategory::Social,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::TrustButVerify,
        steam_id: "TRUST_BUT_VERIFY",
        name: "Trust, but Verify",
        description: "Use Trader for a trade with another player",
        category: AchCategory::Social,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::HardcoreHero,
        steam_id: "HARDCORE_HERO",
        name: "Hardcore Hero",
        description: "Reach level 50 on a hardcore character",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::HardcoreLegend,
        steam_id: "HARDCORE_LEGEND",
        name: "Hardcore Legend",
        description: "Reach level 100 on a hardcore character",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MilitaryVeteran,
        steam_id: "MILITARY_VETERAN",
        name: "Military Veteran",
        description: "Complete 100 military missions",
        category: AchCategory::Special,
        target: 100,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::TunnelRat,
        steam_id: "TUNNEL_RAT",
        name: "Tunnel Rat",
        description: "Complete all tunnel levels",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterTrader,
        steam_id: "MASTER_TRADER",
        name: "Master Trader",
        description: "Max out the Trader profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterAlchemist,
        steam_id: "MASTER_ALCHEMIST",
        name: "Master Alchemist",
        description: "Max out the Alchemist profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterHerbalistProf,
        steam_id: "MASTER_HERBALIST_PROF",
        name: "Master Herbalist (Prof)",
        description: "Max out the Herbalist profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterMiner,
        steam_id: "MASTER_MINER",
        name: "Master Miner",
        description: "Max out the Miner profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterAthlete,
        steam_id: "MASTER_ATHLETE",
        name: "Master Athlete",
        description: "Max out the Athlete profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterAssassin,
        steam_id: "MASTER_ASSASSIN",
        name: "Master Assassin",
        description: "Max out the Assassin profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterThief,
        steam_id: "MASTER_THIEF",
        name: "Master Thief",
        description: "Max out the Thief profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterLightWarrior,
        steam_id: "MASTER_LIGHT_WARRIOR",
        name: "Master of Light",
        description: "Max out the Light Warrior profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterDarkWarrior,
        steam_id: "MASTER_DARK_WARRIOR",
        name: "Master of Darkness",
        description: "Max out the Dark Warrior profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterMercenary,
        steam_id: "MASTER_MERCENARY",
        name: "Master Mercenary",
        description: "Max out the Mercenary profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::MasterClanWarrior,
        steam_id: "MASTER_CLAN_WARRIOR",
        name: "Master Clan Warrior",
        description: "Max out the Clan Warrior profession",
        category: AchCategory::Special,
        target: 0,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SilverNovice,
        steam_id: "SILVER_NOVICE",
        name: "Silver Novice",
        description: "Mine 100 silver units",
        category: AchCategory::Collection,
        target: 100,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SilverCollector,
        steam_id: "SILVER_COLLECTOR",
        name: "Silver Collector",
        description: "Mine 1,000 silver units",
        category: AchCategory::Collection,
        target: 1000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SilverHoarder,
        steam_id: "SILVER_HOARDER",
        name: "Silver Hoarder",
        description: "Mine 10,000 silver units",
        category: AchCategory::Collection,
        target: 10000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SilverBaron,
        steam_id: "SILVER_BARON",
        name: "Silver Baron",
        description: "Mine 100,000 silver units",
        category: AchCategory::Collection,
        target: 100000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SilverTycoon,
        steam_id: "SILVER_TYCOON",
        name: "Silver Tycoon",
        description: "Mine 1,000,000 silver units",
        category: AchCategory::Collection,
        target: 1000000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SilverMagnate,
        steam_id: "SILVER_MAGNATE",
        name: "Silver Magnate",
        description: "Mine 10,000,000 silver units",
        category: AchCategory::Collection,
        target: 10000000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::SilverLegend,
        steam_id: "SILVER_LEGEND",
        name: "Silver Legend",
        description: "Mine 50,000,000 silver units",
        category: AchCategory::Collection,
        target: 50000000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GoldNovice,
        steam_id: "GOLD_NOVICE",
        name: "Gold Novice",
        description: "Mine 50 gold units",
        category: AchCategory::Collection,
        target: 50,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GoldCollector,
        steam_id: "GOLD_COLLECTOR",
        name: "Gold Collector",
        description: "Mine 500 gold units",
        category: AchCategory::Collection,
        target: 500,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GoldHoarder,
        steam_id: "GOLD_HOARDER",
        name: "Gold Hoarder",
        description: "Mine 5,000 gold units",
        category: AchCategory::Collection,
        target: 5000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GoldBaron,
        steam_id: "GOLD_BARON",
        name: "Gold Baron",
        description: "Mine 50,000 gold units",
        category: AchCategory::Collection,
        target: 50000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GoldTycoon,
        steam_id: "GOLD_TYCOON",
        name: "Gold Tycoon",
        description: "Mine 500,000 gold units",
        category: AchCategory::Collection,
        target: 500000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GoldMagnate,
        steam_id: "GOLD_MAGNATE",
        name: "Gold Magnate",
        description: "Mine 5,000,000 gold units",
        category: AchCategory::Collection,
        target: 5000000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::GoldLegend,
        steam_id: "GOLD_LEGEND",
        name: "Gold Legend",
        description: "Mine 50,000,000 gold units",
        category: AchCategory::Collection,
        target: 50000000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::CoinCollector,
        steam_id: "COIN_COLLECTOR",
        name: "Coin Collector",
        description: "Earn 10,000 gold total",
        category: AchCategory::Collection,
        target: 10000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::WealthyAdventurer,
        steam_id: "WEALTHY_ADVENTURER",
        name: "Wealthy Adventurer",
        description: "Earn 100,000 gold total",
        category: AchCategory::Collection,
        target: 100000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::RichNoble,
        steam_id: "RICH_NOBLE",
        name: "Rich Noble",
        description: "Earn 1,000,000 gold total",
        category: AchCategory::Collection,
        target: 1000000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Millionaire,
        steam_id: "MILLIONAIRE",
        name: "Millionaire",
        description: "Earn 10,000,000 gold total",
        category: AchCategory::Collection,
        target: 10000000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Recruit,
        steam_id: "RECRUIT",
        name: "Recruit",
        description: "Complete 10 military missions",
        category: AchCategory::Combat,
        target: 10,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Soldier,
        steam_id: "SOLDIER",
        name: "Soldier",
        description: "Complete 25 military missions",
        category: AchCategory::Combat,
        target: 25,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Commander,
        steam_id: "COMMANDER",
        name: "Commander",
        description: "Complete 250 military missions",
        category: AchCategory::Combat,
        target: 250,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::General,
        steam_id: "GENERAL",
        name: "General",
        description: "Complete 500 military missions",
        category: AchCategory::Combat,
        target: 500,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::WarLegend,
        steam_id: "WAR_LEGEND",
        name: "War Legend",
        description: "Complete 1,000 military missions",
        category: AchCategory::Combat,
        target: 1000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::TunnelExplorer,
        steam_id: "TUNNEL_EXPLORER",
        name: "Tunnel Explorer",
        description: "Complete 10 tunnel sections",
        category: AchCategory::Exploration,
        target: 10,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::TunnelRunner,
        steam_id: "TUNNEL_RUNNER",
        name: "Tunnel Runner",
        description: "Complete 25 tunnel sections",
        category: AchCategory::Exploration,
        target: 25,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::TunnelVeteran,
        steam_id: "TUNNEL_VETERAN",
        name: "Tunnel Veteran",
        description: "Complete 50 tunnel sections",
        category: AchCategory::Exploration,
        target: 50,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Looter,
        steam_id: "LOOTER",
        name: "Looter",
        description: "Open 10 chests",
        category: AchCategory::Collection,
        target: 10,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::TreasureMaster,
        steam_id: "TREASURE_MASTER",
        name: "Treasure Master",
        description: "Open 100 chests",
        category: AchCategory::Collection,
        target: 100,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::LegendaryLooter,
        steam_id: "LEGENDARY_LOOTER",
        name: "Legendary Looter",
        description: "Open 500 chests",
        category: AchCategory::Collection,
        target: 500,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::PotionMaster,
        steam_id: "POTION_MASTER",
        name: "Potion Master",
        description: "Brew 500 potions",
        category: AchCategory::Crafting,
        target: 500,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::LegendaryBrewer,
        steam_id: "LEGENDARY_BREWER",
        name: "Legendary Brewer",
        description: "Brew 1,000 potions",
        category: AchCategory::Crafting,
        target: 1000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::DedicatedPlayer,
        steam_id: "DEDICATED_PLAYER",
        name: "Dedicated Player",
        description: "Play for 24 hours total",
        category: AchCategory::Special,
        target: 1440,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::VeteranPlayer,
        steam_id: "VETERAN_PLAYER",
        name: "Veteran Player",
        description: "Play for 100 hours total",
        category: AchCategory::Special,
        target: 6000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::UgarisLifer,
        steam_id: "UGARIS_LIFER",
        name: "Ugaris Lifer",
        description: "Play for 500 hours total",
        category: AchCategory::Special,
        target: 30000,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Regular,
        steam_id: "REGULAR",
        name: "Regular",
        description: "Log in 7 days in a row",
        category: AchCategory::Special,
        target: 7,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Committed,
        steam_id: "COMMITTED",
        name: "Committed",
        description: "Log in 30 days in a row",
        category: AchCategory::Special,
        target: 30,
        hidden: false,
    },
    AchievementDef {
        ty: AchievementType::Devoted,
        steam_id: "DEVOTED",
        name: "Devoted",
        description: "Log in 100 days in a row",
        category: AchCategory::Special,
        target: 100,
        hidden: false,
    },
];

/// C `achievement_get_def` (`achievement.c:560-565`).
pub fn achievement_def(ty: AchievementType) -> &'static AchievementDef {
    &ACHIEVEMENT_DEFS[ty as usize]
}

/// C `achievement_area_to_pent_index` (`achievement.c:378-391`).
pub fn area_to_pent_index(area_id: i32) -> Option<PentArea> {
    match area_id {
        4 => Some(PentArea::Earth),
        7 => Some(PentArea::Fire),
        9 => Some(PentArea::Ice),
        34 => Some(PentArea::Hell),
        _ => None,
    }
}

/// C `struct Achievement` (`achievement.h:218-223`).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Achievement {
    /// Unix timestamp when earned; `0` = not achieved.
    pub timestamp: i64,
    pub progress: u32,
    pub target: u32,
    /// Character name who earned it (C `char achieved_by[40]`).
    pub achieved_by: String,
}

/// C `struct AccountAchievements` (`achievement.h:226-229`): per-subscriber
/// (account-wide in C; left per-character here pending the PPD/DB wiring
/// task noted in the module doc comment above) achievement storage.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AccountAchievements {
    pub version: u32,
    #[serde(with = "achievement_array_serde")]
    pub achievements: [Achievement; MAX_ACHIEVEMENTS],
}

/// `serde` support for the fixed-size 128-entry `Achievement` array.
/// `#[derive(Serialize, Deserialize)]` only covers array lengths 0..=32
/// out of the box (`Achievement` isn't `Copy`, so the const-generic array
/// impl serde otherwise offers doesn't apply here either); this goes
/// through a `Vec` on the wire and rebuilds the fixed array on load, padding
/// short/legacy data with `Achievement::default()` rather than erroring.
mod achievement_array_serde {
    use super::{Achievement, MAX_ACHIEVEMENTS};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        value: &[Achievement; MAX_ACHIEVEMENTS],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[Achievement; MAX_ACHIEVEMENTS], D::Error> {
        let vec = Vec::<Achievement>::deserialize(deserializer)?;
        let mut out: [Achievement; MAX_ACHIEVEMENTS] =
            std::array::from_fn(|_| Achievement::default());
        for (slot, value) in out.iter_mut().zip(vec) {
            *slot = value;
        }
        Ok(out)
    }
}

impl Default for AccountAchievements {
    fn default() -> Self {
        AccountAchievements {
            version: 0,
            achievements: std::array::from_fn(|_| Achievement::default()),
        }
    }
}

impl AccountAchievements {
    /// C `achievement_is_unlocked` (`achievement.c:567-576`).
    pub fn is_unlocked(&self, ty: AchievementType) -> bool {
        self.achievements[ty as usize].timestamp != 0
    }

    /// C `achievement_get_progress` (`achievement.c:671-680`).
    pub fn get_progress(&self, ty: AchievementType) -> u32 {
        self.achievements[ty as usize].progress
    }

    /// C `achievement_award` (`achievement.c:578-632`), minus the
    /// Steam-sync/DB-first-unlock/chat-announce/log side effects the C
    /// version performs inline (those need `World`/networking access this
    /// leaf module doesn't have; the caller should perform them when this
    /// returns `true`). Returns `true` if this call newly unlocked the
    /// achievement (`false` if it was already unlocked).
    pub fn award(&mut self, ty: AchievementType, achieved_by: &str, now: i64) -> bool {
        let def = achievement_def(ty);
        let ach = &mut self.achievements[ty as usize];
        if ach.timestamp != 0 {
            return false;
        }
        ach.timestamp = now;
        ach.progress = if def.target > 0 { def.target } else { 1 };
        ach.target = def.target;
        ach.achieved_by = achieved_by.chars().take(39).collect();
        true
    }

    /// C `achievement_add_progress` (`achievement.c:634-669`). Returns
    /// `true` if this call's progress crossed the target and newly
    /// unlocked the achievement.
    pub fn add_progress(
        &mut self,
        ty: AchievementType,
        amount: u32,
        achieved_by: &str,
        now: i64,
    ) -> bool {
        let def = achievement_def(ty);
        {
            let ach = &mut self.achievements[ty as usize];
            if ach.timestamp != 0 {
                return false;
            }
            if def.target == 0 {
                return false;
            }
            ach.progress = ach.progress.saturating_add(amount);
            ach.target = def.target;
        }
        if self.achievements[ty as usize].progress >= def.target {
            self.award(ty, achieved_by, now)
        } else {
            false
        }
    }
}

/// C `struct AchievementStats` (`achievement.h:232-276`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct AchievementStats {
    pub flowers_picked: u32,
    pub mushrooms_picked: u32,
    pub berries_picked: u32,
    pub potions_brewed: u32,

    pub demons_defeated: u64,
    pub demons_per_area: [u64; PENT_AREA_COUNT],
    pub enemies_killed: u32,
    pub pvp_kills: u32,

    pub pents_solved: u32,
    pub pents_per_area: [u32; PENT_AREA_COUNT],
    pub lucky_pents_hit: u32,

    pub chests_opened: u32,
    pub earth_stones: u32,
    pub fire_stones: u32,
    pub ice_stones: u32,

    pub military_missions: u32,
    pub tunnel_levels: u32,

    pub silver_mined: u64,
    pub gold_mined: u64,

    pub gold_earned: u64,

    pub play_time_minutes: u32,

    pub login_streak: u32,
    pub last_login_day: u32,
}

/// C `achievement_get_stat_progress` (`achievement.c:398-530`): the
/// current-progress value used for progress-bar display, derived from
/// `AchievementStats` rather than the per-achievement `progress` field
/// (which C keeps separately via `achievement_add_progress` and which this
/// module's stat-update functions below don't call - matching the C
/// module, where the `add_*` family calls `achievement_award` directly
/// once a threshold is crossed rather than `achievement_add_progress`).
pub fn get_stat_progress(stats: &AchievementStats, ty: AchievementType) -> u32 {
    use AchievementType::*;
    match ty {
        GreenThumb | BotanyEnthusiast | NaturesFriend | Herbalist | MasterHerbalist => {
            stats.flowers_picked
        }
        MushroomHunter | FungusFinder | SporeSeeker | MushroomMaster | Mycologist => {
            stats.mushrooms_picked
        }
        BerryPicker | FruitForager | BerryEnthusiast | HarvestHero | MasterGatherer => {
            stats.berries_picked
        }
        Alchemist | JourneymanBrewer | ArcaneAlchemist | GrandmasterBrewer | PotionMaster
        | LegendaryBrewer => stats.potions_brewed,
        FiendFighter | Demonbane | DreadDestroyer | DemonicExterminator => {
            stats.demons_defeated.min(u32::MAX as u64) as u32
        }
        FullOfSolves | RuneMaster | GrandmasterPentagram => stats.pents_solved,
        Looter | TreasureHunter | TreasureMaster | LegendaryLooter => stats.chests_opened,
        EarthRocks => stats.earth_stones,
        FireRocks => stats.fire_stones,
        IceRocks => stats.ice_stones,
        Recruit | Soldier | MilitaryVeteran | Commander | General | WarLegend => {
            stats.military_missions
        }
        TunnelExplorer | TunnelRunner | TunnelVeteran => stats.tunnel_levels,
        SilverNovice | SilverCollector | SilverHoarder | SilverBaron | SilverTycoon
        | SilverMagnate | SilverLegend => stats.silver_mined.min(u32::MAX as u64) as u32,
        GoldNovice | GoldCollector | GoldHoarder | GoldBaron | GoldTycoon | GoldMagnate
        | GoldLegend => stats.gold_mined.min(u32::MAX as u64) as u32,
        CoinCollector | WealthyAdventurer | RichNoble | Millionaire => {
            stats.gold_earned.min(u32::MAX as u64) as u32
        }
        DedicatedPlayer | VeteranPlayer | UgarisLifer => stats.play_time_minutes,
        Regular | Committed | Devoted => stats.login_streak,
        // Demon lord achievements are tracked separately, not in AchievementStats.
        SlayerOfDemonLords => 0,
        // All other achievements are instant/one-time, no progress tracking.
        _ => 0,
    }
}

/// Helper: push `ty` into `out` if `data.award(...)` newly unlocked it.
fn push_if_awarded(
    out: &mut Vec<AchievementType>,
    data: &mut AccountAchievements,
    ty: AchievementType,
    achieved_by: &str,
    now: i64,
) {
    if data.award(ty, achieved_by, now) {
        out.push(ty);
    }
}

/// C `achievement_add_flowers` (`achievement.c:686-710`).
pub fn add_flowers(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.flowers_picked = stats.flowers_picked.saturating_add(count);
    let mut out = Vec::new();
    if stats.flowers_picked >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GreenThumb,
            achieved_by,
            now,
        );
    }
    if stats.flowers_picked >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::BotanyEnthusiast,
            achieved_by,
            now,
        );
    }
    if stats.flowers_picked >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::NaturesFriend,
            achieved_by,
            now,
        );
    }
    if stats.flowers_picked >= 500 {
        push_if_awarded(&mut out, data, AchievementType::Herbalist, achieved_by, now);
    }
    if stats.flowers_picked >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MasterHerbalist,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_mushrooms` (`achievement.c:712-736`).
pub fn add_mushrooms(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.mushrooms_picked = stats.mushrooms_picked.saturating_add(count);
    let mut out = Vec::new();
    if stats.mushrooms_picked >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MushroomHunter,
            achieved_by,
            now,
        );
    }
    if stats.mushrooms_picked >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FungusFinder,
            achieved_by,
            now,
        );
    }
    if stats.mushrooms_picked >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SporeSeeker,
            achieved_by,
            now,
        );
    }
    if stats.mushrooms_picked >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MushroomMaster,
            achieved_by,
            now,
        );
    }
    if stats.mushrooms_picked >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::Mycologist,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_berries` (`achievement.c:738-762`).
pub fn add_berries(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.berries_picked = stats.berries_picked.saturating_add(count);
    let mut out = Vec::new();
    if stats.berries_picked >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::BerryPicker,
            achieved_by,
            now,
        );
    }
    if stats.berries_picked >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FruitForager,
            achieved_by,
            now,
        );
    }
    if stats.berries_picked >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::BerryEnthusiast,
            achieved_by,
            now,
        );
    }
    if stats.berries_picked >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::HarvestHero,
            achieved_by,
            now,
        );
    }
    if stats.berries_picked >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MasterGatherer,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_potions` (`achievement.c:764-791`).
pub fn add_potions(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.potions_brewed = stats.potions_brewed.saturating_add(count);
    let mut out = Vec::new();
    if stats.potions_brewed >= 10 {
        push_if_awarded(&mut out, data, AchievementType::Alchemist, achieved_by, now);
    }
    if stats.potions_brewed >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::JourneymanBrewer,
            achieved_by,
            now,
        );
    }
    if stats.potions_brewed >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::ArcaneAlchemist,
            achieved_by,
            now,
        );
    }
    if stats.potions_brewed >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GrandmasterBrewer,
            achieved_by,
            now,
        );
    }
    if stats.potions_brewed >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::PotionMaster,
            achieved_by,
            now,
        );
    }
    if stats.potions_brewed >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::LegendaryBrewer,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_demons` (`achievement.c:793-819`).
pub fn add_demons(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    area_id: i32,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    if let Some(idx) = area_to_pent_index(area_id) {
        stats.demons_per_area[idx as usize] =
            stats.demons_per_area[idx as usize].saturating_add(count as u64);
    }
    stats.demons_defeated = stats.demons_defeated.saturating_add(count as u64);
    let mut out = Vec::new();
    if stats.demons_defeated >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FiendFighter,
            achieved_by,
            now,
        );
    }
    if stats.demons_defeated >= 2500 {
        push_if_awarded(&mut out, data, AchievementType::Demonbane, achieved_by, now);
    }
    if stats.demons_defeated >= 15000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::DreadDestroyer,
            achieved_by,
            now,
        );
    }
    if stats.demons_defeated >= 250000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::DemonicExterminator,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_pents` (`achievement.c:821-863`).
pub fn add_pents(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    area_id: i32,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    if let Some(idx) = area_to_pent_index(area_id) {
        stats.pents_per_area[idx as usize] =
            stats.pents_per_area[idx as usize].saturating_add(count);
        let area_ach = match idx {
            PentArea::Earth => AchievementType::EarthboundNovice,
            PentArea::Fire => AchievementType::FlameInitiate,
            PentArea::Ice => AchievementType::FightingTheFrost,
            PentArea::Hell => AchievementType::ThroughGatesOfHell,
        };
        push_if_awarded(&mut out, data, area_ach, achieved_by, now);
    }

    stats.pents_solved = stats.pents_solved.saturating_add(count);
    if stats.pents_solved >= 1 {
        push_if_awarded(&mut out, data, AchievementType::Solved, achieved_by, now);
    }
    if stats.pents_solved >= 20 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FullOfSolves,
            achieved_by,
            now,
        );
    }
    if stats.pents_solved >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::RuneMaster,
            achieved_by,
            now,
        );
    }
    if stats.pents_solved >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GrandmasterPentagram,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_chests` (`achievement.c:865-886`).
pub fn add_chests(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.chests_opened = stats.chests_opened.saturating_add(count);
    let mut out = Vec::new();
    if stats.chests_opened >= 10 {
        push_if_awarded(&mut out, data, AchievementType::Looter, achieved_by, now);
    }
    if stats.chests_opened >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TreasureHunter,
            achieved_by,
            now,
        );
    }
    if stats.chests_opened >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TreasureMaster,
            achieved_by,
            now,
        );
    }
    if stats.chests_opened >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::LegendaryLooter,
            achieved_by,
            now,
        );
    }
    out
}

/// Legacy stone-type indices used by C `achievement_add_stones`'s
/// `switch (stone_type)` (`achievement.c:894-913`): `0` = Earth, `1` =
/// Fire, `2` = Ice.
pub const STONE_TYPE_EARTH: i32 = 0;
pub const STONE_TYPE_FIRE: i32 = 1;
pub const STONE_TYPE_ICE: i32 = 2;

/// C `achievement_add_stones` (`achievement.c:888-914`).
pub fn add_stones(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    stone_type: i32,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    match stone_type {
        STONE_TYPE_EARTH => {
            stats.earth_stones = stats.earth_stones.saturating_add(count);
            if stats.earth_stones >= 50 {
                push_if_awarded(
                    &mut out,
                    data,
                    AchievementType::EarthRocks,
                    achieved_by,
                    now,
                );
            }
        }
        STONE_TYPE_FIRE => {
            stats.fire_stones = stats.fire_stones.saturating_add(count);
            if stats.fire_stones >= 100 {
                push_if_awarded(&mut out, data, AchievementType::FireRocks, achieved_by, now);
            }
        }
        STONE_TYPE_ICE => {
            stats.ice_stones = stats.ice_stones.saturating_add(count);
            if stats.ice_stones >= 1000 {
                push_if_awarded(&mut out, data, AchievementType::IceRocks, achieved_by, now);
            }
        }
        _ => {}
    }
    out
}

/// C `achievement_add_enemy_killed` (`achievement.c:916-928`).
pub fn add_enemy_killed(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.enemies_killed = stats.enemies_killed.saturating_add(1);
    let mut out = Vec::new();
    if stats.enemies_killed == 1 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FirstBlood,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_pvp_kill` (`achievement.c:930-941`).
pub fn add_pvp_kill(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.pvp_kills = stats.pvp_kills.saturating_add(1);
    let mut out = Vec::new();
    if stats.pvp_kills >= 1 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::ArenaCombatant,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_military_mission` (`achievement.c:943-970`).
pub fn add_military_mission(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.military_missions = stats.military_missions.saturating_add(1);
    let mut out = Vec::new();
    if stats.military_missions >= 10 {
        push_if_awarded(&mut out, data, AchievementType::Recruit, achieved_by, now);
    }
    if stats.military_missions >= 25 {
        push_if_awarded(&mut out, data, AchievementType::Soldier, achieved_by, now);
    }
    if stats.military_missions >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MilitaryVeteran,
            achieved_by,
            now,
        );
    }
    if stats.military_missions >= 250 {
        push_if_awarded(&mut out, data, AchievementType::Commander, achieved_by, now);
    }
    if stats.military_missions >= 500 {
        push_if_awarded(&mut out, data, AchievementType::General, achieved_by, now);
    }
    if stats.military_missions >= 1000 {
        push_if_awarded(&mut out, data, AchievementType::WarLegend, achieved_by, now);
    }
    out
}

/// C `achievement_add_tunnel_level` (`achievement.c:972-994`).
pub fn add_tunnel_level(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.tunnel_levels = stats.tunnel_levels.saturating_add(1);
    let mut out = Vec::new();
    if stats.tunnel_levels >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TunnelExplorer,
            achieved_by,
            now,
        );
    }
    if stats.tunnel_levels >= 25 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TunnelRunner,
            achieved_by,
            now,
        );
    }
    if stats.tunnel_levels >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TunnelVeteran,
            achieved_by,
            now,
        );
    }
    // C comment: "Award Tunnel Rat achievement after completing 100 tunnel sections".
    if stats.tunnel_levels >= 100 {
        push_if_awarded(&mut out, data, AchievementType::TunnelRat, achieved_by, now);
    }
    out
}

/// C `achievement_add_silver_mined` (`achievement.c:996-1026`).
pub fn add_silver_mined(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    amount: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.silver_mined = stats.silver_mined.saturating_add(amount as u64);
    let mut out = Vec::new();
    if stats.silver_mined >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverNovice,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverCollector,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 10000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverHoarder,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 100000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverBaron,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 1000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverTycoon,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 10000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverMagnate,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 50000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverLegend,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_gold_mined` (`achievement.c:1028-1058`).
pub fn add_gold_mined(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    amount: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.gold_mined = stats.gold_mined.saturating_add(amount as u64);
    let mut out = Vec::new();
    if stats.gold_mined >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldNovice,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldCollector,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 5000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldHoarder,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 50000 {
        push_if_awarded(&mut out, data, AchievementType::GoldBaron, achieved_by, now);
    }
    if stats.gold_mined >= 500000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldTycoon,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 5000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldMagnate,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 50000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldLegend,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_gold_earned` (`achievement.c:1060-1081`).
pub fn add_gold_earned(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    amount: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.gold_earned = stats.gold_earned.saturating_add(amount as u64);
    let mut out = Vec::new();
    if stats.gold_earned >= 10000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::CoinCollector,
            achieved_by,
            now,
        );
    }
    if stats.gold_earned >= 100000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::WealthyAdventurer,
            achieved_by,
            now,
        );
    }
    if stats.gold_earned >= 1000000 {
        push_if_awarded(&mut out, data, AchievementType::RichNoble, achieved_by, now);
    }
    if stats.gold_earned >= 10000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::Millionaire,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_play_time` (`achievement.c:1083-1101`).
pub fn add_play_time(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    minutes: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.play_time_minutes = stats.play_time_minutes.saturating_add(minutes);
    let mut out = Vec::new();
    if stats.play_time_minutes >= 1440 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::DedicatedPlayer,
            achieved_by,
            now,
        );
    }
    if stats.play_time_minutes >= 6000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::VeteranPlayer,
            achieved_by,
            now,
        );
    }
    if stats.play_time_minutes >= 30000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::UgarisLifer,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_check_login_streak` (`achievement.c:1103-1139`). `now`
/// is a Unix timestamp (seconds); C computes `current_day = now / 86400`.
pub fn check_login_streak(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let current_day = (now / 86400) as u32;

    if stats.last_login_day == 0 {
        stats.login_streak = 1;
        stats.last_login_day = current_day;
    } else if current_day == stats.last_login_day {
        // Already logged in today, do nothing.
    } else if current_day == stats.last_login_day + 1 {
        stats.login_streak = stats.login_streak.saturating_add(1);
        stats.last_login_day = current_day;
    } else {
        stats.login_streak = 1;
        stats.last_login_day = current_day;
    }

    let mut out = Vec::new();
    if stats.login_streak >= 7 {
        push_if_awarded(&mut out, data, AchievementType::Regular, achieved_by, now);
    }
    if stats.login_streak >= 30 {
        push_if_awarded(&mut out, data, AchievementType::Committed, achieved_by, now);
    }
    if stats.login_streak >= 100 {
        push_if_awarded(&mut out, data, AchievementType::Devoted, achieved_by, now);
    }
    out
}

/// C `achievement_check_level` (`achievement.c:1145-1176`).
pub fn check_level(
    data: &mut AccountAchievements,
    level: i32,
    is_hardcore: bool,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    if level >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::RisingBeginner,
            achieved_by,
            now,
        );
    }
    if level >= 20 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::ExperiencedHero,
            achieved_by,
            now,
        );
    }
    if level >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::UgarisVeteran,
            achieved_by,
            now,
        );
        if is_hardcore {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::HardcoreHero,
                achieved_by,
                now,
            );
        }
    }
    if level >= 75 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::LegendaryAdventurer,
            achieved_by,
            now,
        );
    }
    if level >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::DemonSlayer,
            achieved_by,
            now,
        );
        if is_hardcore {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::HardcoreLegend,
                achieved_by,
                now,
            );
        }
    }
    if level >= 150 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MasterOfHell,
            achieved_by,
            now,
        );
    }
    if level >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MasterOfUgaris,
            achieved_by,
            now,
        );
    }
    out
}

/// Legacy weapon-skill value range (`V_DAGGER` .. `V_TWOHAND`,
/// `src/server.h:322-326`: dagger/hand-to-hand/staff/sword/two-hand).
pub const V_DAGGER: i32 = 12;
pub const V_TWOHAND: i32 = 16;
/// `src/server.h:328-329`.
pub const V_ATTACK: i32 = 18;
pub const V_PARRY: i32 = 19;
/// `src/server.h:342-344`: `V_FLASH` (Lightning) and `V_FIRE` (alias of
/// `V_FIREBALL`, already `crate::entity::V_FIREBALL`).
pub const V_FLASH: i32 = 32;
pub const V_FIRE: i32 = crate::entity::V_FIREBALL;

/// C `achievement_check_skill` (`achievement.c:1178-1218`).
pub fn check_skill(
    data: &mut AccountAchievements,
    skill_type: i32,
    skill_level: i32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    if (V_DAGGER..=V_TWOHAND).contains(&skill_type) {
        if skill_level >= 10 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::WeaponNovice,
                achieved_by,
                now,
            );
        }
        if skill_level >= 110 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::MasterOfArms,
                achieved_by,
                now,
            );
        }
    }
    if skill_type == V_FIRE || skill_type == V_FLASH {
        if skill_level >= 10 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::ApprenticeMagic,
                achieved_by,
                now,
            );
        }
        if skill_level >= 50 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::IntermediateMagic,
                achieved_by,
                now,
            );
        }
        if skill_level >= 110 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::MasterOfMagic,
                achieved_by,
                now,
            );
        }
    }
    if skill_type == V_ATTACK || skill_type == V_PARRY {
        if skill_level >= 10 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::ApprenticeFighting,
                achieved_by,
                now,
            );
        }
        if skill_level >= 50 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::IntermediateFighting,
                achieved_by,
                now,
            );
        }
        if skill_level >= 110 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::MasterOfFighting,
                achieved_by,
                now,
            );
        }
    }
    out
}

/// Legacy profession type indices (`src/common/professor.c`'s `P_*`
/// constants, documented verbatim in `achievement_check_profession`'s C
/// comment, `achievement.c:1221-1223`).
pub const P_ATHLETE: i32 = 0;
pub const P_ALCHEMIST: i32 = 1;
pub const P_MINER: i32 = 2;
pub const P_ASSASSIN: i32 = 3;
pub const P_THIEF: i32 = 4;
pub const P_LIGHT: i32 = 5;
pub const P_DARK: i32 = 6;
pub const P_TRADER: i32 = 7;
pub const P_MERCENARY: i32 = 8;
pub const P_CLAN: i32 = 9;
pub const P_HERBALIST: i32 = 10;

/// C `achievement_check_profession` (`achievement.c:1220-1285`). Max
/// levels per profession are documented in the C comment
/// (`achievement.c:1225-1226`): Athlete=30, Alchemist=50, Miner=20,
/// Assassin=50, Thief=30, Light=30, Dark=30, Trader=20, Mercenary=20,
/// Clan=30, Herbalist=30.
pub fn check_profession(
    data: &mut AccountAchievements,
    prof_type: i32,
    prof_level: i32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    let (threshold, ty) = match prof_type {
        P_ATHLETE => (30, AchievementType::MasterAthlete),
        P_ALCHEMIST => (50, AchievementType::MasterAlchemist),
        P_MINER => (20, AchievementType::MasterMiner),
        P_ASSASSIN => (50, AchievementType::MasterAssassin),
        P_THIEF => (30, AchievementType::MasterThief),
        P_LIGHT => (30, AchievementType::MasterLightWarrior),
        P_DARK => (30, AchievementType::MasterDarkWarrior),
        P_TRADER => (20, AchievementType::MasterTrader),
        P_MERCENARY => (20, AchievementType::MasterMercenary),
        P_CLAN => (30, AchievementType::MasterClanWarrior),
        P_HERBALIST => (30, AchievementType::MasterHerbalistProf),
        _ => return out,
    };
    if prof_level >= threshold {
        push_if_awarded(&mut out, data, ty, achieved_by, now);
    }
    out
}

/// Legacy area id for Aston (C `achievement_check_exploration`'s
/// `case 3:`, `achievement.c:1801`).
pub const AREA_ASTON: i32 = 3;

/// C `achievement_check_exploration` (`achievement.c:1794-1807`).
pub fn check_exploration(
    data: &mut AccountAchievements,
    area_id: i32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    if area_id == AREA_ASTON {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::UgarisPathfinder,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_clear_all` (`achievement.c:1774-1788`).
pub fn clear_all(data: &mut AccountAchievements, stats: &mut AchievementStats) {
    *data = AccountAchievements::default();
    *stats = AchievementStats::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_700_000_000;

    #[test]
    fn defs_table_has_one_entry_per_achievement_type_in_index_order() {
        assert_eq!(ACHIEVEMENT_DEFS.len(), ACHIEVEMENT_TYPE_COUNT);
        assert_eq!(AchievementType::ALL.len(), ACHIEVEMENT_TYPE_COUNT);
        for (idx, def) in ACHIEVEMENT_DEFS.iter().enumerate() {
            assert_eq!(def.ty as usize, idx, "def at index {idx} has mismatched ty");
            assert_eq!(AchievementType::ALL[idx] as usize, idx);
            assert!(!def.steam_id.is_empty());
            assert!(!def.name.is_empty());
            assert!(!def.description.is_empty());
            assert!(!def.hidden, "no C table entry is hidden today");
        }
    }

    #[test]
    fn defs_table_spot_checks_match_c_source_digit_for_digit() {
        let d = achievement_def(AchievementType::Demonbane);
        assert_eq!(d.steam_id, "DEMONBANE");
        assert_eq!(d.name, "Demonbane");
        assert_eq!(d.description, "Defeat 2,500 demons");
        assert_eq!(d.category, AchCategory::Combat);
        assert_eq!(d.target, 2500);

        let d = achievement_def(AchievementType::DemonicExterminator);
        assert_eq!(d.target, 250_000);

        let d = achievement_def(AchievementType::SilverLegend);
        assert_eq!(d.target, 50_000_000);

        let d = achievement_def(AchievementType::DedicatedPlayer);
        assert_eq!(d.target, 1440);
        assert_eq!(d.description, "Play for 24 hours total");

        let d = achievement_def(AchievementType::MasterHerbalistProf);
        assert_eq!(d.steam_id, "MASTER_HERBALIST_PROF");
        assert_eq!(d.name, "Master Herbalist (Prof)");

        let d = achievement_def(AchievementType::FiveInARow);
        assert_eq!(d.steam_id, "FIVE_IN_A_ROW");
        assert_eq!(d.name, "5 in a Row");

        let d = achievement_def(AchievementType::Devoted);
        assert_eq!(d.target, 100);
        assert_eq!(d.category, AchCategory::Special);
    }

    #[test]
    fn area_to_pent_index_matches_c_switch() {
        assert_eq!(area_to_pent_index(4), Some(PentArea::Earth));
        assert_eq!(area_to_pent_index(7), Some(PentArea::Fire));
        assert_eq!(area_to_pent_index(9), Some(PentArea::Ice));
        assert_eq!(area_to_pent_index(34), Some(PentArea::Hell));
        assert_eq!(area_to_pent_index(1), None);
        assert_eq!(area_to_pent_index(0), None);
    }

    #[test]
    fn award_unlocks_once_and_sets_progress_and_target() {
        let mut data = AccountAchievements::default();
        assert!(!data.is_unlocked(AchievementType::FirstBlood));
        let newly = data.award(AchievementType::FirstBlood, "Hero", NOW);
        assert!(newly);
        assert!(data.is_unlocked(AchievementType::FirstBlood));
        assert_eq!(data.get_progress(AchievementType::FirstBlood), 1); // target 0 -> progress 1
        assert_eq!(
            data.achievements[AchievementType::FirstBlood as usize].achieved_by,
            "Hero"
        );
        assert_eq!(
            data.achievements[AchievementType::FirstBlood as usize].timestamp,
            NOW
        );

        // Second award is a no-op (already unlocked).
        let newly_again = data.award(AchievementType::FirstBlood, "Someone Else", NOW + 1);
        assert!(!newly_again);
        assert_eq!(
            data.achievements[AchievementType::FirstBlood as usize].achieved_by,
            "Hero"
        );
        assert_eq!(
            data.achievements[AchievementType::FirstBlood as usize].timestamp,
            NOW
        );
    }

    #[test]
    fn award_target_based_sets_progress_to_target() {
        let mut data = AccountAchievements::default();
        data.award(AchievementType::FiendFighter, "Hero", NOW);
        assert_eq!(data.get_progress(AchievementType::FiendFighter), 100);
    }

    #[test]
    fn add_progress_unlocks_only_when_target_reached() {
        let mut data = AccountAchievements::default();
        let def = achievement_def(AchievementType::FiendFighter);
        assert_eq!(def.target, 100);

        assert!(!data.add_progress(AchievementType::FiendFighter, 50, "Hero", NOW));
        assert!(!data.is_unlocked(AchievementType::FiendFighter));
        assert_eq!(data.get_progress(AchievementType::FiendFighter), 50);

        assert!(data.add_progress(AchievementType::FiendFighter, 50, "Hero", NOW));
        assert!(data.is_unlocked(AchievementType::FiendFighter));

        // Further progress calls are no-ops once unlocked.
        assert!(!data.add_progress(AchievementType::FiendFighter, 1000, "Hero", NOW));
        assert_eq!(data.get_progress(AchievementType::FiendFighter), 100);
    }

    #[test]
    fn add_progress_on_instant_achievement_is_a_no_op() {
        // FirstBlood has target 0 (instant); add_progress never awards it,
        // matching C's `if (def->target == 0) return;` early-out.
        let mut data = AccountAchievements::default();
        assert!(!data.add_progress(AchievementType::FirstBlood, 1, "Hero", NOW));
        assert!(!data.is_unlocked(AchievementType::FirstBlood));
    }

    #[test]
    fn get_stat_progress_covers_every_stat_category() {
        let mut stats = AchievementStats::default();
        stats.flowers_picked = 5;
        stats.mushrooms_picked = 6;
        stats.berries_picked = 7;
        stats.potions_brewed = 8;
        stats.demons_defeated = 9;
        stats.pents_solved = 10;
        stats.chests_opened = 11;
        stats.earth_stones = 12;
        stats.fire_stones = 13;
        stats.ice_stones = 14;
        stats.military_missions = 15;
        stats.tunnel_levels = 16;
        stats.silver_mined = 17;
        stats.gold_mined = 18;
        stats.gold_earned = 19;
        stats.play_time_minutes = 20;
        stats.login_streak = 21;

        assert_eq!(get_stat_progress(&stats, AchievementType::GreenThumb), 5);
        assert_eq!(
            get_stat_progress(&stats, AchievementType::MushroomHunter),
            6
        );
        assert_eq!(get_stat_progress(&stats, AchievementType::BerryPicker), 7);
        assert_eq!(get_stat_progress(&stats, AchievementType::Alchemist), 8);
        assert_eq!(get_stat_progress(&stats, AchievementType::FiendFighter), 9);
        assert_eq!(get_stat_progress(&stats, AchievementType::FullOfSolves), 10);
        assert_eq!(get_stat_progress(&stats, AchievementType::Looter), 11);
        assert_eq!(get_stat_progress(&stats, AchievementType::EarthRocks), 12);
        assert_eq!(get_stat_progress(&stats, AchievementType::FireRocks), 13);
        assert_eq!(get_stat_progress(&stats, AchievementType::IceRocks), 14);
        assert_eq!(get_stat_progress(&stats, AchievementType::Recruit), 15);
        assert_eq!(
            get_stat_progress(&stats, AchievementType::TunnelExplorer),
            16
        );
        assert_eq!(get_stat_progress(&stats, AchievementType::SilverNovice), 17);
        assert_eq!(get_stat_progress(&stats, AchievementType::GoldNovice), 18);
        assert_eq!(
            get_stat_progress(&stats, AchievementType::CoinCollector),
            19
        );
        assert_eq!(
            get_stat_progress(&stats, AchievementType::DedicatedPlayer),
            20
        );
        assert_eq!(get_stat_progress(&stats, AchievementType::Regular), 21);

        // Demon lords are tracked separately, not via AchievementStats.
        assert_eq!(
            get_stat_progress(&stats, AchievementType::SlayerOfDemonLords),
            0
        );
        // Instant achievements have no stat-driven progress.
        assert_eq!(get_stat_progress(&stats, AchievementType::FirstBlood), 0);
    }

    #[test]
    fn get_stat_progress_caps_u64_counters_at_u32_max() {
        let mut stats = AchievementStats::default();
        stats.demons_defeated = u64::MAX;
        stats.silver_mined = u64::MAX;
        stats.gold_mined = u64::MAX;
        stats.gold_earned = u64::MAX;
        assert_eq!(
            get_stat_progress(&stats, AchievementType::Demonbane),
            u32::MAX
        );
        assert_eq!(
            get_stat_progress(&stats, AchievementType::SilverLegend),
            u32::MAX
        );
        assert_eq!(
            get_stat_progress(&stats, AchievementType::GoldLegend),
            u32::MAX
        );
        assert_eq!(
            get_stat_progress(&stats, AchievementType::Millionaire),
            u32::MAX
        );
    }

    #[test]
    fn add_flowers_full_ladder() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();

        let unlocked = add_flowers(&mut data, &mut stats, 9, "Hero", NOW);
        assert!(unlocked.is_empty());

        let unlocked = add_flowers(&mut data, &mut stats, 1, "Hero", NOW); // 10 total
        assert_eq!(unlocked, vec![AchievementType::GreenThumb]);

        let unlocked = add_flowers(&mut data, &mut stats, 40, "Hero", NOW); // 50
        assert_eq!(unlocked, vec![AchievementType::BotanyEnthusiast]);

        let unlocked = add_flowers(&mut data, &mut stats, 150, "Hero", NOW); // 200
        assert_eq!(unlocked, vec![AchievementType::NaturesFriend]);

        let unlocked = add_flowers(&mut data, &mut stats, 300, "Hero", NOW); // 500
        assert_eq!(unlocked, vec![AchievementType::Herbalist]);

        let unlocked = add_flowers(&mut data, &mut stats, 500, "Hero", NOW); // 1000
        assert_eq!(unlocked, vec![AchievementType::MasterHerbalist]);

        assert_eq!(stats.flowers_picked, 1000);
        for ty in [
            AchievementType::GreenThumb,
            AchievementType::BotanyEnthusiast,
            AchievementType::NaturesFriend,
            AchievementType::Herbalist,
            AchievementType::MasterHerbalist,
        ] {
            assert!(data.is_unlocked(ty));
        }
    }

    #[test]
    fn add_mushrooms_reaches_top_tier() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        add_mushrooms(&mut data, &mut stats, 1000, "Hero", NOW);
        assert_eq!(stats.mushrooms_picked, 1000);
        for ty in [
            AchievementType::MushroomHunter,
            AchievementType::FungusFinder,
            AchievementType::SporeSeeker,
            AchievementType::MushroomMaster,
            AchievementType::Mycologist,
        ] {
            assert!(data.is_unlocked(ty));
        }
    }

    #[test]
    fn add_berries_reaches_top_tier() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        add_berries(&mut data, &mut stats, 1000, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::MasterGatherer));
    }

    #[test]
    fn add_potions_full_ladder_including_legendary_brewer() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        let unlocked = add_potions(&mut data, &mut stats, 1000, "Hero", NOW);
        assert_eq!(
            unlocked,
            vec![
                AchievementType::Alchemist,
                AchievementType::JourneymanBrewer,
                AchievementType::ArcaneAlchemist,
                AchievementType::GrandmasterBrewer,
                AchievementType::PotionMaster,
                AchievementType::LegendaryBrewer,
            ]
        );
    }

    #[test]
    fn add_demons_tracks_per_area_and_thresholds() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();

        add_demons(&mut data, &mut stats, 4, 50, "Hero", NOW); // Earth pents area
        assert_eq!(stats.demons_per_area[PentArea::Earth as usize], 50);
        assert_eq!(stats.demons_defeated, 50);
        assert!(!data.is_unlocked(AchievementType::FiendFighter));

        // Non-pent area: no per-area bucket, but total still counts.
        let unlocked = add_demons(&mut data, &mut stats, 1, 50, "Hero", NOW);
        assert_eq!(stats.demons_defeated, 100);
        assert_eq!(unlocked, vec![AchievementType::FiendFighter]);
        assert_eq!(stats.demons_per_area[PentArea::Earth as usize], 50);

        add_demons(&mut data, &mut stats, 1, 2400, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::Demonbane));
        add_demons(&mut data, &mut stats, 1, 12500, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::DreadDestroyer));
        add_demons(&mut data, &mut stats, 1, 235000, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::DemonicExterminator));
        assert_eq!(stats.demons_defeated, 250000);
    }

    #[test]
    fn add_pents_awards_area_specific_and_tier_achievements() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();

        let unlocked = add_pents(&mut data, &mut stats, 34, 1, "Hero", NOW); // Hell area
        assert_eq!(
            unlocked,
            vec![AchievementType::ThroughGatesOfHell, AchievementType::Solved]
        );
        assert_eq!(stats.pents_per_area[PentArea::Hell as usize], 1);
        assert_eq!(stats.pents_solved, 1);

        // Non-pent area still counts toward the tier ladder, just no area-specific award.
        let unlocked = add_pents(&mut data, &mut stats, 1, 19, "Hero", NOW); // total 20
        assert_eq!(unlocked, vec![AchievementType::FullOfSolves]);

        add_pents(&mut data, &mut stats, 1, 80, "Hero", NOW); // total 100
        assert!(data.is_unlocked(AchievementType::RuneMaster));
        add_pents(&mut data, &mut stats, 1, 400, "Hero", NOW); // total 500
        assert!(data.is_unlocked(AchievementType::GrandmasterPentagram));
    }

    #[test]
    fn add_pents_earth_fire_ice_variants() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        add_pents(&mut data, &mut stats, 4, 1, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::EarthboundNovice));
        add_pents(&mut data, &mut stats, 7, 1, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::FlameInitiate));
        add_pents(&mut data, &mut stats, 9, 1, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::FightingTheFrost));
    }

    #[test]
    fn add_chests_full_ladder() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        add_chests(&mut data, &mut stats, 500, "Hero", NOW);
        for ty in [
            AchievementType::Looter,
            AchievementType::TreasureHunter,
            AchievementType::TreasureMaster,
            AchievementType::LegendaryLooter,
        ] {
            assert!(data.is_unlocked(ty));
        }
    }

    #[test]
    fn add_stones_per_type_thresholds() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();

        let unlocked = add_stones(&mut data, &mut stats, STONE_TYPE_EARTH, 49, "Hero", NOW);
        assert!(unlocked.is_empty());
        let unlocked = add_stones(&mut data, &mut stats, STONE_TYPE_EARTH, 1, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::EarthRocks]);

        let unlocked = add_stones(&mut data, &mut stats, STONE_TYPE_FIRE, 100, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::FireRocks]);

        let unlocked = add_stones(&mut data, &mut stats, STONE_TYPE_ICE, 1000, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::IceRocks]);

        // Unknown stone type is a documented no-op (matches C's switch default).
        let unlocked = add_stones(&mut data, &mut stats, 99, 1000, "Hero", NOW);
        assert!(unlocked.is_empty());
    }

    #[test]
    fn add_enemy_killed_awards_first_blood_once() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        let unlocked = add_enemy_killed(&mut data, &mut stats, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::FirstBlood]);
        let unlocked = add_enemy_killed(&mut data, &mut stats, "Hero", NOW);
        assert!(unlocked.is_empty());
        assert_eq!(stats.enemies_killed, 2);
    }

    #[test]
    fn add_pvp_kill_awards_arena_combatant() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        let unlocked = add_pvp_kill(&mut data, &mut stats, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::ArenaCombatant]);
    }

    #[test]
    fn add_military_mission_full_ladder() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        for _ in 0..1000 {
            add_military_mission(&mut data, &mut stats, "Hero", NOW);
        }
        for ty in [
            AchievementType::Recruit,
            AchievementType::Soldier,
            AchievementType::MilitaryVeteran,
            AchievementType::Commander,
            AchievementType::General,
            AchievementType::WarLegend,
        ] {
            assert!(data.is_unlocked(ty));
        }
    }

    #[test]
    fn add_tunnel_level_full_ladder_incl_tunnel_rat() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        for _ in 0..100 {
            add_tunnel_level(&mut data, &mut stats, "Hero", NOW);
        }
        for ty in [
            AchievementType::TunnelExplorer,
            AchievementType::TunnelRunner,
            AchievementType::TunnelVeteran,
            AchievementType::TunnelRat,
        ] {
            assert!(data.is_unlocked(ty));
        }
    }

    #[test]
    fn add_silver_mined_full_ladder() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        add_silver_mined(&mut data, &mut stats, u32::MAX, "Hero", NOW);
        add_silver_mined(&mut data, &mut stats, u32::MAX, "Hero", NOW);
        assert!(stats.silver_mined >= 50_000_000);
        assert!(data.is_unlocked(AchievementType::SilverLegend));
    }

    #[test]
    fn add_gold_mined_full_ladder() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        add_gold_mined(&mut data, &mut stats, u32::MAX, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::GoldLegend));
    }

    #[test]
    fn add_gold_earned_full_ladder() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        let unlocked = add_gold_earned(&mut data, &mut stats, 10_000, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::CoinCollector]);
        add_gold_earned(&mut data, &mut stats, 90_000, "Hero", NOW); // 100,000
        assert!(data.is_unlocked(AchievementType::WealthyAdventurer));
        add_gold_earned(&mut data, &mut stats, 900_000, "Hero", NOW); // 1,000,000
        assert!(data.is_unlocked(AchievementType::RichNoble));
        add_gold_earned(&mut data, &mut stats, 9_000_000, "Hero", NOW); // 10,000,000
        assert!(data.is_unlocked(AchievementType::Millionaire));
    }

    #[test]
    fn add_play_time_full_ladder() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        add_play_time(&mut data, &mut stats, 1440, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::DedicatedPlayer));
        add_play_time(&mut data, &mut stats, 6000 - 1440, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::VeteranPlayer));
        add_play_time(&mut data, &mut stats, 30000 - 6000, "Hero", NOW);
        assert!(data.is_unlocked(AchievementType::UgarisLifer));
    }

    #[test]
    fn check_login_streak_first_login_sets_streak_one() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        let day0 = 100 * 86400;
        check_login_streak(&mut data, &mut stats, "Hero", day0);
        assert_eq!(stats.login_streak, 1);
        assert_eq!(stats.last_login_day, 100);
    }

    #[test]
    fn check_login_streak_same_day_is_a_no_op() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        let day0 = 100 * 86400;
        check_login_streak(&mut data, &mut stats, "Hero", day0);
        check_login_streak(&mut data, &mut stats, "Hero", day0 + 3600);
        assert_eq!(stats.login_streak, 1);
    }

    #[test]
    fn check_login_streak_consecutive_days_increment() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        let day0 = 100 * 86400;
        for i in 0..7 {
            check_login_streak(&mut data, &mut stats, "Hero", day0 + i * 86400);
        }
        assert_eq!(stats.login_streak, 7);
        assert!(data.is_unlocked(AchievementType::Regular));
    }

    #[test]
    fn check_login_streak_gap_resets_to_one() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        let day0 = 100 * 86400;
        check_login_streak(&mut data, &mut stats, "Hero", day0);
        check_login_streak(&mut data, &mut stats, "Hero", day0 + 1 * 86400);
        assert_eq!(stats.login_streak, 2);
        // Skip a day (gap of 2 days instead of 1) -> streak resets.
        check_login_streak(&mut data, &mut stats, "Hero", day0 + 3 * 86400);
        assert_eq!(stats.login_streak, 1);
    }

    #[test]
    fn check_login_streak_committed_and_devoted_thresholds() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        let day0 = 100 * 86400;
        for i in 0..100 {
            check_login_streak(&mut data, &mut stats, "Hero", day0 + i * 86400);
        }
        assert_eq!(stats.login_streak, 100);
        assert!(data.is_unlocked(AchievementType::Committed));
        assert!(data.is_unlocked(AchievementType::Devoted));
    }

    #[test]
    fn check_level_standard_thresholds() {
        let mut data = AccountAchievements::default();
        for level in [10, 20, 50, 75, 100, 150, 200] {
            check_level(&mut data, level, false, "Hero", NOW);
        }
        for ty in [
            AchievementType::RisingBeginner,
            AchievementType::ExperiencedHero,
            AchievementType::UgarisVeteran,
            AchievementType::LegendaryAdventurer,
            AchievementType::DemonSlayer,
            AchievementType::MasterOfHell,
            AchievementType::MasterOfUgaris,
        ] {
            assert!(data.is_unlocked(ty));
        }
        assert!(!data.is_unlocked(AchievementType::HardcoreHero));
        assert!(!data.is_unlocked(AchievementType::HardcoreLegend));
    }

    #[test]
    fn check_level_hardcore_variants_only_awarded_when_hardcore() {
        let mut data = AccountAchievements::default();
        let unlocked = check_level(&mut data, 50, true, "Hero", NOW);
        assert!(unlocked.contains(&AchievementType::UgarisVeteran));
        assert!(unlocked.contains(&AchievementType::HardcoreHero));

        let unlocked = check_level(&mut data, 100, true, "Hero", NOW);
        assert!(unlocked.contains(&AchievementType::HardcoreLegend));
    }

    #[test]
    fn check_skill_weapon_range_covers_dagger_through_twohand() {
        let mut data = AccountAchievements::default();
        let unlocked = check_skill(&mut data, V_DAGGER, 10, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::WeaponNovice]);
        let unlocked = check_skill(&mut data, V_TWOHAND, 110, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::MasterOfArms]);
        // Out of range: V_ARMORSKILL (17) is not a weapon skill.
        let unlocked = check_skill(&mut data, 17, 110, "Hero", NOW);
        assert!(unlocked.is_empty());
    }

    #[test]
    fn check_skill_magic_ladder() {
        let mut data = AccountAchievements::default();
        let unlocked = check_skill(&mut data, V_FIRE, 10, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::ApprenticeMagic]);
        let unlocked = check_skill(&mut data, V_FLASH, 50, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::IntermediateMagic]);
        let unlocked = check_skill(&mut data, V_FIRE, 110, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::MasterOfMagic]);
    }

    #[test]
    fn check_skill_fighting_ladder() {
        let mut data = AccountAchievements::default();
        let unlocked = check_skill(&mut data, V_ATTACK, 10, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::ApprenticeFighting]);
        let unlocked = check_skill(&mut data, V_PARRY, 50, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::IntermediateFighting]);
        let unlocked = check_skill(&mut data, V_ATTACK, 110, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::MasterOfFighting]);
    }

    #[test]
    fn check_profession_every_branch() {
        let cases = [
            (P_ATHLETE, 30, AchievementType::MasterAthlete),
            (P_ALCHEMIST, 50, AchievementType::MasterAlchemist),
            (P_MINER, 20, AchievementType::MasterMiner),
            (P_ASSASSIN, 50, AchievementType::MasterAssassin),
            (P_THIEF, 30, AchievementType::MasterThief),
            (P_LIGHT, 30, AchievementType::MasterLightWarrior),
            (P_DARK, 30, AchievementType::MasterDarkWarrior),
            (P_TRADER, 20, AchievementType::MasterTrader),
            (P_MERCENARY, 20, AchievementType::MasterMercenary),
            (P_CLAN, 30, AchievementType::MasterClanWarrior),
            (P_HERBALIST, 30, AchievementType::MasterHerbalistProf),
        ];
        for (prof_type, threshold, ty) in cases {
            let mut data = AccountAchievements::default();
            let unlocked = check_profession(&mut data, prof_type, threshold - 1, "Hero", NOW);
            assert!(
                unlocked.is_empty(),
                "unexpected unlock below threshold for {ty:?}"
            );
            let unlocked = check_profession(&mut data, prof_type, threshold, "Hero", NOW);
            assert_eq!(unlocked, vec![ty]);
        }
    }

    #[test]
    fn check_profession_unknown_type_is_a_no_op() {
        let mut data = AccountAchievements::default();
        let unlocked = check_profession(&mut data, 99, 1000, "Hero", NOW);
        assert!(unlocked.is_empty());
    }

    #[test]
    fn check_exploration_awards_only_for_aston() {
        let mut data = AccountAchievements::default();
        let unlocked = check_exploration(&mut data, 1, "Hero", NOW);
        assert!(unlocked.is_empty());
        let unlocked = check_exploration(&mut data, AREA_ASTON, "Hero", NOW);
        assert_eq!(unlocked, vec![AchievementType::UgarisPathfinder]);
    }

    #[test]
    fn clear_all_resets_both_data_and_stats() {
        let mut data = AccountAchievements::default();
        let mut stats = AchievementStats::default();
        data.award(AchievementType::FirstBlood, "Hero", NOW);
        stats.flowers_picked = 42;
        clear_all(&mut data, &mut stats);
        assert!(!data.is_unlocked(AchievementType::FirstBlood));
        assert_eq!(stats.flowers_picked, 0);
    }

    #[test]
    fn achieved_by_name_is_truncated_to_39_chars_matching_c_buffer() {
        let mut data = AccountAchievements::default();
        let long_name: String = "A".repeat(60);
        data.award(AchievementType::FirstBlood, &long_name, NOW);
        assert_eq!(
            data.achievements[AchievementType::FirstBlood as usize]
                .achieved_by
                .len(),
            39
        );
    }

    #[test]
    fn account_achievements_json_roundtrip_preserves_all_128_slots() {
        let mut data = AccountAchievements::default();
        data.award(AchievementType::FirstBlood, "Hero", NOW);
        data.add_progress(AchievementType::DemonSlayer, 3, "Hero", NOW);
        let last = MAX_ACHIEVEMENTS - 1;
        data.achievements[last].progress = 7;

        let json = serde_json::to_string(&data).expect("serialize");
        let restored: AccountAchievements = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, data);
        assert!(restored.is_unlocked(AchievementType::FirstBlood));
        assert_eq!(restored.achievements[last].progress, 7);
    }

    #[test]
    fn account_achievements_deserializes_from_short_legacy_array() {
        // Simulate an older/short PPD blob: only the first few slots
        // present. Missing trailing slots must fall back to
        // `Achievement::default()` instead of erroring.
        let json = r#"{"version":1,"achievements":[
            {"timestamp":5,"progress":1,"target":1,"achieved_by":"Hero"}
        ]}"#;
        let restored: AccountAchievements = serde_json::from_str(json).expect("deserialize");
        assert_eq!(restored.achievements[0].timestamp, 5);
        assert_eq!(restored.achievements[1], Achievement::default());
        assert_eq!(
            restored.achievements[MAX_ACHIEVEMENTS - 1],
            Achievement::default()
        );
    }

    #[test]
    fn achievement_stats_json_roundtrip() {
        let mut stats = AchievementStats::default();
        stats.flowers_picked = 10;
        stats.demons_per_area = [1, 2, 3, 4];
        stats.gold_earned = 123_456_789;

        let json = serde_json::to_string(&stats).expect("serialize");
        let restored: AchievementStats = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, stats);
    }
}
