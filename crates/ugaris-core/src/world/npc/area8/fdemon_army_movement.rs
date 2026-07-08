//! `CDR_FDEMON_ARMY` formation movement: `army_follow_driver`/
//! `army_back_driver`/`army_front_driver`/`army_behind_driver`
//! (`src/area/8/fdemon.c:633-705`). Split out of `fdemon_army.rs` to keep
//! that file within the ~800-line NPC-file guideline (see its own module
//! doc comment for the full driver breakdown and remaining gaps).

use crate::{character_driver::CharacterDriverState, world::*};

use super::fdemon_army::MIS_FOLLOW;

impl World {
    /// C `army_follow_driver(cn, dat, dist)` (`fdemon.c:633-655`): walks
    /// one step toward the leader (`min_dist=2` once the leader is
    /// visible, matching C's fixed `pathfinder(...,2,...)` call - `dist`
    /// only gates the "already close enough, don't move" early-out) or
    /// toward the last-known leader position (`min_dist=0`) when the
    /// leader isn't currently visible. Returns whether a walk action was
    /// queued (C's `return 1`/`return 0`).
    pub fn army_follow_driver(
        &mut self,
        character_id: CharacterId,
        dist: i32,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = character.driver_state.clone() else {
            return false;
        };
        let Some(leader) = self.characters.get(&dat.leader_cn) else {
            return false;
        };

        let daylight = self.date.daylight;
        if char_see_char(character, leader, &self.map, daylight) {
            let (lx, ly) = (leader.x, leader.y);
            let (cx, cy) = (character.x, character.y);
            if let Some(CharacterDriverState::FdemonArmy(dat)) = self
                .characters
                .get_mut(&character_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                dat.lx = lx;
                dat.ly = ly;
            }
            let manhattan =
                (i32::from(cx) - i32::from(lx)).abs() + (i32::from(cy) - i32::from(ly)).abs();
            if manhattan <= dist {
                return false;
            }
            self.setup_walk_toward(
                character_id,
                usize::from(lx),
                usize::from(ly),
                2,
                area_id,
                false,
            )
        } else {
            let (cx, cy) = (character.x, character.y);
            if cx == dat.lx && cy == dat.ly {
                return false;
            }
            self.setup_walk_toward(
                character_id,
                usize::from(dat.lx),
                usize::from(dat.ly),
                0,
                area_id,
                false,
            )
        }
    }

    /// C `army_back_driver(cn, dat)` (`fdemon.c:675-686`): if the soldier
    /// is still standing at the guard post recorded when the "back"
    /// command was issued (`dat->opt1`/`dat->opt2`), take exactly one
    /// step in the direction opposite its current facing (C `(ch[cn].dir
    /// + 3) % 8 + 1`, see [`opposite_direction`]) and return `true`
    /// immediately on success. Otherwise (already moved off the guard
    /// post, or the backward step is blocked) fall back to a timeout:
    /// after 5 seconds with no progress revert the mission to
    /// `MIS_FOLLOW` and return `false`; before that, idle for half a
    /// second and return whether the idle was queued (C `return
    /// do_idle(cn, TICKS/2)`).
    pub fn army_back_driver(&mut self, character_id: CharacterId, area_id: u16) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = character.driver_state.clone() else {
            return false;
        };

