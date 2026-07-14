//! Miscellaneous shared driver helpers: bank driver args, arena/clan-found
//! state constants and the gatekeeper enter-test prechecks.

use super::*;

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
/// C `#define EXP_AREA15_HARDKILL 7500` (`src/common/quest_exp.h:43`).
/// Single source of truth is `crate::quest::quest_exp::EXP_AREA15_HARDKILL`
/// (`i64`); re-exposed here as `i32` to match `ClaraDialogueOutcome::
/// military_exp`/`World::give_military_pts_from_npc`'s `exps: i32`.
pub const EXP_AREA15_HARDKILL: i32 = crate::quest::quest_exp::EXP_AREA15_HARDKILL as i32;
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
pub(super) fn gate_class_choice_is_valid(flags: CharacterFlags, class: i32) -> bool {
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
