//! Clerk NPC (`CDR_ARKHATACLERK`), the Fortress clerk who runs "The
//! Traitors" (quest 76).
//!
//! Ports `src/area/37/arkhata.c::clerk_driver` (`:3591-3832`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! ramin`/`trainer`/`kidnappee`: the caller supplies a per-player fact
//! snapshot ([`ClerkPlayerFacts`]) up front and applies the returned
//! [`ClerkOutcomeEvent`]s afterwards, since `arkhata_ppd.clerk_state`/
//! `clerk_time`/`clerk_bits` live on `crate::player::PlayerRuntime`, not
//! `World`.
//!
//! `clerk_driver`'s seven-state (`0`-`6`) dialogue/timer chain, gated at
//! one point on cross-driver state this file cannot see directly (read
//! via [`ClerkPlayerFacts`]):
//! - `0` needs `arkhata_ppd.captain_state >= 5` (`world::npc::area37::
//!   captain`'s own progress) to advance; C's own `case 0` falls through
//!   into `case 1`'s speech/`questlog_open(76)`/advance-to-`2` in the
//!   same tick - collapsed into one `rs == 0` arm here, same
//!   "fallthrough lands on the next case's action" precedent as
//!   `world::npc::area37::ramin`'s own `rs == 0`/`9`/`11` arms.
//! - `4` is a pure wait state: waiting for the player to say "Aye"
//!   (`NT_TEXT` code `6`, [`super::ARKHATA_QA`]'s `"aye"` row), handled
//!   entirely by this file's own `NT_TEXT` branch.
//! - `5` is the `CLERKTIME` (`60*15` real-world seconds) countdown: on
//!   every subsequent `NT_CHAR` sighting while still `5`, C re-checks
//!   `realtime - clerk_time > CLERKTIME` and, if expired, fails the quest
//!   in place (advances to `6` with a "too late" line) - reproduced as
//!   the `rs == 5` arm's own inline check rather than a separate timer
//!   tick, matching C's own "only re-evaluated on sighting" shape.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim.
//! - The `NT_GIVE` three-note turn-in (`IID_ARKHATA_NOTE1`/`2`/`3`,
//!   `arkhata.c:3759-3813`) has a genuine C quirk reproduced verbatim:
//!   only the *third note's own branch* (`IID_ARKHATA_NOTE3`) sets
//!   `clerk_state = 6` when `clerk_bits` reaches `(1|2|4)` - the NOTE1/
//!   NOTE2 branches complete the quest (`questlog_done`) on the same
//!   condition but never advance `clerk_state` past `5`. If the player's
//!   final note happens to be NOTE1 or NOTE2 rather than NOTE3, the quest
//!   is marked done but `clerk_state` is stuck at `5` forever (C's own
//!   bug, not "fixed" here).
//! - `NT_GIVE`'s "Aye"/"watch" stopwatch creation (`create_item
//!   ("stopwatch")`, `arkhata.c:3728-3731`/`:3745-3748`) needs
//!   `ZoneLoader` item instantiation, which `World` cannot do - both
//!   collapse into one [`ClerkOutcomeEvent::GiveStopwatch`] variant
//!   `ugaris-server::area37::apply_clerk_events` applies, same precedent
//!   as `JudgeOutcomeEvent::GiveEntrancePass`.
//! - C `log_char(co, LOG_SYSTEM, 0, "Your stopwatch will vanish...")`
//!   (both the "Aye" and "watch" branches share the identical string) is
//!   ported via [`World::queue_system_text`], same mapping as
//!   `world::npc::area37::kidnappee`'s own `log_char` port.
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `clerk_driver` body at all (matching the `rammy`/`ramin`/`trainer`
//!   "pure talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:3831`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_ARKHATA_NOTE1, IID_ARKHATA_NOTE2, IID_ARKHATA_NOTE3};
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:3640`, sibling drivers' own
/// identical guard).
const CLERK_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const CLERK_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:3623`).
const CLERK_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:3628`).
const CLERK_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:3825`): idle "return to post" threshold.
const CLERK_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `#define CLERKTIME (60 * 15)` (`arkhata.c:3589`): the "find the
/// traitors" countdown, in wall-clock seconds.
const CLERKTIME: i32 = 60 * 15;
/// C quest 76, "The Traitors".
const QLOG_CLERK_TRAITORS: usize = 76;
/// C `ppd->clerk_bits |= 1/2/4` (`arkhata.c:3766,3780,3794`).
const CLERK_NOTE1_BIT: i32 = 1;
const CLERK_NOTE2_BIT: i32 = 2;
const CLERK_NOTE3_BIT: i32 = 4;
/// C `ppd->clerk_bits == (1 | 2 | 4)` (`arkhata.c:3767,3781,3795`).
const CLERK_ALL_NOTES_BITS: i32 = CLERK_NOTE1_BIT | CLERK_NOTE2_BIT | CLERK_NOTE3_BIT;

