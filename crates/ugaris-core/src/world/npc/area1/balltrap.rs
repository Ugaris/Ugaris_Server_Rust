//! Balltrap-skeleton fight NPC (`CDR_BALLTRAP`).
//!
//! Ports `src/area/1/gwendylon.c::balltrap_skelly_driver` (`:3708-3767`): a
//! stationary skeleton "guard" for a ball-trap mechanism (the `balltrap`
//! item driver, `src/module/base.c::balltrap`, already ported at
//! `crate::item_driver::traps::balltrap_driver`) - it holds its post,
//! fights back if attacked, and every three seconds fires the trap item
//! sitting immediately to its left (`do_use(cn, DX_LEFT, 0)`) regardless of
//! whether anyone is actually there to hit. `ch_died_driver`'s
//! `CDR_BALLTRAP` case dispatches to `balltrap_skelly_dead`
//! (`gwendylon.c:6183-6185`), itself an empty no-op (`:5197-5199`) - no
//! death reward/hook needed for this NPC.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification already established
//!   for `CDR_ROBBER`/`CDR_SANOA`/`CDR_GATE_FIGHT`: C's generic
//!   `fight_driver_update`/`fight_driver_attack_visible`/
//!   `fight_driver_follow_invisible` cascade (backed by the 10-slot
//!   `struct fight_driver_data`) is narrowed to a single tracked `victim`,
//!   set from the unconditional `NT_GOTHIT` message (see
//!   `World::apply_legacy_hurt`). See `world/npc/area1/robber.rs`'s own
//!   module doc comment for the full justification.
//! - `fight_driver_set_dist(cn, 20, 0, 40)` (`gwendylon.c:3726`, on
//!   `NT_CREATE`) configures the generic engine's distance-from-home
//!   enemy-admission gate; this port's single-victim model has no
//!   equivalent gate, same precedent as `robber.rs`/`sanoa.rs`.
//! - `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret,
//!   lastact)` (`gwendylon.c:3753`) reuses `rest_x`/`rest_y` for
//!   `tmpx`/`tmpy`, the same substitution every sibling area-1 NPC's own
//!   module doc comment already documents.
//! - `do_use(cn, DX_LEFT, 0)` (`gwendylon.c:3760`) fires whatever item sits
//!   directly to this NPC's left, unconditionally on the timer; C's
//!   `do_use` itself no-ops (returns 0, falls through to `do_idle`) if
//!   there is no usable item there (confirmed against the live zone data:
//!   two of this NPC's three placements in `above1.map` have no
//!   `balltrap` item adjacent), so this is not a bug in either the C
//!   source or this port.

