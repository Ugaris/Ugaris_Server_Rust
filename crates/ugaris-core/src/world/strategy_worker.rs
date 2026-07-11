//! `strategy_driver`'s (`strategy.c:713-1120`, `DRD_STRATEGYDRIVER`) order-
//! assignment half: the part of the still-unported recruitable-worker
//! character driver that turns a player's spoken NT_TEXT command
//! ("mine"/"follow"/"guard"/"fight"/"home"/"take"/"transfer"/"train")
//! into a new worker order, plus the map-item lookups that back it
//! (`finditem`/`finddepot`, `:515-625`).
//!
//! This is the next slice of the "Areas 23/24" P4 task after
//! `crate::world::strategy`/`crate::world::strategy_special` - see those
//! modules' doc comments for what's already ported and
//! `item_driver::area23_24`'s doc comment for exactly why the NPC-worker
//! branches of `mine`/`storage`/`depot`/`spawner` are still no-ops
//! (no `CDR_*` id or `CharacterDriverState` variant exists for a worker
//! character yet - nothing can spawn one).
//!
//! Ported here, fully pure/testable without any such character actually
//! existing:
//!
//! - [`StrategyWorkerOrder`]: C `struct strategy_data.order`/`or1`/`or2`
//!   (`:101-102`), replaced by one typed enum carrying each order's
//!   payload directly instead of two overloaded `int`s whose meaning
//!   depends on `order` - same simplification precedent as
//!   `ArenaContender`/`str_did_party_lose`'s own doc comment about using
//!   [`CharacterId`] identity directly instead of C's `cn`+`ID` pair (used
//!   here too: a player-typed target-worker number is compared directly
//!   against a `CharacterId`'s raw value instead of C's recyclable `cn`
//!   array slot - see [`World::strategy_worker_apply_order_text`]'s doc
//!   comment).
//! - [`World::strategy_find_item_near`]/[`World::
//!   strategy_find_depot_or_storage_near`]: C `finditem`/`finddepot`
//!   (`:515-625`), the two ring-spiral map searches the order-assignment
//!   logic (and, eventually, the per-tick order-execution switch this
//!   slice does not port) uses to resolve a spoken command into concrete
//!   items.
//! - [`World::strategy_worker_apply_order_text`]: the actual
//!   `strategy_driver` NT_TEXT command cascade (`:743-883`) minus the
//!   caller-side gate C applies before ever calling it (`ch[co].flags &
//!   CF_PLAYER`, `ch[co].ID == ch[cn].group`, `char_see_char(cn, co)`,
//!   `dat->order != OR_ETERNALGUARD`, `:748-749`) - deferred to whatever
//!   eventually drives a live worker character, since none can exist yet.
//!
//! Also ported here, still fully pure/testable:
//!
//! - [`strategy_train_price`]/[`strategy_worker_name`]/
//!   [`strategy_worker_description`]: C `setname` (`:627-664`) split into
//!   its three pure pieces (`TRAINPRICE` macro, the per-order name
//!   template, and the description line) - the `strcmp`+`reset_name`
//!   "did the name actually change" half stays with whatever eventually
//!   drives a live worker character, same deferral as the NT_TEXT
//!   command cascade above.
//! - [`World::strategy_find_storage_owned_by_group`]: C `findstorage`
//!   (`:564-577`) - a linear first-match scan (not a spiral search like
//!   `finditem`/`finddepot`) for the `IDR_STR_STORAGE` item owned by a
//!   given `ch[cn].group`, in ascending item-index order (same
//!   determinism precedent as `ensure_strategy_areas_initialized`'s own
//!   doc comment, since `self.items` is an unordered `HashMap`).
//! - [`World::strategy_worker_rest_place`]: C `restplace` (`:682-712`) -
//!   the worker's "step aside so the next miner in the queue has room"
//!   fixed-offset fallback search. C's `dat->restplace` persists as a raw
//!   `m`-space integer offset; this port carries it as an `Option<(dx,
//!   dy)>` tile-delta pair instead (`None` standing in for C's `0`
//!   sentinel, since no entry in `restlist` is ever `(0, 0)`).
//!
//! `strategy_driver`'s NT_CREATE handling and the full per-tick order-
//! execution switch (movement/`use_driver` dispatch per order), plus the
//! `CDR_STRATEGY`/`CharacterDriverState` wiring this slice's own message-
//! loop/self-defense/order-dispatch logic needs, and the `mine`/
//! `storage`/`depot` item drivers' NPC-worker platin-transfer branches,
//! are now ported too - see `crate::world::npc::area23_24::worker`'s own
//! module doc comment.
//!
//! REMAINING (tracked in `PORTING_TODO.md`): `spawner_sub`/`take_spawner`
//! spawning (needed to ever construct a live worker through real
//! gameplay - `worker.rs`'s driver is fully testable via directly
//! constructed test characters in the meantime, same "ported but not yet
//! spawnable" precedent as this file's own sixth slice), the
//! `IDR_STR_SPAWNER` item driver, and the full `ai_main`/`ai_init`
//! AI-opponent driver.