        if i32::from(character.x) == dat.opt1 && i32::from(character.y) == dat.opt2 {
            let direction = opposite_direction(character.dir);
            let weather_movement_percent = self.settings.weather_movement_percent;
            let earthmud_extra_cost = self.earthmud_extra_movement_cost(character_id);
            let walked = Direction::try_from(direction).is_ok_and(|direction| {
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| {
                        do_walk(
                            character,
                            &mut self.map,
                            direction as u8,
                            area_id,
                            weather_movement_percent,
                            earthmud_extra_cost,
                        )
                        .is_ok()
                    })
            });
            if walked {
                return true;
            }
        }

        if self.tick.0 as i64 - dat.timer > TICKS_PER_SECOND as i64 * 5 {
            if let Some(CharacterDriverState::FdemonArmy(dat)) = self
                .characters
                .get_mut(&character_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                dat.mission = MIS_FOLLOW;
            }
            false
        } else {
            self.characters
                .get_mut(&character_id)
                .is_some_and(|character| do_idle(character, TICKS_PER_SECOND as i32 / 2).is_ok())
        }
    }

    /// C `army_front_driver(cn, dat, dist)` (`fdemon.c:657-673`): walks
    /// one step toward a point 4 tiles ahead of the leader in its current
    /// facing direction (C `dx2offset(ch[co].dir,...)`, [`Direction::
    /// delta`]-equivalent, times 4, added to the leader's position).
    /// Returns whether a walk action was queued (C `return 1`/`return
    /// 0`): `false` if the leader isn't visible, the soldier is already
    /// within `dist` tiles of the target point, or no path is found.
    pub fn army_front_driver(
        &mut self,
        character_id: CharacterId,
        dist: i32,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = character.driver_state.clone() else {
            return false;
        };
        let Some(leader) = self.characters.get(&dat.leader_cn) else {
            return false;
        };

        let daylight = self.date.daylight;
        if !char_see_char(character, leader, &self.map, daylight) {
            return false;
        }
        let Ok(leader_direction) = Direction::try_from(leader.dir) else {
            return false;
        };
        let (dx, dy) = leader_direction.delta();
        let target_x = i32::from(leader.x) + i32::from(dx) * 4;
        let target_y = i32::from(leader.y) + i32::from(dy) * 4;
        if target_x < 0 || target_y < 0 {
            return false;
        }

        let manhattan =
            (i32::from(character.x) - target_x).abs() + (i32::from(character.y) - target_y).abs();
        if manhattan <= dist {
            return false;
        }

        self.setup_walk_toward(
            character_id,
            target_x as usize,
            target_y as usize,
            2,
            area_id,
            false,
        )
    }

    /// C `army_behind_driver(cn, dat)` (`fdemon.c:688-705`): positions the
    /// soldier directly behind whatever character (`co`) the leader is
    /// currently facing, then attacks it. `co` is found by looking up the
    /// map tile immediately in front of the leader (C's
    /// `dx2offset(ch[cc].dir, ...)`); the soldier's target tile is one
    /// step behind `co` in `co`'s own facing direction (C's `(ch[co].dir
    /// + 3) % 8 + 1`, the same [`opposite_direction`] helper
    /// [`World::army_back_driver`] uses). If the soldier isn't already
    /// standing there, C's `move_driver(cn, tx, ty, 0)` (ported as
    /// [`World::setup_walk_toward`], itself exactly `pathfinder` +
    /// `walk_or_use_driver`, i.e. `move_driver`'s own definition) is
    /// tried first; on success this returns `true` immediately without
    /// attacking this tick, matching C's early `return 1`. If the move
    /// fails, the soldier says "cannot go there" and its mission reverts
    /// to `MIS_FOLLOW`, but - matching C's lack of an early return there
    /// - execution still falls through to the attack attempt below.
    /// Returns whether an attack was queued (C's final `return
    /// do_attack(cn, ch[co].dir, co)`), or `false` if the leader or `co`
    /// can no longer be resolved. C's random `AC_ATTACK1 + RANDOM(3)`
    /// variant pick is not reproduced (matching the pre-existing
    /// `action::ATTACK1`-only simplification already used by every other
    /// `do_attack` caller in this codebase, e.g.
    /// `setup_simple_baddy_attack_driver`/`attack_driver_direct` in
    /// `world/npc_fight.rs`).
    pub fn army_behind_driver(&mut self, character_id: CharacterId, area_id: u16) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = character.driver_state.clone() else {
            return false;
        };
        let Some(leader) = self.characters.get(&dat.leader_cn) else {
            return false;
        };
        let Ok(leader_direction) = Direction::try_from(leader.dir) else {
            return false;
        };
        let (fdx, fdy) = leader_direction.delta();
        let Some(front_x) = offset_coordinate(usize::from(leader.x), fdx) else {
            return false;
        };
        let Some(front_y) = offset_coordinate(usize::from(leader.y), fdy) else {
            return false;
        };
        let target_tile_character = self
            .map
            .tile(front_x, front_y)
            .map(|tile| tile.character)
            .unwrap_or(0);
        if target_tile_character == 0 {
            return false;
        }
        let target_id = CharacterId(u32::from(target_tile_character));
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        let (target_x, target_y) = (target.x, target.y);
        let target_dir = target.dir;
        let Ok(behind_direction) = Direction::try_from(opposite_direction(target_dir)) else {
            return false;
        };
        let (bdx, bdy) = behind_direction.delta();
        let Some(behind_x) = offset_coordinate(usize::from(target_x), bdx) else {
            return false;
        };
        let Some(behind_y) = offset_coordinate(usize::from(target_y), bdy) else {
            return false;
        };

        let (cx, cy) = (character.x, character.y);
        if usize::from(cx) != behind_x || usize::from(cy) != behind_y {
            if self.setup_walk_toward(character_id, behind_x, behind_y, 0, area_id, false) {
                return true;
            }
            self.npc_say(character_id, "cannot go there");
            self.set_fdemon_army_mission(character_id, MIS_FOLLOW);
        }

        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(attacker) = self.characters.get_mut(&character_id) else {
            return false;
        };
        crate::do_action::do_attack(
            attacker,
            &self.map,
            &target,
            target_dir,
            action::ATTACK1,
            weather_movement_percent,
        )
        .is_ok()
    }
}

/// C `(ch[cn].dir + 3) % 8 + 1` (`fdemon.c:676`): the direction opposite
/// `dir` - used by [`World::army_back_driver`] to step backward from a
/// held guard post.
fn opposite_direction(dir: u8) -> u8 {
    (u32::from(dir) + 3) as u8 % 8 + 1
}
