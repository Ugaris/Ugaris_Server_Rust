//! Identity-crisis judge/knight/jester NPC (`CDR_NOOK`), area 1's knight
//! castle greeter who hands off to Gwendylon/Lydia and runs the
//! stolen-cap side quest (`QLOG_NOOK`).
//!
//! Ports `src/area/1/gwendylon.c::nook_driver` (`:3175-3449`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`/`world::terion`/`world::gwendylon`/
//! `world::jessica`/`world::jiu`/`world::forest_ranger`/
//! `world::brithildie`). Follows the same `World`/`PlayerRuntime` split
//! established there: the caller supplies a per-player fact snapshot
//! ([`NookPlayerFacts`]) up front and applies the returned
//! [`NookOutcomeEvent`]s afterwards, since `nook_state` (an `area1_ppd`
//! field) and `QLOG_NOOK` live on `crate::player::PlayerRuntime`, not
//! `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - Unlike every other area-1 NPC ported so far, `nook_driver`'s `switch`
//!   has no `realtime`/`seen_timer` reminder gate anywhere in its body
//!   (confirmed: no `ppd->nook_seen_timer` field exists in `area1_ppd` at
//!   all) - every state either advances unconditionally on sight or is a
//!   permanent silent no-op (`break;` with nothing said). So
//!   [`NookPlayerFacts`] carries no `seen_timer` and
//!   [`NookOutcomeEvent`] has no `UpdateSeenTimer` variant - a genuine
//!   structural difference from `world::jessica`/`world::brithildie`.
//! - State 12's `if (ppd->gwendy_state >= GWENDYLON_STATE_DONE_BLESS) {
//!   ppd->nook_state = 16; break; }` (`gwendylon.c:3318-3321`) advances the
//!   state without ever calling `quiet_say` - `didsay` stays false, so
//!   this transition does not update `last_talk`/`current_victim`/facing,
//!   exactly like C.
//! - `IID_AREA1_JESTERCAP`'s `destroy_item_byID` sweep
//!   (`gwendylon.c:3392-3393`) does not reach the account depot (same
//!   documented gap as `world::gwendylon`/`world::yoakin`).
//! - The `NT_GIVE` "unwanted item" give-back (`gwendylon.c:3402-3407`)
//!   calls plain `give_char_item`, not `give_char_item_smart` - no
//!   ground-drop fallback on a full inventory, preserved here via
//!   `World::give_char_item` (same documented asymmetry as
//!   `world::jessica`/`world::brithildie`'s own `NT_GIVE` handlers).
//! - The idle "identity crisis" mutterings (`gwendylon.c:3429-3444`) are a
//!   `Progress Log`-era addition (not upstream C canon flavor text, but
//!   already present in this repository's oracle copy of `gwendylon.c`) -
//!   ported verbatim like every other NPC idle-chatter table
//!   (`world::bank`/`world::merchant`).

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_NOOK, GWENDYLON_QA};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_AREA1_JESTERCAP, IID_AREA1_ROBBERKEY1};
use crate::quest::{GWENDYLON_STATE_SECOND_SKULL_DONE, GWENDYLON_STATE_THIRD_SKULL_DONE};
use crate::world::*;

/// C `char_dist(cn, co) > 16` (`gwendylon.c:3229`): the `NT_CHAR` greeting
/// range.
const NOOK_GREET_DISTANCE: i32 = 16;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const NOOK_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gwendylon.c:3212`).
const NOOK_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gwendylon.c:3217`, `:3349`).
const NOOK_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:3422`): idle "return to post" threshold.
const NOOK_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS * 60` (`gwendylon.c:3429`): idle-mutterings gate.
const NOOK_MUTTER_TICKS: u64 = TICKS_PER_SECOND * 60;

/// C `gwendylon.c:104` (private to `gwendylon_driver`'s own file, so
/// re-declared here rather than imported - same pattern every other
/// area-1 NPC file uses for shared bare-int C state constants).
const GWENDYLON_STATE_DONE_BLESS: i32 = 19;