use super::*;

/// C `struct strategy_data.order`/`or1`/`or2` (`strategy.c:100-113`) -
/// see this module's doc comment for why this is a typed enum instead of
/// three raw `int`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StrategyWorkerOrder {
    /// C `order == 0` (no `#define`; the zero-initialized default, and
    /// the "home" command's target state, `:827-833`).
    None,
    /// C `OR_MINE` (`:800-805`): mine platinum from `mine_item`, haul it
    /// to `depot_item`.
    Mine {
        mine_item: ItemId,
        depot_item: ItemId,
    },
    /// C `OR_FOLLOW` (`:806-812`): follow `leader` around.
    Follow { leader: CharacterId },
    /// C `OR_GUARD` (`:813-819`): stand guard at a fixed map tile.
    Guard { x: u16, y: u16 },
    /// C `OR_FIGHTER` (`:820-826`): fight alongside/on the command of
    /// `leader`.
    Fighter { leader: CharacterId },
    /// C `OR_TAKE` (`:834-840`): take control of `depot_item` on behalf
    /// of `leader`.
    Take {
        depot_item: ItemId,
        leader: CharacterId,
    },
    /// C `OR_TRANSFER` (`:841-869`): haul platinum from `from_item` to
    /// `to_item`.
    Transfer { from_item: ItemId, to_item: ItemId },
    /// C `OR_TRAIN` (`:870-882`): train up a level near `storage_item`.
    Train { storage_item: ItemId },
    /// C `OR_ETERNALGUARD` (`:98`): never NT_TEXT-assignable (only
    /// `spawner_sub`'s NT_CREATE `ch[cn].arg` branch sets it, `:734` -
    /// out of scope for this slice) - carried here only so callers can
    /// recognize/preserve it. [`World::strategy_worker_apply_order_text`]
    /// never produces this variant, matching C exactly (no `strstr`
    /// keyword ever sets `OR_ETERNALGUARD`).
    EternalGuard { x: u16, y: u16 },
}

impl Default for StrategyWorkerOrder {
    fn default() -> Self {
        StrategyWorkerOrder::None
    }
}

/// C `tool.c:1548-1556`: the capitalized worker-speech honorific by
/// gender flag.
fn military_sirname(character: &Character) -> &'static str {
    if character.flags.contains(CharacterFlags::MALE) {
        "Sir"
    } else if character.flags.contains(CharacterFlags::FEMALE) {
        "Ma'am"
    } else {
        "Neuter"
    }
}

/// C `tool.c:1558-1566`: the lowercase variant of [`military_sirname`].
fn military_sirname_lower(character: &Character) -> &'static str {
    if character.flags.contains(CharacterFlags::MALE) {
        "sir"
    } else if character.flags.contains(CharacterFlags::FEMALE) {
        "ma'am"
    } else {
        "neuter"
    }
}

/// C's `atoi` semantics (leading whitespace, optional sign, then
/// digits) over whatever's left of `text` - used by
/// [`strategy_worker_trim_command_prefix`]'s caller.
fn c_atoi(text: &str) -> i32 {
    let trimmed = text.trim_start();
    let mut chars = trimmed.chars().peekable();
    let mut negative = false;
    match chars.peek() {
        Some('-') => {
            negative = true;
            chars.next();
        }
        Some('+') => {
            chars.next();
        }
        _ => {}
    }
    let mut value: i64 = 0;
    for c in chars {
        match c.to_digit(10) {
            Some(d) => value = value * 10 + i64::from(d),
            None => break,
        }
    }
    if negative {
        -value as i32
    } else {
        value as i32
    }
}

