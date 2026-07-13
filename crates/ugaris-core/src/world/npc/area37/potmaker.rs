//! Potmaker NPC (`CDR_POTMAKER`), the Arkhata craftsman who runs "A
//! Special Pot" (quest 73).
//!
//! Ports `src/area/37/arkhata.c::potmaker_driver` (`:3004-3171`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! jada`: the caller supplies a per-player fact snapshot
//! ([`PotmakerPlayerFacts`]) up front and applies the returned
//! [`PotmakerOutcomeEvent`]s afterwards, since `arkhata_ppd.pot_state`
//! lives on `crate::player::PlayerRuntime`, not `World`. The player's
//! level (`ch[co].level >= 48`, `arkhata.c:3064`) is read directly from
//! `World::characters` instead, since `Character::level` is already
//! visible to `World`.
//!
//! `potmaker_driver`'s five-state (`0`/`2`/`3`/`4`, state `1` is never
//! independently stored) dialogue chain is the same shape as `world::
//! npc::area37::jada`'s own:
//! - `0` needs `ch[co].level >= 48` to advance; C's own `case 0` falls
//!   through into `case 1`'s speech/`questlog_open(73)`/advance-to-`2` in
//!   the same tick - collapsed into one `rs == 0` arm here, same
//!   "fallthrough lands on the next case's action" precedent as `jada`'s
//!   own `rs == 0` arm.
//! - `3` is a pure wait state: waiting for the player to bring the iron
//!   pot (`IID_ARKHATA_IRONPOT`), handled entirely by this file's own
//!   `NT_GIVE` branch - no cross-driver dependency.
//! - `4` is a pure wait state: quest already completed, nothing left to
//!   say.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches `jada`'s
//!   identical observation for that file's shared driver shape).
//! - `NT_GIVE`'s pot turn-in (`:3126-3153`) destroys both the cursor item
//!   (`ch[cn].citem`) *and* any other `IID_ARKHATA_IRONPOT` copy the
//!   player still carries (`destroy_item_byID(co, ...)`) - the exact same
//!   double-destroy shape as `jada`'s own blade turn-in
//!   (`destroy_items_by_template_id` then `destroy_item`).
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `potmaker_driver` body at all (matching the `jada`/`rammy`/`jaz`/
//!   `ramin` "pure talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:3170`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_ARKHATA_IRONPOT;
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:3053`, sibling drivers' own
/// identical guard).
const POTMAKER_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const POTMAKER_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:3036`).
const POTMAKER_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:3041`).
const POTMAKER_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:3164`): idle "return to post" threshold.
const POTMAKER_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C quest 73, "A Special Pot".
const QLOG_POTMAKER_SPECIAL_POT: usize = 73;
/// C `ch[co].level >= 48` (`arkhata.c:3064`).
const POTMAKER_QUEST_MIN_LEVEL: u32 = 48;

/// Per-player facts [`World::process_potmaker_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PotmakerPlayerFacts {
    /// `PlayerRuntime::arkhata_pot_state()`.
    pub pot_state: i32,
}

