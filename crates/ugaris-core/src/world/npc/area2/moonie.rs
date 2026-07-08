//! Spider-eating companion NPC (`CDR_MOONIE`, the `moony_elite` zone-file
//! template, `zones/2/below2.chr:1728-1798`).
//!
//! Ports `src/area/2/area2.c::moonie_driver` (`:449-567`): a fight NPC that
//! interrupts whatever it's doing to fetch and "eat" (destroy) any visible
//! `IID_AREA2_SMALLSPIDER`, spends a minute happily munching afterward, and
//! blesses a same-group ally it can see once it isn't busy eating.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification already established
//!   for `CDR_SUPERIOR`/`CDR_BALLTRAP`/etc: C's generic 10-slot
//!   `struct fight_driver_data` is narrowed to a single tracked `victim`,
//!   set from both `NT_CHAR` sightings (`aggressive=1`) and `NT_GOTHIT`
//!   defense, matching `is_valid_enemy(cn, co, -1)` exactly (see
//!   `world::superior`'s module doc comment for the full justification).
//!   `NT_SEEHIT` group-helper aggro (`helper=1`) is not ported - no
//!   sibling NPC ports this branch either (it only matters when multiple
//!   moonie-like NPCs fight alongside each other, an edge case with no
//!   observable single-player difference).
//! - `fight_driver_set_dist(cn, 10, 0, 40)` (`area2.c:466`, on
//!   `NT_CREATE`) is not ported, same precedent as every sibling
//!   single-victim NPC.
//! - The `friend` candidate for `do_bless` (`area2.c:473-476`) is computed
//!   fresh from the current tick's `NT_CHAR` messages exactly like C's own
//!   local variable (never persisted across ticks) via
//!   `World::simple_baddy_can_bless_friend` (the same predicate
//!   `CDR_SIMPLEBADDY`'s own friend-bless machinery already uses),
//!   instead of re-deriving each of C's five individual conditions by
//!   hand.
//! - `take_driver(cn, dat->want_it)` (`area2.c:534-536`) mirrors
//!   `World::janitor_take_item`'s walk-then-pick-up shape (adjacent ->
//!   `do_take`, else `setup_walk_toward`) rather than sharing code with
//!   it, since `take_driver` is not itself a shared primitive anywhere in
//!   this codebase (see `world::janitor`'s own module doc comment).