/// C `strategy_driver`'s NT_TEXT prelude (`strategy.c:752-772`): skips a
/// leading "Word Word:" style prefix (however the NT_TEXT payload was
/// actually formatted upstream - not itself in scope for this slice)
/// plus an optional opening quote, leaving whatever the speaker typed
/// after their own addressed-worker number (if any). [`c_atoi`] on the
/// result gives C's `me`; the same trimmed text is also what every
/// subsequent `strstr(text, ...)` keyword check searches (C never
/// resets `text` back to the untrimmed message).
fn strategy_worker_trim_command_prefix(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let skip_alpha = |i: &mut usize| {
        while *i < bytes.len() && bytes[*i].is_ascii_alphabetic() {
            *i += 1;
        }
    };
    let skip_space = |i: &mut usize| {
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
    };
    skip_alpha(&mut i);
    skip_space(&mut i);
    skip_alpha(&mut i);
    if bytes.get(i) == Some(&b':') {
        i += 1;
    }
    skip_space(&mut i);
    if bytes.get(i) == Some(&b'"') {
        i += 1;
    }
    &text[i..]
}

impl World {
    /// C `finditem(int cn, int drv)` (`strategy.c:515-562`): a
    /// clockwise-ring spiral search centered on `(x, y)` (top row, bottom
    /// row, left column, right column, each ring one tile further out,
    /// `dist` 1..=9) for the nearest map-tile item whose driver is
    /// `driver`.
    pub fn strategy_find_item_near(&self, x: u16, y: u16, driver: u16) -> Option<ItemId> {
        self.strategy_spiral_search(x, y, 9, |item| item.driver == driver)
    }

    /// C `finddepot(int xc, int yc, int group)` (`strategy.c:579-625`):
    /// same ring spiral as [`Self::strategy_find_item_near`], out to
    /// `dist` 1..=19, matching either `IDR_STR_DEPOT` or
    /// `IDR_STR_STORAGE`. C's `group` parameter is accepted but never
    /// actually read inside the function body (a dead parameter in the
    /// original C) - not carried here.
    pub fn strategy_find_depot_or_storage_near(&self, x: u16, y: u16) -> Option<ItemId> {
        self.strategy_spiral_search(x, y, 19, |item| {
            item.driver == IDR_STR_DEPOT || item.driver == IDR_STR_STORAGE
        })
    }

    fn strategy_spiral_search(
        &self,
        x: u16,
        y: u16,
        max_dist: i32,
        matches: impl Fn(&Item) -> bool,
    ) -> Option<ItemId> {
        let xc = i32::from(x);
        let yc = i32::from(y);
        let probe = |px: i32, py: i32| -> Option<ItemId> {
            if px < 0 || py < 0 {
                return None;
            }
            let (px, py) = (px as usize, py as usize);
            if !self.map.legacy_inner_bounds(px, py) {
                return None;
            }
            let tile = self.map.tile(px, py)?;
            if tile.item == 0 {
                return None;
            }
            let item_id = ItemId(tile.item);
            let item = self.items.get(&item_id)?;
            matches(item).then_some(item_id)
        };

        for dist in 1..=max_dist {
            for i in -dist..=dist {
                if let Some(id) = probe(xc + i, yc - dist) {
                    return Some(id);
                }
            }
            for i in -dist..=dist {
                if let Some(id) = probe(xc + i, yc + dist) {
                    return Some(id);
                }
            }
            for i in -dist..=dist {
                if let Some(id) = probe(xc - dist, yc + i) {
                    return Some(id);
                }
            }
            for i in -dist..=dist {
                if let Some(id) = probe(xc + dist, yc + i) {
                    return Some(id);
                }
            }
        }
        None
    }

