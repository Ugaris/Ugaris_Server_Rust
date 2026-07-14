use super::*;

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
    pub(super) want_date: [[i64; MAX_CLAN]; MAX_CLAN],
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
    // C itself spells the Neutral->War and Feud->end transitions as three
    // separate `if` branches with identical bodies (both-want / clan-1
    // -wants-after-delay / clan-2-wants-after-delay, `clan.c:1022-1043`
    // and `clan.c:1061-1082`); kept verbatim for oracle parity.
    #[allow(clippy::if_same_then_else)]
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
