//! Two-City raised skeleton (`CDR_TWOSKELLY`), "Scarcewind" - the murdered
//! governor's ghost, quest 30 ("The Old Governor's Cross").
//!
//! Ports `src/area/17/two.c::skelly` (`:2767-2936`) plus its death hook
//! `skelly_dead` (`:2938-2943`). The skeleton is not part of any zone's
//! static spawn list - it is raised at runtime by the already-ported
//! `IDR_SKELRAISE` item driver (`item_driver::area17_two::skelraise_
//! driver`, template `quest_skeleton`) via `ugaris-server::area_apply::
//! raise_skeleton_from_template`, which instantiates the same `.chr`
//! template every other zone-spawned NPC uses - so the `CDR_TWOSKELLY`
//! spawn-time `driver_state` wiring in `crate::zone` applies here exactly
//! as it would for a static spawn.
//!
//! Follows the same `World`/`PlayerRuntime` split established by
//! `world::npc::area16::william`: `skelly_state` lives on
//! `crate::player::PlayerRuntime::twocity_ppd` (via `twocity_skelly_
//! state`/`set_twocity_skelly_state`), not `World`, so the caller supplies
//! a per-player fact snapshot ([`TwoSkellyPlayerFacts`]) up front and
//! applies the returned [`TwoSkellyOutcomeEvent`]s afterwards.
//!
//! A real, deliberately-reproduced C quirk: the skeleton self-destructs
//! (`kill_char(cn, 0)`, ported as [`World::remove_character`]) 30 seconds
//! after its own `NT_CREATE` message, entirely independent of whether it
//! ever talked to anyone - see [`Self::process_two_skelly_tick`].

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_TWOSKELLY};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_AREA17_CROSS, IID_AREA17_GREENKEY, IID_AREA17_REDKEY};
use crate::world::*;

use super::TWOCITY_QA;

/// C `char_dist(cn, co) > 10` (`two.c:2820`).
const TWO_SKELLY_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`two.c:2803`).
const TWO_SKELLY_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`two.c:2808`, `:2860`).
const TWO_SKELLY_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`two.c:2925`): idle "return to post" threshold.
const TWO_SKELLY_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS * 30` (`two.c:2930`): self-destruct threshold since creation.
const TWO_SKELLY_SELF_DESTRUCT_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `struct skelly_driver_data` (`two.c:2761-2765`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoSkellyDriverData {
    pub last_talk_tick: u64,
    pub current_victim: Option<CharacterId>,
    pub alive_tick: u64,
}

/// Per-player facts [`World::process_two_skelly_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoSkellyPlayerFacts {
    /// `PlayerRuntime::twocity_skelly_state()`.
    pub skelly_state: i32,
}

/// A side effect [`World::process_two_skelly_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoSkellyOutcomeEvent {
    /// Write the new `twocity_ppd.skelly_state` back.
    UpdateSkellyState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 30)` (`two.c:2835`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 30)` (`two.c:2897`).
    QuestDone { player_id: CharacterId },
}

