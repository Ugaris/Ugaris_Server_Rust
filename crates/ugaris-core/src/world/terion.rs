//! Ambient lore NPC (`CDR_TERION`), area 1's village storyteller.
//!
//! Ports `src/area/1/gwendylon.c::terion_driver` (`:1228-1459`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`). Follows the same `World`/
//! `PlayerRuntime` split established there: the caller supplies a
//! per-player fact snapshot ([`TerionPlayerFacts`]) up front and applies
//! the returned [`TerionOutcomeEvent`]s afterwards, since
//! `terion_state`/`gwendy_state`/`reskin_state` (all `area1_ppd` fields)
//! live on `crate::player::PlayerRuntime`, not `World`.
//!
//! Terion is a pure ambient-dialogue NPC: no quest log, no item exchange
//! reward, no gold. Its only quirk not shared with `world::yoakin`/
//! `world::camhermit` is the leading pre-pass over `NT_NPC`/`NTID_DIDSAY`
//! messages (`gwendylon.c:1240-1244`): whenever *another* nearby NPC just
//! finished a line of dialogue (broadcast via `notify_area(..., NT_NPC,
//! NTID_DIDSAY, cn, 0)`, the same self-broadcast every one of these
//! ambient NPCs sends after `didsay`), Terion's own `last_talk` throttle
//! is bumped to the current tick so multiple nearby NPCs don't all launch
//! into dialogue back-to-back. This pre-pass does not consume/remove the
//! message (`remove_message` is only called in the second loop's tail),
//! matching C exactly.
//!
//! Deviations/gaps (documented, not silent):
//! - `struct terion_driver_data`'s `last_walk`/`pos` fields
//!   (`gwendylon.c:1222-1224`) are never read or written anywhere in
//!   `terion_driver`'s body - dead even in C - so they are not ported (see
//!   [`crate::character_driver::TerionDriverData`]'s doc comment).
//! - Unlike `world::yoakin`'s `NT_TEXT` branch, C's own `terion_driver`
//!   has no `current_victim`/`last_talk` gate before calling
//!   `analyse_text_driver` (`gwendylon.c:1421-1424` vs. `gwendylon.c:1118-
//!   1125` for yoakin) - a genuine asymmetry between the two NPCs in the
//!   C source, preserved here rather than "fixed" to match yoakin's shape.

use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    analyse_text_qa, TerionDriverData, TextAnalysisOutcome, CDR_TERION, GWENDYLON_QA, NTID_DIDSAY,
    NTID_TERION,
};
use crate::drvlib::offset2dx;
use crate::quest::{GWENDYLON_STATE_FIRST_SKULL_DONE, GWENDYLON_STATE_SECOND_SKULL_DONE};

/// C `char_dist(cn, co) > 10` (`gwendylon.c:1281`): the `NT_CHAR` greeting
/// range.
const TERION_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const TERION_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gwendylon.c:1266`).
const TERION_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gwendylon.c:1271`).
const TERION_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:1467`): idle "return to post" threshold.
const TERION_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ppd->gwendy_state > GWENDYLON_STATE_SECOND_SKULL_WAIT`
/// (`gwendylon.c:1297`, `src/common/npc_states.h`): named here to match
/// the real C macro (not yet mirrored in `crate::quest`, which only
/// exports the `_DONE` variants `world::terion` also needs).
const GWENDYLON_STATE_SECOND_SKULL_WAIT: i32 = 9;
/// C `ppd->gwendy_state <= GWENDYLON_STATE_THIRD_SKULL_WAIT`
/// (`gwendylon.c:1319`) - also used as the bare literal `13` in case 6
/// (`gwendylon.c:1330`), the same numeric value.
const GWENDYLON_STATE_THIRD_SKULL_WAIT: i32 = 13;
/// C `ppd->reskin_state >= 9` (`gwendylon.c:1359`): no named constant
/// exists in the C source for this threshold.
const RESKIN_STATE_TALKED_ABOUT_BEER: i32 = 9;

/// Terion's bare `int` state values for `ppd->terion_state` - no
/// `#define` names exist in the C source, so these are named here purely
/// for readability.
const TERION_STATE_ENTRY: i32 = 0;
const TERION_STATE_SKELLY_STORY_1: i32 = 1;
const TERION_STATE_SKELLY_STORY_2: i32 = 2;
const TERION_STATE_SKELLY_STORY_3: i32 = 3;
const TERION_STATE_HORDES_GREET: i32 = 4;
const TERION_STATE_YOAKIN_RUINS: i32 = 5;
const TERION_STATE_BRAVE_ADVENTURERS: i32 = 6;
const TERION_STATE_ASTON_DOWNHILL: i32 = 7;
const TERION_STATE_HARD_TIMES: i32 = 8;
const TERION_STATE_SCHOOLS_MAD: i32 = 9;
const TERION_STATE_UNDEAD_SEEN: i32 = 10;
const TERION_STATE_EMPEROR_KILLED: i32 = 11;
const TERION_STATE_DARK_HORDES_SPREAD: i32 = 12;
const TERION_STATE_RESKIN_BEER: i32 = 13;

