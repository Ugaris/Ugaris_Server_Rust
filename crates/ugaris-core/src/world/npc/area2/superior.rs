//! "Superior zombie" guardian NPC (`CDR_SUPERIOR`), area 2's four named
//! crypt guardians (Nazimah/Argatoth/Lorganoth/Markanoth, selected by the
//! `nr` zone-file arg, `arg="1"`..`"4"` in `zones/2/below2.chr`).
//!
//! Ports `src/area/2/area2.c::superior_driver` (`:84-171`): a full
//! self-defense/flee cascade gated by a unique "true name" stun mechanic -
//! any player who says the guardian's secret name within earshot stuns it
//! (forced `SM_STEALTH` idle) for 60 seconds.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification already established
//!   for `CDR_ROBBER`/`CDR_SANOA`/`CDR_BALLTRAP`/`CDR_ASTURIN`: C's generic
//!   10-slot `struct fight_driver_data` (`fight_driver_update`/
//!   `fight_driver_attack_visible`/`fight_driver_follow_invisible`) is
//!   narrowed to a single tracked `victim`. Unlike `CDR_BALLTRAP`
//!   (`aggressive=0`, self-defense only), this driver calls
//!   `standard_message_driver(cn, msg, 1, 0)` (`aggressive=1`): C's
//!   `is_valid_enemy(cn, co, -1)` check (`drvlib.c:897-927` - not self,
//!   same group excluded, `can_attack`, `char_see_char`) fires on *every*
//!   `NT_CHAR` sighting, not just `NT_GOTHIT`, so this port's victim
//!   tracking is set from both message types using the identical
//!   predicate.
//! - `fight_driver_set_dist(cn, 40, 0, 200)` (`area2.c:98`, on
//!   `NT_CREATE`) is not ported, same precedent as every other
//!   single-victim NPC's own module doc comment (the generic engine's
//!   distance-from-home enemy-admission gate has no equivalent in the
//!   single-victim model).
//! - `fight_driver_flee` (`drvlib.c:1018-1092`) is a full 9-direction,
//!   light-weighted pathing score across all 10 tracked enemies. This
//!   port simplifies to: flee directly away from the single tracked
//!   victim's current position (mirrored through the guardian's own
//!   position), reusing `World::secure_move_driver` toward a point 10
//!   tiles past the guardian in the opposite direction, clamped to the
//!   map. If no victim is currently visible there is nothing to flee
//!   from, matching C's `fight_driver_flee` returning `0` when its own
//!   `mindist` never updates (no visible enemy within 30 tiles).
//! - `dat->nr` (`area2.c:99`, `atoi(ch[cn].arg)`) is parsed once at zone
//!   spawn time in `Zone Loader::create_character_with_id` instead of on
//!   the first `NT_CREATE` message, since the loader has direct access to
//!   the raw zone-file `arg` string and this NPC's data has no other use
//!   for message-loop `NT_CREATE` handling.

use crate::world::*;

/// C `#define M_FIGHT 1` (`area2.c:81`).
pub const SUPERIOR_MODE_FIGHT: i32 = 1;
/// C `#define M_RUN 2` (`area2.c:82`).
pub const SUPERIOR_MODE_RUN: i32 = 2;

/// C `ticker + TICKS * 60` (`area2.c:107-116`): how long saying a
/// guardian's true name stuns it.
const SUPERIOR_STUN_TICKS: u64 = 60;

/// C's four `if (strcasestr(..., "Name") && dat->nr == N)` branches
/// (`area2.c:106-117`): each guardian's secret true name, keyed by the
/// zone-file `nr` arg.
const SUPERIOR_TRUE_NAMES: [(i32, &str); 4] = [
    (1, "Nazimah"),
    (2, "Argatoth"),
    (3, "Lorganoth"),
    (4, "Markanoth"),
];