use crate::world::*;

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_BALLTRAP`
    /// characters (C `ch_driver`'s `CDR_BALLTRAP` case, `gwendylon.c:6084-
    /// 6086`).
    pub fn process_balltrap_actions(&mut self, area_id: u16) -> usize {
        let balltrap_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_BALLTRAP
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for balltrap_id in balltrap_ids {
            if self.process_balltrap_tick(balltrap_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `balltrap_skelly_driver`'s per-tick body (`gwendylon.c:3712-3767`).
    fn process_balltrap_tick(&mut self, balltrap_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&balltrap_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Balltrap(data)) => data,
            _ => BalltrapDriverData::default(),
        };

        // C's message loop (`standard_message_driver(cn, msg, 0, 0)`),
        // narrowed to the `NT_GOTHIT` self-defense branch - see module
        // doc comment.
        let messages = self
            .characters
            .get_mut(&balltrap_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();
        for message in &messages {
            if message.message_type != NT_GOTHIT || message.dat1 <= 0 {
                continue;
            }
            let attacker_id = CharacterId(message.dat1 as u32);
            if let Some((balltrap, attacker)) = self
                .characters
                .get(&balltrap_id)
                .cloned()
                .zip(self.characters.get(&attacker_id).cloned())
            {
                // C `if (ch[cn].group == ch[co].group) break; if
                // (!can_attack(cn,co)) break; fight_driver_add_enemy(cn,
                // co, 1, 1);`.
                if balltrap.group != attacker.group && can_attack(&balltrap, &attacker, &self.map) {
                    data.victim = Some(attacker_id);
                }
            }
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&balltrap_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((balltrap, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&balltrap, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&balltrap_id) {
            character.driver_state = Some(CharacterDriverState::Balltrap(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(balltrap_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`: walk
            // toward the last known position; give up once close enough
            // without finding him there.
            let arrived = self.characters.get(&balltrap_id).is_some_and(|balltrap| {
                balltrap.x.abs_diff(data.victim_last_x) < 2
                    && balltrap.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Balltrap(state)) = self
                    .characters
                    .get_mut(&balltrap_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                balltrap_id,
                data.victim_last_x,
                data.victim_last_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            ) {
                return true;
            }
        }

        // C `if (spell_self_driver(cn)) return;`.
        if self.spell_self_simple_baddy(balltrap_id) {
            return true;
        }
        // C `if (regenerate_driver(cn)) return;`.
        if self.regenerate_simple_baddy(balltrap_id) {
            return true;
        }

        // C `if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT,
        // ret, lastact)) return;` (`gwendylon.c:3753-3755`): return to the
        // post position (`rest_x`/`rest_y` substitution - see module doc).
        let (post_x, post_y) = self
            .characters
            .get(&balltrap_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            balltrap_id,
            post_x,
            post_y,
            Direction::Left as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `if (ticker > dat->last_fire + TICKS * 3) { dat->last_fire =
        // ticker; if (do_use(cn, DX_LEFT, 0)) return; }`
        // (`gwendylon.c:3757-3763`).
        let mut state = match self
            .characters
            .get(&balltrap_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Balltrap(data)) => data,
            _ => BalltrapDriverData::default(),
        };
        let mut fired = false;
        if self.tick.0 > state.last_fire + TICKS_PER_SECOND * 3 {
            state.last_fire = self.tick.0;
            fired = self.balltrap_fire_left(balltrap_id);
        }
        if let Some(character) = self.characters.get_mut(&balltrap_id) {
            character.driver_state = Some(CharacterDriverState::Balltrap(state));
        }
        if fired {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`gwendylon.c:3766`).
        self.idle_simple_baddy(balltrap_id)
    }

    /// C `do_use(cn, DX_LEFT, 0)` (`gwendylon.c:3760`): trigger whatever
    /// usable item sits directly to this NPC's left (the `balltrap` trap
    /// mechanism, when present - see module doc comment).
    fn balltrap_fire_left(&mut self, balltrap_id: CharacterId) -> bool {
        let Some(balltrap) = self.characters.get(&balltrap_id).cloned() else {
            return false;
        };
        let (dx, dy) = Direction::Left.delta();
        let Some(x) = offset_coordinate(usize::from(balltrap.x), dx) else {
            return false;
        };
        let Some(y) = offset_coordinate(usize::from(balltrap.y), dy) else {
            return false;
        };
        let item_id = self.map.tile(x, y).map(|tile| tile.item).unwrap_or(0);
        if item_id == 0 {
            return false;
        }
        let Some(item) = self.items.get(&ItemId(item_id)).cloned() else {
            return false;
        };
        if !item.flags.contains(ItemFlags::USE) {
            return false;
        }
        let Some(character) = self.characters.get_mut(&balltrap_id) else {
            return false;
        };
        do_use(
            character,
            &self.map,
            &item,
            Direction::Left as u8,
            0,
            self.settings.weather_movement_percent,
        )
        .is_ok()
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_BALLTRAP;

/// C `struct balltrap_skelly_driver_data` (`src/area/1/gwendylon.c:3708-
/// 3710`): the fire-timer, plus this port's own single-victim self-defense
/// tracking (see module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BalltrapDriverData {
    pub last_fire: u64,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
