//! Per-tick NPC and queued-event passes, one fn per legacy
//! driver/drain, grouped by area. `run_all` preserves the
//! exact pre-extraction execution order. New NPC plumbing
//! goes here, never in `main.rs`.

pub(crate) mod admin_tasks;
pub(crate) mod area1;
pub(crate) mod area11;
pub(crate) mod area12;
pub(crate) mod area13;
pub(crate) mod area16;
pub(crate) mod area17;
pub(crate) mod area19;
pub(crate) mod area2;
pub(crate) mod area20;
pub(crate) mod area22;
pub(crate) mod area23_24;
pub(crate) mod area25;
pub(crate) mod area26;
pub(crate) mod area28;
pub(crate) mod area29;
pub(crate) mod area3;
pub(crate) mod area30;
pub(crate) mod area31;
pub(crate) mod area32;
pub(crate) mod area33;
pub(crate) mod area34;
pub(crate) mod area36;
pub(crate) mod area37;
pub(crate) mod area38;
pub(crate) mod area4;
pub(crate) mod area8;
pub(crate) mod arena;
pub(crate) mod system;

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_all(
    world: &mut World,
    runtime: &mut ServerRuntime,
    zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    args: &Args,
    completed_actions: &[WorldActionCompletion],
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // Purely mechanical shorthand: `pass!(f)` expands to calling `f` with
    // the standard 17 forwarded driver-pass arguments (plus any extras),
    // exactly as the pre-split code spelled out per call. Defined inside
    // `run_all` so the identifiers resolve to the parameters above.
    macro_rules! pass {
        ($f:expr $(, $extra:expr)* $(,)?) => {
            $f(
                &mut *world,
                &mut *runtime,
                &mut *zone_loader,
                config,
                args,
                completed_actions,
                achievement_repository,
                character_repository,
                area_repository,
                clan_repository,
                clan_log_repository,
                merchant_repository,
                military_master_storage_repository,
                military_advisor_storage_repository,
                notes_repository,
                anticheat_repository,
                auction_repository,
                $($extra,)*
            )
            .await
        };
    }

    pass!(area22::pass_0);
    pass!(system::death_1);
    pass!(system::macro_track_exp_gain_2);
    pass!(system::macro_driver_3);
    pass!(area22::lostcon_driver_4);
    let merchants_before_tick = pass!(system::merchant_actions_5);
    pass!(arena::aclerk_driver_6);
    pass!(system::bank_driver_7);
    pass!(system::trader_driver_8);
    pass!(area30::clanmaster_driver_9);
    pass!(area30::clanclerk_driver_10);
    pass!(area13::dungeonmaster_11);
    pass!(area13::dungeondoor_12);
    pass!(area13::dungeonfighter_13);
    pass!(arena::clubmaster_driver_14);
    pass!(admin_tasks::lastseen_15);
    pass!(admin_tasks::acstatus_16);
    pass!(admin_tasks::querystats_17);
    pass!(admin_tasks::jail_18);
    pass!(admin_tasks::jail_19);
    pass!(admin_tasks::change_area_20);
    pass!(area13::build_remove_tile_21);
    pass!(admin_tasks::rmdeath_22);
    pass!(admin_tasks::complain_23);
    pass!(admin_tasks::god_24);
    pass!(admin_tasks::rename_25);
    pass!(admin_tasks::lockname_26);
    pass!(admin_tasks::punish_27);
    pass!(admin_tasks::unpunish_28);
    pass!(admin_tasks::exterminate_29);
    pass!(admin_tasks::look_30);
    pass!(admin_tasks::klog_31);
    pass!(admin_tasks::showvalues_32);
    pass!(admin_tasks::values_33);
    pass!(admin_tasks::allow_34);
    pass!(area32::military_master_driver_35);
    pass!(area30::military_advisor_driver_36);
    pass!(area13::tick_clan_37);
    pass!(arena::master_driver_38);
    pass!(arena::fighter_driver_39);
    pass!(arena::manager_driver_40);
    pass!(area1::camhermit_driver_41);
    pass!(area1::yoakin_driver_42);
    pass!(area1::terion_driver_43);
    pass!(area1::gwendylon_driver_44);
    pass!(area1::gwendylon_driver_45);
    pass!(area1::greeter_driver_46);
    pass!(area1::jessica_driver_47);
    pass!(area1::jiu_driver_48);
    pass!(area1::forest_ranger_driver_49);
    pass!(area1::gate_welcome_driver_50);
    pass!(area22::gate_fight_driver_51);
    pass!(system::janitor_driver_52);
    pass!(system::merchant_driver_53, &merchants_before_tick);
    pass!(system::maintenance_60s_task_54);
    pass!(system::maintenance_60s_task_55);
    pass!(system::world_56);
    pass!(area32::world_57);
    pass!(area32::world_58);
    pass!(admin_tasks::player_update_59);
    pass!(system::init_event_system_60);
    pass!(system::tick_player_61);
    pass!(area1::brithildie_driver_62);
    pass!(area1::nook_driver_63);
    pass!(area1::lydia_driver_64);
    pass!(area1::robber_driver_65);
    pass!(area1::sanoa_driver_66);
    pass!(area1::asturin_driver_67);
    pass!(area1::reskin_driver_68);
    pass!(area1::guiwynn_driver_69);
    pass!(area1::james_driver_70);
    pass!(area1::balltrap_driver_71);
    pass!(area1::logain_driver_72);
    pass!(area2::superior_driver_73);
    pass!(area2::moonie_driver_74);
    pass!(area2::vampire_driver_75);
    pass!(area2::vampire2_driver_76);
    pass!(area3::astro1_driver_77);
    pass!(area3::astro2_driver_80);
    pass!(area3::thomas_driver_78);
    pass!(area3::sir_jones_driver_79);
    pass!(area3::seymour_driver_81);
    pass!(area3::kelly_driver_82);
    pass!(area3::carlos_driver_83);
    pass!(area3::kassim_driver_84);
    pass!(area3::supermax_driver_85);
    pass!(area3::lampghost_driver_86);
    pass!(area4::tester_driver_87);
    pass!(area8::fdemon_demon_driver_88);
    pass!(area8::fdemon_boss_driver_89);
    pass!(area11::islena_driver_90);
    pass!(area11::palace_guard_driver_91);
    pass!(area12::golemkeyhold_driver_92);
    pass!(area3::clara_driver_93);
    pass!(area16::forest_imp_driver_94);
    pass!(area16::forest_william_driver_95);
    pass!(area16::forest_hermit_driver_96);
    pass!(area17::two_skelly_driver_97);
    pass!(area17::two_alchemist_driver_98);
    pass!(area17::two_sanwyn_driver_99);
    pass!(area17::two_barkeeper_driver_100);
    pass!(area17::two_guard_driver_101);
    pass!(area17::two_servant_driver_102);
    pass!(area17::two_thiefguard_driver_103);
    pass!(area17::two_thiefmaster_driver_104);
    pass!(area19::nomad_driver_105);
    pass!(area19::madhermit_driver_106);
    pass!(area20::lqnpc_driver_107);
    pass!(area22::labgnome_driver_108);
    pass!(area22::lab2herald_driver_109);
    pass!(area22::lab2deamon_driver_110);
    pass!(area22::lab3passguard_driver_111);
    pass!(area22::lab3prisoner_driver_112);
    pass!(area22::lab4seyan_driver_113);
    pass!(area22::lab4gnalb_driver_114);
    pass!(area22::lab5seyan_driver_115);
    pass!(area22::lab5daemon_driver_116);
    pass!(area22::lab5mage_driver_117);
    pass!(area23_24::strategy_boss_driver_118);
    pass!(area25::warpmaster_driver_119);
    pass!(area25::warpfighter_driver_120);
    pass!(area26::smugglecom_driver_121);
    pass!(area26::rouven_driver_130);
    pass!(area28::aristocrat_driver_131);
    pass!(area28::yoatin_driver_132);
    pass!(area29::spiritbran_driver_133);
    pass!(area29::countbran_driver_134);
    pass!(area29::countessabran_driver_135);
    pass!(area29::daughterbran_driver_136);
    pass!(area29::forestbran_driver_137);
    pass!(area29::brennethbran_driver_138);
    pass!(area29::broklin_driver_139);
    pass!(area29::guardbran_driver_140);
    pass!(area29::grinnich_driver_141);
    pass!(area29::shanra_driver_142);
    pass!(area31::dwarfchief_driver_143);
    pass!(area31::lostdwarf_driver_144);
    pass!(area31::dwarfshaman_driver_145);
    pass!(area31::dwarfsmith_driver_146);
    pass!(area32::mission_giver_driver_147);
    pass!(area33::gorwin_driver_158);
    pass!(area34::teufelquest_driver_159);
    pass!(area34::teufelgambler_driver_160);
    pass!(area36::caligar_guard_driver_161);
    pass!(area36::caligar_guard2_driver_162);
    pass!(area36::caligar_glori_driver_163);
    pass!(area36::caligar_arquin_driver_164);
    pass!(area36::caligar_smith_driver_165);
    pass!(area36::caligar_homden_driver_166);
    pass!(area37::nop_driver_167);
    pass!(area37::rammy_driver_168);
    pass!(area37::jaz_driver_169);
    pass!(area37::fiona_driver_170);
    pass!(area37::bridgeguard_driver_171);
    pass!(area37::gladiator_driver_172);
    pass!(area37::ramin_driver_173);
    pass!(area37::arkhatamonk_driver_174);
    pass!(area37::captain_driver_175);
    pass!(area37::judge_driver_176);
    pass!(area37::jada_driver_177);
    pass!(area37::potmaker_driver_178);
    pass!(area37::hunter_driver_179);
    pass!(area37::thaipan_driver_180);
    pass!(area37::trainer_driver_181);
    pass!(area37::kidnappee_driver_182);
    pass!(area37::clerk_driver_183);
    pass!(area37::krenach_driver_184);
    pass!(area38::shr_werewolf_driver_185);
    pass!(system::professor_driver_186);
}
