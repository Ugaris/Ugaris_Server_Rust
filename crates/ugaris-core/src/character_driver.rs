//! Static character-driver registry boundary for legacy `ch_driver` dispatch.
//!
//! The C server dynamically probes module libraries. The Rust rewrite keeps the
//! same numeric compatibility at the registry edge while routing known drivers
//! to typed outcomes that can be filled in incrementally.

use crate::{
    entity::{Character, CharacterFlags, CharacterValue, Item, INVENTORY_SIZE, POWERSCALE},
    ids::{CharacterId, ItemId},
    item_driver::IDR_POTION,
};
pub const CDT_DRIVER: u16 = 0;
pub const CDT_ITEM: u16 = 1;
pub const CDT_DEAD: u16 = 2;
pub const CDT_RESPAWN: u16 = 3;
pub const CDT_SPECIAL: u16 = 4;
/// C `#define CDR_ACLERK 4` (`src/system/drvlib.h`): the arena clerk in
/// Cameron (`src/module/merchants/merchant.c::aclerk_driver`).
pub const CDR_ACLERK: u16 = 4;
pub const CDR_LOSTCON: u16 = 5;
pub const CDR_MERCHANT: u16 = 6;
pub const CDR_SIMPLEBADDY: u16 = 7;
/// C `#define CDR_BANK 22` (`src/system/drvlib.h`): generic bank driver.
pub const CDR_BANK: u16 = 22;
pub const CDR_MACRO: u16 = 37;
pub const CDR_SWAMPCLARA: u16 = 54;
pub const CDR_SWAMPMONSTER: u16 = 56;
pub const CDR_PALACEISLENA: u16 = 57;
pub const CDR_TWOSKELLY: u16 = 70;
pub const CDR_TRADER: u16 = 72;
pub const CDR_LQNPC: u16 = 74;
pub const CDR_JANITOR: u16 = 85;
pub const CDR_TEUFELDEMON: u16 = 114;
pub const CDR_TEUFELGAMBLER: u16 = 115;
pub const CDR_TEUFELQUEST: u16 = 116;
pub const CDR_TEUFELRAT: u16 = 117;
pub const CDR_CALIGARSKELLY: u16 = 124;
pub const CDR_LAB2UNDEAD: u16 = 198;
/// C `#define CDR_CAMHERMIT 14` (`src/system/drvlib.h`): the forest
/// hermit NPC in area 1 (`src/area/1/gwendylon.c::camhermit_driver`).
pub const CDR_CAMHERMIT: u16 = 14;
/// C `#define CDR_YOAKIN 9` (`src/system/drvlib.h`): the area-1 hunter
/// quest giver at the knight castle (`src/area/1/gwendylon.c::
/// yoakin_driver`).
pub const CDR_YOAKIN: u16 = 9;
/// C `#define CDR_TERION 11` (`src/system/drvlib.h`): the ambient lore NPC
/// in area 1's village (`src/area/1/gwendylon.c::terion_driver`).
pub const CDR_TERION: u16 = 11;
/// C `#define CDR_GWENDYLON 8` (`src/system/drvlib.h`): the area-1 main
/// quest-giver mage at the knight castle
/// (`src/area/1/gwendylon.c::gwendylon_driver`).
pub const CDR_GWENDYLON: u16 = 8;
/// C `#define CDR_GREETER 13` (`src/system/drvlib.h`): the "specific NPC
/// in area1, stronghold" (Cameron, the tutorial-town Governor) greeting
/// NPC (`src/area/1/gwendylon.c::greeter_driver`).
pub const CDR_GREETER: u16 = 13;
/// C `#define CDR_JESSICA 125` (`src/system/drvlib.h`, "Cameron: robbers"):
/// the area-1 robber-operations quest NPC
/// (`src/area/1/gwendylon.c::jessica_driver`).
pub const CDR_JESSICA: u16 = 125;
/// C `#define CDR_JIU 127` (`src/system/drvlib.h`, "Cameron: Jiu"): the
/// riverbeast quest-giving pilgrim NPC
/// (`src/area/1/gwendylon.c::jiu_driver`).
pub const CDR_JIU: u16 = 127;
/// C `#define CDR_RIVERBEAST 128` (`src/system/drvlib.h`, "Cameron:
/// Riverbeast (Jiu Quest)"): the killable beast whose death
/// (`src/area/1/gwendylon.c::riverbeast_dead`, `:2255-2272`) advances
/// `CDR_JIU`'s quest chain from `JIU_STATE_WAIT_FOR_KILL` to
/// `JIU_STATE_BEAST_KILLED`. See
/// `crate::world::jiu`/`ugaris-server::world_events::
/// apply_riverbeast_death_from_hurt_event`.
pub const CDR_RIVERBEAST: u16 = 128;
/// C `#define CDR_CAMERON_FORESTMONSTER 129` (`src/system/drvlib.h`,
/// "Cameron: Mobs for stone circle"): area 1's forest bear monster, whose
/// death (`src/area/1/gwendylon.c::monster_dead`, `:5201-5231`) increments
/// `CDR_CAMHERMIT`'s `camhermit_kills` counter and separately re-glows a
/// worn weapon at noon in the stone-circle area. See
/// `ugaris-server::world_events::apply_area1_monster_death_from_hurt_event`
/// and `World::apply_area1_monster_death_driver`.
pub const CDR_CAMERON_FORESTMONSTER: u16 = 129;
/// C `#define CDR_BREDEL 154` (`src/system/drvlib.h`, "Cameron: Bredel
/// driver"): the robber-operations boss whose death
/// (`src/area/1/gwendylon.c::bredel_dead`, `:2825-2842`) advances
/// `CDR_JESSICA`'s quest chain from `JESSICA_STATE_QUEST2_DO` to
/// `JESSICA_STATE_QUEST2_FINISH`. See
/// `crate::world::jessica`/`ugaris-server::world_events::
/// apply_bredel_death_from_hurt_event`.
pub const CDR_BREDEL: u16 = 154;
/// C `#define CDR_FOREST_RANGER 155` (`src/system/drvlib.h`, "Cameron:
/// Forest Ranger (warns from bigbadspider)"): the stationary
/// bear-attack-warning sentry near area 1's stone circle
/// (`src/area/1/gwendylon.c::forest_ranger_driver`).
pub const CDR_FOREST_RANGER: u16 = 155;
/// C `#define CDR_BRITHILDIE 126` (`src/system/drvlib.h`, "Cameron:
/// Brithildie"): the Governor's-mother ambient lore NPC
/// (`src/area/1/gwendylon.c::brithildie_driver`) who unlocks
/// `QLOG_BRITHILDIE`.
pub const CDR_BRITHILDIE: u16 = 126;
/// C `#define CDR_BIGBADSPIDER 153` (`src/system/drvlib.h`, "Cameron:
/// BigBadSpider driver"): the killable spider whose death
/// (`src/area/1/gwendylon.c::bigbadspider_dead`, `:2850-2870`) completes
/// `CDR_BRITHILDIE`'s `QLOG_BRITHILDIE` quest, advancing
/// `BRITHILDIE_STATE_NOMORETALES_QOPEN` to `_QDONE`. Dispatched through
/// `CDR_SIMPLEBADDY` for its own combat driver (`gwendylon.c:6138`), same
/// as `CDR_RIVERBEAST`/`CDR_BREDEL`/`CDR_CAMERON_FORESTMONSTER` above. See
/// `ugaris-server::world_events::apply_bigbadspider_death_from_hurt_event`.
pub const CDR_BIGBADSPIDER: u16 = 153;
/// C `#define CDR_JAMES 12` (`src/system/drvlib.h`): "specific NPC in
/// area1, knight castle" - the town drunkard's Lydia-quest hand-off/
/// hardcore-recruiter/paid-advice NPC (`src/area/1/gwendylon.c::
/// james_driver`).
pub const CDR_JAMES: u16 = 12;
/// C `#define CDR_NOOK 15` (`src/system/drvlib.h`): "specific NPC in
/// area1, knight castle" - the identity-crisis jester/judge/knight NPC
/// (`src/area/1/gwendylon.c::nook_driver`).
pub const CDR_NOOK: u16 = 15;
/// C `#define CDR_LYDIA 16` (`src/system/drvlib.h`): "specific NPC in
/// area1, knight castle" - Gwendylon's hungover daughter and her
/// hangover-potion quest chain (`src/area/1/gwendylon.c::lydia_driver`).
pub const CDR_LYDIA: u16 = 16;
/// C `#define CDR_GATE_WELCOME 39` (`src/system/drvlib.h`): the stationary
/// gatekeeper-welcome NPC (`gate_welcome` template,
/// `src/system/gatekeeper.c::gate_welcome_driver`).
pub const CDR_GATE_WELCOME: u16 = 39;
/// C `#define CDR_ROBBER 17` (`src/system/drvlib.h`, "specific enemy in
/// area1, wood"): the midnight-meeting forest patrol NPC
/// (`src/area/1/gwendylon.c::robber_driver`).
pub const CDR_ROBBER: u16 = 17;
/// C `#define CDR_SANOA 18` (`src/system/drvlib.h`, "specific NPC (enemy)
/// in area1, city"): the ambient twelve-waypoint city walker
/// (`src/area/1/gwendylon.c::sanoa_driver`), no dialogue at all.
pub const CDR_SANOA: u16 = 18;
/// C `#define CDR_BALLTRAP 10` (`src/system/drvlib.h`, "specific enemy,
/// area1, wood"): the stationary ball-trap-mechanism guard skeleton
/// (`src/area/1/gwendylon.c::balltrap_skelly_driver`).
pub const CDR_BALLTRAP: u16 = 10;
/// C `#define CDR_ASTURIN 19` (`src/system/drvlib.h`, "specific NPC
/// (enemy) in area1, city"): the private-quarters guard NPC
/// (`src/area/1/gwendylon.c::asturin_driver`).
pub const CDR_ASTURIN: u16 = 19;
/// C `#define CDR_RESKIN 20` (`src/system/drvlib.h`, "specific NPC in
/// area1, city"): the tavern-keeper/alchemy-turn-in NPC
/// (`src/area/1/gwendylon.c::reskin_driver`).
pub const CDR_RESKIN: u16 = 20;
/// C `#define CDR_GUIWYNN 21` (`src/system/drvlib.h`, "specific quest
/// giver, area1, city"): the town-mage NPC running the two-part "Order of
/// Mages" investigation quest chain (`src/area/1/gwendylon.c::
/// guiwynn_driver`).
pub const CDR_GUIWYNN: u16 = 21;
/// C `#define CDR_LOGAIN 23` (`src/system/drvlib.h`, "specific quest
/// giver, area1, city"): the retired knight-trainer's mystery-quest
/// dialogue chain (`src/area/1/gwendylon.c::logain_driver`) - the last
/// driver in `ch_driver`'s dispatch table (`gwendylon.c:6076-6155`).
pub const CDR_LOGAIN: u16 = 23;
/// C `#define CDR_CLANMASTER 27` (`src/system/drvlib.h`): the clan
/// foundations NPC (`src/area/30/clanmaster.c::clanmaster_driver`).
pub const CDR_CLANMASTER: u16 = 27;
/// C `#define CDR_CLANCLERK 28` (`src/system/drvlib.h`): the clan
/// administration/treasury NPC (`src/area/30/clanmaster.c::clanclerk_driver`).
pub const CDR_CLANCLERK: u16 = 28;
/// C `#define CDR_CLUBMASTER 113` (`src/system/drvlib.h`): the club
/// foundations/administration NPC (`src/system/clubmaster.c::
/// clubmaster_driver`) - a single driver combining what `CDR_CLANMASTER`/
/// `CDR_CLANCLERK` split into two separate NPCs. See `crate::club`'s
/// module doc comment for the club/clan split, and
/// `crate::world::clubmaster` for the port itself.
pub const CDR_CLUBMASTER: u16 = 113;
/// C `#define CDR_GATE_FIGHT 40` (`src/system/drvlib.h`): the private-room
/// opponent NPC spawned by `enter_room` (`gatekeeper_w`/`gatekeeper_m`/
/// `gatekeeper_s` templates, `src/system/gatekeeper.c::gate_fight_driver`).
pub const CDR_GATE_FIGHT: u16 = 40;
/// C `#define CDR_MILITARY_MASTER 42` (`src/system/drvlib.h`): the
/// mission-giving Military Master NPC (`src/module/military.c::
/// military_master_driver`).
pub const CDR_MILITARY_MASTER: u16 = 42;
/// C `#define CDR_MILITARY_ADVISOR 43` (`src/system/drvlib.h`): the paid
/// mission-recommendation NPC (`src/module/military.c::
/// military_advisor_driver`).
pub const CDR_MILITARY_ADVISOR: u16 = 43;
/// C `#define CDR_ARENAMASTER 48` (`src/system/drvlib.h`): the arena
/// tournament master NPC (`src/system/arena.c::master_driver`) - pairs
/// registered contenders, watches the fight, and scores the result. See
/// the "Arena rankings" P3 task in `PORTING_TODO.md`.
pub const CDR_ARENAMASTER: u16 = 48;
/// C `#define CDR_ARENAFIGHTER 49` (`src/system/drvlib.h`): the
/// autonomous tournament "fighter" bot (`arena.c::fighter_driver`) that
/// registers itself, enters, and fights via the generic `fight_driver_*`
/// helpers (narrowed here to a single tracked enemy, same simplification
/// as `CDR_GATE_FIGHT` - see `world/arena.rs`'s `process_arena_fighter_actions`).
pub const CDR_ARENAFIGHTER: u16 = 49;
/// C `#define CDR_ARENAMANAGER 50` (`src/system/drvlib.h`): the
/// arena-rental NPC (`arena.c::manager_driver`, `rent`/`invite:`/`enter`/
/// `leave` commands - despite the "paid" name, C's own `manager_driver`
/// never touches gold at all). See `world/arena.rs`'s
/// `process_arena_manager_actions`.
pub const CDR_ARENAMANAGER: u16 = 50;
/// C `#define CDR_DUNGEONMASTER 51` (`src/system/drvlib.h`): the clan-raid
/// catacomb reception NPC (`src/area/13/dungeon.c::dungeonmaster`) -
/// `attack <nr>`/`enter <nr>`/`list`/(GM-only) `destroy <nr>` text
/// commands, the per-slot expiry/warning tick, and the greeting. See
/// `world/dungeon_master.rs`'s `process_dungeonmaster_actions`.
pub const CDR_DUNGEONMASTER: u16 = 51;
/// C `#define CDR_DUNGEONFIGHTER 52` (`src/system/drvlib.h`): the
/// autonomous raid-boss combat driver (`dungeon.c::dungeonfighter`/
/// `dungeon_potion`/`fighter_dead`, `dungeon.c:1956-2161`) spawned inside
/// a live catacomb. The message-loop/potion half is ported - see
/// `world/dungeon_fighter.rs`'s `process_dungeonfighter_actions`; its own
/// module doc comment lists what's still REMAINING (the SimpleBaddy-AI
/// tail call and `fighter_dead`).
pub const CDR_DUNGEONFIGHTER: u16 = 52;
/// C `#define CDR_SUPERIOR 26` (`src/system/drvlib.h`, "superior zombie
/// driver in area 2"): the four named crypt guardians
/// (`src/area/2/area2.c::superior_driver`).
pub const CDR_SUPERIOR: u16 = 26;
/// C `#define CDR_MOONIE 31` (`src/system/drvlib.h`, "stop to eat
/// spiders"): the spider-eating companion NPC
/// (`src/area/2/area2.c::moonie_driver`).
pub const CDR_MOONIE: u16 = 31;
/// C `#define CDR_VAMPIRE 34` (`src/system/drvlib.h`, "special NPC in area
/// 2"): the Vampire Lord boss NPC (`src/area/2/area2.c::vampire_driver`).
pub const CDR_VAMPIRE: u16 = 34;
/// C `#define CDR_VAMPIRE2 38` (`src/system/drvlib.h`, "special NPC in
/// area 2"): the Vampire Lord 2 boss NPC
/// (`src/area/2/area2.c::vampire2_driver`).
pub const CDR_VAMPIRE2: u16 = 38;
pub const DRD_SIMPLEBADDYDRIVER: u32 = 0x0100_0013;
pub const DRD_CLARADRIVER: u32 = 0x0100_0059;
pub const DRD_SKELLYDRIVER: u32 = 0x0100_006a;
pub const DRD_LAB2_UNDEAD: u32 = 0x0200_0001;
pub const NT_CHAR: i32 = 1;
pub const NT_ITEM: i32 = 2;
pub const NT_GOTHIT: i32 = 3;
pub const NT_DIDHIT: i32 = 4;
pub const NT_SEEHIT: i32 = 5;
pub const NT_DEAD: i32 = 6;
pub const NT_SPELL: i32 = 7;
pub const NT_GIVE: i32 = 8;
pub const NT_CREATE: i32 = 9;
pub const NT_TEXT: i32 = 200;
pub const NT_NPC: i32 = 300;
pub const NTID_MERCHANT: i32 = 1;
pub const NTID_TERION: i32 = 2;
pub const NTID_ASTURIN: i32 = 3;
pub const NTID_GATEKEEPER: i32 = 4;
pub const NTID_DIDSAY: i32 = 5;
pub const NTID_TUTORIAL: i32 = 6;
pub const NTID_PALACE_ALERT: i32 = 7;
pub const NTID_ARENA: i32 = 8;
pub const NTID_DUNGEON: i32 = 9;
pub const NTID_TWOCITY: i32 = 10;
pub const NTID_TWOCITY_PICK: i32 = 11;
pub const NTID_DICE: i32 = 12;
pub const NTID_LABGNOMETORCH: i32 = 13;
pub const NTID_LAB2_DEAMONCHECK: i32 = 14;
pub const NTID_SALTMINE_USEITEM: i32 = 15;
pub const NTID_GLADIATOR: i32 = 16;
pub const NTID_FDEMON: i32 = 17;
pub const FDEMON_MSG_WAYPOINT: i32 = 1;
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CharacterDriverMessage {
    pub message_type: i32,
    pub dat1: i32,
    pub dat2: i32,
    pub dat3: i32,
    #[serde(default)]
    pub text: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CharacterDriverState {
    SimpleBaddy(SimpleBaddyDriverData),
    Clara(ClaraDriverData),
    TwoSkelly(TwoSkellyDriverData),
    Lab2Undead(Lab2UndeadDriverData),
    Merchant(MerchantDriverData),
    Aclerk(AclerkDriverData),
    Lostcon(LostconDriverData),
    Bank(BankDriverData),
    Trader(TraderDriverData),
    Janitor(JanitorDriverData),
    GateWelcome(GateWelcomeDriverData),
    GateFight(GateFightDriverData),
    Clanmaster(ClanmasterDriverData),
    /// C `struct clan_found_data` (`src/area/30/clanmaster.c:288-292`),
    /// stored via `set_data(co, DRD_CLANFOUND, ...)` on the *player*
    /// being talked to, not on the clanmaster NPC itself. Reusing the
    /// same `driver_state` slot for a player character is a new case for
    /// this codebase (every prior `CharacterDriverState` variant belongs
    /// to an NPC) but is safe: no other feature currently reads or writes
    /// a player's `driver_state`, and C's own `set_data` is likewise just
    /// a per-character named-slot store with no NPC-only restriction.
    ClanFound(ClanFoundData),
    Clanclerk(ClanclerkDriverData),
    Clubmaster(ClubmasterDriverData),
    MilitaryMaster(MilitaryMasterDriverData),
    MilitaryAdvisor(MilitaryAdvisorDriverData),
    ArenaMaster(ArenaMasterDriverData),
    ArenaFighter(ArenaFighterDriverData),
    ArenaManager(ArenaManagerDriverData),
    Dungeonmaster(DungeonmasterDriverData),
    Dungeonfighter(DungeonfighterDriverData),
    Macro(MacroDriverData),
    Camhermit(CamhermitDriverData),
    Yoakin(YoakinDriverData),
    Terion(TerionDriverData),
    Gwendylon(GwendylonDriverData),
    Greeter(GreeterDriverData),
    Jessica(JessicaDriverData),
    Jiu(JiuDriverData),
    ForestRanger(ForestRangerDriverData),
    Brithildie(BrithildieDriverData),
    Nook(NookDriverData),
    Lydia(LydiaDriverData),
    Robber(RobberDriverData),
    Sanoa(SanoaDriverData),
    Asturin(AsturinDriverData),
    Reskin(ReskinDriverData),
    Guiwynn(GuiwynnDriverData),
    James(JamesDriverData),
    Balltrap(BalltrapDriverData),
    Logain(LogainDriverData),
    Superior(SuperiorDriverData),
    Moonie(MoonieDriverData),
    Vampire(VampireDriverData),
    Vampire2(Vampire2DriverData),
}
/// C `bank_driver_parse` from `src/module/bank.c`. The C driver defaults
/// opening hours to 6..23 before parsing (`bank_driver` lines 304-309).
pub fn parse_bank_driver_args(args: &str) -> BankDriverData {
    let mut data = BankDriverData {
        open: 6,
        close: 23,
        ..BankDriverData::default()
    };
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "dir" => data.dir = parsed,
            "dayx" => data.dayx = parsed,
            "dayy" => data.dayy = parsed,
            "daydir" => data.daydir = parsed,
            "nightx" => data.nightx = parsed,
            "nighty" => data.nighty = parsed,
            "nightdir" => data.nightdir = parsed,
            "storefx" => data.storefx = parsed,
            "storefy" => data.storefy = parsed,
            "storetx" => data.storetx = parsed,
            "storety" => data.storety = parsed,
            "doorx" => data.doorx = parsed,
            "doory" => data.doory = parsed,
            "open" => data.open = parsed,
            "close" => data.close = parsed,
            _ => {}
        }
        rest = next;
    }
    data
}
/// C `#define MS_PAIR 0` (`arena.c:222`): searching for a contender pair.
pub const MS_PAIR: u8 = 0;
/// C `#define MS_IN 1` (`arena.c:223`): waiting for both fighters to step
/// into the arena box.
pub const MS_IN: u8 = 1;
/// C `#define MS_FIGHT 2` (`arena.c:224`): fight in progress.
pub const MS_FIGHT: u8 = 2;
/// C `#define FS_LEISURE 0` ... `#define FS_FIGHT 6` (`arena.c:790-796`):
/// `fighter_driver`'s (`CDR_ARENAFIGHTER`) autonomous tournament
/// practice-bot state machine.
pub const FS_LEISURE: u8 = 0;
pub const FS_START: u8 = 1;
pub const FS_REGISTER: u8 = 2;
pub const FS_WAIT: u8 = 3;
pub const FS_ENTER: u8 = 4;
pub const FS_WAIT2: u8 = 5;
pub const FS_FIGHT: u8 = 6;
/// C `struct clan_found_data` (`src/area/30/clanmaster.c:288-292`), stored
/// on the *player* who is in the middle of founding a clan (see
/// [`CharacterDriverState::ClanFound`]'s doc comment for why this lives on
/// the player, not the NPC).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClanFoundData {
    /// C `dat->state`: `0` nothing pending, `1` a name has been chosen and
    /// is waiting for a Clan Jewel to be handed over.
    pub state: i32,
    /// C `dat->nr`: the newly founded clan's number, filled in by
    /// `found_clan` once the Clan Jewel is handed over.
    pub nr: u16,
    pub name: String,
}
//-----------------------
// Generic NPC small-talk keyword matcher.
//
// C `analyse_text_driver` is duplicated near-verbatim across
// `src/module/merchants/merchant.c`, `src/area/1/gwendylon.c`,
// `src/module/bank.c`, `src/module/base.c`, `src/module/military.c`,
// `src/area/16/forest.c`, `src/area/3/area3.c`, `src/area/37/arkhata.c` and
// `src/module/orbbank/orb_bank_npc.c`. Every copy shares the same core:
// tokenize the spoken text into lowercase words (splitting on
// `' ' ',' ':' '?' '!' '"' '.'`), drop any word equal to the NPC's own
// name (`strcasecmp(wordlist[w], ch[cn].name)`), then scan a `struct qa`
// table in order for the first entry whose word pattern matches the
// tokenized message *exactly* (same word count, same words in order -
// the C inner loop only reports a hit when `n == w && !qa[q].word[n]`,
// i.e. both the message and the pattern run out of words together).
//
// C's tokenizer is fed the *full* formatted log line (`"Name says:
// \"text\""`) and skips a leading `alpha+space+alpha+':'+space+'"'`
// prefix to strip the speaker name/verb before splitting into words; the
// Rust driver messages (`push_driver_text_message`) already carry only
// the bare spoken text, so that prefix-skip has no equivalent here.
// C also never flushes the last accumulated word unless a delimiter
// follows it - harmless in C because the trailing `'"'` of the quoted
// log line always supplies one. Since our `text` has no such trailing
// quote, we flush the final word unconditionally to keep the same
// user-visible matching behavior.