    /// C's shared `abs(it[in].x - it[in2].x) > 18 || abs(it[in].y -
    /// it[in2].y) > 18 || abs(it[in].x - it[in2].x) + abs(it[in].y -
    /// it[in2].y) > 20` "too far apart" guard, inverted to a
    /// "close enough" predicate (`strategy.c:793-794`/`856-857`) - used
    /// by both the "mine" and "transfer" order commands. `false` if
    /// either item no longer exists.
    fn strategy_items_close_enough(&self, a: ItemId, b: ItemId) -> bool {
        let (Some(item_a), Some(item_b)) = (self.items.get(&a), self.items.get(&b)) else {
            return false;
        };
        let dx = (i32::from(item_a.x) - i32::from(item_b.x)).abs();
        let dy = (i32::from(item_a.y) - i32::from(item_b.y)).abs();
        dx <= 18 && dy <= 18 && dx + dy <= 20
    }

    /// C `strategy_driver`'s NT_TEXT command cascade
    /// (`strategy.c:743-883`), minus the caller-side gate C applies
    /// before this code ever runs (`ch[co].flags & CF_PLAYER`,
    /// `ch[co].ID == ch[cn].group`, `char_see_char(cn, co)`,
    /// `dat->order != OR_ETERNALGUARD`, `:748-749`) - see this module's
    /// doc comment for why that's deferred.
    ///
    /// `worker_numeric_id` stands in for C's `cn` (the worker's raw
    /// character-array slot number, which is what a player actually
    /// types to address one specific worker among several, since it's
    /// baked into each worker's visible name by `setname`, e.g. "Neutral's
    /// Worker 42") - same [`CharacterId`]-identity simplification
    /// precedent as `str_did_party_lose`'s own doc comment. Pass the
    /// addressed worker's [`CharacterId`]`.0` here.
    ///
    /// Returns the worker's new order (unchanged if no keyword matched,
    /// the message wasn't addressed to this worker, or a validation
    /// check inside "mine"/"transfer" failed) plus every `say()` line C
    /// would have emitted, in order. A failed "mine"/"transfer"
    /// validation check returns immediately with only that one message,
    /// matching C's `remove_message(cn, msg); continue;` (which skips
    /// every other keyword check for *this* message, `:776-778` etc.) -
    /// unlike a real message queue, this port only ever processes one
    /// message per call, so "skip the rest of this message" and "return"
    /// are equivalent here.
    pub fn strategy_worker_apply_order_text(
        &self,
        current_order: StrategyWorkerOrder,
        worker_pos: (u16, u16),
        worker_numeric_id: u32,
        speaker: &Character,
        text: &str,
    ) -> (StrategyWorkerOrder, Vec<String>) {
        let mut order = current_order;
        let mut messages = Vec::new();

        let trimmed = strategy_worker_trim_command_prefix(text);
        let me = c_atoi(trimmed);

        // C: `if (!me && char_dist(cn, co) > 30) { remove_message(...);
        // continue; }` - message not addressed to anyone in particular,
        // and too far away to plausibly mean us; ignore entirely.
        if me == 0 && map_dist(worker_pos.0, worker_pos.1, speaker.x, speaker.y) > 30 {
            return (order, messages);
        }
        // C: `(!me || me == cn)` gates every single keyword check below.
        if me != 0 && me as u32 != worker_numeric_id {
            return (order, messages);
        }

        let rank = army_rank_name(army_rank_for_points(speaker.military_points));
        let cap_sir = military_sirname(speaker);
        let low_sir = military_sirname_lower(speaker);

        if trimmed.contains("mine") {
            let Some(mine_item) = self.strategy_find_item_near(speaker.x, speaker.y, IDR_STR_MINE)
            else {
                messages.push(format!(
                    "{cap_sir}, {rank}, {low_sir}, sorry {low_sir}, but I cannot find that mine."
                ));
                return (order, messages);
            };
            let Some(depot_item) = self.strategy_find_depot_or_storage_near(speaker.x, speaker.y)
            else {
                messages.push(format!(
                    "{cap_sir}, {rank}, {low_sir}, sorry {low_sir}, but I cannot find a depot."
                ));
                return (order, messages);
            };
            if !self.strategy_items_close_enough(mine_item, depot_item) {
                messages.push(format!(
                    "{cap_sir}, {rank}, {low_sir}, sorry {low_sir}, but those are too far apart."
                ));
                return (order, messages);
            }
            order = StrategyWorkerOrder::Mine {
                mine_item,
                depot_item,
            };
            messages.push(format!(
                "{low_sir}, {rank}, yes, {low_sir}, mine, {low_sir}!"
            ));
        }

        if trimmed.contains("follow") {
            order = StrategyWorkerOrder::Follow { leader: speaker.id };
            messages.push(format!(
                "{rank}, {low_sir}, yes, {low_sir}, follow, {low_sir}!"
            ));
        }

        if trimmed.contains("guard") {
            order = StrategyWorkerOrder::Guard {
                x: speaker.x,
                y: speaker.y,
            };
            messages.push(format!(
                "{rank}, {low_sir}, yes, {low_sir}, guard, {low_sir}!"
            ));
        }

        if trimmed.contains("fight") {
            order = StrategyWorkerOrder::Fighter { leader: speaker.id };
            messages.push(format!(
                "{rank}, {low_sir}, yes, {low_sir}, fight, {low_sir}!"
            ));
        }

        if trimmed.contains("home") {
            order = StrategyWorkerOrder::None;
            messages.push(format!(
                "{rank}, {low_sir}, yes, {low_sir}, go home, {low_sir}!"
            ));
        }

        if trimmed.contains("take") {
            if let Some(depot_item) =
                self.strategy_find_item_near(speaker.x, speaker.y, IDR_STR_DEPOT)
            {
                order = StrategyWorkerOrder::Take {
                    depot_item,
                    leader: speaker.id,
                };
                messages.push(format!(
                    "{rank}, {low_sir}, yes, {low_sir}, take, {low_sir}!"
                ));
            }
        }

        if trimmed.contains("transfer") {
            let (dx, dy) = Direction::try_from(speaker.dir)
                .ok()
                .map(Direction::delta)
                .unwrap_or((0, 0));
            let from_item = self
                .strategy_find_item_near(speaker.x, speaker.y, IDR_STR_DEPOT)
                .or_else(|| self.strategy_find_item_near(speaker.x, speaker.y, IDR_STR_STORAGE));
            let Some(from_item) = from_item else {
                messages.push(format!(
                    "{cap_sir}, {rank}, {low_sir}, sorry {low_sir}, but I cannot find the first depot."
                ));
                return (order, messages);
            };
            let probe_x =
                (i32::from(speaker.x) + i32::from(dx) * 16).clamp(0, i32::from(u16::MAX)) as u16;
            let probe_y =
                (i32::from(speaker.y) + i32::from(dy) * 16).clamp(0, i32::from(u16::MAX)) as u16;
            let Some(to_item) = self.strategy_find_depot_or_storage_near(probe_x, probe_y) else {
                messages.push(format!(
                    "{cap_sir}, {rank}, {low_sir}, sorry {low_sir}, but I cannot find the second depot."
                ));
                return (order, messages);
            };
            if !self.strategy_items_close_enough(from_item, to_item) {
                messages.push(format!(
                    "{cap_sir}, {rank}, {low_sir}, sorry {low_sir}, but those are too far apart."
                ));
                return (order, messages);
            }
            order = StrategyWorkerOrder::Transfer { from_item, to_item };
            messages.push(format!(
                "{rank}, {low_sir}, yes, {low_sir}, transfer, {low_sir}!"
            ));
        }

        if trimmed.contains("train") {
            let Some(storage_item) =
                self.strategy_find_item_near(speaker.x, speaker.y, IDR_STR_STORAGE)
            else {
                messages.push(format!(
                    "{cap_sir}, {rank}, {low_sir}, sorry {low_sir}, but I cannot find a storage."
                ));
                return (order, messages);
            };
            order = StrategyWorkerOrder::Train { storage_item };
            messages.push(format!(
                "{rank}, {low_sir}, yes, {low_sir}, train, {low_sir}!"
            ));
        }

        (order, messages)
    }
}

