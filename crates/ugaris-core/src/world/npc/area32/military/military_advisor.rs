//! `CDR_MILITARY_ADVISOR` (`military_advisor_driver`, `military.c:
//! 2606-2803`): the paid "favor"/specific-mission-recommendation sales
//! economy, its [`MilitaryAdvisorStorage`] sales counters, and the
//! advisor-recommendation NT_CHAR flow. Split out of the former single
//! `military.rs` for size - see `super` (`military/mod.rs`) for the
//! full porting history.

use super::*;

/// C `calculate_advisor_index(storage_id)` (`military.c:2231-2244`):
/// maps an Advisor NPC's `storage_ID` (a compact-but-non-contiguous
/// numbering scheme - IDs below 27 count from 7, IDs 27 and above skip a
/// 4-wide gap and count from 31) to a `0..MAXADVISOR` (20) slot index
/// into `military_ppd::advisor_last[]`. Out-of-range results (either
/// branch going negative or `>= MAXADVISOR`) fall back to slot `0`,
/// matching C's own `if (idx < 0 || idx >= MAXADVISOR) idx = 0;` exactly.
pub fn calculate_advisor_index(storage_id: i32) -> usize {
    let idx = if storage_id < 27 {
        storage_id - 7
    } else {
        storage_id - 31 + 3
    };
    if !(0..MILITARY_PPD_MAXADVISOR_I32).contains(&idx) {
        0
    } else {
        idx as usize
    }
}

/// `MAXADVISOR` (`military.h:17`) as `i32`, for [`calculate_advisor_index`]'s
/// range check (the accessor-side constant, [`crate::player::
/// MILITARY_PPD_MAXADVISOR`], is a private-module `usize`).
pub(crate) const MILITARY_PPD_MAXADVISOR_I32: i32 = 20;

/// C `advisor_price(level)` (`military.c:2288-2299`): the base gold price
/// (100 = 1G) an Advisor NPC's "favor" costs before the size multiplier
/// ([`offer_favor_cost`]) is applied, banded by player level.
pub fn advisor_price(level: i32) -> i32 {
    if level < 25 {
        400
    } else if level < 45 {
        800
    } else if level < 65 {
        1200
    } else if level < 85 {
        1500
    } else {
        2000
    }
}

/// C `offer_favor`'s cost calculation (`military.c:2318-2372`): the 5
/// favor sizes (small/medium/big/huge/vast, `favor_size` `0..=4`) each
/// apply a multiplier to [`advisor_price`]'s level-banded base price.
/// Returns `None` for an invalid `favor_size` (C's own `default: return
/// 0;` bail-out).
pub fn offer_favor_cost(level: i32, favor_size: i32) -> Option<i32> {
    let multiplier = match favor_size {
        0 => 1.0,
        1 => 3.0,
        2 => 10.0,
        3 => 20.0,
        4 => 35.0,
        _ => return None,
    };
    Some((f64::from(advisor_price(level)) * multiplier) as i32)
}

/// C `specific_mission_price(level, difficulty, mission_type)`
/// (`military.c:392-467`): the gold price an Advisor NPC quotes for a
/// specific paid mission recommendation.
pub fn specific_mission_price(level: i32, difficulty: i32, mission_type: i32) -> i32 {
    let base_price = (level * level) / 10 + level * 5;

    let difficulty_multiplier: f64 = match difficulty {
        0 => 0.4,
        1 => 0.8,
        2 => 1.0,
        3 => 1.5,
        4 => 1.8,
        _ => 1.0,
    };

    let type_multiplier: f64 = match mission_type {
        1 => 1.0,
        2 => 1.1,
        3 => 1.2,
        _ => 1.0,
    };

    let mut level_scaling = (100.0 / f64::from(level)).min(1.0);
    level_scaling = level_scaling.max(0.5);

    let price = (f64::from(base_price)
        * difficulty_multiplier
        * type_multiplier
        * (1.0 - (1.0 - level_scaling) * 0.5)) as i32;

    let min_price = match difficulty {
        0 => 200,
        1 => 400,
        2 => 800,
        3 => 1500,
        4 => 3000,
        _ => 200,
    };

    price.max(min_price)
}

/// C `offer_favor`'s favor-size name table (`military.c:2373-2378`
/// switch, used both in the offer text and [`ProcessFavorPaymentOutcome`]
/// rendering).
pub fn favor_size_name(favor_size: i32) -> &'static str {
    match favor_size {
        0 => "small",
        1 => "medium",
        2 => "big",
        3 => "huge",
        _ => "vast",
    }
}

/// C `handle_advisor_message`'s admin-only "info" qa code's own
/// `static char *fav_name[5]` (`military.c:2532`). Deliberately **not**
/// the same strings as [`favor_size_name`] - C itself uses "normal" here
/// for index 1 where `offer_favor`'s own switch says "medium", a genuine
/// (if seemingly accidental) inconsistency between the two tables,
/// reproduced verbatim rather than "fixed" to match.
pub const ADVISOR_INFO_FAVOR_NAMES: [&str; 5] = ["small", "normal", "big", "huge", "vast"];