/// One `struct qa` row shared by every `analyse_text_driver` copy.
#[derive(Debug, Clone, Copy)]
pub struct TextQaEntry {
    /// Lowercase word pattern (`qa[q].word[..]`), matched for an exact
    /// (same length, same order) hit against the tokenized message.
    pub words: &'static [&'static str],
    /// `qa[q].answer`: a canned reply template fed to
    /// `quiet_say(cn, answer, ch[co].name, ch[cn].name)`. `%s` placeholders
    /// are substituted in order: speaker name, then the NPC's own name.
    pub answer: Option<&'static str>,
    /// `qa[q].answer_code`: reported back to the caller when `answer` is
    /// `None`, for area-specific dialogue branches to interpret.
    pub answer_code: i32,
}
/// Result of [`analyse_text_qa`], mirroring the two ways C
/// `analyse_text_driver` reports a qa-table hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextAnalysisOutcome {
    /// Matched an entry with a canned `answer` template; text already has
    /// `%s` placeholders substituted - the caller should `quiet_say` it.
    Said(String),
    /// Matched an entry with `answer: None`; carries `answer_code` for the
    /// caller to interpret.
    Matched(i32),
    /// No qa entry matched the tokenized message (including the case of
    /// an empty word list, matching C's `if (w) { ... }` guard).
    NoMatch,
}
/// Tokenizes spoken `text` into lowercase words the way every
/// `analyse_text_driver` copy does: split on `' ' ',' ':' '?' '!' '"'
/// '.'`, drop words equal to `own_name` (`strcasecmp`), cap at 20 words
/// (`if (w < 20) w++`), and bail out (returning `None`) if any single
/// word exceeds 250 bytes (`if (n > 250) return 0;`).
pub fn tokenize_text_words(text: &str, own_name: &str) -> Option<Vec<String>> {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, words: &mut Vec<String>| {
        if !current.is_empty() {
            let lower = current.to_ascii_lowercase();
            if !lower.eq_ignore_ascii_case(own_name) && words.len() < 20 {
                words.push(lower);
            }
            current.clear();
        }
    };
    for c in text.chars() {
        match c {
            ' ' | ',' | ':' | '?' | '!' | '"' | '.' => flush(&mut current, &mut words),
            _ => {
                current.push(c);
                if current.len() > 250 {
                    return None;
                }
            }
        }
    }
    flush(&mut current, &mut words);
    Some(words)
}
/// Substitutes `%s` placeholders in a qa `answer` template: the first
/// with `speaker_name`, the second with `own_name`, matching C's
/// `quiet_say(cn, qa[q].answer, ch[co].name, ch[cn].name)`.
fn format_qa_answer(template: &str, speaker_name: &str, own_name: &str) -> String {
    let mut args = [speaker_name, own_name].into_iter();
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' && chars.peek() == Some(&'s') {
            chars.next();
            out.push_str(args.next().unwrap_or(""));
        } else {
            out.push(c);
        }
    }
    out
}
/// C `analyse_text_driver`'s shared tokenize-and-match core. Callers are
/// responsible for the guard clauses that precede tokenization in C
/// (ignore system/info log messages, ignore our own talk, ignore
/// non-players, distance and visibility checks) since those need access
/// to `World` state this module does not have.
pub fn analyse_text_qa(
    text: &str,
    own_name: &str,
    speaker_name: &str,
    qa: &[TextQaEntry],
) -> TextAnalysisOutcome {
    let Some(words) = tokenize_text_words(text, own_name) else {
        return TextAnalysisOutcome::NoMatch;
    };
    if words.is_empty() {
        return TextAnalysisOutcome::NoMatch;
    }
    for entry in qa {
        if entry.words.len() == words.len()
            && entry
                .words
                .iter()
                .zip(words.iter())
                .all(|(pattern, word)| pattern.eq_ignore_ascii_case(word))
        {
            return match entry.answer {
                Some(template) => {
                    TextAnalysisOutcome::Said(format_qa_answer(template, speaker_name, own_name))
                }
                None => TextAnalysisOutcome::Matched(entry.answer_code),
            };
        }
    }
    TextAnalysisOutcome::NoMatch
}
/// C `struct qa qa[]` from `src/system/gatekeeper.c:83-112`
/// (`gate_welcome_driver`'s small-talk plus the class-choice answer
/// codes). Unlike [`MERCHANT_QA`]/[`TRADER_QA`], every row past `"nay"`
/// carries `answer: NULL` and a distinct `answer_code` the caller must
/// interpret: `2` repeat/restart (resets `welcome_state` to `0`), `3`
/// aye, `4` nay, `5`/`6`/`7`/`8` the Arch-Warrior/Arch-Mage/Arch-Seyan'Du/
/// Seyan'Du class choice fed to `enter_test`, and `9` reset (deletes
/// `DRD_LAB_PPD` for `CF_GOD` speakers). Word patterns are copied
/// verbatim; C's tokenizer only splits on `' ' ',' ':' '?' '!' '"' '.'`
/// so `"arch-warrior"`/`"seyan'du"` stay single tokens (hyphen and
/// apostrophe are not delimiters).
pub const GATEKEEPER_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm fine!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hello"],
        answer: Some("Hello, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hi"],
        answer: Some("Hi, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["greetings"],
        answer: Some("Greetings, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hail"],
        answer: Some("And hail to you, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what", "is", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["aye"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["nay"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["arch", "warrior"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["arch-warrior"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["arch", "mage"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["arch-mage"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["arch-seyan", "du"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch", "seyan", "du"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch-seyan'du"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch", "seyan'du"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch", "seyan"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["arch-seyan"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["seyan", "du"],
        answer: None,
        answer_code: 8,
    },
    TextQaEntry {
        words: &["seyan'du"],
        answer: None,
        answer_code: 8,
    },
    TextQaEntry {
        words: &["seyan"],
        answer: None,
        answer_code: 8,
    },
    TextQaEntry {
        words: &["reset"],
        answer: None,
        answer_code: 9,
    },
];
//-----------------------
// Generic per-character driver memory.
//
// C `src/system/drvlib.c`'s `struct char_mem_data`/`mem_add_driver`/
// `mem_check_driver`/`mem_erase_driver` (declared in `src/system/drvlib.h`,
// *not* `src/system/mem.c`, which is an unrelated allocator-tracking
// module despite the similar name). Every driver shares 8 memory slots
// (`nr` 0..=7) per character, addressed via `set_data(cn, DRD_CHARMEM +
// nr, ...)` in C; each slot holds a list of "remembered" character
// identifiers with no membership limit besides `dat->max` growing by 8 at
// a time. C dedupes slot membership by a stable identity (`ch[co].ID |
// 0x80000000` for logged-in players, else `ch[co].serial & 0x7fffffff`)
// that survives character-table slot reuse; the existing merchant-greet
// port (`world/merchant.rs`) already simplified this to the raw runtime
// `CharacterId`, so the generic port below keeps that same simplification
// for consistency rather than threading persistent player IDs through.
// Timeouts are *not* part of `mem_add_driver` itself in C - callers keep
// their own "next clear" tick (e.g. merchant.c's `dat->memcleartimer`) and
// call `mem_erase_driver` when it elapses; `MerchantDriverData` keeps that
// per-driver timer field for the same reason.

/// C `mem_add_driver`/`mem_check_driver`/`mem_erase_driver`'s `nr` range
/// (`if (nr < 0 || nr > 7) return 0;`).
pub const DRIVER_MEMORY_SLOTS: usize = 8;
/// C `struct char_mem_data`, stored per-character (one instance covering
/// all 8 slots, mirroring how C addresses each slot via `DRD_CHARMEM +
/// nr` off the same character's driver-data list).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DriverMemory {
    slots: [Vec<u32>; DRIVER_MEMORY_SLOTS],
}
impl Default for DriverMemory {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| Vec::new()),
        }
    }
}
/// C `mem_add_driver(cn, co, nr)`: remembers `target` in memory slot
/// `slot`. A no-op duplicate add still returns `true` (C: `if
/// (dat->xID[n] == xID) return 1;`); an out-of-range slot returns `false`
/// (C: `return 0;`).
pub fn mem_add_driver(memory: &mut DriverMemory, slot: usize, target: u32) -> bool {
    let Some(bucket) = memory.slots.get_mut(slot) else {
        return false;
    };
    if !bucket.contains(&target) {
        bucket.push(target);
    }
    true
}
/// C `mem_check_driver(cn, co, nr)`: `true` if `target` is remembered in
/// memory slot `slot`.
pub fn mem_check_driver(memory: &DriverMemory, slot: usize, target: u32) -> bool {
    memory
        .slots
        .get(slot)
        .is_some_and(|bucket| bucket.contains(&target))
}
/// C `mem_erase_driver(cn, nr)`: clears memory slot `slot` (all other
/// slots are left untouched, matching C only zeroing `dat->cnt` for the
/// requested `nr`).
pub fn mem_erase_driver(memory: &mut DriverMemory, slot: usize) {
    if let Some(bucket) = memory.slots.get_mut(slot) {
        bucket.clear();
    }
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SimpleBaddyDriverData {
    pub startdist: i32,
    pub chardist: i32,
    pub stopdist: i32,
    #[serde(default)]
    pub home_x: u16,
    #[serde(default)]
    pub home_y: u16,
    pub aggressive: i32,
    pub helper: i32,
    pub scavenger: i32,
    pub dir: i32,
    pub dayx: i32,
    pub dayy: i32,
    pub daydir: i32,
    pub nightx: i32,
    pub nighty: i32,
    pub nightdir: i32,
    pub teleport: i32,
    pub helpid: i32,
    pub creation_time: i32,
    pub notsecure: i32,
    pub mindist: i32,
    pub lastfight: i32,
    #[serde(default)]
    pub last_hit: i32,
    #[serde(default)]
    pub pending_bless_friend: Option<CharacterId>,
    pub poison_power: i32,
    pub poison_chance: i32,
    pub poison_type: i32,
    pub drinkspecial: i32,
    pub drink_inventory_potions: i32,
    #[serde(default)]
    pub enemies: Vec<SimpleBaddyEnemy>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SimpleBaddyEnemy {
    pub target_id: CharacterId,
    pub priority: i32,
    pub last_seen_tick: i32,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub last_x: u16,
    #[serde(default)]
    pub last_y: u16,
}
/// C `struct fight_driver_data` (`src/common/fight.h:27-37`), stored via
/// `set_data(cn, DRD_FIGHTDRIVER, ...)` - a slot independent of whichever
/// `driver`/`driver_state` a character currently has (C's `set_data` lets
/// one character hold named data blobs for several drivers/subsystems at
/// once; the `simple_baddy` driver's own `startdist`/`chardist`/`stopdist`/
/// `lastfight` fields on [`SimpleBaddyDriverData`] are a *different*,
/// simple_baddy-owned copy only used to seed this one once at creation via
/// `fight_driver_set_dist`, `simple_baddy.c:189` - see
/// `apply_simple_baddy_create_message`). Lives on the dedicated
/// [`crate::entity::Character::fight_driver`] field, mirroring the
/// existing `Character::dungeonfighter` precedent, so any character
/// (SimpleBaddy NPC, lostcon corpse, or a normal playing character with a
/// `no*`/`auto*` toggle set) can drive `fight_driver_attack_enemy`'s
/// enemy-tracking without needing a `SimpleBaddyDriverData` of its own.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct FightDriverData {
    /// C `struct person enemy[10]`.
    #[serde(default)]
    pub enemies: Vec<SimpleBaddyEnemy>,
    /// C `start_dist`: distance from home at which to start attacking.
    #[serde(default)]
    pub start_dist: i32,
    /// C `stop_dist`: distance from home at which to stop attacking.
    #[serde(default)]
    pub stop_dist: i32,
    /// C `char_dist`: distance from the character we start attacking.
    #[serde(default)]
    pub char_dist: i32,
    /// C `home_x`/`home_y`: position `start_dist`/`stop_dist` are measured
    /// from; falls back to the respawn point (then current position) when
    /// zero, exactly like `fight_driver_dist_from_home`.
    #[serde(default)]
    pub home_x: u16,
    #[serde(default)]
    pub home_y: u16,
    /// C `lasthit`: tick of the last `fight_driver_note_hit` call, read by
    /// `fight_driver_regen_value`'s post-hit regen-suppression window.
    #[serde(default)]
    pub last_hit: i32,
}
impl Default for SimpleBaddyDriverData {
    fn default() -> Self {
        Self {
            startdist: 20,
            chardist: 0,
            stopdist: 40,
            home_x: 0,
            home_y: 0,
            aggressive: 0,
            helper: 0,
            scavenger: 0,
            dir: 3,
            dayx: 0,
            dayy: 0,
            daydir: 0,
            nightx: 0,
            nighty: 0,
            nightdir: 0,
            teleport: 0,
            helpid: 0,
            creation_time: 0,
            notsecure: 0,
            mindist: 0,
            lastfight: 0,
            last_hit: 0,
            pending_bless_friend: None,
            poison_power: 0,
            poison_chance: 0,
            poison_type: 0,
            drinkspecial: 0,
            drink_inventory_potions: 0,
            enemies: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSimpleBaddyArgument {
    pub name: String,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleBaddyParseResult {
    pub data: SimpleBaddyDriverData,
    pub unknown: Vec<UnknownSimpleBaddyArgument>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleBaddyMessageOutcome {
    UseInventoryPotion {
        item_id: ItemId,
        reason: PotionUseReason,
    },
    BlessFriend {
        target_id: crate::ids::CharacterId,
    },
    PoisonHit {
        target_id: CharacterId,
        power: u16,
        poison_type: u16,
        chance: i32,
    },
    AddEnemy {
        caller_id: CharacterId,
        target_id: CharacterId,
    },
    RemoveEnemy {
        target_id: CharacterId,
    },
    StandardAggro {
        target_id: CharacterId,
        priority: i32,
        require_visible: bool,
        hurtme: bool,
    },
    StandardSeenHit {
        attacker_id: CharacterId,
        victim_id: CharacterId,
    },
    TextNotification {
        speaker_id: CharacterId,
        text_token: i32,
        text: Option<String>,
    },
    NoteHit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotionUseReason {
    LowHp,
    LowMana,
}
pub fn parse_simple_baddy_driver_args(args: &str) -> SimpleBaddyParseResult {
    let mut data = SimpleBaddyDriverData::default();
    let mut unknown = Vec::new();
    let mut rest = args;

    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "aggressive" => data.aggressive = parsed,
            "scavenger" => data.scavenger = parsed,
            "helper" => data.helper = parsed,
            "startdist" => data.startdist = parsed,
            "chardist" => data.chardist = parsed,
            "stopdist" => data.stopdist = parsed,
            "dir" => data.dir = parsed,
            "dayx" => data.dayx = parsed,
            "dayy" => data.dayy = parsed,
            "daydir" => data.daydir = parsed,
            "nightx" => data.nightx = parsed,
            "nighty" => data.nighty = parsed,
            "nightdir" => data.nightdir = parsed,
            "teleport" => data.teleport = parsed,
            "helpid" => data.helpid = parsed,
            "notsecure" => data.notsecure = parsed,
            "mindist" => data.mindist = parsed,
            "poisonpower" => data.poison_power = parsed,
            "poisontype" => data.poison_type = parsed,
            "poisonchance" => data.poison_chance = parsed,
            "drinkspecial" => data.drinkspecial = parsed,
            "drinkinvpots" => data.drink_inventory_potions = parsed,
            _ => unknown.push(UnknownSimpleBaddyArgument {
                name: name.to_string(),
                value: value.to_string(),
            }),
        }
        rest = next;
    }

    SimpleBaddyParseResult { data, unknown }
}
pub fn apply_simple_baddy_create_message(
    character: &mut Character,
    args: Option<&str>,
    current_tick: i32,
) -> Vec<UnknownSimpleBaddyArgument> {
    let mut data = match character.driver_state.take() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data,
        Some(
            CharacterDriverState::Clara(_)
            | CharacterDriverState::TwoSkelly(_)
            | CharacterDriverState::Lab2Undead(_)
            | CharacterDriverState::Merchant(_)
            | CharacterDriverState::Aclerk(_)
            | CharacterDriverState::Lostcon(_)
            | CharacterDriverState::Bank(_)
            | CharacterDriverState::Trader(_)
            | CharacterDriverState::Janitor(_)
            | CharacterDriverState::GateWelcome(_)
            | CharacterDriverState::GateFight(_)
            | CharacterDriverState::Clanmaster(_)
            | CharacterDriverState::ClanFound(_)
            | CharacterDriverState::Clanclerk(_)
            | CharacterDriverState::Clubmaster(_)
            | CharacterDriverState::MilitaryMaster(_)
            | CharacterDriverState::MilitaryAdvisor(_)
            | CharacterDriverState::ArenaMaster(_)
            | CharacterDriverState::ArenaFighter(_)
            | CharacterDriverState::ArenaManager(_)
            | CharacterDriverState::Dungeonmaster(_)
            | CharacterDriverState::Dungeonfighter(_)
            | CharacterDriverState::Macro(_)
            | CharacterDriverState::Camhermit(_)
            | CharacterDriverState::Yoakin(_)
            | CharacterDriverState::Terion(_)
            | CharacterDriverState::Gwendylon(_)
            | CharacterDriverState::Greeter(_)
            | CharacterDriverState::Jessica(_)
            | CharacterDriverState::Jiu(_)
            | CharacterDriverState::ForestRanger(_)
            | CharacterDriverState::Brithildie(_)
            | CharacterDriverState::Nook(_)
            | CharacterDriverState::Lydia(_)
            | CharacterDriverState::Robber(_)
            | CharacterDriverState::Sanoa(_)
            | CharacterDriverState::Asturin(_)
            | CharacterDriverState::Reskin(_)
            | CharacterDriverState::Guiwynn(_)
            | CharacterDriverState::James(_)
            | CharacterDriverState::Balltrap(_)
            | CharacterDriverState::Logain(_)
            | CharacterDriverState::Superior(_)
            | CharacterDriverState::Moonie(_)
            | CharacterDriverState::Vampire(_)
            | CharacterDriverState::Vampire2(_),
        ) => SimpleBaddyDriverData::default(),
        None => SimpleBaddyDriverData::default(),
    };

    let unknown = if let Some(args) = args.filter(|args| !args.is_empty()) {
        let parsed = parse_simple_baddy_driver_args(args);
        data = parsed.data;
        parsed.unknown
    } else {
        Vec::new()
    };

    data.creation_time = current_tick;
    // C `fight_driver_set_dist(cn, dat->startdist, dat->chardist,
    // dat->stopdist)` (`simple_baddy.c:189`): seeds the independent
    // `DRD_FIGHTDRIVER` slot's distance config from simple_baddy's own
    // freshly (re)parsed copy, leaving any already-tracked enemies/home
    // position/last-hit tick untouched (`fight_driver_set_dist` itself
    // only ever writes the three distance fields).
    let fight_driver = character
        .fight_driver
        .get_or_insert_with(FightDriverData::default);
    fight_driver.start_dist = data.startdist;
    fight_driver.char_dist = data.chardist;
    fight_driver.stop_dist = data.stopdist;
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(data));
    character
        .driver_messages
        .retain(|message| message.message_type != NT_CREATE);

    if character.inventory.get(30).and_then(|slot| *slot).is_some()
        && character.flags.contains(CharacterFlags::NOBODY)
    {
        character.flags.remove(CharacterFlags::NOBODY);
        character.flags.insert(CharacterFlags::ITEMDEATH);
    }

    unknown
}
pub fn process_simple_baddy_messages(
    character: &mut Character,
    carried_items: &[Item],
) -> Vec<SimpleBaddyMessageOutcome> {
    let drink_inventory_potions = matches!(
        character.driver_state.as_ref(),
        Some(CharacterDriverState::SimpleBaddy(data)) if data.drink_inventory_potions != 0
    );
    let helper = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data.helper,
        _ => 0,
    };
    let aggressive = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data.aggressive,
        _ => 0,
    };
    let poison = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) if data.poison_power > 0 => Some((
            data.poison_power as u16,
            data.poison_type.max(0) as u16,
            data.poison_chance,
        )),
        _ => None,
    };
    let helpid = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data.helpid,
        _ => 0,
    };
    let mut outcomes = Vec::new();

    let messages = std::mem::take(&mut character.driver_messages);
    for message in messages {
        if message.message_type == NT_CHAR && helper != 0 && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::BlessFriend {
                target_id: crate::ids::CharacterId(message.dat1 as u32),
            });
        }

        if message.message_type == NT_CHAR && aggressive != 0 && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::StandardAggro {
                target_id: CharacterId(message.dat1 as u32),
                priority: 0,
                require_visible: true,
                hurtme: false,
            });
        }

        if message.message_type == NT_SEEHIT && helper != 0 && message.dat1 > 0 && message.dat2 > 0
        {
            outcomes.push(SimpleBaddyMessageOutcome::StandardSeenHit {
                attacker_id: CharacterId(message.dat1 as u32),
                victim_id: CharacterId(message.dat2 as u32),
            });
        }

        if message.message_type == NT_TEXT && message.dat3 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::TextNotification {
                speaker_id: CharacterId(message.dat3 as u32),
                text_token: message.dat2,
                text: message.text.clone(),
            });
        }

        if message.message_type == NT_GOTHIT && drink_inventory_potions {
            if let Some(item_id) = find_simple_baddy_inventory_potion(
                character,
                carried_items,
                CharacterValue::Hp,
                2,
                PotionUseReason::LowHp,
            ) {
                outcomes.push(SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id,
                    reason: PotionUseReason::LowHp,
                });
            }

            if let Some(item_id) = find_simple_baddy_inventory_potion(
                character,
                carried_items,
                CharacterValue::Mana,
                4,
                PotionUseReason::LowMana,
            ) {
                outcomes.push(SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id,
                    reason: PotionUseReason::LowMana,
                });
            }
        }

        if message.message_type == NT_GOTHIT {
            outcomes.push(SimpleBaddyMessageOutcome::NoteHit);
        }

        if message.message_type == NT_GOTHIT && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::StandardAggro {
                target_id: CharacterId(message.dat1 as u32),
                priority: 1,
                require_visible: false,
                hurtme: true,
            });
        }

        if message.message_type == NT_DIDHIT && message.dat1 > 0 && message.dat2 > 0 {
            if let Some((power, poison_type, chance)) = poison {
                outcomes.push(SimpleBaddyMessageOutcome::PoisonHit {
                    target_id: CharacterId(message.dat1 as u32),
                    power,
                    poison_type,
                    chance,
                });
            }
        }

        if message.message_type == NT_NPC
            && helpid != 0
            && message.dat1 == helpid
            && message.dat2 > 0
        {
            outcomes.push(SimpleBaddyMessageOutcome::AddEnemy {
                caller_id: CharacterId(message.dat2 as u32),
                target_id: CharacterId(message.dat3.max(0) as u32),
            });
        }

        if message.message_type == NT_DEAD && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::RemoveEnemy {
                target_id: CharacterId(message.dat1 as u32),
            });
        }
    }

    outcomes
}
pub fn add_simple_baddy_enemy(
    character: &mut Character,
    caller: &Character,
    target_id: CharacterId,
    current_tick: i32,
) -> bool {
    if caller.id == character.id || caller.group != character.group {
        return false;
    }

    add_simple_baddy_enemy_unchecked(character, target_id, 1, current_tick)
}
pub fn add_simple_baddy_enemy_unchecked(
    character: &mut Character,
    target_id: CharacterId,
    priority: i32,
    current_tick: i32,
) -> bool {
    // C `fight_driver_add_enemy` (`drvlib.c:2056`) reads/writes the
    // `DRD_FIGHTDRIVER` slot independently of whatever driver `cn` is
    // currently running - it is shared by the `CDR_SIMPLEBADDY`/
    // `CDR_DUNGEONFIGHTER` NPC driver, the `CDR_LOSTCON` self-defense
    // driver, and (via the player-side `no*` toggles) a normal playing
    // character. No `driver_state` gate here, matching that.
    let data = character
        .fight_driver
        .get_or_insert_with(FightDriverData::default);

    if let Some(enemy) = data
        .enemies
        .iter_mut()
        .take(9)
        .find(|enemy| enemy.target_id == target_id)
    {
        enemy.priority = priority;
        enemy.last_seen_tick = current_tick;
        return false;
    }

    let enemy = SimpleBaddyEnemy {
        target_id,
        priority,
        last_seen_tick: current_tick,
        visible: false,
        last_x: 0,
        last_y: 0,
    };
    if data.enemies.len() < 10 {
        data.enemies.push(enemy);
    } else {
        data.enemies[9] = enemy;
    }
    true
}
pub fn remove_simple_baddy_enemy(character: &mut Character, target_id: CharacterId) -> bool {
    // C `fight_driver_remove_enemy` (`drvlib.c:2144`): same
    // driver-independent `DRD_FIGHTDRIVER` slot as `add_simple_baddy_enemy_
    // unchecked` above - no `driver_state` gate.
    let Some(data) = character.fight_driver.as_mut() else {
        return false;
    };

    let previous_len = data.enemies.len();
    data.enemies.retain(|enemy| enemy.target_id != target_id);
    data.enemies.len() != previous_len
}
fn find_simple_baddy_inventory_potion(
    character: &Character,
    carried_items: &[Item],
    value: CharacterValue,
    divisor: i32,
    reason: PotionUseReason,
) -> Option<ItemId> {
    let max_value = character_value(character, value);
    if max_value == 0 {
        return None;
    }

    let current = match value {
        CharacterValue::Hp => character.hp,
        CharacterValue::Mana => character.mana,
        _ => return None,
    };
    if current >= max_value * POWERSCALE / divisor {
        return None;
    }

    character
        .inventory
        .get(30..INVENTORY_SIZE)
        .unwrap_or_default()
        .iter()
        .flatten()
        .find(|item_id| {
            carried_items
                .iter()
                .find(|item| item.id == **item_id)
                .is_some_and(|item| {
                    item.driver == IDR_POTION
                        && match reason {
                            PotionUseReason::LowHp => drdata(item, 1) != 0,
                            PotionUseReason::LowMana => drdata(item, 2) != 0,
                        }
                })
        })
        .copied()
}
fn character_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default()
        .into()
}
fn drdata(item: &Item, index: usize) -> u8 {
    item.driver_data.get(index).copied().unwrap_or_default()
}
pub(crate) fn next_legacy_name_value(input: &str) -> Option<(&str, &str, &str)> {
    let input = input.trim_start_matches(char::is_whitespace);
    let name_len = input
        .bytes()
        .take(60)
        .take_while(|byte| byte.is_ascii_alphabetic())
        .count();
    if name_len == 0 {
        return None;
    }
    let name = &input[..name_len];
    let input = input[name_len..].trim_start_matches(char::is_whitespace);
    let input = input.strip_prefix('=')?;
    let input = input.trim_start_matches(char::is_whitespace);
    let value_len = input
        .bytes()
        .take(60)
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        .count();
    let value = &input[..value_len];
    let input = input[value_len..].strip_prefix(';')?;
    Some((name, value, input.trim_start_matches(char::is_whitespace)))
}
pub const EXP_AREA15_HARDKILL: i32 = 5_000;
/// C `teleport_next_lab(cn, 0)` truthiness (`src/system/lab.c:94-104`).
/// With `do_teleport = 0`, `teleport_lab`'s `!do_teleport ||
/// change_area(...)` always short-circuits true without touching the map,
/// so the loop's outcome depends only on whether every known lab
/// checkpoint bit (`src/system/lab.c:40-83`'s `teleport_lab` switch -
/// levels 10/15/20/25/30, i.e. `crate::item_driver::legacy_lab_destination`)
/// is already solved; the character's level only changes *which* nonzero
/// value would be returned (`1` vs `-required_level`), never the
/// truthiness this needs.
pub fn needs_next_lab(lab_solved_bits: u64) -> bool {
    (0..64_u8).any(|lab_level| {
        let bit = 1_u64 << lab_level;
        lab_solved_bits & bit == 0
            && crate::item_driver::legacy_lab_destination(lab_level).is_some()
    })
}
/// C `enter_test`'s class-choice/item-carrying preconditions
/// (`gatekeeper.c:316-390`), excluding the side-effecting tail
/// (`take_money`, `enter_room` room search) which needs `World` access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateEnterTestPrecheck {
    /// C `ch[cn].flags & CF_PAID`.
    pub is_paid: bool,
    /// C `teleport_next_lab(cn, 0)` truthiness.
    pub needs_lab: bool,
    /// C `ch[cn].flags & CF_GOD`.
    pub is_god: bool,
    /// C `ch[cn].flags & CF_NOEXP`.
    pub is_noexp: bool,
    pub flags: CharacterFlags,
    /// C's `cnt`: carried items in slots `30..INVENTORYSIZE` plus
    /// `ch[cn].citem`.
    pub carried_item_count: u32,
    /// The chosen class: `5` Arch-Warrior, `6` Arch-Mage, `7`
    /// Arch-Seyan'Du, `8` Seyan'Du.
    pub class: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateEnterTestOutcome {
    /// "Sorry, only paying players may take the test."
    NotPaid,
    /// "Sorry, you may not enter before you have solved the labyrinth."
    LabNotSolved,
    /// "Sorry, you may not enter if you have the /noexp mode turned on."
    NoExpMode,
    /// C's class-validation `switch` (or its `default`) returned `0`
    /// silently; the caller (`gate_welcome_driver`) then says "That is
    /// not a possible choice."
    InvalidClass,
    /// "Sorry, you may not enter while you are carrying items. You
    /// currently have %d items." (any items, non-Seyan'Du classes).
    CarryingItems { count: u32 },
    /// "Sorry, you may not enter while you are carrying more than three
    /// items. You currently have %d items." (Seyan'Du class only).
    CarryingTooManyItems { count: u32 },
    /// All preconditions satisfied; caller should attempt
    /// `take_money(cn, 100 * 100)` then the `enter_room` search.
    Ready,
}
fn gate_class_choice_is_valid(flags: CharacterFlags, class: i32) -> bool {
    use CharacterFlags as F;
    match class {
        5 => !flags.intersects(F::MAGE | F::ARCH),
        6 => !flags.intersects(F::WARRIOR | F::ARCH),
        7 => !flags.contains(F::ARCH) && flags.contains(F::WARRIOR) && flags.contains(F::MAGE),
        8 => !flags.contains(F::ARCH) && !(flags.contains(F::WARRIOR) && flags.contains(F::MAGE)),
        _ => false,
    }
}
pub fn gate_enter_test_precheck(input: GateEnterTestPrecheck) -> GateEnterTestOutcome {
    if !input.is_paid {
        return GateEnterTestOutcome::NotPaid;
    }
    if input.needs_lab && !input.is_god {
        return GateEnterTestOutcome::LabNotSolved;
    }
    if input.is_noexp {
        return GateEnterTestOutcome::NoExpMode;
    }
    if !input.is_god {
        if !gate_class_choice_is_valid(input.flags, input.class) {
            return GateEnterTestOutcome::InvalidClass;
        }
        if input.carried_item_count > 0 && input.class != 8 {
            return GateEnterTestOutcome::CarryingItems {
                count: input.carried_item_count,
            };
        }
        if input.carried_item_count > 3 {
            return GateEnterTestOutcome::CarryingTooManyItems {
                count: input.carried_item_count,
            };
        }
    }
    GateEnterTestOutcome::Ready
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverKind {
    SimpleBaddy,
    Macro,
    SwampClara,
    SwampMonster,
    PalaceIslena,
    TwoSkelly,
    Trader,
    LqNpc,
    Janitor,
    TeufelDemon,
    TeufelGambler,
    TeufelQuest,
    TeufelRat,
    CaligarSkelly,
    Lab2Undead,
}
impl CharacterDriverKind {
    pub fn from_legacy_id(driver: u16) -> Option<Self> {
        match driver {
            CDR_SIMPLEBADDY => Some(Self::SimpleBaddy),
            CDR_MACRO => Some(Self::Macro),
            CDR_SWAMPCLARA => Some(Self::SwampClara),
            CDR_SWAMPMONSTER => Some(Self::SwampMonster),
            CDR_PALACEISLENA => Some(Self::PalaceIslena),
            CDR_TWOSKELLY => Some(Self::TwoSkelly),
            CDR_TRADER => Some(Self::Trader),
            CDR_LQNPC => Some(Self::LqNpc),
            CDR_JANITOR => Some(Self::Janitor),
            CDR_TEUFELDEMON => Some(Self::TeufelDemon),
            CDR_TEUFELGAMBLER => Some(Self::TeufelGambler),
            CDR_TEUFELQUEST => Some(Self::TeufelQuest),
            CDR_TEUFELRAT => Some(Self::TeufelRat),
            CDR_CALIGARSKELLY => Some(Self::CaligarSkelly),
            CDR_LAB2UNDEAD => Some(Self::Lab2Undead),
            _ => None,
        }
    }

    pub fn legacy_id(self) -> u16 {
        match self {
            Self::SimpleBaddy => CDR_SIMPLEBADDY,
            Self::Macro => CDR_MACRO,
            Self::SwampClara => CDR_SWAMPCLARA,
            Self::SwampMonster => CDR_SWAMPMONSTER,
            Self::PalaceIslena => CDR_PALACEISLENA,
            Self::TwoSkelly => CDR_TWOSKELLY,
            Self::Trader => CDR_TRADER,
            Self::LqNpc => CDR_LQNPC,
            Self::Janitor => CDR_JANITOR,
            Self::TeufelDemon => CDR_TEUFELDEMON,
            Self::TeufelGambler => CDR_TEUFELGAMBLER,
            Self::TeufelQuest => CDR_TEUFELQUEST,
            Self::TeufelRat => CDR_TEUFELRAT,
            Self::CaligarSkelly => CDR_CALIGARSKELLY,
            Self::Lab2Undead => CDR_LAB2UNDEAD,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverCall {
    Tick { ret: i32, last_action: i32 },
    Died { killer_character_id: u32 },
    Respawn,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverOutcome {
    /// `simple_baddy_dead`: earth demons create earth/rain retaliation effects
    /// at the killer position when the dead NPC can see the killer.
    SimpleBaddyDeath { killer_character_id: u32 },
    /// Legacy handler returned `1`; behavior is intentionally deferred to a
    /// future typed implementation for this concrete driver.
    HandledStub {
        kind: CharacterDriverKind,
        call: CharacterDriverCall,
    },
    /// Legacy module probing would continue and eventually return `0`.
    Unsupported {
        driver: u16,
        call: CharacterDriverCall,
    },
}
impl CharacterDriverOutcome {
    pub fn legacy_return_code(self) -> i32 {
        match self {
            Self::SimpleBaddyDeath { .. } => 1,
            Self::HandledStub { .. } => 1,
            Self::Unsupported { .. } => 0,
        }
    }
}
pub fn execute_character_driver(driver: u16, ret: i32, last_action: i32) -> CharacterDriverOutcome {
    let call = CharacterDriverCall::Tick { ret, last_action };
    dispatch_known_character_driver(driver, call)
}
pub fn execute_character_died_driver(
    driver: u16,
    killer_character_id: u32,
) -> CharacterDriverOutcome {
    let call = CharacterDriverCall::Died {
        killer_character_id,
    };
    dispatch_known_character_driver(driver, call)
}
pub fn execute_character_respawn_driver(driver: u16) -> CharacterDriverOutcome {
    dispatch_known_character_driver(driver, CharacterDriverCall::Respawn)
}
fn dispatch_known_character_driver(
    driver: u16,
    call: CharacterDriverCall,
) -> CharacterDriverOutcome {
    if driver == CDR_SIMPLEBADDY {
        if let CharacterDriverCall::Died {
            killer_character_id,
        } = call
        {
            return CharacterDriverOutcome::SimpleBaddyDeath {
                killer_character_id,
            };
        }
    }

    match CharacterDriverKind::from_legacy_id(driver) {
        Some(kind) => CharacterDriverOutcome::HandledStub { kind, call },
        None => CharacterDriverOutcome::Unsupported { driver, call },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity::{ItemFlags, SpeedMode},
        ids::ItemId,
    };

    #[test]
    fn legacy_dispatch_type_constants_match_c_libload() {
        assert_eq!(CDT_DRIVER, 0);
        assert_eq!(CDT_ITEM, 1);
        assert_eq!(CDT_DEAD, 2);
        assert_eq!(CDT_RESPAWN, 3);
        assert_eq!(CDT_SPECIAL, 4);
    }

    #[test]
    fn parse_clanclerk_driver_args_reads_bare_clan_number() {
        assert_eq!(parse_clanclerk_driver_args("5").clan, 5);
        assert_eq!(parse_clanclerk_driver_args(" 12 ").clan, 12);
        assert_eq!(parse_clanclerk_driver_args("").clan, 0);
        assert_eq!(parse_clanclerk_driver_args("not-a-number").clan, 0);
    }

    #[test]
    fn cdr_clanclerk_matches_c_drvlib() {
        assert_eq!(CDR_CLANMASTER, 27);
        assert_eq!(CDR_CLANCLERK, 28);
    }

    #[test]
    fn cdr_clubmaster_matches_c_drvlib() {
        assert_eq!(CDR_CLUBMASTER, 113);
    }

    #[test]
    fn parse_clubmaster_driver_args_reads_dir() {
        assert_eq!(parse_clubmaster_driver_args("dir=3;").dir, 3);
        assert_eq!(parse_clubmaster_driver_args("").dir, 0);
    }

    #[test]
    fn cdr_arena_constants_match_c_drvlib() {
        assert_eq!(CDR_ARENAMASTER, 48);
        assert_eq!(CDR_ARENAFIGHTER, 49);
        assert_eq!(CDR_ARENAMANAGER, 50);
    }

    #[test]
    fn parse_arena_manager_driver_args_reads_real_zone_file_arg() {
        // Verbatim `arg=` from `ugaris_data/zones/3/above3_generic.chr`.
        let data = parse_arena_manager_driver_args(
            "arenax=233;arenay=122;arenafx=230;arenafy=119;arenatx=242;arenaty=125;",
        );
        assert_eq!(data.arena_x, 233);
        assert_eq!(data.arena_y, 122);
        assert_eq!(data.arena_fx, 230);
        assert_eq!(data.arena_fy, 119);
        assert_eq!(data.arena_tx, 242);
        assert_eq!(data.arena_ty, 125);
        assert_eq!(data.renter, None);
        assert!(data.invite.is_empty());
    }

    #[test]
    fn parse_arena_manager_driver_args_ignores_unknown_names() {
        let data = parse_arena_manager_driver_args("arenax=5;bogus=9;arenay=6;");
        assert_eq!(data.arena_x, 5);
        assert_eq!(data.arena_y, 6);
    }

    #[test]
    fn notify_constants_match_c_notify_header() {
        assert_eq!(NT_CHAR, 1);
        assert_eq!(NT_ITEM, 2);
        assert_eq!(NT_GOTHIT, 3);
        assert_eq!(NT_DIDHIT, 4);
        assert_eq!(NT_SEEHIT, 5);
        assert_eq!(NT_DEAD, 6);
        assert_eq!(NT_SPELL, 7);
        assert_eq!(NT_GIVE, 8);
        assert_eq!(NT_CREATE, 9);
        assert_eq!(NT_TEXT, 200);
        assert_eq!(NT_NPC, 300);
        assert_eq!(NTID_MERCHANT, 1);
        assert_eq!(NTID_GLADIATOR, 16);
    }

    #[test]
    fn analyse_text_qa_matches_keyword_and_substitutes_names() {
        // C: `quiet_say(cn, "Hello, %s!", ch[co].name, ch[cn].name)`.
        assert_eq!(
            analyse_text_qa("hello", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::Said("Hello, Bob!".to_string())
        );
    }

    #[test]
    fn analyse_text_qa_is_case_insensitive() {
        assert_eq!(
            analyse_text_qa("HELLO", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::Said("Hello, Bob!".to_string())
        );
        assert_eq!(
            analyse_text_qa("HeLLo", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::Said("Hello, Bob!".to_string())
        );
    }

    #[test]
    fn analyse_text_qa_reports_no_match_for_unknown_text() {
        assert_eq!(
            analyse_text_qa("blahblah nonsense", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::NoMatch
        );
        // Empty word list (e.g. only punctuation) is also NoMatch, matching
        // C's `if (w) { ... }` guard around the qa scan.
        assert_eq!(
            analyse_text_qa("...", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::NoMatch
        );
    }

    #[test]
    fn analyse_text_qa_filters_own_name_out_of_wordlist() {
        // C: `strcasecmp(wordlist[w], ch[cn].name)` drops the NPC's own
        // name from the tokenized message before matching, so addressing
        // the merchant by name doesn't break a match.
        assert_eq!(
            analyse_text_qa("Dolf, hello", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::Said("Hello, Bob!".to_string())
        );
        assert_eq!(
            analyse_text_qa("hello Dolf", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::Said("Hello, Bob!".to_string())
        );
    }

    #[test]
    fn analyse_text_qa_requires_exact_word_count_match() {
        // C's inner match loop requires the tokenized message and the qa
        // pattern to run out of words together (`n == w && !qa[q].word[n]`);
        // a longer or shorter phrase around a keyword is not a match.
        assert_eq!(
            analyse_text_qa("well hello there", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::NoMatch
        );
        assert_eq!(
            analyse_text_qa("how are you doing", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::NoMatch
        );
        assert_eq!(
            analyse_text_qa("how are you", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::Said("I'm fine!".to_string())
        );
    }

    #[test]
    fn analyse_text_qa_reports_answer_code_when_no_canned_answer() {
        // C: `who are you` -> `answer: NULL, answer_code: 1` -> callers
        // that don't special-case it (like `gwendylon_driver`) get the
        // raw code back to interpret themselves.
        assert_eq!(
            analyse_text_qa("who are you", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::Matched(1)
        );
        assert_eq!(
            analyse_text_qa("what is your name", "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::Matched(1)
        );
    }

    #[test]
    fn analyse_text_qa_rejects_oversized_words() {
        // C: `if (n > 250) return 0;` bails out of tokenization entirely.
        let huge_word = "a".repeat(300);
        assert_eq!(
            analyse_text_qa(&huge_word, "Dolf", "Bob", MERCHANT_QA),
            TextAnalysisOutcome::NoMatch
        );
    }

    #[test]
    fn mem_check_driver_is_false_until_added() {
        let memory = DriverMemory::default();
        assert!(!mem_check_driver(&memory, 7, 42));
    }

    #[test]
    fn mem_add_then_check_driver_remembers_target() {
        let mut memory = DriverMemory::default();
        assert!(mem_add_driver(&mut memory, 7, 42));
        assert!(mem_check_driver(&memory, 7, 42));
        // C: unrelated slots and unrelated targets stay untouched.
        assert!(!mem_check_driver(&memory, 6, 42));
        assert!(!mem_check_driver(&memory, 7, 99));
    }

    #[test]
    fn mem_add_driver_is_idempotent_for_duplicate_targets() {
        // C: `if (dat->xID[n] == xID) return 1;` - no duplicate entry, and
        // erasing the slot removes the target in one shot either way.
        let mut memory = DriverMemory::default();
        assert!(mem_add_driver(&mut memory, 3, 7));
        assert!(mem_add_driver(&mut memory, 3, 7));
        assert_eq!(memory.slots[3].len(), 1);
    }

    #[test]
    fn mem_add_and_check_driver_reject_out_of_range_slots() {
        // C: `if (nr < 0 || nr > 7) return 0;`.
        let mut memory = DriverMemory::default();
        assert!(!mem_add_driver(&mut memory, DRIVER_MEMORY_SLOTS, 1));
        assert!(!mem_check_driver(&memory, DRIVER_MEMORY_SLOTS, 1));
    }

    #[test]
    fn mem_erase_driver_clears_only_the_requested_slot() {
        let mut memory = DriverMemory::default();
        mem_add_driver(&mut memory, 2, 1);
        mem_add_driver(&mut memory, 7, 2);
        mem_erase_driver(&mut memory, 7);
        assert!(!mem_check_driver(&memory, 7, 2));
        assert!(mem_check_driver(&memory, 2, 1));
    }

    #[test]
    fn mem_erase_driver_out_of_range_slot_is_a_silent_no_op() {
        let mut memory = DriverMemory::default();
        mem_add_driver(&mut memory, 0, 1);
        mem_erase_driver(&mut memory, DRIVER_MEMORY_SLOTS);
        assert!(mem_check_driver(&memory, 0, 1));
    }

    #[test]
    fn base_character_driver_ids_match_c_drvlib() {
        assert_eq!(CDR_LOSTCON, 5);
        assert_eq!(CDR_SIMPLEBADDY, 7);
        assert_eq!(CDR_MACRO, 37);
        assert_eq!(CDR_SWAMPCLARA, 54);
        assert_eq!(CDR_SWAMPMONSTER, 56);
        assert_eq!(CDR_PALACEISLENA, 57);
        assert_eq!(CDR_TWOSKELLY, 70);
        assert_eq!(CDR_TRADER, 72);
        assert_eq!(CDR_LQNPC, 74);
        assert_eq!(CDR_JANITOR, 85);
        assert_eq!(CDR_TEUFELDEMON, 114);
        assert_eq!(CDR_TEUFELGAMBLER, 115);
        assert_eq!(CDR_TEUFELQUEST, 116);
        assert_eq!(CDR_TEUFELRAT, 117);
        assert_eq!(CDR_CALIGARSKELLY, 124);
        assert_eq!(CDR_LAB2UNDEAD, 198);
        assert_eq!(DRD_SIMPLEBADDYDRIVER, 0x0100_0013);
        assert_eq!(
            CharacterDriverKind::SimpleBaddy.legacy_id(),
            CDR_SIMPLEBADDY
        );
        assert_eq!(CharacterDriverKind::Macro.legacy_id(), CDR_MACRO);
        assert_eq!(CharacterDriverKind::SwampClara.legacy_id(), CDR_SWAMPCLARA);
        assert_eq!(
            CharacterDriverKind::SwampMonster.legacy_id(),
            CDR_SWAMPMONSTER
        );
        assert_eq!(
            CharacterDriverKind::PalaceIslena.legacy_id(),
            CDR_PALACEISLENA
        );
        assert_eq!(CharacterDriverKind::TwoSkelly.legacy_id(), CDR_TWOSKELLY);
        assert_eq!(CharacterDriverKind::Trader.legacy_id(), CDR_TRADER);
        assert_eq!(CharacterDriverKind::LqNpc.legacy_id(), CDR_LQNPC);
        assert_eq!(CharacterDriverKind::Janitor.legacy_id(), CDR_JANITOR);
        assert_eq!(
            CharacterDriverKind::TeufelDemon.legacy_id(),
            CDR_TEUFELDEMON
        );
        assert_eq!(
            CharacterDriverKind::TeufelGambler.legacy_id(),
            CDR_TEUFELGAMBLER
        );
        assert_eq!(
            CharacterDriverKind::TeufelQuest.legacy_id(),
            CDR_TEUFELQUEST
        );
        assert_eq!(CharacterDriverKind::TeufelRat.legacy_id(), CDR_TEUFELRAT);
        assert_eq!(
            CharacterDriverKind::CaligarSkelly.legacy_id(),
            CDR_CALIGARSKELLY
        );
        assert_eq!(CharacterDriverKind::Lab2Undead.legacy_id(), CDR_LAB2UNDEAD);
        assert_eq!(DRD_CLARADRIVER, 0x0100_0059);
        assert_eq!(DRD_SKELLYDRIVER, 0x0100_006a);
        assert_eq!(DRD_LAB2_UNDEAD, 0x0200_0001);
    }

    #[test]
    fn two_skelly_driver_state_matches_legacy_runtime_data_shape() {
        let mut data = TwoSkellyDriverData::default();
        assert_eq!(data.last_talk_tick, 0);
        assert_eq!(data.current_victim, None);
        assert_eq!(data.alive_tick, 0);

        data.last_talk_tick = 111;
        data.current_victim = Some(CharacterId(12));
        data.alive_tick = 222;
        assert_eq!(
            CharacterDriverState::TwoSkelly(data),
            CharacterDriverState::TwoSkelly(TwoSkellyDriverData {
                last_talk_tick: 111,
                current_victim: Some(CharacterId(12)),
                alive_tick: 222,
            })
        );
    }

    #[test]
    fn clara_driver_state_matches_legacy_runtime_data_shape() {
        let mut data = ClaraDriverData::default();
        assert_eq!(data.last_talk_tick, 0);
        assert_eq!(data.current_victim, None);

        data.last_talk_tick = 1234;
        data.current_victim = Some(CharacterId(77));
        assert_eq!(
            CharacterDriverState::Clara(data),
            CharacterDriverState::Clara(ClaraDriverData {
                last_talk_tick: 1234,
                current_victim: Some(CharacterId(77)),
            })
        );
    }

    #[test]
    fn known_base_tick_drivers_are_handled_like_c_ch_driver() {
        for (driver, kind) in [
            (CDR_SIMPLEBADDY, CharacterDriverKind::SimpleBaddy),
            (CDR_MACRO, CharacterDriverKind::Macro),
            (CDR_SWAMPCLARA, CharacterDriverKind::SwampClara),
            (CDR_SWAMPMONSTER, CharacterDriverKind::SwampMonster),
            (CDR_PALACEISLENA, CharacterDriverKind::PalaceIslena),
            (CDR_TWOSKELLY, CharacterDriverKind::TwoSkelly),
            (CDR_TRADER, CharacterDriverKind::Trader),
            (CDR_LQNPC, CharacterDriverKind::LqNpc),
            (CDR_JANITOR, CharacterDriverKind::Janitor),
            (CDR_TEUFELDEMON, CharacterDriverKind::TeufelDemon),
            (CDR_TEUFELGAMBLER, CharacterDriverKind::TeufelGambler),
            (CDR_TEUFELQUEST, CharacterDriverKind::TeufelQuest),
            (CDR_TEUFELRAT, CharacterDriverKind::TeufelRat),
            (CDR_LAB2UNDEAD, CharacterDriverKind::Lab2Undead),
        ] {
            let outcome = execute_character_driver(driver, 7, 11);
            assert_eq!(
                outcome,
                CharacterDriverOutcome::HandledStub {
                    kind,
                    call: CharacterDriverCall::Tick {
                        ret: 7,
                        last_action: 11,
                    },
                }
            );
            assert_eq!(outcome.legacy_return_code(), 1);
        }
    }

    #[test]
    fn known_base_death_and_respawn_drivers_are_handled_like_c() {
        let simple_died = execute_character_died_driver(CDR_SIMPLEBADDY, 123);
        assert_eq!(
            simple_died,
            CharacterDriverOutcome::SimpleBaddyDeath {
                killer_character_id: 123,
            }
        );
        assert_eq!(simple_died.legacy_return_code(), 1);

        let died = execute_character_died_driver(CDR_JANITOR, 123);
        assert_eq!(
            died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Janitor,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(died.legacy_return_code(), 1);

        let islena_died = execute_character_died_driver(CDR_PALACEISLENA, 123);
        assert_eq!(
            islena_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::PalaceIslena,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(islena_died.legacy_return_code(), 1);

        let clara_died = execute_character_died_driver(CDR_SWAMPCLARA, 123);
        assert_eq!(
            clara_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SwampClara,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(clara_died.legacy_return_code(), 1);

        let two_skelly_died = execute_character_died_driver(CDR_TWOSKELLY, 123);
        assert_eq!(
            two_skelly_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::TwoSkelly,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(two_skelly_died.legacy_return_code(), 1);

        let swamp_monster_died = execute_character_died_driver(CDR_SWAMPMONSTER, 123);
        assert_eq!(
            swamp_monster_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SwampMonster,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(swamp_monster_died.legacy_return_code(), 1);

        let simple_respawn = execute_character_respawn_driver(CDR_SIMPLEBADDY);
        assert_eq!(
            simple_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SimpleBaddy,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(simple_respawn.legacy_return_code(), 1);

        let respawn = execute_character_respawn_driver(CDR_TRADER);
        assert_eq!(
            respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Trader,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(respawn.legacy_return_code(), 1);

        let islena_respawn = execute_character_respawn_driver(CDR_PALACEISLENA);
        assert_eq!(
            islena_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::PalaceIslena,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(islena_respawn.legacy_return_code(), 1);

        let clara_respawn = execute_character_respawn_driver(CDR_SWAMPCLARA);
        assert_eq!(
            clara_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SwampClara,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(clara_respawn.legacy_return_code(), 1);

        let two_skelly_respawn = execute_character_respawn_driver(CDR_TWOSKELLY);
        assert_eq!(
            two_skelly_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::TwoSkelly,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(two_skelly_respawn.legacy_return_code(), 1);

        let swamp_monster_respawn = execute_character_respawn_driver(CDR_SWAMPMONSTER);
        assert_eq!(
            swamp_monster_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SwampMonster,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(swamp_monster_respawn.legacy_return_code(), 1);

        let lab2_undead_died = execute_character_died_driver(CDR_LAB2UNDEAD, 123);
        assert_eq!(
            lab2_undead_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Lab2Undead,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(lab2_undead_died.legacy_return_code(), 1);

        let lab2_undead_respawn = execute_character_respawn_driver(CDR_LAB2UNDEAD);
        assert_eq!(
            lab2_undead_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Lab2Undead,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(lab2_undead_respawn.legacy_return_code(), 1);
    }

    #[test]
    fn unknown_character_driver_returns_legacy_zero() {
        let outcome = execute_character_driver(999, 0, 0);
        assert_eq!(
            outcome,
            CharacterDriverOutcome::Unsupported {
                driver: 999,
                call: CharacterDriverCall::Tick {
                    ret: 0,
                    last_action: 0,
                },
            }
        );
        assert_eq!(outcome.legacy_return_code(), 0);
    }

    #[test]
    fn simple_baddy_defaults_match_create_message_initialization() {
        let data = SimpleBaddyDriverData::default();
        assert_eq!(data.aggressive, 0);
        assert_eq!(data.helper, 0);
        assert_eq!(data.startdist, 20);
        assert_eq!(data.chardist, 0);
        assert_eq!(data.stopdist, 40);
        assert_eq!(data.scavenger, 0);
        assert_eq!(data.dir, 3);
        assert_eq!(data.last_hit, 0);
        assert_eq!(data.drink_inventory_potions, 0);
    }

    #[test]
    fn parses_simple_baddy_legacy_arg_string() {
        let parsed = parse_simple_baddy_driver_args(
            " aggressive = 1; helper=2; startdist=12; poisonpower=-4; poisontype=3; poisonchance=25; drinkinvpots=1; unknown=99;",
        );

        assert_eq!(parsed.data.aggressive, 1);
        assert_eq!(parsed.data.helper, 2);
        assert_eq!(parsed.data.startdist, 12);
        assert_eq!(parsed.data.poison_power, -4);
        assert_eq!(parsed.data.poison_type, 3);
        assert_eq!(parsed.data.poison_chance, 25);
        assert_eq!(parsed.data.drink_inventory_potions, 1);
        assert_eq!(
            parsed.unknown,
            vec![UnknownSimpleBaddyArgument {
                name: "unknown".to_string(),
                value: "99".to_string(),
            }]
        );
    }

    #[test]
    fn simple_baddy_arg_parser_stops_like_c_nextnv_on_malformed_pair() {
        let parsed = parse_simple_baddy_driver_args("aggressive=1; broken 7; helper=1;");

        assert_eq!(parsed.data.aggressive, 1);
        assert_eq!(parsed.data.helper, 0);
        assert!(parsed.unknown.is_empty());
    }

    #[test]
    fn simple_baddy_create_initializes_state_and_item_body_flags() {
        let mut character = test_character();
        character.flags.insert(CharacterFlags::NOBODY);
        character.inventory[30] = Some(ItemId(77));
        character.push_driver_message(NT_CREATE, 0, 0, 0);

        let unknown = apply_simple_baddy_create_message(
            &mut character,
            Some("aggressive=1; startdist=9; drinkinvpots=1; unknown=7;"),
            1234,
        );

        assert_eq!(
            unknown,
            vec![UnknownSimpleBaddyArgument {
                name: "unknown".to_string(),
                value: "7".to_string(),
            }]
        );
        assert!(!character.flags.contains(CharacterFlags::NOBODY));
        assert!(character.flags.contains(CharacterFlags::ITEMDEATH));
        assert!(character.driver_messages.is_empty());

        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.aggressive, 1);
        assert_eq!(data.startdist, 9);
        assert_eq!(data.drink_inventory_potions, 1);
        assert_eq!(data.creation_time, 1234);

        // C `fight_driver_set_dist(cn, dat->startdist, dat->chardist,
        // dat->stopdist)` (`simple_baddy.c:189`): the independent
        // `DRD_FIGHTDRIVER` slot gets seeded from the same freshly-parsed
        // distances, not just `simple_baddy`'s own copy.
        let fight_driver = character.fight_driver.expect("fight driver state missing");
        assert_eq!(fight_driver.start_dist, 9);
        assert_eq!(fight_driver.char_dist, 0);
        assert_eq!(fight_driver.stop_dist, 40);
    }

    #[test]
    fn simple_baddy_create_reseeds_fight_driver_distances_without_clearing_enemies() {
        // C `fight_driver_set_dist` only ever writes `start_dist`/
        // `char_dist`/`stop_dist` - a re-creation (e.g. `#reset`-style
        // template reload) must not wipe out already-tracked enemies, home
        // position, or last-hit tick.
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.fight_driver = Some(FightDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: crate::ids::CharacterId(2),
                priority: 1,
                last_seen_tick: 5,
                visible: true,
                last_x: 11,
                last_y: 12,
            }],
            start_dist: 20,
            stop_dist: 40,
            char_dist: 0,
            home_x: 11,
            home_y: 12,
            last_hit: 7,
        });
        character.push_driver_message(NT_CREATE, 0, 0, 0);

        apply_simple_baddy_create_message(&mut character, Some("startdist=6; stopdist=12;"), 42);

        let fight_driver = character.fight_driver.expect("fight driver state missing");
        assert_eq!(fight_driver.start_dist, 6);
        assert_eq!(fight_driver.stop_dist, 12);
        assert_eq!(fight_driver.char_dist, 0);
        assert_eq!(fight_driver.home_x, 11);
        assert_eq!(fight_driver.home_y, 12);
        assert_eq!(fight_driver.last_hit, 7);
        assert_eq!(fight_driver.enemies.len(), 1);
        assert_eq!(
            fight_driver.enemies[0].target_id,
            crate::ids::CharacterId(2)
        );
    }

    #[test]
    fn lab2_undead_create_parses_legacy_args_and_graveyard_patrol() {
        let mut character = test_character();
        character.push_driver_message(NT_CREATE, 0, 0, 0);

        let unknown = apply_lab2_undead_create_message(
            &mut character,
            Some("aggressive=1; helper=1; patrol=1; undead=1; strange=7;"),
        );

        assert_eq!(
            unknown,
            vec![UnknownSimpleBaddyArgument {
                name: "strange".to_string(),
                value: "7".to_string(),
            }]
        );
        assert!(character.driver_messages.is_empty());
        let Some(CharacterDriverState::Lab2Undead(data)) = character.driver_state else {
            panic!("lab2 undead state missing");
        };
        assert_eq!(data.aggressive, 1);
        assert_eq!(data.helper, 0);
        assert_eq!(data.undead, 1);
        assert_eq!(data.patrol, 1);
        assert_eq!(data.patstep, 4);
        assert_eq!(&data.patx[..4], &[168, 168, 204, 204]);
        assert_eq!(&data.paty[..4], &[178, 218, 218, 178]);
    }

    #[test]
    fn lab2_undead_crypt_patrol_matches_c_coordinate_table() {
        let mut character = test_character();

        apply_lab2_undead_create_message(&mut character, Some("helper=1; patrol=2;"));

        let Some(CharacterDriverState::Lab2Undead(data)) = character.driver_state else {
            panic!("lab2 undead state missing");
        };
        assert_eq!(data.helper, 0);
        assert_eq!(data.patstep, 8);
        assert_eq!(data.patx, [171, 138, 138, 165, 167, 138, 138, 171]);
        assert_eq!(data.paty, [164, 164, 146, 146, 146, 146, 164, 164]);
    }

    #[test]
    fn simple_baddy_gothit_uses_matching_inventory_potions_when_low() {
        let mut character = test_character();
        character.values[1][CharacterValue::Hp as usize] = 20;
        character.values[1][CharacterValue::Mana as usize] = 20;
        character.hp = 9 * POWERSCALE;
        character.mana = 4 * POWERSCALE;
        character.inventory[30] = Some(ItemId(30));
        character.inventory[31] = Some(ItemId(31));
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            drink_inventory_potions: 1,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_GOTHIT, 0, 0, 0);

        let outcomes = process_simple_baddy_messages(
            &mut character,
            &[
                test_item(ItemId(30), IDR_POTION, &[0, 1, 0]),
                test_item(ItemId(31), IDR_POTION, &[0, 0, 1]),
            ],
        );

        assert_eq!(
            outcomes,
            vec![
                SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id: ItemId(30),
                    reason: PotionUseReason::LowHp,
                },
                SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id: ItemId(31),
                    reason: PotionUseReason::LowMana,
                },
                SimpleBaddyMessageOutcome::NoteHit,
            ]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_gothit_ignores_disabled_or_wrong_potions() {
        let mut character = test_character();
        character.values[1][CharacterValue::Hp as usize] = 20;
        character.hp = 9 * POWERSCALE;
        character.inventory[29] = Some(ItemId(29));
        character.inventory[30] = Some(ItemId(30));
        character.inventory[31] = Some(ItemId(31));
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            drink_inventory_potions: 1,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_GOTHIT, 0, 0, 0);

        let outcomes = process_simple_baddy_messages(
            &mut character,
            &[
                test_item(ItemId(29), IDR_POTION, &[0, 1, 0]),
                test_item(ItemId(30), 999, &[0, 1, 0]),
                test_item(ItemId(31), IDR_POTION, &[0, 0, 1]),
            ],
        );

        assert_eq!(outcomes, vec![SimpleBaddyMessageOutcome::NoteHit]);
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_char_messages_emit_ordered_helper_bless_candidates() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helper: 1,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_CHAR, 2, 0, 0);
        character.push_driver_message(NT_CHAR, 3, 0, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![
                SimpleBaddyMessageOutcome::BlessFriend {
                    target_id: crate::ids::CharacterId(2),
                },
                SimpleBaddyMessageOutcome::BlessFriend {
                    target_id: crate::ids::CharacterId(3),
                },
            ]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_char_message_ignores_bless_when_helper_disabled() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.push_driver_message(NT_CHAR, 2, 0, 0);

        assert!(process_simple_baddy_messages(&mut character, &[]).is_empty());
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_standard_messages_emit_aggro_outcomes() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            aggressive: 1,
            helper: 1,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_CHAR, 2, 0, 0);
        character.push_driver_message(NT_SEEHIT, 3, 4, 0);
        character.push_driver_message(NT_GOTHIT, 5, 10, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![
                SimpleBaddyMessageOutcome::BlessFriend {
                    target_id: crate::ids::CharacterId(2),
                },
                SimpleBaddyMessageOutcome::StandardAggro {
                    target_id: crate::ids::CharacterId(2),
                    priority: 0,
                    require_visible: true,
                    hurtme: false,
                },
                SimpleBaddyMessageOutcome::StandardSeenHit {
                    attacker_id: crate::ids::CharacterId(3),
                    victim_id: crate::ids::CharacterId(4),
                },
                SimpleBaddyMessageOutcome::NoteHit,
                SimpleBaddyMessageOutcome::StandardAggro {
                    target_id: crate::ids::CharacterId(5),
                    priority: 1,
                    require_visible: false,
                    hurtme: true,
                },
            ]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_text_message_preserves_tabunga_notification_boundary() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.push_driver_message(NT_TEXT, 0, 12345, 7);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::TextNotification {
                speaker_id: crate::ids::CharacterId(7),
                text_token: 12345,
                text: None,
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_text_message_preserves_optional_text_payload() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.push_driver_text_message(crate::ids::CharacterId(7), "Tabunga please");

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::TextNotification {
                speaker_id: crate::ids::CharacterId(7),
                text_token: 0,
                text: Some("Tabunga please".to_string()),
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_didhit_emits_poison_hit_outcome() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            poison_power: 7,
            poison_type: 2,
            poison_chance: 35,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_DIDHIT, 42, 3, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::PoisonHit {
                target_id: crate::ids::CharacterId(42),
                power: 7,
                poison_type: 2,
                chance: 35,
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_didhit_requires_power_target_and_damage() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            poison_power: 7,
            poison_type: 2,
            poison_chance: 100,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_DIDHIT, 0, 3, 0);
        character.push_driver_message(NT_DIDHIT, 42, 0, 0);

        assert!(process_simple_baddy_messages(&mut character, &[]).is_empty());
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_npc_message_emits_helpid_enemy_outcome() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helpid: NTID_GLADIATOR,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_NPC, NTID_MERCHANT, 2, 99);
        character.push_driver_message(NT_NPC, NTID_GLADIATOR, 2, 99);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::AddEnemy {
                caller_id: crate::ids::CharacterId(2),
                target_id: crate::ids::CharacterId(99),
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_npc_message_preserves_zero_target_like_c() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helpid: NTID_GLADIATOR,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_NPC, NTID_GLADIATOR, 2, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::AddEnemy {
                caller_id: crate::ids::CharacterId(2),
                target_id: crate::ids::CharacterId(0),
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_dead_message_emits_remove_enemy_outcome() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.push_driver_message(NT_DEAD, 42, 7, 0);
        character.push_driver_message(NT_DEAD, 0, 7, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::RemoveEnemy {
                target_id: crate::ids::CharacterId(42),
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn add_simple_baddy_enemy_requires_same_group_caller_and_updates_existing() {
        let mut character = test_character();
        character.group = 7;
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        let mut caller = test_character();
        caller.id = crate::ids::CharacterId(2);
        caller.group = 8;

        assert!(!add_simple_baddy_enemy(
            &mut character,
            &caller,
            crate::ids::CharacterId(99),
            10,
        ));

        caller.group = 7;
        assert!(add_simple_baddy_enemy(
            &mut character,
            &caller,
            crate::ids::CharacterId(99),
            10,
        ));
        assert!(!add_simple_baddy_enemy(
            &mut character,
            &caller,
            crate::ids::CharacterId(99),
            12,
        ));

        let data = character.fight_driver.expect("fight driver state missing");
        assert_eq!(
            data.enemies,
            vec![SimpleBaddyEnemy {
                target_id: crate::ids::CharacterId(99),
                priority: 1,
                last_seen_tick: 12,
                visible: false,
                last_x: 0,
                last_y: 0,
            }]
        );
    }

    #[test]
    fn add_simple_baddy_enemy_keeps_legacy_ten_entry_table() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));

        for target in 10..22 {
            assert!(add_simple_baddy_enemy_unchecked(
                &mut character,
                crate::ids::CharacterId(target),
                0,
                target as i32,
            ));
        }

        let data = character.fight_driver.expect("fight driver state missing");
        assert_eq!(data.enemies.len(), 10);
        assert_eq!(data.enemies[0].target_id, crate::ids::CharacterId(10));
        assert_eq!(data.enemies[8].target_id, crate::ids::CharacterId(18));
        assert_eq!(data.enemies[9].target_id, crate::ids::CharacterId(21));
    }

    #[test]
    fn add_simple_baddy_enemy_matches_c_slot_nine_overflow_semantics() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));

        for target in 1..=10 {
            assert!(add_simple_baddy_enemy_unchecked(
                &mut character,
                crate::ids::CharacterId(target),
                0,
                target as i32,
            ));
        }

        assert!(add_simple_baddy_enemy_unchecked(
            &mut character,
            crate::ids::CharacterId(10),
            1,
            99,
        ));

        let data = character.fight_driver.expect("fight driver state missing");
        assert_eq!(data.enemies.len(), 10);
        assert_eq!(data.enemies[9].target_id, crate::ids::CharacterId(10));
        assert_eq!(data.enemies[9].priority, 1);
        assert_eq!(data.enemies[9].last_seen_tick, 99);
    }

    #[test]
    fn add_simple_baddy_enemy_overwrites_priority_like_c_hurtme_flag() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));

        assert!(add_simple_baddy_enemy_unchecked(
            &mut character,
            crate::ids::CharacterId(2),
            1,
            10,
        ));
        assert!(!add_simple_baddy_enemy_unchecked(
            &mut character,
            crate::ids::CharacterId(2),
            0,
            11,
        ));

        let data = character.fight_driver.expect("fight driver state missing");
        assert_eq!(data.enemies[0].priority, 0);
        assert_eq!(data.enemies[0].last_seen_tick, 11);
    }

    #[test]
    fn remove_simple_baddy_enemy_matches_fight_driver_remove_boundary() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.fight_driver = Some(FightDriverData {
            enemies: vec![
                SimpleBaddyEnemy {
                    target_id: crate::ids::CharacterId(2),
                    priority: 0,
                    last_seen_tick: 10,
                    visible: true,
                    last_x: 20,
                    last_y: 21,
                },
                SimpleBaddyEnemy {
                    target_id: crate::ids::CharacterId(3),
                    priority: 1,
                    last_seen_tick: 11,
                    visible: false,
                    last_x: 30,
                    last_y: 31,
                },
            ],
            ..FightDriverData::default()
        });

        assert!(remove_simple_baddy_enemy(
            &mut character,
            crate::ids::CharacterId(2),
        ));
        assert!(!remove_simple_baddy_enemy(
            &mut character,
            crate::ids::CharacterId(99),
        ));

        let data = character.fight_driver.expect("fight driver state missing");
        assert_eq!(data.enemies.len(), 1);
        assert_eq!(data.enemies[0].target_id, crate::ids::CharacterId(3));
    }

    #[test]
    fn remove_simple_baddy_enemy_ignores_missing_fight_driver_data() {
        // No `driver_state` gate anymore (matches C's driver-independent
        // `DRD_FIGHTDRIVER` slot) - this now only exercises the "no
        // `fight_driver` data at all yet" early return.
        let mut character = test_character();

        assert!(!remove_simple_baddy_enemy(
            &mut character,
            crate::ids::CharacterId(2),
        ));
    }

    #[test]
    fn add_and_remove_simple_baddy_enemy_work_without_simple_baddy_driver_state() {
        // C `fight_driver_add_enemy`/`fight_driver_remove_enemy` operate on
        // any character's independent `DRD_FIGHTDRIVER` slot - a
        // `CDR_LOSTCON` lingering character (or, eventually, a normal
        // playing character) has no `SimpleBaddyDriverData` at all.
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
            deadline: 0,
        }));

        assert!(add_simple_baddy_enemy_unchecked(
            &mut character,
            crate::ids::CharacterId(2),
            1,
            10,
        ));
        assert_eq!(character.fight_driver.as_ref().unwrap().enemies.len(), 1);

        assert!(remove_simple_baddy_enemy(
            &mut character,
            crate::ids::CharacterId(2),
        ));
        assert!(character.fight_driver.unwrap().enemies.is_empty());
    }

    #[test]
    fn clara_dialogue_ports_initial_report_state_machine() {
        let outcome = clara_dialogue_step(clara_context(0, 0));
        assert_eq!(outcome.clara_state, 1);
        assert_eq!(
            outcome.text.as_deref(),
            Some(
                "Greetings, Hero! I am Clara, First Sergeant of the Seyan'Du and commander of this outpost."
            )
        );

        let blocked = clara_dialogue_step(clara_context(1, 14));
        assert_eq!(blocked.clara_state, 1);
        assert_eq!(blocked.text, None);

        let report = clara_dialogue_step(clara_context(1, 15));
        assert_eq!(report.clara_state, 3);
        assert_eq!(
            report.text.as_deref(),
            Some(
                "I assume thou hast been sent from Aston, Private, to report on our status. The road through the swamp is no longer secure and we have been under attack from beasts emerging from the swamp."
            )
        );

        let dismissed = clara_dialogue_step(clara_context(4, 15));
        assert_eq!(dismissed.clara_state, 5);
        assert_eq!(
            dismissed.text.as_deref(),
            Some(
                "Afterwards come back here, I have more work for thee. That will be all, Private. Dismissed!"
            )
        );
    }

    #[test]
    fn clara_dialogue_ports_hardkill_quest_gates_and_rewards() {
        let blocked = clara_dialogue_step(clara_context(5, 17));
        assert_eq!(blocked.clara_state, 5);
        assert_eq!(blocked.text, None);

        let mission = clara_dialogue_step(clara_context(5, 18));
        assert_eq!(mission.clara_state, 7);
        assert_eq!(mission.open_questlog, Some(21));
        assert_eq!(
            mission.text.as_deref(),
            Some(
                "I have a difficult mission for thee, Hero. The main reason we had to retreat to this camp was one huge swamp beast. It seemed to be immune to our attacks."
            )
        );

        let no_hardkill = clara_dialogue_step(clara_context(9, 18));
        assert_eq!(no_hardkill.clara_state, 9);
        assert_eq!(no_hardkill.text, None);

        let mut context = clara_context(9, 18);
        context.has_hardkill_item = true;
        context.hardkill_ritual_progress = 24;
        let partial_ritual = clara_dialogue_step(context);
        assert_eq!(partial_ritual.clara_state, 11);
        assert_eq!(partial_ritual.military_points, 4);
        assert_eq!(partial_ritual.military_exp, EXP_AREA15_HARDKILL);
        assert_eq!(
            partial_ritual.text.as_deref(),
            Some(
                "So that is how one can kill them. Thou wilt need to find all three stone circles and perform the ritual in each one, then, Hero."
            )
        );

        let mut context = clara_context(11, 18);
        context.has_hardkill_item = true;
        context.hardkill_ritual_progress = 36;
        let ready_to_kill = clara_dialogue_step(context);
        assert_eq!(ready_to_kill.clara_state, 13);
        assert_eq!(
            ready_to_kill.text.as_deref(),
            Some("Now that thou knowest how to kill that beast, please go and do it.")
        );

        let mut context = clara_context(14, 18);
        context.questlog_21_count = 1;
        let done = clara_dialogue_step(context);
        assert_eq!(done.clara_state, 15);
        assert_eq!(done.complete_questlog, Some(21));
        assert_eq!(done.military_points, 8);
        assert_eq!(done.military_exp, 1);
        assert_eq!(done.text.as_deref(), Some("Well done indeed, Hero!"));
    }

    #[test]
    fn clara_replay_and_monster_death_match_c_state_boundaries() {
        assert_eq!(clara_replay_state_after_text_analysis(5, 2), 0);
        assert_eq!(clara_replay_state_after_text_analysis(9, 2), 6);
        assert_eq!(clara_replay_state_after_text_analysis(11, 2), 10);
        assert_eq!(clara_replay_state_after_text_analysis(13, 2), 12);
        assert_eq!(clara_replay_state_after_text_analysis(16, 2), 15);
        assert_eq!(clara_replay_state_after_text_analysis(14, 2), 14);
        assert_eq!(clara_replay_state_after_text_analysis(13, 1), 13);

        assert_eq!(clara_state_after_swamp_monster_death(12, true, true), 14);
        assert_eq!(clara_state_after_swamp_monster_death(13, true, true), 14);
        assert_eq!(clara_state_after_swamp_monster_death(11, true, true), 11);
        assert_eq!(clara_state_after_swamp_monster_death(12, false, true), 12);
        assert_eq!(clara_state_after_swamp_monster_death(12, true, false), 12);
    }

    fn gate_context(
        welcome_state: i32,
        needs_lab: bool,
        flags: CharacterFlags,
    ) -> GateWelcomeContext<'static> {
        GateWelcomeContext {
            player_name: "Hero",
            welcome_state,
            needs_lab,
            flags,
        }
    }

    #[test]
    fn gatekeeper_qa_matches_c_table_words_and_codes() {
        assert_eq!(
            analyse_text_qa("how are you", "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Said("I'm fine!".to_string())
        );
        assert_eq!(
            analyse_text_qa("hello", "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Said("Hello, Hero!".to_string())
        );
        assert_eq!(
            analyse_text_qa("repeat", "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Matched(2)
        );
        assert_eq!(
            analyse_text_qa("please restart", "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Matched(2)
        );
        assert_eq!(
            analyse_text_qa("aye", "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Matched(3)
        );
        assert_eq!(
            analyse_text_qa("nay", "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Matched(4)
        );
        // Every accepted class-choice spelling variant maps to the same
        // `answer_code` C's table does (`gatekeeper.c:97-109`).
        for phrase in ["arch warrior", "arch-warrior"] {
            assert_eq!(
                analyse_text_qa(phrase, "Gatekeeper", "Hero", GATEKEEPER_QA),
                TextAnalysisOutcome::Matched(5),
                "phrase={phrase}"
            );
        }
        for phrase in ["arch mage", "arch-mage"] {
            assert_eq!(
                analyse_text_qa(phrase, "Gatekeeper", "Hero", GATEKEEPER_QA),
                TextAnalysisOutcome::Matched(6),
                "phrase={phrase}"
            );
        }
        for phrase in [
            "arch-seyan du",
            "arch seyan du",
            "arch-seyan'du",
            "arch seyan'du",
            "arch seyan",
            "arch-seyan",
        ] {
            assert_eq!(
                analyse_text_qa(phrase, "Gatekeeper", "Hero", GATEKEEPER_QA),
                TextAnalysisOutcome::Matched(7),
                "phrase={phrase}"
            );
        }
        for phrase in ["seyan du", "seyan'du", "seyan"] {
            assert_eq!(
                analyse_text_qa(phrase, "Gatekeeper", "Hero", GATEKEEPER_QA),
                TextAnalysisOutcome::Matched(8),
                "phrase={phrase}"
            );
        }
        assert_eq!(
            analyse_text_qa("reset", "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Matched(9)
        );
        assert_eq!(
            analyse_text_qa("what's up", "Gatekeeper", "Hero", GATEKEEPER_QA),
            TextAnalysisOutcome::Said("Everything that isn't nailed down.".to_string())
        );
    }

    #[test]
    fn gate_welcome_dialogue_greets_then_explains_the_test() {
        let flags = CharacterFlags::USED;
        let greet = gate_welcome_dialogue_step(gate_context(0, false, flags));
        assert_eq!(greet.welcome_state, 1);
        assert_eq!(
            greet.text.as_deref(),
            Some(
                "Be greeted, Hero. These are the halls of Ishtar. Only the greatest fighters and magic users come here, to take the final test and fight the Gatekeeper."
            )
        );

        let explain = gate_welcome_dialogue_step(gate_context(1, false, flags));
        assert_eq!(explain.welcome_state, 2);
        assert!(explain
            .text
            .unwrap()
            .starts_with("Those who succeed in this test"));
    }

    #[test]
    fn gate_welcome_dialogue_sends_to_labyrinth_when_needed_and_waits() {
        let flags = CharacterFlags::USED;
        let sent = gate_welcome_dialogue_step(gate_context(2, true, flags));
        assert_eq!(sent.welcome_state, 3);
        assert_eq!(
            sent.text.as_deref(),
            Some(
                "Before thou mayest engage the Gatekeeper, thou must solve the Labyrinth built by Ishtar. Thou canst enter the labyrinth through the door to the east."
            )
        );

        // Re-entering at state 3 while the labyrinth is still unsolved:
        // C `case 3`'s `else break;` - no text, no state change.
        let waiting = gate_welcome_dialogue_step(gate_context(3, true, flags));
        assert_eq!(waiting.welcome_state, 3);
        assert_eq!(waiting.text, None);
    }

    #[test]
    fn gate_welcome_dialogue_offers_class_choice_when_lab_already_solved() {
        // Fast path: state 2 with no labyrinth requirement falls through
        // case 3 into case 4 in the same call, ending at state 6 and
        // skipping the `case 5` "name the class" message entirely
        // (`gatekeeper.c`'s undocumented quirk - see `gate_case4` doc).
        let single_class = gate_welcome_dialogue_step(gate_context(
            2,
            false,
            CharacterFlags::USED | CharacterFlags::WARRIOR,
        ));
        assert_eq!(single_class.welcome_state, 6);
        assert_eq!(
            single_class.text.as_deref(),
            Some(
                "The choice is hard, and so is the test. If thou wishest to take the test, decide which path to follow. That of the Arch-Warrior, or that of the Seyan'Du."
            )
        );

        let seyan_already = gate_welcome_dialogue_step(gate_context(
            2,
            false,
            CharacterFlags::USED | CharacterFlags::WARRIOR | CharacterFlags::MAGE,
        ));
        assert_eq!(seyan_already.welcome_state, 6);
        assert_eq!(
            seyan_already.text.as_deref(),
            Some("Since thou art already a Seyan'Du, thy only choice is to become Arch-Seyan'Du.")
        );

        let arch_already = gate_welcome_dialogue_step(gate_context(
            2,
            false,
            CharacterFlags::USED | CharacterFlags::WARRIOR | CharacterFlags::ARCH,
        ));
        assert_eq!(arch_already.welcome_state, 6);
        assert_eq!(
            arch_already.text.as_deref(),
            Some("There is nothing I can do for thee, Hero, though, since thou art already an Arch-Warrior.")
        );
    }

    #[test]
    fn gate_welcome_dialogue_slow_path_ends_one_state_lower_than_fast_path() {
        // Slow path: entering directly at state 3 (labyrinth requirement
        // just got satisfied since the last call) falls through case 3
        // into case 4 with `state == 4` on entry, so the non-arch
        // branches' `welcome_state++` lands on `5`, not `6` - the next
        // call will show the `case 5` "name the class" message that the
        // fast path (`gate_welcome_dialogue_offers_class_choice_when_
        // lab_already_solved`) never shows.
        let slow = gate_welcome_dialogue_step(gate_context(
            3,
            false,
            CharacterFlags::USED | CharacterFlags::WARRIOR,
        ));
        assert_eq!(slow.welcome_state, 5);
        assert_eq!(
            slow.text.as_deref(),
            Some(
                "The choice is hard, and so is the test. If thou wishest to take the test, decide which path to follow. That of the Arch-Warrior, or that of the Seyan'Du."
            )
        );

        let name_class = gate_welcome_dialogue_step(gate_context(
            5,
            false,
            CharacterFlags::USED | CharacterFlags::WARRIOR,
        ));
        assert_eq!(name_class.welcome_state, 6);
        assert_eq!(
            name_class.text.as_deref(),
            Some("Name the class thou wishest to become to begin the test. Each try will cost thee 100 gold coins.")
        );
    }

    #[test]
    fn gate_welcome_dialogue_waits_silently_at_state_six() {
        let waiting = gate_welcome_dialogue_step(gate_context(6, false, CharacterFlags::USED));
        assert_eq!(waiting.welcome_state, 6);
        assert_eq!(waiting.text, None);
    }

    #[test]
    fn gate_welcome_state_after_repeat_resets_only_below_state_seven() {
        assert_eq!(gate_welcome_state_after_repeat(0), 0);
        assert_eq!(gate_welcome_state_after_repeat(6), 0);
        assert_eq!(gate_welcome_state_after_repeat(7), 7);
    }

    #[test]
    fn needs_next_lab_is_true_until_every_checkpoint_is_solved() {
        // Nothing solved: level 10 is the first checkpoint bit checked.
        assert!(needs_next_lab(0));
        // All five checkpoints solved: no lab needed anymore.
        let all_solved = (1_u64 << 10) | (1 << 15) | (1 << 20) | (1 << 25) | (1 << 30);
        assert!(!needs_next_lab(all_solved));
        // Missing just the last checkpoint still counts as needing a lab.
        let all_but_last = (1_u64 << 10) | (1 << 15) | (1 << 20) | (1 << 25);
        assert!(needs_next_lab(all_but_last));
        // Bits outside the known checkpoints (e.g. bit 0) never matter.
        assert!(needs_next_lab(1));
        assert!(!needs_next_lab(all_solved | 1));
    }

    #[test]
    fn gate_enter_test_precheck_orders_preconditions_like_c() {
        let base = GateEnterTestPrecheck {
            is_paid: true,
            needs_lab: false,
            is_god: false,
            is_noexp: false,
            flags: CharacterFlags::USED | CharacterFlags::WARRIOR,
            carried_item_count: 0,
            class: 5,
        };

        assert_eq!(
            gate_enter_test_precheck(GateEnterTestPrecheck {
                is_paid: false,
                ..base
            }),
            GateEnterTestOutcome::NotPaid
        );
        assert_eq!(
            gate_enter_test_precheck(GateEnterTestPrecheck {
                needs_lab: true,
                ..base
            }),
            GateEnterTestOutcome::LabNotSolved
        );
        // CF_GOD bypasses the labyrinth check but not CF_PAID/CF_NOEXP.
        assert_eq!(
            gate_enter_test_precheck(GateEnterTestPrecheck {
                needs_lab: true,
                is_god: true,
                ..base
            }),
            GateEnterTestOutcome::Ready
        );
        assert_eq!(
            gate_enter_test_precheck(GateEnterTestPrecheck {
                is_noexp: true,
                ..base
            }),
            GateEnterTestOutcome::NoExpMode
        );
        assert_eq!(
            gate_enter_test_precheck(GateEnterTestPrecheck {
                flags: CharacterFlags::USED | CharacterFlags::WARRIOR | CharacterFlags::MAGE,
                ..base
            }),
            GateEnterTestOutcome::InvalidClass
        );
        assert_eq!(
            gate_enter_test_precheck(GateEnterTestPrecheck {
                carried_item_count: 2,
                ..base
            }),
            GateEnterTestOutcome::CarryingItems { count: 2 }
        );
        assert_eq!(gate_enter_test_precheck(base), GateEnterTestOutcome::Ready);

        // Seyan'Du (class 8) tolerates up to three carried items.
        let seyan = GateEnterTestPrecheck {
            flags: CharacterFlags::USED,
            class: 8,
            carried_item_count: 3,
            ..base
        };
        assert_eq!(gate_enter_test_precheck(seyan), GateEnterTestOutcome::Ready);
        assert_eq!(
            gate_enter_test_precheck(GateEnterTestPrecheck {
                carried_item_count: 4,
                ..seyan
            }),
            GateEnterTestOutcome::CarryingTooManyItems { count: 4 }
        );

        // CF_GOD also bypasses class/item-count validation entirely.
        assert_eq!(
            gate_enter_test_precheck(GateEnterTestPrecheck {
                is_god: true,
                flags: CharacterFlags::USED | CharacterFlags::ARCH,
                carried_item_count: 99,
                ..base
            }),
            GateEnterTestOutcome::Ready
        );
    }

    #[test]
    fn gate_class_choice_validation_matches_c_flag_checks() {
        use CharacterFlags as F;
        // Arch-Warrior (5): blocked if already MAGE or ARCH.
        assert!(gate_class_choice_is_valid(F::USED | F::WARRIOR, 5));
        assert!(!gate_class_choice_is_valid(F::USED | F::MAGE, 5));
        assert!(!gate_class_choice_is_valid(F::USED | F::ARCH, 5));

        // Arch-Mage (6): blocked if already WARRIOR or ARCH.
        assert!(gate_class_choice_is_valid(F::USED | F::MAGE, 6));
        assert!(!gate_class_choice_is_valid(F::USED | F::WARRIOR, 6));

        // Arch-Seyan'Du (7): requires both WARRIOR and MAGE, not ARCH.
        assert!(gate_class_choice_is_valid(
            F::USED | F::WARRIOR | F::MAGE,
            7
        ));
        assert!(!gate_class_choice_is_valid(F::USED | F::WARRIOR, 7));
        assert!(!gate_class_choice_is_valid(
            F::USED | F::WARRIOR | F::MAGE | F::ARCH,
            7
        ));

        // Seyan'Du (8): blocked if already ARCH or already both WARRIOR+MAGE.
        assert!(gate_class_choice_is_valid(F::USED | F::WARRIOR, 8));
        assert!(gate_class_choice_is_valid(F::USED, 8));
        assert!(!gate_class_choice_is_valid(
            F::USED | F::WARRIOR | F::MAGE,
            8
        ));
        assert!(!gate_class_choice_is_valid(F::USED | F::ARCH, 8));

        // Unknown class values are always invalid (C's `default: return 0;`).
        assert!(!gate_class_choice_is_valid(F::USED, 0));
        assert!(!gate_class_choice_is_valid(F::USED, 99));
    }

    fn test_character() -> Character {
        Character {
            merchant: None,
            template_key: String::new(),
            respawn_ticks: 0,
            id: crate::ids::CharacterId(1),
            serial: 1,
            name: "Rat".to_string(),
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
            x: 0,
            y: 0,
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
            level: 0,
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
            driver_memory: DriverMemory::default(),
            class: 0,
            dungeonfighter: None,
            fight_driver: None,
        }
    }

    fn clara_context(clara_state: i32, kelly_state: i32) -> ClaraDialogueContext<'static> {
        ClaraDialogueContext {
            player_name: "Hero",
            clara_name: "Clara",
            army_rank: "Private",
            kelly_state,
            clara_state,
            has_hardkill_item: false,
            hardkill_ritual_progress: 0,
            questlog_21_count: 0,
        }
    }

    fn test_item(id: ItemId, driver: u16, driver_data: &[u8]) -> Item {
        Item {
            id,
            name: String::new(),
            description: String::new(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; crate::entity::MAX_MODIFIERS],
            modifier_value: [0; crate::entity::MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver,
            driver_data: driver_data.to_vec(),
            serial: 0,
        }
    }
}
// Re-exports keep the historical `crate::character_driver::X` paths
// stable while each NPC owns its file under `world::npc`.
pub use crate::world::npc::aclerk::{parse_aclerk_driver_args, AclerkDriverData};
pub use crate::world::npc::area1::asturin::AsturinDriverData;
pub use crate::world::npc::area1::balltrap::BalltrapDriverData;
pub use crate::world::npc::area1::brithildie::BrithildieDriverData;
pub use crate::world::npc::area1::camhermit::CamhermitDriverData;
pub use crate::world::npc::area1::forest_ranger::ForestRangerDriverData;
pub use crate::world::npc::area1::greeter::GreeterDriverData;
pub use crate::world::npc::area1::guiwynn::GuiwynnDriverData;
pub use crate::world::npc::area1::gwendylon::{GwendylonDriverData, GWENDYLON_QA};
pub use crate::world::npc::area1::james::JamesDriverData;
pub use crate::world::npc::area1::jessica::JessicaDriverData;
pub use crate::world::npc::area1::jiu::JiuDriverData;
pub use crate::world::npc::area1::logain::LogainDriverData;
pub use crate::world::npc::area1::lydia::LydiaDriverData;
pub use crate::world::npc::area1::nook::NookDriverData;
pub use crate::world::npc::area1::reskin::ReskinDriverData;
pub use crate::world::npc::area1::robber::RobberDriverData;
pub use crate::world::npc::area1::sanoa::SanoaDriverData;
pub use crate::world::npc::area1::terion::TerionDriverData;
pub use crate::world::npc::area1::yoakin::YoakinDriverData;
pub use crate::world::npc::area13::dungeon_master::{
    DungeonfighterDriverData, DungeonmasterDriverData, DUNGEONMASTER_QA, DUNGEON_SLOT_COUNT,
};
pub use crate::world::npc::area17::two_skelly::TwoSkellyDriverData;
pub use crate::world::npc::area2::moonie::MoonieDriverData;
pub use crate::world::npc::area2::superior::SuperiorDriverData;
pub use crate::world::npc::area2::vampire::VampireDriverData;
pub use crate::world::npc::area2::vampire2::Vampire2DriverData;
pub use crate::world::npc::area22::lab2_undead::{
    apply_lab2_undead_create_message, parse_lab2_undead_driver_args, Lab2UndeadDriverData,
};
pub use crate::world::npc::area3::clara::{
    clara_dialogue_step, clara_replay_state_after_text_analysis,
    clara_state_after_swamp_monster_death, ClaraDialogueContext, ClaraDialogueOutcome,
    ClaraDriverData,
};
pub use crate::world::npc::area30::clanclerk::{parse_clanclerk_driver_args, ClanclerkDriverData};
pub use crate::world::npc::area30::clanmaster::{
    parse_clanmaster_driver_args, ClanmasterDriverData, CLANMASTER_QA,
};
pub use crate::world::npc::area32::military::{
    parse_military_advisor_driver_args, parse_military_master_driver_args,
    MilitaryAdvisorDriverData, MilitaryMasterDriverData, MILITARY_QA,
};
pub use crate::world::npc::arena::{
    parse_arena_manager_driver_args, ArenaContender, ArenaFighterDriverData,
    ArenaManagerDriverData, ArenaMasterDriverData, ARENA_FIGHTER_MASTER_POS,
    ARENA_FIGHTER_REST_POS, ARENA_MAX_CONTENDER, ARENA_QA,
};
pub use crate::world::npc::bank::{BankDriverData, BANK_QA};
pub use crate::world::npc::clubmaster::{
    parse_clubmaster_driver_args, ClubmasterDriverData, CLUBMASTER_QA,
};
pub use crate::world::npc::gate_fight::GateFightDriverData;
pub use crate::world::npc::gate_welcome::{
    gate_welcome_dialogue_step, gate_welcome_state_after_repeat, GateWelcomeContext,
    GateWelcomeDriverData, GateWelcomeOutcome,
};
pub use crate::world::npc::janitor::JanitorDriverData;
pub use crate::world::npc::lostcon::LostconDriverData;
pub use crate::world::npc::macro_npc::{MacroDriverData, MacroDriverState};
pub use crate::world::npc::merchant::{
    parse_merchant_driver_args, MerchantDriverData, MERCHANT_QA,
};
pub use crate::world::npc::trader::{TraderDriverData, TRADER_QA};
