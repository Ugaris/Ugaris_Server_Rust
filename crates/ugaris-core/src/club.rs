//! Club system (`src/system/club.c` + `club.h`) - a parallel, much
//! larger-scale variant of the clan system that reuses the same
//! `Character.clan`/`.clan_rank`/`.clan_serial` fields, distinguished by
//! `clan >= CLUBOFFSET` (`crate::clan::CLUB_OFFSET`, `club.h:5`). See the
//! "Clan system" P3 task in `PORTING_TODO.md` for the surrounding clan
//! task this is a dependency of.
//!
//! Ported here: the identity/serial registry - [`ClubRegistry::exists`]/
//! `create_club`/`rename_club`/`kill_club` (`club.c:132-212`) - and the
//! validating membership lookup [`ClubRegistry::get_char_club`]
//! (`get_char_club`, `club.c:29-61`), plus the weekly billing tick
//! [`ClubRegistry::tick_billing`] (`tick_club`'s `areaID == 3` branch,
//! `club.c:82-111`).
//!
//! Not yet ported (documented, not silently dropped):
//! - No persistence: `crates/ugaris-db` has no club repository/migration
//!   yet, so - like `crate::clan::ClanRegistry` before its own DB wiring
//!   landed - this registry only lives in server memory. [`ClubRegistry::
//!   dirty`]/`clear_dirty` exist now so a future DB-backed save task has
//!   the same "only write when changed" seam `ClanRegistry` already uses.
//! - `tick_club`'s `areaID != 3` branch (`schedule_clubs`/`db_read_clubs`,
//!   a periodic full re-sync from the database used by every *other*
//!   area process in the legacy multi-process architecture, `club.c:86-
//!   89`, `database_clubs.c:51-82,116-118`) has no equivalent: with no DB
//!   wiring yet there is nothing to resync from.
//! - `show_club_info`/`showclub` (the `look_char`/`/club` text
//!   formatters, `club.c:65-130`) and the `CDR_CLUBMASTER` founding/
//!   joining NPC driver (`clubmaster.c`, driver 113) are not ported: no
//!   club is ever actually founded or joined by a real player yet. Today
//!   this registry is reachable only from the not-yet-wired `/joinclub`/
//!   `/killclub`/`/renclub` GM cheats (`command.c:6445-6467,6484-6497,
//!   4548-4585`) and from `crate::world::clanmaster::is_club_member`'s
//!   membership gate, which now calls [`ClubRegistry::get_char_club`] for
//!   real validation instead of the bare `clan >= CLUB_OFFSET` range
//!   check it used as an approximation before this module existed.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::clan::CLUB_OFFSET;
use crate::entity::Character;

/// C `#define MAXCLUB 16384` (`club.h:4`). Club numbers are `1..MAX_CLUB`
/// - slot `0` is part of C's static array but never assigned by
/// `create_club`, whose free-slot scan starts at `n = 1` (`club.c:161`).
pub const MAX_CLUB: usize = 16384;

/// C `struct club`'s mutable fields (`club.h:7-12`; `serial` is tracked
/// separately by [`ClubRegistry`] so it survives [`ClubRegistry::
/// tick_billing`]'s deletion, matching C's always-allocated static array
/// leaving `club[n].serial` untouched when only `name[0]` is cleared).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubIdentity {
    /// C `char name[80]`. Presence of a [`ClubIdentity`] at all *is* the
    /// Rust equivalent of `club[n].name[0]` truthiness, so this is never
    /// empty while the identity exists (mirrors `crate::clan::
    /// ClanIdentity::name`'s own invariant).
    pub name: String,
    /// C `int paid` - `realtime` seconds the next weekly rent is due.
    pub paid: i64,
    /// C `int money`, in 1/100 gold (matches `showclub`'s `/ 100`
    /// formatting, `club.c:127-128`).
    pub money: i32,
}

