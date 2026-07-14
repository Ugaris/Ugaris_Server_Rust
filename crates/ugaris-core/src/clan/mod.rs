//! Clan relation state machine ported from `src/system/clan.c` in the
//! legacy C server (`clan.h` for constants/data model).
//!
//! This is built up in self-contained slices of the much larger clan
//! system (see the "Clan system" P3 entry in `PORTING_TODO.md`). What is
//! ported here: the `CS_*` relation levels (`clan.h:41-56`), the per-pair
//! `current_relation`/`want_relation`/`want_date` state (`struct
//! clan_status`, `clan.h:58-64`), clan founding's relation reset
//! (`found_clan`/`clan_standards`/`zero_relation`, `clan.c:76-95,450-492`),
//! the unilateral relation-request setter (`set_clan_relation`,
//! `clan.c:839-860`), the read-only policy queries (`may_enter_clan`,
//! `clan_can_attack_outside`, `clan_can_attack_inside`, `clan_alliance`,
//! `clan.c:881-934`), the daily escalation/de-escalation tick state
//! machine (`update_relations`, `clan.c:936-1089`), and - via
//! [`ClanRegistry`] - clan identity (name/rank names/website/message,
//! `struct clan`'s `name`/`rankname`/`website`/`message` fields,
//! `clan.h:88-101`) plus the membership wiring onto `Character.clan`/
//! `.clan_rank`/`.clan_serial` (`found_clan`, `get_char_clan`,
//! `add_member`, `remove_member`, `clan.c:242-272,460-492,1186-1221`),
//! and the treasury/bonus economy (jewels, weekly upkeep cost, debt
//! accrual/auto-pay, bankrupt-clan deletion, bonus levels, depot money,
//! dungeon training-score decay - `update_treasure`, `update_training`,
//! `add_jewel`, `swap_jewels`, `cnt_jewels`, `get_clan_bonus`,
//! `set_clan_bonus`, `get_bonus_name`, `get_clan_money`,
//! `clan_money_change`, `clan.c:494-544,1105-1182,222-244`), and the
//! merchant trade bonus (`clan_trade_bonus`, `clan.c:1545-1552` - see
//! [`crate::world::World::clan_trade_bonus`], wired into every merchant
//! price computation in `crate::world::merchant`), and the dungeon-guard
//! configuration accessors (`get_clan_dungeon_cost`/`set_clan_dungeon_use`/
//! `get_clan_dungeon`, `clan.c:617-799`) - these are pure state on `struct
//! clan_dungeon` with a real, reachable caller today (the clanclerk NPC's
//! `use` command, `area/30/clanmaster.c:854-899`, ported in
//! `crate::world::clanclerk`), even though `get_clan_dungeon`'s own only
//! C caller (`area/13/dungeon.c`'s raid-spawn setup) is not ported yet -
//! same "pure logic first, wiring later" precedent as `set_clan_raid`
//! before it - and the potion half of the dungeon-guard economy
//! (`add_alc_potion`/`add_simple_potion`, `clan.c:1457-1533`, the
//! `alc_pot`/`simple_pot` stockpile populated by finished alchemy
//! flasks/potions handed to the clan clerk NPC, `area/30/
//! clanmaster.c:763-771,1176-1189`), ported alongside the guard-count
//! accessors above it.
//!
//! NOT ported yet (left for follow-up slices): the `total`-owned half of
//! each guard-count pair, which stays permanently `0` since C's own `buy`
//! command that would set it is dead code; the raid-spawn consumer in
//! `area/13/dungeon.c` itself (the only real reader of `get_clan_dungeon`/
//! the potion stockpile), the `doraid` raid-toggle clamp inside
//! `update_relations`/`set_clan_bonus`
//! (dead in practice once a clan's first tick has run - see the comment
//! on [`ClanRelations::update`] - and meaningless without the dungeon/
//! raid system, so intentionally skipped in both places),
//! clan-log persistence (`add_clanlog`/SQL `clanlog` table - this module
//! returns event enums like [`ClanRelationEvent`]/[`ClanMoneyChange`]/
//! [`ClanTreasuryEvent`] for the runtime caller to format and log; see
//! `crates/ugaris-server/src/world_events.rs`'s `apply_clan_economy_tick`
//! for [`ClanRelations::update`]/[`ClanRegistry::update_treasure`]/
//! [`ClanRegistry::update_training`]'s live wiring), achievement awarding
//! on membership change (`ACHIEVEMENT_CLAN_MEMBER`/`ACHIEVEMENT_CLUB_
//! MEMBER`, left to the runtime caller per the pattern used elsewhere -
//! see [`ClanRegistry::add_member`]), and clan-hall transport access
//! beyond direct membership.