/// Per-player facts [`World::process_clerk_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClerkPlayerFacts {
    /// `PlayerRuntime::arkhata_clerk_state()`.
    pub clerk_state: i32,
    /// `PlayerRuntime::arkhata_clerk_time_seconds()`.
    pub clerk_time: i32,
    /// `PlayerRuntime::arkhata_clerk_bits()`.
    pub clerk_bits: i32,
    /// `PlayerRuntime::arkhata_captain_state()` (`ppd->captain_state`,
    /// `arkhata.c:3651`): gates `rs` `0`.
    pub captain_state: i32,
    /// `Character::flags.contains(CharacterFlags::GOD)` (`arkhata.c:
    /// 3718`): the "Aye" branch's admin bypass.
    pub is_god: bool,
}

/// A side effect [`World::process_clerk_actions`] could not apply
/// directly because it touches `PlayerRuntime` or needs `ZoneLoader`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClerkOutcomeEvent {
    /// Write the new `arkhata_ppd.clerk_state` back, `clerk_time`
    /// untouched.
    UpdateClerkState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->clerk_time = realtime; ppd->clerk_state = 5;` (`arkhata.c:
    /// 3719-3720`), the "Aye" branch's countdown start.
    StartClerkTimer {
        player_id: CharacterId,
        realtime_seconds: i32,
    },
    /// C `questlog_open(co, 76)` (`arkhata.c:3658`).
    QuestOpen76 { player_id: CharacterId },
    /// C `questlog_done(co, 76)` (`arkhata.c:3769,3783,3797`).
    QuestDone76 { player_id: CharacterId },
    /// Write the new `arkhata_ppd.clerk_bits` back.
    UpdateClerkBits { player_id: CharacterId, bits: i32 },
    /// C `create_item("stopwatch")` (`arkhata.c:3728`/`:3745`) plus its
    /// `log_char` flavor line - needs `ZoneLoader`.
    GiveStopwatch { player_id: CharacterId },
}

