//! Typed per-character driver state: the `CharacterDriverState` registry enum,
//! driver messages and the generic per-character driver memory.

use super::*;

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
    TwoSanwyn(TwoSanwynDriverData),
    TwoSkelly(TwoSkellyDriverData),
    TwoAlchemist(TwoAlchemistDriverData),
    TwoBarkeeper(TwoBarkeeperDriverData),
    TwoServant(TwoServantDriverData),
    TwoGuard(TwoGuardDriverData),
    TwoThiefGuard(TwoThiefGuardDriverData),
    TwoThiefMaster(TwoThiefMasterDriverData),
    Lab2Undead(Lab2UndeadDriverData),
    Lab2Herald(Lab2HeraldDriverData),
    Lab2Deamon(crate::world::npc::area22::lab2_deamon::Lab2DeamonDriverData),
    LabGnome(crate::world::npc::area22::lab1_gnome::LabGnomeDriverData),
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
    Astro1(Astro1DriverData),
    Astro2(Astro2DriverData),
    Thomas(ThomasDriverData),
    SirJones(SirJonesDriverData),
    Seymour(SeymourDriverData),
    Kelly(KellyDriverData),
    Lampghost(LampghostDriverData),
    Carlos(CarlosDriverData),
    Kassim(KassimDriverData),
    Supermax(SupermaxDriverData),
    Tester(TesterDriverData),
    /// C `struct engrave_data` (`src/area/3/area3.c:318-320`), stored via
    /// `set_data(co, DRD_ENGRAVE_DATA, ...)` on the *player* mid-
    /// transaction with Kassim, not on Kassim himself. Same "player, not
    /// NPC" precedent as [`CharacterDriverState::ClanFound`].
    Engrave(EngraveDriverData),
    FdemonArmy(FarmyData),
    Islena(IslenaDriverData),
    PalaceGuard(PalaceGuardDriverData),
    GolemKeyhold(GolemKeyholdDriverData),
    ForestImp(ForestImpDriverData),
    ForestWilliam(ForestWilliamDriverData),
    ForestHermit(ForestHermitDriverData),
    Nomad(crate::world::npc::area19::NomadDriverData),
    Madhermit(crate::world::npc::area19::MadhermitDriverData),
    LqNpc(crate::world::npc::area20::LqNpcDriverData),
    Lab3Passguard(crate::world::npc::area22::lab3_passguard::Lab3PassguardDriverData),
    Lab3Prisoner(crate::world::npc::area22::lab3_prisoner::Lab3PrisonerDriverData),
    Lab4Seyan(crate::world::npc::area22::lab4_seyan::Lab4SeyanDriverData),
    Lab4Gnalb(crate::world::npc::area22::lab4_gnalb::Lab4GnalbDriverData),
    Lab5Seyan(crate::world::npc::area22::lab5_seyan::Lab5SeyanDriverData),
    Lab5Daemon(crate::world::npc::area22::lab5_daemon::Lab5DaemonDriverData),
    Lab5Mage(crate::world::npc::area22::lab5_mage::Lab5MageDriverData),
    StrategyWorker(crate::world::npc::area23_24::worker::StrategyWorkerDriverData),
    WarpFighter(crate::world::npc::area25::warpfighter::WarpFighterDriverData),
    Warpmaster(crate::world::npc::area25::warpmaster::WarpmasterDriverData),
    SmuggleCom(crate::world::npc::area26::smugglecom::SmuggleComDriverData),
    Rouven(crate::world::npc::area26::rouven::RouvenDriverData),
    Aristocrat(crate::world::npc::area28::aristocrat::AristocratDriverData),
    Yoatin(crate::world::npc::area28::yoatin::YoatinDriverData),
    SpiritBran(crate::world::npc::area29::spiritbran::SpiritBranDriverData),
    GuardBran(crate::world::npc::area29::guardbran::GuardBranDriverData),
    BrennethBran(crate::world::npc::area29::brennethbran::BrennethBranDriverData),
    Broklin(crate::world::npc::area29::broklin::BroklinDriverData),
    CountBran(crate::world::npc::area29::countbran::CountBranDriverData),
    CountessaBran(crate::world::npc::area29::countessabran::CountessaBranDriverData),
    DaughterBran(crate::world::npc::area29::daughterbran::DaughterBranDriverData),
    ForestBran(crate::world::npc::area29::forestbran::ForestBranDriverData),
    Grinnich(crate::world::npc::area29::grinnich::GrinnichDriverData),
    Shanra(crate::world::npc::area29::shanra::ShanraDriverData),
    DwarfChief(crate::world::npc::area31::dwarfchief::DwarfChiefDriverData),
    LostDwarf(crate::world::npc::area31::lostdwarf::LostDwarfDriverData),
    DwarfShaman(crate::world::npc::area31::dwarfshaman::DwarfShamanDriverData),
    DwarfSmith(crate::world::npc::area31::dwarfsmith::DwarfSmithDriverData),
    MissionGiver(crate::world::npc::area32::governor::MissionGiverDriverData),
    Gorwin(crate::world::npc::area33::gorwin::GorwinDriverData),
    TeufelGambler(crate::world::npc::area34::teufelgambler::TeufelGambleDriverData),
    TeufelQuest(crate::world::npc::area34::teufelquest::TeufelQuestDriverData),
    Nop(crate::world::npc::area37::nop::NopDriverData),
    Rammy(crate::world::npc::area37::rammy::RammyDriverData),
    Jaz(crate::world::npc::area37::jaz::JazDriverData),
    Fiona(crate::world::npc::area37::fiona::FionaDriverData),
    BridgeGuard(crate::world::npc::area37::bridgeguard::BridgeGuardDriverData),
    Gladiator(crate::world::npc::area37::gladiator::GladiatorDriverData),
    Ramin(crate::world::npc::area37::ramin::RaminDriverData),
    Arkhatamonk(crate::world::npc::area37::arkhatamonk::ArkhatamonkDriverData),
    Captain(crate::world::npc::area37::captain::CaptainDriverData),
    Judge(crate::world::npc::area37::judge::JudgeDriverData),
    Jada(crate::world::npc::area37::jada::JadaDriverData),
    Potmaker(crate::world::npc::area37::potmaker::PotmakerDriverData),
    Hunter(crate::world::npc::area37::hunter::HunterDriverData),
    Thaipan(crate::world::npc::area37::thaipan::ThaipanDriverData),
    Trainer(crate::world::npc::area37::trainer::TrainerDriverData),
    Kidnappee(crate::world::npc::area37::kidnappee::KidnappeeDriverData),
    Clerk(crate::world::npc::area37::clerk::ClerkDriverData),
    Krenach(crate::world::npc::area37::krenach::KrenachDriverData),
    Professor(crate::world::npc::professor::ProfessorDriverData),
}
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
    pub(super) slots: [Vec<u32>; DRIVER_MEMORY_SLOTS],
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
