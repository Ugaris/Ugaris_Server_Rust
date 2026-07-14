//! Recruitable worker/fighter/miner NPC (`CDR_STRATEGY`).
//!
//! Ports C `src/area/23_24/strategy.c::strategy_driver` (`:714-1121`): the
//! per-tick body of every character spawned by the still-unported
//! `spawner_sub`/`take_spawner` ("worker"/"fighter"/"guard"/"eguard"),
//! following orders assigned by [`World::strategy_worker_apply_order_text`]
//! (already ported in `crate::world::strategy_worker`). Also ports the
//! NPC-worker branches of `mine`/`storage`/`depot` (`:1123-1239`, the
//! `!(ch[cn].flags & CF_PLAYER)` halves - the `CF_PLAYER` "look" branches
//! were already ported in `crate::item_driver::area23_24`) so a worker's
//! `OR_MINE`/`OR_TRANSFER`/`OR_TAKE`/`OR_TRAIN` orders actually move
//! Platinum between buildings, not just walk toward them.
//!
//! No live `CDR_STRATEGY` character can exist yet - `spawner_sub`/
//! `take_spawner` spawning remains unported (see `crate::world::
//! strategy_worker`'s module doc comment) - so every behavior here is
//! exercised via directly-constructed test characters, same "ported but
//! not yet spawnable" precedent as that module's own order-assignment
//! slice.
//!
//! Deviations/gaps (documented, not silent):
//! - C's self-defense is the fully generic 10-slot `struct
//!   fight_driver_data` fed by `standard_message_driver`'s NT_CHAR/
//!   NT_SEEHIT/NT_GOTHIT handling (with an `aggressive`/`helper` toggle
//!   that differs by order: `1,1` for GUARD/FIGHTER/TRAIN/ETERNALGUARD,
//!   `0,0` otherwise). This port tracks only the single most-recent
//!   NT_GOTHIT attacker as `victim`, the same single-enemy simplification
//!   already established for `CDR_ROBBER`/`CDR_GATE_FIGHT` (see
//!   `world/npc/area1/robber.rs`'s own module doc comment) - so a worker
//!   never proactively engages a merely-sighted or ally-under-attack
//!   enemy regardless of order, only reacts once actually hit. "Attack
//!   visible" reuses `World::attack_driver_direct`; "follow invisible" is
//!   not ported (C doesn't call it here either - `strategy_driver` only
//!   ever calls `fight_driver_attack_visible`, never `_follow_invisible`).
//! - `fight_driver_set_dist(cn, 26, 0, 30)` (NT_CREATE) and the per-order
//!   `fight_driver_set_home` calls are not ported - same "single-victim
//!   model has no equivalent gate" precedent as `robber.rs`.
//! - `ch[co].ID == ch[cn].group` (the NT_TEXT command-giver gate,
//!   `:749`) becomes `speaker.group == worker.group`: this port has no
//!   separate persistent-ID-vs-array-slot distinction (`CharacterId` IS
//!   the stable identity everywhere), and `group` is exactly the field
//!   `army8_army`'s own soldier-spawning already uses to carry an owning
//!   player's identity onto a recruited NPC (`ugaris-server::area8_army::
//!   spawn_army_soldier`'s `pgroup` parameter) - the natural equivalent
//!   within this codebase's existing conventions.
//! - `reset_name(cn)` (cached colored-name client resync on a name
//!   change) is not ported - same documented gap as everywhere else in
//!   this codebase (see `world/exp.rs`'s own doc comment).
//! - NT_CREATE's `if (ch[cn].arg) { ...become an eternal guard... }`
//!   branch is not ported: no zone-file `arg` scratch field is modeled
//!   for this driver (same class of omission as `astro1.rs`), and no
//!   currently-ported spawner ever sets it anyway (`create_eguard`, the
//!   only caller that would need it, is part of the still-unported
//!   `ai_main`/`ai_init` system).
//! - `OR_TAKE`'s `dat->or2 == 0` fallback (become `OR_GUARD` at the
//!   worker's own position instead of `OR_FIGHTER`) is not modeled -
//!   [`crate::world::strategy_worker::StrategyWorkerOrder::Take`] always
//!   carries a real `leader`, since the only reachable assignment path
//!   ([`World::strategy_worker_apply_order_text`]'s "take" keyword) never
//!   produces `or2 == 0`; only `ai_init` (unported) would ever need it.
//! - The decorative "melt the snow under your boots" easter egg
//!   (`:910-924`, `lastact == AC_WALK && !RANDOM(10)`) is not ported -
//!   purely cosmetic, no gameplay effect.
//! - `mine`/`storage`/`depot`'s `cn == 0` cosmetic-naming/periodic-income
//!   branches remain documented gaps in `crate::item_driver::
//!   area23_24`'s own module doc comment (unrelated to this driver).