/// C `nook_mutterings[]` (`gwendylon.c:3430-3443`): the identity-crisis
/// idle-muttering table, 12 entries, indexed by `RANDOM(12)`.
const NOOK_MUTTERINGS: [&str; 12] = [
    "I am Nook, the... the... what was it again?",
    "Judge? Knight? Jester? One of those. Definitely one of those.",
    "Why does everyone laugh when I introduce myself?",
    "I practiced my title in the mirror this morning. Then forgot it by breakfast.",
    "Cousin Jessica never forgets HER title. Show-off.",
    "I wrote my title on my hand once. Then I washed my hands.",
    "The Knight of the Shining... no. The Jester of the... no. The Nook. Just the Nook.",
    "Some days I feel like a judge. Other days, more of a jester. Most days, confused.",
    "Maybe I should just make up a new title entirely. Nook the Magnificent!",
    "I had a dream where I remembered my title. Then I woke up and forgot the dream.",
    "Father always said I'd amount to something. Still waiting on the specifics.",
    "At least I know my NAME. That's a start. Right?",
];

/// Per-player facts [`World::process_nook_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NookPlayerFacts {
    /// `PlayerRuntime::area1_nook_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_gwendy_state()` - gates states 4/5/12.
    pub gwendy_state: i32,
}

/// A side effect [`World::process_nook_actions`] could not apply directly
/// because it touches `PlayerRuntime`. See the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NookOutcomeEvent {
    /// Write the new `area1_ppd.nook_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 6)` (`gwendylon.c:3274`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 6)` (`gwendylon.c:3391`).
    QuestDone { player_id: CharacterId },
}