/// Errors for [`ClubRegistry::create_club`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClubCreateError {
    /// C: a character in `name` is neither `' '` nor `isalpha` (`club.c:
    /// 143-147`).
    InvalidName,
    /// C: `strlen(name) > 75` (`club.c:148-150`).
    NameTooLong,
    /// C: an existing club already has this exact name (`club.c:152-159`).
    /// An empty `name` always hits this in practice, since C's duplicate
    /// scan compares against every slot's `name[0]` - including unused
    /// slots, whose name is the empty string - and MAXCLUB (16384) always
    /// has at least one unused slot long before the list is genuinely
    /// full.
    NameTaken,
    /// C: every slot `1..MAXCLUB` already has a name (`club.c:161-168`).
    ClubListFull,
}

/// Errors for [`ClubRegistry::rename_club`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClubRenameError {
    /// C: a character in `name` is neither `' '` nor `isalpha` (`club.c:
    /// 188-192`).
    InvalidName,
    /// C: `strlen(name) > 75` (`club.c:193-195`).
    NameTooLong,
    /// C: an existing club already has this exact name (`club.c:197-204`).
    /// See [`ClubCreateError::NameTaken`] for the empty-name case.
    NameTaken,
    /// C: `nr < 1 || nr >= MAXCLUB` (`club.c:205-207`).
    OutOfRange,
}

/// Outcome of one [`ClubRegistry::tick_billing`] call, for the caller to
/// log (mirrors C's own `xlog` calls, `club.c:100,105`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClubBillingEvent {
    /// C: `club[n].money -= 10000*100; club[n].paid += 60*60*24*7;`
    /// (`club.c:103-105`).
    Paid {
        club: u16,
        name: String,
        money: i32,
        paid: i64,
    },
    /// C: `club[n].name[0] = 0;` (`club.c:100-101`) - `money`/`paid`/the
    /// serial are left untouched, matching C exactly.
    Deleted { club: u16, name: String },
}

/// C's weekly club rent, `10000 * 100` in 1/100 gold (`club.c:99,103`).
const WEEKLY_RENT: i32 = 10_000 * 100;
/// C's weekly billing period, `60 * 60 * 24 * 7` seconds (`club.c:104,
/// 172`).
const WEEK_SECONDS: i64 = 60 * 60 * 24 * 7;

/// Club identity + serial registry. See the module doc comment for what
/// is and is not ported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubRegistry {
    /// C `club[n].serial`, tracked separately from [`Self::identities`]
    /// so a slot's serial survives [`Self::tick_billing`]'s deletion
    /// (name-only clear) and keeps rising across re-creations
    /// ([`Self::create_club`] always increments, never resets).
    serials: Vec<u32>,
    /// Sparse identity storage: only clubs that currently exist have an
    /// entry. `MAXCLUB` (16384) is too large to model as C's fixed
    /// `[Option<ClubIdentity>; MAX_CLUB]` array the way `crate::clan::
    /// ClanRegistry` does for its own much smaller `MAX_CLAN` (32);
    /// membership presence here is exactly C's `club[n].name[0]`
    /// truthiness check.
    identities: HashMap<u16, ClubIdentity>,
    /// Same "needs a real save" flag as `crate::clan::ClanRegistry::
    /// dirty` - not yet consumed by any save task since no DB repository
    /// exists (see the module doc comment).
    #[serde(skip)]
    dirty: bool,
}

impl Default for ClubRegistry {
    fn default() -> Self {
        ClubRegistry {
            serials: vec![0; MAX_CLUB],
            identities: HashMap::new(),
            dirty: false,
        }
    }
}