/// C `TRAINPRICE(cn)` macro (`strategy.c:86`): `(ch[cn].level - 45) * 10`.
pub fn strategy_train_price(level: i32) -> i32 {
    (level - 45) * 10
}

/// C `setname`'s per-order name template (`strategy.c:630-659`), applied
/// to `owner_name` (C's `dat->name`) and `worker_numeric_id` (C's `cn`,
/// same identity-simplification precedent as
/// [`World::strategy_worker_apply_order_text`]'s own doc comment).
pub fn strategy_worker_name(
    order: StrategyWorkerOrder,
    owner_name: &str,
    worker_numeric_id: u32,
) -> String {
    let label = match order {
        StrategyWorkerOrder::Mine { .. } => "Miner",
        StrategyWorkerOrder::Follow { .. } => "Minion",
        StrategyWorkerOrder::Guard { .. } => "Guard",
        StrategyWorkerOrder::EternalGuard { .. } => "E-Guard",
        StrategyWorkerOrder::Fighter { .. } | StrategyWorkerOrder::Take { .. } => "Fighter",
        StrategyWorkerOrder::Transfer { .. } => "Transfer",
        StrategyWorkerOrder::Train { .. } => "Trainee",
        StrategyWorkerOrder::None => "Worker",
    };
    format!("{owner_name}'s {label} {worker_numeric_id}")
}