/// A side effect [`World::process_potmaker_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotmakerOutcomeEvent {
    /// Write the new `arkhata_ppd.pot_state` back.
    UpdatePotState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 73)` (`arkhata.c:3073`).
    QuestOpen73 { player_id: CharacterId },
    /// C `questlog_done(co, 73)` plus its `create_item("infravision_pot")`
    /// reward (`arkhata.c:3132-3140`), the `NT_GIVE` pot turn-in.
    QuestDone73GiveInfravisionPot { player_id: CharacterId },
}

impl World {
    /// C `potmaker_driver`'s per-tick body (`arkhata.c:3004-3171`).
    pub fn process_potmaker_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, PotmakerPlayerFacts>,
        area_id: u16,
    ) -> Vec<PotmakerOutcomeEvent> {
        let potmaker_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_POTMAKER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for potmaker_id in potmaker_ids {
            self.process_potmaker_messages(potmaker_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_potmaker_messages(
        &mut self,
        potmaker_id: CharacterId,
        player_facts: &HashMap<CharacterId, PotmakerPlayerFacts>,
        area_id: u16,
        events: &mut Vec<PotmakerOutcomeEvent>,
    ) {
        let Some(potmaker_name) = self
            .characters
            .get(&potmaker_id)
            .map(|potmaker| potmaker.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Potmaker(mut data)) = self
            .characters
            .get(&potmaker_id)
            .and_then(|potmaker| potmaker.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&potmaker_id)
            .map(|potmaker| std::mem::take(&mut potmaker.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.potmaker_handle_char_message(
                    potmaker_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.potmaker_handle_text_message(
                    potmaker_id,
                    &potmaker_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.potmaker_handle_give_message(potmaker_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(potmaker) = self.characters.get_mut(&potmaker_id) {
            potmaker.driver_state = Some(CharacterDriverState::Potmaker(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:3160-3162`).
        if let (Some(potmaker), Some((tx, ty))) =
            (self.characters.get(&potmaker_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(potmaker.x), i32::from(potmaker.y), tx, ty)
            {
                if let Some(potmaker_mut) = self.characters.get_mut(&potmaker_id) {
                    let _ = turn(potmaker_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:3164-3168`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(potmaker) = self.characters.get(&potmaker_id) {
            match potmaker.driver_state.as_ref() {
                Some(CharacterDriverState::Potmaker(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + POTMAKER_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(potmaker) = self.characters.get(&potmaker_id) else {
                return;
            };
            let (post_x, post_y) = (potmaker.rest_x, potmaker.rest_y);
            self.secure_move_driver(
                potmaker_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `potmaker_driver`'s `NT_CHAR` branch (`arkhata.c:3020-3094`).
    #[allow(clippy::too_many_arguments)]
    fn potmaker_handle_char_message(
        &mut self,
        potmaker_id: CharacterId,
        data: &mut PotmakerDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, PotmakerPlayerFacts>,
        events: &mut Vec<PotmakerOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(potmaker) = self.characters.get(&potmaker_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:3024`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:3030`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:3036`).
        if tick < data.last_talk + POTMAKER_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:3041`).
        if tick < data.last_talk + POTMAKER_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:3047`).
        if potmaker_id == player_id
            || !char_see_char(&potmaker, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:3053`).
        if char_dist(&potmaker, &player) > POTMAKER_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.pot_state;
        match facts.pot_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:
            // 3063-3076`) - see the module doc comment.
            0 if player.level >= POTMAKER_QUEST_MIN_LEVEL => {
                self.npc_quiet_say(
                    potmaker_id,
                    "Hello Stranger, I'm afraid someone stole a rather special pot I made on order from the Monk Thai Pan. I made it from iron blessed with holy water from a spring in the mountains outside the fortress.",
                );
                events.push(PotmakerOutcomeEvent::QuestOpen73 { player_id });
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:3077-3083`).
            2 => {
                self.npc_quiet_say(
                    potmaker_id,
                    "It is quite a valuable pot, it can hold water in temperatures far below freezing without it turning to ice. Thai Pan has told me he could sense it's magic south of his temple, perhaps you should search the forest in that direction.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3: break;` (`arkhata.c:3084-3085`): waiting for the
            // pot.
            3 => {}
            // C `case 4: break;` (`arkhata.c:3086-3087`): all done.
            4 => {}
            _ => {}
        }

        if new_state != facts.pot_state {
            events.push(PotmakerOutcomeEvent::UpdatePotState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:3089-3093`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `potmaker_driver`'s `NT_TEXT` branch (`arkhata.c:3098-3123`),
    /// wired through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn potmaker_handle_text_message(
        &mut self,
        potmaker_id: CharacterId,
        potmaker_name: &str,
        data: &mut PotmakerDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, PotmakerPlayerFacts>,
        events: &mut Vec<PotmakerOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:3101-3103`).
        let tick = self.tick.0;
        if tick > data.last_talk + POTMAKER_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:3105`).
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
        let Some(potmaker) = self.characters.get(&potmaker_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if potmaker_id == speaker_id {
            return;
        }
        if char_dist(&potmaker, &speaker) > POTMAKER_QA_DISTANCE
            || !char_see_char(&potmaker, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let pot_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.pot_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, potmaker_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(potmaker_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:3110-3117`): rewind to state
            // 1 while the turn-in window (`1..=3`) is open.
            TextAnalysisOutcome::Matched(2) => {
                if pot_state > 0 && pot_state <= 3 {
                    data.last_talk = 0;
                    events.push(PotmakerOutcomeEvent::UpdatePotState {
                        player_id: speaker_id,
                        new_state: 1,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by potmaker's
            // own `switch` but still counts as `didsay` (C: `switch
            // (didsay = analyse_text_driver(...))` - any nonzero return
            // is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:3119-3122`) - note this does *not* touch `dat->
        // last_talk` (except the "repeat" branch's own explicit reset
        // above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `potmaker_driver`'s `NT_GIVE` branch (`arkhata.c:3126-3154`).
    fn potmaker_handle_give_message(
        &mut self,
        potmaker_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, PotmakerPlayerFacts>,
        events: &mut Vec<PotmakerOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&potmaker_id)
            .and_then(|potmaker| potmaker.cursor_item.take())
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

        // C `if (ppd && it[in].ID == IID_ARKHATA_IRONPOT && ppd->pot_state
        // > 0 && ppd->pot_state <= 3)` (`arkhata.c:3130`).
        if item.template_id == IID_ARKHATA_IRONPOT
            && is_player
            && facts.is_some_and(|facts| facts.pot_state > 0 && facts.pot_state <= 3)
        {
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_IRONPOT);
            events.push(PotmakerOutcomeEvent::QuestDone73GiveInfravisionPot {
                player_id: giver_id,
            });
            events.push(PotmakerOutcomeEvent::UpdatePotState {
                player_id: giver_id,
                new_state: 4,
            });
            self.npc_quiet_say(
                potmaker_id,
                "May you be blessed by all that is good in this world, I'm in your debt. Here, take this smaller pot which holds the same holy water. You might find need for it some time.",
            );
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:3145-3151`): hand the
        // item back to the giver.
        self.npc_say(
            potmaker_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_POTMAKER;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `potmaker_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::jada`'s `JadaDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PotmakerDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_POTMAKER_SPECIAL_POT`] to `ugaris-server`'s
/// `apply_potmaker_events`.
pub const fn qlog_potmaker_special_pot() -> usize {
    QLOG_POTMAKER_SPECIAL_POT
}
