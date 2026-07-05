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

use serde::{Deserialize, Serialize};

use crate::entity::{Character, CharacterValue, MAX_MODIFIERS};

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

/// C relation levels (`clan.h:41-56`). Numeric order matters: the
/// escalation/de-escalation state machine (`update_relations`) moves the
/// current relation up or down by one step using plain integer
/// increment/decrement on the C `#define`s, so the derived variant order
/// below must match exactly (`None` is never assigned by game logic but is
/// C's zero-initialized/invalid value, mirrored here for completeness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ClanRelation {
    /// C: uninitialized/invalid relation value (0). Never set by any
    /// `clan.c` function on an existing clan pair; kept only so an
    /// out-of-range C byte round-trips without panicking.
    None = 0,
    /// C `CS_ALLIANCE` (1): members can never attack each other; needs 24h
    /// one-sided request to de-escalate to `PeaceTreaty`.
    Alliance = 1,
    /// C `CS_PEACETREATY` (2): members can never attack each other; needs
    /// 24h one-sided request to de-escalate to `Neutral`.
    PeaceTreaty = 2,
    /// C `CS_NEUTRAL` (3): members cannot attack each other; needs 1h
    /// one-sided request to escalate to `War` (or an immediate mutual
    /// request either way).
    Neutral = 3,
    /// C `CS_WAR` (4): members can attack each other in clan areas only;
    /// killed enemies keep their EXP but lose their items.
    War = 4,
    /// C `CS_FEUD` (5): members can attack each other everywhere; killed
    /// enemies keep both EXP and items. Needs 24h one-sided request to
    /// de-escalate to `War`.
    Feud = 5,
}

impl ClanRelation {
    /// C `rel_name[]` (`clan.c:52`).
    pub fn display_name(self) -> &'static str {
        match self {
            ClanRelation::None => "none",
            ClanRelation::Alliance => "Alliance",
            ClanRelation::PeaceTreaty => "Peace-Treaty",
            ClanRelation::Neutral => "Neutral",
            ClanRelation::War => "War",
            ClanRelation::Feud => "Feud",
        }
    }

    /// C: `cur++` - one step toward `Feud` (worse relation).
    fn increment(self) -> ClanRelation {
        use ClanRelation::*;
        match self {
            None => Alliance,
            Alliance => PeaceTreaty,
            PeaceTreaty => Neutral,
            Neutral => War,
            War | Feud => Feud,
        }
    }

    /// C: `cur--` - one step toward `Alliance` (better relation).
    fn decrement(self) -> ClanRelation {
        use ClanRelation::*;
        match self {
            None | Alliance => Alliance,
            PeaceTreaty => Alliance,
            Neutral => PeaceTreaty,
            War => Neutral,
            Feud => War,
        }
    }
}

/// One step of the relation escalation/de-escalation state machine
/// (`update_relations`, `clan.c:936-1089`) taking effect for a clan pair,
/// matching one of the seven distinct C `add_clanlog` message shapes. The
/// caller (once clan identity/log persistence lands) formats these with
/// the two clans' names, e.g. `"{relation} with {other clan name}
/// ({other clan number}) started"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanRelationChange {
    /// C: `want1 == want2` branch (`clan.c:980-985`) - both clans requested
    /// the same relation, so it took effect immediately without a delay.
    /// Message shape: `"{relation} with {other} ({nr}) started"`.
    Agreed { relation: ClanRelation },
    /// C: `case CS_ALLIANCE` decrement (`clan.c:989-1004`). Message:
    /// `"Alliance with {other} ({nr}) ended"`.
    AllianceEnded,
    /// C: `case CS_PEACETREATY` decrement (`clan.c:1005-1021`). Message:
    /// `"Peace Treaty with {other} ({nr}) ended"` (note: hardcoded message
    /// text uses a space, unlike `rel_name`'s hyphenated `"Peace-Treaty"`).
    PeaceTreatyEnded,
    /// C: `case CS_NEUTRAL` increment (`clan.c:1022-1043`). Message:
    /// `"War with {other} ({nr}) started"`.
    WarStarted,
    /// C: `case CS_NEUTRAL` decrement (`clan.c:1044-1050`). Message:
    /// `"Peace Treaty with {other} ({nr}) started"`.
    PeaceTreatyStarted,
    /// C: `case CS_WAR` decrement (`clan.c:1052-1060`). Message:
    /// `"War with {other} ({nr}) ended"`.
    WarEnded,
    /// C: `case CS_FEUD` decrement (`clan.c:1061-1083`). Message:
    /// `"Feud with {other} ({nr}) ended"`.
    FeudEnded,
}

impl ClanRelationChange {
    /// Formats one side of this change's `add_clanlog` message
    /// (`clan.c:936-1089`'s seven message shapes), given the *other*
    /// clan's name/number. The caller writes this to both sides of the
    /// pair, swapping which clan is "other" each time (`clan_a`'s log
    /// names `clan_b` as other and vice versa).
    pub fn log_message(&self, other_name: &str, other_nr: u16) -> String {
        match self {
            // C: `"%s with %s (%d) started", rel_name[want1], ...` - uses
            // the raw `rel_name[]` text (hyphenated "Peace-Treaty"),
            // unlike the hardcoded space-variant below.
            ClanRelationChange::Agreed { relation } => {
                format!(
                    "{} with {other_name} ({other_nr}) started",
                    relation.display_name()
                )
            }
            ClanRelationChange::AllianceEnded => {
                format!("Alliance with {other_name} ({other_nr}) ended")
            }
            ClanRelationChange::PeaceTreatyEnded => {
                format!("Peace Treaty with {other_name} ({other_nr}) ended")
            }
            ClanRelationChange::WarStarted => {
                format!("War with {other_name} ({other_nr}) started")
            }
            ClanRelationChange::PeaceTreatyStarted => {
                format!("Peace Treaty with {other_name} ({other_nr}) started")
            }
            ClanRelationChange::WarEnded => {
                format!("War with {other_name} ({other_nr}) ended")
            }
            ClanRelationChange::FeudEnded => {
                format!("Feud with {other_name} ({other_nr}) ended")
            }
        }
    }
}

/// A single relation-state change produced by [`ClanRelations::update`].
/// C logs this symmetrically for both clans in the pair
/// (`add_clanlog(n, ...)` and `add_clanlog(m, ...)`); this struct
/// represents one unordered pair, the caller logs it to both sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClanRelationEvent {
    pub clan_a: u16,
    pub clan_b: u16,
    pub change: ClanRelationChange,
}

/// Error returned by [`ClanRelations::set_relation`], mirroring the `-1`
/// return of C `set_clan_relation` (`clan.c:839-860`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanRelationError {
    /// C: `cnr < 1 || cnr >= MAXCLAN`.
    InvalidClan(u16),
    /// C: `rel < 1 || rel > 5`.
    InvalidRelation,
}

/// Clan relation registry: which clan numbers currently exist, and the
/// full pairwise relation state. This intentionally omits everything else
/// in C `struct clan` (name text, rank names, treasury, dungeon economy) -
/// see the module doc comment for what remains unported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClanRelations {
    /// C: `clan[n].name[0]` truthiness (a clan "exists" while it has a
    /// non-empty name). Index `0` is always `false` ("no clan").
    exists: [bool; MAX_CLAN],
    /// C: `clan[n].status.current_relation[m]`.
    current_relation: [[ClanRelation; MAX_CLAN]; MAX_CLAN],
    /// C: `clan[n].status.want_relation[m]`.
    want_relation: [[ClanRelation; MAX_CLAN]; MAX_CLAN],
    /// C: `clan[n].status.want_date[m]` (`realtime` seconds).
    want_date: [[i64; MAX_CLAN]; MAX_CLAN],
}

impl Default for ClanRelations {
    fn default() -> Self {
        ClanRelations {
            exists: [false; MAX_CLAN],
            current_relation: [[ClanRelation::None; MAX_CLAN]; MAX_CLAN],
            want_relation: [[ClanRelation::None; MAX_CLAN]; MAX_CLAN],
            want_date: [[0; MAX_CLAN]; MAX_CLAN],
        }
    }
}

impl ClanRelations {
    pub fn new() -> Self {
        Self::default()
    }

    fn valid_clan(nr: u16) -> bool {
        nr >= 1 && (nr as usize) < MAX_CLAN
    }

    pub fn exists(&self, nr: u16) -> bool {
        Self::valid_clan(nr) && self.exists[nr as usize]
    }

    /// C `found_clan` + `clan_standards` + `zero_relation`
    /// (`clan.c:76-95,450-492`): registers a new clan number and resets its
    /// relation to every other clan (in both directions) to `Neutral`.
    /// Returns `false` if `nr` is out of range (C: caller finds the first
    /// free slot itself via a linear scan, `clan.c:470-478`, so an
    /// out-of-range value here indicates a caller bug rather than a normal
    /// "clan list is full" condition).
    pub fn found_clan(&mut self, nr: u16, now: i64) -> bool {
        if !Self::valid_clan(nr) {
            return false;
        }
        let nr = nr as usize;
        self.exists[nr] = true;
        for other in 1..MAX_CLAN {
            // clan_standards(): our own relation to everyone else.
            self.current_relation[nr][other] = ClanRelation::Neutral;
            self.want_relation[nr][other] = ClanRelation::Neutral;
            self.want_date[nr][other] = now;
            // zero_relation(): everyone else's relation toward us.
            self.current_relation[other][nr] = ClanRelation::Neutral;
            self.want_relation[other][nr] = ClanRelation::Neutral;
            self.want_date[other][nr] = now;
        }
        true
    }

    /// C `kill_clan`/`update_treasure`'s broke-deletion path clearing
    /// `clan[cnr].name[0] = 0`. Only the existence flag is cleared here;
    /// relation arrays are left as-is since C never zeroes them on
    /// deletion (a re-founded clan re-runs `found_clan`, which resets them
    /// anyway).
    pub fn delete_clan(&mut self, nr: u16) {
        if Self::valid_clan(nr) {
            self.exists[nr as usize] = false;
        }
    }

    /// C `set_clan_relation` (`clan.c:839-860`): unilaterally request a new
    /// relation with another clan. Only sets the "want" side; the actual
    /// relation only changes once [`ClanRelations::update`] runs the
    /// escalation/de-escalation rules.
    pub fn set_relation(
        &mut self,
        clan: u16,
        other: u16,
        relation: ClanRelation,
        now: i64,
    ) -> Result<(), ClanRelationError> {
        if !Self::valid_clan(clan) {
            return Err(ClanRelationError::InvalidClan(clan));
        }
        if !Self::valid_clan(other) {
            return Err(ClanRelationError::InvalidClan(other));
        }
        if relation == ClanRelation::None {
            return Err(ClanRelationError::InvalidRelation);
        }
        let (clan, other) = (clan as usize, other as usize);
        if self.want_relation[clan][other] != relation {
            self.want_date[clan][other] = now;
        }
        self.want_relation[clan][other] = relation;
        Ok(())
    }

    pub fn current_relation(&self, clan: u16, other: u16) -> ClanRelation {
        if !Self::valid_clan(clan) || !Self::valid_clan(other) {
            return ClanRelation::None;
        }
        self.current_relation[clan as usize][other as usize]
    }

    /// C: `clan[clan].status.want_relation[other]`, read by
    /// `show_clan_relation` (`clan.c:339-342`).
    pub fn want_relation(&self, clan: u16, other: u16) -> ClanRelation {
        if !Self::valid_clan(clan) || !Self::valid_clan(other) {
            return ClanRelation::None;
        }
        self.want_relation[clan as usize][other as usize]
    }

    /// C: `clan[clan].status.want_date[other]` (`realtime` seconds), read
    /// by `show_clan_relation` (`clan.c:341-342`).
    pub fn want_date(&self, clan: u16, other: u16) -> i64 {
        if !Self::valid_clan(clan) || !Self::valid_clan(other) {
            return 0;
        }
        self.want_date[clan as usize][other as usize]
    }

    /// C `may_enter_clan` (`clan.c:881-905`): `own_clan` is the entering
    /// character's own (already-validated) clan number, `target_clan` is
    /// the clan-hall being entered.
    pub fn may_enter(&self, own_clan: u16, target_clan: u16) -> bool {
        if own_clan == target_clan {
            return true; // C: "everybody may enter his own clan"
        }
        if own_clan == 0 {
            return false; // C: "non clan members may never enter"
        }
        if !self.exists(target_clan) {
            return false; // C: "you may not enter deleted clans"
        }
        self.current_relation(target_clan, own_clan) == ClanRelation::Alliance
    }

    /// C `clan_can_attack_outside` (`clan.c:907-914`): outside clan areas,
    /// only a feud allows attacking.
    pub fn can_attack_outside(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.current_relation(attacker_clan, defender_clan) == ClanRelation::Feud
    }

    /// C `clan_can_attack_inside` (`clan.c:916-925`): inside clan areas,
    /// war or feud allows attacking.
    pub fn can_attack_inside(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        matches!(
            self.current_relation(attacker_clan, defender_clan),
            ClanRelation::War | ClanRelation::Feud
        )
    }

    /// C `clan_alliance` (`clan.c:927-934`).
    pub fn alliance(&self, clan_a: u16, clan_b: u16) -> bool {
        self.current_relation(clan_a, clan_b) == ClanRelation::Alliance
    }