impl World {
    /// C `skelly`'s per-tick body (`two.c:2767-2936`).
    pub fn process_two_skelly_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TwoSkellyPlayerFacts>,
        area_id: u16,
    ) -> Vec<TwoSkellyOutcomeEvent> {
        let skelly_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TWOSKELLY
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for skelly_id in skelly_ids {
            self.process_two_skelly_tick(skelly_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_two_skelly_tick(
        &mut self,
        skelly_id: CharacterId,
        player_facts: &HashMap<CharacterId, TwoSkellyPlayerFacts>,
        area_id: u16,
        events: &mut Vec<TwoSkellyOutcomeEvent>,
    ) {
        let Some(skelly_name) = self.characters.get(&skelly_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::TwoSkelly(mut data)) = self
            .characters
            .get(&skelly_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&skelly_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                // C `if (msg->type == NT_CREATE) dat->alive = ticker;`
                // (`two.c:2782-2784`).
                NT_CREATE => {
                    data.alive_tick = self.tick.0;
                }
                NT_CHAR => self.two_skelly_handle_char_message(
                    skelly_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.two_skelly_handle_text_message(
                    skelly_id,
                    &skelly_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.two_skelly_handle_give_message(skelly_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(skelly) = self.characters.get_mut(&skelly_id) {
            skelly.driver_state = Some(CharacterDriverState::TwoSkelly(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`two.c:2921-2923`).
        if let (Some(skelly), Some((tx, ty))) =
            (self.characters.get(&skelly_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(skelly.x), i32::from(skelly.y), tx, ty) {
                if let Some(skelly_mut) = self.characters.get_mut(&skelly_id) {
                    let _ = turn(skelly_mut, direction as u8);
                }
            }
        }

        let data = match self
            .characters
            .get(&skelly_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::TwoSkelly(data)) => *data,
            _ => return,
        };

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (secure_move_
        // driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret, lastact))
        // return; }` (`two.c:2925-2929`). `tmpx`/`tmpy` reuse `rest_x`/
        // `rest_y`, the same substitution every other stationary NPC in
        // this codebase makes.
        if data.last_talk_tick + TWO_SKELLY_RETURN_TO_POST_TICKS < self.tick.0 {
            let (post_x, post_y) = self
                .characters
                .get(&skelly_id)
                .map(|skelly| (skelly.rest_x, skelly.rest_y))
                .unwrap_or_default();
            if self.secure_move_driver(
                skelly_id,
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

        // C `if (ticker - dat->alive > TICKS*30) { kill_char(cn, 0);
        // return; }` (`two.c:2930-2933`): the skeleton is temporary.
        if self.tick.0.saturating_sub(data.alive_tick) > TWO_SKELLY_SELF_DESTRUCT_TICKS {
            self.remove_character(skelly_id);
        }
        // C `do_idle(cn, TICKS);` (`two.c:2935`) - not modeled, same
        // precedent as every other stationary dialogue-only NPC in this
        // codebase: it has no observable effect in this message-driven
        // architecture.
    }

    /// C `skelly`'s `NT_CHAR` branch (`two.c:2787-2854`).
    #[allow(clippy::too_many_arguments)]
    fn two_skelly_handle_char_message(
        &mut self,
        skelly_id: CharacterId,
        data: &mut TwoSkellyDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoSkellyPlayerFacts>,
        events: &mut Vec<TwoSkellyOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(skelly) = self.characters.get(&skelly_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`two.c:2790-2794`) - note only `CF_PLAYER`, unlike
        // `william_driver`'s `CF_PLAYER | CF_PLAYERLIKE` check.
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`two.c:2796-2800`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`two.c:2802-2806`).
        if tick < data.last_talk_tick + TWO_SKELLY_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`two.c:2808-2811`).
        if tick < data.last_talk_tick + TWO_SKELLY_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`two.c:2813-2817`).
        if skelly_id == player_id || !char_see_char(&skelly, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`two.c:2819-2823`).
        if char_dist(&skelly, &player) > TWO_SKELLY_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->skelly_state) { ... }` (`two.c:2829-2847`).
        match facts.skelly_state {
            0 => {
                self.npc_say(
                    skelly_id,
                    &format!(
                        "My greetings, {}. I am {}, Governor of Exkordon. Former governor, I should say.",
                        player.name, skelly.name
                    ),
                );
                events.push(TwoSkellyOutcomeEvent::UpdateSkellyState {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
                events.push(TwoSkellyOutcomeEvent::QuestOpen { player_id });
            }
            1 => {
                self.npc_emote(skelly_id, "speaks in a different voice now");
                self.npc_say(
                    skelly_id,
                    "Pass the green, go to the red, find the cross, bring me peace.",
                );
                events.push(TwoSkellyOutcomeEvent::UpdateSkellyState {
                    player_id,
                    new_state: 2,
                });
                didsay = true;
            }
            // `skelly_state == 2`/`3`: silent (`two.c:2843-2846`).
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`two.c:2848-2852`).
        if didsay {
            data.last_talk_tick = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `skelly`'s `NT_TEXT` branch (`two.c:2857-2882`), wired through the
    /// generic `analyse_text_qa` matcher (same pattern as `world::npc::
    /// area16::william`'s text handler).
    #[allow(clippy::too_many_arguments)]
    fn two_skelly_handle_text_message(
        &mut self,
        skelly_id: CharacterId,
        skelly_name: &str,
        data: &mut TwoSkellyDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoSkellyPlayerFacts>,
        events: &mut Vec<TwoSkellyOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`two.c:2860-2862`).
        let tick = self.tick.0;
        if tick > data.last_talk_tick + TWO_SKELLY_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
        {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`two.c:2864-2867`).
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
        if skelly_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(skelly) = self.characters.get(&skelly_id).cloned() else {
            return;
        };
        if !char_see_char(&skelly, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let skelly_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.skelly_state)
            .unwrap_or(0);

        let mut didsay = false;
        // C's `analyse_text_driver` calls `say(cn, qa[q].answer, ...)`
        // directly (`two.c:206`), unlike `william_driver`'s reply, which
        // Rust routes through `npc_quiet_say` - port `two.c`'s own choice
        // exactly.
        match analyse_text_qa(text, skelly_name, &speaker.name, TWOCITY_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(skelly_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`two.c:2869-2877`).
            TextAnalysisOutcome::Matched(2) => {
                if skelly_state <= 2 {
                    data.last_talk_tick = 0;
                    events.push(TwoSkellyOutcomeEvent::UpdateSkellyState {
                        player_id: speaker_id,
                        new_state: 0,
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
        // (`two.c:2878-2881`) - note this does *not* touch `dat->
        // last_talk` (except the explicit reset inside the `case 2`
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `skelly`'s `NT_GIVE` branch (`two.c:2885-2913`): the cross
    /// turn-in completes quest 30 and cleans up the green/red puzzle keys;
    /// anything else is handed straight back (falling back to destroying
    /// it if the player's inventory is full), matching C's plain `give_
    /// char_item` (not `give_char_item_smart`).
    fn two_skelly_handle_give_message(
        &mut self,
        skelly_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoSkellyPlayerFacts>,
        events: &mut Vec<TwoSkellyOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get(&skelly_id)
            .and_then(|skelly| skelly.cursor_item)
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            return;
        };
        let skelly_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.skelly_state)
            .unwrap_or(0);

        if template_id == IID_AREA17_CROSS && skelly_state <= 2 {
            // C `say(cn, "My cross, ... I thank thee, %s.", ch[co].name);
            // ppd->skelly_state = 3; questlog_done(co, 30);
            // destroy_item_byID(co, IID_AREA17_CROSS);
            // destroy_item_byID(co, IID_AREA17_GREENKEY);
            // destroy_item_byID(co, IID_AREA17_REDKEY);
            // destroy_item(ch[cn].citem); ch[cn].citem = 0;`
            // (`two.c:2892-2904`).
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_say(
                skelly_id,
                &format!(
                    "My cross, the insignia of my office. Now, I may rest in peace. I thank thee, {}.",
                    giver_name
                ),
            );
            events.push(TwoSkellyOutcomeEvent::UpdateSkellyState {
                player_id: giver_id,
                new_state: 3,
            });
            events.push(TwoSkellyOutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA17_CROSS);
            self.destroy_items_by_template_id(giver_id, IID_AREA17_GREENKEY);
            self.destroy_items_by_template_id(giver_id, IID_AREA17_REDKEY);
            if let Some(skelly) = self.characters.get_mut(&skelly_id) {
                skelly.cursor_item = None;
            }
            self.destroy_item(item_id);
        } else {
            // C `else { say("Thou hast better use..."); if (!give_char_
            // item(co, ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].
            // citem = 0; }` (`two.c:2905-2911`).
            self.npc_say(
                skelly_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if let Some(skelly) = self.characters.get_mut(&skelly_id) {
                skelly.cursor_item = None;
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