use crate::world::*;

impl World {
    /// C `ch_driver`'s `CDR_STRATEGY` case (`strategy.c:1611-1613`).
    pub fn process_strategy_worker_actions(&mut self, area_id: u16) -> usize {
        let worker_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_STRATEGY
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for worker_id in worker_ids {
            if self.process_strategy_worker_tick(worker_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `strategy_driver`'s per-tick body (`strategy.c:714-1121`).
    fn process_strategy_worker_tick(&mut self, worker_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&worker_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::StrategyWorker(data)) => data,
            _ => StrategyWorkerDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&worker_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            // C `if (msg->type == NT_CREATE) { ...; ch[cn].level =
            // ch[cn].value[1][V_WIS]; }` (`:730-741`) - the `ch[cn].arg`
            // eternal-guard branch is not ported, see module doc comment.
            if message.message_type == NT_CREATE {
                if let Some(character) = self.characters.get_mut(&worker_id) {
                    let wis = character_value_present(character, CharacterValue::Wisdom);
                    character.level = wis.max(0) as u32;
                }
            }

            // C `if (msg->type == NT_TEXT) { tabunga(...); if (...) {
            // ...apply order text... } }` (`:744-885`).
            if message.message_type == NT_TEXT {
                let speaker_id = CharacterId(message.dat3 as u32);
                if let Some(text) = message.text.as_deref() {
                    self.apply_tabunga_text_notification(worker_id, speaker_id, text);
                    if !matches!(data.order, StrategyWorkerOrder::EternalGuard { .. }) {
                        if let (Some(worker), Some(speaker)) = (
                            self.characters.get(&worker_id).cloned(),
                            self.characters.get(&speaker_id).cloned(),
                        ) {
                            if speaker.flags.contains(CharacterFlags::PLAYER)
                                && speaker.group == worker.group
                                && char_see_char(&worker, &speaker, &self.map, self.date.daylight)
                            {
                                let (new_order, lines) = self.strategy_worker_apply_order_text(
                                    data.order,
                                    (worker.x, worker.y),
                                    worker_id.0,
                                    &speaker,
                                    text,
                                );
                                data.order = new_order;
                                for line in lines {
                                    self.npc_say(worker_id, &line);
                                }
                            }
                        }
                    }
                }
            }

            // C `if (msg->type == NT_GIVE) { destroy_item(ch[cn].citem);
            // ch[cn].citem = 0; }` (`:888-891`).
            if message.message_type == NT_GIVE {
                let cursor_item = self
                    .characters
                    .get(&worker_id)
                    .and_then(|character| character.cursor_item);
                if let Some(item_id) = cursor_item {
                    self.destroy_item(item_id);
                }
                if let Some(character) = self.characters.get_mut(&worker_id) {
                    character.cursor_item = None;
                }
            }

            // C `if (msg->type == NT_GOTHIT) { dat->lasthit = ticker; }`
            // (`:893-895`) plus this port's single-victim self-defense
            // tracking (see module doc comment).
            if message.message_type == NT_GOTHIT {
                data.lasthit = self.tick.0;
                if message.dat1 > 0 {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    if let Some((worker, attacker)) = self
                        .characters
                        .get(&worker_id)
                        .cloned()
                        .zip(self.characters.get(&attacker_id).cloned())
                    {
                        if worker.group != attacker.group
                            && can_attack(&worker, &attacker, &self.map)
                        {
                            data.victim = Some(attacker_id);
                        }
                    }
                }
            }
        }

        // C `setname(cn, dat)` (`:926`, `strategy.c:627-665`).
        if let Some(character) = self.characters.get_mut(&worker_id) {
            let new_name = strategy_worker_name(data.order, &data.owner_name, worker_id.0);
            if character.name != new_name {
                character.name = new_name;
                // C `reset_name(cn)` intentionally not ported - see module
                // doc comment.
            }
            character.description =
                strategy_worker_description(data.platin, data.exp, character.level as i32);
        }

        // C `fight_driver_update(cn)` (`:928`): refresh the tracked
        // victim's visibility/last-seen position, or drop it once it's
        // gone (same shape as `robber.rs`/`gate_fight.rs`).
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&worker_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((worker, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&worker, &victim, &self.map, self.date.daylight) {
                        data.victim_visible = true;
                        data.victim_last_x = victim.x;
                        data.victim_last_y = victim.y;
                    } else {
                        data.victim_visible = false;
                    }
                }
                _ => {
                    data.victim = None;
                    data.victim_visible = false;
                }
            }
        }

