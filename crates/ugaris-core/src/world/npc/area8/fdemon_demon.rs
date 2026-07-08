//! Fire Demon/Fire Golem roaming AI (`CDR_FDEMON_DEMON`).
//!
//! Ports C `src/area/8/fdemon.c::fdemon_demon` (`:2741-2849`) - the
//! non-`sprite==190` branch. The `sprite==190` "Fire Golem" boss variant
//! (`fdemon_big1` in `ugaris_data/zones/8/fire.chr`) is a full,
//! unconditional-every-tick tail call to `char_driver(CDR_SIMPLEBADDY,
//! CDT_DRIVER, cn, ret, lastact)` (`fdemon.c:2746-2749`) - 100%
//! indistinguishable from a plain `CDR_SIMPLEBADDY` character, including
//! its own `NT_CREATE` arg parsing - so it is spawned with
//! `character.driver = CDR_SIMPLEBADDY` directly (see `zone.rs`'s
//! `CDR_FDEMON_DEMON` branch) rather than special-cased here.
//!
//! The remaining "Fire Demon" trash-mob templates (`fdemon1s..fdemon10s`)
//! reuse `CharacterDriverState::SimpleBaddy` wholesale too - same
//! precedent as `CDR_PENTER`/`CDR_DUNGEONFIGHTER` (see `CDR_FDEMON_DEMON`'s
//! own doc comment) - since C's own `NT_CREATE` handler
//! (`fight_driver_set_dist(cn, 0, 30, 0)`, no zone-file arg parsing) and
//! per-tick message handling (`standard_message_driver(cn, msg, 1, 1)`,
//! i.e. hardcoded `aggressive=1, helper=1`) are both already fully covered
//! by the existing generic `SimpleBaddy` message-processing pass
//! (`world::npc_messages::process_simple_baddy_message_actions`, gated on
//! `driver_state` alone, not `driver` id) once this struct's `aggressive`/
//! `helper` fields are seeded accordingly - see `zone.rs`. Only the extra
//! per-tick behavior C's `fdemon_demon` adds on top of plain SimpleBaddy -
//! the home/gohome hysteresis and the waypoint-graph hunt fallback
//! (`world::fdemon`) - needs this dedicated driver function, called
//! directly with the reused `SimpleBaddyDriverData`'s `dir`/`fdemon_gohome`
//! fields (see that struct's own doc comment).
//!
//! Deviations (documented, not silent):
//! - `add_enemy_to_waypoint` in C only fires from a live `NT_CHAR`
//!   message (populated by the generic `notify_area` sight/sound
//!   broadcast, `text.rs`'s own `NOTIFY_SIZE = 32` radius). Since the
//!   *same* messages are also drained every tick by the generic
//!   `SimpleBaddy` message pass (registered earlier in `tick_npc::run_all`
//!   than any new area-8 pass could be, and it unconditionally
//!   `mem::take`s `driver_messages`), this port instead does a direct
//!   per-tick scan of every `PLAYER`/`PLAYERLIKE` character within that
//!   same `NOTIFY_SIZE` radius of the demon's own position, checking
//!   `char_see_char` - the same class of "replace message-driven sighting
//!   with a direct scan" simplification already established by
//!   `world::npc::area4::tester` (see its own module doc comment) and
//!   `world::npc::janitor`. Functionally equivalent (same visibility
//!   check, same underlying broadcast radius), just not literally
//!   message-shaped.

use crate::{
    character_driver::{CharacterDriverState, CDR_FDEMON_DEMON},
    world::*,
};

/// Matches `world::text::notify_area`'s own `NOTIFY_SIZE` broadcast radius
/// - see this module's doc comment.
const SIGHTING_SCAN_RADIUS: u16 = 32;

/// C `#define TICKS ...` idle durations used verbatim by `fdemon_demon`.
const IDLE_SHORT: i32 = TICKS_PER_SECOND as i32;
const IDLE_LONG: i32 = TICKS_PER_SECOND as i32 * 2;

