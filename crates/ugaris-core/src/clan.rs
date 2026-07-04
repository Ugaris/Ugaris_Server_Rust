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
//! `add_member`, `remove_member`, `clan.c:242-272,460-492,1186-1221`).
//!
//! NOT ported yet (left for follow-up slices): treasury/bonus/
//! dungeon-guard economy (`update_treasure`, `update_training`, `struct
//! clan_dungeon`), the `doraid` raid-toggle clamp inside
//! `update_relations` (dead in practice once a clan's first tick has run
//! - see the comment on [`ClanRelations::update`] - and meaningless
//! without the dungeon/raid system, so intentionally skipped),
//! clan-log persistence (`add_clanlog`/SQL `clanlog` table - this module
//! returns [`ClanRelationEvent`]s for a future caller to format and log),
//! `crates/ugaris-db/src/clan.rs` (no DB repository/migration yet, so
//! [`ClanRegistry`] is not yet persisted across restarts), the
//! `ClanAttackPolicy` wiring in `do_action.rs` (still `NoClanAttackPolicy`
//! everywhere), achievement awarding on membership change
//! (`ACHIEVEMENT_CLAN_MEMBER`/`ACHIEVEMENT_CLUB_MEMBER`, left to the
//! runtime caller per the pattern used elsewhere - see
//! [`ClanRegistry::add_member`]), chat channel gating, and clan-hall
//! transport access beyond direct membership.

use serde::{Deserialize, Serialize};

use crate::entity::Character;

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

/// C `#define CLUBOFFSET 1024` (`club.h:5`). Clan numbers `>= CLUB_OFFSET`
/// stored in `Character.clan` mean "club #`n - CLUB_OFFSET`", a separate
/// not-yet-ported system (`src/system/club.c`) that reuses the same
/// character fields. [`ClanRegistry`] treats such values as out of its
/// jurisdiction rather than as invalid.
pub const CLUB_OFFSET: u16 = 1024;

/// C `struct clan`'s identity fields (`clan.h:88-101`, minus the treasury/
/// bonus/dungeon-guard economy - see the module doc comment). Owned by
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
}

impl ClanIdentity {
    /// C `clan_standards` (`clan.c:76-95`), identity portion only (the
    /// relation-reset portion is [`ClanRelations::found_clan`]).
    fn standard(name: String) -> Self {
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
}

impl Default for ClanRegistry {
    fn default() -> Self {
        ClanRegistry {
            relations: ClanRelations::default(),
            serials: [0; MAX_CLAN],
            identities: std::array::from_fn(|_| None),
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

    pub fn relations_mut(&mut self) -> &mut ClanRelations {
        &mut self.relations
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
        self.identities[slot] = Some(ClanIdentity::standard(name.to_string()));
        self.relations.found_clan(slot as u16, now);
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
        Ok(())
    }

    /// C `set_clan_message` (`clan.c:595-604`). See
    /// [`ClanRegistry::set_website`] for why the trailing-character-strip
    /// quirk is deferred rather than ported here.
    pub fn set_message(&mut self, cnr: u16, message: &str) -> Result<(), ClanIdentityError> {
        let identity = self.identity_mut(cnr).ok_or(ClanIdentityError::NotFound)?;
        identity.message = message.chars().take(79).collect();
        Ok(())
    }
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
}
