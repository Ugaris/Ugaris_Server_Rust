use super::*;

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

    pub(super) fn identity_mut(&mut self, nr: u16) -> Option<&mut ClanIdentity> {
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
        if log && !(0..100).contains(&diff) {
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

    /// C `/setclanjewels`'s direct assignment `clan[clan_nr].treasure.
    /// jewels = jewels` (`command.c:7563-7596`). Returns the previous
    /// jewel count so the caller can format "changed from %d to %d",
    /// or `None` if the clan number is out of range or does not exist.
    /// C's own array is preallocated for every in-range slot, so it would
    /// happily write through a nameless slot too (an admin footgun, not a
    /// feature); this registry has no such slot to write into, matching
    /// the same "cannot occur here" reasoning already documented on
    /// [`Self::clan_money_change`].
    pub fn set_jewels(&mut self, cnr: u16, jewels: i32) -> Option<i32> {
        let identity = self.identity_mut(cnr)?;
        let old_jewels = identity.economy.treasure.jewels;
        identity.economy.treasure.jewels = jewels;
        self.dirty = true;
        Some(old_jewels)
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

    /// C `dungeondoor`'s `first_solve` jewel-steal economy mutation
    /// (`area/13/dungeon.c:1855-1891`, applied via the `server_chat(1028,
    /// ...)` inter-process message and resolved in `clan.c:1343-1372`'s
    /// `'J'` case of `clan_dungeon_chat`). Unlike [`Self::swap_jewels`]
    /// (which only charges the loser debt and is used for the generic
    /// `clan_can_attack_*` combat path), this is the catacomb-specific
    /// formula: the defender (`cnr`) gets `training_score += 150` and
    /// `treasure.debt += cnt * 1000 + 1000` (note the extra flat `1000`,
    /// absent from `swap_jewels`'s plain `cnt * 1000`); the attacker
    /// (`onr`) gets `treasure.jewels += cnt` directly, uncapped (`cnt` was
    /// already derived from the defender's own jewel count by the
    /// caller, so it can never exceed what the defender could lose). A
    /// no-op for either clan number if it does not exist, or if `cnt <=
    /// 0` (mirrors C's own `if (cnt > 0)` guard around this whole
    /// mutation).
    pub fn dungeon_jewel_steal(&mut self, cnr: u16, onr: u16, cnt: i32) {
        if cnt <= 0 {
            return;
        }
        if let Some(identity) = self.identity_mut(cnr) {
            identity.economy.training_score += 150;
            identity.economy.treasure.debt += cnt * 1000 + 1000;
        }
        if let Some(identity) = self.identity_mut(onr) {
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

    /// C `clan_dungeon_chat`'s `'a'` case consumption side (`clan.c:1391-
    /// 1396`): `if (clan[cnr].dungeon.alc_pot[nr][str] > 0) { clan[cnr].
    /// dungeon.alc_pot[nr][str]--; }`. Same single-server direct-
    /// consumption simplification as [`Self::consume_simple_pot`] -
    /// `crate::world::dungeon_fighter`'s `dungeon_potion` port is the one
    /// caller, replacing C's `dungeon_potion` -> `server_chat(1028, ...)`
    /// -> master-server `clan_dungeon_chat` round trip.
    pub fn consume_alc_pot(&mut self, cnr: u16, kind: usize, tier: usize) -> bool {
        let Some(identity) = self.identity_mut(cnr) else {
            return false;
        };
        if identity.economy.alc_pot[kind][tier] == 0 {
            return false;
        }
        identity.economy.alc_pot[kind][tier] -= 1;
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

    /// C `clan_dungeon_chat`'s `'s'` case consumption side (`clan.c:1379-
    /// 1384`): `if (clan[cnr].dungeon.simple_pot[nr][str] > 0) {
    /// clan[cnr].dungeon.simple_pot[nr][str]--; }`. In C this runs on the
    /// master server in response to a `server_chat(1028, ...)` message a
    /// `dungeonfighter` NPC sent after drinking a stockpiled potion; this
    /// codebase has no master/slave server split (single area server per
    /// process), so `crate::world::dungeon_fighter`'s `dungeonfighter`
    /// port calls this directly and locally instead, matching
    /// [`Self::bump_simple_pot`]'s own increment-side precedent. Returns
    /// `false` (no-op, matching C's own `> 0` guard and the nonexistent-
    /// clan case) when there is nothing to consume.
    pub fn consume_simple_pot(&mut self, cnr: u16, kind: usize, size: usize) -> bool {
        let Some(identity) = self.identity_mut(cnr) else {
            return false;
        };
        if identity.economy.simple_pot[kind][size] == 0 {
            return false;
        }
        identity.economy.simple_pot[kind][size] -= 1;
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
