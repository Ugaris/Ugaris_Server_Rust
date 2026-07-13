//! Hunter NPC (`CDR_HUNTER`), the Arkhata hunter who runs "The Blue Harpy"
//! (quest 77).
//!
//! Ports `src/area/37/arkhata.c::hunter_driver` (`:3173-3369`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:109-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::jaz`:
//! the caller supplies a per-player fact snapshot ([`HunterPlayerFacts`])
//! up front and applies the returned [`HunterOutcomeEvent`]s afterwards,
//! since `arkhata_ppd.hunter_state` lives on `crate::player::PlayerRuntime`,
//! not `World`.
//!
//! `hunter_driver`'s eleven-state (`0`-`10`) dialogue chain is entirely
//! local to this file except for its single greeting gate:
//! - `0` needs `arkhata_ppd.pot_state > 0` (`world::npc::area37::potmaker`'s
//!   own progress) to advance, then falls through into `case 1`'s speech/
//!   advance-to-`2` in the same tick - collapsed into one `hs == 0` arm
//!   here, same "fallthrough lands on the next case's action" precedent as
//!   `world::npc::area37::rammy`'s own `rs == 6`/`13`/`17` arms. Unlike
//!   those, state `1` *can* be independently stored: the `NT_TEXT` "repeat"
//!   reset (`case 2`, below) rewinds straight to `1`, not `0` (C:
//!   `ppd->hunter_state = 1;`, `arkhata.c:3316`) - so a dedicated `hs == 1`
//!   arm exists too, duplicating `case 1`'s own text, same "rewound state
//!   gets its own arm" precedent as `world::npc::area37::rammy`'s `rs ==
//!   14` arm.
//! - `4` needs `ch[co].level >= 58` to advance, then falls through into
//!   `case 5`'s speech/`questlog_open(77)`/advance-to-`6` in the same tick -
//!   collapsed into one `hs == 4` arm. State `5` itself is never
//!   independently stored (no reset path targets it), so no separate arm
//!   exists for it.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches
//!   `world::npc::area37::rammy`'s identical observation for that file's
//!   shared driver shape).
//! - `NT_GIVE`'s successful harpy-skin turn-in (`:3331-3342`) speaks before
//!   destroying/paying (unlike `jaz_driver`'s silent turn-in) - reproduced
//!   verbatim, including the double item-destruction quirk: C both
//!   `destroy_item_byID(co, IID_ARKHATA_HARPY)` (searches the giver's own
//!   inventory for a *second* copy, if any) and separately
//!   `destroy_item(ch[cn].citem)` (destroys the one just handed over,
//!   already removed from `ch[cn].citem` by the message-loop's own logic in
//!   C - here, taken from `cursor_item` up front like every other `NT_GIVE`
//!   handler in this codebase).
//! - No self-defense/regen/spell-self cascade exists in C's `hunter_driver`
//!   body at all (matching the `rammy_driver`/`brannington.c` "pure talker"
//!   NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:3368`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_ARKHATA_HARPY;
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:3222`).
const HUNTER_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const HUNTER_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:3205`).
const HUNTER_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:3210`).
const HUNTER_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:3362`): idle "return to post" threshold.
const HUNTER_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C quest 77, "The Blue Harpy".
const QLOG_HUNTER_HARPY: usize = 77;
/// C `give_money(co, 150 * 100, "Solved Hunter Quest")` (`arkhata.c:3338`).
const HUNTER_REWARD_GOLD: u32 = 150 * 100;

/// Per-player facts [`World::process_hunter_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HunterPlayerFacts {
    /// `PlayerRuntime::arkhata_hunter_state()`.
    pub hunter_state: i32,
    /// `PlayerRuntime::arkhata_pot_state()` (`ppd->pot_state`,
    /// `arkhata.c:3233`): gates `hs` `0`.
    pub pot_state: i32,
}

