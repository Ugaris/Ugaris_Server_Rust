//! Main quest-giver NPC (`CDR_GWENDYLON`), area 1's mage at the knight
//! castle - the highest-value area-1 NPC (first quest chain new players
//! see).
//!
//! Ports `src/area/1/gwendylon.c::gwendylon_driver` (`:234-673`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`/`world::terion`). Follows the same
//! `World`/`PlayerRuntime` split established there: the caller supplies a
//! per-player fact snapshot ([`GwendylonPlayerFacts`]) up front and applies
//! the returned [`GwendylonOutcomeEvent`]s afterwards, since
//! `gwendy_state`/`gwendy_seen_timer` (`area1_ppd` fields) and the four
//! `QLOG_GWENDY_*` quest-log entries live on `crate::player::PlayerRuntime`,
//! not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - The `GWENDYLON_STATE_DONE_BLESS` branch's `if (may_add_spell(co,
//!   IDR_BLESS) && do_bless(cn, co)) { ppd->gwendy_seen_timer = realtime;
//!   return; }` (`gwendylon.c:464-467`) is a genuine early `return` *before*
//!   `remove_message` runs for the triggering `NT_CHAR` message - the
//!   message is never removed from the queue that tick, so C reprocesses
//!   the very same message next tick (by which point the freshly-applied
//!   bless makes `may_add_spell` fail, falling through to the normal
//!   `didsay` path and finally consuming the message). This is reproduced
//!   here: [`World::process_gwendylon_actions`] restores the triggering
//!   message (and any not-yet-processed ones after it) back onto the
//!   NPC's `driver_messages` and skips the turn/idle-move tail entirely
//!   for that tick, exactly like C's `return`.
//! - `IID_CALIGARLETTER`'s `change_area(co, 36, 240, 10)` (`gwendylon.c:
//!   623-640`) needs a DB lookup and session redirect `World` has no
//!   access to (same architectural gap as `world::jail`/`world::macro_npc`)
//!   - queued as a [`GwendylonCrossAreaTransfer`] for `ugaris-server`'s
//!   `area1.rs::apply_gwendylon_cross_area_transfers`, which calls the
//!   shared `attempt_cross_area_transfer` helper. The `give_char_item_
//!   smart(co, ch[cn].citem, 1)` hand-back (unconditional, regardless of
//!   whether the transfer itself later succeeds) still happens
//!   synchronously here, matching C's ordering.
//! - `destroy_item_byID(co, ID)` (the four skull-turn-in branches) sweeps
//!   the player's equipment/inventory/cursor via
//!   [`World::destroy_items_by_template_id`] but not the account depot
//!   (`DRD_DEPOT_PPD`) - that storage lives in `ugaris-server`'s
//!   `PlayerRuntime`/DB layer, not `World` (same documented gap as
//!   `world::yoakin`).
//! - The four `_WAIT` reminder lines wrap "repeat" in `COL_LIGHT_BLUE`/
//!   `COL_RESET` markers in C (`gwendylon.c:333,372,411,449`); dropped
//!   here for the same reason documented on `world::camhermit`'s module
//!   doc comment (`World::npc_quiet_say` broadcasts a plain UTF-8
//!   `String`).

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_GWENDYLON, NTID_TUTORIAL};
use crate::drvlib::offset2dx;
use crate::item_driver::{
    IID_AREA1_MAGESKULL, IID_AREA1_SKELKEY1, IID_AREA1_SKELKEY2, IID_AREA1_SKELKEY3,
    IID_AREA1_SKELSKULL, IID_AREA1_WARLOCKKEY, IID_AREA1_WARLOCKSKULL, IID_AREA1_WOODKEY,
    IID_AREA1_WOODSKULL, IID_CALIGARLETTER,
};
use crate::quest::quest_exp::{
    MONEY_AREA1_SKULL1, MONEY_AREA1_SKULL2, MONEY_AREA1_SKULL3, MONEY_AREA1_SKULL4,
};
use crate::quest::{
    GWENDYLON_STATE_ENTRY, GWENDYLON_STATE_FIRST_SKULL_DONE, GWENDYLON_STATE_FOUL_MAGICIAN_DONE,
    GWENDYLON_STATE_SECOND_SKULL_DONE, GWENDYLON_STATE_THIRD_SKULL_DONE, QLOG_GWENDY_FIRST_SKULL,
    QLOG_GWENDY_FOUL_MAGICIAN, QLOG_GWENDY_SECOND_SKULL, QLOG_GWENDY_THIRD_SKULL,
};
use crate::world::*;