        if let Some(character) = self.characters.get_mut(&worker_id) {
            character.driver_state = Some(CharacterDriverState::StrategyWorker(data.clone()));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;` (`:929-931`).
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(worker_id, victim_id, area_id) {
                    return true;
                }
            }
        }

        // C `if (ticker - dat->lasthit > TICKS*10 && regenerate_driver(cn))
        // return;` (`:932-934`).
        if self.tick.0.saturating_sub(data.lasthit) > TICKS_PER_SECOND * 10
            && self.regenerate_simple_baddy(worker_id)
        {
            return true;
        }

        // C `switch (dat->order) { ... }` (`:936-1118`).
        let acted = match data.order {
            StrategyWorkerOrder::Mine {
                mine_item,
                depot_item,
            } => self
                .strategy_worker_tick_mine_or_transfer(worker_id, mine_item, depot_item, area_id),
            StrategyWorkerOrder::Transfer { from_item, to_item } => {
                self.strategy_worker_tick_mine_or_transfer(worker_id, from_item, to_item, area_id)
            }
            StrategyWorkerOrder::Take { depot_item, leader } => {
                self.strategy_worker_tick_take(worker_id, depot_item, leader, &mut data, area_id)
            }
            StrategyWorkerOrder::Follow { leader } | StrategyWorkerOrder::Fighter { leader } => {
                self.strategy_worker_tick_follow_or_fight(worker_id, leader, &mut data, area_id)
            }
            StrategyWorkerOrder::Guard { x, y } | StrategyWorkerOrder::EternalGuard { x, y } => {
                self.strategy_worker_tick_guard(worker_id, x, y, area_id)
            }
            StrategyWorkerOrder::Train { storage_item } => {
                self.strategy_worker_tick_train(worker_id, storage_item, &mut data, area_id)
            }
            StrategyWorkerOrder::None => self.strategy_worker_tick_default(worker_id, area_id),
        };

        if let Some(character) = self.characters.get_mut(&worker_id) {
            character.driver_state = Some(CharacterDriverState::StrategyWorker(data));
        }

        if acted {
            return true;
        }

        // C's trailing `do_idle(cn, TICKS); return;` (`:1120`), shared by
        // every switch arm that didn't already return - see this file's
        // per-order helpers' own doc comments for why they simplify to
        // `return false` instead of each calling this directly.
        self.idle_simple_baddy(worker_id)
    }