/// C `handle_specific_mission_request`/`process_favor_payment`'s
/// mission-type name table (`military.c:521-533`, `2429-2440`).
pub fn mission_type_name(mission_type: i32) -> &'static str {
    match mission_type {
        1 => "demon-slaying",
        2 => "ratling-hunting",
        3 => "silver-mining",
        _ => "unknown",
    }
}

/// C `adv_introduction` (`military.c:2262-2281`): the Advisor NPC's
/// initial greeting, varying by `dat->storage_ID % 4`. Every branch wraps
/// "favor" in `COL_LIGHT_BLUE`/`COL_RESET` in C; restored here via the
/// `COL_STR_LIGHT_BLUE`/`COL_STR_RESET` sentinels (see `crate::text`) -
/// callers must use `npc_quiet_say_bytes`, not the lossy `npc_quiet_say`.
pub fn adv_introduction_text(storage_id: i32, player_name: &str) -> String {
    let template = match storage_id.rem_euclid(4) {
        0 => {
            "I could do you a \u{E0C4}favor\u{E0C0}, {name}, I could mention your name to the \
             military governor of Aston. I'm sure that'd help you get that promotion early!"
        }
        1 => {
            "Say, {name}, would you like to speed up your way up the rank ladder? I could speak \
             to the military governor of Aston if you want me to do you that \u{E0C4}favor\u{E0C0}."
        }
        2 => {
            "Not getting promoted as fast as you want, {name}? I could do you the \
             \u{E0C4}favor\u{E0C0} of talking to the military governor of Aston about you."
        }
        _ => "Need a \u{E0C4}favor\u{E0C0}, {name}?",
    };
    template.replace("{name}", player_name)
}

/// C `adv_favor_desc` (`military.c:2296-2308`): the two-line "favor
/// sizes"/"specific missions" explanation, sent as two separate
/// `quiet_say` calls. C wraps every favor-size word and the two example
/// phrases in `COL_LIGHT_BLUE`/`COL_RESET`; restored here via
/// `COL_STR_LIGHT_BLUE`/`COL_STR_RESET` sentinels - callers must use
/// `npc_quiet_say_bytes`, not the lossy `npc_quiet_say`.
pub fn adv_favor_desc_lines() -> [&'static str; 2] {
    [
        "My favors come in five sizes, \u{E0C4}small\u{E0C0}, \u{E0C4}medium\u{E0C0}, \
         \u{E0C4}big\u{E0C0}, \u{E0C4}huge\u{E0C0} and \u{E0C4}vast\u{E0C0}.",
        "I can also recommend you for specific missions. Just tell me the difficulty and type \
         like \u{E0C4}easy demon\u{E0C0} or \u{E0C4}insane mining\u{E0C0}.",
    ]
}

/// [`World::offer_favor`]'s outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferFavorOutcome {
    /// C: `ppd->advisor_last[idx] == yday + 1` -> "Mentioning your name
    /// twice a day won't accomplish much, %s.".
    AlreadyUsedToday,
    /// C's own `default: return 0;` bail-out for an out-of-range
    /// `favor_size` - unreachable via [`crate::character_driver::
    /// MILITARY_QA`]'s fixed qa-code mapping, ported defensively anyway.
    InvalidFavorSize,
    /// The offer was made: `ppd->advisor_cost`/`advisor_state`/
    /// `advisor_storage_nr` were stamped. `cost` is in gold cents (100 =
    /// 1G).
    Offered { favor_size: i32, cost: i32 },
}

/// [`World::handle_specific_mission_request`]'s outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecificMissionRequestOutcome {
    /// C: `ppd->advisor_last[idx] == yday + 1` -> "I've already used my
    /// influence for you today, %s. Come back tomorrow.".
    AlreadyUsedToday,
    /// C: `mission_type < 1 || mission_type > 3` -> "I don't know about
    /// that type of mission, %s.".
    InvalidMissionType,
    /// C: `difficulty < 0 || difficulty > 4` -> "I don't know that
    /// difficulty level, %s.".
    InvalidDifficulty,
    /// C: ratling missions need `level` odd in `9..=39` -> "Ratling
    /// missions are only available at odd levels between 9 and 39, %s.".
    RatlingLevelGate,
    /// C: silver missions need `level >= 12` -> "Silver missions are
    /// only available at level 12 and above, %s.".
    SilverLevelGate,
    /// The offer was made: `ppd->advisor_cost`/`advisor_state`/
    /// `advisor_storage_nr`/`temp_mission_type`/`temp_mission_difficulty`
    /// were stamped. `already_completed_today`/`has_active_mission` carry
    /// the two non-terminal warnings C emits *before* the offer text
    /// (both can be true simultaneously, matching C's `if`/`if` - not
    /// `if`/`else if` - chain).
    Offered {
        difficulty: i32,
        mission_type: i32,
        cost: i32,
        already_completed_today: bool,
        has_active_mission: bool,
    },
}

