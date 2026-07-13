//! Shrike werewolf NPC (`CDR_SHR_WEREWOLF`), the invisible-by-day monster
//! guarding the wolf pit in area 38.
//!
//! Ports `src/area/38/shrike.c::shr_werewolf_driver`/`shr_werewolf_dead`
//! (`:344-391`, `:379-391` for the driver body). C's driver:
//!
//! ```c
//! void shr_werewolf_driver(int cn, int ret, int lastact) {
//!     if (is_fullnight()) {
//!         ch[cn].flags &= ~CF_INVISIBLE;
//!         char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret, lastact);
//!         return;
//!     }
//!     ch[cn].flags |= CF_INVISIBLE;
//!     if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret, lastact)) {
//!         return;
//!     }
//!     do_idle(cn, TICKS);
//! }
//! ```
//!
//! At full night ([`World::is_fullnight`], ported in `world::shrike`) the
//! werewolf becomes visible and behaves exactly like a plain
//! `CDR_SIMPLEBADDY` - its own `CharacterDriverState::SimpleBaddy` data,
//! populated at spawn time from the zone-file `arg=` string (see
//! `crate::zone`'s `CDR_SHR_WEREWOLF` handling, same precedent as
//! `CDR_WHITEROBBERBOSS`/`CDR_CENTINEL`), drives that behavior via
//! [`World::process_simple_baddy_message_actions`]/
//! [`World::process_simple_baddy_attack_action_with_random`]/
//! [`World::process_simple_baddy_noncombat_action_with_random_and_context`],
//! called directly per-character here rather than through the shared
//! `pass_0`/`lostcon_driver_4` batch sweeps in `ugaris-server`'s
//! `tick_npc::area22` - those sweeps run unconditionally every tick
//! regardless of day/night, which would let the werewolf fight/wander
//! during the day too (wrong: C's day/night gate lives inside
//! `shr_werewolf_driver` itself, before it ever reaches
//! `CDR_SIMPLEBADDY`). Accordingly, `CDR_SHR_WEREWOLF` is *not* added to
//! the `character.driver == CDR_SIMPLEBADDY` gates widened in
//! `world::npc_fight`/`world::npc_idle` for other "pure tail call"
//! drivers - only the single-character functions' own driver allow-lists
//! there needed widening, so this module could call them directly.
//!
//! During the day it is `CF_INVISIBLE` and walks home (`ch[cn].tmpx`/
//! `tmpy`, ported as `rest_x`/`rest_y` per the established
//! `world::npc::area31::lostdwarf` substitution) via [`World::
//! secure_move_driver`]; C's trailing `do_idle(cn, TICKS)` fallback is not
//! ported, matching the established stationary/simple-NPC precedent (see
//! `lostdwarf`'s own doc comment).
//!
//! `shr_werewolf_dead` (`:344-354`) is ported as a death hook,
//! `ugaris-server`'s `apply_shr_werewolf_death_from_hurt_event` (needs
//! `PlayerRuntime::area1_shrike_fails`, so it can't live in `World`
//! alone - same precedent as `apply_asturin_death_from_hurt_event`).

use crate::character_driver::CDR_SHR_WEREWOLF;
use crate::world::*;

impl World {
    /// C `shr_werewolf_driver`'s per-tick body (`shrike.c:379-391`) for
    /// every character currently under `CDR_SHR_WEREWOLF`.
    pub fn process_shr_werewolf_actions(&mut self, area_id: u16) -> usize {
        let mut seed = self.legacy_random_seed;
        let acted = self.process_shr_werewolf_actions_with_random(area_id, |below| {
            legacy_random_below_from_seed(&mut seed, below)
        });
        self.legacy_random_seed = seed;
        acted
    }

    pub fn process_shr_werewolf_actions_with_random(
        &mut self,
        area_id: u16,
        mut random: impl FnMut(u32) -> u32,
    ) -> usize {
        let werewolf_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SHR_WEREWOLF
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for werewolf_id in werewolf_ids {
            if self.process_shr_werewolf_action(werewolf_id, area_id, &mut random) {
                acted += 1;
            }
        }
        acted
    }

    fn process_shr_werewolf_action(
        &mut self,
        werewolf_id: CharacterId,
        area_id: u16,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        // C `is_fullnight()` (`shrike.c:79-81`).
        if self.is_fullnight() {
            return self.shr_werewolf_night_action(werewolf_id, area_id, random);
        }
        self.shr_werewolf_day_action(werewolf_id, area_id)
    }

    /// C `shr_werewolf_driver`'s full-night branch (`shrike.c:380-384`):
    /// `ch[cn].flags &= ~CF_INVISIBLE; char_driver(CDR_SIMPLEBADDY,
    /// CDT_DRIVER, cn, ret, lastact);` - a full `CDR_SIMPLEBADDY` tick
    /// (message loop, then attack, falling back to noncombat wander) for
    /// this one character, `ret`/`lastact` hardcoded to `0` same as every
    /// other NPC-driven (not player-completion-driven) SimpleBaddy caller
    /// in this codebase.
    fn shr_werewolf_night_action(
        &mut self,
        werewolf_id: CharacterId,
        area_id: u16,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        if let Some(werewolf) = self.characters.get_mut(&werewolf_id) {
            if werewolf.flags.contains(CharacterFlags::INVISIBLE) {
                werewolf.flags.remove(CharacterFlags::INVISIBLE);
                let (x, y) = (werewolf.x, werewolf.y);
                self.mark_dirty_sector(usize::from(x), usize::from(y));
            }
        }

        let has_messages = self
            .characters
            .get(&werewolf_id)
            .is_some_and(|werewolf| !werewolf.driver_messages.is_empty());
        let mut acted = false;
        if has_messages {
            acted |= !self
                .process_simple_baddy_message_actions(werewolf_id, area_id)
                .is_empty();
        }

        if self.process_simple_baddy_attack_action_with_random(werewolf_id, area_id, &mut *random) {
            return true;
        }

        acted |= self.process_simple_baddy_noncombat_action_with_random_and_context(
            werewolf_id,
            area_id,
            0,
            0,
            |limit| {
                if limit <= 0 {
                    0
                } else {
                    random(limit as u32) as i32
                }
            },
        );
        acted
    }

    /// C `shr_werewolf_driver`'s day branch (`shrike.c:386-390`):
    /// `ch[cn].flags |= CF_INVISIBLE; if (secure_move_driver(cn,
    /// ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret, lastact)) return; do_idle
    /// (cn, TICKS);`. `do_idle`'s tail fallback is not ported, matching
    /// `lostdwarf`'s precedent.
    fn shr_werewolf_day_action(&mut self, werewolf_id: CharacterId, area_id: u16) -> bool {
        if let Some(werewolf) = self.characters.get_mut(&werewolf_id) {
            if !werewolf.flags.contains(CharacterFlags::INVISIBLE) {
                werewolf.flags.insert(CharacterFlags::INVISIBLE);
                let (x, y) = (werewolf.x, werewolf.y);
                self.mark_dirty_sector(usize::from(x), usize::from(y));
            }
        }

        let Some(werewolf) = self.characters.get(&werewolf_id) else {
            return false;
        };
        let (post_x, post_y) = (werewolf.rest_x, werewolf.rest_y);
        self.secure_move_driver(
            werewolf_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        )
    }
}
