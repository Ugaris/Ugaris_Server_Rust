//! Two-City military quest giver (`CDR_TWOSANWYN`), "Sanwyn" - Sergeant
//! Major in the Imperial Army, quest 29 ("Dirty Hands").
//!
//! Ports `src/area/17/two.c::sanwyn` (`:2249-2484`); C's `ch_died_driver`/
//! `ch_respawn_driver` dispatch for `CDR_TWOSANWYN` are plain `return 1;`
//! no-ops (same as `CDR_TWOALCHEMIST`), so no death/respawn hook exists
//! for this NPC.
//!
//! Follows the same `World`/`PlayerRuntime` split established by
//! `world::npc::area17::two_skelly`/`alchemist`: `sanwyn_state`/
//! `sanwyn_bits` live on `crate::player::PlayerRuntime::twocity_ppd` (via
//! `twocity_sanwyn_state`/`set_twocity_sanwyn_state`/`twocity_sanwyn_bits`/
//! `set_twocity_sanwyn_bits`), not `World`, so the caller supplies a
//! per-player fact snapshot ([`TwoSanwynPlayerFacts`]) up front and
//! applies the returned [`TwoSanwynOutcomeEvent`]s afterwards.
//!
//! Unlike `two_skelly`/`alchemist`, the `NT_GIVE` turn-in accepts three
//! distinct items (the three incriminating palace notes), each tracked by
//! its own bit in `sanwyn_bits` (`1`/`2`/`4`) so a player can turn them in
//! in any order; `sanwyn_state` only advances to `7` once all three bits
//! are set. Each individual note turn-in awards military points directly
//! via [`World::give_military_pts_from_npc`] (`two.c`'s own `give_
//! military_pts(cn, co, 15, min(level_value(ch[co].level)/5, 15000))`,
//! called once per note, up to three times total per character - matching
//! `questlog.c`'s own "exp awarded in driver, 45000 total" comment on
//! quest 29's zero `exp` field) - this needs only `Character::level`,
//! already visible to `World`, so (unlike `alchemist`'s potion reward)
//! it does not need to be deferred to a server-side event.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_TWOSANWYN};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_AREA17_PALACENOTE1, IID_AREA17_PALACENOTE2, IID_AREA17_PALACENOTE3};
use crate::world::*;

use super::TWOCITY_QA;

/// C `char_dist(cn, co) > 10` (`two.c:2299`).
const TWO_SANWYN_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`two.c:2282`).
const TWO_SANWYN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`two.c:2287`, `:2340`).
const TWO_SANWYN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`two.c:2478`): idle "return to post" threshold.
const TWO_SANWYN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `#define IID_AREA17_PALACENOTE1`'s `sanwyn_bits` mask (`two.c:2422`).
const SANWYN_BIT_NOTE1: i32 = 1;
/// C `#define IID_AREA17_PALACENOTE2`'s `sanwyn_bits` mask (`two.c:2434`).
const SANWYN_BIT_NOTE2: i32 = 2;
/// C `#define IID_AREA17_PALACENOTE3`'s `sanwyn_bits` mask (`two.c:2446`).
const SANWYN_BIT_NOTE3: i32 = 4;
/// C `ppd->sanwyn_bits == 7` (`two.c:2426`, `:2438`, `:2450`): all three
/// palace notes turned in.
const SANWYN_BITS_ALL: i32 = SANWYN_BIT_NOTE1 | SANWYN_BIT_NOTE2 | SANWYN_BIT_NOTE3;
/// C `give_military_pts(cn, co, 15, ...)` (`two.c:2428`, `:2440`,
/// `:2452`).
const SANWYN_NOTE_MILITARY_PTS: i32 = 15;
/// C `min(level_value(ch[co].level) / 5, 15000)` (`two.c:2428`, `:2440`,
/// `:2452`).
const SANWYN_NOTE_MILITARY_EXP_CAP: i64 = 15000;

/// C `struct sanwyn_driver_data` (`two.c:2249-2252`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoSanwynDriverData {
    pub last_talk_tick: u64,
    pub current_victim: Option<CharacterId>,
}

/// Per-player facts [`World::process_two_sanwyn_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoSanwynPlayerFacts {
    /// `PlayerRuntime::twocity_sanwyn_state()`.
    pub sanwyn_state: i32,
    /// `PlayerRuntime::twocity_sanwyn_bits()`.
    pub sanwyn_bits: i32,
}

