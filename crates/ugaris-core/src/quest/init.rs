use super::*;

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
pub(crate) const GWENDYLON_STATE_ENTRY: i32 = 0;

pub(crate) const GWENDYLON_STATE_FIRST_SKULL_DONE: i32 = 6;

pub(crate) const GWENDYLON_STATE_SECOND_SKULL_DONE: i32 = 10;

pub(crate) const GWENDYLON_STATE_THIRD_SKULL_DONE: i32 = 14;

pub(crate) const GWENDYLON_STATE_FOUL_MAGICIAN_DONE: i32 = 18;

pub(crate) const JESSICA_STATE_QUEST1_GIVE_1: i32 = 1;

/// C `JESSICA_STATE_QUEST1_DO` (`src/common/npc_states.h:90`), needed by
/// `world::jessica::process_jessica_actions` (`jessica_driver`,
/// `src/area/1/gwendylon.c:1809-2065`) but not by `questlog_init_area1`.
pub(crate) const JESSICA_STATE_QUEST1_DO: i32 = 6;

pub(crate) const JESSICA_STATE_QUEST1_FINISH: i32 = 7;

pub(crate) const JESSICA_STATE_QUEST2_GIVE_1: i32 = 8;

/// C `JESSICA_STATE_QUEST2_DO` (`src/common/npc_states.h:94`), same
/// rationale as `JESSICA_STATE_QUEST1_DO` above.
pub(crate) const JESSICA_STATE_QUEST2_DO: i32 = 10;

pub(crate) const JESSICA_STATE_QUEST2_FINISH: i32 = 11;

pub(super) const BRITHILDIE_STATE_NOMORETALES_QOPEN: i32 = 20;

pub(super) const BRITHILDIE_STATE_NOMORETALES_QDONE: i32 = 21;

pub(crate) const CAMHERMIT_STATE_QUEST1DO: i32 = 5;

pub(crate) const CAMHERMIT_STATE_QUEST2WAIT: i32 = 6;

/// C `CAMHERMIT_STATE_QUEST2_1` (`src/common/npc_states.h:17`), used by
/// `questlog_reopen_q83` (`src/system/questlog.c:586-594`) - not read by
/// `questlog_init_area1`, so it wasn't needed until the reopen dispatch.
pub(crate) const CAMHERMIT_STATE_QUEST2_1: i32 = 7;

pub(crate) const CAMHERMIT_STATE_QUEST2DO: i32 = 11;

pub(crate) const CAMHERMIT_STATE_DONE: i32 = 13;

// The remaining `CAMHERMIT_STATE_*` constants (`src/common/npc_states.h:
// 10-24`), needed by `world::camhermit::process_camhermit_actions`
// (`camhermit_driver`, `src/area/1/gwendylon.c:707-996`) but not by
// `questlog_init_area1`/`questlog_reopen`, so they weren't defined above.
pub(crate) const CAMHERMIT_STATE_ENTRY: i32 = 0;

pub(crate) const CAMHERMIT_STATE_QUEST1WAIT: i32 = 1;

pub(crate) const CAMHERMIT_STATE_QUEST1_1: i32 = 2;

pub(crate) const CAMHERMIT_STATE_QUEST1_2: i32 = 3;

pub(crate) const CAMHERMIT_STATE_QUEST1_3: i32 = 4;

pub(crate) const CAMHERMIT_STATE_QUEST2_2: i32 = 8;

pub(crate) const CAMHERMIT_STATE_QUEST2_3: i32 = 9;

pub(crate) const CAMHERMIT_STATE_QUEST2_4: i32 = 10;

pub(crate) const CAMHERMIT_STATE_QUEST2_REOPEN: i32 = 12;

pub(crate) const CAMHERMIT_STATE_QUEST2DO_WAIT: i32 = 14;

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