mod economy;
mod registry;
mod relations;

pub use economy::*;
pub use registry::*;
pub use relations::*;

#[cfg(test)]
mod tests;

use crate::entity::{Character, CharacterValue, MAX_MODIFIERS};
use serde::{Deserialize, Serialize};

/// C `#define MAXCLAN 32` (`clan.h:19`). Clan numbers are `1..MAX_CLAN`;
/// `0` means "no clan" and numbers `>= 1024` (`CLUBOFFSET`) mean a club,
/// which is a separate, not-yet-ported system that reuses the same
/// character fields (see `src/system/club.c`).
pub const MAX_CLAN: usize = 32;

/// C `#define MAXBONUS 14` (`clan.h:20`).
pub const MAX_BONUS: usize = 14;

/// C `#define CLANHALLRENT 5` (`clan.c:47`): flat weekly rent added on top
/// of bonus upkeep in `update_treasure`, expressed in whole gold (the
/// treasury itself tracks cost in thousandths, so this gets multiplied by
/// 1000 before use, matching `CLANHALLRENT * 1000` at `clan.c:1125`).
pub const CLAN_HALL_RENT: i32 = 5;

/// Bonus slot index 2 ("Merchant") is the only one with a name that
/// matters gameplay-wise today; `clan_trade_bonus` reads
/// `get_clan_bonus(cnr, 2)` (`clan.c:1545-1552`). Kept as a named constant
/// so future callers don't have to guess the magic number.
pub const CLAN_BONUS_MERCHANT: usize = 2;

/// Bonus slot index 1 ("Military Advisor", `bonus_name[1]`,
/// `clan.c:64`): the periodic per-clan military-points feed
/// `update_clan_points` (`military.c:1815-1832`) reads
/// `get_clan_bonus(cnr, 1) * 20` every 60 seconds. Named the same way as
/// [`CLAN_BONUS_MERCHANT`].
pub const CLAN_BONUS_MILITARY_ADVISOR: usize = 1;

/// C `score_to_level` (`clan.c:72-74`): converts a clan's dungeon-guard
/// training score into the training-derived bonus level shown by
/// `showclan` ("guard bonus: +%d", `clan.c:196-198`).
pub fn score_to_level(score: i32) -> i32 {
    score / 100
}

/// C `get_clan_dungeon_cost` (`clan.c:732-780`): the training-point cost
/// of setting a dungeon-guard configuration slot (`type` `1..=21`, see
/// [`ClanEconomy::dungeon_guard_use`]) to `number`. Per-tier multipliers
/// repeat identically across the warrior/mage/seyan `+0..+5` triples (`*
/// 1/2/4/8/12/16`); teleport traps/fake walls/locked doors get their own
/// flat multipliers. An unknown `type` is C server-side-log-only dead
/// code in practice (every real caller already validates `type` first)
/// and returns `0`, matching C's own fallback return after the `elog`.
pub fn get_clan_dungeon_cost(dungeon_type: i32, number: i32) -> i32 {
    match dungeon_type {
        1 | 7 | 13 => number,
        2 | 8 | 14 => number * 2,
        3 | 9 | 15 => number * 4,
        4 | 10 | 16 => number * 8,
        5 | 11 | 17 => number * 12,
        6 | 12 | 18 => number * 16,
        19 => number * 8,
        20 => number * 16,
        21 => number * 12,
        _ => 0,
    }
}

/// C `get_bonus_name` + `static char *bonus_name[MAXBONUS]`
/// (`clan.c:63-68,522-527`). Takes `i32` (not `usize`) to mirror C's
/// `nr < 0` half of the range guard directly, since callers may pass an
/// arbitrary client-supplied bonus index.
pub fn bonus_name(nr: i32) -> &'static str {
    match nr {
        0 => "Pentagram Quest",
        1 => "Military Advisor",
        2 => "Merchant",
        n if n > 2 && (n as usize) < MAX_BONUS => "unassigned",
        _ => "Unknown", // C `get_bonus_name`'s `nr < 0 || nr >= MAXBONUS` guard.
    }
}