/// C `setname`'s description line (`strategy.c:664`).
pub fn strategy_worker_description(platin: i32, exp: i32, level: i32) -> String {
    format!(
        "Carrying {platin} Platinum, {exp} of {} exp",
        strategy_train_price(level)
    )
}

/// C `restplace`'s fixed-offset fallback list (`strategy.c:683-697`), as
/// `(dx, dy)` tile deltas instead of raw `m`-space integer offsets - see
/// this module's doc comment for why. Order matters: it's a search
/// priority, not a set.
const STRATEGY_REST_OFFSETS: [(i32, i32); 32] = [
    (-3, -5),
    (-4, -5),
    (-5, -5),
    (-6, -5),
    (-3, 5),
    (-4, 5),
    (-5, 5),
    (-6, 5),
    (3, -5),
    (4, -5),
    (5, -5),
    (6, -5),
    (3, 5),
    (4, 5),
    (5, 5),
    (6, 5),
    (-3, -3),
    (-4, -3),
    (-5, -3),
    (-6, -3),
    (-3, 3),
    (-4, 3),
    (-5, 3),
    (-6, 3),
    (3, -3),
    (4, -3),
    (5, -3),
    (6, -3),
    (3, 3),
    (4, 3),
    (5, 3),
    (6, 3),
];

impl World {
    /// C `findstorage(int cn)` (`strategy.c:564-577`): the first
    /// `IDR_STR_STORAGE` item (ascending item-index order) whose owner
    /// code ([`str_item_owner`]) matches `group`.
    pub fn strategy_find_storage_owned_by_group(&self, group: u16) -> Option<ItemId> {
        let mut item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter(|(_, item)| !item.flags.is_empty())
            .map(|(id, _)| *id)
            .collect();
        item_ids.sort_by_key(|id| id.0);

        for item_id in item_ids {
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            if item.driver == IDR_STR_STORAGE && str_item_owner(item) == u32::from(group) {
                return Some(item_id);
            }
        }
        None
    }

    /// Whether the tile at `(x, y)` is a legal `restplace` target for
    /// `worker`: not blocked (`MF_MOVEBLOCK`/`MF_TMOVEBLOCK`), or blocked
    /// only by `worker` itself already standing there (C's `map[m +
    /// dat->restplace].ch == cn` override, `strategy.c:701`). Out-of-map
    /// tiles (C never bounds-checks `m + offset`) are treated as
    /// illegal, matching every other spiral-search helper in this file.
    fn strategy_rest_tile_is_free(
        &self,
        worker: CharacterId,
        x: i32,
        y: i32,
    ) -> Option<(u16, u16)> {
        if x < 0 || y < 0 {
            return None;
        }
        let (ux, uy) = (x as usize, y as usize);
        if !self.map.legacy_inner_bounds(ux, uy) {
            return None;
        }
        let tile = self.map.tile(ux, uy)?;
        let blocked = tile
            .flags
            .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK);
        let is_self = tile.character == worker.0 as u16;
        if blocked && !is_self {
            return None;
        }
        Some((x as u16, y as u16))
    }

    /// C `restplace(int cn, int m, struct strategy_data *dat)`
    /// (`strategy.c:682-712`). `current_offset` stands in for C's
    /// `dat->restplace` (see this module's doc comment); returns the
    /// offset to persist back into that field (unchanged if neither the
    /// cached offset nor any fallback candidate is free - matching C's
    /// "return `m` unmodified, `dat->restplace` untouched" tail,
    /// `:711-712`) plus the resolved target tile (`base` itself in that
    /// same all-blocked case).
    pub fn strategy_worker_rest_place(
        &self,
        worker: CharacterId,
        base: (u16, u16),
        current_offset: Option<(i32, i32)>,
    ) -> (Option<(i32, i32)>, (u16, u16)) {
        let (bx, by) = (i32::from(base.0), i32::from(base.1));

        if let Some((dx, dy)) = current_offset {
            if let Some(pos) = self.strategy_rest_tile_is_free(worker, bx + dx, by + dy) {
                return (current_offset, pos);
            }
        }

        for &(dx, dy) in &STRATEGY_REST_OFFSETS {
            if let Some(pos) = self.strategy_rest_tile_is_free(worker, bx + dx, by + dy) {
                return (Some((dx, dy)), pos);
            }
        }

        (current_offset, base)
    }
}