impl World {
    /// C `ch_driver`'s `CDR_FDEMON_DEMON` case (`fdemon.c:3021-3023`) for
    /// every live, non-`sprite==190` Fire Demon this tick.
    pub fn process_fdemon_demon_actions_with_completions(
        &mut self,
        area_id: u16,
        completions: &[WorldActionCompletion],
    ) -> usize {
        let mut seed = self.legacy_random_seed;
        let character_ids: Vec<CharacterId> = self
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                (character.driver == CDR_FDEMON_DEMON
                    && matches!(
                        character.driver_state,
                        Some(CharacterDriverState::SimpleBaddy(_))
                    ))
                .then_some(character_id)
            })
            .collect();

        let count = character_ids
            .into_iter()
            .filter(|&character_id| {
                let (ret, last_action) = completions
                    .iter()
                    .rev()
                    .find(|completion| completion.character_id == character_id)
                    .map(|completion| (completion.legacy_return_code, completion.action_id))
                    .unwrap_or((0, 0));
                self.process_fdemon_demon_action_with_random(
                    character_id,
                    area_id,
                    ret,
                    last_action,
                    |below| legacy_random_below_from_seed(&mut seed, below),
                )
            })
            .count();
        self.legacy_random_seed = seed;
        count
    }

    pub(crate) fn process_fdemon_demon_action_with_random(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        ret: i32,
        last_action: u16,
        mut random: impl FnMut(u32) -> u32,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character.driver != CDR_FDEMON_DEMON
            || character.action != action::IDLE
            || character.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }

        self.ensure_fdemon_waypoints_built();
        self.fdemon_demon_track_sightings(&character);

        // C `fdemon_demon`'s gohome hysteresis (`fdemon.c:2795-2810`):
        // `ch[cn].tmpx`/`tmpy` is this port's `Character::rest_x`/`rest_y`.
        let mut gohome = matches!(
            character.driver_state.as_ref(),
            Some(CharacterDriverState::SimpleBaddy(data)) if data.fdemon_gohome
        );
        if !fdemon_may_hunt_there(character.rest_x, character.rest_y, character.x, character.y) {
            gohome = true;
        }
        if character.x.abs_diff(character.rest_x) < 15
            && character.y.abs_diff(character.rest_y) < 15
        {
            gohome = false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.fdemon_gohome = gohome;
        }

        if gohome
            && self.secure_move_driver(
                character_id,
                character.rest_x,
                character.rest_y,
                Direction::Down as u8,
                ret,
                last_action,
                area_id,
            )
        {
            return true;
        }

        if self.fight_driver_attack_visible_and_follow(
            character_id,
            &character,
            area_id,
            FightDriverSuppressions::default(),
            true,
            &mut random,
        ) {
            return true;
        }

        if self.regenerate_simple_baddy(character_id) {
            return true;
        }
        if self.spell_self_simple_baddy(character_id) {
            return true;
        }

        if !gohome && self.fdemon_hunt_driver(character_id, area_id) {
            return true;
        }

        // C: `if (!RANDOM(4)) { do_idle(cn, TICKS*2); return; }`
        if random(4) == 0 {
            return self
                .characters
                .get_mut(&character_id)
                .is_some_and(|character| do_idle(character, IDLE_LONG).is_ok());
        }

        let dir = match character.driver_state.as_ref() {
            Some(CharacterDriverState::SimpleBaddy(data)) if data.dir != 0 => data.dir,
            _ => (random(8) as i32) + 1,
        };
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.dir = dir;
        }

        // C `do_walk(cn, dat->dir)`: the raw walk primitive, deliberately
        // *not* `move_driver`/`walk_or_use_driver`'s door-bump-open
        // fallback (this random-wander fallback never opens doors in C).
        let weather_movement_percent = self.settings.weather_movement_percent;
        let earthmud_extra_cost = self.earthmud_extra_movement_cost(character_id);
        let walked = u8::try_from(dir)
            .ok()
            .and_then(|dir| Direction::try_from(dir).ok())
            .is_some_and(|direction| {
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
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.dir = 0;
        }

        self.characters
            .get_mut(&character_id)
            .is_some_and(|character| do_idle(character, IDLE_SHORT).is_ok())
    }

    /// See module doc comment: replaces C's `NT_CHAR`-message-driven
    /// `add_enemy_to_waypoint(co)` call with a direct scan.
    fn fdemon_demon_track_sightings(&mut self, demon: &Character) {
        let min_x = demon.x.saturating_sub(SIGHTING_SCAN_RADIUS);
        let max_x = demon.x.saturating_add(SIGHTING_SCAN_RADIUS);
        let min_y = demon.y.saturating_sub(SIGHTING_SCAN_RADIUS);
        let max_y = demon.y.saturating_add(SIGHTING_SCAN_RADIUS);
        let daylight = self.date.daylight;
        let sighted: Vec<(u16, u16)> = self
            .characters
            .values()
            .filter(|target| {
                target.x >= min_x
                    && target.x <= max_x
                    && target.y >= min_y
                    && target.y <= max_y
                    && target.id != demon.id
                    && target
                        .flags
                        .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
            })
            .filter(|target| char_see_char(demon, target, &self.map, daylight))
            .map(|target| (target.x, target.y))
            .collect();
        for (x, y) in sighted {
            self.add_fdemon_enemy_to_waypoint(x, y);
        }
    }
}
