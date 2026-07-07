//! Sanoa city-walker NPC (`CDR_SANOA`).
//!
//! Ports `src/area/1/gwendylon.c::sanoa_driver` (`:3961-4094`): a purely
//! ambient city NPC with no dialogue at all - just a twelve-waypoint
//! daily walk (post -> library -> south loop -> back to post) gated by a
//! handful of fixed departure times, plus the same self-defense/spell/
//! regeneration cascade every simple grunt NPC in this file runs.
//! `ch_died_driver`'s `CDR_SANOA` case dispatches to `balltrap_skelly_
//! dead` (`gwendylon.c:6186-6188`), itself an empty no-op
//! (`:5197-5199`) - no death reward/hook needed for this NPC.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification already established
//!   for `CDR_ROBBER`/`CDR_GATE_FIGHT`: C's generic `fight_driver_update`/
//!   `fight_driver_attack_visible`/`fight_driver_follow_invisible` cascade
//!   (backed by the 10-slot `struct fight_driver_data`) is narrowed to a
//!   single tracked `victim`, set from the unconditional `NT_GOTHIT`
//!   message (see `World::apply_legacy_hurt`). See `world/npc/area1/
//!   robber.rs`'s own module doc comment for the full justification; this
//!   port is structurally identical to that one's `NT_GOTHIT` handling
//!   (both drivers call `standard_message_driver(cn, msg, 0, 0)`, which
//!   with `agressive=0`/`helper=0` only ever does anything on `NT_GOTHIT`).
//! - `fight_driver_set_dist(cn, 20, 0, 40)` (`gwendylon.c:3975`, on
//!   `NT_CREATE`) configures the generic engine's distance-from-home
//!   enemy-admission gate; this port's single-victim model has no
//!   equivalent gate, same precedent as `robber.rs`.
//! - `fight_driver_set_home(cn, ch[cn].x, ch[cn].y)` (`gwendylon.c:4002`,
//!   every tick) is likewise not ported - same precedent as `robber.rs`
//!   (C's `home` is reset to Sanoa's own *current* position every tick
//!   anyway, so the gate would almost always pass trivially even in C).
//! - `charlog(cn, "my ...")` calls do not exist in this driver (unlike
//!   `robber_driver`'s ladder/hole waypoints) - Sanoa's door waypoint has
//!   no missing-item fallback branch in the C source at all.

