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

mod dispatch;
mod ids;
mod misc;
mod simple_baddy;
mod state;
mod text_qa;

pub use dispatch::*;
pub use ids::*;
pub use misc::*;
pub use simple_baddy::*;
pub use state::*;
pub use text_qa::*;

#[cfg(test)]
// The path-stable re-export block below intentionally stays at the end of
// this file (see the module layout rules), after this test module.
#[allow(clippy::items_after_test_module)]
mod tests;

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
pub use crate::world::npc::area11::islena::IslenaDriverData;
pub use crate::world::npc::area11::palace_guard::{
    parse_palace_guard_driver_args, PalaceGuardDriverData,
};
pub use crate::world::npc::area12::golemkeyholder::GolemKeyholdDriverData;
pub use crate::world::npc::area13::dungeon_master::{
    DungeonfighterDriverData, DungeonmasterDriverData, DUNGEONMASTER_QA, DUNGEON_SLOT_COUNT,
};
pub use crate::world::npc::area16::hermit::ForestHermitDriverData;
pub use crate::world::npc::area16::imp::ForestImpDriverData;
pub use crate::world::npc::area16::william::ForestWilliamDriverData;
pub use crate::world::npc::area17::alchemist::TwoAlchemistDriverData;
pub use crate::world::npc::area17::barkeeper::TwoBarkeeperDriverData;
pub use crate::world::npc::area17::guard::TwoGuardDriverData;
pub use crate::world::npc::area17::sanwyn::TwoSanwynDriverData;
pub use crate::world::npc::area17::servant::TwoServantDriverData;
pub use crate::world::npc::area17::thiefguard::TwoThiefGuardDriverData;
pub use crate::world::npc::area17::thiefmaster::TwoThiefMasterDriverData;
pub use crate::world::npc::area17::two_skelly::TwoSkellyDriverData;
pub use crate::world::npc::area2::moonie::MoonieDriverData;
pub use crate::world::npc::area2::superior::SuperiorDriverData;
pub use crate::world::npc::area2::vampire::VampireDriverData;
pub use crate::world::npc::area2::vampire2::Vampire2DriverData;
pub use crate::world::npc::area22::lab1_gnome::{
    apply_labgnome_create_message, LabGnomeDriverData,
};
pub use crate::world::npc::area22::lab2_herald::{
    apply_lab2_herald_create_message, Lab2HeraldDriverData,
};
pub use crate::world::npc::area22::lab2_undead::{
    apply_lab2_undead_create_message, parse_lab2_undead_driver_args, Lab2UndeadDriverData,
};
pub use crate::world::npc::area22::lab4_gnalb::{
    apply_lab4_gnalb_create_message, Lab4GnalbDriverData,
};
pub use crate::world::npc::area22::lab4_seyan::{
    apply_lab4_seyan_create_message, Lab4SeyanDriverData,
};
pub use crate::world::npc::area22::lab5_daemon::{
    apply_lab5_daemon_create_message, Lab5DaemonDriverData,
};
pub use crate::world::npc::area22::lab5_mage::{
    apply_lab5_mage_create_message, Lab5MageDriverData,
};
pub use crate::world::npc::area22::lab5_seyan::{
    apply_lab5_seyan_create_message, Lab5SeyanDriverData,
};
pub use crate::world::npc::area26::rouven::RouvenDriverData;
pub use crate::world::npc::area26::smugglecom::SmuggleComDriverData;
pub use crate::world::npc::area26::AREA26_QA;
pub use crate::world::npc::area28::aristocrat::AristocratDriverData;
pub use crate::world::npc::area28::yoatin::YoatinDriverData;
pub use crate::world::npc::area28::AREA28_QA;
pub use crate::world::npc::area29::brennethbran::BrennethBranDriverData;
pub use crate::world::npc::area29::broklin::BroklinDriverData;
pub use crate::world::npc::area29::countbran::CountBranDriverData;
pub use crate::world::npc::area29::countessabran::CountessaBranDriverData;
pub use crate::world::npc::area29::daughterbran::DaughterBranDriverData;
pub use crate::world::npc::area29::forestbran::ForestBranDriverData;
pub use crate::world::npc::area29::grinnich::GrinnichDriverData;
pub use crate::world::npc::area29::guardbran::GuardBranDriverData;
pub use crate::world::npc::area29::shanra::ShanraDriverData;
pub use crate::world::npc::area29::spiritbran::SpiritBranDriverData;
pub use crate::world::npc::area3::astro1::Astro1DriverData;
pub use crate::world::npc::area3::astro2::Astro2DriverData;
pub use crate::world::npc::area3::carlos::CarlosDriverData;
pub use crate::world::npc::area3::clara::{
    clara_dialogue_step, clara_replay_state_after_text_analysis,
    clara_state_after_swamp_monster_death, ClaraDialogueContext, ClaraDialogueOutcome,
    ClaraDriverData,
};
pub use crate::world::npc::area3::kassim::{EngraveDriverData, KassimDriverData};
pub use crate::world::npc::area3::kelly::KellyDriverData;
pub use crate::world::npc::area3::lampghost::LampghostDriverData;
pub use crate::world::npc::area3::seymour::SeymourDriverData;
pub use crate::world::npc::area3::sir_jones::SirJonesDriverData;
pub use crate::world::npc::area3::supermax::SupermaxDriverData;
pub use crate::world::npc::area3::thomas::ThomasDriverData;
pub use crate::world::npc::area30::clanclerk::{parse_clanclerk_driver_args, ClanclerkDriverData};
pub use crate::world::npc::area30::clanmaster::{
    parse_clanmaster_driver_args, ClanmasterDriverData, CLANMASTER_QA,
};
pub use crate::world::npc::area31::dwarfchief::DwarfChiefDriverData;
pub use crate::world::npc::area31::dwarfshaman::DwarfShamanDriverData;
pub use crate::world::npc::area31::dwarfsmith::DwarfSmithDriverData;
pub use crate::world::npc::area31::lostdwarf::LostDwarfDriverData;
pub use crate::world::npc::area31::AREA31_QA;
pub use crate::world::npc::area32::governor::MissionGiverDriverData;
pub use crate::world::npc::area32::military::{
    parse_military_advisor_driver_args, parse_military_master_driver_args,
    MilitaryAdvisorDriverData, MilitaryMasterDriverData, MILITARY_QA,
};
pub use crate::world::npc::area33::gorwin::GorwinDriverData;
pub use crate::world::npc::area34::teufelgambler::TeufelGambleDriverData;
pub use crate::world::npc::area34::teufelquest::TeufelQuestDriverData;
pub use crate::world::npc::area37::hunter::HunterDriverData;
pub use crate::world::npc::area37::jaz::JazDriverData;
pub use crate::world::npc::area37::nop::{parse_nop_driver_args, NopDriverData};
pub use crate::world::npc::area37::rammy::RammyDriverData;
pub use crate::world::npc::area4::tester::TesterDriverData;
pub use crate::world::npc::area8::fdemon_army::FarmyData;
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
pub use crate::world::npc::professor::{
    parse_professor_driver_args, ProfessorDriverData, PROFESSOR_QA,
};
pub use crate::world::npc::trader::{TraderDriverData, TRADER_QA};