    /// C `case OR_MINE:`/`case OR_TRANSFER:` (`:937-977`) - byte-for-byte
    /// identical bodies (`source_item`/`dest_item` stand in for
    /// `dat->or1`/`dat->or2`, whichever pair the caller's order variant
    /// carries).
    fn strategy_worker_tick_mine_or_transfer(
        &mut self,
        worker_id: CharacterId,
        source_item: ItemId,
        dest_item: ItemId,
        area_id: u16,
    ) -> bool {
        let carrying = matches!(
            self.characters.get(&worker_id).and_then(|c| c.driver_state.as_ref()),
            Some(CharacterDriverState::StrategyWorker(data)) if data.platin != 0
        );
        if !carrying {
            let Some(source) = self.items.get(&source_item).cloned() else {
                return false;
            };
            if str_item_gold(&source) == 0 {
                let Some(worker) = self.characters.get(&worker_id) else {
                    return false;
                };
                let dx = (i32::from(worker.x) - i32::from(source.x)).abs();
                let dy = (i32::from(worker.y) - i32::from(source.y)).abs();
                if (dx != 3 || dy != 3)
                    && self.setup_walk_toward(
                        worker_id,
                        usize::from(source.x),
                        usize::from(source.y),
                        3,
                        area_id,
                        false,
                    )
                {
                    return true;
                }
                return false;
            }
            self.strategy_worker_use_item(worker_id, source_item, area_id)
        } else {
            self.strategy_worker_use_item(worker_id, dest_item, area_id)
        }
    }

    /// C `case OR_TAKE:` (`:979-996`).
    fn strategy_worker_tick_take(
        &mut self,
        worker_id: CharacterId,
        depot_item: ItemId,
        leader: CharacterId,
        data: &mut StrategyWorkerDriverData,
        area_id: u16,
    ) -> bool {
        let Some(item) = self.items.get(&depot_item).cloned() else {
            return false;
        };
        let Some(worker) = self.characters.get(&worker_id) else {
            return false;
        };
        if str_item_owner(&item) == u32::from(worker.group) {
            // C's `dat->or2 == 0` fallback to `OR_GUARD` is not modeled -
            // see module doc comment.
            data.order = StrategyWorkerOrder::Fighter { leader };
            return self
                .characters
                .get_mut(&worker_id)
                .is_some_and(|character| {
                    do_idle(character, (TICKS_PER_SECOND / 4) as i32).is_ok()
                });
        }
        self.strategy_worker_use_item(worker_id, depot_item, area_id)
    }

    /// C `case OR_FOLLOW:`/`case OR_FIGHTER:` (`:998-1013`).
    fn strategy_worker_tick_follow_or_fight(
        &mut self,
        worker_id: CharacterId,
        leader: CharacterId,
        data: &mut StrategyWorkerDriverData,
        area_id: u16,
    ) -> bool {
        let leader_valid = self
            .characters
            .get(&leader)
            .is_some_and(|character| !character.flags.contains(CharacterFlags::DEAD));
        if !leader_valid {
            data.order = StrategyWorkerOrder::None;
            return false;
        }
        let Some((worker_x, worker_y)) = self
            .characters
            .get(&worker_id)
            .map(|character| (character.x, character.y))
        else {
            return false;
        };
        let (leader_x, leader_y) = self
            .characters
            .get(&leader)
            .map(|character| (character.x, character.y))
            .unwrap_or_default();
        if worker_x.abs_diff(leader_x) > 2 || worker_y.abs_diff(leader_y) > 2 {
            self.setup_walk_toward(
                worker_id,
                usize::from(leader_x),
                usize::from(leader_y),
                2,
                area_id,
                false,
            )
        } else {
            false
        }
    }