impl ClubRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn valid_club(nr: u16) -> bool {
        nr >= 1 && (nr as usize) < MAX_CLUB
    }

    /// C: `club[nr].name[0]` truthiness.
    pub fn exists(&self, nr: u16) -> bool {
        Self::valid_club(nr) && self.identities.contains_key(&nr)
    }

    /// C `club[nr].serial`.
    pub fn serial(&self, nr: u16) -> u32 {
        if Self::valid_club(nr) {
            self.serials[nr as usize]
        } else {
            0
        }
    }

    pub fn identity(&self, nr: u16) -> Option<&ClubIdentity> {
        if Self::valid_club(nr) {
            self.identities.get(&nr)
        } else {
            None
        }
    }

    fn identity_mut(&mut self, nr: u16) -> Option<&mut ClubIdentity> {
        if Self::valid_club(nr) {
            self.identities.get_mut(&nr)
        } else {
            None
        }
    }

    /// C `club[nr].money` (in 1/100 gold, see [`ClubIdentity::money`]'s
    /// doc comment). `0` for an out-of-range/nonexistent club - C has no
    /// dedicated `get_club_money` accessor (`show_club_info`/
    /// `clubmaster.c` both read `club[n].money` directly), this mirrors
    /// [`crate::clan::ClanRegistry::clan_money`]'s shape for symmetry.
    pub fn club_money(&self, nr: u16) -> i32 {
        self.identity(nr).map(|id| id.money).unwrap_or(0)
    }

    /// Applies `diff` (positive = deposit, negative = withdrawal) to a
    /// club's treasury (`clubmaster.c:492,511`: `club[n].money +=
    /// val;`/`club[n].money -= val;`). Returns `false` for an out-of-
    /// range/nonexistent club (a no-op, matching every other mutator in
    /// this registry); the caller is expected to have already validated
    /// the amount (positive, affordable) exactly like `clubmaster_driver`
    /// does before calling into either branch.
    pub fn club_money_change(&mut self, nr: u16, diff: i32) -> bool {
        let Some(identity) = self.identity_mut(nr) else {
            return false;
        };
        identity.money += diff;
        self.dirty = true;
        true
    }

    pub fn name(&self, nr: u16) -> Option<&str> {
        self.identity(nr).map(|id| id.name.as_str())
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// C's shared `isalpha`/space character-class scan used by both
    /// `create_club` and `rename_club` (`club.c:143-147,188-192`).
    fn valid_name_chars(name: &str) -> bool {
        name.chars().all(|c| c == ' ' || c.is_ascii_alphabetic())
    }

    fn name_taken(&self, name: &str) -> bool {
        name.is_empty() || self.identities.values().any(|id| id.name == name)
    }

    /// C `create_club` (`club.c:140-183`): validates the name, rejects
    /// duplicates, allocates the first free slot (`1..MAXCLUB`), and
    /// always increments that slot's serial (whether freshly allocated or
    /// reused from a previously deleted club) - unlike `crate::clan::
    /// ClanRegistry::found_clan`, which preserves the old serial across
    /// re-creation.
    pub fn create_club(&mut self, name: &str, now: i64) -> Result<u16, ClubCreateError> {
        if !Self::valid_name_chars(name) {
            return Err(ClubCreateError::InvalidName);
        }
        if name.len() > 75 {
            return Err(ClubCreateError::NameTooLong);
        }
        if self.name_taken(name) {
            return Err(ClubCreateError::NameTaken);
        }
        let slot = (1..MAX_CLUB as u16).find(|n| !self.identities.contains_key(n));
        let Some(slot) = slot else {
            return Err(ClubCreateError::ClubListFull);
        };
        self.serials[slot as usize] = self.serials[slot as usize].wrapping_add(1);
        self.identities.insert(
            slot,
            ClubIdentity {
                name: name.to_string(),
                paid: now + WEEK_SECONDS,
                money: 0,
            },
        );
        self.dirty = true;
        Ok(slot)
    }

    /// C `rename_club` (`club.c:185-212`), the `/renclub` admin tool.
    /// Unlike [`Self::create_club`], C writes directly into the
    /// always-allocated static array without checking whether `nr`
    /// already has an identity, so it can "create" a club at an explicit
    /// slot with whatever serial that slot already has (`0` if never
    /// used) rather than bumping it - an admin-tool quirk kept for
    /// fidelity, same precedent as `crate::clan::ClanRegistry::
    /// set_name`'s doc comment. `paid`/`money` are left at their prior
    /// value (`0`/never-due) when renaming an already-existing club,
    /// matching C leaving those fields untouched.
    pub fn rename_club(&mut self, nr: u16, name: &str) -> Result<(), ClubRenameError> {
        if !Self::valid_name_chars(name) {
            return Err(ClubRenameError::InvalidName);
        }
        if name.len() > 75 {
            return Err(ClubRenameError::NameTooLong);
        }
        if self.name_taken(name) {
            return Err(ClubRenameError::NameTaken);
        }
        if !Self::valid_club(nr) {
            return Err(ClubRenameError::OutOfRange);
        }
        match self.identities.get_mut(&nr) {
            Some(identity) => identity.name = name.to_string(),
            None => {
                self.identities.insert(
                    nr,
                    ClubIdentity {
                        name: name.to_string(),
                        paid: 0,
                        money: 0,
                    },
                );
            }
        }
        self.dirty = true;
        Ok(())
    }

    /// C `kill_club` (`club.c:132-138`): forces a club bankrupt so the
    /// next [`Self::tick_billing`] deletes it. A no-op for an out-of-range
    /// or nonexistent `nr`, matching C's own no-op-on-nameless-slot
    /// outcome (writing `paid`/`money` on a slot with `name[0] == 0` has
    /// no observable effect there either, since existence is defined by
    /// the name).
    pub fn kill_club(&mut self, nr: u16) {
        if let Some(identity) = self.identities.get_mut(&nr) {
            identity.paid = 1;
            identity.money = 0;
            self.dirty = true;
        }
    }

    /// C `get_char_club` (`club.c:29-61`): validates a character's club
    /// membership fields against the live registry, clearing them (and
    /// returning `None`) on any mismatch - the same self-healing idiom
    /// `crate::clan::ClanRegistry::get_char_clan` uses for clans. Unlike
    /// C, this always fully validates: C skips validation while
    /// `!club_update_done` (storage still loading at server boot,
    /// `club.c:46-48`), which has no equivalent in this always-ready
    /// in-memory registry (the same deviation `get_char_clan` documents).
    pub fn get_char_club(&self, character: &mut Character) -> Option<u16> {
        let cnr = character.clan;
        if cnr == 0 || cnr < CLUB_OFFSET {
            return None;
        }
        let club_nr = cnr - CLUB_OFFSET;
        if !Self::valid_club(club_nr)
            || character.clan_serial != self.serial(club_nr)
            || !self.exists(club_nr)
        {
            character.clan = 0;
            character.clan_rank = 0;
            character.clan_serial = 0;
            return None;
        }
        Some(club_nr)
    }

    /// C `tick_club`'s `areaID == 3` billing branch (`club.c:90-111`):
    /// processes at most one due club per call - a flat weekly rent, or
    /// deletion if the club can't afford it (name cleared; `money`/
    /// `paid`/serial left untouched, matching C exactly). Returns `None`
    /// if no due club exists (or `area_id != 3` - see the module doc
    /// comment for why the "other areas" branch has no port).
    pub fn tick_billing(&mut self, area_id: u16, now: i64) -> Option<ClubBillingEvent> {
        if area_id != 3 {
            return None;
        }
        for n in 0..MAX_CLUB as u16 {
            let Some(identity) = self.identities.get_mut(&n) else {
                continue;
            };
            if identity.paid > now {
                continue;
            }
            self.dirty = true;
            if identity.money < WEEKLY_RENT {
                let name = identity.name.clone();
                self.identities.remove(&n);
                return Some(ClubBillingEvent::Deleted { club: n, name });
            }
            identity.money -= WEEKLY_RENT;
            identity.paid += WEEK_SECONDS;
            return Some(ClubBillingEvent::Paid {
                club: n,
                name: identity.name.clone(),
                money: identity.money,
                paid: identity.paid,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(clan: u16, clan_rank: u8, clan_serial: u32) -> Character {
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
            clan,
            clan_rank,
            clan_serial,
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
            dungeonfighter: None,
            fight_driver: None,
            lq_usurp: None,
        }
    }

    #[test]
    fn create_club_allocates_first_free_slot_and_bumps_serial() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 1_000).unwrap();
        assert_eq!(nr, 1);
        assert_eq!(registry.name(1), Some("Rangers"));
        assert_eq!(registry.serial(1), 1);
        let identity = registry.identity(1).unwrap();
        assert_eq!(identity.money, 0);
        assert_eq!(identity.paid, 1_000 + WEEK_SECONDS);
    }

    #[test]
    fn create_club_rejects_digits_and_symbols() {
        let mut registry = ClubRegistry::new();
        assert_eq!(
            registry.create_club("Rangers123", 0),
            Err(ClubCreateError::InvalidName)
        );
    }

    #[test]
    fn create_club_rejects_overlong_name() {
        let mut registry = ClubRegistry::new();
        let name = "a".repeat(76);
        assert_eq!(
            registry.create_club(&name, 0),
            Err(ClubCreateError::NameTooLong)
        );
    }

    #[test]
    fn create_club_rejects_duplicate_name() {
        let mut registry = ClubRegistry::new();
        registry.create_club("Rangers", 0).unwrap();
        assert_eq!(
            registry.create_club("Rangers", 0),
            Err(ClubCreateError::NameTaken)
        );
    }

    #[test]
    fn create_club_rejects_empty_name() {
        let mut registry = ClubRegistry::new();
        assert_eq!(registry.create_club("", 0), Err(ClubCreateError::NameTaken));
    }

    #[test]
    fn create_club_reuses_deleted_slot_and_increments_serial_again() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 0).unwrap();
        assert_eq!(registry.serial(nr), 1);
        registry.kill_club(nr);
        let event = registry.tick_billing(3, WEEK_SECONDS).unwrap();
        assert_eq!(
            event,
            ClubBillingEvent::Deleted {
                club: nr,
                name: "Rangers".to_string()
            }
        );
        assert!(!registry.exists(nr));
        // Serial is untouched by deletion.
        assert_eq!(registry.serial(nr), 1);

        let nr2 = registry.create_club("Rovers", 0).unwrap();
        assert_eq!(nr2, nr, "the freed slot is reused");
        assert_eq!(registry.serial(nr2), 2, "serial keeps rising on reuse");
    }

    #[test]
    fn rename_club_renames_existing_club() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 0).unwrap();
        registry.rename_club(nr, "Rovers").unwrap();
        assert_eq!(registry.name(nr), Some("Rovers"));
        assert_eq!(registry.serial(nr), 1, "rename does not bump the serial");
    }

    #[test]
    fn rename_club_can_create_at_an_explicit_never_used_slot() {
        let mut registry = ClubRegistry::new();
        registry.rename_club(500, "Ghosts").unwrap();
        assert!(registry.exists(500));
        assert_eq!(registry.serial(500), 0, "never bumped, unlike create_club");
    }

    #[test]
    fn rename_club_rejects_out_of_range() {
        let mut registry = ClubRegistry::new();
        assert_eq!(
            registry.rename_club(0, "Rovers"),
            Err(ClubRenameError::OutOfRange)
        );
        assert_eq!(
            registry.rename_club(MAX_CLUB as u16, "Rovers"),
            Err(ClubRenameError::OutOfRange)
        );
    }

    #[test]
    fn rename_club_rejects_duplicate_name() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 0).unwrap();
        registry.create_club("Rovers", 0).unwrap();
        assert_eq!(
            registry.rename_club(nr, "Rovers"),
            Err(ClubRenameError::NameTaken)
        );
    }

    #[test]
    fn kill_club_is_noop_for_nonexistent_slot() {
        let mut registry = ClubRegistry::new();
        registry.kill_club(5);
        assert!(!registry.dirty());
        assert!(!registry.exists(5));
    }

    #[test]
    fn get_char_club_validates_matching_serial() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 0).unwrap();
        let serial = registry.serial(nr);
        let mut character = member(CLUB_OFFSET + nr, 2, serial);
        assert_eq!(registry.get_char_club(&mut character), Some(nr));
        // Fields untouched on success.
        assert_eq!(character.clan, CLUB_OFFSET + nr);
        assert_eq!(character.clan_rank, 2);
    }

    #[test]
    fn get_char_club_clears_stale_serial() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 0).unwrap();
        let mut character = member(CLUB_OFFSET + nr, 2, registry.serial(nr) + 1);
        assert_eq!(registry.get_char_club(&mut character), None);
        assert_eq!(character.clan, 0);
        assert_eq!(character.clan_rank, 0);
        assert_eq!(character.clan_serial, 0);
    }

    #[test]
    fn get_char_club_clears_deleted_club() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 0).unwrap();
        let serial = registry.serial(nr);
        registry.kill_club(nr);
        registry.tick_billing(3, WEEK_SECONDS).unwrap();
        let mut character = member(CLUB_OFFSET + nr, 1, serial);
        assert_eq!(registry.get_char_club(&mut character), None);
        assert_eq!(character.clan, 0);
    }

    #[test]
    fn get_char_club_ignores_clan_numbers() {
        let registry = ClubRegistry::new();
        let mut character = member(5, 0, 0);
        assert_eq!(registry.get_char_club(&mut character), None);
        // Not cleared: this is a clan reference, not a club one.
        assert_eq!(character.clan, 5);
    }

    #[test]
    fn get_char_club_no_clan_returns_none_without_clearing() {
        let registry = ClubRegistry::new();
        let mut character = member(0, 0, 0);
        assert_eq!(registry.get_char_club(&mut character), None);
        assert_eq!(character.clan, 0);
    }

    #[test]
    fn tick_billing_ignores_other_areas() {
        let mut registry = ClubRegistry::new();
        registry.create_club("Rangers", 0).unwrap();
        assert_eq!(registry.tick_billing(5, 1_000_000_000), None);
    }

    #[test]
    fn tick_billing_pays_when_affordable() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 0).unwrap();
        registry.identities.get_mut(&nr).unwrap().money = WEEKLY_RENT + 500;
        let event = registry.tick_billing(3, WEEK_SECONDS).unwrap();
        assert_eq!(
            event,
            ClubBillingEvent::Paid {
                club: nr,
                name: "Rangers".to_string(),
                money: 500,
                paid: WEEK_SECONDS * 2,
            }
        );
        assert!(registry.exists(nr));
    }

    #[test]
    fn tick_billing_deletes_when_unaffordable() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 0).unwrap();
        let event = registry.tick_billing(3, WEEK_SECONDS).unwrap();
        assert_eq!(
            event,
            ClubBillingEvent::Deleted {
                club: nr,
                name: "Rangers".to_string()
            }
        );
        assert!(!registry.exists(nr));
    }

    #[test]
    fn tick_billing_skips_clubs_not_yet_due() {
        let mut registry = ClubRegistry::new();
        registry.create_club("Rangers", 0).unwrap();
        assert_eq!(registry.tick_billing(3, 0), None);
    }

    #[test]
    fn club_money_reads_zero_for_out_of_range_or_nonexistent_club() {
        let registry = ClubRegistry::new();
        assert_eq!(registry.club_money(0), 0);
        assert_eq!(registry.club_money(5), 0);
    }

    #[test]
    fn club_money_change_deposits_and_withdraws() {
        let mut registry = ClubRegistry::new();
        let nr = registry.create_club("Rangers", 0).unwrap();
        assert_eq!(registry.club_money(nr), 0);
        assert!(registry.club_money_change(nr, 1_000_000));
        assert_eq!(registry.club_money(nr), 1_000_000);
        assert!(registry.club_money_change(nr, -400_000));
        assert_eq!(registry.club_money(nr), 600_000);
    }

    #[test]
    fn club_money_change_is_noop_for_nonexistent_club() {
        let mut registry = ClubRegistry::new();
        assert!(!registry.club_money_change(5, 100));
        assert_eq!(registry.club_money(5), 0);
    }

    #[test]
    fn tick_billing_processes_only_one_club_per_call() {
        let mut registry = ClubRegistry::new();
        let a = registry.create_club("Rangers", 0).unwrap();
        let b = registry.create_club("Rovers", 0).unwrap();
        for nr in [a, b] {
            registry.identities.get_mut(&nr).unwrap().money = WEEKLY_RENT + 100;
        }
        let first = registry.tick_billing(3, WEEK_SECONDS).unwrap();
        // Only the lowest-numbered due club is handled this call.
        assert!(matches!(first, ClubBillingEvent::Paid { club, .. } if club == a));
        // The other club is still due and gets handled on the next call.
        let second = registry.tick_billing(3, WEEK_SECONDS).unwrap();
        assert!(matches!(second, ClubBillingEvent::Paid { club, .. } if club == b));
    }
}
