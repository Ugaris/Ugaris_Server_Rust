use super::*;

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
    /// Immunity+N" potions, indexed `0..6` for the `+4..=+24` tiers. Fed
    /// by [`ClanRegistry::add_alc_potion`] (the `NT_GIVE` `IDR_FLASK`
    /// branch, `crate::world::clanclerk`), ported in iteration 135 - a
    /// freshly-founded clan still reads all zero here, same as C.
    /// `#[serde(default)]` keeps this backward compatible with any
    /// snapshot saved before this field existed.
    #[serde(default)]
    pub alc_pot: [[u16; 6]; 2],
    /// C `struct clan_dungeon`'s `simple_pot[3][3]` (`clan.h:75`): the
    /// clan's simple-potion stockpile, `[0]` = healing, `[1]` = mana,
    /// `[2]` = combo, indexed `0..3` for Small/Medium/Big. Fed by
    /// [`ClanRegistry::bump_simple_pot`] (the `add potions` text
    /// command, `crate::world::clanclerk`), ported in iteration 135 -
    /// see [`ClanEconomy::alc_pot`].
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
    pub(super) fn reduce_highest_bonus(&mut self) {
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
    pub(super) fn bonus_upkeep_cost(&self) -> i32 {
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
    pub(super) fn standard(name: String, now: i64) -> Self {
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
