//! `CDR_TEUFELDEMON` (Teufelheim's own demon-suit guardians), ports
//! `src/area/34/teufel.c::teufeldemon_driver` (`:373-394`) plus
//! `is_demon` (`:366-371`, shared with `teufeldoor`/`teufelarena` and
//! ported in `super::is_demon`).
//!
//! C's function has exactly two parts: an `NT_CHAR` self-defense hook
//! (attack any sighted player who isn't wearing one of the three
//! demon-suit sprites), then an unconditional tail call to
//! `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret, lastact)` reusing
//! the SimpleBaddy driver's full idle-wander/auto-attack AI wholesale.
//!
//! Unlike the *pure* tail-call NPCs (`CDR_PENTER`/`CDR_FORESTMONSTER`/
//! `CDR_TWOROBBER`, ported by widening the `character.driver ==
//! CDR_SIMPLEBADDY` gates in `world::npc_fight`/`world::npc_idle` with no
//! extra per-tick logic of their own), `CDR_TEUFELDEMON` also needs that
//! same gate widening (this driver's own `apply_teufeldemon_sighting_
//! messages` below only *adds* enemies - the actual attack/follow/idle
//! AI that consumes them is the shared SimpleBaddy code, gated on
//! `character.driver` in those two modules) *plus* its own `NT_CHAR`
//! hook. C's `char_driver` never calls `remove_message` between
//! `teufeldemon_driver`'s own loop and its `CDR_SIMPLEBADDY` tail call,
//! so both handlers observe the *same* still-queued `NT_CHAR` messages
//! in the same tick - ported here by calling
//! [`World::apply_teufeldemon_sighting_messages`] from
//! `world::npc_messages::process_simple_baddy_message_actions_with_random`
//! immediately before that function's own generic message drain (which
//! *does* consume/clear `driver_messages`), rather than as a separate
//! `tick_npc` pass: any later, differently-ordered pass would observe an
//! already-drained queue and never see the same messages, since real
//! `zones/34/teufel.chr` `teufer1`/`teufer2`/`teufer3` templates spawn
//! with `aggressive=0` (so the generic SimpleBaddy `NT_CHAR` aggro branch,
//! gated on that flag, never fires on its own - this driver's own
//! override is the *entire* source of Teufelheim demons attacking
//! disguise-less players).
//!
//! The `is_valid_enemy(cn, co, -1)` + `fight_driver_add_enemy(cn, co, 0,
//! 1)` pair (`teufel.c:385-388`) is exactly the already-ported
//! `World::simple_baddy_can_add_standard_enemy(.., require_visible: true,
//! hurtme: false)` + `add_simple_baddy_enemy_unchecked(.., priority: 0,
//! ..)` combination the generic aggressive-mode `StandardAggro` message
//! outcome already uses (`world::npc_messages::process_simple_baddy_
//! messages`'s `NT_CHAR` branch) - reused directly rather than
//! reimplemented, since `is_valid_enemy`'s group/`can_attack`/
//! `char_see_char` checks plus `fight_driver_add_enemy`'s own
//! `start_dist`/`char_dist`/neutral-zone gating (all folded into that one
//! helper) are identical to what C actually runs here.

use crate::character_driver::{add_simple_baddy_enemy_unchecked, CDR_TEUFELDEMON};
use crate::world::npc::area34::is_demon;
use crate::world::*;

impl World {
    /// C `teufeldemon_driver`'s own `NT_CHAR` loop (`teufel.c:378-391`),
    /// called ahead of the generic SimpleBaddy message drain - see this
    /// module's doc comment for why this can't be its own `tick_npc`
    /// pass.
    pub(crate) fn apply_teufeldemon_sighting_messages(&mut self, demon_id: CharacterId) {
        if self
            .characters
            .get(&demon_id)
            .is_none_or(|demon| demon.driver != CDR_TEUFELDEMON)
        {
            return;
        }
        let messages = self
            .characters
            .get(&demon_id)
            .map(|demon| demon.driver_messages.clone())
            .unwrap_or_default();
        let tick = self.tick.0 as i32;
        for message in &messages {
            if message.message_type != NT_CHAR || message.dat1 <= 0 {
                continue;
            }
            let target_id = CharacterId(message.dat1 as u32);
            // C `(ch[co].flags & CF_PLAYER) && !is_demon(co)`
            // (`teufel.c:384`): only non-disguised players are ever
            // considered.
            let is_disguise_less_player = self.characters.get(&target_id).is_some_and(|target| {
                target.flags.contains(CharacterFlags::PLAYER) && !is_demon(target.sprite)
            });
            if !is_disguise_less_player {
                continue;
            }
            // C `is_valid_enemy(cn, co, -1)` then `fight_driver_add_enemy(
            // cn, co, 0, 1)` (`teufel.c:385-386`): `hurtme=0`,
            // `visible=1` - matching `require_visible: true, hurtme:
            // false` below.
            if !self.simple_baddy_can_add_standard_enemy(demon_id, target_id, true, false) {
                continue;
            }
            if let Some(demon) = self.characters.get_mut(&demon_id) {
                add_simple_baddy_enemy_unchecked(demon, target_id, 0, tick);
            }
            self.sort_simple_baddy_enemies_like_c(demon_id);
        }
    }
}