/// A side effect [`World::process_hunter_actions`] could not apply directly
/// because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunterOutcomeEvent {
    /// Write the new `arkhata_ppd.hunter_state` back.
    UpdateHunterState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 77)` (`arkhata.c:3264`).
    QuestOpen77 { player_id: CharacterId },
    /// C `questlog_done(co, 77)` (`arkhata.c:3333`), the `NT_GIVE`
    /// harpy-skin turn-in.
    QuestDone77 { player_id: CharacterId },
}

impl World {
    /// C `hunter_driver`'s per-tick body (`arkhata.c:3173-3369`).
    pub fn process_hunter_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, HunterPlayerFacts>,
        area_id: u16,
    ) -> Vec<HunterOutcomeEvent> {
        let hunter_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_HUNTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for hunter_id in hunter_ids {
            self.process_hunter_messages(hunter_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_hunter_messages(
        &mut self,
        hunter_id: CharacterId,
        player_facts: &HashMap<CharacterId, HunterPlayerFacts>,
        area_id: u16,
        events: &mut Vec<HunterOutcomeEvent>,
    ) {
        let Some(hunter_name) = self
            .characters
            .get(&hunter_id)
            .map(|hunter| hunter.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Hunter(mut data)) = self
            .characters
            .get(&hunter_id)
            .and_then(|hunter| hunter.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&hunter_id)
            .map(|hunter| std::mem::take(&mut hunter.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.hunter_handle_char_message(
                    hunter_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.hunter_handle_text_message(
                    hunter_id,
                    &hunter_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.hunter_handle_give_message(hunter_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(hunter) = self.characters.get_mut(&hunter_id) {
            hunter.driver_state = Some(CharacterDriverState::Hunter(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:3358-3360`).
        if let (Some(hunter), Some((tx, ty))) =
            (self.characters.get(&hunter_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(hunter.x), i32::from(hunter.y), tx, ty) {
                if let Some(hunter_mut) = self.characters.get_mut(&hunter_id) {
                    let _ = turn(hunter_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:3362-3366`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(hunter) = self.characters.get(&hunter_id) {
            match hunter.driver_state.as_ref() {
                Some(CharacterDriverState::Hunter(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + HUNTER_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(hunter) = self.characters.get(&hunter_id) else {
                return;
            };
            let (post_x, post_y) = (hunter.rest_x, hunter.rest_y);
            self.secure_move_driver(
                hunter_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `hunter_driver`'s `NT_CHAR` branch (`arkhata.c:3189-3296`).
    #[allow(clippy::too_many_arguments)]
    fn hunter_handle_char_message(
        &mut self,
        hunter_id: CharacterId,
        data: &mut HunterDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, HunterPlayerFacts>,
        events: &mut Vec<HunterOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(hunter) = self.characters.get(&hunter_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:3193`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:3199`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:3205`).
        if tick < data.last_talk + HUNTER_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:3210`).
        if tick < data.last_talk + HUNTER_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:3216`).
        if hunter_id == player_id || !char_see_char(&hunter, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:3222`).
        if char_dist(&hunter, &player) > HUNTER_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.hunter_state;
        match facts.hunter_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:3232-
            // 3243`) - see the module doc comment.
            0 if facts.pot_state > 0 => {
                self.npc_quiet_say(
                    hunter_id,
                    "Hail adventurer! I see you are seeking for a ceremonial pot. Well that is an odd coincidence.",
                );
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 1:` (`arkhata.c:3238-3243`), reached directly only via
            // the `NT_TEXT` "repeat" rewind (see the module doc comment) -
            // `case 0`'s own fallthrough always lands on `2` in one tick.
            1 => {
                self.npc_quiet_say(
                    hunter_id,
                    "Hail adventurer! I see you are seeking for a ceremonial pot. Well that is an odd coincidence.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`arkhata.c:3244-3249`).
            2 => {
                self.npc_quiet_say(
                    hunter_id,
                    "Last night I heard the strangest noises from the bandit's hideout, one of them came running out yelling about a frozen toe.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:3250-3254`).
            3 => {
                self.npc_quiet_say(
                    hunter_id,
                    "He made the wolf I was about to slay run away from me with all that screaming.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` falling through into `case 5:` (`arkhata.c:3255-
            // 3267`) - see the module doc comment.
            4 if player.level >= 58 => {
                self.npc_quiet_say(
                    hunter_id,
                    "It's great to see you again adventurer! News of your deeds in Arkhata has reached even me.",
                );
                events.push(HunterOutcomeEvent::QuestOpen77 { player_id });
                new_state = 6;
                didsay = true;
            }
            4 => {}
            // C `case 6:` (`arkhata.c:3268-3273`).
            6 => {
                self.npc_quiet_say(
                    hunter_id,
                    "Along with several complaints of a troublesome blue harpy. I have searched for it many a night.",
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7:` (`arkhata.c:3274-3279`).
            7 => {
                self.npc_quiet_say(
                    hunter_id,
                    "Following hints from people who's seen it. Yet I have failed to seek out and slay the beast, it appears to move across a vast territory.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8:` (`arkhata.c:3280-3284`).
            8 => {
                self.npc_quiet_say(
                    hunter_id,
                    "I will reward you for slaying it, but its skin and the honour should be mine.",
                );
                new_state = 9;
                didsay = true;
            }
            // C `case 9: break;` (`arkhata.c:3285-3286`): waiting for the
            // harpy skin, handled by `NT_GIVE`.
            9 => {}
            // C `case 10: break;` (`arkhata.c:3287-3288`): all done.
            10 => {}
            _ => {}
        }

        if new_state != facts.hunter_state {
            events.push(HunterOutcomeEvent::UpdateHunterState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:3290-3294`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `hunter_driver`'s `NT_TEXT` branch (`arkhata.c:3299-3324`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn hunter_handle_text_message(
        &mut self,
        hunter_id: CharacterId,
        hunter_name: &str,
        data: &mut HunterDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, HunterPlayerFacts>,
        events: &mut Vec<HunterOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:3302-3304`).
        let tick = self.tick.0;
        if tick > data.last_talk + HUNTER_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:3306`).
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
        let Some(hunter) = self.characters.get(&hunter_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if hunter_id == speaker_id {
            return;
        }
        if char_dist(&hunter, &speaker) > HUNTER_QA_DISTANCE
            || !char_see_char(&hunter, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let hunter_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.hunter_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, hunter_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(hunter_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:3311-3319`): rewind to `1` if
            // the greeting/pot-noise/wolf mini-block is in progress.
            TextAnalysisOutcome::Matched(2) => {
                if hunter_state > 0 && hunter_state <= 4 {
                    data.last_talk = 0;
                    events.push(HunterOutcomeEvent::UpdateHunterState {
                        player_id: speaker_id,
                        new_state: 1,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by hunter's own
            // `switch` but still counts as `didsay` (C: `switch (didsay =
            // analyse_text_driver(...))` - any nonzero return is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:3320-3323`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit resets above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `hunter_driver`'s `NT_GIVE` branch (`arkhata.c:3327-3351`).
    fn hunter_handle_give_message(
        &mut self,
        hunter_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, HunterPlayerFacts>,
        events: &mut Vec<HunterOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&hunter_id)
            .and_then(|hunter| hunter.cursor_item.take())
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

        // C `if (it[in].ID == IID_ARKHATA_HARPY && ppd->hunter_state >= 5
        // && ppd->hunter_state <= 9)` (`arkhata.c:3331`).
        if item.template_id == IID_ARKHATA_HARPY
            && is_player
            && facts.is_some_and(|facts| (5..=9).contains(&facts.hunter_state))
        {
            events.push(HunterOutcomeEvent::QuestDone77 {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_HARPY);
            events.push(HunterOutcomeEvent::UpdateHunterState {
                player_id: giver_id,
                new_state: 10,
            });
            self.npc_say(
                hunter_id,
                "Ah you did it! I knew I could count on you. Here take these 150 gold coins and say nothing of this to anyone. Farewell!",
            );
            if let Some(player) = self.characters.get_mut(&giver_id) {
                player.gold = player.gold.saturating_add(HUNTER_REWARD_GOLD);
                player.flags.insert(CharacterFlags::ITEMS);
            }
            self.queue_system_text_bytes(giver_id, give_money_message(HUNTER_REWARD_GOLD));
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:3343-3349`): hand the item
        // back to the giver.
        self.npc_say(
            hunter_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_HUNTER;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `hunter_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::rammy`'s `RammyDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HunterDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_HUNTER_HARPY`] to `ugaris-server`'s `apply_hunter_events`.
pub const fn qlog_hunter_harpy() -> usize {
    QLOG_HUNTER_HARPY
}