/// C `char_dist(cn, co) > 16` (`gwendylon.c:283`): the `NT_CHAR` greeting
/// range - wider than every other area-1 NPC's own 10/12 (matching the
/// C source verbatim, not a copy/paste slip to "fix").
const GWENDYLON_GREET_DISTANCE: i32 = 16;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const GWENDYLON_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gwendylon.c:266`).
const GWENDYLON_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gwendylon.c:271`, `:486`).
const GWENDYLON_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:661`): idle "return to post" threshold.
const GWENDYLON_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `realtime - ppd->gwendy_seen_timer > 60`, shared by every `_WAIT`
/// state's reminder gate (`gwendylon.c:330,369,408,446,462`).
const GWENDYLON_REMINDER_SECONDS: i32 = 60;

/// C's bare `int` state values for `ppd->gwendy_state`
/// (`src/common/npc_states.h:28-47`) not already exported from
/// `crate::quest` (which only needed the four `_DONE` checkpoints for
/// `init_area1_quests`). Named here to match the real C macros exactly.
const GWENDYLON_STATE_FIRST_SKULL_1: i32 = 1;
const GWENDYLON_STATE_FIRST_SKULL_2: i32 = 2;
const GWENDYLON_STATE_FIRST_SKULL_3: i32 = 3;
const GWENDYLON_STATE_FIRST_SKULL_4: i32 = 4;
const GWENDYLON_STATE_FIRST_SKULL_WAIT: i32 = 5;
const GWENDYLON_STATE_SECOND_SKULL_1: i32 = 7;
const GWENDYLON_STATE_SECOND_SKULL_2: i32 = 8;
const GWENDYLON_STATE_SECOND_SKULL_WAIT: i32 = 9;
const GWENDYLON_STATE_THIRD_SKULL_1: i32 = 11;
const GWENDYLON_STATE_THIRD_SKULL_2: i32 = 12;
const GWENDYLON_STATE_THIRD_SKULL_WAIT: i32 = 13;
const GWENDYLON_STATE_FOUL_MAGICIAN_1: i32 = 15;
const GWENDYLON_STATE_FOUL_MAGICIAN_2: i32 = 16;
const GWENDYLON_STATE_FOUL_MAGICIAN_WAIT: i32 = 17;
const GWENDYLON_STATE_DONE_BLESS: i32 = 19;

/// Per-player facts [`World::process_gwendylon_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GwendylonPlayerFacts {
    /// `PlayerRuntime::area1_gwendy_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_gwendy_seen_timer()` (C `realtime` wall-clock
    /// seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
    /// `PlayerRuntime::quest_log.is_done(QLOG_GWENDY_SECOND_SKULL)`: the
    /// `GWENDYLON_STATE_FIRST_SKULL_DONE` skip-ahead check
    /// (`gwendylon.c:341`).
    pub quest2_isdone: bool,
    /// `PlayerRuntime::quest_log.is_done(QLOG_GWENDY_THIRD_SKULL)`: the
    /// `GWENDYLON_STATE_SECOND_SKULL_DONE` skip-ahead check
    /// (`gwendylon.c:379`).
    pub quest3_isdone: bool,
    /// `PlayerRuntime::quest_log.is_done(QLOG_GWENDY_FOUL_MAGICIAN)`: the
    /// `GWENDYLON_STATE_THIRD_SKULL_DONE` skip-ahead check
    /// (`gwendylon.c:418`).
    pub quest4_isdone: bool,
    /// `PlayerRuntime::quest_log.count(QLOG_GWENDY_FIRST_SKULL)`, sampled
    /// *before* this tick's completion - `0` means C's `questlog_done`
    /// would return `1` (first completion), gating the skull-1 gold
    /// reward.
    pub quest1_done_count: u8,
    /// Same as `quest1_done_count`, for `QLOG_GWENDY_SECOND_SKULL`.
    pub quest2_done_count: u8,
    /// Same as `quest1_done_count`, for `QLOG_GWENDY_THIRD_SKULL`.
    pub quest3_done_count: u8,
    /// Same as `quest1_done_count`, for `QLOG_GWENDY_FOUL_MAGICIAN`.
    pub quest4_done_count: u8,
}

/// A side effect [`World::process_gwendylon_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GwendylonOutcomeEvent {
    /// Write the new `area1_ppd.gwendy_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->gwendy_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message that did *not* take the `return`-early
    /// bless path (`gwendylon.c:473`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, ...)`.
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `questlog_done(co, ...)` - the caller must apply
    /// `PlayerRuntime::quest_log.complete_legacy` (exp reward + resend).
    QuestDone {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `give_char_item_smart(co, in, 1)`'s `IF_MONEY` branch for the
    /// `create_money_item(MONEY_AREA1_SKULL*)` rewards - applied directly
    /// rather than round-tripping through a throwaway `Item` (same
    /// simplification as `world::yoakin`'s `GoldEarned`).
    GoldEarned { player_id: CharacterId, amount: u32 },
}