// ---- `spawner`/`spawner_sub` (`strategy.c:1244-1381`) ----

use crate::{character_driver::CDR_STRATEGY, player::StrategyPpd};

/// Everything [`World::try_dispatch_strategy_spawner_use`] needs to hand
/// off once eligibility passes - `ugaris-server` builds the actual fresh
/// `"strategy_npc"` character from these fields (via `ZoneLoader` +
/// `World::spawn_character_from_item_drop`) and finishes it off with
/// [`World::finish_strategy_worker_spawn`]. Field names mirror C
/// `spawner_sub`'s own parameters/`ppd` reads (`strategy.c:1244-1286`).
pub struct StrategySpawnerSpawnPlan {
    pub spawner_id: ItemId,
    /// C `group` (`ch[cn].ID`, i.e. the recruiting player's
    /// [`Character::serial`]) - narrowed to `u16` to match
    /// [`Character::group`]'s own field type, same precedent as
    /// [`World::str_did_party_lose`]'s doc comment.
    pub group: u16,
    /// C `name` (`ch[cn].name`, truncated to 20 chars by `spawner_sub`'s
    /// own `strncpy(dat->name, name, 19)`).
    pub owner_name: String,
    pub warcry: i32,
    pub endurance: i32,
    pub speed: i32,
    pub trainspeed: i32,
    pub max_level: i32,
    pub npc_color: i32,
}

/// Result of [`World::try_dispatch_strategy_spawner_use`].
pub enum StrategySpawnerUseOutcome {
    /// Not eligible right now (missing character/item, ownership
    /// mismatch, missing storage item, not enough Platinum, or the
    /// worker-count cap already reached) - the matching C failure message
    /// is already queued via `queue_system_text` (or, for the first two
    /// "shouldn't happen" cases, silently, matching C's own early
    /// `return;` with no message).
    Rejected,
    /// C `spawner_sub`'s own eligibility scan passed and the `NPCPRICE`
    /// Platinum cost has already been deducted (see this variant's own
    /// doc comment on [`World::try_dispatch_strategy_spawner_use`] for
    /// the real C quirk that ordering preserves) - the caller should now
    /// build the fresh worker character.
    Ready(StrategySpawnerSpawnPlan),
}

