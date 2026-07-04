//! Clan relation state machine ported from `src/system/clan.c` in the
//! legacy C server (`clan.h` for constants/data model).
//!
//! This is a first, self-contained slice of the much larger clan system
//! (see the "Clan system" P3 entry in `PORTING_TODO.md`). What is ported
//! here: the `CS_*` relation levels (`clan.h:41-56`), the per-pair
//! `current_relation`/`want_relation`/`want_date` state (`struct
//! clan_status`, `clan.h:58-64`), clan founding's relation reset
//! (`found_clan`/`clan_standards`/`zero_relation`, `clan.c:76-95,450-492`),
//! the unilateral relation-request setter (`set_clan_relation`,
//! `clan.c:839-860`), the read-only policy queries (`may_enter_clan`,
//! `clan_can_attack_outside`, `clan_can_attack_inside`, `clan_alliance`,
//! `clan.c:881-934`), and the daily escalation/de-escalation tick state
//! machine (`update_relations`, `clan.c:936-1089`).
//!
//! NOT ported yet (left for follow-up slices): clan identity beyond a bare
//! "does this clan number exist" flag (no name/rank-name/website/message
//! storage), membership (`ch[cn].clan`/`.clan_rank`/`.clan_serial` already
//! exist as plain `Character` fields per `PORTING_LEDGER.md`, but
//! `found_clan`/`add_member`/`remove_member`/`get_char_clan` are not
//! wired), treasury/bonus/dungeon-guard economy (`update_treasure`,
//! `update_training`, `struct clan_dungeon`), the `doraid` raid-toggle
//! clamp inside `update_relations` (dead in practice once a clan's first
//! tick has run - see the comment on [`ClanRelations::update`] - and
//! meaningless without the dungeon/raid system, so intentionally skipped),
//! clan-log persistence (`add_clanlog`/SQL `clanlog` table - this module
//! returns [`ClanRelationEvent`]s for a future caller to format and log),
//! `crates/ugaris-db/src/clan.rs` (no DB repository yet), the
//! `ClanAttackPolicy` wiring in `do_action.rs` (still `NoClanAttackPolicy`
//! everywhere), chat channel gating, and clan-hall transport access beyond
//! direct membership.

use serde::{Deserialize, Serialize};

/// C `#define MAXCLAN 32` (`clan.h:19`). Clan numbers are `1..MAX_CLAN`;
/// `0` means "no clan" and numbers `>= 1024` (`CLUBOFFSET`) mean a club,
/// which is a separate, not-yet-ported system that reuses the same
/// character fields (see `src/system/club.c`).
pub const MAX_CLAN: usize = 32;

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
}
