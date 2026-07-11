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
//! REMAINING (tracked in `PORTING_TODO.md`): `strategy_driver`'s NT_CREATE
//! handling, `setname`/`restplace`/`findstorage`, the full per-tick
//! order-execution switch (movement/`use_driver` dispatch per order), the
//! `CDR_STRATEGY`/`CharacterDriverState` wiring and `spawner_sub`
//! spawning needed to ever construct a live worker in the first place,
//! the `mine`/`storage`/`depot`/`spawner` item drivers' NPC-worker
//! branches, and the full `ai_main`/`ai_init` AI-opponent driver.

use super::*;

/// C `struct strategy_data.order`/`or1`/`or2` (`strategy.c:100-113`) -
/// see this module's doc comment for why this is a typed enum instead of
/// three raw `int`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
