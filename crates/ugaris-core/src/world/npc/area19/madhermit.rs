//! The Nomad Plains flower-guarding hermit (`CDR_MADHERMIT`),
//! `src/area/19/nomad.c::madhermit_driver` (`:1177-1239`).
//!
//! A stationary self-defense NPC with a single quirk: it also picks a
//! fight with anyone it sees `USE`ing an `IDR_FLOWER` item nearby (C's own
//! flower-picking item driver, already ported), shouting a warning.
//!
//! C's `NT_CREATE` handler only calls `fight_driver_set_dist(cn, 30, 0,
//! 60)` (a fixed, template-independent seed) - ported by seeding
//! `Character::fight_driver` directly at spawn time in `crate::zone`'s
//! `CDR_MADHERMIT` branch instead of round-tripping an `NT_CREATE`
//! message, same "seed `DRD_FIGHTDRIVER` at spawn" precedent as
//! `ugaris-server::area8_army::spawn_army_soldier`.
//!
//! A deliberate, documented gap: C's `tabunga(cn, co, ptr)` call in the
//! `NT_TEXT` branch (`nomad.c:1207`) is a `CF_GOD`-only debug stat dump
//! (`src/system/tool.c:3837-3877`), not ported - see `super::nomad_text`'s
//! module doc comment for the same precedent.

use crate::character_driver::{add_simple_baddy_enemy_unchecked, CDR_MADHERMIT};
use crate::item_driver::IDR_FLOWER;
use crate::world::*;

/// C `AC_USE 7` (`src/system/act.h:30`) - not yet ported as a shared
/// action-kind constant/enum anywhere in this crate.
const AC_USE: u16 = 7;

impl World {
    /// C `madhermit_driver`'s per-tick body (`nomad.c:1177-1239`).
    pub fn process_madhermit_actions(&mut self, area_id: u16) {
        let madhermit_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_MADHERMIT
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for madhermit_id in madhermit_ids {
            self.process_madhermit_tick(madhermit_id, area_id);
        }
    }

    fn process_madhermit_tick(&mut self, madhermit_id: CharacterId, area_id: u16) {
        let messages = self
            .characters
            .get_mut(&madhermit_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            // C `if (msg->type == NT_CHAR) { co = msg->dat1; if (ch[co]
            // .action == AC_USE && (in = ch[co].act1) && ... it[in].driver
            // == IDR_FLOWER) { if (fight_driver_add_enemy(cn, co, 1, 1))
            // shout(...); } }` (`nomad.c:1193-1203`).
            if message.message_type == NT_CHAR {
                let player_id = CharacterId(message.dat1.max(0) as u32);
                let Some(player) = self.characters.get(&player_id).cloned() else {
                    continue;
                };
                if player.action != AC_USE || player.act1 <= 0 {
                    continue;
                }
                let item_id = ItemId(player.act1 as u32);
                let is_flower = self
                    .items
                    .get(&item_id)
                    .is_some_and(|item| item.driver == IDR_FLOWER);
                if !is_flower {
                    continue;
                }
                let tick = self.tick.0 as i32;
                let added = self
                    .characters
                    .get_mut(&madhermit_id)
                    .is_some_and(|hermit| {
                        add_simple_baddy_enemy_unchecked(hermit, player_id, 1, tick)
                    });
                if added {
                    self.npc_shout(
                        madhermit_id,
                        &format!("Hey! {}! Those flowers are mine!", player.name),
                    );
                }
            }
            // C's `NT_TEXT` branch only calls `tabunga` - see the module
            // doc comment for why that's a documented gap.
        }

        // C `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
        // 0)) return; if (fight_driver_follow_invisible(cn)) return;`
        // (`nomad.c:1217-1223`) - unlike `CDR_TWOTHIEFGUARD`, C calls
        // *both* halves here, so `may_follow_invisible: true`.
        if let Some(madhermit) = self.characters.get(&madhermit_id).cloned() {
            let mut seed = self.legacy_random_seed;
            let attacked = self.fight_driver_attack_visible_and_follow(
                madhermit_id,
                &madhermit,
                area_id,
                FightDriverSuppressions::default(),
                true,
                &mut |below| legacy_random_below_from_seed(&mut seed, below),
            );
            self.legacy_random_seed = seed;
            if attacked {
                return;
            }
        }

        // C `if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN,
        // ret, lastact)) return;` (`nomad.c:1226-1228`) - unconditional
        // every tick (no `last_talk` gate), same precedent as `world::npc::
        // area1::asturin`'s own unconditional return-to-post call.
        let (post_x, post_y) = self
            .characters
            .get(&madhermit_id)
            .map(|hermit| (hermit.rest_x, hermit.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            madhermit_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return;
        }

        // C `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return;` (`nomad.c:1230-1236`).
        if self.regenerate_simple_baddy(madhermit_id) {
            return;
        }
        self.spell_self_simple_baddy(madhermit_id);
        // C `do_idle(cn, TICKS/2);` (`nomad.c:1238`) - not modeled, same
        // precedent as every other stationary NPC in this codebase.
    }
}

/// C `struct madhermit` has no zone-file-parsed driver data (`set_data`'s
/// C equivalent is never called for this driver - only the driver-
/// independent `DRD_FIGHTDRIVER` slot is used) - this marker type exists
/// only so [`crate::character_driver::CharacterDriverState::Madhermit`]
/// has somewhere to park the "this character is a madhermit" fact for the
/// exhaustive `driver_state` matches elsewhere in the crate (same "no
/// real data" precedent as `crate::character_driver::TraderDriverData`
/// before its own first field was added).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MadhermitDriverData;