    /// C `update_relations` (`clan.c:936-1089`), the daily tick that moves
    /// `current_relation` one step toward `want_relation` per clan pair,
    /// subject to the agreement/delay rules described on each
    /// [`ClanRelationChange`] variant.
    ///
    /// C additionally clamps relations down to `Neutral` whenever either
    /// clan's `dungeon.doraid` flag is off (`clan.c:945-968`), but the same
    /// loop unconditionally forces every existing clan's `doraid` to `1`
    /// the moment it's visited (`clan.c:945-950`), so after a clan's first
    /// tick that clamp can never trigger again in practice; since the
    /// dungeon/raid system is not ported at all yet, it is intentionally
    /// skipped here rather than ported as permanently-dead code.
    pub fn update(&mut self, now: i64) -> Vec<ClanRelationEvent> {
        let mut events = Vec::new();
        for n in 1..MAX_CLAN {
            if !self.exists[n] {
                continue;
            }
            for m in 1..MAX_CLAN {
                if n == m || !self.exists[m] {
                    continue;
                }

                let want1 = self.want_relation[n][m];
                let diff1 = now - self.want_date[n][m];
                let want2 = self.want_relation[m][n];
                let diff2 = now - self.want_date[m][n];
                let cur = self.current_relation[n][m];

                if want1 == want2 && cur == want1 {
                    continue; // both want what they have, no work
                }

                const DAY: i64 = 60 * 60 * 24;
                const HOUR: i64 = 60 * 60;

                let mut next = cur;
                let mut change = None;

                if want1 == want2 {
                    next = want1;
                    change = Some(ClanRelationChange::Agreed { relation: want1 });
                } else {
                    match cur {
                        ClanRelation::Alliance => {
                            if (want1 > ClanRelation::Alliance && diff1 > DAY)
                                || (want2 > ClanRelation::Alliance && diff2 > DAY)
                            {
                                next = cur.increment();
                                change = Some(ClanRelationChange::AllianceEnded);
                            }
                        }
                        ClanRelation::PeaceTreaty => {
                            if (want1 > ClanRelation::PeaceTreaty && diff1 > DAY)
                                || (want2 > ClanRelation::PeaceTreaty && diff2 > DAY)
                            {
                                next = cur.increment();
                                change = Some(ClanRelationChange::PeaceTreatyEnded);
                            }
                        }
                        ClanRelation::Neutral => {
                            if want1 > ClanRelation::Neutral && want2 > ClanRelation::Neutral {
                                next = cur.increment();
                                change = Some(ClanRelationChange::WarStarted);
                            } else if want1 > ClanRelation::Neutral && diff1 > HOUR {
                                next = cur.increment();
                                change = Some(ClanRelationChange::WarStarted);
                            } else if want2 > ClanRelation::Neutral && diff2 > HOUR {
                                next = cur.increment();
                                change = Some(ClanRelationChange::WarStarted);
                            } else if want1 < ClanRelation::Neutral && want2 < ClanRelation::Neutral
                            {
                                next = cur.decrement();
                                change = Some(ClanRelationChange::PeaceTreatyStarted);
                            }
                        }
                        ClanRelation::War => {
                            if want1 < ClanRelation::War && want2 < ClanRelation::War {
                                next = cur.decrement();
                                change = Some(ClanRelationChange::WarEnded);
                            }
                        }
                        ClanRelation::Feud => {
                            if want1 < ClanRelation::Feud && want2 < ClanRelation::Feud {
                                next = cur.decrement();
                                change = Some(ClanRelationChange::FeudEnded);
                            } else if want1 < ClanRelation::Feud && diff1 > DAY {
                                next = cur.decrement();
                                change = Some(ClanRelationChange::FeudEnded);
                            } else if want2 < ClanRelation::Feud && diff2 > DAY {
                                next = cur.decrement();
                                change = Some(ClanRelationChange::FeudEnded);
                            }
                        }
                        ClanRelation::None => {}
                    }
                }

                if let Some(change) = change {
                    events.push(ClanRelationEvent {
                        clan_a: n as u16,
                        clan_b: m as u16,
                        change,
                    });
                }
                self.current_relation[n][m] = next;
                self.current_relation[m][n] = next;
            }
        }
        events
    }
}

impl crate::do_action::ClanAttackPolicy for ClanRelations {
    /// C `clan_alliance` (`clan.c:927-934`), used by `can_attack`'s arena
    /// same-clan-or-allied suppression check.
    fn are_allied(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.alliance(attacker_clan, defender_clan)
    }

    /// C `clan_can_attack_inside` (`clan.c:916-925`).
    fn can_attack_inside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.can_attack_inside(attacker_clan, defender_clan)
    }

    /// C `clan_can_attack_outside` (`clan.c:907-914`).
    fn can_attack_outside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.can_attack_outside(attacker_clan, defender_clan)
    }

    // `has_pk_hate` is intentionally left at the trait's default (`false`):
    // the PK-hate-list system (`DRD_PK_PPD`) is unrelated clan data, backed
    // separately by `RuntimePlayerAttackPolicy` in `world/combat.rs`.
}

/// C `#define CLUBOFFSET 1024` (`club.h:5`). Clan numbers `>= CLUB_OFFSET`
/// stored in `Character.clan` mean "club #`n - CLUB_OFFSET`", a separate
/// not-yet-ported system (`src/system/club.c`) that reuses the same
/// character fields. [`ClanRegistry`] treats such values as out of its
/// jurisdiction rather than as invalid.
pub const CLUB_OFFSET: u16 = 1024;

/// C `struct clan_treasure` (`clan.h:34-39`). All money-like fields are
/// tracked in thousandths of a gold piece ("jewel-thousandths"), matching
/// C's comment on each field - only [`ClanEconomy::depot_money`] is whole
/// gold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClanTreasure {
    /// C `int jewels`.
    pub jewels: i32,
    /// C `int cost_per_week` - in 1/1000 gold.
    pub cost_per_week: i32,
    /// C `int debt` - in 1/1000 gold.
    pub debt: i32,
    /// C `int payed_till` - `realtime` seconds.
    pub payed_till: i64,
}

/// C `struct clan`'s bonus/depot/treasure/training fields (`clan.h:93-97`
/// plus the `training_score`/`last_training_update` pair from `struct
/// clan_dungeon`, `clan.h:79-80`). The rest of `struct clan_dungeon`
/// (guard counts, potions, raid flags) is out of scope for this slice -
/// see the module doc comment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClanEconomy {
    /// C `struct clan_bonus bonus[MAXBONUS]` (`clan.h:93`), just the
    /// `level` field of each slot.
    pub bonus_level: [i32; MAX_BONUS],
    /// C `struct clan_depot depot` (`clan.h:95`), just `money`.
    pub depot_money: i32,
    pub treasure: ClanTreasure,
    /// C `struct clan_dungeon`'s `training_score` (`clan.h:78`).
    pub training_score: i32,
    /// C `struct clan_dungeon`'s `last_training_update` (`clan.h:79`).
    pub last_training_update: i64,
    /// C `struct clan_dungeon`'s `doraid` (`clan.h:79`, `unsigned int`
    /// used as a bool): whether the clan can currently be attacked/raided
    /// (`get_clan_raid`). Pulled out on its own, same precedent as
    /// `training_score`/`last_training_update` - the rest of
    /// `struct clan_dungeon` (guard counts/potions) stays out of scope
    /// (see the module doc comment).
    pub raid: bool,
    /// C `struct clan_dungeon`'s `raidonstart` (`clan.h:80`, `realtime`
    /// seconds): a pending "raiding on" request's start timestamp, set by
    /// [`ClanRegistry::set_clan_raid`] and read only for the (currently
    /// unwired) 48-hour "PENDING" countdown display in C's `showclan`
    /// (`clan.c:232-234`) - not surfaced here since no caller shows clan
    /// info yet.
    pub raid_on_start: i64,
    /// C `struct clan_dungeon`'s `alc_pot[2][6]` (`clan.h:74`): the
    /// clan's alchemy-potion dungeon-guard stockpile, `[0]` = "Attack,
    /// Parry, Immunity+N" potions, `[1]` = "Flash, Magic Shield,
    /// Immunity+N" potions, indexed `0..6` for the `+4..=+24` tiers.
    /// Nothing feeds this yet (`add_alc_potion`'s `NT_GIVE` `IDR_FLASK`
    /// call site is part of the still-unported alchemy-potion economy -
    /// see the module doc comment), so every clan reads all zero here,
    /// same as a freshly-founded C clan. `#[serde(default)]` keeps this
    /// backward compatible with any snapshot saved before this field
    /// existed.
    #[serde(default)]
    pub alc_pot: [[u16; 6]; 2],
    /// C `struct clan_dungeon`'s `simple_pot[3][3]` (`clan.h:75`): the
    /// clan's simple-potion stockpile, `[0]` = healing, `[1]` = mana,
    /// `[2]` = combo, indexed `0..3` for Small/Medium/Big. Nothing feeds
    /// this yet either (`add_simple_potion` is the same unported
    /// call site) - see [`ClanEconomy::alc_pot`].
    #[serde(default)]
    pub simple_pot: [[u16; 3]; 3],
    /// C `struct clan_dungeon`'s `warrior[1]`/`mage[1]`/`seyan[1]`/
    /// `teleport[1]`/`fake[1]`/`key[1]` (`clan.h:67-71`), the "use per
    /// dungeon" configured guard/trap/wall/key counts, flattened into one
    /// array indexed `type - 1` for `type` `1..=21` (warrior `+0..+5` =
    /// `1..=6`, mage `+0..+5` = `7..=12`, seyan `+0..+5` = `13..=18`,
    /// teleport = `19`, fake wall = `20`, locked door key = `21`) - see
    /// [`get_clan_dungeon_cost`]/[`ClanRegistry::set_clan_dungeon_use`]/
    /// [`ClanRegistry::get_clan_dungeon`]. The mirrored `[0]` "total
    /// owned" half of each C pair is not modeled: it is only ever written
    /// by C's own `buy` command, which is unconditionally dead code (see
    /// `crate::world::clanclerk`'s module doc comment), so it stays
    /// permanently `0` in every real C server too. `#[serde(default)]`
    /// keeps this backward compatible with any snapshot saved before this
    /// field existed.
    #[serde(default)]
    pub dungeon_guard_use: [i32; 21],
}

impl ClanEconomy {
    /// C `clan_standards`' treasury portion (`clan.c:92-93`):
    /// `c->treasure.payed_till = realtime; c->treasure.debt = 0;`. Every
    /// other field (bonus levels, depot money, jewels, training score) is
    /// left at C's implicit zero-initialized default, matching the
    /// static `struct clan clan[MAXCLAN]` array.
    fn standard(now: i64) -> Self {
        ClanEconomy {
            bonus_level: [0; MAX_BONUS],
            depot_money: 0,
            treasure: ClanTreasure {
                jewels: 0,
                cost_per_week: 0,
                debt: 0,
                payed_till: now,
            },
            training_score: 0,
            last_training_update: now,
            raid: false,
            raid_on_start: 0,
            alc_pot: [[0; 6]; 2],
            simple_pot: [[0; 3]; 3],
            dungeon_guard_use: [0; 21],
        }
    }

    /// C `reduce_clan_bonus` (`clan.c:1091-1099`): finds the
    /// highest-leveled bonus and reduces it by one. A no-op when every
    /// bonus is already at level 0.
    fn reduce_highest_bonus(&mut self) {
        let mut best_n = 0;
        let mut best_level = 0;
        for (n, level) in self.bonus_level.iter().enumerate() {
            if *level > best_level {
                best_level = *level;
                best_n = n;
            }
        }
        if best_level > 0 {
            self.bonus_level[best_n] -= 1;
        }
    }

    /// C: the bonus-upkeep sum inside `update_treasure`'s `do`/`while`
    /// loop (`clan.c:1112-1116`) - total weekly cost of every bonus level,
    /// in 1/1000 gold, before adding the flat clan-hall rent.
    fn bonus_upkeep_cost(&self) -> i32 {
        self.bonus_level.iter().map(|level| level * 1000).sum()
    }
}

/// C `struct clan`'s identity fields (`clan.h:88-101`). Owned by
/// [`ClanRegistry`], one per existing clan number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClanIdentity {
    /// C `char name[80]` (`clan.h:89`). Presence of a `ClanIdentity` at
    /// all *is* the Rust equivalent of C's `clan[n].name[0]` "clan exists"
    /// check, so this is never empty while the identity exists.
    pub name: String,
    /// C `char rankname[5][40]` (`clan.h:91`), indices `0..=4` matching
    /// `Character.clan_rank`. Defaults set by `clan_standards`
    /// (`clan.c:79-83`): `["Member", "Member", "Recruiter", "Treasurer",
    /// "Leader"]`.
    pub rank_names: [String; 5],
    /// C `char website[80]` (`clan.h:98`).
    pub website: String,
    /// C `char message[80]` (`clan.h:99`).
    pub message: String,
    /// C `struct clan_bonus bonus[MAXBONUS]` / `struct clan_depot depot` /
    /// `struct clan_treasure treasure` / part of `struct clan_dungeon`
    /// (`clan.h:93-97`, `clan.h:78-79`).
    pub economy: ClanEconomy,
}