/// [`World::process_favor_payment`]'s outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessFavorPaymentOutcome {
    /// C: `ppd->current_advisor != dat->storage_ID || ppd->advisor_state
    /// != 2` -> "Pay for what? We haven't agreed on anything yet.",
    /// resets `advisor_state = 1`.
    NothingAgreed,
    /// C: `ch[co].gold < ppd->advisor_cost` -> "Alas, you do not have
    /// enough money.".
    InsufficientGold,
    /// C: `ppd->temp_mission_type > 0` branch - a specific mission
    /// recommendation was arranged; `mission_type_preference`/
    /// `mission_difficulty_preference` were stamped and the temp fields
    /// cleared.
    SpecificMissionArranged { mission_type: i32, difficulty: i32 },
    /// C's `else` branch - a plain favor was arranged;
    /// `ppd->current_pts` gained `2 + favor_size * 2`.
    FavorArranged { favor_size: i32 },
}

impl World {
    /// C `offer_favor(cn, co, ppd, idx, favor_size)` (`military.c:2339-
    /// 2382`). The sales-economy `struct cost_data` bookkeeping
    /// `process_favor_payment` records on acceptance is deliberately out
    /// of scope (no Rust storage-blob equivalent yet - see this module's
    /// doc comment); this only stamps the payment-confirmation state.
    pub fn offer_favor(
        &self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        idx: usize,
        favor_size: i32,
        yday: i32,
    ) -> OfferFavorOutcome {
        if player.military_advisor_last(idx) == yday + 1 {
            return OfferFavorOutcome::AlreadyUsedToday;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return OfferFavorOutcome::InvalidFavorSize;
        };
        let Some(cost) = offer_favor_cost(character.level as i32, favor_size) else {
            return OfferFavorOutcome::InvalidFavorSize;
        };
        player.set_advisor_cost(cost);
        player.set_advisor_state(2);
        player.set_advisor_storage_nr(favor_size);
        OfferFavorOutcome::Offered { favor_size, cost }
    }

    /// C `handle_specific_mission_request(cn, co, ppd, dat, idx,
    /// difficulty, mission_type)` (`military.c:481-566`).
    pub fn handle_specific_mission_request(
        &self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        idx: usize,
        difficulty: i32,
        mission_type: i32,
        yday: i32,
    ) -> SpecificMissionRequestOutcome {
        if player.military_advisor_last(idx) == yday + 1 {
            return SpecificMissionRequestOutcome::AlreadyUsedToday;
        }
        if !(1..=3).contains(&mission_type) {
            return SpecificMissionRequestOutcome::InvalidMissionType;
        }
        if !(0..=4).contains(&difficulty) {
            return SpecificMissionRequestOutcome::InvalidDifficulty;
        }

        let Some(character) = self.characters.get(&character_id) else {
            return SpecificMissionRequestOutcome::InvalidMissionType;
        };
        let level = character.level as i32;

        if mission_type == 2 && (!(9..=39).contains(&level) || level % 2 == 0) {
            return SpecificMissionRequestOutcome::RatlingLevelGate;
        }
        if mission_type == 3 && level < 12 {
            return SpecificMissionRequestOutcome::SilverLevelGate;
        }

        let already_completed_today = player.military_solved_yday() == yday + 1;
        let has_active_mission = player.military_took_mission() != 0;
        let cost = specific_mission_price(level, difficulty, mission_type);

        player.set_advisor_cost(cost);
        player.set_advisor_state(2);
        player.set_advisor_storage_nr(difficulty);
        player.set_temp_mission_type(mission_type);
        player.set_temp_mission_difficulty(difficulty);

        SpecificMissionRequestOutcome::Offered {
            difficulty,
            mission_type,
            cost,
            already_completed_today,
            has_active_mission,
        }
    }