    /// C `case OR_GUARD:`/`case OR_ETERNALGUARD:` (`:1015-1042`) - every
    /// failure path ends in C's shared trailing `do_idle`, so this
    /// simplifies to "try progressively looser `move_driver` min-dists,
    /// `false` if every one fails" (see the outer function's own doc
    /// comment).
    fn strategy_worker_tick_guard(
        &mut self,
        worker_id: CharacterId,
        guard_x: u16,
        guard_y: u16,
        area_id: u16,
    ) -> bool {
        let Some(worker) = self.characters.get(&worker_id) else {
            return false;
        };
        let dist = manhattan_distance(
            usize::from(worker.x),
            usize::from(worker.y),
            usize::from(guard_x),
            usize::from(guard_y),
        );
        if dist == 0 {
            return false;
        }
        if self.setup_walk_toward(
            worker_id,
            usize::from(guard_x),
            usize::from(guard_y),
            0,
            area_id,
            false,
        ) {
            return true;
        }
        if dist > 2 {
            if self.setup_walk_toward(
                worker_id,
                usize::from(guard_x),
                usize::from(guard_y),
                2,
                area_id,
                false,
            ) {
                return true;
            }
            if dist > 4 {
                return self.setup_walk_toward(
                    worker_id,
                    usize::from(guard_x),
                    usize::from(guard_y),
                    4,
                    area_id,
                    false,
                );
            }
        }
        false
    }

    /// C `case OR_TRAIN:` (`:1044-1105`).
    fn strategy_worker_tick_train(
        &mut self,
        worker_id: CharacterId,
        storage_item: ItemId,
        data: &mut StrategyWorkerDriverData,
        area_id: u16,
    ) -> bool {
        let Some(worker) = self.characters.get(&worker_id).cloned() else {
            return false;
        };
        if i64::from(worker.level) >= i64::from(data.max_level) {
            data.order = StrategyWorkerOrder::Guard {
                x: worker.x,
                y: worker.y,
            };
            return false;
        }

        if data.platin >= data.trainspeed {
            data.platin -= data.trainspeed;
            data.exp += data.trainspeed * TRAINMULTI;
            let price = strategy_train_price(worker.level as i32);
            if data.exp >= price {
                data.exp -= price;
                let new_level = (worker.level + 1).min(115);
                if let Some(character) = self.characters.get_mut(&worker_id) {
                    character.level = new_level;
                    for value in [
                        CharacterValue::Wisdom,
                        CharacterValue::Intelligence,
                        CharacterValue::Agility,
                        CharacterValue::Strength,
                        CharacterValue::Hand,
                        CharacterValue::Attack,
                        CharacterValue::Parry,
                    ] {
                        if let Some(slot) = character
                            .values
                            .get_mut(1)
                            .and_then(|row| row.get_mut(value as usize))
                        {
                            *slot = new_level as i16;
                        }
                    }
                    // C `reset_name(cn)` intentionally not ported - see
                    // module doc comment.
                }
                self.update_character(worker_id);
            }
            return self.strategy_worker_move_to_restplace(worker_id, storage_item, data, area_id);
        }

        let Some(storage) = self.items.get(&storage_item).cloned() else {
            return false;
        };
        if str_item_gold(&storage) < data.trainspeed.max(0) as u32 {
            return self.strategy_worker_move_to_restplace(worker_id, storage_item, data, area_id);
        }
        self.strategy_worker_use_item(worker_id, storage_item, area_id)
    }