impl World {
    /// C `nook_driver`'s per-tick body (`gwendylon.c:3175-3449`).
    pub fn process_nook_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, NookPlayerFacts>,
        area_id: u16,
    ) -> Vec<NookOutcomeEvent> {
        let nook_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_NOOK
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for nook_id in nook_ids {
            self.process_nook_messages(nook_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_nook_messages(
        &mut self,
        nook_id: CharacterId,
        player_facts: &HashMap<CharacterId, NookPlayerFacts>,
        area_id: u16,
        events: &mut Vec<NookOutcomeEvent>,
    ) {
        let Some(nook_name) = self.characters.get(&nook_id).map(|nook| nook.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::Nook(mut data)) = self
            .characters
            .get(&nook_id)
            .and_then(|nook| nook.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&nook_id)
            .map(|nook| std::mem::take(&mut nook.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.nook_handle_char_message(
                    nook_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.nook_handle_text_message(
                    nook_id,
                    &nook_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.nook_handle_give_message(nook_id, message, player_facts, events),
                _ => {}
            }
        }

        if let Some(nook) = self.characters.get_mut(&nook_id) {
            nook.driver_state = Some(CharacterDriverState::Nook(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:3418-3420`).
        if let (Some(nook), Some((tx, ty))) = (self.characters.get(&nook_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(nook.x), i32::from(nook.y), tx, ty) {
                if let Some(nook_mut) = self.characters.get_mut(&nook_id) {
                    let _ = turn(nook_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`gwendylon.c:3422-3426`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary area-1 NPC uses.
        let last_talk = if let Some(nook) = self.characters.get(&nook_id) {
            match nook.driver_state.as_ref() {
                Some(CharacterDriverState::Nook(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + NOOK_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(nook) = self.characters.get(&nook_id) else {
                return;
            };
            let (post_x, post_y) = (nook.rest_x, nook.rest_y);
            let moved = self.secure_move_driver(
                nook_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
            if moved {
                return;
            }
        }

        // C's "Nook idle mutterings" block (`gwendylon.c:3429-3445`).
        self.nook_idle_chatter(nook_id, last_talk);
    }

    /// C `nook_driver`'s `NT_CHAR` branch (`gwendylon.c:3196-3343`).
    fn nook_handle_char_message(
        &mut self,
        nook_id: CharacterId,
        data: &mut NookDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, NookPlayerFacts>,
        events: &mut Vec<NookOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(nook) = self.characters.get(&nook_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:3200-3203`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:3205-3209`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`gwendylon.c:3211-3215`).
        if tick < data.last_talk + NOOK_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`gwendylon.c:3217-3220`) - note nook's own
        // check has no `dat->current_victim &&` guard, unlike jessica's/
        // brithildie's, so a `None` current_victim (C's `0`) still fails
        // the inequality (and thus blocks the message) whenever `co != 0`.
        if tick < data.last_talk + NOOK_TALK_VICTIM_TICKS
            && data.current_victim.map_or(0, |victim| victim.0) != player_id.0
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:3223-3226`).
        if nook_id == player_id || !char_see_char(&nook, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 16) continue;` (`gwendylon.c:3229-
        // 3232`).
        if char_dist(&nook, &player) > NOOK_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.state;

        // C `switch (ppd->nook_state) { ... }` (`gwendylon.c:3238-3336`).
        match facts.state {
            0 => {
                self.npc_quiet_say(
                    nook_id,
                    &format!("Hullo, {}. I am {}, the judge.", player.name, nook.name),
                );
                new_state = 1;
                didsay = true;
            }
            1 => {
                self.npc_quiet_say(
                    nook_id,
                    &format!(
                        "Oh, wait. I am {}, the Knight of the Shining Armor.",
                        nook.name
                    ),
                );
                new_state = 2;
                didsay = true;
            }
            2 => {
                self.npc_quiet_say(
                    nook_id,
                    &format!(
                        "No, that's not right either. I am Jester, the {}.",
                        nook.name
                    ),
                );
                new_state = 3;
                didsay = true;
            }
            3 => {
                self.npc_quiet_say(
                    nook_id,
                    "If thou art looking for Gwendylon, the mage, take the northernmost door. If thou wishest to talk to Lydia, his noble mageness' daughter, take this door.",
                );
                new_state = 4;
                didsay = true;
            }
            4 => {
                // C `if (ppd->gwendy_state < GWENDYLON_STATE_SECOND_SKULL_DONE)
                // break;` (`gwendylon.c:3261-3263`).
                if facts.gwendy_state >= GWENDYLON_STATE_SECOND_SKULL_DONE {
                    self.npc_quiet_say(
                        nook_id,
                        "I heard Yoakin talk about skeletons coming out from his back room! How poor a jest to make!",
                    );
                    new_state = 5;
                    didsay = true;
                }
            }
            5 => {
                // C `if (ppd->gwendy_state < GWENDYLON_STATE_THIRD_SKULL_DONE)
                // break;` (`gwendylon.c:3270-3272`).
                if facts.gwendy_state >= GWENDYLON_STATE_THIRD_SKULL_DONE {
                    self.npc_quiet_say(nook_id, &format!("Oh, hello {}.", player.name));
                    events.push(NookOutcomeEvent::QuestOpen { player_id });
                    new_state = 6;
                    didsay = true;
                }
            }
            6 => {
                self.npc_quiet_say(
                    nook_id,
                    "I've heard the mage is looking for a hiding place in this tower. When I started working here, before Gwendylon bought the tower, the old owner - he was a bit strange - used to murmur: 'Stand between torches, in the courtyard, stand between torches'.",
                );
                new_state = 7;
                didsay = true;
            }
            7 => {
                self.npc_quiet_say(
                    nook_id,
                    &format!(
                        "I could never make anything of it, but maybe thou art wise enough to understand it, {}.",
                        player.name
                    ),
                );
                new_state = 8;
                didsay = true;
            }
            8 => {
                self.npc_quiet_say(
                    nook_id,
                    "But before thou searchest for that skull, I'd like to ask thee for a favor myself. A band of robbers has stolen my cap. It's just a normal cap, but I've inherited it from my father, and it is very dear to me.",
                );
                new_state = 9;
                didsay = true;
            }
            9 => {
                self.npc_quiet_say(
                    nook_id,
                    "Now these robbers are demanding a ransom for that cap. But alas, I am poor and cannot pay. I saw a robber escaping with stolen goods into the forest east and south of the city. Perhaps my cap too were taken that way.",
                );
                new_state = 10;
                didsay = true;
            }
            10 => {
                self.npc_quiet_say(
                    nook_id,
                    &format!(
                        "Please, {}, help me. I cannot offer thee a reward, but I'd really, really appreciate thy help.",
                        player.name
                    ),
                );
                new_state = 11;
                didsay = true;
            }
            // 11: break (no-op, waiting for the cap to be handed over).
            12 => {
                // C `if (ppd->gwendy_state >= GWENDYLON_STATE_DONE_BLESS) {
                // ppd->nook_state = 16; break; }` (`gwendylon.c:3318-3321`)
                // - note no `quiet_say` here, so `didsay` stays false.
                if facts.gwendy_state >= GWENDYLON_STATE_DONE_BLESS {
                    new_state = 16;
                }
            }
            13 => {
                self.npc_quiet_say(
                    nook_id,
                    "When I started working here, before Gwendylon bought the tower, the old owner - he was a bit strange - used to murmur: 'Stand between torches, in the courtyard, stand between torches'.",
                );
                new_state = 14;
                didsay = true;
            }
            // 14/15/16: break (no-op, matching C's comment-documented
            // dead-end cases).
            _ => {}
        }

        if new_state != facts.state {
            events.push(NookOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:3337-3341`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `nook_driver`'s `NT_TEXT` branch (`gwendylon.c:3346-3380`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::jessica`'s/`world::brithildie`'s text handlers).
    fn nook_handle_text_message(
        &mut self,
        nook_id: CharacterId,
        nook_name: &str,
        data: &mut NookDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, NookPlayerFacts>,
        events: &mut Vec<NookOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:3349-3351`).
        let tick = self.tick.0;
        if tick > data.last_talk + NOOK_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:3353-3356`).
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
        if nook_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(nook) = self.characters.get(&nook_id).cloned() else {
            return;
        };
        if char_dist(&nook, &speaker) > NOOK_QA_DISTANCE
            || !char_see_char(&nook, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, nook_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(nook_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:3359-3373`): three disjoint `if`s,
            // each resetting `nook_state` back to a checkpoint - at most
            // one applies since the ranges don't overlap.
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    let reset_state = if facts.state <= 4 {
                        Some(0)
                    } else if (5..=11).contains(&facts.state) {
                        Some(5)
                    } else if (12..=16).contains(&facts.state) {
                        Some(12)
                    } else {
                        None
                    };
                    if let Some(new_state) = reset_state {
                        data.last_talk = 0;
                        events.push(NookOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by nook's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:3376-3379`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2` branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `nook_driver`'s `NT_GIVE` branch (`gwendylon.c:3383-3410`).
    fn nook_handle_give_message(
        &mut self,
        nook_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, NookPlayerFacts>,
        events: &mut Vec<NookOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&nook_id)
            .and_then(|nook| nook.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let facts = player_facts.get(&giver_id).copied();

        if template_id == IID_AREA1_JESTERCAP
            && facts.is_some_and(|facts| (5..=11).contains(&facts.state))
        {
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_quiet_say(
                nook_id,
                &format!("Ah. There it is! My cap! Oh, {}, I thank thee!", giver_name),
            );
            events.push(NookOutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA1_JESTERCAP);
            self.destroy_items_by_template_id(giver_id, IID_AREA1_ROBBERKEY1);

            self.npc_quiet_say(
                nook_id,
                "I cannot give thee a reward, but you have my eternal gratitude.",
            );
            events.push(NookOutcomeEvent::UpdateState {
                player_id: giver_id,
                new_state: 12,
            });

            // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;`
            // (`gwendylon.c:3399-3400`).
            self.destroy_item(item_id);
        } else {
            // C `else { quiet_say(...); if (!give_char_item(co,
            // ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].citem =
            // 0; }` (`gwendylon.c:3402-3407`) - the plain `give_char_item`,
            // not `give_char_item_smart` (see the module doc comment's
            // last bullet).
            self.npc_quiet_say(
                nook_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }

    /// C's "Nook idle mutterings" block (`gwendylon.c:3429-3445`): once
    /// per minute, on a `RANDOM(25)` 1-in-25 hit, murmur a random
    /// identity-crisis line.
    fn nook_idle_chatter(&mut self, nook_id: CharacterId, last_talk: u64) {
        let tick = self.tick.0;
        if last_talk + NOOK_MUTTER_TICKS >= tick {
            return;
        }
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, 25) != 0 {
            return;
        }
        let index = legacy_random_below_from_seed(
            &mut self.legacy_random_seed,
            NOOK_MUTTERINGS.len() as u32,
        ) as usize;
        self.npc_murmur(nook_id, NOOK_MUTTERINGS[index]);

        if let Some(CharacterDriverState::Nook(data)) = self
            .characters
            .get_mut(&nook_id)
            .and_then(|nook| nook.driver_state.as_mut())
        {
            data.last_talk = tick;
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct nook_driver_data` (`src/area/1/gwendylon.c:3175-3178`): the
/// identity-crisis NPC's own driver memory (`CDR_NOOK`, distinct from the
/// per-player `nook_state` field in `crate::player::PlayerRuntime`'s
/// `area1_ppd` - see `world::nook`'s module doc comment for the split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NookDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