    /// C `process_favor_payment(cn, co, ppd, dat, idx)` (`military.c:
    /// 2402-2474`). `add_cost(ppd->advisor_cost, dat->storage_data +
    /// ppd->advisor_storage_nr)` is ported via [`MilitaryAdvisorStorageRegistry::
    /// add_cost`]; `update_advisor_storage`'s `storage_state`
    /// bump (the C-only async-DB-blob state machine kickoff) is not
    /// reproduced since the in-memory registry has no such state machine
    /// to kick - see this module's doc comment.
    pub fn process_favor_payment(
        &mut self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        idx: usize,
        storage_id: i32,
        yday: i32,
    ) -> ProcessFavorPaymentOutcome {
        if player.current_advisor() != storage_id || player.advisor_state() != 2 {
            player.set_advisor_state(1);
            return ProcessFavorPaymentOutcome::NothingAgreed;
        }

        let Some(character) = self.characters.get_mut(&character_id) else {
            player.set_advisor_state(1);
            return ProcessFavorPaymentOutcome::NothingAgreed;
        };
        let advisor_cost = player.advisor_cost().max(0) as u32;
        if character.gold < advisor_cost {
            return ProcessFavorPaymentOutcome::InsufficientGold;
        }
        character.gold -= advisor_cost;
        character.flags.insert(CharacterFlags::ITEMS);

        self.military_advisor_storage.add_cost(
            storage_id,
            player.advisor_storage_nr().clamp(0, 4) as usize,
            advisor_cost as i32,
        );

        let outcome = if player.temp_mission_type() > 0 {
            let mission_type = player.temp_mission_type();
            let difficulty = player.temp_mission_difficulty();
            player.set_mission_type_preference(mission_type);
            player.set_mission_difficulty_preference(difficulty);
            player.set_temp_mission_type(0);
            player.set_temp_mission_difficulty(-1);
            ProcessFavorPaymentOutcome::SpecificMissionArranged {
                mission_type,
                difficulty,
            }
        } else {
            let favor_size = player.advisor_storage_nr();
            player.set_military_current_pts(player.military_current_pts() + 2 + favor_size * 2);
            ProcessFavorPaymentOutcome::FavorArranged { favor_size }
        };

        player.set_advisor_state(1);
        player.set_military_advisor_last(idx, yday + 1);

        outcome
    }
}

/// C `struct cost_data` (`system/tool.h:94-101`): a single favor size's
/// market-driven sales counters. Only [`Self::earned`]/[`Self::sold`] are
/// ported - the rolling `amount[20]`/`date[20]` sale-history window and
/// the `created` creation timestamp exist in C purely to feed
/// `calc_cost`'s market-driven pricing formula (`tool.c:3187-3215`), and
/// `calc_cost` is never actually called anywhere in the C tree (checked
/// via `grep -rn calc_cost src/` - only its own declaration/definition
/// match), so those fields would be genuinely dead weight with no reader
/// anywhere, unlike `earned`/`sold` which the admin-only "info" qa code
/// (`military.c:2534-2537`) does read back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CostData {
    /// `dat->earned` (`long long`): total gold (in copper/gold-cent
    /// units, matching `ch[].gold`) ever recorded via `add_cost`.
    earned: i64,
    /// `dat->sold` (`int`): number of `add_cost` calls ever recorded.
    sold: i32,
}

impl CostData {
    /// C `add_cost(cost, dat)`'s `dat->earned += cost; dat->sold++;`
    /// half (`tool.c:3219-3226`) - the `amount`/`date` ring-buffer shift
    /// is intentionally not reproduced, see this type's doc comment.
    fn add_cost(&mut self, cost: i32) {
        self.earned += i64::from(cost);
        self.sold += 1;
    }
}

/// C `struct military_advisor_data`'s `struct cost_data storage_data[5]`
/// (`military.c:374`): one [`CostData`] slot per favor size (small/
/// normal/big/huge/vast, indices `0..=4`) - the same 5-way index
/// `ppd->advisor_storage_nr` already uses for both plain favors
/// ([`World::offer_favor`]'s `favor_size`) and specific mission requests
/// ([`World::handle_specific_mission_request`]'s `difficulty`, reused as
/// the storage index verbatim by C's own `dat->storage_data +
/// ppd->advisor_storage_nr` in `process_favor_payment`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MilitaryAdvisorStorage {
    cost_data: [CostData; 5],
}

impl MilitaryAdvisorStorage {
    /// `dat->storage_data[n].earned`. Out-of-range `n` reads as `0`,
    /// matching a fresh zero-initialized slot.
    pub fn earned(&self, favor_size: usize) -> i64 {
        self.cost_data.get(favor_size).map_or(0, |d| d.earned)
    }

    /// `dat->storage_data[n].sold`.
    pub fn sold(&self, favor_size: usize) -> i32 {
        self.cost_data.get(favor_size).map_or(0, |d| d.sold)
    }

    /// C `add_cost(ppd->advisor_cost, dat->storage_data +
    /// ppd->advisor_storage_nr)` (`military.c:2421`).
    fn add_cost(&mut self, favor_size: usize, cost: i32) {
        if let Some(slot) = self.cost_data.get_mut(favor_size) {
            slot.add_cost(cost);
        }
    }
}

/// A registry of [`MilitaryAdvisorStorage`] blobs keyed by `storage_id`
/// (the zone-file `storage=N;` arg every Military Advisor NPC is
/// configured with - `crate::character_driver::MilitaryAdvisorDriverData`),
/// mirroring [`MilitaryMasterStorageRegistry`]'s own shape exactly (one
/// typed struct per consumer, in-memory only for now - see that type's
/// doc comment for the rationale). DB persistence for this registry
/// (a `military_advisor_storage(storage_id integer primary key,
/// storage_json jsonb, updated_at)` table following `crates/ugaris-db/
/// src/military.rs`'s `PgMilitaryMasterStorageRepository` pattern) is
/// left for a future slice - see the "Military ranks" task in
/// `PORTING_TODO.md`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MilitaryAdvisorStorageRegistry {
    entries: std::collections::BTreeMap<i32, MilitaryAdvisorStorage>,
    #[serde(skip)]
    dirty: bool,
}