    /// Shared "walk to the `restplace` beside `building_item`" tail used
    /// by both of `OR_TRAIN`'s two `restplace` call sites (`:1069-1083`/
    /// `1089-1100`) - both try `move_driver` at `min_dist` 0 then 2,
    /// `false` (shared trailing idle) if both fail.
    fn strategy_worker_move_to_restplace(
        &mut self,
        worker_id: CharacterId,
        building_item: ItemId,
        data: &mut StrategyWorkerDriverData,
        area_id: u16,
    ) -> bool {
        let Some(building) = self.items.get(&building_item).cloned() else {
            return false;
        };
        let (new_offset, (tx, ty)) =
            self.strategy_worker_rest_place(worker_id, (building.x, building.y), data.restplace);
        data.restplace = new_offset;
        let Some(worker) = self.characters.get(&worker_id) else {
            return false;
        };
        if worker.x != tx || worker.y != ty {
            if self.setup_walk_toward(
                worker_id,
                usize::from(tx),
                usize::from(ty),
                0,
                area_id,
                false,
            ) {
                return true;
            }
            if self.setup_walk_toward(
                worker_id,
                usize::from(tx),
                usize::from(ty),
                2,
                area_id,
                false,
            ) {
                return true;
            }
        }
        false
    }

    /// C `default:` (`:1107-1117`). Unlike C, this port recomputes
    /// `findstorage` fresh every tick instead of caching it in `dat->or1`
    /// (`StrategyWorkerOrder::None` carries no payload to cache into) -
    /// a deterministic first-match scan either way, so purely a minor
    /// perf simplification, never a behavior difference.
    fn strategy_worker_tick_default(&mut self, worker_id: CharacterId, area_id: u16) -> bool {
        let Some(worker) = self.characters.get(&worker_id).cloned() else {
            return false;
        };
        let Some(storage_item) = self.strategy_find_storage_owned_by_group(worker.group) else {
            return false;
        };
        let Some(storage) = self.items.get(&storage_item).cloned() else {
            return false;
        };
        let dx = worker.x.abs_diff(storage.x);
        let dy = worker.y.abs_diff(storage.y);
        if dx > 3 || dy > 3 {
            return self.setup_walk_toward(
                worker_id,
                usize::from(storage.x),
                usize::from(storage.y),
                3,
                area_id,
                false,
            );
        }
        false
    }

    /// C `use_driver(cn, in, 0)` for a fixed-position building item: walk
    /// to it if not already adjacent, then use it (dispatching to
    /// `str_mine_driver`/`str_storage_driver`/`str_depot_driver`'s
    /// NPC-worker branch on action completion). Identical shape to
    /// `World::lampghost_use_lamp`/`World::janitor_use_light`.
    fn strategy_worker_use_item(
        &mut self,
        worker_id: CharacterId,
        item_id: ItemId,
        area_id: u16,
    ) -> bool {
        let Some(item) = self.items.get(&item_id).cloned() else {
            return false;
        };
        if !item.flags.contains(ItemFlags::USE) {
            return false;
        }
        let Some(worker) = self.characters.get(&worker_id) else {
            return false;
        };
        let direction = adjacent_use_direction(
            worker.x,
            worker.y,
            usize::from(item.x),
            usize::from(item.y),
            item.flags.contains(ItemFlags::FRONTWALL),
        );
        if let Some(direction) = direction {
            let Some(worker) = self.characters.get_mut(&worker_id) else {
                return false;
            };
            do_use(
                worker,
                &self.map,
                &item,
                direction as u8,
                0,
                self.settings.weather_movement_percent,
            )
            .is_ok()
        } else {
            self.setup_walk_toward_use_item(
                worker_id,
                usize::from(item.x),
                usize::from(item.y),
                item.flags,
                area_id,
            )
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_STRATEGY, NT_CREATE, NT_GIVE, NT_GOTHIT, NT_TEXT};

/// C `struct strategy_data` (`strategy.c:99-112`), plus this port's own
/// single-victim self-defense tracking (see module doc comment). `name`
/// becomes `owner_name` (C's `dat->name`, the recruiting player's name
/// captured at spawn time - not yet populated by any ported spawner).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StrategyWorkerDriverData {
    pub order: StrategyWorkerOrder,
    pub platin: i32,
    pub exp: i32,
    pub trainspeed: i32,
    pub max_level: i32,
    pub owner_name: String,
    pub restplace: Option<(i32, i32)>,
    pub lasthit: u64,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