impl ClanIdentity {
    /// C `clan_standards` (`clan.c:76-95`), identity portion only (the
    /// relation-reset portion is [`ClanRelations::found_clan`]).
    fn standard(name: String, now: i64) -> Self {
        ClanIdentity {
            name,
            rank_names: [
                "Member".to_string(),
                "Member".to_string(),
                "Recruiter".to_string(),
                "Treasurer".to_string(),
                "Leader".to_string(),
            ],
            website: String::new(),
            message: String::new(),
            economy: ClanEconomy::standard(now),
        }
    }
}

/// C `found_clan`'s `strlen(name) > 78` guard (`clan.c:463-465`) and the
/// "no free slot" case (linear scan hits `MAXCLAN`, `clan.c:476-478`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanFoundError {
    /// C: `strlen(name) > 78`.
    NameTooLong,
    /// C: every clan slot `1..MAXCLAN` already has a name.
    ClanListFull,
}

/// Errors for [`ClanRegistry`] identity mutators (`set_clan_rankname`,
/// `set_clan_website`, `set_clan_message`, `clan.c:584-604,862-879`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanIdentityError {
    /// C: `cnr` names a clan slot with no identity (`clan[cnr].name[0] ==
    /// 0` or out of range).
    NotFound,
    /// C `set_clan_rankname`'s `rank < 0 || rank >= 5` guard
    /// (`clan.c:866-868`).
    InvalidRank,
    /// C `set_clan_rankname`'s `strlen(name) > 37` guard (`clan.c:869-871`).
    NameTooLong,
}

/// Errors for [`ClanRegistry::set_clan_raid`]/[`ClanRegistry::set_clan_raid_god`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanRaidError {
    /// `cnr` is out of range or names a slot with no identity.
    NotFound,
    /// C's `return 1` case: the requested on/off state was already the
    /// current (or already-pending) state - a no-op the caller reports
    /// as a failure message, not silently ignores.
    NoOp,
}

/// Errors for [`ClanRegistry::set_clan_dungeon_use`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanDungeonUseError {
    /// C's `return -1` case: an invalid `cnr`, a `type` outside `1..=21`,
    /// or a `number` outside that type's own guard/trap/wall/key cap
    /// (`clan.c:619-664`).
    InvalidRequest,
    /// C's `return cost` case (`clan.c:679-681`): the resulting total
    /// training-point spend across all 21 slots would exceed the
    /// 400-point budget. Only checked when raising a value (`number >
    /// 0`) - lowering a slot is always allowed even if the clan's
    /// existing configuration is (impossibly, in practice) already over
    /// budget.
    OverBudget(i32),
}

/// Errors for [`ClanRegistry::add_member`], mirroring the boundaries C
/// `add_member`'s caller is expected to have already checked (C itself
/// does not validate `cnr`; this is a stricter Rust seam).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanMembershipError {
    /// `cnr` is out of range or names a slot with no identity.
    NotFound,
    /// `cnr >= CLUB_OFFSET` - use the (not yet ported) club system instead.
    IsClub,
}

/// Clan identity + membership registry: which clan numbers exist, their
/// name/rank-names/website/message, and a per-slot serial used to
/// invalidate stale `Character.clan_serial` references (e.g. after a clan
/// is deleted and a different clan later re-founded in the same slot).
/// Wraps [`ClanRelations`] for the relation state machine, which is
/// unaffected by this struct beyond `found_clan`/`delete_clan` keeping
/// both in sync.
///
/// Not yet persisted: no `crates/ugaris-db` repository/migration exists,
/// so this registry currently only lives in server memory (see the
/// module doc comment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClanRegistry {
    relations: ClanRelations,
    serials: [u32; MAX_CLAN],
    identities: [Option<ClanIdentity>; MAX_CLAN],
    /// C `static int clan_changed` (`clan.c:61`): set on every mutation,
    /// cleared once the registry has been flushed to persistent storage
    /// (`update_state == 6/7`, `clan.c:415-430`). Lets the periodic save
    /// task skip rewriting an unchanged registry. Deliberately not
    /// serialized: a registry freshly loaded from the database is, by
    /// definition, already in sync with what's stored there, so it starts
    /// clean (`false`, matching `Deserialize`'s use of `Default` for
    /// skipped fields) rather than forcing an immediate redundant save.
    #[serde(skip)]
    dirty: bool,
}

impl Default for ClanRegistry {
    fn default() -> Self {
        ClanRegistry {
            relations: ClanRelations::default(),
            serials: [0; MAX_CLAN],
            identities: std::array::from_fn(|_| None),
            dirty: false,
        }
    }
}