impl MilitaryAdvisorStorageRegistry {
    /// Read-only lookup; a `storage_id` with no entry yet reads as a
    /// fresh all-zero [`MilitaryAdvisorStorage`], matching C's own
    /// zero-initialized `struct military_advisor_data` before the first
    /// `create_storage` round trip completes.
    pub fn earned(&self, storage_id: i32, favor_size: usize) -> i64 {
        self.entries
            .get(&storage_id)
            .map_or(0, |storage| storage.earned(favor_size))
    }

    /// `dat->storage_data[n].sold`.
    pub fn sold(&self, storage_id: i32, favor_size: usize) -> i32 {
        self.entries
            .get(&storage_id)
            .map_or(0, |storage| storage.sold(favor_size))
    }

    /// C `add_cost(ppd->advisor_cost, dat->storage_data +
    /// ppd->advisor_storage_nr)` (`military.c:2421`), fed by
    /// [`World::process_favor_payment`]. Creates the entry on first use,
    /// matching C's `create_storage` lazily bringing a fresh zeroed blob
    /// into existence.
    fn add_cost(&mut self, storage_id: i32, favor_size: usize, cost: i32) {
        self.entries
            .entry(storage_id)
            .or_default()
            .add_cost(favor_size, cost);
        self.dirty = true;
    }

    /// Whether any entry has changed since the last [`Self::clear_dirty`].
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// All `(storage_id, storage)` rows currently held, for a future DB
    /// repository's per-row upsert on save (mirrors
    /// [`MilitaryMasterStorageRegistry::iter`]).
    pub fn iter(&self) -> impl Iterator<Item = (i32, &MilitaryAdvisorStorage)> {
        self.entries.iter().map(|(id, storage)| (*id, storage))
    }

    /// Rebuilds a registry from persisted `(storage_id, storage)` rows
    /// without marking it dirty (mirrors
    /// [`MilitaryMasterStorageRegistry::from_rows`]).
    pub fn from_rows(rows: impl IntoIterator<Item = (i32, MilitaryAdvisorStorage)>) -> Self {
        Self {
            entries: rows.into_iter().collect(),
            dirty: false,
        }
    }
}

/// C `process_advisor_recommendation`'s own difficulty-name ternary
/// (`military.c:1706-1710`), embedded in the greeting/accept-prompt
/// text: `pref == 0 ? "easy" : pref == 1 ? "normal" : pref == 2 ? "hard" :
/// pref == 3 ? "impossible" : "insane"`. Deliberately distinct from
/// [`mission_difficulty_name`]'s out-of-range fallback (`"easy"`, via
/// `get_colored_difficulty_name`'s own clamp) - this ternary instead
/// falls through to `"insane"` for anything other than `0..=3`, matching
/// C exactly (the caller already guards `mission_difficulty_preference
/// >= 0`, so only the upper side of the range can differ from
/// `mission_difficulty_name`).
pub(crate) fn advisor_recommendation_difficulty_text(preference: i32) -> &'static str {
    match preference {
        0 => "easy",
        1 => "normal",
        2 => "hard",
        3 => "impossible",
        _ => "insane",
    }
}

/// Outcome of [`World::process_advisor_recommendation`] (C
/// `process_advisor_recommendation`, `military.c:1685-1755`), mirroring
/// every distinct `say()` branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvisorRecommendationOutcome {
    /// C: `ppd->recommend == yday + 1` -> already processed today, no
    /// text at all.
    AlreadyProcessed,
    /// C: `mission_type_preference > 0 && mission_difficulty_preference
    /// >= 0` branch - a paid-favor specific-mission recommendation.
    SpecificMission {
        /// The initial "Be greeted... oddly specific request..." line.
        greeting: String,
        /// [`describe_mission_text`]'s own text for the freshly
        /// (re)generated preferred slot - `None` if the mission slot
        /// ended up empty/unrecognized, matching C's own `describe_
        /// mission` silently returning `0` in that case (no text at
        /// all, not even a fallback line).
        description: Option<String>,
        /// The trailing conditional line: already-completed-today /
        /// active-mission-conflict / the "say X to accept" prompt.
        followup: String,
    },
    /// C: the `else` branch - one line per matching `advisor_last[n]`
    /// entry (`military.c:1748-1752`), possibly empty if none matched
    /// today, matching C's own unconditional loop (no "nothing to
    /// report" fallback line either).
    StandardRecommendations(Vec<String>),
}