impl World {
    /// C `clerk_driver`'s per-tick body (`arkhata.c:3591-3832`).
    pub fn process_clerk_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ClerkPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<ClerkOutcomeEvent> {
        let clerk_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ARKHATACLERK
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for clerk_id in clerk_ids {
            self.process_clerk_messages(clerk_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    #[allow(clippy::too_many_arguments)]
    fn process_clerk_messages(
        &mut self,
        clerk_id: CharacterId,
        player_facts: &HashMap<CharacterId, ClerkPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<ClerkOutcomeEvent>,
    ) {
        let Some(clerk_name) = self
            .characters
            .get(&clerk_id)
            .map(|clerk| clerk.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Clerk(mut data)) = self
            .characters
            .get(&clerk_id)
            .and_then(|clerk| clerk.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&clerk_id)
            .map(|clerk| std::mem::take(&mut clerk.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.clerk_handle_char_message(
                    clerk_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.clerk_handle_text_message(
                    clerk_id,
                    &clerk_name,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.clerk_handle_give_message(clerk_id, message, now, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(clerk) = self.characters.get_mut(&clerk_id) {
            clerk.driver_state = Some(CharacterDriverState::Clerk(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:3821-3823`).
        if let (Some(clerk), Some((tx, ty))) =
            (self.characters.get(&clerk_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(clerk.x), i32::from(clerk.y), tx, ty) {
                if let Some(clerk_mut) = self.characters.get_mut(&clerk_id) {
                    let _ = turn(clerk_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:3825-3829`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(clerk) = self.characters.get(&clerk_id) {
            match clerk.driver_state.as_ref() {
                Some(CharacterDriverState::Clerk(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + CLERK_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(clerk) = self.characters.get(&clerk_id) else {
                return;
            };
            let (post_x, post_y) = (clerk.rest_x, clerk.rest_y);
            self.secure_move_driver(
                clerk_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `clerk_driver`'s `NT_CHAR` branch (`arkhata.c:3606-3693`).
    #[allow(clippy::too_many_arguments)]
    fn clerk_handle_char_message(
        &mut self,
        clerk_id: CharacterId,
        data: &mut ClerkDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ClerkPlayerFacts>,
        now: i32,
        events: &mut Vec<ClerkOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(clerk) = self.characters.get(&clerk_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:3610`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:3616`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:3622`).
        if tick < data.last_talk + CLERK_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:3627`).
        if tick < data.last_talk + CLERK_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:3633`).
        if clerk_id == player_id || !char_see_char(&clerk, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:3639`).
        if char_dist(&clerk, &player) > CLERK_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.clerk_state;
        match facts.clerk_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:
            // 3649-3660`) - see the module doc comment.
            0 if facts.captain_state >= 5 => {
                self.npc_quiet_say(clerk_id, "So the Captain sent thee to me? That is good.");
                events.push(ClerkOutcomeEvent::QuestOpen76 { player_id });
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:3662-3667`).
            2 => {
                self.npc_quiet_say(
                    clerk_id,
                    "Three notes containing information about a transport from Brannington to Arkhata due tomorow night are missing. Most likely the traitors will pass these notes to the robbers tonight.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:3669-3674`).
            3 => {
                self.npc_quiet_say(
                    clerk_id,
                    "Are thou able to find these traitors in time? You will have three hours (Astonia time)! Say Aye when you are ready!",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4: break;` (`arkhata.c:3675-3676`): waiting for the
            // player's "Aye".
            4 => {}
            // C `case 5:` (`arkhata.c:3677-3683`) - see the module doc
            // comment.
            5 => {
                if now.saturating_sub(facts.clerk_time) > CLERKTIME {
                    self.npc_quiet_say(
                        clerk_id,
                        &format!("Thou art too late, {}. Thou hast failed me.", player.name),
                    );
                    new_state = 6;
                    didsay = true;
                }
            }
            // C `case 6: break;` (`arkhata.c:3684-3685`): all done.
            6 => {}
            _ => {}
        }

        if new_state != facts.clerk_state {
            events.push(ClerkOutcomeEvent::UpdateClerkState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:3687-3691`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `clerk_driver`'s `NT_TEXT` branch (`arkhata.c:3696-3756`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn clerk_handle_text_message(
        &mut self,
        clerk_id: CharacterId,
        clerk_name: &str,
        data: &mut ClerkDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ClerkPlayerFacts>,
        now: i32,
        events: &mut Vec<ClerkOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:3699-3701`).
        let tick = self.tick.0;
        if tick > data.last_talk + CLERK_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:3703`).
        if data.current_victim.is_some() && data.current_victim != Some(speaker_id) {
            return;
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(clerk) = self.characters.get(&clerk_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if clerk_id == speaker_id {
            return;
        }
        if char_dist(&clerk, &speaker) > CLERK_QA_DISTANCE
            || !char_see_char(&clerk, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let facts = player_facts.get(&speaker_id).copied();
        let clerk_state = facts.map(|facts| facts.clerk_state).unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, clerk_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(clerk_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:3708-3715`): rewind to state
            // 1 while the introduction window (`1..=4`) is open.
            TextAnalysisOutcome::Matched(2) => {
                if clerk_state > 0 && clerk_state <= 4 {
                    data.last_talk = 0;
                    events.push(ClerkOutcomeEvent::UpdateClerkState {
                        player_id: speaker_id,
                        new_state: 1,
                    });
                }
                didsay = true;
            }
            // "aye" (`arkhata.c:3716-3736`): starts the countdown.
            TextAnalysisOutcome::Matched(6) => {
                if facts.is_some_and(|facts| facts.clerk_state == 4 || facts.is_god) {
                    events.push(ClerkOutcomeEvent::StartClerkTimer {
                        player_id: speaker_id,
                        realtime_seconds: now,
                    });
                    self.npc_quiet_say(
                        clerk_id,
                        "Very good, you have 3 hours (astonia time) to retrieve the notes. Good Luck!",
                    );
                    self.queue_system_text(
                        speaker_id,
                        "Your stopwatch will vanish if you leave the area or log off. The quest, however, will still be open and the three hours will continue to run out. The clerk will give you a new watch if you ask him for a 'watch'.",
                    );
                    events.push(ClerkOutcomeEvent::GiveStopwatch {
                        player_id: speaker_id,
                    });
                    self.destroy_items_by_template_id(speaker_id, IID_ARKHATA_NOTE1);
                    self.destroy_items_by_template_id(speaker_id, IID_ARKHATA_NOTE2);
                    self.destroy_items_by_template_id(speaker_id, IID_ARKHATA_NOTE3);
                }
                didsay = true;
            }
            // "watch" (`arkhata.c:3737-3750`): replaces a lost stopwatch.
            TextAnalysisOutcome::Matched(7) => {
                if facts.is_some_and(|facts| facts.clerk_state == 5) {
                    self.queue_system_text(
                        speaker_id,
                        "Your stopwatch will vanish if you leave the area or log off. The quest, however, will still be open and the three hours will continue to run out. The clerk will give you a new watch if you ask him for a 'watch'.",
                    );
                    events.push(ClerkOutcomeEvent::GiveStopwatch {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)) is unhandled by clerk's own `switch` but still
            // counts as `didsay` (C: `switch (didsay = analyse_text_driver
            // (...))` - any nonzero return is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:3752-3755`) - note this does *not* touch `dat->
        // last_talk` (except the "repeat" branch's own explicit reset
        // above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `clerk_driver`'s `NT_GIVE` branch (`arkhata.c:3759-3814`) - see
    /// the module doc comment for the `clerk_state = 6`-only-on-NOTE3
    /// quirk this reproduces verbatim.
    fn clerk_handle_give_message(
        &mut self,
        clerk_id: CharacterId,
        message: &CharacterDriverMessage,
        now: i32,
        player_facts: &HashMap<CharacterId, ClerkPlayerFacts>,
        events: &mut Vec<ClerkOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&clerk_id)
            .and_then(|clerk| clerk.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let is_player = giver.flags.contains(CharacterFlags::PLAYER);
        let facts = player_facts.get(&giver_id).copied();
        let within_time =
            facts.is_some_and(|facts| now.saturating_sub(facts.clerk_time) < CLERKTIME);

        for (template_id, bit, sets_state_six) in [
            (IID_ARKHATA_NOTE1, CLERK_NOTE1_BIT, false),
            (IID_ARKHATA_NOTE2, CLERK_NOTE2_BIT, false),
            (IID_ARKHATA_NOTE3, CLERK_NOTE3_BIT, true),
        ] {
            let eligible = item.template_id == template_id
                && is_player
                && within_time
                && facts.is_some_and(|facts| facts.clerk_state == 5 && facts.clerk_bits & bit == 0);
            if !eligible {
                continue;
            }
            self.destroy_items_by_template_id(giver_id, template_id);
            let new_bits = facts.map(|facts| facts.clerk_bits | bit).unwrap_or(bit);
            events.push(ClerkOutcomeEvent::UpdateClerkBits {
                player_id: giver_id,
                bits: new_bits,
            });
            if new_bits == CLERK_ALL_NOTES_BITS {
                self.npc_quiet_say(
                    clerk_id,
                    "You have done a great job, Now the transport will be safe, thank you.",
                );
                events.push(ClerkOutcomeEvent::QuestDone76 {
                    player_id: giver_id,
                });
                if sets_state_six {
                    events.push(ClerkOutcomeEvent::UpdateClerkState {
                        player_id: giver_id,
                        new_state: 6,
                    });
                }
            } else {
                self.npc_quiet_say(clerk_id, "Oh there might be hope after all then.");
            }
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:3806-3811`): hand the
        // item back to the giver.
        self.npc_say(
            clerk_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_ARKHATACLERK;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `clerk_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::ramin`'s `RaminDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClerkDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_CLERK_TRAITORS`] to `ugaris-server`'s
/// `apply_clerk_events`.
pub const fn qlog_clerk_traitors() -> usize {
    QLOG_CLERK_TRAITORS
}