/// `gwendylon_driver`'s `IID_CALIGARLETTER` hand-off - see the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GwendylonCrossAreaTransfer {
    pub gwendylon_id: CharacterId,
    pub player_id: CharacterId,
}

impl World {
    /// Drains every queued `IID_CALIGARLETTER` hand-off - see
    /// [`GwendylonCrossAreaTransfer`].
    pub fn drain_pending_gwendylon_cross_area_transfers(
        &mut self,
    ) -> Vec<GwendylonCrossAreaTransfer> {
        self.pending_gwendylon_cross_area_transfers
            .drain(..)
            .collect()
    }

    /// C `gwendylon_driver`'s per-tick body (`gwendylon.c:234-673`). `now`
    /// is C's wall-clock `realtime` (seconds).
    pub fn process_gwendylon_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, GwendylonPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<GwendylonOutcomeEvent> {
        let gwendylon_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_GWENDYLON
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for gwendylon_id in gwendylon_ids {
            self.process_gwendylon_messages(gwendylon_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_gwendylon_messages(
        &mut self,
        gwendylon_id: CharacterId,
        player_facts: &HashMap<CharacterId, GwendylonPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<GwendylonOutcomeEvent>,
    ) {
        let Some(gwendylon_name) = self
            .characters
            .get(&gwendylon_id)
            .map(|gwendylon| gwendylon.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Gwendylon(mut data)) = self
            .characters
            .get(&gwendylon_id)
            .and_then(|gwendylon| gwendylon.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&gwendylon_id)
            .map(|gwendylon| std::mem::take(&mut gwendylon.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        let mut index = 0;
        while index < messages.len() {
            let message = &messages[index];
            match message.message_type {
                NT_CHAR => {
                    let blessed_and_returned = self.gwendylon_handle_char_message(
                        gwendylon_id,
                        &mut data,
                        message,
                        player_facts,
                        now,
                        events,
                        &mut face_target,
                    );
                    if blessed_and_returned {
                        // C `return;` before `remove_message` runs for this
                        // message (`gwendylon.c:466`) - see the module doc
                        // comment. Restore the triggering message and every
                        // not-yet-processed one after it, then skip the
                        // turn/idle-move tail entirely for this tick.
                        let mut restored: Vec<CharacterDriverMessage> = messages[index..].to_vec();
                        if let Some(gwendylon) = self.characters.get_mut(&gwendylon_id) {
                            restored.extend(std::mem::take(&mut gwendylon.driver_messages));
                            gwendylon.driver_messages = restored;
                            gwendylon.driver_state = Some(CharacterDriverState::Gwendylon(data));
                        }
                        return;
                    }
                }
                NT_TEXT => self.gwendylon_handle_text_message(
                    gwendylon_id,
                    &gwendylon_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.gwendylon_handle_give_message(gwendylon_id, message, player_facts, events)
                }
                _ => {}
            }
            index += 1;
        }

        if let Some(gwendylon) = self.characters.get_mut(&gwendylon_id) {
            gwendylon.driver_state = Some(CharacterDriverState::Gwendylon(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:657-659`).
        if let (Some(gwendylon), Some((tx, ty))) =
            (self.characters.get(&gwendylon_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(gwendylon.x), i32::from(gwendylon.y), tx, ty)
            {
                if let Some(gwendylon_mut) = self.characters.get_mut(&gwendylon_id) {
                    let _ = turn(gwendylon_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; } do_idle(cn, TICKS);` (`gwendylon.c:661-672`).
        // The NPC's post position (C's `tmpx`/`tmpy`) reuses `rest_x`/
        // `rest_y`, the same substitution `world::camhermit`/`world::yoakin`/
        // `world::terion` already use for other stationary NPCs' spawn tiles.
        let last_talk = if let Some(gwendylon) = self.characters.get(&gwendylon_id) {
            match gwendylon.driver_state.as_ref() {
                Some(CharacterDriverState::Gwendylon(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + GWENDYLON_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(gwendylon) = self.characters.get(&gwendylon_id) else {
                return;
            };
            let (post_x, post_y) = (gwendylon.rest_x, gwendylon.rest_y);
            self.secure_move_driver(
                gwendylon_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `gwendylon_driver`'s `NT_CHAR` branch (`gwendylon.c:250-480`).
    /// Returns `true` only for the `GWENDYLON_STATE_DONE_BLESS` early
    /// `return` case - see the module doc comment.
    #[allow(clippy::too_many_arguments)]
    fn gwendylon_handle_char_message(
        &mut self,
        gwendylon_id: CharacterId,
        data: &mut GwendylonDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GwendylonPlayerFacts>,
        now: i32,
        events: &mut Vec<GwendylonOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) -> bool {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(gwendylon) = self.characters.get(&gwendylon_id).cloned() else {
            return false;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return false;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:254-257`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return false;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:260-263`).
        if player.driver == CDR_LOSTCON {
            return false;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`gwendylon.c:266-269`).
        if tick < data.last_talk + GWENDYLON_TALK_MIN_TICKS {
            return false;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`gwendylon.c:271-274`) -
        // note the extra `dat->current_victim &&` truthy gate here, unlike
        // `world::yoakin`'s plain `!=` (a genuine per-NPC asymmetry in the
        // C source, preserved as-is).
        if tick < data.last_talk + GWENDYLON_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim.map(|victim| victim.0) != Some(player_id.0)
        {
            return false;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:276-280`).
        if gwendylon_id == player_id
            || !char_see_char(&gwendylon, &player, &self.map, self.date.daylight)
        {
            return false;
        }
        // C `if (char_dist(cn, co) > 16) continue;` (`gwendylon.c:282-285`).
        if char_dist(&gwendylon, &player) > GWENDYLON_GREET_DISTANCE {
            return false;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return false;
        };

        let mut didsay = false;
        let mut new_state = facts.state;
        let mut early_return = false;

        if facts.state == GWENDYLON_STATE_ENTRY {
            self.npc_quiet_say(
                gwendylon_id,
                &format!("Welcome {}! I am {} the Mage.", player.name, gwendylon.name),
            );
            events.push(GwendylonOutcomeEvent::QuestOpen {
                player_id,
                quest: QLOG_GWENDY_FIRST_SKULL,
            });
            new_state = GWENDYLON_STATE_FIRST_SKULL_1;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_FIRST_SKULL_1 {
            self.npc_quiet_say(
                gwendylon_id,
                "South of my tower lies an old ruin. 'Tis inhabitated by skeletons, raised by some evil magic.",
            );
            new_state = GWENDYLON_STATE_FIRST_SKULL_2;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_FIRST_SKULL_2 {
            self.npc_quiet_say(
                gwendylon_id,
                &format!(
                    "I am trying to understand this magic. But I am too old to travel there, so I couldst use thine help, {}. Wouldst thou go thither and look for magical items?",
                    player.name
                ),
            );
            new_state = GWENDYLON_STATE_FIRST_SKULL_3;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_FIRST_SKULL_3 {
            self.npc_quiet_say(
                gwendylon_id,
                "It could be anything: An enhanced bone, an ancient jewel, even a magical spoon of doom.",
            );
            new_state = GWENDYLON_STATE_FIRST_SKULL_4;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_FIRST_SKULL_4 {
            self.npc_quiet_say(
                gwendylon_id,
                "If thou couldst find it and bring it to me, I would reward thee.",
            );
            new_state = GWENDYLON_STATE_FIRST_SKULL_WAIT;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_FIRST_SKULL_WAIT {
            if now.saturating_sub(facts.seen_timer) > GWENDYLON_REMINDER_SECONDS {
                self.npc_quiet_say(
                    gwendylon_id,
                    &format!(
                        "Be greeted, {}! Didst thou find anything magical in the skeleton's ruin? Or dost thou want me to repeat mine offer?",
                        player.name
                    ),
                );
                self.notify_area(
                    gwendylon.x,
                    gwendylon.y,
                    NT_NPC,
                    NTID_TUTORIAL,
                    1,
                    player_id.0 as i32,
                );
                didsay = true;
            }
        } else if facts.state == GWENDYLON_STATE_FIRST_SKULL_DONE {
            // C `if (questlog_isdone(co, 2)) { ppd->gwendy_state = 10;
            // break; }` (`gwendylon.c:341-344`).
            if facts.quest2_isdone {
                new_state = GWENDYLON_STATE_SECOND_SKULL_DONE;
            } else {
                self.npc_quiet_say(
                    gwendylon_id,
                    "I have analysed the item thou brought me. It seems there are more places with skeletons close by.",
                );
                events.push(GwendylonOutcomeEvent::QuestOpen {
                    player_id,
                    quest: QLOG_GWENDY_SECOND_SKULL,
                });
                new_state = GWENDYLON_STATE_SECOND_SKULL_1;
                didsay = true;
            }
        } else if facts.state == GWENDYLON_STATE_SECOND_SKULL_1 {
            self.npc_quiet_say(
                gwendylon_id,
                "It has to be close to the first skull. I do not think it is within the same ruins, but it must be close to them. Surely there must be some skeletons close to the entrance.",
            );
            new_state = GWENDYLON_STATE_SECOND_SKULL_2;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_SECOND_SKULL_2 {
            self.npc_quiet_say(
                gwendylon_id,
                "Somewhere in that place, there must be another magical item. Couldst thou bring me that one as well? I would double thine reward.",
            );
            new_state = GWENDYLON_STATE_SECOND_SKULL_WAIT;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_SECOND_SKULL_WAIT {
            if now.saturating_sub(facts.seen_timer) > GWENDYLON_REMINDER_SECONDS {
                self.npc_quiet_say(
                    gwendylon_id,
                    &format!(
                        "Be greeted, {}! Didst thou find anything magical in the other skeleton place? Or dost thou want me to repeat mine offer?",
                        player.name
                    ),
                );
                didsay = true;
            }
        } else if facts.state == GWENDYLON_STATE_SECOND_SKULL_DONE {
            // C `if (questlog_isdone(co, 3)) { ppd->gwendy_state = 13;
            // break; }` (`gwendylon.c:379-382`).
            if facts.quest3_isdone {
                new_state = GWENDYLON_STATE_THIRD_SKULL_WAIT;
            } else {
                self.npc_quiet_say(
                    gwendylon_id,
                    "There must be another of these skulls, adorned with a green jewel. But I have no idea where to look for it. If I had it, I could use all three skulls to locate their maker.",
                );
                events.push(GwendylonOutcomeEvent::QuestOpen {
                    player_id,
                    quest: QLOG_GWENDY_THIRD_SKULL,
                });
                new_state = GWENDYLON_STATE_THIRD_SKULL_1;
                didsay = true;
            }
        } else if facts.state == GWENDYLON_STATE_THIRD_SKULL_1 {
            self.npc_quiet_say(
                gwendylon_id,
                &format!(
                    "Oh, if thou couldst find it, {}, I would be very grateful. Pray thee {}, locate this skull and bring it to me.",
                    player.name, player.name
                ),
            );
            new_state = GWENDYLON_STATE_THIRD_SKULL_2;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_THIRD_SKULL_2 {
            self.npc_quiet_say(
                gwendylon_id,
                "Maybe Nook could help you with finding it, he often knows of the rumors around town.",
            );
            new_state = GWENDYLON_STATE_THIRD_SKULL_WAIT;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_THIRD_SKULL_WAIT {
            if now.saturating_sub(facts.seen_timer) > GWENDYLON_REMINDER_SECONDS {
                self.npc_quiet_say(
                    gwendylon_id,
                    &format!(
                        "Ah, {}! Didst thou find the skull? It really is of the utmost importance. Or dost thou want me to repeat mine offer?",
                        player.name
                    ),
                );
                didsay = true;
            }
        } else if facts.state == GWENDYLON_STATE_THIRD_SKULL_DONE {
            // C `if (questlog_isdone(co, 4)) { ppd->gwendy_state = 17;
            // break; }` (`gwendylon.c:418-421`).
            if facts.quest4_isdone {
                new_state = GWENDYLON_STATE_FOUL_MAGICIAN_WAIT;
            } else {
                self.npc_quiet_say(
                    gwendylon_id,
                    "This is most disturbing. My analysis shows that the skulls were created in this very tower.",
                );
                events.push(GwendylonOutcomeEvent::QuestOpen {
                    player_id,
                    quest: QLOG_GWENDY_FOUL_MAGICIAN,
                });
                new_state = GWENDYLON_STATE_FOUL_MAGICIAN_1;
                didsay = true;
            }
        } else if facts.state == GWENDYLON_STATE_FOUL_MAGICIAN_1 {
            self.npc_quiet_say(
                gwendylon_id,
                &format!(
                    "But I know every room in here. How can this be? Thou hast been most clever so far, {}. If thou couldst search the tower?",
                    player.name
                ),
            );
            new_state = GWENDYLON_STATE_FOUL_MAGICIAN_2;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_FOUL_MAGICIAN_2 {
            self.npc_quiet_say(
                gwendylon_id,
                "If thou dost find that foul magician, make certain thou killest him. I am certain he does have a fourth skull. Please bring it to me so I can destroy it.",
            );
            new_state = GWENDYLON_STATE_FOUL_MAGICIAN_WAIT;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_FOUL_MAGICIAN_WAIT {
            if now.saturating_sub(facts.seen_timer) > GWENDYLON_REMINDER_SECONDS {
                self.npc_quiet_say(
                    gwendylon_id,
                    &format!(
                        "Ah, {}! I am most concerned. Didst thou find anything? Or dost thou want me to repeat what I said about it?",
                        player.name
                    ),
                );
                didsay = true;
            }
        } else if facts.state == GWENDYLON_STATE_FOUL_MAGICIAN_DONE {
            self.npc_quiet_say(
                gwendylon_id,
                &format!("I do not have any more work for thee, {}.", player.name),
            );
            new_state = GWENDYLON_STATE_DONE_BLESS;
            didsay = true;
        } else if facts.state == GWENDYLON_STATE_DONE_BLESS
            && now.saturating_sub(facts.seen_timer) > GWENDYLON_REMINDER_SECONDS
        {
            self.npc_quiet_say(gwendylon_id, &format!("Nice to see you, {}.", player.name));
            if self.setup_bless_spell(gwendylon_id, player_id) {
                events.push(GwendylonOutcomeEvent::UpdateSeenTimer {
                    player_id,
                    value: now,
                });
                early_return = true;
            } else {
                didsay = true;
            }
        }
        // Every other value (`>= 20`, unreachable in practice): no-op,
        // matching C's `switch` with no matching `case`.

        if early_return {
            // C's `return` (`gwendylon.c:466`) skips the `last_talk`/
            // `current_victim`/`talkdir` update entirely too - nothing
            // further to do here.
            return true;
        }

        if new_state != facts.state {
            events.push(GwendylonOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }
        // C `ppd->gwendy_seen_timer = realtime;` (`gwendylon.c:473`):
        // unconditional (this point is only reached when the `return` above
        // was *not* taken).
        events.push(GwendylonOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:474-478`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
        false
    }

    /// C `gwendylon_driver`'s `NT_TEXT` branch (`gwendylon.c:483-528`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::camhermit`/`world::yoakin`/`world::terion`'s text
    /// handlers).
    fn gwendylon_handle_text_message(
        &mut self,
        gwendylon_id: CharacterId,
        gwendylon_name: &str,
        data: &mut GwendylonDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GwendylonPlayerFacts>,
        events: &mut Vec<GwendylonOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:486-488`).
        let tick = self.tick.0;
        if tick > data.last_talk + GWENDYLON_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:490-493`).
        if let Some(current_victim) = data.current_victim {
            if current_victim != speaker_id {
                return;
            }
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`gwendylon.c:136-
        // 149`): ignore our own talk, non-players, distance > 12,
        // not-visible.
        if gwendylon_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(gwendylon) = self.characters.get(&gwendylon_id).cloned() else {
            return;
        };
        if char_dist(&gwendylon, &speaker) > GWENDYLON_QA_DISTANCE
            || !char_see_char(&gwendylon, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, gwendylon_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(gwendylon_id, &reply);
                didsay = true;
            }
            // C `case 2: // Said Repeat` (`gwendylon.c:496-521`): four
            // disjoint `if`s, each resetting `gwendy_state` back to a
            // checkpoint and zeroing `last_talk` - at most one applies
            // since the ranges don't overlap.
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    let reset_state = if facts.state <= GWENDYLON_STATE_FIRST_SKULL_WAIT {
                        Some(GWENDYLON_STATE_FIRST_SKULL_1)
                    } else if (GWENDYLON_STATE_FIRST_SKULL_DONE..=GWENDYLON_STATE_SECOND_SKULL_WAIT)
                        .contains(&facts.state)
                    {
                        Some(GWENDYLON_STATE_FIRST_SKULL_DONE)
                    } else if (GWENDYLON_STATE_SECOND_SKULL_DONE..=GWENDYLON_STATE_THIRD_SKULL_WAIT)
                        .contains(&facts.state)
                    {
                        Some(GWENDYLON_STATE_SECOND_SKULL_DONE)
                    } else if (GWENDYLON_STATE_THIRD_SKULL_DONE
                        ..=GWENDYLON_STATE_FOUL_MAGICIAN_WAIT)
                        .contains(&facts.state)
                    {
                        Some(GWENDYLON_STATE_THIRD_SKULL_DONE)
                    } else if facts.state >= GWENDYLON_STATE_FOUL_MAGICIAN_DONE {
                        Some(GWENDYLON_STATE_FOUL_MAGICIAN_DONE)
                    } else {
                        None
                    };
                    if let Some(new_state) = reset_state {
                        data.last_talk = 0;
                        events.push(GwendylonOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by gwendylon's own C
            // `switch` (only meaningful to other area-1 NPCs' text
            // handlers) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:524-527`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2` branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `gwendylon_driver`'s `NT_GIVE` branch (`gwendylon.c:531-650`).
    fn gwendylon_handle_give_message(
        &mut self,
        gwendylon_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GwendylonPlayerFacts>,
        events: &mut Vec<GwendylonOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&gwendylon_id)
            .and_then(|gwendylon| gwendylon.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let facts = player_facts.get(&giver_id).copied();
        let giver_name = self
            .characters
            .get(&giver_id)
            .map(|giver| giver.name.clone())
            .unwrap_or_default();

        if template_id == IID_AREA1_SKELSKULL
            && facts.is_some_and(|facts| facts.state <= GWENDYLON_STATE_FIRST_SKULL_WAIT)
        {
            let facts = facts.expect("checked above");
            self.gwendylon_turn_in_skull(
                gwendylon_id,
                giver_id,
                &giver_name,
                item_id,
                events,
                QLOG_GWENDY_FIRST_SKULL,
                GWENDYLON_STATE_FIRST_SKULL_DONE,
                &[
                    IID_AREA1_SKELSKULL,
                    IID_AREA1_SKELKEY1,
                    IID_AREA1_SKELKEY2,
                    IID_AREA1_SKELKEY3,
                ],
                facts.quest1_done_count,
                MONEY_AREA1_SKULL1,
                "Ahh, yes, that might be the thing I was looking for. Thank thee, ",
            );
        } else if template_id == IID_AREA1_WOODSKULL
            && facts.is_some_and(|facts| {
                (GWENDYLON_STATE_FIRST_SKULL_DONE..=GWENDYLON_STATE_SECOND_SKULL_WAIT)
                    .contains(&facts.state)
            })
        {
            let facts = facts.expect("checked above");
            self.gwendylon_turn_in_skull(
                gwendylon_id,
                giver_id,
                &giver_name,
                item_id,
                events,
                QLOG_GWENDY_SECOND_SKULL,
                GWENDYLON_STATE_SECOND_SKULL_DONE,
                &[IID_AREA1_WOODSKULL, IID_AREA1_WOODKEY],
                facts.quest2_done_count,
                MONEY_AREA1_SKULL2,
                "Ahh, yes, this is the thing I was looking for. Thank thee, ",
            );
        } else if template_id == IID_AREA1_MAGESKULL
            && facts.is_some_and(|facts| {
                (GWENDYLON_STATE_SECOND_SKULL_DONE..=GWENDYLON_STATE_THIRD_SKULL_WAIT)
                    .contains(&facts.state)
            })
        {
            let facts = facts.expect("checked above");
            self.gwendylon_turn_in_skull(
                gwendylon_id,
                giver_id,
                &giver_name,
                item_id,
                events,
                QLOG_GWENDY_THIRD_SKULL,
                GWENDYLON_STATE_THIRD_SKULL_DONE,
                &[IID_AREA1_MAGESKULL],
                facts.quest3_done_count,
                MONEY_AREA1_SKULL3,
                "Ahh, yes, this is the third skull. Thank thee, ",
            );
        } else if template_id == IID_AREA1_WARLOCKSKULL
            && facts.is_some_and(|facts| {
                (GWENDYLON_STATE_THIRD_SKULL_DONE..=GWENDYLON_STATE_FOUL_MAGICIAN_WAIT)
                    .contains(&facts.state)
            })
        {
            let facts = facts.expect("checked above");
            self.gwendylon_turn_in_skull(
                gwendylon_id,
                giver_id,
                &giver_name,
                item_id,
                events,
                QLOG_GWENDY_FOUL_MAGICIAN,
                GWENDYLON_STATE_FOUL_MAGICIAN_DONE,
                &[IID_AREA1_WARLOCKSKULL, IID_AREA1_WARLOCKKEY],
                facts.quest4_done_count,
                MONEY_AREA1_SKULL4,
                "This is the last skull! The evil mage must be dead then. I thank thee, ",
            );
        } else if template_id == IID_CALIGARLETTER {
            self.npc_quiet_say(
                gwendylon_id,
                "Hmm, I see. Well, I can teleport you to the area but I am uncertain of what will be there waiting for you. Be prepared adventurer. I would not trust those mages as far as I could throw them!",
            );
            self.queue_system_text(
                giver_id,
                "While you are still trying to figure out how far Gwendylon might be able to throw those mages he quickly mutters a spell and teleports you.".to_string(),
            );
            // C `if (!give_char_item_smart(co, ch[cn].citem, 1)) {
            // destroy_item(ch[cn].citem); }` - `give_char_item_smart`
            // already destroys the item internally on every failure path,
            // so no extra destroy call is needed here.
            self.give_char_item_smart(giver_id, item_id, true);
            // C `if (!change_area(co, 36, 240, 10)) { quiet_say(cn, "Uh-Oh.
            // ..."); }` (`gwendylon.c:637-640`) - deferred to `ugaris-server`
            // since `World` has no DB/session access; see the module doc
            // comment.
            self.pending_gwendylon_cross_area_transfers
                .push(GwendylonCrossAreaTransfer {
                    gwendylon_id,
                    player_id: giver_id,
                });
        } else {
            self.npc_quiet_say(
                gwendylon_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            self.give_char_item_smart(giver_id, item_id, true);
        }
    }

    /// Shared tail of every skull turn-in branch (`gwendylon.c:538-622`):
    /// say the acceptance line, `questlog_done`, sweep the matching quest
    /// items from the player's own inventory, advance `gwendy_state`,
    /// destroy the held skull, and reward gold on first completion only
    /// (C `if (tmp == 1)`).
    #[allow(clippy::too_many_arguments)]
    fn gwendylon_turn_in_skull(
        &mut self,
        gwendylon_id: CharacterId,
        giver_id: CharacterId,
        giver_name: &str,
        held_item_id: ItemId,
        events: &mut Vec<GwendylonOutcomeEvent>,
        quest: usize,
        new_state: i32,
        sweep_template_ids: &[u32],
        prior_done_count: u8,
        reward_amount: i64,
        acceptance_prefix: &str,
    ) {
        self.npc_quiet_say(gwendylon_id, &format!("{acceptance_prefix}{giver_name}."));
        events.push(GwendylonOutcomeEvent::QuestDone {
            player_id: giver_id,
            quest,
        });
        for template_id in sweep_template_ids {
            self.destroy_items_by_template_id(giver_id, *template_id);
        }
        events.push(GwendylonOutcomeEvent::UpdateState {
            player_id: giver_id,
            new_state,
        });

        // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;` (the held
        // skull item Gwendylon just received, distinct from the sweep
        // above which targets the *player's own* remaining inventory).
        self.destroy_item(held_item_id);

        // C `if (tmp == 1)` (only reward gold the first time this quest is
        // completed).
        if prior_done_count == 0 {
            let amount = reward_amount.max(0) as u32;
            if let Some(giver) = self.characters.get_mut(&giver_id) {
                giver.gold = giver.gold.saturating_add(amount);
                giver.flags.insert(CharacterFlags::ITEMS);
            }
            self.queue_system_text_bytes(giver_id, give_money_message(amount));
            events.push(GwendylonOutcomeEvent::GoldEarned {
                player_id: giver_id,
                amount,
            });
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct gwendylon_driver_data` (`src/area/1/gwendylon.c:227-232`): the
/// main quest-giver mage's own driver memory (`CDR_GWENDYLON`, distinct
/// from the per-player `gwendy_state`/`gwendy_seen_timer` fields in
/// `crate::player::PlayerRuntime`'s `area1_ppd` - see `world::gwendylon`'s
/// module doc comment for the split). The C struct's `nighttime`/`giveto`
/// fields are never read or written anywhere in `gwendylon_driver`'s body -
/// dead even in C - so they are not ported.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GwendylonDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// C `struct qa qa[]` from `src/area/1/gwendylon.c:87-108` - the small-talk
/// table `analyse_text_driver`'s own local copy in this file feeds every
/// area-1 NPC driver that calls it (`gwendylon_driver`, `camhermit_driver`,
/// `yoakin_driver`, etc.), not just one. Unlike [`MERCHANT_QA`]/
/// [`GATEKEEPER_QA`], most of the non-canned-answer codes here
/// (`3` advice, `4` buy advice, `9` promise/word/oath, `10` raiseme, `11`
/// hardcore, `12` learn/accept-the-rules) are only meaningful to
/// `gwendylon_driver` itself (Gwendylon is the tutorial/hardcore-mode NPC);
/// every other area-1 driver's own `switch` only ever cases on `2`
/// (repeat/restart) and, for `gwendylon_driver` alone, `13` (repeat all) -
/// any other matched code just counts as `didsay` with no further effect,
/// exactly like `GATEKEEPER_QA`'s `"aye"`/`"nay"` codes.
pub const GWENDYLON_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm fine!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hello"],
        answer: Some("Hello, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hi"],
        answer: Some("Hi, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["greetings"],
        answer: Some("Greetings, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hail"],
        answer: Some("And hail to you, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what", "is", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["repeat", "all"],
        answer: None,
        answer_code: 13,
    },
    TextQaEntry {
        words: &["restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["advice"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["buy", "advice"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["promise"],
        answer: None,
        answer_code: 9,
    },
    TextQaEntry {
        words: &["word"],
        answer: None,
        answer_code: 9,
    },
    TextQaEntry {
        words: &["oath"],
        answer: None,
        answer_code: 9,
    },
    TextQaEntry {
        words: &["raiseme"],
        answer: None,
        answer_code: 10,
    },
    TextQaEntry {
        words: &["hardcore"],
        answer: None,
        answer_code: 11,
    },
    TextQaEntry {
        words: &[
            "i",
            "accept",
            "the",
            "rules",
            "and",
            "wish",
            "to",
            "become",
            "a",
            "hardcore",
            "character",
        ],
        answer: None,
        answer_code: 12,
    },
    TextQaEntry {
        words: &["learn"],
        answer: None,
        answer_code: 12,
    },
];