/// Per-player facts [`World::process_terion_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerionPlayerFacts {
    /// `PlayerRuntime::area1_terion_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_gwendy_state()`: gates the skull-story
    /// intro/advance branches.
    pub gwendy_state: i32,
    /// `PlayerRuntime::area1_reskin_state()`: gates the state 9 -> 10
    /// transition.
    pub reskin_state: i32,
}

/// A side effect [`World::process_terion_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerionOutcomeEvent {
    /// Write the new `area1_ppd.terion_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
}

impl World {
    /// C `terion_driver`'s per-tick body (`gwendylon.c:1228-1472`).
    pub fn process_terion_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TerionPlayerFacts>,
        area_id: u16,
    ) -> Vec<TerionOutcomeEvent> {
        let terion_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TERION
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for terion_id in terion_ids {
            self.process_terion_messages(terion_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_terion_messages(
        &mut self,
        terion_id: CharacterId,
        player_facts: &HashMap<CharacterId, TerionPlayerFacts>,
        area_id: u16,
        events: &mut Vec<TerionOutcomeEvent>,
    ) {
        let Some(terion_name) = self
            .characters
            .get(&terion_id)
            .map(|terion| terion.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Terion(mut data)) = self
            .characters
            .get(&terion_id)
            .and_then(|terion| terion.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&terion_id)
            .map(|terion| std::mem::take(&mut terion.driver_messages))
            .unwrap_or_default();

        // C's first pass over the (not-yet-removed) message queue
        // (`gwendylon.c:1240-1244`): any `NT_NPC`/`NTID_DIDSAY` broadcast
        // from someone else resets our own talk throttle to "just
        // talked".
        for message in &messages {
            if message.message_type == NT_NPC
                && message.dat1 == NTID_DIDSAY
                && message.dat2 != terion_id.0 as i32
            {
                data.last_talk = self.tick.0;
            }
        }

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.terion_handle_char_message(
                    terion_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.terion_handle_text_message(
                    terion_id,
                    &terion_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.terion_handle_give_message(terion_id, message),
                _ => {}
            }
        }

        if let Some(terion) = self.characters.get_mut(&terion_id) {
            terion.driver_state = Some(CharacterDriverState::Terion(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:1464-1466`).
        if let (Some(terion), Some((tx, ty))) =
            (self.characters.get(&terion_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(terion.x), i32::from(terion.y), tx, ty) {
                if let Some(terion_mut) = self.characters.get_mut(&terion_id) {
                    let _ = turn(terion_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; } do_idle(cn, TICKS);` (`gwendylon.c:1468-
        // 1472`). The NPC's post position (C's `tmpx`/`tmpy`) reuses
        // `rest_x`/`rest_y`, the same substitution `world::camhermit`/
        // `world::yoakin` already use for other stationary NPCs' spawn
        // tiles.
        let last_talk = if let Some(terion) = self.characters.get(&terion_id) {
            match terion.driver_state.as_ref() {
                Some(CharacterDriverState::Terion(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + TERION_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(terion) = self.characters.get(&terion_id) else {
                return;
            };
            let (post_x, post_y) = (terion.rest_x, terion.rest_y);
            self.secure_move_driver(
                terion_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `terion_driver`'s `NT_CHAR` branch (`gwendylon.c:1252-1414`).
    #[allow(clippy::too_many_arguments)]
    fn terion_handle_char_message(
        &mut self,
        terion_id: CharacterId,
        data: &mut TerionDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TerionPlayerFacts>,
        events: &mut Vec<TerionOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(terion) = self.characters.get(&terion_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:1256-1259`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:1261-1265`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`gwendylon.c:1267-1270`).
        if tick < data.last_talk + TERION_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`gwendylon.c:1272-1275`).
        if tick < data.last_talk + TERION_TALK_VICTIM_TICKS
            && data.current_victim.map_or(0, |victim| victim.0) != player_id.0
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:1277-1280`).
        if terion_id == player_id || !char_see_char(&terion, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`gwendylon.c:1282-
        // 1285`).
        if char_dist(&terion, &player) > TERION_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.state;

        match facts.state {
            TERION_STATE_ENTRY => {
                // C `case 0:` (`gwendylon.c:1291-1307`).
                if facts.gwendy_state < GWENDYLON_STATE_FIRST_SKULL_DONE {
                    // dont offer skelly stories before skull 2 - silent.
                } else if facts.gwendy_state > GWENDYLON_STATE_SECOND_SKULL_WAIT {
                    // advance to second hint if player solved skull 2
                    // already - silent state jump, no dialogue.
                    new_state = TERION_STATE_HORDES_GREET;
                } else {
                    self.npc_quiet_say(
                        terion_id,
                        &format!("Be greeted, {}! My name is {}.", player.name, terion.name),
                    );
                    new_state = TERION_STATE_SKELLY_STORY_1;
                    didsay = true;
                }
            }
            TERION_STATE_SKELLY_STORY_1 => {
                self.npc_quiet_say(
                    terion_id,
                    "I have heard some stories about skeletons emerging from a hidden dungeon near the old ruins.",
                );
                new_state = TERION_STATE_SKELLY_STORY_2;
                didsay = true;
            }
            TERION_STATE_SKELLY_STORY_2 => {
                self.npc_quiet_say(
                    terion_id,
                    "Some of the lads from the village went looking for them, but had no luck. Or they were lucky, depends on the way thou lookst at it I guess.",
                );
                self.notify_area(
                    terion.x,
                    terion.y,
                    NT_NPC,
                    NTID_TERION,
                    terion_id.0 as i32,
                    1,
                );
                new_state = TERION_STATE_SKELLY_STORY_3;
                didsay = true;
            }
            TERION_STATE_SKELLY_STORY_3 => {
                self.npc_quiet_say(
                    terion_id,
                    "Anyway. They found nothing. Guess that is because they were too fearful to seek out dark corners and hidden places.",
                );
                new_state = TERION_STATE_HORDES_GREET;
                didsay = true;
            }
            TERION_STATE_HORDES_GREET => {
                // C `case 4:` (`gwendylon.c:1320-1326`).
                if facts.gwendy_state >= GWENDYLON_STATE_SECOND_SKULL_DONE
                    && facts.gwendy_state <= GWENDYLON_STATE_THIRD_SKULL_WAIT
                {
                    self.npc_quiet_say(
                        terion_id,
                        &format!(
                            "Be greeted again, {}. I hope this day finds thee well.",
                            player.name
                        ),
                    );
                    new_state = TERION_STATE_YOAKIN_RUINS;
                    didsay = true;
                }
            }
            TERION_STATE_YOAKIN_RUINS => {
                self.npc_quiet_say(
                    terion_id,
                    "I've been thinking about the skeletons, and the skulls Gwendylon are researching. It seems these skeletons are always seen near old ruins. And I remembered that Yoakin the Hunter once told me that his house was built on top of an old ruin.",
                );
                self.notify_area(
                    terion.x,
                    terion.y,
                    NT_NPC,
                    NTID_TERION,
                    terion_id.0 as i32,
                    3,
                );
                new_state = TERION_STATE_BRAVE_ADVENTURERS;
                didsay = true;
            }
            TERION_STATE_BRAVE_ADVENTURERS => {
                // C `case 6:` (`gwendylon.c:1336-1344`).
                if facts.gwendy_state >= GWENDYLON_STATE_THIRD_SKULL_WAIT {
                    let gender_word = if player.flags.contains(CharacterFlags::MALE) {
                        "men"
                    } else {
                        "women"
                    };
                    self.npc_quiet_say(
                        terion_id,
                        &format!(
                            "Ah, {}. 'Tis good to see there are brave {} about who will fight the evil which has been invading our lives lately.",
                            player.name, gender_word
                        ),
                    );
                    new_state = TERION_STATE_ASTON_DOWNHILL;
                    didsay = true;
                }
            }
            TERION_STATE_ASTON_DOWNHILL => {
                self.npc_quiet_say(
                    terion_id,
                    "Ever since the dark hordes attacked Aston, things have been going downhill. Some years ago, we frequently had visitors from Aston and beyond. But today no one dares to travel unless he must.",
                );
                new_state = TERION_STATE_HARD_TIMES;
                didsay = true;
            }
            TERION_STATE_HARD_TIMES => {
                self.npc_quiet_say(
                    terion_id,
                    "And those skeletons all around... We live in hard times.",
                );
                new_state = TERION_STATE_SCHOOLS_MAD;
                didsay = true;
            }
            TERION_STATE_SCHOOLS_MAD => {
                // C `case 9:` (`gwendylon.c:1357-1365`).
                if facts.reskin_state >= RESKIN_STATE_TALKED_ABOUT_BEER {
                    self.npc_quiet_say(
                        terion_id,
                        "Oh, what has become of this world? The two schools gone mad beyond cure, skeletons everywhere. And I've heard rumors that things all over the land are the same.",
                    );
                    new_state = TERION_STATE_UNDEAD_SEEN;
                    didsay = true;
                }
            }
            TERION_STATE_UNDEAD_SEEN => {
                self.npc_quiet_say(
                    terion_id,
                    "Undead, strange beasts and even demons have been seen in Aston. Didst thou know that?",
                );
                new_state = TERION_STATE_EMPEROR_KILLED;
                didsay = true;
            }
            TERION_STATE_EMPEROR_KILLED => {
                self.npc_quiet_say(
                    terion_id,
                    "Some years ago those monsters attacked our capital and killed our emperor. Most of his honor guard, the Seyan'Du, died in his defense, but to no avail. With the emperor dead, and the imperial palace in ruins, there was no one to organize our defenses.",
                );
                new_state = TERION_STATE_DARK_HORDES_SPREAD;
                didsay = true;
            }
            TERION_STATE_DARK_HORDES_SPREAD => {
                self.npc_quiet_say(
                    terion_id,
                    "And so the dark hordes are spreading all over the land.",
                );
                new_state = TERION_STATE_RESKIN_BEER;
                didsay = true;
            }
            TERION_STATE_RESKIN_BEER => {
                self.npc_quiet_say(
                    terion_id,
                    "Hey! Reskin! Art thou certain that thou dost not have any beer? I couldst use a drink now.",
                );
                self.notify_area(
                    terion.x,
                    terion.y,
                    NT_NPC,
                    NTID_TERION,
                    terion_id.0 as i32,
                    5,
                );
                new_state += 1;
                didsay = true;
            }
            // Every other value (>= 14): no-op, matching C's `switch`
            // with no matching `case`.
            _ => {}
        }

        if new_state != facts.state {
            events.push(TerionOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; notify_area(..., NTID_DIDSAY, cn, 0);
        // }` (`gwendylon.c:1409-1413`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
            self.notify_area(
                terion.x,
                terion.y,
                NT_NPC,
                NTID_DIDSAY,
                terion_id.0 as i32,
                0,
            );
        }
    }

    /// C `terion_driver`'s `NT_TEXT` branch (`gwendylon.c:1418-1442`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::camhermit`/`world::yoakin`'s text handlers). Unlike
    /// those two, C's own `terion_driver` has no `current_victim`/
    /// `last_talk` gate here - see the module doc comment.
    fn terion_handle_text_message(
        &mut self,
        terion_id: CharacterId,
        terion_name: &str,
        data: &mut TerionDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TerionPlayerFacts>,
        events: &mut Vec<TerionOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`gwendylon.c:136-
        // 149`): ignore our own talk, non-players, distance > 12,
        // not-visible.
        if terion_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(terion) = self.characters.get(&terion_id).cloned() else {
            return;
        };
        if char_dist(&terion, &speaker) > TERION_QA_DISTANCE
            || !char_see_char(&terion, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, terion_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(terion_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:1422-1433`): four disjoint
            // `if`s, each resetting `terion_state` back to a checkpoint
            // and zeroing `last_talk` - at most one applies since the
            // ranges don't overlap.
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    let reset_state = if facts.state <= TERION_STATE_HORDES_GREET {
                        Some(TERION_STATE_ENTRY)
                    } else if (TERION_STATE_YOAKIN_RUINS..=TERION_STATE_BRAVE_ADVENTURERS)
                        .contains(&facts.state)
                    {
                        Some(TERION_STATE_HORDES_GREET)
                    } else if (TERION_STATE_ASTON_DOWNHILL..=TERION_STATE_SCHOOLS_MAD)
                        .contains(&facts.state)
                    {
                        Some(TERION_STATE_BRAVE_ADVENTURERS)
                    } else if (TERION_STATE_UNDEAD_SEEN..=14).contains(&facts.state) {
                        Some(TERION_STATE_SCHOOLS_MAD)
                    } else {
                        None
                    };
                    if let Some(new_state) = reset_state {
                        data.last_talk = 0;
                        events.push(TerionOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by terion's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:1438-1441`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2` branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `terion_driver`'s `NT_GIVE` branch (`gwendylon.c:1445-1455`).
    fn terion_handle_give_message(
        &mut self,
        terion_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&terion_id)
            .and_then(|terion| terion.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            terion_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        self.give_char_item_smart(giver_id, item_id, true);
    }
}