use crate::world::*;

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_MOONIE`
    /// characters (C `ch_driver`'s `CDR_MOONIE` case, `area2.c:991-993`).
    pub fn process_moonie_actions(&mut self, area_id: u16) -> usize {
        let moonie_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_MOONIE
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for moonie_id in moonie_ids {
            if self.process_moonie_tick(moonie_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `moonie_driver`'s per-tick body (`area2.c:449-567`).
    fn process_moonie_tick(&mut self, moonie_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&moonie_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Moonie(data)) => data,
            _ => MoonieDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&moonie_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        // C's local `friend` variable (`area2.c:452`): recomputed fresh
        // every tick, never persisted - see module doc comment.
        let mut friend: Option<CharacterId> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR if message.dat1 > 0 => {
                    let seen_id = CharacterId(message.dat1 as u32);
                    // C `area2.c:473-476`: candidate ally to bless.
                    if self.simple_baddy_can_bless_friend(moonie_id, seen_id) {
                        friend = Some(seen_id);
                    }
                    // C `standard_message_driver`'s `NT_CHAR` branch
                    // (`aggressive=1`): any valid enemy becomes the
                    // tracked victim.
                    if self.moonie_is_valid_enemy(moonie_id, seen_id) {
                        data.victim = Some(seen_id);
                    }
                }
                // C `area2.c:479-490`.
                NT_ITEM => {
                    let item_id = ItemId(message.dat1.max(0) as u32);
                    let is_spider = self
                        .characters
                        .get(&moonie_id)
                        .zip(self.items.get(&item_id))
                        .is_some_and(|(moonie, item)| {
                            item.template_id == IID_AREA2_SMALLSPIDER
                                && char_see_item(moonie, item, &self.map, self.date.daylight)
                        });
                    if is_spider && data.want_it.is_none() && data.yummy == 0 {
                        self.npc_say(moonie_id, "Ohhh. Yummy spider! Want it! Want it!");
                        data.want_it = Some(item_id);
                    }
                }
                // C `area2.c:492-498`, then falls through to
                // `standard_message_driver`'s `NT_GOTHIT` defend branch.
                NT_GOTHIT if message.dat1 > 0 => {
                    if data.yummy != 0 || data.want_it.is_some() {
                        self.npc_say(moonie_id, "Ouch.");
                    }
                    data.yummy = 0;
                    data.want_it = None;

                    let attacker_id = CharacterId(message.dat1 as u32);
                    if self.moonie_is_valid_enemy(moonie_id, attacker_id) {
                        data.victim = Some(attacker_id);
                    }
                }
                _ => {}
            }
        }

        // C `area2.c:507-515`.
        if let Some(item_id) = self
            .characters
            .get(&moonie_id)
            .and_then(|moonie| moonie.cursor_item)
        {
            let is_spider = self
                .items
                .get(&item_id)
                .is_some_and(|item| item.template_id == IID_AREA2_SMALLSPIDER);
            if is_spider {
                self.npc_say(moonie_id, "Such a nice, yummy spider! Ahh. Mmmmh.");
                data.yummy = self.tick.0 + TICKS_PER_SECOND * 60;
                data.lastmunch = self.tick.0;
            }
            self.destroy_item(item_id);
            if let Some(moonie) = self.characters.get_mut(&moonie_id) {
                moonie.cursor_item = None;
            }
        }

        // C `area2.c:517-526`.
        if data.yummy > self.tick.0 {
            if self.tick.0 > data.lastmunch + TICKS_PER_SECOND * 10 {
                self.npc_say(moonie_id, "Munch, munch. Ohhh, so good!");
                data.lastmunch = self.tick.0;
            }
            if let Some(character) = self.characters.get_mut(&moonie_id) {
                character.driver_state = Some(CharacterDriverState::Moonie(data));
            }
            return self
                .characters
                .get_mut(&moonie_id)
                .is_some_and(|character| do_idle(character, TICKS_PER_SECOND as i32).is_ok());
        }
        data.yummy = 0;

        // C `area2.c:528-531`.
        if let Some(item_id) = data.want_it {
            let still_visible = self
                .characters
                .get(&moonie_id)
                .zip(self.items.get(&item_id))
                .is_some_and(|(moonie, item)| {
                    char_see_item(moonie, item, &self.map, self.date.daylight)
                });
            if !still_visible {
                self.npc_say(moonie_id, "Oh. Gone.");
                data.want_it = None;
            }
        }

        // C `fight_driver_update(cn)`.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&moonie_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((moonie, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&moonie, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&moonie_id) {
            character.driver_state = Some(CharacterDriverState::Moonie(data));
        }

        // C `area2.c:533-537`.
        if let Some(item_id) = data.want_it {
            if self.moonie_take_item(moonie_id, item_id, area_id) {
                return true;
            }
        }

        // C `area2.c:541-546`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(moonie_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            let arrived = self.characters.get(&moonie_id).is_some_and(|moonie| {
                moonie.x.abs_diff(data.victim_last_x) < 2
                    && moonie.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Moonie(state)) = self
                    .characters
                    .get_mut(&moonie_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                moonie_id,
                data.victim_last_x,
                data.victim_last_y,
                Direction::Left as u8,
                0,
                0,
                area_id,
            ) {
                return true;
            }
        }

        // C `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret,
        // lastact)` (`area2.c:548-550`).
        let (post_x, post_y) = self
            .characters
            .get(&moonie_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            moonie_id,
            post_x,
            post_y,
            Direction::Left as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `area2.c:552-558`.
        if self.regenerate_simple_baddy(moonie_id) {
            return true;
        }
        if self.spell_self_simple_baddy(moonie_id) {
            return true;
        }

        // C `if (friend && do_bless(cn, friend)) return;` (`area2.c:561-563`).
        if let Some(friend_id) = friend {
            if self.setup_bless_spell(moonie_id, friend_id) {
                return true;
            }
        }

        // C `do_idle(cn, TICKS);` (`area2.c:566`).
        self.idle_simple_baddy(moonie_id)
    }

    /// C `is_valid_enemy(cn, co, -1)` (`drvlib.c:897-927`).
    fn moonie_is_valid_enemy(&self, character_id: CharacterId, target_id: CharacterId) -> bool {
        if character_id == target_id {
            return false;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        character.group != target.group
            && can_attack(character, target, &self.map)
            && char_see_char(character, target, &self.map, self.date.daylight)
    }

    /// C `take_driver(cn, dat->want_it)`: walk to the item if not already
    /// adjacent, then pick it up. See module doc comment.
    fn moonie_take_item(&mut self, moonie_id: CharacterId, item_id: ItemId, area_id: u16) -> bool {
        let Some(item) = self.items.get(&item_id).cloned() else {
            return false;
        };
        let Some(moonie) = self.characters.get(&moonie_id) else {
            return false;
        };
        let direction =
            adjacent_direction(moonie.x, moonie.y, usize::from(item.x), usize::from(item.y));
        if let Some(direction) = direction {
            let Some(moonie) = self.characters.get_mut(&moonie_id) else {
                return false;
            };
            do_take(
                moonie,
                &self.map,
                &item,
                direction as u8,
                true,
                self.settings.weather_movement_percent,
            )
            .is_ok()
        } else {
            self.setup_walk_toward(
                moonie_id,
                usize::from(item.x),
                usize::from(item.y),
                1,
                area_id,
                false,
            )
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_MOONIE;
use crate::item_driver::IID_AREA2_SMALLSPIDER;

/// C `struct moonie_data` (`area2.c:443-447`), plus this port's own
/// single-victim self-defense tracking (see module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoonieDriverData {
    pub want_it: Option<ItemId>,
    pub yummy: u64,
    pub lastmunch: u64,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