impl World {
    /// C `process_advisor_recommendation(cn, co, ppd)`
    /// (`military.c:1685-1755`): the Military Master driver's per-visit
    /// paid-advisor-recommendation greeting, called just before
    /// [`crate::PlayerRuntime::greet_player`] in C's own `NT_CHAR`
    /// handler (`military.c:2150-2151`) - [`crate::PlayerRuntime::
    /// greet_player`]'s own `military_recommend() == yday + 1 &&
    /// mission_type_preference() > 0 &&
    /// mission_difficulty_preference() >= 0` short-circuit
    /// (`GreetPlayerOutcome::AdvisorRecommendationAlreadyShown`) exists
    /// specifically to detect
    /// that this function already greeted the player this same call,
    /// matching C's own back-to-back `process_advisor_recommendation`/
    /// `greet_player` call order.
    ///
    /// Reuses [`handle_mission_request`]'s own rank-cubed `military_pts`
    /// floor / level-7 floor / [`crate::PlayerRuntime::
    /// apply_mission_offer`] pattern for the `mission_yday != yday + 1`
    /// regeneration branch (C's own `generate_mission_with_preference(co,
    /// ppd, ppd->mission_type_preference)` call, `military.c:1712-1714`,
    /// is the *full* ppd-mutating function - not the pure table-builder
    /// of the same name in this module - which does exactly that floor/
    /// clamp/stamp sequence internally).
    pub fn process_advisor_recommendation(
        &self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        yday: i32,
        rng_seed: &mut u32,
        player_name: &str,
    ) -> AdvisorRecommendationOutcome {
        if player.military_recommend() == yday + 1 {
            return AdvisorRecommendationOutcome::AlreadyProcessed;
        }

        let outcome = if player.mission_type_preference() > 0
            && player.mission_difficulty_preference() >= 0
        {
            let preferred_type = player.mission_type_preference();
            let diff_pref = player.mission_difficulty_preference();
            let diff_text = advisor_recommendation_difficulty_text(diff_pref);
            let greeting = format!(
                "Be greeted, {player_name}. You have been recommended by my trusty advisor, with \
                 an oddly specific request for {diff_text} {}. Alas, thine wish be granted.",
                mission_type_name(preferred_type)
            );

            if player.mission_yday() != yday + 1 {
                if let Some(character) = self.characters.get(&character_id) {
                    let rank = army_rank_for_points(character.military_points);
                    let rank_cubed = rank.saturating_mul(rank).saturating_mul(rank);
                    if rank_cubed > player.military_pts() {
                        player.set_military_pts(rank_cubed);
                    }
                    let level = (character.level as i32).max(7);
                    let military_pts = player.military_pts();
                    player.apply_mission_offer(level, military_pts, preferred_type, yday, rng_seed);
                }
            }

            let diff_idx = diff_pref.max(0) as usize;
            let mission = player.military_mission(diff_idx);
            let description = describe_mission_text(&mission, diff_idx, player_name);

            let followup = if player.military_solved_yday() == yday + 1 {
                format!(
                    "However, you've already completed a mission today, {player_name}. Come \
                     back tomorrow and this mission will be waiting for you."
                )
            } else if player.military_took_mission() != 0 {
                format!(
                    "However, you already have an active mission, {player_name}. Complete or \
                     abandon it first, then come back to accept this one."
                )
            } else {
                format!(
                    "Say {COL_STR_LIGHT_BLUE}{diff_text}{COL_STR_RESET} to accept this mission."
                )
            };

            AdvisorRecommendationOutcome::SpecificMission {
                greeting,
                description,
                followup,
            }
        } else {
            let mut lines = Vec::new();
            for n in 0..crate::player::MILITARY_PPD_MAXADVISOR {
                if player.military_advisor_last(n) == yday + 1 {
                    lines.push(format!(
                        "Be greeted, {player_name}. You have been recommended by my trusty \
                         advisor {n}"
                    ));
                }
            }
            AdvisorRecommendationOutcome::StandardRecommendations(lines)
        };

        player.set_military_recommend(yday + 1);
        outcome
    }

    /// Reads the Advisor NPC's `storage_ID` (`military.c:369-375`'s
    /// `struct military_advisor_data`, stored via zone-file `storage=N;`
    /// parsing at spawn time - see `crate::zone`) out of its
    /// `driver_state`, defaulting to `0` for a not-yet-initialized or
    /// mismatched driver state (shouldn't happen for a real
    /// `CDR_MILITARY_ADVISOR` character, but mirrors every other
    /// `driver_state`-reading helper's defensive fallback in this
    /// codebase).
    pub fn advisor_storage_id(&self, advisor_id: CharacterId) -> i32 {
        self.characters
            .get(&advisor_id)
            .and_then(|character| character.driver_state.clone())
            .map(|state| match state {
                CharacterDriverState::MilitaryAdvisor(data) => data.storage_id,
                _ => 0,
            })
            .unwrap_or(0)
    }

    pub fn drain_pending_military_advisor_events(&mut self) -> Vec<MilitaryAdvisorEvent> {
        self.pending_military_advisor_events.drain(..).collect()
    }