use crate::world::*;

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_SANOA`
    /// characters (C `ch_driver`'s `CDR_SANOA` case, `gwendylon.c:6102-
    /// 6104`).
    pub fn process_sanoa_actions(&mut self, area_id: u16) -> usize {
        let sanoa_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SANOA
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for sanoa_id in sanoa_ids {
            if self.process_sanoa_tick(sanoa_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `sanoa_driver`'s per-tick body (`gwendylon.c:3961-4094`).
    fn process_sanoa_tick(&mut self, sanoa_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&sanoa_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Sanoa(data)) => data,
            _ => SanoaDriverData::default(),
        };

        // C's message loop (`standard_message_driver(cn, msg, 0, 0)`),
        // narrowed to the `NT_GOTHIT` self-defense branch - see module
        // doc comment.
        let messages = self
            .characters
            .get_mut(&sanoa_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();
        for message in &messages {
            if message.message_type != NT_GOTHIT || message.dat1 <= 0 {
                continue;
            }
            let attacker_id = CharacterId(message.dat1 as u32);
            if let Some((sanoa, attacker)) = self
                .characters
                .get(&sanoa_id)
                .cloned()
                .zip(self.characters.get(&attacker_id).cloned())
            {
                // C `if (ch[cn].group == ch[co].group) break; if
                // (!can_attack(cn,co)) break; fight_driver_add_enemy(cn,
                // co, 1, 1);`.
                if sanoa.group != attacker.group && can_attack(&sanoa, &attacker, &self.map) {
                    data.victim = Some(attacker_id);
                }
            }
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&sanoa_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((sanoa, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&sanoa, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&sanoa_id) {
            character.driver_state = Some(CharacterDriverState::Sanoa(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(sanoa_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`: walk
            // toward the last known position; give up once close enough
            // without finding him there.
            let arrived = self.characters.get(&sanoa_id).is_some_and(|sanoa| {
                sanoa.x.abs_diff(data.victim_last_x) < 2 && sanoa.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Sanoa(state)) = self
                    .characters
                    .get_mut(&sanoa_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                sanoa_id,
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
        if self.spell_self_simple_baddy(sanoa_id) {
            return true;
        }
        // C `if (regenerate_driver(cn)) return;`.
        if self.regenerate_simple_baddy(sanoa_id) {
            return true;
        }

        // C `fight_driver_set_home(cn, ch[cn].x, ch[cn].y)` intentionally
        // not ported - see module doc comment.

        let mut state = match self
            .characters
            .get(&sanoa_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Sanoa(data)) => data,
            _ => SanoaDriverData::default(),
        };

        let hour = self.date.hour;
        let minute = self.date.minute;

        // C's twelve-state city walk (`gwendylon.c:4005-4091`), verbatim.
        let acted = match state.state {
            0 => {
                if self.secure_move_driver(sanoa_id, 16, 31, Direction::Right as u8, 0, 0, area_id)
                {
                    true
                } else {
                    if (hour == 7 && minute < 30)
                        || (hour == 10 && minute < 30)
                        || (hour == 12 && minute < 30)
                        || (hour == 15 && minute < 30)
                        || (hour == 18 && minute < 30)
                    {
                        state.state = 1;
                    }
                    false
                }
            }
            1 => {
                if self.setup_walk_toward(sanoa_id, 21, 31, 1, area_id, false) {
                    true
                } else {
                    state.state = 2;
                    false
                }
            }
            2 => {
                if self.setup_walk_toward(sanoa_id, 21, 25, 0, area_id, false) {
                    true
                } else {
                    state.state = 3;
                    false
                }
            }
            3 => {
                if !self.sanoa_door_is_closed(21, 26)
                    && self.sanoa_use_item_at(sanoa_id, 21, 26, area_id)
                {
                    true
                } else {
                    state.state = 4;
                    false
                }
            }
            4 => {
                if self.setup_walk_toward(sanoa_id, 21, 23, 1, area_id, false) {
                    true
                } else {
                    state.state = 5;
                    false
                }
            }
            5 => {
                if self.setup_walk_toward(sanoa_id, 69, 23, 1, area_id, false) {
                    true
                } else {
                    state.state = 6;
                    false
                }
            }
            6 => {
                if self.setup_walk_toward(sanoa_id, 69, 42, 1, area_id, false) {
                    true
                } else {
                    if minute > 30 {
                        state.state = 7;
                    }
                    false
                }
            }
            7 => {
                if self.setup_walk_toward(sanoa_id, 69, 23, 1, area_id, false) {
                    true
                } else {
                    state.state = 8;
                    false
                }
            }
            8 => {
                if self.setup_walk_toward(sanoa_id, 21, 23, 1, area_id, false) {
                    true
                } else {
                    state.state = 9;
                    false
                }
            }
            9 => {
                if self.setup_walk_toward(sanoa_id, 21, 27, 0, area_id, false) {
                    true
                } else {
                    state.state = 10;
                    false
                }
            }
            10 => {
                if !self.sanoa_door_is_closed(21, 26)
                    && self.sanoa_use_item_at(sanoa_id, 21, 26, area_id)
                {
                    true
                } else {
                    state.state = 11;
                    false
                }
            }
            11 => {
                if self.setup_walk_toward(sanoa_id, 21, 31, 1, area_id, false) {
                    true
                } else {
                    state.state = 0;
                    false
                }
            }
            _ => false,
        };

        if let Some(character) = self.characters.get_mut(&sanoa_id) {
            character.driver_state = Some(CharacterDriverState::Sanoa(state));
        }

        if acted {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`gwendylon.c:4093`).
        self.idle_simple_baddy(sanoa_id)
    }

    /// C `is_closed(x, y)` (`drvlib.c:2543-2557`): `true` only for a door
    /// item (`IDR_DOOR`) whose `drdata[0]` (open flag) is unset.
    fn sanoa_door_is_closed(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        let Some(tile) = self.map.tile(x as usize, y as usize) else {
            return false;
        };
        if tile.item == 0 {
            return false;
        }
        let Some(item) = self.items.get(&ItemId(tile.item)) else {
            return false;
        };
        item.driver == IDR_DOOR && !door_open_state(item)
    }

    /// C `use_item_at(cn, x, y, spec)` (`drvlib.c:2581-2601`): first tries
    /// `use_driver` directly (here: toggle the door item at the tile
    /// in-place), then falls back to pathing adjacent (`mindist` 1) and
    /// walking/using. Same shape as `World::bank_use_item_at`
    /// (`world/npc/bank.rs`).
    fn sanoa_use_item_at(&mut self, sanoa_id: CharacterId, x: i32, y: i32, area_id: u16) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        let (ux, uy) = (x as usize, y as usize);
        let item_id = self.map.tile(ux, uy).map(|tile| tile.item).unwrap_or(0);
        if item_id != 0
            && self
                .items
                .get(&ItemId(item_id))
                .is_some_and(|item| item.driver == IDR_DOOR)
            && matches!(
                self.toggle_door(ItemId(item_id), sanoa_id),
                DoorToggleResult::Toggled
            )
        {
            return true;
        }
        self.setup_walk_toward(sanoa_id, ux, uy, 1, area_id, false)
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_SANOA;

/// C `struct sanoa_driver_data` (`src/area/1/gwendylon.c:3957-3959`): the
/// walking-route state, plus this port's own single-victim self-defense
/// tracking (see module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SanoaDriverData {
    pub state: i32,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
