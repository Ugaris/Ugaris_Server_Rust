//! Robber patrol/self-defense NPC (`CDR_ROBBER`).
//!
//! Ports `src/area/1/gwendylon.c::robber_driver` (`:3771-3953`): a forest
//! grunt that stands guard near a hidden ladder most of the day, then
//! (once the in-game clock passes `23:45`) walks a fixed nine-waypoint
//! route down into a ladder, across, and through a hole to a midnight
//! "meeting" and back, waiting there until past `00:15` before returning.
//! `ch_died_driver`'s `CDR_ROBBER` case dispatches to `balltrap_skelly_
//! dead` (`gwendylon.c:6183-6185`), itself an empty no-op
//! (`:5197-5199`) - no death reward/hook needed for this NPC.
//!
//! Deviations/gaps (documented, not silent):
//! - C's self-defense cascade (`fight_driver_update`/`fight_driver_
//!   attack_visible`/`fight_driver_follow_invisible`) is backed by the
//!   fully generic 10-slot `struct fight_driver_data` (`drvlib.c:2170-
//!   2345`). This port tracks only the single most-recent attacker as
//!   `victim` (set from the `NT_GOTHIT` message any hurt character
//!   already receives unconditionally - see `World::apply_legacy_hurt`),
//!   the same single-enemy simplification already established for
//!   `CDR_GATE_FIGHT` (`world/npc/gate_fight.rs`'s own module doc
//!   comment). "Attack visible" reuses the already-generic
//!   `World::attack_driver_direct`; "follow invisible" reuses
//!   `secure_move_driver` toward the last known position. The C
//!   `NT_GOTHIT` handler's `if (ch[cn].group == ch[co].group) break;`
//!   guard is ported verbatim (`robber_group_matches_default_gate`
//!   inlined below); since neither this NPC's zone template nor a
//!   default player sets a non-zero `group`, this gate is almost always
//!   true in practice, matching C's own effectively-inert self-defense
//!   path for ungrouped characters (not a bug in this port).
//! - `fight_driver_set_dist(cn, 20, 0, 40)` (`gwendylon.c:3790`, on
//!   `NT_CREATE`) and the every-tick `fight_driver_set_home` call
//!   (`gwendylon.c:3830`) configure the generic engine's distance-from-
//!   home enemy-admission gate; this port's single-victim model has no
//!   equivalent gate (a direct attacker is always a valid victim
//!   candidate) - never observably different in practice, since C's
//!   `home` is reset to the robber's own *current* position every tick
//!   anyway (so the gate would almost always pass trivially even in C).
//! - `charlog(cn, "my ladder/hole is gone!")` (`gwendylon.c:3852,3941`) is
//!   a server-logfile write only (`src/system/logging/log.c::charlog`),
//!   never observable to any client; only the accompanying `dat->state =
//!   0` reset (the actually observable behavior) is ported.

use crate::world::*;