/// A side effect [`World::process_two_sanwyn_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoSanwynOutcomeEvent {
    /// Write the new `twocity_ppd.sanwyn_state` back.
    UpdateSanwynState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// Write the new `twocity_ppd.sanwyn_bits` back.
    UpdateSanwynBits {
        player_id: CharacterId,
        new_bits: i32,
    },
    /// C `questlog_open(co, 29)` (`two.c:2318`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 29)` (`two.c:2401`).
    QuestDone { player_id: CharacterId },
}

impl World {
    /// C `sanwyn`'s per-tick body (`two.c:2254-2484`).
    pub fn process_two_sanwyn_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TwoSanwynPlayerFacts>,
        area_id: u16,
    ) -> Vec<TwoSanwynOutcomeEvent> {
        let sanwyn_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TWOSANWYN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for sanwyn_id in sanwyn_ids {
            self.process_two_sanwyn_tick(sanwyn_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_two_sanwyn_tick(
        &mut self,
        sanwyn_id: CharacterId,
        player_facts: &HashMap<CharacterId, TwoSanwynPlayerFacts>,
        area_id: u16,
        events: &mut Vec<TwoSanwynOutcomeEvent>,
    ) {
        let Some(sanwyn_name) = self.characters.get(&sanwyn_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::TwoSanwyn(mut data)) = self
            .characters
            .get(&sanwyn_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&sanwyn_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.two_sanwyn_handle_char_message(
                    sanwyn_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.two_sanwyn_handle_text_message(
                    sanwyn_id,
                    &sanwyn_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.two_sanwyn_handle_give_message(
                    sanwyn_id,
                    message,
                    player_facts,
                    events,
                    area_id,
                ),
                _ => {}
            }
        }

        if let Some(sanwyn) = self.characters.get_mut(&sanwyn_id) {
            sanwyn.driver_state = Some(CharacterDriverState::TwoSanwyn(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`two.c:2474-2476`).
        if let (Some(sanwyn), Some((tx, ty))) =
            (self.characters.get(&sanwyn_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(sanwyn.x), i32::from(sanwyn.y), tx, ty) {
                if let Some(sanwyn_mut) = self.characters.get_mut(&sanwyn_id) {
                    let _ = turn(sanwyn_mut, direction as u8);
                }
            }
        }

        let data = match self
            .characters
            .get(&sanwyn_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::TwoSanwyn(data)) => *data,
            _ => return,
        };

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (secure_move_
        // driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret, lastact))
        // return; }` (`two.c:2478-2482`). `tmpx`/`tmpy` reuse `rest_x`/
        // `rest_y`, the same substitution every other stationary NPC in
        // this codebase makes.
        if data.last_talk_tick + TWO_SANWYN_RETURN_TO_POST_TICKS < self.tick.0 {
            let (post_x, post_y) = self
                .characters
                .get(&sanwyn_id)
                .map(|sanwyn| (sanwyn.rest_x, sanwyn.rest_y))
                .unwrap_or_default();
            if self.secure_move_driver(
                sanwyn_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            ) {
                return;
            }
        }
        // C `do_idle(cn, TICKS);` (`two.c:2483`) - not modeled, same
        // precedent as every other stationary dialogue-only NPC in this
        // codebase: it has no observable effect in this message-driven
        // architecture.
    }

    /// C `sanwyn`'s `NT_CHAR` branch (`two.c:2267-2405`).
    #[allow(clippy::too_many_arguments)]
    fn two_sanwyn_handle_char_message(
        &mut self,
        sanwyn_id: CharacterId,
        data: &mut TwoSanwynDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoSanwynPlayerFacts>,
        events: &mut Vec<TwoSanwynOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(sanwyn) = self.characters.get(&sanwyn_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`two.c:2270-2274`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message;
        // continue; }` (`two.c:2276-2280`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`two.c:2282-2286`).
        if tick < data.last_talk_tick + TWO_SANWYN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`two.c:2287-2290`).
        if tick < data.last_talk_tick + TWO_SANWYN_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`two.c:2292-2296`).
        if sanwyn_id == player_id || !char_see_char(&sanwyn, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`two.c:2298-2302`).
        if char_dist(&sanwyn, &player) > TWO_SANWYN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->sanwyn_state) { ... }` (`two.c:2312-2371`).
        match facts.sanwyn_state {
            0 => {
                self.npc_say(
                    sanwyn_id,
                    &format!(
                        "Welcome, {}, to the noble and honorable city of Exkordon. I am {}, Sergeant Major in the Imperial Army.",
                        player.name, sanwyn.name
                    ),
                );
                events.push(TwoSanwynOutcomeEvent::QuestOpen { player_id });
                events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
            }
            1 => {
                self.npc_say(
                    sanwyn_id,
                    &format!(
                        "They are so noble and honorable that they decided to defect from the Empire. Exkordon is, for all we care, a lawless town, {}. Be careful in there, but be aware that thou canst do as thou wilt in Exkordon - we don't care.",
                        army_rank_name(army_rank_for_points(player.military_points))
                    ),
                );
                events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                    player_id,
                    new_state: 2,
                });
                didsay = true;
            }
            2 => {
                self.npc_say(
                    sanwyn_id,
                    "Inofficially, if thou wert to burn the whole city down, the whole Imperial army would applaud thee, even though we'd have to apologize, officially. Anyway.",
                );
                events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                    player_id,
                    new_state: 3,
                });
                didsay = true;
            }
            3 => {
                self.npc_say(
                    sanwyn_id,
                    &format!(
                        "The Imperial army suspects that the current governor of Exkordon - the one who decided to defect from the Empire - has dirty hands. Shouldst thou happen to find any incriminating documents, {}, I'd be very grateful if thou wouldst bring them to me.",
                        player.name
                    ),
                );
                events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                    player_id,
                    new_state: 4,
                });
                didsay = true;
            }
            4 => {
                self.npc_say(
                    sanwyn_id,
                    &format!(
                        "The thieves guild might be helpful in thy search, {}. Thou canst find their headquarter in the sewers. One entrance is a bit east of the city gate, behind a guard house. They won't have those documents, of course, but they might be able to help thee enter the palace.",
                        player.name
                    ),
                );
                events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                    player_id,
                    new_state: 5,
                });
                didsay = true;
            }
            5 => {
                self.npc_say(
                    sanwyn_id,
                    &format!(
                        "That will be all, {}. May thy stay in Exkordon be... destructive.",
                        army_rank_name(army_rank_for_points(player.military_points))
                    ),
                );
                events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                    player_id,
                    new_state: 6,
                });
                didsay = true;
            }
            // `sanwyn_state == 6`: waiting for documents (`two.c:2367`).
            6 => {}
            7 => {
                self.npc_say(
                    sanwyn_id,
                    "Dirty hands indeed! Well, well, well. We'll be able to stir up quite a bit of trouble with these.",
                );
                events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                    player_id,
                    new_state: 8,
                });
                events.push(TwoSanwynOutcomeEvent::QuestDone { player_id });
                didsay = true;
            }
            // `sanwyn_state == 8`: done (`two.c:2402-2403`).
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`two.c:2397-2401`).
        if didsay {
            data.last_talk_tick = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `sanwyn`'s `NT_TEXT` branch (`two.c:2408-2433`), wired through
    /// the generic `analyse_text_qa` matcher (same pattern as `world::
    /// npc::area17::two_skelly`/`alchemist`'s text handlers).
    #[allow(clippy::too_many_arguments)]
    fn two_sanwyn_handle_text_message(
        &mut self,
        sanwyn_id: CharacterId,
        sanwyn_name: &str,
        data: &mut TwoSanwynDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoSanwynPlayerFacts>,
        events: &mut Vec<TwoSanwynOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`two.c:2411-2413`).
        let tick = self.tick.0;
        if tick > data.last_talk_tick + TWO_SANWYN_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
        {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`two.c:2415-2418`).
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

        // C `analyse_text_driver`'s own guard clauses (`two.c:126-144`):
        // ignore our own talk, non-players/player-likes, not-visible.
        if sanwyn_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(sanwyn) = self.characters.get(&sanwyn_id).cloned() else {
            return;
        };
        if !char_see_char(&sanwyn, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let sanwyn_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.sanwyn_state)
            .unwrap_or(0);

        let mut didsay = false;
        // C's `analyse_text_driver` calls `say(cn, qa[q].answer, ...)`
        // directly (`two.c:206`), same as `two_skelly`/`alchemist`.
        match analyse_text_qa(text, sanwyn_name, &speaker.name, TWOCITY_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(sanwyn_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`two.c:2419-2429`): resets to `0`
            // while `sanwyn_state <= 6`, or back to `7` (not `8`) when
            // it was already `8` (done).
            TextAnalysisOutcome::Matched(2) => {
                if sanwyn_state <= 6 {
                    data.last_talk_tick = 0;
                    events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                if sanwyn_state == 8 {
                    data.last_talk_tick = 0;
                    events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                        player_id: speaker_id,
                        new_state: 7,
                    });
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`two.c:2430-2433`) - note this does *not* touch `dat->
        // last_talk` (except the explicit reset inside the `case 2`
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `sanwyn`'s `NT_GIVE` branch (`two.c:2437-2470`): each of the
    /// three palace notes sets its own `sanwyn_bits` bit (turning it in
    /// twice, or turning in a note while `sanwyn_state > 6`, falls
    /// through to the give-back branch, matching C's exact `!(ppd->
    /// sanwyn_bits & N)` guard), promotes `sanwyn_state` to `7` once all
    /// three bits are set, and awards `15` military points (exp capped at
    /// `min(level_value(level)/5, 15000)`) via [`World::
    /// give_military_pts_from_npc`] - once per note, independent of
    /// whether this note completes the set. Anything else is handed
    /// straight back (falling back to destroying it if the player's
    /// inventory is full), matching C's plain `give_char_item` (not
    /// `give_char_item_smart`).
    fn two_sanwyn_handle_give_message(
        &mut self,
        sanwyn_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoSanwynPlayerFacts>,
        events: &mut Vec<TwoSanwynOutcomeEvent>,
        area_id: u16,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get(&sanwyn_id)
            .and_then(|sanwyn| sanwyn.cursor_item)
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            return;
        };
        let facts = player_facts
            .get(&giver_id)
            .copied()
            .unwrap_or(TwoSanwynPlayerFacts {
                sanwyn_state: 0,
                sanwyn_bits: 0,
            });

        let note_bit = if template_id == IID_AREA17_PALACENOTE1 {
            Some(SANWYN_BIT_NOTE1)
        } else if template_id == IID_AREA17_PALACENOTE2 {
            Some(SANWYN_BIT_NOTE2)
        } else if template_id == IID_AREA17_PALACENOTE3 {
            Some(SANWYN_BIT_NOTE3)
        } else {
            None
        };

        let accepted = match note_bit {
            Some(bit) if facts.sanwyn_state <= 6 && facts.sanwyn_bits & bit == 0 => Some(bit),
            _ => None,
        };

        if let Some(bit) = accepted {
            // C `say(cn, "Ah. Well done, %s.", ch[co].name); ppd->
            // sanwyn_bits |= N; if (ppd->sanwyn_bits == 7) ppd->
            // sanwyn_state = 7; give_military_pts(cn, co, 15, min(
            // level_value(ch[co].level)/5, 15000)); destroy_item(ch[cn].
            // citem); ch[cn].citem = 0;` (`two.c:2422-2432`, mirrored for
            // notes 2/3).
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_say(sanwyn_id, &format!("Ah. Well done, {giver_name}."));
            let new_bits = facts.sanwyn_bits | bit;
            events.push(TwoSanwynOutcomeEvent::UpdateSanwynBits {
                player_id: giver_id,
                new_bits,
            });
            if new_bits == SANWYN_BITS_ALL {
                events.push(TwoSanwynOutcomeEvent::UpdateSanwynState {
                    player_id: giver_id,
                    new_state: 7,
                });
            }
            let level = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.level)
                .unwrap_or(0);
            let exp = (i64::from(level_value(level)) / 5).min(SANWYN_NOTE_MILITARY_EXP_CAP) as i32;
            self.give_military_pts_from_npc(
                giver_id,
                sanwyn_id,
                SANWYN_NOTE_MILITARY_PTS,
                exp,
                u32::from(area_id),
            );
            if let Some(sanwyn) = self.characters.get_mut(&sanwyn_id) {
                sanwyn.cursor_item = None;
            }
            self.destroy_item(item_id);
        } else {
            // C `else { say("Thou hast better use..."); if (!give_char_
            // item(co, ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].
            // citem = 0; }` (`two.c:2453-2459`).
            self.npc_say(
                sanwyn_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if let Some(sanwyn) = self.characters.get_mut(&sanwyn_id) {
                sanwyn.cursor_item = None;
            }
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;