    /// C `military_advisor_driver`'s `NT_TEXT`/`NT_GIVE` message loop
    /// (`military.c:2678-2691`), via `handle_advisor_message`
    /// (`military.c:2481-2605`). `NT_CHAR` is handled separately by
    /// [`Self::greet_nearby_military_advisor_players`] (matching
    /// `process_military_master_messages`'s split).
    pub(crate) fn process_military_advisor_messages(&mut self, advisor_id: CharacterId) {
        let Some(advisor) = self.characters.get(&advisor_id).cloned() else {
            return;
        };
        let messages = {
            let Some(advisor_mut) = self.characters.get_mut(&advisor_id) else {
                return;
            };
            std::mem::take(&mut advisor_mut.driver_messages)
        };

        let mut destroy_cursor = false;
        let mut replies: Vec<String> = Vec::new();
        let mut events: Vec<MilitaryAdvisorEvent> = Vec::new();

        for message in messages {
            match message.message_type {
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3 as u32);
                    if speaker_id == advisor_id {
                        continue;
                    }
                    let Some(text) = message.text.as_deref() else {
                        continue;
                    };
                    let Some(speaker) = self.characters.get(&speaker_id) else {
                        continue;
                    };
                    if !speaker.flags.contains(CharacterFlags::PLAYER) {
                        continue;
                    }
                    if char_dist(&advisor, speaker) > MILITARY_MASTER_TEXT_DISTANCE {
                        continue;
                    }
                    if !char_see_char(&advisor, speaker, &self.map, self.date.daylight) {
                        continue;
                    }
                    let speaker_name = speaker.name.clone();

                    match analyse_text_qa(text, &advisor.name, &speaker_name, MILITARY_QA) {
                        TextAnalysisOutcome::Said(reply) => replies.push(reply),
                        // C: `answer_code == 1` -> `quiet_say(cn, "I'm
                        // %s.", ch[cn].name)`.
                        TextAnalysisOutcome::Matched(1) => {
                            replies.push(format!("I'm {}.", advisor.name));
                        }
                        TextAnalysisOutcome::Matched(2) => {
                            events.push(MilitaryAdvisorEvent::Repeat {
                                advisor_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(3) => {
                            events.push(MilitaryAdvisorEvent::FavorDesc {
                                advisor_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(code @ 4..=8) => {
                            events.push(MilitaryAdvisorEvent::Favor {
                                advisor_id,
                                player_id: speaker_id,
                                favor_size: (code - 4),
                            });
                        }
                        TextAnalysisOutcome::Matched(9) => {
                            events.push(MilitaryAdvisorEvent::Pay {
                                advisor_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(code @ 30..=44) => {
                            let offset = code - 30;
                            let mission_type = 1 + offset / 5;
                            let difficulty = offset % 5;
                            events.push(MilitaryAdvisorEvent::SpecificMissionRequest {
                                advisor_id,
                                player_id: speaker_id,
                                difficulty,
                                mission_type,
                            });
                        }
                        // C: `if (!(ch[co].flags & CF_GOD)) { break; }`
                        // (`military.c:2523-2525`), same admin-only guard
                        // shape as the Master driver's codes 18-21.
                        TextAnalysisOutcome::Matched(18)
                            if speaker.flags.contains(CharacterFlags::GOD) =>
                        {
                            events.push(MilitaryAdvisorEvent::Info {
                                advisor_id,
                                player_id: speaker_id,
                            });
                        }
                        // Master-only codes (10-17, 19-22), a non-admin
                        // speaker's "info" (18), and any unmatched text:
                        // no handling, matches C's own `default: return
                        // 0;` / admin-gate `break;`.
                        TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
                    }
                }
                NT_GIVE => {
                    destroy_cursor = true;
                    replies.push("That's junk.".to_string());
                }
                _ => {}
            }
        }

        if destroy_cursor {
            let cursor = self
                .characters
                .get_mut(&advisor_id)
                .and_then(|advisor| advisor.cursor_item.take());
            if let Some(item_id) = cursor {
                self.destroy_item(item_id);
            }
        }

        for reply in replies {
            self.npc_quiet_say(advisor_id, &reply);
        }

        self.pending_military_advisor_events.extend(events);
    }

    /// C `military_advisor_driver`'s `NT_CHAR` greeting branch
    /// (`military.c:2639-2661`), ported as the same periodic
    /// nearby-player-scan simplification
    /// [`Self::greet_nearby_military_master_players`] already
    /// established.
    pub(crate) fn greet_nearby_military_advisor_players(&mut self, advisor_id: CharacterId) {
        let Some(advisor) = self.characters.get(&advisor_id).cloned() else {
            return;
        };

        let mut nearby: Vec<CharacterId> = Vec::new();
        for character in self.characters.values() {
            if character.id == advisor_id || !character.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            if char_dist(&advisor, character) > MILITARY_MASTER_GREET_DISTANCE {
                continue;
            }
            if !char_see_char(&advisor, character, &self.map, self.date.daylight) {
                continue;
            }
            nearby.push(character.id);
        }

        self.pending_military_advisor_events
            .extend(
                nearby
                    .into_iter()
                    .map(|player_id| MilitaryAdvisorEvent::NearbyPlayer {
                        advisor_id,
                        player_id,
                    }),
            );
    }

    /// C `military_advisor_driver`'s movement section (`military.c:
    /// 2696-2699`): stationary NPC returning to its `rest_x`/`rest_y`
    /// spawn tile, facing `DX_RIGHT` - unlike the Military Master's
    /// `DX_DOWN`, a genuine (if arbitrary) C difference between the two
    /// drivers, preserved verbatim.
    pub(crate) fn process_military_advisor_tick_action(
        &mut self,
        advisor_id: CharacterId,
        area_id: u16,
    ) {
        let Some(advisor) = self.characters.get(&advisor_id).cloned() else {
            return;
        };
        if self.setup_walk_toward(
            advisor_id,
            usize::from(advisor.rest_x),
            usize::from(advisor.rest_y),
            0,
            area_id,
            false,
        ) {
            return;
        }
        if advisor.dir != MILITARY_ADVISOR_REST_DIRECTION {
            if let Some(advisor_mut) = self.characters.get_mut(&advisor_id) {
                let _ = turn(advisor_mut, MILITARY_ADVISOR_REST_DIRECTION);
            }
        }
    }

    /// Military Advisor NPC tick: process messages, greet scan, and the
    /// movement fallback. Ports the per-tick body of C
    /// `military_advisor_driver` (minus the still-unported storage-blob
    /// async persistence state machine - see this module's doc comment).
    pub fn process_military_advisor_actions(&mut self, area_id: u16) {
        let advisor_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_MILITARY_ADVISOR
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for advisor_id in advisor_ids {
            self.process_military_advisor_messages(advisor_id);
            self.greet_nearby_military_advisor_players(advisor_id);
            self.process_military_advisor_tick_action(advisor_id, area_id);
        }
    }
}

/// C `DX_RIGHT` (`common/direction.h:21`): the Military Advisor's fixed
/// resting facing (C's own `secure_move_driver(cn, ch[cn].tmpx,
/// ch[cn].tmpy, DX_RIGHT, ret, lastact)`, `military.c:2698`).
pub(crate) const MILITARY_ADVISOR_REST_DIRECTION: u8 = 4;

/// A `military_advisor_driver` outcome that needs `PlayerRuntime`'s
/// `military_ppd` to finish applying - see this module's seventh-slice
/// doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MilitaryAdvisorEvent {
    /// C `military_advisor_driver`'s `NT_CHAR` branch (`military.c:
    /// 2639-2661`): the initial-greeting/`current_advisor` stamp.
    NearbyPlayer {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 2 ("repeat"): `ppd->advisor_state = 0;`, no text.
    Repeat {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 3 ("favor"): [`adv_favor_desc_lines`], gated on
    /// `advisor_last[idx]`.
    FavorDesc {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa codes 4-8 ("small".."vast"): [`World::offer_favor`].
    /// `favor_size` is `0..=4`.
    Favor {
        advisor_id: CharacterId,
        player_id: CharacterId,
        favor_size: i32,
    },
    /// qa code 9 ("pay"): [`World::process_favor_payment`].
    Pay {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa codes 30-44 (e.g. "easy demon".."insane silver"):
    /// [`World::handle_specific_mission_request`]. `difficulty` is
    /// `0..=4`, `mission_type` is `1..=3`.
    SpecificMissionRequest {
        advisor_id: CharacterId,
        player_id: CharacterId,
        difficulty: i32,
        mission_type: i32,
    },
    /// Admin-only qa code 18 ("info", `military.c:2525-2538`): shows the
    /// speaker each favor size's sales stats (only nonzero-`sold` sizes),
    /// via [`MilitaryAdvisorStorageRegistry`].
    Info {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
}

/// C `struct military_advisor_data`'s zone-file-parsed half
/// (`src/module/military.c:369-375`) - just the `storage_ID` used by
/// [`crate::world::calculate_advisor_index`] and `adv_introduction`'s
/// `storage_ID % 4` greeting-variant selector. The `struct cost_data
/// storage_data[5]` sales-economy counters are out of scope for this
/// slice - see the "Military ranks" task in `PORTING_TODO.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MilitaryAdvisorDriverData {
    pub storage_id: i32,
}

/// C `military_advisor_parse` (`military.c:2221-2230`): the only
/// zone-file arg this driver reads is `storage=N;`, same shape as
/// [`parse_military_master_driver_args`].
pub fn parse_military_advisor_driver_args(args: &str) -> MilitaryAdvisorDriverData {
    let mut data = MilitaryAdvisorDriverData::default();
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        if name == "storage" {
            data.storage_id = value.parse::<i32>().unwrap_or(0);
        }
        rest = next;
    }
    data
}