fn superior_true_name(nr: i32) -> Option<&'static str> {
    SUPERIOR_TRUE_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == nr)
        .map(|(_, name)| *name)
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_SUPERIOR`
    /// characters (C `ch_driver`'s `CDR_SUPERIOR` case, `area2.c:988-990`).
    pub fn process_superior_actions(&mut self, area_id: u16) -> usize {
        let superior_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SUPERIOR
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for superior_id in superior_ids {
            if self.process_superior_tick(superior_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `superior_driver`'s per-tick body (`area2.c:84-171`).
    fn process_superior_tick(&mut self, superior_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&superior_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Superior(data)) => data,
            _ => SuperiorDriverData {
                mode: SUPERIOR_MODE_FIGHT,
                ..Default::default()
            },
        };

        let messages = self
            .characters
            .get_mut(&superior_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                // C `standard_message_driver`'s `NT_CHAR` branch
                // (`drvlib.c:2470-2476`, `aggressive=1`): any newly-seen
                // valid enemy becomes the tracked victim.
                NT_CHAR if message.dat1 > 0 => {
                    let seen_id = CharacterId(message.dat1 as u32);
                    if self.superior_is_valid_enemy(superior_id, seen_id) {
                        data.victim = Some(seen_id);
                    }
                }
                // C `area2.c:104-118`: any overheard `say`/`shout` naming
                // this guardian's true name stuns it for a minute.
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if (message.dat1 == 1 || message.dat1 == 2) && speaker_id != superior_id {
                        if let (Some(text), Some(true_name)) =
                            (message.text.as_deref(), superior_true_name(data.nr))
                        {
                            if text.to_lowercase().contains(&true_name.to_lowercase()) {
                                data.stun = self.tick.0 + SUPERIOR_STUN_TICKS * TICKS_PER_SECOND;
                            }
                        }
                    }
                }
                // C `standard_message_driver`'s `NT_GOTHIT` branch
                // (`drvlib.c:2512-2538`): defend against whoever hit us.
                NT_GOTHIT if message.dat1 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    if self.superior_is_valid_enemy(superior_id, attacker_id) {
                        data.victim = Some(attacker_id);
                    }
                }
                _ => {}
            }
        }

        // C `if (dat->stun > ticker) { ch[cn].speed_mode = SM_STEALTH;
        // do_idle(cn, TICKS); return; }` (`area2.c:125-129`).
        if data.stun > self.tick.0 {
            if let Some(character) = self.characters.get_mut(&superior_id) {
                character.speed_mode = SpeedMode::Stealth;
            }
            if let Some(character) = self.characters.get_mut(&superior_id) {
                character.driver_state = Some(CharacterDriverState::Superior(data));
            }
            return self
                .characters
                .get_mut(&superior_id)
                .is_some_and(|character| do_idle(character, TICKS_PER_SECOND as i32).is_ok());
        }

        // C `ch[cn].speed_mode = SM_NORMAL;` (`area2.c:131`).
        if let Some(character) = self.characters.get_mut(&superior_id) {
            character.speed_mode = SpeedMode::Normal;
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&superior_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((superior, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&superior, &victim, &self.map, self.date.daylight) {
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

        // C `area2.c:135-142`: low resources -> flee; fully recovered ->
        // fight again.
        if let Some(superior) = self.characters.get(&superior_id) {
            let hp = superior.hp;
            let lifeshield = superior.lifeshield;
            let mana = superior.mana;
            let max_hp = character_value(superior, CharacterValue::Hp) * POWERSCALE;
            let max_mana = character_value(superior, CharacterValue::Mana) * POWERSCALE;
            let max_shield = character_value(superior, CharacterValue::MagicShield) * POWERSCALE;
            if hp < 10 * POWERSCALE || lifeshield < POWERSCALE * 5 {
                data.mode = SUPERIOR_MODE_RUN;
            }
            if hp >= max_hp && mana >= max_mana && lifeshield >= max_shield {
                data.mode = SUPERIOR_MODE_FIGHT;
            }
        }

        if let Some(character) = self.characters.get_mut(&superior_id) {
            character.driver_state = Some(CharacterDriverState::Superior(data));
        }

        // C `area2.c:144-155`.
        if data.mode == SUPERIOR_MODE_RUN {
            if self.superior_flee(superior_id, area_id, &data) {
                return true;
            }
        } else {
            if data.victim_visible {
                if let Some(victim_id) = data.victim {
                    if self.attack_driver_direct(superior_id, victim_id, area_id) {
                        return true;
                    }
                }
            } else if data.victim.is_some() {
                let arrived = self.characters.get(&superior_id).is_some_and(|superior| {
                    superior.x.abs_diff(data.victim_last_x) < 2
                        && superior.y.abs_diff(data.victim_last_y) < 2
                });
                if arrived {
                    if let Some(CharacterDriverState::Superior(state)) = self
                        .characters
                        .get_mut(&superior_id)
                        .and_then(|character| character.driver_state.as_mut())
                    {
                        state.victim = None;
                    }
                } else if self.secure_move_driver(
                    superior_id,
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
        }

        // C `ch[cn].speed_mode = SM_STEALTH;` (`area2.c:157`).
        if let Some(character) = self.characters.get_mut(&superior_id) {
            character.speed_mode = SpeedMode::Stealth;
        }

        // C `if (regenerate_driver(cn)) return;` (`area2.c:159-161`).
        if self.regenerate_simple_baddy(superior_id) {
            return true;
        }
        // C `if (spell_self_driver(cn)) return;` (`area2.c:162-164`).
        if self.spell_self_simple_baddy(superior_id) {
            return true;
        }

        // C `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)` (`area2.c:166-168`): return to post
        // (`rest_x`/`rest_y` substitution, same as every sibling
        // single-victim NPC).
        let (post_x, post_y) = self
            .characters
            .get(&superior_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            superior_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`area2.c:170`).
        self.idle_simple_baddy(superior_id)
    }

    /// C `is_valid_enemy(cn, co, -1)` (`drvlib.c:897-927`, `mem == -1`
    /// case): not self, different group, `can_attack`, `char_see_char`.
    fn superior_is_valid_enemy(&self, character_id: CharacterId, target_id: CharacterId) -> bool {
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

    /// C `fight_driver_flee(cn)` (`drvlib.c:1018-1092`), simplified to a
    /// single tracked victim - see module doc comment.
    fn superior_flee(
        &mut self,
        superior_id: CharacterId,
        area_id: u16,
        data: &SuperiorDriverData,
    ) -> bool {
        if !data.victim_visible {
            return false;
        }
        let Some(superior) = self.characters.get(&superior_id) else {
            return false;
        };
        let dx = i32::from(superior.x) - i32::from(data.victim_last_x);
        let dy = i32::from(superior.y) - i32::from(data.victim_last_y);
        if dx == 0 && dy == 0 {
            return false;
        }
        let target_x = (i32::from(superior.x) + dx.signum() * 10).clamp(1, MAX_MAP as i32 - 2);
        let target_y = (i32::from(superior.y) + dy.signum() * 10).clamp(1, MAX_MAP as i32 - 2);
        self.secure_move_driver(
            superior_id,
            target_x as u16,
            target_y as u16,
            Direction::Down as u8,
            0,
            0,
            area_id,
        )
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_SUPERIOR;

/// C `struct superior_driver_data` (`area2.c:75-79`), plus this port's own
/// single-victim self-defense tracking (see module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SuperiorDriverData {
    pub nr: i32,
    pub stun: u64,
    pub mode: i32,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