/// C `ch[cn].item[WN_LHAND]` slot index (`worn_slot_by_name`'s `NAMES`
/// array position in `crate::zone`, 0-based: `WN_NECK, WN_HEAD, WN_CLOAK,
/// WN_ARMS, WN_BODY, WN_BELT, WN_RHAND, WN_LEGS, WN_LHAND, ...`).
const ROBBER_TORCH_SLOT: usize = 8;

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_ROBBER`
    /// characters (C `ch_driver`'s `CDR_ROBBER` case, `gwendylon.c:6099-
    /// 6100`).
    pub fn process_robber_actions(&mut self, zone_loader: &mut ZoneLoader, area_id: u16) -> usize {
        let robber_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ROBBER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for robber_id in robber_ids {
            if self.process_robber_tick(robber_id, zone_loader, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `robber_driver`'s per-tick body (`gwendylon.c:3775-3953`).
    fn process_robber_tick(
        &mut self,
        robber_id: CharacterId,
        zone_loader: &mut ZoneLoader,
        area_id: u16,
    ) -> bool {
        let mut data = match self
            .characters
            .get(&robber_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Robber(data)) => data,
            _ => RobberDriverData::default(),
        };

        // C's message loop (`standard_message_driver(cn, msg, 0, 0)`),
        // narrowed to the `NT_GOTHIT` self-defense branch (see module doc
        // comment for the single-victim simplification and the group-
        // equality gate).
        let messages = self
            .characters
            .get_mut(&robber_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();
        for message in &messages {
            if message.message_type != NT_GOTHIT || message.dat1 <= 0 {
                continue;
            }
            let attacker_id = CharacterId(message.dat1 as u32);
            if let Some((robber, attacker)) = self
                .characters
                .get(&robber_id)
                .cloned()
                .zip(self.characters.get(&attacker_id).cloned())
            {
                // C `if (ch[cn].group == ch[co].group) break; if
                // (!can_attack(cn,co)) break; fight_driver_add_enemy(cn,
                // co, 1, 1);`.
                if robber.group != attacker.group && can_attack(&robber, &attacker, &self.map) {
                    data.victim = Some(attacker_id);
                }
            }
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&robber_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((robber, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&robber, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&robber_id) {
            character.driver_state = Some(CharacterDriverState::Robber(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(robber_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`: walk
            // toward the last known position; give up once close enough
            // without finding him there.
            let arrived = self.characters.get(&robber_id).is_some_and(|robber| {
                robber.x.abs_diff(data.victim_last_x) < 2
                    && robber.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Robber(state)) = self
                    .characters
                    .get_mut(&robber_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                robber_id,
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
        if self.spell_self_simple_baddy(robber_id) {
            return true;
        }
        // C `if (regenerate_driver(cn)) return;`.
        if self.regenerate_simple_baddy(robber_id) {
            return true;
        }

        // C: torch upkeep (`gwendylon.c:3817-3828`) - keep a lit torch in
        // `WN_LHAND` at all times.
        self.robber_maintain_torch(robber_id, zone_loader, area_id);

        // C `fight_driver_set_home(cn, ch[cn].x, ch[cn].y)` intentionally
        // not ported - see module doc comment.

        let mut state = match self
            .characters
            .get(&robber_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Robber(data)) => data,
            _ => RobberDriverData::default(),
        };

        let hour = self.date.hour;
        let minute = self.date.minute;

        // C's nine-waypoint walk (`gwendylon.c:3833-3949`), verbatim.
        let acted = match state.state {
            0 => {
                if self.secure_move_driver(robber_id, 30, 242, Direction::Up as u8, 0, 0, area_id) {
                    true
                } else {
                    if hour == 23 && minute > 45 {
                        state.state = 1;
                    }
                    false
                }
            }
            1 => {
                if self.setup_walk_toward(robber_id, 30, 237, 1, area_id, false) {
                    true
                } else {
                    state.state = 2;
                    false
                }
            }
            2 => {
                let item_id = self.map.tile(31, 237).map(|tile| tile.item).unwrap_or(0);
                if item_id == 0 {
                    // C `charlog(cn, "my ladder is gone!")`: server-log
                    // only, no client-visible effect - see module doc.
                    state.state = 0;
                    false
                } else if self.robber_use_waypoint_item(robber_id, ItemId(item_id), area_id) {
                    true
                } else {
                    state.state = 3;
                    false
                }
            }
            3 => {
                if self.setup_walk_toward(robber_id, 222, 78, 1, area_id, false) {
                    true
                } else {
                    state.state = 4;
                    false
                }
            }
            4 => {
                if self.setup_walk_toward(robber_id, 190, 78, 1, area_id, false) {
                    true
                } else {
                    state.state = 5;
                    false
                }
            }
            5 => {
                if self.setup_walk_toward(robber_id, 173, 78, 1, area_id, false) {
                    true
                } else {
                    state.state = 6;
                    false
                }
            }
            6 => {
                if self.setup_walk_toward(robber_id, 173, 54, 1, area_id, false) {
                    true
                } else {
                    state.state = 7;
                    false
                }
            }
            7 => {
                if self.setup_walk_toward(robber_id, 145, 54, 1, area_id, false) {
                    true
                } else {
                    state.state = 8;
                    false
                }
            }
            8 => {
                if self.setup_walk_toward(robber_id, 145, 72, 1, area_id, false) {
                    true
                } else {
                    if self
                        .characters
                        .get(&robber_id)
                        .is_some_and(|robber| robber.dir != Direction::Up as u8)
                    {
                        if let Some(robber) = self.characters.get_mut(&robber_id) {
                            let _ = turn(robber, Direction::Up as u8);
                        }
                    }
                    if (hour > 0 || minute > 15) && hour != 23 {
                        state.state = 9;
                    }
                    false
                }
            }
            9 => {
                if self.setup_walk_toward(robber_id, 145, 72, 1, area_id, false) {
                    true
                } else {
                    state.state = 10;
                    false
                }
            }
            10 => {
                if self.setup_walk_toward(robber_id, 145, 54, 1, area_id, false) {
                    true
                } else {
                    state.state = 11;
                    false
                }
            }
            11 => {
                if self.setup_walk_toward(robber_id, 173, 54, 1, area_id, false) {
                    true
                } else {
                    state.state = 12;
                    false
                }
            }
            12 => {
                if self.setup_walk_toward(robber_id, 173, 78, 1, area_id, false) {
                    true
                } else {
                    state.state = 13;
                    false
                }
            }
            13 => {
                if self.setup_walk_toward(robber_id, 190, 78, 1, area_id, false) {
                    true
                } else {
                    state.state = 14;
                    false
                }
            }
            14 => {
                if self.setup_walk_toward(robber_id, 222, 78, 1, area_id, false) {
                    true
                } else {
                    state.state = 15;
                    false
                }
            }
            15 => {
                let item_id = self.map.tile(244, 78).map(|tile| tile.item).unwrap_or(0);
                if item_id == 0 {
                    // C `charlog(cn, "my hole is gone!")`: server-log
                    // only, no client-visible effect - see module doc.
                    state.state = 0;
                    false
                } else if self.robber_use_waypoint_item(robber_id, ItemId(item_id), area_id) {
                    true
                } else {
                    state.state = 0;
                    false
                }
            }
            _ => false,
        };

        if let Some(character) = self.characters.get_mut(&robber_id) {
            character.driver_state = Some(CharacterDriverState::Robber(state));
        }

        if acted {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`gwendylon.c:3952`).
        self.idle_simple_baddy(robber_id)
    }

    /// C `use_driver(cn, in, 0)` for a fixed-position waypoint item (the
    /// ladder/hole triggers at states 2/15): walk to it if not already
    /// adjacent, then use it. Identical shape to `World::janitor_
    /// use_light` (`world/npc/janitor.rs`).
    fn robber_use_waypoint_item(
        &mut self,
        robber_id: CharacterId,
        item_id: ItemId,
        area_id: u16,
    ) -> bool {
        let Some(item) = self.items.get(&item_id).cloned() else {
            return false;
        };
        if !item.flags.contains(ItemFlags::USE) {
            return false;
        }
        let Some(robber) = self.characters.get(&robber_id) else {
            return false;
        };
        let direction = adjacent_use_direction(
            robber.x,
            robber.y,
            usize::from(item.x),
            usize::from(item.y),
            item.flags.contains(ItemFlags::FRONTWALL),
        );
        if let Some(direction) = direction {
            let Some(robber) = self.characters.get_mut(&robber_id) else {
                return false;
            };
            do_use(
                robber,
                &self.map,
                &item,
                direction as u8,
                0,
                self.settings.weather_movement_percent,
            )
            .is_ok()
        } else {
            self.setup_walk_toward_use_item(
                robber_id,
                usize::from(item.x),
                usize::from(item.y),
                item.flags,
                area_id,
            )
        }
    }

    /// C `gwendylon.c:3817-3828`:
    /// ```c
    /// in = ch[cn].item[WN_LHAND];
    /// if (!in) {
    ///     in = create_item("torch");
    ///     it[in].carried = cn;
    ///     ch[cn].item[WN_LHAND] = in;
    ///     update_char(cn);
    /// } else {
    ///     if (!it[in].drdata[0]) {
    ///         use_item(cn, in);
    ///     }
    /// }
    /// ```
    /// `use_item` dispatches the item driver synchronously (unlike
    /// `use_driver`, which queues a timed action) - ported via
    /// `World::execute_item_driver_request`, the same synchronous-
    /// dispatch entry point `process_simple_baddy_message_actions_
    /// with_random` already uses for inventory potions.
    fn robber_maintain_torch(
        &mut self,
        robber_id: CharacterId,
        zone_loader: &mut ZoneLoader,
        area_id: u16,
    ) {
        let Some(torch_slot) = self
            .characters
            .get(&robber_id)
            .and_then(|robber| robber.inventory.get(ROBBER_TORCH_SLOT).copied())
        else {
            return;
        };

        match torch_slot {
            None => {
                let Ok(item) = zone_loader.instantiate_item_template("torch", Some(robber_id))
                else {
                    return;
                };
                let item_id = item.id;
                self.items.insert(item_id, item);
                if let Some(robber) = self.characters.get_mut(&robber_id) {
                    robber.inventory[ROBBER_TORCH_SLOT] = Some(item_id);
                }
                self.update_character(robber_id);
            }
            Some(item_id) => {
                let lit = self
                    .items
                    .get(&item_id)
                    .is_some_and(|item| item.driver_data.first().copied().unwrap_or(0) != 0);
                if !lit {
                    let _ = self.execute_item_driver_request(
                        ItemDriverRequest::Driver {
                            driver: IDR_TORCH,
                            item_id,
                            character_id: robber_id,
                            spec: 0,
                        },
                        area_id,
                    );
                }
            }
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_ROBBER;

/// C `struct robber_driver_data` (`src/area/1/gwendylon.c:3771-3773`):
/// the walking-route state, plus this port's own single-victim self-
/// defense tracking (see module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RobberDriverData {
    pub state: i32,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