impl ClanRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn valid_clan(nr: u16) -> bool {
        nr >= 1 && (nr as usize) < MAX_CLAN
    }

    /// C: `clan[nr].name[0]` truthiness.
    pub fn exists(&self, nr: u16) -> bool {
        Self::valid_clan(nr) && self.identities[nr as usize].is_some()
    }

    /// C `clan[nr].status.serial`.
    pub fn serial(&self, nr: u16) -> u32 {
        if Self::valid_clan(nr) {
            self.serials[nr as usize]
        } else {
            0
        }
    }

    pub fn identity(&self, nr: u16) -> Option<&ClanIdentity> {
        if Self::valid_clan(nr) {
            self.identities[nr as usize].as_ref()
        } else {
            None
        }
    }

    fn identity_mut(&mut self, nr: u16) -> Option<&mut ClanIdentity> {
        if Self::valid_clan(nr) {
            self.identities[nr as usize].as_mut()
        } else {
            None
        }
    }

    /// C `get_clan_name` (`clan.c:286-292`).
    pub fn name(&self, nr: u16) -> Option<&str> {
        self.identity(nr).map(|id| id.name.as_str())
    }

    pub fn relations(&self) -> &ClanRelations {
        &self.relations
    }

    /// Returns a mutable handle to the relation state machine. Since
    /// callers can mutate `ClanRelations` freely through the returned
    /// reference (e.g. the daily `update` tick), this conservatively
    /// marks the registry dirty on every call rather than trying to
    /// detect whether the eventual mutation was a no-op - matching how
    /// C's own `clan_changed = 1` is set unconditionally at every one of
    /// `update_relations`'s many relation-transition sites
    /// (`clan.c:936-1089`).
    pub fn relations_mut(&mut self) -> &mut ClanRelations {
        self.dirty = true;
        &mut self.relations
    }

    /// C `static int clan_changed` read (`clan.c:416`): whether the
    /// registry has unsaved mutations since the last [`ClanRegistry::
    /// clear_dirty`] call.
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// C: `clan_changed = 0` after a successful `update_storage` write
    /// (`clan.c:430`). Callers should invoke this after persisting the
    /// registry.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// C `found_clan` (`clan.c:460-492`) + `clan_standards`
    /// (`clan.c:76-95`) + `zero_relation` (`clan.c:450-458`, folded into
    /// [`ClanRelations::found_clan`]): allocates the first free clan slot
    /// (a slot with no identity - either never used or previously
    /// deleted), sets its name and standard rank names, and resets its
    /// relation to every other clan (in both directions) to neutral.
    ///
    /// Unlike C (which takes the founding character and calls
    /// `add_clanlog` itself, `clan.c:489`), clan-log persistence and the
    /// `ACHIEVEMENT_CLAN_MASTER` award are left to the caller - this
    /// function only returns the new clan number.
    pub fn found_clan(&mut self, name: &str, now: i64) -> Result<u16, ClanFoundError> {
        if name.len() > 78 {
            return Err(ClanFoundError::NameTooLong);
        }
        let slot = (1..MAX_CLAN).find(|&n| self.identities[n].is_none());
        let Some(slot) = slot else {
            return Err(ClanFoundError::ClanListFull);
        };
        self.identities[slot] = Some(ClanIdentity::standard(name.to_string(), now));
        self.relations.found_clan(slot as u16, now);
        self.dirty = true;
        Ok(slot as u16)
    }

    /// C: the bankrupt-clan deletion path (`update_treasure`,
    /// `clan.c:1154-1160`) generalized to any clan deletion - clears the
    /// clan's identity and bumps its serial so stale
    /// `Character.clan_serial` references from former members become
    /// invalid the next time [`ClanRegistry::get_char_clan`] runs (C:
    /// `clan[cnr].name[0] = 0; clan[cnr].status.serial++;`).
    pub fn delete_clan(&mut self, nr: u16) {
        if Self::valid_clan(nr) {
            self.identities[nr as usize] = None;
            self.serials[nr as usize] = self.serials[nr as usize].wrapping_add(1);
            self.relations.delete_clan(nr);
            self.dirty = true;
        }
    }

    /// C `get_char_clan` (`clan.c:242-272`): validates a character's clan
    /// membership fields against the live registry, clearing them (and
    /// returning `None`) on any mismatch, exactly like C's
    /// `ch[cn].clan = ch[cn].clan_rank = ch[cn].clan_serial = 0`.
    ///
    /// Clubs (`clan >= CLUB_OFFSET`) are out of scope for this registry
    /// (`club.c` is a separate, not-yet-ported system reusing the same
    /// fields, mirroring C's own `cnr >= CLUBOFFSET` early return,
    /// `clan.c:249-251`): a club reference is left untouched but never
    /// confirmed by this function.
    ///
    /// Unlike C, this always fully validates: C skips validation while
    /// `!update_done` (storage still loading at server boot, `clan.c:259-
    /// 261`), which has no equivalent in this always-ready in-memory
    /// registry.
    pub fn get_char_clan(&self, character: &mut Character) -> Option<u16> {
        let cnr = character.clan;
        if cnr == 0 {
            return None;
        }
        if cnr >= CLUB_OFFSET {
            return None;
        }
        if !Self::valid_clan(cnr)
            || character.clan_serial != self.serials[cnr as usize]
            || !self.exists(cnr)
        {
            character.clan = 0;
            character.clan_rank = 0;
            character.clan_serial = 0;
            return None;
        }
        Some(cnr)
    }

    /// C `get_char_clan_name` (`clan.c:274-284`).
    pub fn char_clan_name(&self, character: &mut Character) -> Option<&str> {
        let cnr = self.get_char_clan(character)?;
        self.name(cnr)
    }

    /// C `add_member` (`clan.c:1186-1206`): assigns clan membership
    /// fields. Notably does *not* set `clan_rank` (a new member always
    /// keeps whatever rank they already had, normally `0`/Member).
    ///
    /// Runtime-wide side effects the caller is expected to trigger
    /// separately (out of scope for this pure registry): the
    /// `ACHIEVEMENT_CLAN_MEMBER`/`ACHIEVEMENT_CLUB_MEMBER` award, the
    /// clan-log entry, and resetting every other player's "knows this
    /// character's name" flag (`set_player_knows_name`, `clan.c:1194-
    /// 1196`).
    pub fn add_member(
        &self,
        character: &mut Character,
        cnr: u16,
    ) -> Result<(), ClanMembershipError> {
        if cnr >= CLUB_OFFSET {
            return Err(ClanMembershipError::IsClub);
        }
        if !self.exists(cnr) {
            return Err(ClanMembershipError::NotFound);
        }
        character.clan = cnr;
        character.clan_serial = self.serials[cnr as usize];
        Ok(())
    }

    /// C `remove_member` (`clan.c:1208-1221`): clears clan membership
    /// fields unconditionally (C performs no validation and always
    /// succeeds). Clan-log entry and name-recognition reset are left to
    /// the caller, as in [`ClanRegistry::add_member`].
    pub fn remove_member(&self, character: &mut Character) {
        character.clan = 0;
        character.clan_rank = 0;
        character.clan_serial = 0;
    }

    /// C `set_clan_rankname` (`clan.c:862-879`).
    pub fn set_rankname(
        &mut self,
        cnr: u16,
        rank: usize,
        name: &str,
    ) -> Result<(), ClanIdentityError> {
        if rank >= 5 {
            return Err(ClanIdentityError::InvalidRank);
        }
        if name.len() > 37 {
            return Err(ClanIdentityError::NameTooLong);
        }
        let identity = self.identity_mut(cnr).ok_or(ClanIdentityError::NotFound)?;
        identity.rank_names[rank] = name.to_string();
        self.dirty = true;
        Ok(())
    }

    /// C `set_clan_website` (`clan.c:584-593`). C additionally strips the
    /// string's final character after truncating to 79 bytes
    /// (`website[strlen(website)-1] = 0`), which depends on the raw
    /// command-line parser appending a trailing delimiter before calling
    /// this function - a calling convention that doesn't exist yet in the
    /// (not-yet-wired) Rust command layer, so that quirk is deferred to
    /// whichever future task wires a `/clan` command parser rather than
    /// guessed here. This function only enforces the 79-character length
    /// cap.
    pub fn set_website(&mut self, cnr: u16, site: &str) -> Result<(), ClanIdentityError> {
        let identity = self.identity_mut(cnr).ok_or(ClanIdentityError::NotFound)?;
        identity.website = site.chars().take(79).collect();
        self.dirty = true;
        Ok(())
    }

    /// C `set_clan_message` (`clan.c:595-604`). See
    /// [`ClanRegistry::set_website`] for why the trailing-character-strip
    /// quirk is deferred rather than ported here.
    pub fn set_message(&mut self, cnr: u16, message: &str) -> Result<(), ClanIdentityError> {
        let identity = self.identity_mut(cnr).ok_or(ClanIdentityError::NotFound)?;
        identity.message = message.chars().take(79).collect();
        self.dirty = true;
        Ok(())
    }

    /// C `clan_setname` (`clan.c:1419-1423`), the `/renclan` admin rename
    /// tool: overwrites an existing clan's display name, truncated to 78
    /// bytes (`strncpy(clan[cnr].name, name, 78); name[78] = 0;` - C never
    /// rejects an over-long name, it just silently truncates, unlike
    /// [`ClanRegistry::found_clan`]'s `NameTooLong` rejection).
    ///
    /// Deviation from C: C only range-checks `cnr` (`cnr > 0 && cnr <
    /// MAXCLAN`) and writes directly into the always-allocated static
    /// `clan[]` array, so it can technically "rename" a slot with no
    /// identity yet (leaving every other identity/relation field at its
    /// C zero-value default - an admin-tool quirk, not a normal code
    /// path). This registry has no such always-allocated slot to write
    /// into, so - consistently with every other identity mutator
    /// ([`ClanRegistry::set_rankname`]/[`ClanRegistry::set_website`]/
    /// [`ClanRegistry::set_message`]) - renaming requires the clan to
    /// already exist ([`ClanRegistry::found_clan`] first).
    pub fn set_name(&mut self, cnr: u16, name: &str) -> Result<(), ClanIdentityError> {
        let identity = self.identity_mut(cnr).ok_or(ClanIdentityError::NotFound)?;
        identity.name = name.chars().take(78).collect();
        self.dirty = true;
        Ok(())
    }

    /// C `get_clan_money` (`clan.c:222-228`). Out-of-range/nonexistent
    /// clans read as `0`, matching C's zero-initialized `clan[]` slots.
    pub fn clan_money(&self, cnr: u16) -> i32 {
        self.identity(cnr)
            .map(|id| id.economy.depot_money)
            .unwrap_or(0)
    }

    /// C `clan_money_change` (`clan.c:230-244`): applies `diff` to the
    /// clan depot and reports what to clan-log, if anything. C's `cn`
    /// parameter serves two purposes - "is this attributable to a real
    /// character" (truthy) and, via `ch[cn].name`, the log message's
    /// actor name; since this pure registry has no character table, the
    /// caller passes `log` for the first purpose and formats the message
    /// itself (with the acting character's name) from the returned
    /// [`ClanMoneyChange`] using [`ClanMoneyChange::log_message`].
    /// Nonexistent/out-of-range clans are a silent no-op, matching C's
    /// `cnr < 1 || cnr >= MAXCLAN` guard (C additionally *would* apply the
    /// diff to an in-range-but-nameless slot; this registry has no such
    /// slot to write into, so that quirk cannot occur here).
    pub fn clan_money_change(&mut self, cnr: u16, diff: i32, log: bool) -> Option<ClanMoneyChange> {
        let identity = self.identity_mut(cnr)?;
        identity.economy.depot_money += diff;
        self.dirty = true;
        // C: `if (cn && (diff >= 100 || diff < 0))`.
        if log && (diff >= 100 || diff < 0) {
            Some(if diff > 0 {
                ClanMoneyChange::Deposited(diff)
            } else {
                ClanMoneyChange::Withdrew(-diff)
            })
        } else {
            None
        }
    }

    /// C `cnt_jewels` (`clan.c:514-516`). Out-of-range/nonexistent clans
    /// read as `0`.
    pub fn jewel_count(&self, cnr: u16) -> i32 {
        self.identity(cnr)
            .map(|id| id.economy.treasure.jewels)
            .unwrap_or(0)
    }

    /// C `add_jewel` (`clan.c:494-499`): increments the clan's jewel
    /// count. C also unconditionally logs `"%s added a jewel"` with the
    /// acting character's name - left to the caller (no character table
    /// here), same pattern as [`ClanRegistry::clan_money_change`].
    pub fn add_jewel(&mut self, cnr: u16) -> Result<(), ClanIdentityError> {
        let identity = self.identity_mut(cnr).ok_or(ClanIdentityError::NotFound)?;
        identity.economy.treasure.jewels += 1;
        self.dirty = true;
        Ok(())
    }

    /// C `swap_jewels` (`clan.c:501-513`): moves up to `cnt` jewels from
    /// `from` to `to`, charging `from`'s treasury a matching debt (this is
    /// the dungeon-raid jewel-theft primitive - C never removes jewels
    /// from the loser directly, it adds debt that `update_treasure` later
    /// converts to a jewel loss). A no-op if `from` has no jewels at all;
    /// silently clamps `cnt` down to `from`'s jewel count otherwise.
    /// Nonexistent/out-of-range clan numbers are a no-op (mirrors
    /// [`ClanRegistry::clan_money_change`]'s reasoning for why C's
    /// "writes to a nameless in-range slot" quirk cannot occur here).
    pub fn swap_jewels(&mut self, from: u16, to: u16, cnt: i32) {
        let available = self.jewel_count(from);
        if available < 1 {
            return;
        }
        let cnt = cnt.min(available);
        if let Some(identity) = self.identity_mut(from) {
            identity.economy.treasure.debt += cnt * 1000;
        }
        if let Some(identity) = self.identity_mut(to) {
            identity.economy.treasure.jewels += cnt;
        }
        self.dirty = true;
    }

    /// C `get_clan_bonus` (`clan.c:518-520`). Out-of-range clan numbers or
    /// bonus slots read as `0`.
    pub fn bonus_level(&self, cnr: u16, nr: usize) -> i32 {
        if nr >= MAX_BONUS {
            return 0;
        }
        self.identity(cnr)
            .map(|id| id.economy.bonus_level[nr])
            .unwrap_or(0)
    }

    /// C `set_clan_bonus` (`clan.c:536-544`). C also rejects slot `3`
    /// while `!clan[cnr].dungeon.doraid`; since the dungeon/raid system is
    /// not ported (see the module doc comment's note on why `doraid` is
    /// skipped in [`ClanRelations::update`]), that clamp is intentionally
    /// not ported here either - it is dead in practice once a clan's
    /// first relation tick has run, and slot 3 has no assigned name or
    /// function anyway (`bonus_name(3) == "unassigned"`).
    pub fn set_bonus_level(
        &mut self,
        cnr: u16,
        nr: usize,
        level: i32,
    ) -> Result<(), ClanIdentityError> {
        if nr >= MAX_BONUS {
            return Err(ClanIdentityError::NotFound);
        }
        let identity = self.identity_mut(cnr).ok_or(ClanIdentityError::NotFound)?;
        identity.economy.bonus_level[nr] = level;
        self.dirty = true;
        Ok(())
    }

    /// C `get_clan_dungeon` (`clan.c:783-799`). Out-of-range/nonexistent
    /// clans and out-of-range `type`s read as `0`, matching C's own
    /// `cnr` bounds check and `default: return 0;` switch arm. The only
    /// C caller (`area/13/dungeon.c`'s raid-spawn setup) is not ported
    /// yet, so this currently has no live reader either - see the module
    /// doc comment.
    pub fn get_clan_dungeon(&self, cnr: u16, dungeon_type: i32) -> i32 {
        if !(1..=21).contains(&dungeon_type) {
            return 0;
        }
        self.identity(cnr)
            .map(|id| id.economy.dungeon_guard_use[(dungeon_type - 1) as usize])
            .unwrap_or(0)
    }

    /// C `set_clan_dungeon_use` (`clan.c:617-729`): the clan-leader-facing
    /// dungeon-guard configuration setter, live behind the clanclerk
    /// NPC's `use` command (`crate::world::clanclerk`). Validates `cnr`
    /// (via [`ClanRegistry::identity_mut`]), the `type`/`number` range for
    /// that specific slot (`clan.c:619-664`), then recomputes the total
    /// training-point cost across all 21 slots (`clan.c:667-682`) - the
    /// candidate `number` substituted in for `dungeon_type`'s own slot,
    /// every other slot read at its current stored value - and rejects
    /// (without mutating anything) if that total exceeds 400 while
    /// raising a value.
    pub fn set_clan_dungeon_use(
        &mut self,
        cnr: u16,
        dungeon_type: i32,
        number: i32,
    ) -> Result<(), ClanDungeonUseError> {
        let identity = self
            .identity_mut(cnr)
            .ok_or(ClanDungeonUseError::InvalidRequest)?;
        if !(1..=21).contains(&dungeon_type) {
            return Err(ClanDungeonUseError::InvalidRequest);
        }
        let max_for_type = match dungeon_type {
            19 => 25,
            20 => 1,
            21 => 2,
            _ => 10,
        };
        if number < 0 || number > max_for_type {
            return Err(ClanDungeonUseError::InvalidRequest);
        }

        let mut cost = 0;
        for (offset, &current) in identity.economy.dungeon_guard_use.iter().enumerate() {
            let slot_type = (offset + 1) as i32;
            let value = if slot_type == dungeon_type {
                number
            } else {
                current
            };
            cost += get_clan_dungeon_cost(slot_type, value);
        }
        if cost > 400 && number > 0 {
            return Err(ClanDungeonUseError::OverBudget(cost));
        }

        identity.economy.dungeon_guard_use[(dungeon_type - 1) as usize] = number;
        self.dirty = true;
        Ok(())
    }

    /// C `add_alc_potion` (`clan.c:1457-1474`): registers a finished
    /// alchemy flask handed to the clan clerk NPC (`NT_GIVE`,
    /// `area/30/clanmaster.c:1176-1188`) in the clan's dungeon-guard
    /// potion stockpile. Takes the flask's own `modifier_index`/
    /// `modifier_value` (C's `it[in].mod_index`/`mod_value`) directly
    /// rather than a whole `Item`, keeping this module's existing "no
    /// `Item`/`Character` types beyond what's needed" import discipline.
    /// Returns `false` for a mismatched driver/modifier combination (C's
    /// `return -1`) - the caller (`crate::world::clanclerk`) uses that to
    /// decide whether to destroy the item or leave it (matching C's own
    /// "try to give it back, then let it vanish" fallback).
    ///
    /// Deviation from C: `str = min(5, (mod_value[0]/4)-1)` can go
    /// negative in C when `mod_value[0] < 4` (an out-of-bounds array
    /// write in the original - never observed from a real finished
    /// flask, whose modifier values are always `>= 4`, see
    /// `item_driver::alchemy::finish_flask_mix`) - clamped to `0` here
    /// instead of guessing at C's undefined behavior.
    pub fn add_alc_potion(
        &mut self,
        cnr: u16,
        modifier_index: [i16; MAX_MODIFIERS],
        modifier_value: [i16; MAX_MODIFIERS],
    ) -> bool {
        let kind = if modifier_index[0] == CharacterValue::Attack as i16
            && modifier_index[1] == CharacterValue::Parry as i16
            && modifier_index[2] == CharacterValue::Immunity as i16
        {
            0usize
        } else if modifier_index[0] == CharacterValue::Flash as i16
            && modifier_index[1] == CharacterValue::MagicShield as i16
            && modifier_index[2] == CharacterValue::Immunity as i16
        {
            1usize
        } else {
            return false;
        };
        let tier = (i32::from(modifier_value[0]) / 4 - 1).clamp(0, 5) as usize;
        let Some(identity) = self.identity_mut(cnr) else {
            return false;
        };
        identity.economy.alc_pot[kind][tier] += 1;
        self.dirty = true;
        true
    }

    /// C `add_simple_potion`'s per-match stockpile increment
    /// (`clan.c:1487-1521`'s nine `if (flag) { ... clan[nr].dungeon.
    /// simple_pot[k][s]++; }` branches) - given an already-classified
    /// `(kind, size)` slot (see `crate::world::clanclerk`'s per-item
    /// pattern match on `drdata[1..4]`, the only caller), bumps
    /// `ClanEconomy::simple_pot[kind][size]`. Returns `false` (no-op) for
    /// a nonexistent clan, matching every other `ClanIdentity`-mutating
    /// method in this module.
    pub fn bump_simple_pot(&mut self, cnr: u16, kind: usize, size: usize) -> bool {
        let Some(identity) = self.identity_mut(cnr) else {
            return false;
        };
        identity.economy.simple_pot[kind][size] += 1;
        self.dirty = true;
        true
    }

    /// C `get_clan_raid` (`clan.c:1541-1543`). Out-of-range/nonexistent
    /// clans read as `false` (C reads `clan[cnr].dungeon.doraid` directly
    /// with no bounds check at all - a stricter Rust seam, same reasoning
    /// as every other read accessor in this module).
    pub fn get_clan_raid(&self, cnr: u16) -> bool {
        self.identity(cnr).is_some_and(|id| id.economy.raid)
    }

    /// C `set_clan_raid` (`clan.c:547-563`): the member-facing "raiding
    /// on"/"raiding off" toggle. Only ever sets the *pending* `raid_on_
    /// start` timestamp, never `raid` itself directly (see
    /// [`ClanRegistry::set_clan_raid_god`] for the only path that flips
    /// `raid` - matching C exactly: outside of `update_relations`'s
    /// intentionally-unported first-tick auto-enable - see the module
    /// doc comment - nothing ever promotes a pending `raid_on_start`
    /// request into `raid = true` on its own). Returns
    /// [`ClanRaidError::NoOp`] for C's `return 1` case (asking to turn on
    /// something already on/pending, or off something not pending),
    /// matching C's silent-failure branch the driver reports as "I'm
    /// sorry, I was unable to enable/disable raiding for your clan."
    pub fn set_clan_raid(&mut self, cnr: u16, onoff: bool, now: i64) -> Result<(), ClanRaidError> {
        let identity = self.identity_mut(cnr).ok_or(ClanRaidError::NotFound)?;
        let economy = &mut identity.economy;
        if onoff && !economy.raid && economy.raid_on_start == 0 {
            economy.raid_on_start = now;
            self.dirty = true;
            return Ok(());
        }
        if !onoff && !economy.raid && economy.raid_on_start != 0 {
            economy.raid_on_start = 0;
            self.dirty = true;
            return Ok(());
        }
        Err(ClanRaidError::NoOp)
    }

    /// C `set_clan_raid_god` (`clan.c:565-580`): the GM-only immediate
    /// override, flipping `raid` directly (skipping the member-facing
    /// pending-timer dance [`ClanRegistry::set_clan_raid`] goes through).
    pub fn set_clan_raid_god(&mut self, cnr: u16, onoff: bool) -> Result<(), ClanRaidError> {
        let identity = self.identity_mut(cnr).ok_or(ClanRaidError::NotFound)?;
        let economy = &mut identity.economy;
        if onoff && !economy.raid {
            economy.raid_on_start = 0;
            economy.raid = true;
            self.dirty = true;
            return Ok(());
        }
        if !onoff && economy.raid {
            economy.raid_on_start = 0;
            economy.raid = false;
            self.dirty = true;
            return Ok(());
        }
        Err(ClanRaidError::NoOp)
    }

    /// C `update_treasure` (`clan.c:1105-1159`), the periodic tick run for
    /// every existing clan: shrinks bonuses that have become unaffordable,
    /// recomputes the weekly upkeep cost, accrues debt for elapsed time
    /// since `payed_till`, auto-pays off debt with jewels when possible,
    /// and deletes any clan whose debt reaches 2 jewels' worth
    /// (`>= 2000`). Wired into the live tick loop by
    /// `crates/ugaris-server/src/world_events.rs`'s
    /// `apply_clan_economy_tick`.
    pub fn update_treasure(&mut self, now: i64) -> Vec<ClanTreasuryEvent> {
        let mut events = Vec::new();
        for cnr in 1..MAX_CLAN {
            let Some(identity) = self.identities[cnr].as_mut() else {
                continue;
            };
            let economy = &mut identity.economy;

            // C's `do { ... } while (cost > 0 && cost / 250 > jewels)`.
            let mut cost;
            loop {
                cost = economy.bonus_upkeep_cost();
                if cost / 250 > economy.treasure.jewels {
                    economy.reduce_highest_bonus();
                }
                if !(cost > 0 && cost / 250 > economy.treasure.jewels) {
                    break;
                }
            }

            cost += CLAN_HALL_RENT * 1000;

            if economy.treasure.cost_per_week != cost {
                economy.treasure.cost_per_week = cost;
                self.dirty = true;
            }

            let diff = now - economy.treasure.payed_till;
            if diff > 60 * 5 {
                // update 5 minutes late to reduce load
                let step = (60 * 60 * 24 * 7) / cost as i64;
                let n = diff / step + 1;
                economy.treasure.debt += n as i32;
                economy.treasure.payed_till += step * n;
                self.dirty = true;
            }

            if economy.treasure.debt >= 1000 && economy.treasure.jewels > 0 {
                let mut n = economy.treasure.debt / 1000;
                if n > economy.treasure.jewels {
                    n = economy.treasure.jewels;
                    economy.treasure.jewels = 0;
                } else {
                    economy.treasure.jewels -= n;
                }
                economy.treasure.debt -= n * 1000;
                self.dirty = true;
                events.push(ClanTreasuryEvent::PaidDebtWithJewels {
                    clan: cnr as u16,
                    jewels_paid: n,
                });
            }

            if economy.treasure.debt >= 2000 {
                let name = identity.name.clone();
                // C logs `get_clan_name(cnr)`/`clan_serial(cnr)` *before*
                // clearing the name and bumping `status.serial`
                // (`clan.c:1155-1158`), so the serial captured here must
                // be the pre-deletion one - [`ClanRegistry::delete_clan`]
                // bumps it, and any post-hoc `self.serial(cnr)` call
                // after this method returns would observe the bumped
                // value instead.
                let serial = self.serials[cnr];
                events.push(ClanTreasuryEvent::WentBroke {
                    clan: cnr as u16,
                    serial,
                    name,
                });
                self.delete_clan(cnr as u16);
            }
        }
        events
    }

    /// C `update_training` (`clan.c:1166-1182`): once an hour per clan,
    /// decays the clan's dungeon training score by 5% (`* 0.95f`, C's
    /// `xlog` here is a server debug log only, no player-facing
    /// clan-log entry - unlike [`ClanRegistry::update_treasure`]'s broke
    /// deletion, so this has no event return). Wired into the live tick
    /// loop alongside `update_treasure` - see that method's doc comment.
    pub fn update_training(&mut self, now: i64) {
        for cnr in 1..MAX_CLAN {
            let Some(identity) = self.identities[cnr].as_mut() else {
                continue;
            };
            let economy = &mut identity.economy;
            if now - economy.last_training_update < 60 * 60 {
                continue;
            }
            economy.last_training_update = now;
            economy.training_score = (economy.training_score as f32 * 0.95f32) as i32;
            self.dirty = true;
        }
    }
}

