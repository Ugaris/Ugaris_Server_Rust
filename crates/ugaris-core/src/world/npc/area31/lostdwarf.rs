//! Lost Dwarf NPC (`CDR_LOSTDWARF`), the four (`nr` 1-4) missing miners
//! `world::npc::area31::dwarfchief`'s "A Miner's Misery"/"A Miner's Bane"/
//! "A Miner's Anguish"/"A Miner Lost" quest chain sends the player to
//! rescue with a `dwarf_recallNN` scroll.
//!
//! Ports `src/area/31/warrmines.c::lostdwarf_driver` (`:905-1044`). Unlike
//! every other `warrmines.c` NPC driver, this one has no `analyse_text_
//! driver`/QA table hookup at all (C never wires `NT_TEXT` for it) and its
//! own `struct lostdwarf_data.nr` (`:905-909`) is parsed once at spawn
//! time from the zone-file `arg="N"` (see [`crate::zone`]'s `CDR_LOSTDWARF`
//! handling), same precedent as `world::npc::area2::superior`'s `dat->nr`
//! - not re-parsed via `NT_CREATE` here.
//!
//! Follows the same `World`/`PlayerRuntime` split as every other
//! `warrmines.c`/`brannington.c` driver: the caller supplies a per-player
//! fact snapshot ([`LostdwarfPlayerFacts`]) up front and applies the
//! returned [`LostdwarfOutcomeEvent`]s afterwards, since `staffer_ppd.
//! dwarfchief_state` lives on `crate::player::PlayerRuntime`, not `World`.
//! Note this driver *writes* `dwarfchief_state` directly (`ppd->
//! dwarfchief_state = N`, not `questlog_done`) - `dwarfchief_driver`'s own
//! `case 3`/`case 6`/`case 9`/`case 12` "waiting" states simply notice the
//! external jump on their next `NT_CHAR` tick, same shape as `world::npc::
//! area29::broklin`'s `robberboss_dead`-driven `broklin_state` jump to
//! `11`.
//!
//! Deviations/gaps (documented, not silent):
//! - C `log_char(co, LOG_SYSTEM, 0, "...")` (the `nr==2`/`nr==3` "beard
//!   looks thin"/"is barefoot" flavor lines) is ported via [`World::
//!   queue_system_text`] (same mapping as `world::npc::area3::supermax`'s
//!   own doc comment).
//! - C `log_area(x, y, LOG_INFO, cn, 10, "%s uses a scroll of recall and
//!   vanishes.", ...)` is ported via a direct `pending_area_texts` push
//!   (same mapping as `world::npc::area22::lab3_prisoner::
//!   log_area_gesture`'s own doc comment) - `LOG_INFO`'s per-viewer
//!   visibility-check nuance (`src/system/talk.h:21`) is not modeled
//!   beyond the plain distance cutoff every other `log_area` port uses.
//! - `set_sector`/`mark_dirty_sector` calls are ported via [`World::
//!   mark_dirty_sector`] (same mapping as `world::npc::area16::imp`'s own
//!   `CF_INVISIBLE` toggle).
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:1043`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::world::*;

/// C `char_dist(cn, co) > 10` (`warrmines.c:967`).
const LOSTDWARF_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 60` (`warrmines.c:955`): lostdwarf's own greeting is
/// throttled purely by elapsed time, unlike every other `warrmines.c`/
/// `brannington.c` driver's `TICKS*4`/`TICKS*10` two-stage throttle - no
/// `current_victim` tracking exists in C's `struct lostdwarf_data` at all.
const LOSTDWARF_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `TICKS * 30` (`warrmines.c:988`/`1000`/`1012`/`1023`): how long the
/// rescued miner stays `CF_INVISIBLE` before reappearing.
const LOSTDWARF_INVIS_TICKS: u64 = TICKS_PER_SECOND * 30;

/// Per-player facts [`World::process_lostdwarf_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LostdwarfPlayerFacts {
    /// `PlayerRuntime::staffer_dwarfchief_state()`.
    pub dwarfchief_state: i32,
}

/// A side effect [`World::process_lostdwarf_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LostdwarfOutcomeEvent {
    /// C `ppd->dwarfchief_state = N` (`warrmines.c:983`/`994`/`1006`/
    /// `1018`): a direct state assignment, not `questlog_done` (that
    /// happens later, driven by `dwarfchief_driver` itself noticing the
    /// jump).
    UpdateDwarfchiefState {
        player_id: CharacterId,
        new_state: i32,
    },
}