impl World {
    /// C `spawner(int in, int cn)`'s `ch[cn].flags & CF_PLAYER` branch
    /// (`strategy.c:1355-1381`) plus `spawner_sub`'s own worker-count
    /// eligibility scan (`:1244-1253`) - everything computable without
    /// actually creating a character (that half needs `ZoneLoader`, only
    /// `ugaris-server` has one - see [`StrategySpawnerUseOutcome`]'s own
    /// doc comment).
    ///
    /// A real C quirk is preserved deliberately, not "fixed": once the
    /// worker-count cap check passes, `spawner_sub` deducts the storage's
    /// `NPCPRICE` Platinum *unconditionally*, *before* `create_char`/
    /// `item_drop_char` ever runs - so a subsequent drop failure (no free
    /// adjacent tile) still spends the Platinum with nothing to show for
    /// it. This method reproduces that ordering exactly: the deduction
    /// happens right here, in the `Ready` branch, before the caller ever
    /// attempts to build the character; the caller must NOT refund it if
    /// character creation subsequently fails.
    pub fn try_dispatch_strategy_spawner_use(
        &mut self,
        character_id: CharacterId,
        spawner_id: ItemId,
        ppd: &StrategyPpd,
    ) -> StrategySpawnerUseOutcome {
        let Some(character) = self.characters.get(&character_id) else {
            return StrategySpawnerUseOutcome::Rejected;
        };
        let serial = character.serial;
        // C `strncpy(dat->name, name, 19); dat->name[19] = 0;` - 19
        // chars plus the null terminator, so 19 visible characters (the
        // sibling `strategy_take_spawner`'s own `name` truncation uses 20
        // - a real, harmless one-character difference between the two C
        // call sites, reproduced faithfully rather than unified).
        let owner_name: String = character.name.chars().take(19).collect();

        let Some(spawner) = self.items.get(&spawner_id) else {
            return StrategySpawnerUseOutcome::Rejected;
        };
        if str_item_owner(spawner) != serial {
            self.queue_system_text(
                character_id,
                "This spawner belongs to somebody else.".to_string(),
            );
            return StrategySpawnerUseOutcome::Rejected;
        }

        let Some(storage_id) = self.str_spawner_storage_item(spawner_id) else {
            self.queue_system_text(
                character_id,
                "Failed. Please report bug #25476e".to_string(),
            );
            return StrategySpawnerUseOutcome::Rejected;
        };
        let Some(storage) = self.items.get(&storage_id) else {
            self.queue_system_text(
                character_id,
                "Failed. Please report bug #25476e".to_string(),
            );
            return StrategySpawnerUseOutcome::Rejected;
        };
        if str_item_gold(storage) < NPCPRICE as u32 {
            self.queue_system_text(
                character_id,
                "Not enough Platinum to create a worker.".to_string(),
            );
            return StrategySpawnerUseOutcome::Rejected;
        }

        // C `spawner_sub`'s own eligibility scan (`:1246-1252`): count
        // every live `CDR_STRATEGY` character in this same `group` whose
        // order isn't `OR_ETERNALGUARD`.
        let group = serial as u16;
        let worker_count = self
            .characters
            .values()
            .filter(|worker| {
                if worker.driver != CDR_STRATEGY
                    || !worker.flags.contains(CharacterFlags::USED)
                    || worker.group != group
                {
                    return false;
                }
                match &worker.driver_state {
                    Some(CharacterDriverState::StrategyWorker(data)) => {
                        !matches!(data.order, StrategyWorkerOrder::EternalGuard { .. })
                    }
                    _ => true,
                }
            })
            .count();
        if worker_count as i32 >= ppd.max_worker {
            self.queue_system_text(
                character_id,
                "No space to drop char or max worker reached.".to_string(),
            );
            return StrategySpawnerUseOutcome::Rejected;
        }

        // C `*(unsigned int *)(it[in2].drdata + 4) -= NPCPRICE;` - see
        // this method's own doc comment for the drop-failure quirk this
        // preserves.
        if let Some(item) = self.items.get_mut(&storage_id) {
            let new_gold = str_item_gold(item).saturating_sub(NPCPRICE as u32);
            set_str_item_gold(item, new_gold);
        }

        StrategySpawnerUseOutcome::Ready(StrategySpawnerSpawnPlan {
            spawner_id,
            group,
            owner_name,
            warcry: ppd.warcry,
            endurance: ppd.endurance,
            speed: ppd.speed,
            trainspeed: ppd.trainspeed,
            max_level: ppd.max_level,
            npc_color: ppd.npc_color,
        })
    }

    /// C `spawner_sub`'s driver-state stamp (`strategy.c:1280-1286`),
    /// once the fresh worker character already exists in `self.
    /// characters` (`ugaris-server` handles `create_char`/`item_drop_char`
    /// /`update_char`/hp-endurance-mana-to-max/dir/sprite/group first -
    /// see `ugaris-server::area23_24::spawn_strategy_worker`).
    pub fn finish_strategy_worker_spawn(
        &mut self,
        character_id: CharacterId,
        owner_name: String,
        trainspeed: i32,
        max_level: i32,
    ) {
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.driver = CDR_STRATEGY;
            character.driver_state = Some(CharacterDriverState::StrategyWorker(
                StrategyWorkerDriverData {
                    owner_name,
                    trainspeed,
                    max_level,
                    ..StrategyWorkerDriverData::default()
                },
            ));
        }
    }
}