/// Result of [`ClanRegistry::clan_money_change`] when the change should be
/// clan-logged - the caller formats these with the acting character's
/// name (`clan.c:236-241`'s `"%s deposited %dG"`/`"%s withdrew %dG"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClanMoneyChange {
    Deposited(i32),
    Withdrew(i32),
}

impl ClanMoneyChange {
    /// C `clan_money_change`'s two `add_clanlog` message shapes
    /// (`clan.c:1245,1247`): `"%s deposited %dG"`/`"%s withdrew %dG"`,
    /// prio 28 (left for the caller to pass alongside this, matching
    /// every other clan-log write site in this codebase).
    pub fn log_message(&self, actor_name: &str) -> String {
        match self {
            ClanMoneyChange::Deposited(amount) => format!("{actor_name} deposited {amount}G"),
            ClanMoneyChange::Withdrew(amount) => format!("{actor_name} withdrew {amount}G"),
        }
    }
}

/// Events produced by [`ClanRegistry::update_treasure`]. See each
/// variant's doc comment for which C log (server debug `xlog` vs
/// player-facing `add_clanlog`) it corresponds to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClanTreasuryEvent {
    /// C `xlog("clan %s, paid %d jewels", ...)` (`clan.c:1151`) - server
    /// debug log only, no player-facing clan-log entry in C.
    PaidDebtWithJewels { clan: u16, jewels_paid: i32 },
    /// C: the bankrupt-clan deletion path (`clan.c:1154-1160`) -
    /// `xlog("clan %s is broke, removing", ...)` plus a real
    /// player-facing `add_clanlog(cnr, ..., "Clan %s went broke and was
    /// deleted", ...)` entry (priority 1, actor char ID 0 meaning
    /// "system").
    WentBroke {
        clan: u16,
        serial: u32,
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn found_clan_resets_relations_to_neutral_both_ways() {
        let mut relations = ClanRelations::new();
        assert!(relations.found_clan(1, 1_000));
        assert!(relations.exists(1));
        assert_eq!(relations.current_relation(1, 5), ClanRelation::Neutral);
        assert_eq!(relations.current_relation(5, 1), ClanRelation::Neutral);
    }

    #[test]
    fn found_clan_rejects_out_of_range_numbers() {
        let mut relations = ClanRelations::new();
        assert!(!relations.found_clan(0, 0));
        assert!(!relations.found_clan(32, 0));
    }

    #[test]
    fn set_relation_validates_inputs() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        assert_eq!(
            relations.set_relation(0, 2, ClanRelation::War, 0),
            Err(ClanRelationError::InvalidClan(0))
        );
        assert_eq!(
            relations.set_relation(1, 32, ClanRelation::War, 0),
            Err(ClanRelationError::InvalidClan(32))
        );
        assert_eq!(
            relations.set_relation(1, 2, ClanRelation::None, 0),
            Err(ClanRelationError::InvalidRelation)
        );
    }

    #[test]
    fn score_to_level_matches_c_integer_division() {
        assert_eq!(score_to_level(0), 0);
        assert_eq!(score_to_level(99), 0);
        assert_eq!(score_to_level(100), 1);
        assert_eq!(score_to_level(999), 9);
        assert_eq!(score_to_level(1000), 10);
    }

    #[test]
    fn want_relation_and_want_date_read_the_set_relation_side() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations
            .set_relation(1, 2, ClanRelation::War, 500)
            .unwrap();

        assert_eq!(relations.want_relation(1, 2), ClanRelation::War);
        assert_eq!(relations.want_date(1, 2), 500);
        // The reverse direction and the current relation are unaffected.
        assert_eq!(relations.want_relation(2, 1), ClanRelation::Neutral);
        assert_eq!(relations.current_relation(1, 2), ClanRelation::Neutral);
    }

    #[test]
    fn want_relation_and_want_date_are_none_zero_for_invalid_clans() {
        let relations = ClanRelations::new();
        assert_eq!(relations.want_relation(0, 1), ClanRelation::None);
        assert_eq!(relations.want_relation(1, 32), ClanRelation::None);
        assert_eq!(relations.want_date(0, 1), 0);
    }

    #[test]
    fn set_relation_only_bumps_want_date_on_change() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations
            .set_relation(1, 2, ClanRelation::War, 100)
            .unwrap();
        // Re-requesting the same relation later must not reset the timer,
        // matching C's `if (clan[cnr].status.want_relation[onr] != rel)`
        // guard (`clan.c:850-852`).
        relations
            .set_relation(1, 2, ClanRelation::War, 500)
            .unwrap();
        assert_eq!(relations.want_date[1][2], 100);
    }

    #[test]
    fn both_want_same_relation_takes_effect_immediately() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations.set_relation(1, 2, ClanRelation::War, 0).unwrap();
        relations.set_relation(2, 1, ClanRelation::War, 0).unwrap();
        let events = relations.update(0);
        assert_eq!(
            events,
            vec![ClanRelationEvent {
                clan_a: 1,
                clan_b: 2,
                change: ClanRelationChange::Agreed {
                    relation: ClanRelation::War
                },
            }]
        );
        assert_eq!(relations.current_relation(1, 2), ClanRelation::War);
        assert_eq!(relations.current_relation(2, 1), ClanRelation::War);
    }

    #[test]
    fn one_sided_war_request_needs_one_hour() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations.set_relation(1, 2, ClanRelation::War, 0).unwrap();

        // Before the 1h delay: no change yet.
        let events = relations.update(60 * 60 - 1);
        assert!(events.is_empty());
        assert_eq!(relations.current_relation(1, 2), ClanRelation::Neutral);

        // After the 1h delay: escalates to war.
        let events = relations.update(60 * 60 + 1);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].change, ClanRelationChange::WarStarted);
        assert_eq!(relations.current_relation(1, 2), ClanRelation::War);
    }

    #[test]
    fn both_want_same_relation_jumps_directly_even_across_multiple_steps() {
        // C: the `want1 == want2` check (`clan.c:980-985`) happens before the
        // per-level switch, so when both clans agree on a new relation it
        // takes effect in one tick regardless of how many levels away it is
        // from the current one - it does not step through intermediate
        // levels one tick at a time.
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations
            .set_relation(1, 2, ClanRelation::Alliance, 0)
            .unwrap();
        relations
            .set_relation(2, 1, ClanRelation::Alliance, 0)
            .unwrap();
        let events = relations.update(0);
        assert_eq!(
            events[0].change,
            ClanRelationChange::Agreed {
                relation: ClanRelation::Alliance
            }
        );
        assert_eq!(relations.current_relation(1, 2), ClanRelation::Alliance);
    }

    #[test]
    fn both_want_better_but_different_relations_deescalates_one_step() {
        // Clan 1 wants Alliance, clan 2 wants Peace-Treaty: both want
        // something better than Neutral, but they disagree, so the switch
        // branch applies (one step toward Alliance) rather than the
        // immediate-agreement branch.
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations
            .set_relation(1, 2, ClanRelation::Alliance, 0)
            .unwrap();
        relations
            .set_relation(2, 1, ClanRelation::PeaceTreaty, 0)
            .unwrap();
        let events = relations.update(0);
        assert_eq!(events[0].change, ClanRelationChange::PeaceTreatyStarted);
        assert_eq!(relations.current_relation(1, 2), ClanRelation::PeaceTreaty);
    }

    #[test]
    fn alliance_ends_after_24h_one_sided_request() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations
            .set_relation(1, 2, ClanRelation::Alliance, 0)
            .unwrap();
        relations
            .set_relation(2, 1, ClanRelation::Alliance, 0)
            .unwrap();
        relations.update(0);
        assert_eq!(relations.current_relation(1, 2), ClanRelation::Alliance);

        // One side wants out.
        relations
            .set_relation(1, 2, ClanRelation::PeaceTreaty, 1_000)
            .unwrap();

        let events = relations.update(1_000 + 60 * 60 * 24 - 1);
        assert!(events.is_empty());
        assert_eq!(relations.current_relation(1, 2), ClanRelation::Alliance);

        let events = relations.update(1_000 + 60 * 60 * 24 + 1);
        assert_eq!(events[0].change, ClanRelationChange::AllianceEnded);
        assert_eq!(relations.current_relation(1, 2), ClanRelation::PeaceTreaty);
    }

    #[test]
    fn war_ends_only_when_both_want_better() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations.set_relation(1, 2, ClanRelation::War, 0).unwrap();
        relations.set_relation(2, 1, ClanRelation::War, 0).unwrap();
        relations.update(0);
        assert_eq!(relations.current_relation(1, 2), ClanRelation::War);

        // Only clan 1 wants peace: war does not end, even after a long time.
        relations
            .set_relation(1, 2, ClanRelation::Neutral, 0)
            .unwrap();
        let events = relations.update(60 * 60 * 24 * 30);
        assert!(events.is_empty());
        assert_eq!(relations.current_relation(1, 2), ClanRelation::War);

        // Both want a better relation now, but *different* ones (Neutral vs
        // Peace-Treaty): this exercises the one-step de-escalation switch
        // branch rather than the immediate-agreement branch, since the two
        // wants differ.
        relations
            .set_relation(2, 1, ClanRelation::PeaceTreaty, 60 * 60 * 24 * 30)
            .unwrap();
        let events = relations.update(60 * 60 * 24 * 30);
        assert_eq!(events[0].change, ClanRelationChange::WarEnded);
        assert_eq!(relations.current_relation(1, 2), ClanRelation::Neutral);
    }

    #[test]
    fn feud_ends_after_24h_one_sided_request() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations.set_relation(1, 2, ClanRelation::Feud, 0).unwrap();
        relations.set_relation(2, 1, ClanRelation::Feud, 0).unwrap();
        relations.update(0);
        assert_eq!(relations.current_relation(1, 2), ClanRelation::Feud);

        relations
            .set_relation(1, 2, ClanRelation::War, 5_000)
            .unwrap();
        let events = relations.update(5_000 + 60 * 60 * 24 + 1);
        assert_eq!(events[0].change, ClanRelationChange::FeudEnded);
        assert_eq!(relations.current_relation(1, 2), ClanRelation::War);
    }

    #[test]
    fn relation_change_log_messages_match_c_add_clanlog_text_exactly() {
        // `clan.c:980-1083`'s seven distinct `add_clanlog` message shapes,
        // letter-for-letter (note the "Peace-Treaty" vs "Peace Treaty"
        // discrepancy between the `rel_name[]`-driven `Agreed` message
        // and the other hardcoded ones is intentional, matching C).
        assert_eq!(
            ClanRelationChange::Agreed {
                relation: ClanRelation::War
            }
            .log_message("Enemies", 3),
            "War with Enemies (3) started"
        );
        assert_eq!(
            ClanRelationChange::Agreed {
                relation: ClanRelation::PeaceTreaty
            }
            .log_message("Friends", 2),
            "Peace-Treaty with Friends (2) started"
        );
        assert_eq!(
            ClanRelationChange::AllianceEnded.log_message("Foo", 1),
            "Alliance with Foo (1) ended"
        );
        assert_eq!(
            ClanRelationChange::PeaceTreatyEnded.log_message("Foo", 1),
            "Peace Treaty with Foo (1) ended"
        );
        assert_eq!(
            ClanRelationChange::WarStarted.log_message("Foo", 1),
            "War with Foo (1) started"
        );
        assert_eq!(
            ClanRelationChange::PeaceTreatyStarted.log_message("Foo", 1),
            "Peace Treaty with Foo (1) started"
        );
        assert_eq!(
            ClanRelationChange::WarEnded.log_message("Foo", 1),
            "War with Foo (1) ended"
        );
        assert_eq!(
            ClanRelationChange::FeudEnded.log_message("Foo", 1),
            "Feud with Foo (1) ended"
        );
    }

    #[test]
    fn may_enter_own_clan_always_allowed() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        assert!(relations.may_enter(1, 1));
    }

    #[test]
    fn may_enter_denied_for_non_clan_members() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        assert!(!relations.may_enter(0, 1));
    }

    #[test]
    fn may_enter_denied_for_deleted_clan() {
        let relations = ClanRelations::new();
        assert!(!relations.may_enter(1, 5));
    }

    #[test]
    fn may_enter_allowed_only_with_alliance() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        assert!(!relations.may_enter(1, 2)); // still neutral

        relations
            .set_relation(2, 1, ClanRelation::Alliance, 0)
            .unwrap();
        relations
            .set_relation(1, 2, ClanRelation::Alliance, 0)
            .unwrap();
        relations.update(0);
        assert!(relations.may_enter(1, 2));
    }

    #[test]
    fn attack_outside_requires_feud() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        assert!(!relations.can_attack_outside(1, 2)); // neutral

        relations.set_relation(1, 2, ClanRelation::Feud, 0).unwrap();
        relations.set_relation(2, 1, ClanRelation::Feud, 0).unwrap();
        relations.update(0);
        assert!(relations.can_attack_outside(1, 2));
        assert!(relations.can_attack_inside(1, 2)); // war/feud also allow inside
    }

    #[test]
    fn attack_inside_allows_war_or_feud_not_neutral() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        assert!(!relations.can_attack_inside(1, 2));

        relations.set_relation(1, 2, ClanRelation::War, 0).unwrap();
        relations.set_relation(2, 1, ClanRelation::War, 0).unwrap();
        relations.update(0);
        assert!(relations.can_attack_inside(1, 2));
        assert!(!relations.can_attack_outside(1, 2)); // war alone doesn't allow outside
    }

    #[test]
    fn alliance_query_matches_current_relation() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        assert!(!relations.alliance(1, 2));

        relations
            .set_relation(1, 2, ClanRelation::Alliance, 0)
            .unwrap();
        relations
            .set_relation(2, 1, ClanRelation::Alliance, 0)
            .unwrap();
        relations.update(0);
        assert!(relations.alliance(1, 2));
        assert!(relations.alliance(2, 1));
    }

    #[test]
    fn delete_clan_clears_existence_but_keeps_relations() {
        let mut relations = ClanRelations::new();
        relations.found_clan(1, 0);
        relations.found_clan(2, 0);
        relations.delete_clan(2);
        assert!(!relations.exists(2));
        assert!(!relations.may_enter(1, 2));
    }

    #[test]
    fn out_of_range_queries_return_safe_defaults() {
        let relations = ClanRelations::new();
        assert!(!relations.exists(0));
        assert!(!relations.exists(32));
        assert_eq!(relations.current_relation(0, 1), ClanRelation::None);
        assert!(!relations.can_attack_inside(0, 1));
        assert!(!relations.can_attack_outside(1, 100));
        assert!(!relations.alliance(1, 100));
    }

    fn test_character() -> Character {
        Character {
            merchant: None,
            template_key: String::new(),
            respawn_ticks: 0,
            id: crate::ids::CharacterId(1),
            serial: 1,
            name: "tester".to_string(),
            description: String::new(),
            flags: crate::entity::CharacterFlags::USED,
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
            speed_mode: crate::entity::SpeedMode::Normal,
            x: 0,
            y: 0,
            rest_area: 0,
            rest_x: 0,
            rest_y: 0,
            tox: 0,
            toy: 0,
            dir: 4,
            action: 0,
            duration: 0,
            step: 0,
            act1: 0,
            act2: 0,
            hp: 1000,
            mana: 1000,
            endurance: 1000,
            lifeshield: 0,
            level: 1,
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
            driver_memory: crate::character_driver::DriverMemory::default(),
            class: 0,
        }
    }

    #[test]
    fn registry_found_clan_allocates_first_free_slot_with_standard_ranks() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("The Founders", 1_000).unwrap();
        assert_eq!(nr, 1);
        assert!(registry.exists(1));
        assert_eq!(registry.name(1), Some("The Founders"));
        let identity = registry.identity(1).unwrap();
        assert_eq!(
            identity.rank_names,
            ["Member", "Member", "Recruiter", "Treasurer", "Leader"]
        );
        // Relations were reset to neutral by the wrapped ClanRelations.
        assert_eq!(
            registry.relations().current_relation(1, 5),
            ClanRelation::Neutral
        );
    }

    #[test]
    fn registry_found_clan_rejects_names_over_78_chars() {
        let mut registry = ClanRegistry::new();
        let long_name = "x".repeat(79);
        assert_eq!(
            registry.found_clan(&long_name, 0),
            Err(ClanFoundError::NameTooLong)
        );
    }

    #[test]
    fn registry_found_clan_reuses_slot_after_delete_and_bumps_serial() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("First", 0).unwrap();
        assert_eq!(registry.serial(nr), 0);
        registry.delete_clan(nr);
        assert!(!registry.exists(nr));

        let nr2 = registry.found_clan("Second", 0).unwrap();
        assert_eq!(nr2, nr); // same slot reused
        assert_eq!(registry.name(nr2), Some("Second"));
        assert_eq!(registry.serial(nr2), 1); // serial bumped by delete_clan
    }

    #[test]
    fn registry_found_clan_returns_list_full_when_all_slots_used() {
        let mut registry = ClanRegistry::new();
        for n in 1..MAX_CLAN {
            registry.found_clan(&format!("Clan{n}"), 0).unwrap();
        }
        assert_eq!(
            registry.found_clan("Overflow", 0),
            Err(ClanFoundError::ClanListFull)
        );
    }

    #[test]
    fn add_member_then_get_char_clan_round_trips() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Allies", 0).unwrap();
        let mut character = test_character();

        registry.add_member(&mut character, nr).unwrap();
        assert_eq!(character.clan, nr);
        assert_eq!(character.clan_serial, registry.serial(nr));
        assert_eq!(character.clan_rank, 0); // add_member never sets rank

        assert_eq!(registry.get_char_clan(&mut character), Some(nr));
        assert_eq!(character.clan, nr); // untouched on success
    }

    #[test]
    fn add_member_rejects_unknown_clan() {
        let registry = ClanRegistry::new();
        let mut character = test_character();
        assert_eq!(
            registry.add_member(&mut character, 5),
            Err(ClanMembershipError::NotFound)
        );
    }

    #[test]
    fn add_member_rejects_club_numbers() {
        let registry = ClanRegistry::new();
        let mut character = test_character();
        assert_eq!(
            registry.add_member(&mut character, CLUB_OFFSET),
            Err(ClanMembershipError::IsClub)
        );
    }

    #[test]
    fn remove_member_clears_all_three_fields() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Allies", 0).unwrap();
        let mut character = test_character();
        registry.add_member(&mut character, nr).unwrap();
        character.clan_rank = 3;

        registry.remove_member(&mut character);
        assert_eq!(character.clan, 0);
        assert_eq!(character.clan_rank, 0);
        assert_eq!(character.clan_serial, 0);
    }

    #[test]
    fn get_char_clan_clears_stale_reference_after_clan_deleted_and_refounded() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Original", 0).unwrap();
        let mut character = test_character();
        registry.add_member(&mut character, nr).unwrap();

        registry.delete_clan(nr);
        let nr2 = registry.found_clan("Replacement", 0).unwrap();
        assert_eq!(nr2, nr);

        // Character still has the old serial - must be treated as former
        // member of a now-different clan, exactly like C's
        // `ch[cn].clan_serial != clan[cnr].status.serial` check.
        assert_eq!(registry.get_char_clan(&mut character), None);
        assert_eq!(character.clan, 0);
        assert_eq!(character.clan_rank, 0);
        assert_eq!(character.clan_serial, 0);
    }

    #[test]
    fn get_char_clan_ignores_club_numbers() {
        let registry = ClanRegistry::new();
        let mut character = test_character();
        character.clan = CLUB_OFFSET + 3;
        character.clan_rank = 2;
        character.clan_serial = 7;

        assert_eq!(registry.get_char_clan(&mut character), None);
        // Untouched: club membership is a different (unported) system.
        assert_eq!(character.clan, CLUB_OFFSET + 3);
        assert_eq!(character.clan_rank, 2);
        assert_eq!(character.clan_serial, 7);
    }

    #[test]
    fn get_char_clan_zero_means_no_clan() {
        let registry = ClanRegistry::new();
        let mut character = test_character();
        assert_eq!(registry.get_char_clan(&mut character), None);
    }

    #[test]
    fn char_clan_name_resolves_through_get_char_clan() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Named Clan", 0).unwrap();
        let mut character = test_character();
        registry.add_member(&mut character, nr).unwrap();
        assert_eq!(registry.char_clan_name(&mut character), Some("Named Clan"));
    }

    #[test]
    fn set_rankname_validates_rank_and_length() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Ranked", 0).unwrap();

        registry.set_rankname(nr, 4, "Warlord").unwrap();
        assert_eq!(registry.identity(nr).unwrap().rank_names[4], "Warlord");

        assert_eq!(
            registry.set_rankname(nr, 5, "Invalid"),
            Err(ClanIdentityError::InvalidRank)
        );
        assert_eq!(
            registry.set_rankname(nr, 0, &"x".repeat(38)),
            Err(ClanIdentityError::NameTooLong)
        );
        assert_eq!(
            registry.set_rankname(99, 0, "Nobody"),
            Err(ClanIdentityError::NotFound)
        );
    }

    #[test]
    fn set_website_and_message_update_identity() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Web", 0).unwrap();
        registry.set_website(nr, "https://example.com").unwrap();
        registry.set_message(nr, "Welcome!").unwrap();
        let identity = registry.identity(nr).unwrap();
        assert_eq!(identity.website, "https://example.com");
        assert_eq!(identity.message, "Welcome!");
    }

    #[test]
    fn set_website_truncates_to_79_chars() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Web", 0).unwrap();
        let long = "y".repeat(200);
        registry.set_website(nr, &long).unwrap();
        assert_eq!(registry.identity(nr).unwrap().website.len(), 79);
    }

    #[test]
    fn set_name_renames_an_existing_clan() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Old Name", 0).unwrap();
        registry.set_name(nr, "New Name").unwrap();
        assert_eq!(registry.name(nr), Some("New Name"));
    }

    #[test]
    fn set_name_truncates_to_78_chars_without_error() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Old Name", 0).unwrap();
        let long = "z".repeat(200);
        registry.set_name(nr, &long).unwrap();
        assert_eq!(registry.identity(nr).unwrap().name.len(), 78);
    }

    #[test]
    fn set_name_rejects_nonexistent_clan() {
        let mut registry = ClanRegistry::new();
        assert_eq!(
            registry.set_name(5, "Ghost"),
            Err(ClanIdentityError::NotFound)
        );
    }

    #[test]
    fn fresh_registry_is_not_dirty() {
        let registry = ClanRegistry::new();
        assert!(!registry.dirty());
    }

    #[test]
    fn found_clan_marks_registry_dirty() {
        let mut registry = ClanRegistry::new();
        assert!(!registry.dirty());
        registry.found_clan("Dirty", 0).unwrap();
        assert!(registry.dirty());
    }

    #[test]
    fn found_clan_failure_does_not_mark_dirty() {
        let mut registry = ClanRegistry::new();
        assert_eq!(
            registry.found_clan(&"x".repeat(79), 0),
            Err(ClanFoundError::NameTooLong)
        );
        assert!(!registry.dirty());
    }

    #[test]
    fn clear_dirty_resets_the_flag() {
        let mut registry = ClanRegistry::new();
        registry.found_clan("Dirty", 0).unwrap();
        assert!(registry.dirty());
        registry.clear_dirty();
        assert!(!registry.dirty());
    }

    #[test]
    fn delete_clan_marks_registry_dirty_only_when_valid() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Doomed", 0).unwrap();
        registry.clear_dirty();
        registry.delete_clan(999);
        assert!(!registry.dirty(), "out-of-range delete must not mutate");
        registry.delete_clan(nr);
        assert!(registry.dirty());
    }

    #[test]
    fn identity_mutators_mark_registry_dirty() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Mutable", 0).unwrap();

        registry.clear_dirty();
        registry.set_rankname(nr, 0, "Chief").unwrap();
        assert!(registry.dirty());

        registry.clear_dirty();
        registry.set_website(nr, "https://example.com").unwrap();
        assert!(registry.dirty());

        registry.clear_dirty();
        registry.set_message(nr, "hi").unwrap();
        assert!(registry.dirty());

        registry.clear_dirty();
        registry.set_name(nr, "Renamed").unwrap();
        assert!(registry.dirty());
    }

    #[test]
    fn identity_mutator_failure_does_not_mark_dirty() {
        let mut registry = ClanRegistry::new();
        assert_eq!(
            registry.set_rankname(1, 0, "Nobody"),
            Err(ClanIdentityError::NotFound)
        );
        assert!(!registry.dirty());
    }

    #[test]
    fn relations_mut_marks_registry_dirty() {
        let mut registry = ClanRegistry::new();
        assert!(!registry.dirty());
        registry.relations_mut().found_clan(1, 0);
        assert!(registry.dirty());
    }

    #[test]
    fn get_clan_raid_defaults_false_and_nonexistent_reads_false() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Raiders", 0).unwrap();
        assert!(!registry.get_clan_raid(nr));
        assert!(!registry.get_clan_raid(999));
    }

    #[test]
    fn set_clan_raid_on_then_off_toggles_pending_timer_not_raid_itself() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Raiders", 0).unwrap();

        assert_eq!(registry.set_clan_raid(nr, true, 1_000), Ok(()));
        // Only the pending timer moves; `get_clan_raid` (`doraid`) stays
        // false until a GM `set_clan_raid_god` override, matching C.
        assert!(!registry.get_clan_raid(nr));
        assert_eq!(registry.identity(nr).unwrap().economy.raid_on_start, 1_000);

        // Asking for "on" again while already pending is C's `return 1`
        // no-op case.
        assert_eq!(
            registry.set_clan_raid(nr, true, 2_000),
            Err(ClanRaidError::NoOp)
        );

        assert_eq!(registry.set_clan_raid(nr, false, 3_000), Ok(()));
        assert_eq!(registry.identity(nr).unwrap().economy.raid_on_start, 0);

        // Asking for "off" again with nothing pending is also a no-op.
        assert_eq!(
            registry.set_clan_raid(nr, false, 4_000),
            Err(ClanRaidError::NoOp)
        );
    }

    #[test]
    fn set_clan_raid_god_flips_raid_directly_and_clears_pending_timer() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Raiders", 0).unwrap();
        registry.set_clan_raid(nr, true, 1_000).unwrap();

        assert_eq!(registry.set_clan_raid_god(nr, true), Ok(()));
        assert!(registry.get_clan_raid(nr));
        assert_eq!(registry.identity(nr).unwrap().economy.raid_on_start, 0);

        // Already on: no-op.
        assert_eq!(
            registry.set_clan_raid_god(nr, true),
            Err(ClanRaidError::NoOp)
        );

        assert_eq!(registry.set_clan_raid_god(nr, false), Ok(()));
        assert!(!registry.get_clan_raid(nr));

        assert_eq!(
            registry.set_clan_raid_god(nr, false),
            Err(ClanRaidError::NoOp)
        );
    }

    #[test]
    fn set_clan_raid_nonexistent_clan_is_not_found() {
        let mut registry = ClanRegistry::new();
        assert_eq!(
            registry.set_clan_raid(999, true, 0),
            Err(ClanRaidError::NotFound)
        );
        assert_eq!(
            registry.set_clan_raid_god(999, true),
            Err(ClanRaidError::NotFound)
        );
    }

    #[test]
    fn get_clan_dungeon_cost_matches_c_multiplier_table() {
        // warrior/mage/seyan +0..+5 tiers all repeat 1/2/4/8/12/16.
        assert_eq!(get_clan_dungeon_cost(1, 3), 3);
        assert_eq!(get_clan_dungeon_cost(6, 3), 48);
        assert_eq!(get_clan_dungeon_cost(7, 3), 3);
        assert_eq!(get_clan_dungeon_cost(12, 3), 48);
        assert_eq!(get_clan_dungeon_cost(13, 3), 3);
        assert_eq!(get_clan_dungeon_cost(18, 3), 48);
        assert_eq!(get_clan_dungeon_cost(19, 2), 16); // teleport traps *8
        assert_eq!(get_clan_dungeon_cost(20, 1), 16); // fake wall *16
        assert_eq!(get_clan_dungeon_cost(21, 2), 24); // locked door key *12
        assert_eq!(get_clan_dungeon_cost(22, 5), 0); // unknown type -> 0
    }

    #[test]
    fn set_clan_dungeon_use_rejects_invalid_type_or_out_of_range_number() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Dungeoneers", 0).unwrap();
        assert_eq!(
            registry.set_clan_dungeon_use(nr, 0, 1),
            Err(ClanDungeonUseError::InvalidRequest)
        );
        assert_eq!(
            registry.set_clan_dungeon_use(nr, 22, 1),
            Err(ClanDungeonUseError::InvalidRequest)
        );
        // warrior/mage/seyan slots cap at 10.
        assert_eq!(
            registry.set_clan_dungeon_use(nr, 1, 11),
            Err(ClanDungeonUseError::InvalidRequest)
        );
        // teleport traps cap at 25.
        assert_eq!(
            registry.set_clan_dungeon_use(nr, 19, 26),
            Err(ClanDungeonUseError::InvalidRequest)
        );
        // fake walls cap at 1.
        assert_eq!(
            registry.set_clan_dungeon_use(nr, 20, 2),
            Err(ClanDungeonUseError::InvalidRequest)
        );
        // locked doors cap at 2.
        assert_eq!(
            registry.set_clan_dungeon_use(nr, 21, 3),
            Err(ClanDungeonUseError::InvalidRequest)
        );
        assert_eq!(
            registry.set_clan_dungeon_use(999, 1, 1),
            Err(ClanDungeonUseError::InvalidRequest)
        );
    }

    #[test]
    fn set_clan_dungeon_use_applies_within_budget_and_reads_back() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Dungeoneers", 0).unwrap();
        assert_eq!(registry.get_clan_dungeon(nr, 1), 0);
        assert_eq!(registry.set_clan_dungeon_use(nr, 1, 5), Ok(()));
        assert_eq!(registry.get_clan_dungeon(nr, 1), 5);
        // Lowering back to 0 is always allowed.
        assert_eq!(registry.set_clan_dungeon_use(nr, 1, 0), Ok(()));
        assert_eq!(registry.get_clan_dungeon(nr, 1), 0);
    }

    #[test]
    fn set_clan_dungeon_use_rejects_over_budget_configuration_without_mutating() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Dungeoneers", 0).unwrap();
        // Warrior slots 1-5 (multipliers 1/2/4/8/12) maxed at 10 each:
        // running cost 10, 30, 70, 150, 270 - all within budget.
        assert_eq!(registry.set_clan_dungeon_use(nr, 1, 10), Ok(()));
        assert_eq!(registry.set_clan_dungeon_use(nr, 2, 10), Ok(()));
        assert_eq!(registry.set_clan_dungeon_use(nr, 3, 10), Ok(()));
        assert_eq!(registry.set_clan_dungeon_use(nr, 4, 10), Ok(()));
        assert_eq!(registry.set_clan_dungeon_use(nr, 5, 10), Ok(()));
        // Slot 6 (multiplier 16) at 8: 270 + 128 = 398, still <= 400.
        assert_eq!(registry.set_clan_dungeon_use(nr, 6, 8), Ok(()));
        assert_eq!(registry.get_clan_dungeon(nr, 6), 8);
        // Raising slot 6 to 9 would cost 270 + 144 = 414 > 400 - rejected
        // without mutating the stored value.
        match registry.set_clan_dungeon_use(nr, 6, 9) {
            Err(ClanDungeonUseError::OverBudget(cost)) => assert_eq!(cost, 414),
            other => panic!("expected OverBudget(414), got {other:?}"),
        }
        assert_eq!(registry.get_clan_dungeon(nr, 6), 8);
        // Lowering a different slot is always allowed even while the
        // clan sits near budget.
        assert_eq!(registry.set_clan_dungeon_use(nr, 1, 0), Ok(()));
        assert_eq!(registry.get_clan_dungeon(nr, 1), 0);
    }

    #[test]
    fn get_clan_dungeon_reads_zero_for_invalid_type_or_clan() {
        let registry = ClanRegistry::new();
        assert_eq!(registry.get_clan_dungeon(999, 1), 0);
        assert_eq!(registry.get_clan_dungeon(1, 0), 0);
        assert_eq!(registry.get_clan_dungeon(1, 22), 0);
    }

    #[test]
    fn add_alc_potion_matches_attack_recipe_and_computes_tier() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Alchemists", 0).unwrap();
        let modifier_index = {
            let mut idx = [-1i16; MAX_MODIFIERS];
            idx[0] = CharacterValue::Attack as i16;
            idx[1] = CharacterValue::Parry as i16;
            idx[2] = CharacterValue::Immunity as i16;
            idx
        };
        let modifier_value = {
            let mut val = [0i16; MAX_MODIFIERS];
            val[0] = 12; // tier (12/4)-1 = 2
            val
        };
        assert!(registry.add_alc_potion(nr, modifier_index, modifier_value));
        assert_eq!(registry.identity(nr).unwrap().economy.alc_pot[0][2], 1);
    }

    #[test]
    fn add_alc_potion_matches_flash_recipe_and_clamps_tier_at_five() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Alchemists", 0).unwrap();
        let modifier_index = {
            let mut idx = [-1i16; MAX_MODIFIERS];
            idx[0] = CharacterValue::Flash as i16;
            idx[1] = CharacterValue::MagicShield as i16;
            idx[2] = CharacterValue::Immunity as i16;
            idx
        };
        let modifier_value = {
            let mut val = [0i16; MAX_MODIFIERS];
            val[0] = 40; // (40/4)-1 = 9, clamped to 5
            val
        };
        assert!(registry.add_alc_potion(nr, modifier_index, modifier_value));
        assert_eq!(registry.identity(nr).unwrap().economy.alc_pot[1][5], 1);
    }

    #[test]
    fn add_alc_potion_rejects_unmatched_modifiers() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Alchemists", 0).unwrap();
        assert!(!registry.add_alc_potion(nr, [-1; MAX_MODIFIERS], [0; MAX_MODIFIERS]));
        assert_eq!(
            registry.identity(nr).unwrap().economy.alc_pot,
            [[0; 6], [0; 6]]
        );
    }

    #[test]
    fn add_alc_potion_returns_false_for_nonexistent_clan() {
        let mut registry = ClanRegistry::new();
        let modifier_index = {
            let mut idx = [-1i16; MAX_MODIFIERS];
            idx[0] = CharacterValue::Attack as i16;
            idx[1] = CharacterValue::Parry as i16;
            idx[2] = CharacterValue::Immunity as i16;
            idx
        };
        assert!(!registry.add_alc_potion(999, modifier_index, [4; MAX_MODIFIERS]));
    }

    #[test]
    fn bump_simple_pot_increments_the_given_slot() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Alchemists", 0).unwrap();
        assert!(registry.bump_simple_pot(nr, 0, 1));
        assert!(registry.bump_simple_pot(nr, 0, 1));
        assert_eq!(registry.identity(nr).unwrap().economy.simple_pot[0][1], 2);
        assert_eq!(registry.identity(nr).unwrap().economy.simple_pot[0][0], 0);
    }

    #[test]
    fn bump_simple_pot_returns_false_for_nonexistent_clan() {
        let mut registry = ClanRegistry::new();
        assert!(!registry.bump_simple_pot(999, 0, 0));
    }

    #[test]
    fn money_change_log_message_matches_c_format() {
        assert_eq!(
            ClanMoneyChange::Deposited(150).log_message("Godmode"),
            "Godmode deposited 150G"
        );
        assert_eq!(
            ClanMoneyChange::Withdrew(30).log_message("Godmode"),
            "Godmode withdrew 30G"
        );
    }

    #[test]
    fn dirty_flag_is_not_persisted_across_serde_round_trip() {
        let mut registry = ClanRegistry::new();
        registry.found_clan("Dirty", 0).unwrap();
        assert!(registry.dirty());

        let json = serde_json::to_string(&registry).unwrap();
        let reloaded: ClanRegistry = serde_json::from_str(&json).unwrap();
        assert!(
            !reloaded.dirty(),
            "a freshly deserialized registry starts clean, matching what was just saved"
        );
        assert_eq!(reloaded.name(1), Some("Dirty"));
    }

    #[test]
    fn bonus_name_matches_c_table_and_out_of_range_guard() {
        assert_eq!(bonus_name(0), "Pentagram Quest");
        assert_eq!(bonus_name(1), "Military Advisor");
        assert_eq!(bonus_name(2), "Merchant");
        assert_eq!(bonus_name(3), "unassigned");
        assert_eq!(bonus_name(13), "unassigned");
        assert_eq!(bonus_name(-1), "Unknown");
        assert_eq!(bonus_name(14), "Unknown");
    }

    #[test]
    fn found_clan_initializes_economy_to_standard_defaults() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Traders", 1_000).unwrap();
        let economy = registry.identity(nr).unwrap().economy;
        assert_eq!(economy.bonus_level, [0; MAX_BONUS]);
        assert_eq!(economy.depot_money, 0);
        assert_eq!(economy.treasure.jewels, 0);
        assert_eq!(economy.treasure.cost_per_week, 0);
        assert_eq!(economy.treasure.debt, 0);
        // C: `c->treasure.payed_till = realtime;` (`clan.c:92`).
        assert_eq!(economy.treasure.payed_till, 1_000);
        assert_eq!(economy.training_score, 0);
    }

    #[test]
    fn clan_money_defaults_to_zero_for_unknown_clans() {
        let registry = ClanRegistry::new();
        assert_eq!(registry.clan_money(1), 0);
        assert_eq!(registry.clan_money(99), 0);
    }

    #[test]
    fn clan_money_change_applies_diff_and_gates_logging_by_threshold() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Bankers", 0).unwrap();

        // Small deposit (< 100): applied, but not logged.
        assert_eq!(registry.clan_money_change(nr, 50, true), None);
        assert_eq!(registry.clan_money(nr), 50);

        // Large deposit (>= 100): applied and logged.
        assert_eq!(
            registry.clan_money_change(nr, 150, true),
            Some(ClanMoneyChange::Deposited(150))
        );
        assert_eq!(registry.clan_money(nr), 200);

        // Any withdrawal is logged, regardless of size.
        assert_eq!(
            registry.clan_money_change(nr, -30, true),
            Some(ClanMoneyChange::Withdrew(30))
        );
        assert_eq!(registry.clan_money(nr), 170);

        // `log` false suppresses the log event even for a qualifying diff.
        assert_eq!(registry.clan_money_change(nr, -30, false), None);
        assert_eq!(registry.clan_money(nr), 140);
    }

    #[test]
    fn clan_money_change_on_unknown_clan_is_a_no_op() {
        let mut registry = ClanRegistry::new();
        assert_eq!(registry.clan_money_change(5, 500, true), None);
        assert_eq!(registry.clan_money(5), 0);
    }

    #[test]
    fn jewel_count_and_add_jewel() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Jewelers", 0).unwrap();
        assert_eq!(registry.jewel_count(nr), 0);
        registry.add_jewel(nr).unwrap();
        registry.add_jewel(nr).unwrap();
        assert_eq!(registry.jewel_count(nr), 2);
    }

    #[test]
    fn add_jewel_rejects_unknown_clan() {
        let mut registry = ClanRegistry::new();
        assert_eq!(registry.add_jewel(5), Err(ClanIdentityError::NotFound));
    }

    #[test]
    fn swap_jewels_charges_debt_without_removing_source_jewels() {
        let mut registry = ClanRegistry::new();
        let a = registry.found_clan("Raided", 0).unwrap();
        let b = registry.found_clan("Raider", 0).unwrap();
        for _ in 0..5 {
            registry.add_jewel(a).unwrap();
        }

        registry.swap_jewels(a, b, 2);

        // C: `swap_jewels` only adds debt to the source, it never
        // decrements the source's jewel count directly (`clan.c:501-513`).
        assert_eq!(registry.jewel_count(a), 5);
        assert_eq!(registry.jewel_count(b), 2);
        assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 2000);
    }

    #[test]
    fn swap_jewels_clamps_to_available_jewels_and_no_ops_when_empty() {
        let mut registry = ClanRegistry::new();
        let a = registry.found_clan("Poor", 0).unwrap();
        let b = registry.found_clan("Rich", 0).unwrap();

        // No jewels at all: no-op even though a debt-only change would be
        // "harmless" - matches C's `if (cnt_jewels(nr1) < 1) return;` early
        // exit before any state is touched.
        registry.swap_jewels(a, b, 10);
        assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 0);
        assert_eq!(registry.jewel_count(b), 0);

        registry.add_jewel(a).unwrap();
        registry.add_jewel(a).unwrap();
        // Requesting more than available (2) clamps down to 2.
        registry.swap_jewels(a, b, 10);
        assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 2000);
        assert_eq!(registry.jewel_count(b), 2);
    }

    #[test]
    fn bonus_level_get_and_set() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Bonused", 0).unwrap();
        assert_eq!(registry.bonus_level(nr, 2), 0);

        registry.set_bonus_level(nr, 2, 3).unwrap();
        assert_eq!(registry.bonus_level(nr, 2), 3);

        assert_eq!(
            registry.set_bonus_level(nr, MAX_BONUS, 1),
            Err(ClanIdentityError::NotFound)
        );
        assert_eq!(registry.bonus_level(nr, MAX_BONUS), 0);
        assert_eq!(
            registry.set_bonus_level(99, 0, 1),
            Err(ClanIdentityError::NotFound)
        );
    }

    #[test]
    fn update_treasure_charges_flat_clan_hall_rent_with_no_bonuses() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Rentpayers", 0).unwrap();
        registry.update_treasure(0);
        assert_eq!(
            registry
                .identity(nr)
                .unwrap()
                .economy
                .treasure
                .cost_per_week,
            CLAN_HALL_RENT * 1000
        );
    }

    #[test]
    fn update_treasure_reduces_unaffordable_bonus_to_zero() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Overspent", 0).unwrap();
        registry.set_bonus_level(nr, 0, 1).unwrap(); // no jewels to support it
        registry.update_treasure(0);
        assert_eq!(registry.bonus_level(nr, 0), 0);
    }

    #[test]
    fn update_treasure_keeps_affordable_bonus() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Sponsored", 0).unwrap();
        registry.set_bonus_level(nr, 0, 1).unwrap(); // costs 1000/250=4 <= 5 jewels
        for _ in 0..5 {
            registry.add_jewel(nr).unwrap();
        }
        registry.update_treasure(0);
        assert_eq!(registry.bonus_level(nr, 0), 1);
        assert_eq!(
            registry
                .identity(nr)
                .unwrap()
                .economy
                .treasure
                .cost_per_week,
            1000 + CLAN_HALL_RENT * 1000
        );
    }

    #[test]
    fn update_treasure_skips_debt_accrual_before_five_minutes_late() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("OnTime", 0).unwrap();
        registry.update_treasure(300); // exactly 300s: C requires `diff > 300`
        assert_eq!(registry.identity(nr).unwrap().economy.treasure.debt, 0);
    }

    #[test]
    fn update_treasure_accrues_small_debt_once_five_minutes_late() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Late", 0).unwrap();
        // cost = 5000 (rent only) => step = 604800/5000 = 120.
        // diff = 301 => n = 301/120 + 1 = 3.
        registry.update_treasure(301);
        let treasure = registry.identity(nr).unwrap().economy.treasure;
        assert_eq!(treasure.debt, 3);
        assert_eq!(treasure.payed_till, 120 * 3);
    }

    #[test]
    fn update_treasure_pays_off_debt_with_jewels_when_affordable() {
        let mut registry = ClanRegistry::new();
        let a = registry.found_clan("Payer", 0).unwrap();
        let b = registry.found_clan("Other", 0).unwrap();
        for _ in 0..5 {
            registry.add_jewel(a).unwrap();
        }
        registry.swap_jewels(a, b, 2); // a now owes 2000 debt, keeps 5 jewels

        let events = registry.update_treasure(0);
        assert_eq!(
            events,
            vec![ClanTreasuryEvent::PaidDebtWithJewels {
                clan: a,
                jewels_paid: 2,
            }]
        );
        assert_eq!(registry.jewel_count(a), 3);
        assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 0);
        assert!(registry.exists(a), "debt fully paid off, clan survives");
    }

    #[test]
    fn update_treasure_deletes_clan_that_goes_broke_with_no_jewels() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Broke", 0).unwrap();
        let serial_before = registry.serial(nr);

        // cost = 5000, step = 120; diff = 250_000 => n = 250000/120 + 1 = 2084,
        // which lands debt at 2084 (>= 2000) with zero jewels to pay it off.
        let events = registry.update_treasure(250_000);
        assert_eq!(
            events,
            vec![ClanTreasuryEvent::WentBroke {
                clan: nr,
                serial: serial_before,
                name: "Broke".to_string(),
            }]
        );
        assert!(!registry.exists(nr));
        assert!(registry.serial(nr) > serial_before);
    }

    #[test]
    fn update_treasure_pays_partial_debt_then_still_goes_broke() {
        let mut registry = ClanRegistry::new();
        let a = registry.found_clan("AlmostBroke", 0).unwrap();
        let b = registry.found_clan("Other", 0).unwrap();
        let serial_a_before = registry.serial(a);
        registry.add_jewel(a).unwrap(); // only 1 jewel available

        // Four raids, each clamped to the single available jewel, push
        // debt to 4000 while the jewel count itself never drops (`swap_
        // jewels` only ever adds debt to the source, `clan.c:501-513`).
        for _ in 0..4 {
            registry.swap_jewels(a, b, 3);
        }
        assert_eq!(registry.identity(a).unwrap().economy.treasure.debt, 4000);

        let events = registry.update_treasure(0);
        // n = debt/1000 = 4, clamped down to the 1 available jewel:
        // jewels -> 0, debt -= 1*1000 = 3000, which is still >= 2000.
        assert_eq!(
            events,
            vec![
                ClanTreasuryEvent::PaidDebtWithJewels {
                    clan: a,
                    jewels_paid: 1,
                },
                ClanTreasuryEvent::WentBroke {
                    clan: a,
                    serial: serial_a_before,
                    name: "AlmostBroke".to_string(),
                },
            ]
        );
        assert!(!registry.exists(a));
    }

    #[test]
    fn update_training_decays_score_by_five_percent_after_one_hour() {
        let mut registry = ClanRegistry::new();
        let nr = registry.found_clan("Trainers", 0).unwrap();
        registry.identity_mut(nr).unwrap().economy.training_score = 1000;

        registry.update_training(3599);
        assert_eq!(registry.identity(nr).unwrap().economy.training_score, 1000);

        registry.update_training(3600);
        assert_eq!(registry.identity(nr).unwrap().economy.training_score, 950);
        assert_eq!(
            registry.identity(nr).unwrap().economy.last_training_update,
            3600
        );
    }
}