impl World {
    /// C `lostdwarf_driver`'s per-tick body (`warrmines.c:911-1044`).
    pub fn process_lostdwarf_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, LostdwarfPlayerFacts>,
        area_id: u16,
    ) -> Vec<LostdwarfOutcomeEvent> {
        let lostdwarf_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LOSTDWARF
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for lostdwarf_id in lostdwarf_ids {
            self.process_lostdwarf_messages(lostdwarf_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_lostdwarf_messages(
        &mut self,
        lostdwarf_id: CharacterId,
        player_facts: &HashMap<CharacterId, LostdwarfPlayerFacts>,
        area_id: u16,
        events: &mut Vec<LostdwarfOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::LostDwarf(mut data)) = self
            .characters
            .get(&lostdwarf_id)
            .and_then(|lostdwarf| lostdwarf.driver_state.clone())
        else {
            return;
        };

        // C `if ((ch[cn].flags & CF_INVISIBLE) && dat->invis_tick <
        // ticker) { ch[cn].flags &= ~CF_INVISIBLE; set_sector(...); }`
        // (`warrmines.c:922-925`).
        let tick = self.tick.0;
        if let Some(lostdwarf) = self.characters.get(&lostdwarf_id) {
            if lostdwarf.flags.contains(CharacterFlags::INVISIBLE) && data.invis_tick < tick {
                let (x, y) = (lostdwarf.x, lostdwarf.y);
                if let Some(lostdwarf_mut) = self.characters.get_mut(&lostdwarf_id) {
                    lostdwarf_mut.flags.remove(CharacterFlags::INVISIBLE);
                }
                self.mark_dirty_sector(usize::from(x), usize::from(y));
            }
        }

        let messages = self
            .characters
            .get_mut(&lostdwarf_id)
            .map(|lostdwarf| std::mem::take(&mut lostdwarf.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.lostdwarf_handle_char_message(lostdwarf_id, &mut data, message),
                NT_GIVE => self.lostdwarf_handle_give_message(
                    lostdwarf_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                ),
                _ => {}
            }
        }

        if let Some(lostdwarf) = self.characters.get_mut(&lostdwarf_id) {
            lostdwarf.driver_state = Some(CharacterDriverState::LostDwarf(data));
        }

        // C `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)` (`warrmines.c:1039-1041`), unconditional (no `last_talk`
        // gate like the other `warrmines.c` drivers). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::brennethbran` already uses.
        let Some(lostdwarf) = self.characters.get(&lostdwarf_id) else {
            return;
        };
        let (post_x, post_y) = (lostdwarf.rest_x, lostdwarf.rest_y);
        self.secure_move_driver(
            lostdwarf_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        );
    }

    /// C `lostdwarf_driver`'s `NT_CHAR` branch (`warrmines.c:938-974`).
    fn lostdwarf_handle_char_message(
        &mut self,
        lostdwarf_id: CharacterId,
        data: &mut LostDwarfDriverData,
        message: &CharacterDriverMessage,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(lostdwarf) = self.characters.get(&lostdwarf_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) continue;` (`warrmines.c:942-
        // 946`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) continue;` (`warrmines.c:
        // 948-952`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*60) continue;`
        // (`warrmines.c:954-958`).
        if tick < data.last_talk + LOSTDWARF_TALK_MIN_TICKS {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`warrmines.c:960-964`).
        if lostdwarf_id == player_id
            || !char_see_char(&lostdwarf, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`warrmines.c:966-
        // 970`).
        if char_dist(&lostdwarf, &player) > LOSTDWARF_GREET_DISTANCE {
            return;
        }

        // C `quiet_say(cn, "I hope you have a dwarven recall scroll for
        // me! If not, be off with you!"); dat->last_talk = ticker;`
        // (`warrmines.c:972-973`).
        self.npc_quiet_say(
            lostdwarf_id,
            "I hope you have a dwarven recall scroll for me! If not, be off with you!",
        );
        data.last_talk = tick;
    }

    /// C `lostdwarf_driver`'s `NT_GIVE` branch (`warrmines.c:977-1031`).
    fn lostdwarf_handle_give_message(
        &mut self,
        lostdwarf_id: CharacterId,
        data: &mut LostDwarfDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, LostdwarfPlayerFacts>,
        events: &mut Vec<LostdwarfOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&lostdwarf_id)
            .and_then(|lostdwarf| lostdwarf.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let Some(lostdwarf) = self.characters.get(&lostdwarf_id).cloned() else {
            return;
        };
        let dwarfchief_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.dwarfchief_state)
            .unwrap_or(0);
        let tick = self.tick.0;

        let rescue = match data.nr {
            // C `if (ppd && ppd->dwarfchief_state <= 3 && dat->nr == 1 &&
            // it[in].ID == IID_DWARFRECALL1)` (`warrmines.c:982`).
            1 if dwarfchief_state <= 3 && item.template_id == IID_DWARFRECALL1 => {
                Some((4, "I got so hungry I almost ate my beard.", None))
            }
            // C `if (ppd && ppd->dwarfchief_state >= 5 &&
            // ppd->dwarfchief_state <= 6 && dat->nr == 2 && it[in].ID ==
            // IID_DWARFRECALL2)` (`warrmines.c:992-993`).
            2 if (5..=6).contains(&dwarfchief_state) && item.template_id == IID_DWARFRECALL2 => {
                Some((
                    7,
                    "I got so hungry I almost ate my boots.",
                    Some("You notice that the dwarf's beard looks somewhat thin."),
                ))
            }
            // C `if (ppd && ppd->dwarfchief_state >= 8 &&
            // ppd->dwarfchief_state <= 9 && dat->nr == 3 && it[in].ID ==
            // IID_DWARFRECALL3)` (`warrmines.c:1004-1005`).
            3 if (8..=9).contains(&dwarfchief_state) && item.template_id == IID_DWARFRECALL3 => {
                Some((
                    10,
                    "I got so hungry I almost ate my pick-axe.",
                    Some("You notice that the dwarf is barefoot."),
                ))
            }
            // C `if (ppd && ppd->dwarfchief_state >= 11 &&
            // ppd->dwarfchief_state <= 12 && dat->nr == 4 && it[in].ID ==
            // IID_DWARFRECALL4)` (`warrmines.c:1016-1017`).
            4 if (11..=12).contains(&dwarfchief_state) && item.template_id == IID_DWARFRECALL4 => {
                Some((13, "I got so hungry I did eat my pick-axe.", None))
            }
            _ => None,
        };

        if let Some((new_state, say_suffix, system_note)) = rescue {
            events.push(LostdwarfOutcomeEvent::UpdateDwarfchiefState {
                player_id: giver_id,
                new_state,
            });
            self.npc_say(
                lostdwarf_id,
                &format!("Thank you for saving me, {}. {}", giver.name, say_suffix),
            );
            if let Some(note) = system_note {
                self.queue_system_text(giver_id, note);
            }
            // C `log_area(ch[cn].x, ch[cn].y, LOG_INFO, cn, 10, "%s uses a
            // scroll of recall and vanishes.", ch[cn].name);` (`warrmines.c:
            // 985-986` et al.).
            self.pending_area_texts.push(WorldAreaText {
                x: lostdwarf.x,
                y: lostdwarf.y,
                max_distance: 10,
                message: format!("{} uses a scroll of recall and vanishes.", lostdwarf.name),
            });
            // C `ch[cn].flags |= CF_INVISIBLE; dat->invis_tick = ticker +
            // TICKS*30; set_sector(...);`.
            if let Some(lostdwarf_mut) = self.characters.get_mut(&lostdwarf_id) {
                lostdwarf_mut.flags.insert(CharacterFlags::INVISIBLE);
            }
            data.invis_tick = tick + LOSTDWARF_INVIS_TICKS;
            self.mark_dirty_sector(usize::from(lostdwarf.x), usize::from(lostdwarf.y));
        }

        // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;`
        // (`warrmines.c:1027-1029`): unconditional, regardless of whether
        // any of the four rescue branches matched.
        self.destroy_item(item_id);
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_LOSTCON, CDR_LOSTDWARF};
use crate::item_driver::{IID_DWARFRECALL1, IID_DWARFRECALL2, IID_DWARFRECALL3, IID_DWARFRECALL4};

/// C `struct lostdwarf_data` (`src/area/31/warrmines.c:905-909`). `nr` is
/// parsed once at spawn time from the zone-file `arg="N"` (see the module
/// doc comment), not re-parsed via `NT_CREATE`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LostDwarfDriverData {
    pub nr: i32,
    #[serde(default)]
    pub invis_tick: u64,
    #[serde(default)]
    pub last_talk: u64,
}
