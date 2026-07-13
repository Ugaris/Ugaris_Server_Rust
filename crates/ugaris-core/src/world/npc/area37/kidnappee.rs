//! Kidnappee NPC (`CDR_KIDNAPPEE`), the trainer's kidnapped student
//! rescued as part of quest 75 ("A Kidnapped Student").
//!
//! Ports `src/area/37/arkhata.c::kidnappee_driver` (`:4015-4195`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! ramin`/`trainer`: the caller supplies a per-player fact snapshot
//! ([`KidnappeePlayerFacts`]) up front and applies the returned
//! [`KidnappeeOutcomeEvent`]s afterwards, since `arkhata_ppd.kid_state`
//! lives on `crate::player::PlayerRuntime`, not `World`.
//!
//! `kidnappee_driver`'s six-state (`0`-`5`) rescue chain, gated at one
//! point on cross-driver state this file cannot see directly (read via
//! [`KidnappeePlayerFacts`]):
//! - `0` needs `arkhata_ppd.trainer_state > 0` (`world::npc::area37::
//!   trainer`'s own progress) to advance; C's own `case 0` falls through
//!   into `case 1`'s speech/advance-to-`2` in the same tick - collapsed
//!   into one `rs == 0` arm here, same "fallthrough lands on the next
//!   case's action" precedent as `world::npc::area37::ramin`'s own
//!   `rs == 0`/`9`/`11` arms.
//! - `2`/`3` are the Bend Iron Potion (`IID_ARKHATA_IRONPOTION`) check/
//!   wait pair: `2` consumes the potion via `has_item` (not `NT_GIVE`) if
//!   the player is carrying it and jumps straight to `4`, otherwise drops
//!   to `3` and waits; `3` jumps back to `2` once the potion appears -
//!   note C's own `case 3` never sets `didsay` (`arkhata.c:4105-4109`),
//!   so this state's own re-check is silent (no talk, no `last_talk`/
//!   `current_victim` update) until the potion actually shows up.
//! - `4` sets `CF_INVISIBLE` for [`KIDNAPPEE_INVIS_TICKS`] (the "walks
//!   off with the trainer" flourish) - `set_sector(ch[cn].x, ch[co].y)`
//!   (a genuine C typo mixing the kidnappee's own `x` with the player's
//!   `y`, `arkhata.c:4119`) is reproduced faithfully via [`World::
//!   mark_dirty_sector`] on that same mixed coordinate pair, matching the
//!   `set_sector`-via-`mark_dirty_sector` mapping `world::npc::area31::
//!   lostdwarf`'s own module doc comment established. The reappear tail
//!   (`arkhata.c:4179-4182`) uses the *correct* `ch[cn].x`/`ch[cn].y` pair.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim.
//! - `NT_TEXT`'s "repeat"/"restart" match arm (`arkhata.c:4150-4155`) is a
//!   dead comment in C (`// if (ppd && ...) {...}` - the actual state
//!   reset is commented out) - `didsay` is still truthy from
//!   `analyse_text_driver`'s own return value, so `current_victim`/
//!   `talkdir` still update, but no state change happens - reproduced
//!   verbatim as a no-op state-reset branch.
//! - `NT_CHAR`'s own `CF_INVISIBLE` early-out (`arkhata.c:4035-4038`,
//!   `if ((ch[cn].flags & CF_INVISIBLE)) { remove_message(...); continue;
//!   }`) and `NT_TEXT`'s matching one (`arkhata.c:4136-4139`) are both
//!   ported as plain early returns on `self.characters.get(&kidnappee_id)
//!   ...flags.contains(CharacterFlags::INVISIBLE)`.
//! - `NT_GIVE` never accepts any item - the only branch is the "hand it
//!   back" fallback (`arkhata.c:4163-4171`).
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `kidnappee_driver` body at all (matching the `rammy`/`ramin`/
//!   `trainer` "pure talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:4194`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_ARKHATA_IRONPOTION;
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:4070`, sibling drivers' own
/// identical guard).
const KIDNAPPEE_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const KIDNAPPEE_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:4053`).
const KIDNAPPEE_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:4058`).
const KIDNAPPEE_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:4188`): idle "return to post" threshold.
const KIDNAPPEE_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS * 60` (`arkhata.c:4179`): how long the rescued student stays
/// `CF_INVISIBLE` before reappearing.
const KIDNAPPEE_INVIS_TICKS: u64 = TICKS_PER_SECOND * 60;

/// Per-player facts [`World::process_kidnappee_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KidnappeePlayerFacts {
    /// `PlayerRuntime::arkhata_kid_state()`.
    pub kid_state: i32,
    /// `PlayerRuntime::arkhata_trainer_state()` (`ppd->trainer_state`,
    /// `arkhata.c:4081`): gates `rs` `0`.
    pub trainer_state: i32,
}

/// A side effect [`World::process_kidnappee_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KidnappeeOutcomeEvent {
    /// Write the new `arkhata_ppd.kid_state` back.
    UpdateKidState {
        player_id: CharacterId,
        new_state: i32,
    },
}

impl World {
    /// C `kidnappee_driver`'s per-tick body (`arkhata.c:4015-4195`).
    pub fn process_kidnappee_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, KidnappeePlayerFacts>,
        area_id: u16,
    ) -> Vec<KidnappeeOutcomeEvent> {
        let kidnappee_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_KIDNAPPEE
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for kidnappee_id in kidnappee_ids {
            self.process_kidnappee_messages(kidnappee_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_kidnappee_messages(
        &mut self,
        kidnappee_id: CharacterId,
        player_facts: &HashMap<CharacterId, KidnappeePlayerFacts>,
        area_id: u16,
        events: &mut Vec<KidnappeeOutcomeEvent>,
    ) {
        let Some(kidnappee_name) = self
            .characters
            .get(&kidnappee_id)
            .map(|kidnappee| kidnappee.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Kidnappee(mut data)) = self
            .characters
            .get(&kidnappee_id)
            .and_then(|kidnappee| kidnappee.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&kidnappee_id)
            .map(|kidnappee| std::mem::take(&mut kidnappee.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.kidnappee_handle_char_message(
                    kidnappee_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.kidnappee_handle_text_message(
                    kidnappee_id,
                    &kidnappee_name,
                    &mut data,
                    message,
                    &mut face_target,
                ),
                NT_GIVE => self.kidnappee_handle_give_message(kidnappee_id, message),
                _ => {}
            }
        }

        if let Some(kidnappee) = self.characters.get_mut(&kidnappee_id) {
            kidnappee.driver_state = Some(CharacterDriverState::Kidnappee(data));
        }

        // C `if ((ch[cn].flags & CF_INVISIBLE) && ticker - dat->misc >
        // TICKS*60) { ch[cn].flags &= ~CF_INVISIBLE; set_sector(...); }`
        // (`arkhata.c:4179-4182`).
        let tick = self.tick.0;
        if let Some(CharacterDriverState::Kidnappee(data)) = self
            .characters
            .get(&kidnappee_id)
            .and_then(|kidnappee| kidnappee.driver_state.clone())
        {
            let is_invisible = self
                .characters
                .get(&kidnappee_id)
                .is_some_and(|kidnappee| kidnappee.flags.contains(CharacterFlags::INVISIBLE));
            if is_invisible && tick.saturating_sub(data.misc) > KIDNAPPEE_INVIS_TICKS {
                if let Some(kidnappee) = self.characters.get_mut(&kidnappee_id) {
                    kidnappee.flags.remove(CharacterFlags::INVISIBLE);
                    let (x, y) = (kidnappee.x, kidnappee.y);
                    self.mark_dirty_sector(usize::from(x), usize::from(y));
                }
            }
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:4184-4186`).
        if let (Some(kidnappee), Some((tx, ty))) =
            (self.characters.get(&kidnappee_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(kidnappee.x), i32::from(kidnappee.y), tx, ty)
            {
                if let Some(kidnappee_mut) = self.characters.get_mut(&kidnappee_id) {
                    let _ = turn(kidnappee_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:4188-4192`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(kidnappee) = self.characters.get(&kidnappee_id) {
            match kidnappee.driver_state.as_ref() {
                Some(CharacterDriverState::Kidnappee(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + KIDNAPPEE_RETURN_TO_POST_TICKS < tick {
            let Some(kidnappee) = self.characters.get(&kidnappee_id) else {
                return;
            };
            let (post_x, post_y) = (kidnappee.rest_x, kidnappee.rest_y);
            self.secure_move_driver(
                kidnappee_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `kidnappee_driver`'s `NT_CHAR` branch (`arkhata.c:4030-4130`).
    #[allow(clippy::too_many_arguments)]
    fn kidnappee_handle_char_message(
        &mut self,
        kidnappee_id: CharacterId,
        data: &mut KidnappeeDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, KidnappeePlayerFacts>,
        events: &mut Vec<KidnappeeOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(kidnappee) = self.characters.get(&kidnappee_id).cloned() else {
            return;
        };
        // C `if ((ch[cn].flags & CF_INVISIBLE))` (`arkhata.c:4035`).
        if kidnappee.flags.contains(CharacterFlags::INVISIBLE) {
            return;
        }
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:4041`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:4047`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:4053`).
        if tick < data.last_talk + KIDNAPPEE_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:4058`).
        if tick < data.last_talk + KIDNAPPEE_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:4064`).
        if kidnappee_id == player_id
            || !char_see_char(&kidnappee, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:4070`).
        if char_dist(&kidnappee, &player) > KIDNAPPEE_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.kid_state;
        let mut make_invisible = false;
        match facts.kid_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:
            // 4080-4091`) - see the module doc comment.
            0 if facts.trainer_state > 0 => {
                self.npc_quiet_say(
                    kidnappee_id,
                    "You must have been sent to rescue me! Oh how glad I am to see you! Please open this cage and let me out!",
                );
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:4092-4104`) - see the module doc
            // comment.
            2 => {
                if self.character_has_item_template(player_id, IID_ARKHATA_IRONPOTION) {
                    self.npc_emote(player_id, "uses the bend iron potion to open the cage.");
                    self.destroy_items_by_template_id(player_id, IID_ARKHATA_IRONPOTION);
                    new_state = 4;
                } else {
                    self.npc_quiet_say(
                        kidnappee_id,
                        "Only their leader could seal my cage, only he gets close to me. He must have the secret to open this cage.",
                    );
                    new_state = 3;
                }
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:4105-4109`) - no `didsay` set here.
            3 => {
                if self.character_has_item_template(player_id, IID_ARKHATA_IRONPOTION) {
                    new_state = 2;
                }
            }
            // C `case 4:` (`arkhata.c:4110-4120`) - see the module doc
            // comment for the `set_sector` coordinate-mixing quirk.
            4 => {
                self.npc_quiet_say(kidnappee_id, "Thank thee so much for rescuing me.");
                self.queue_system_text(
                    player_id,
                    "You've rescued the student. Now go back to the trainer to claim your reward.",
                );
                new_state = 5;
                didsay = true;
                make_invisible = true;
            }
            // C `case 5: break;` (`arkhata.c:4121-4122`): all done.
            5 => {}
            _ => {}
        }

        if new_state != facts.kid_state {
            events.push(KidnappeeOutcomeEvent::UpdateKidState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:4124-4128`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }

        // C `dat->misc = ticker; ch[cn].flags |= CF_INVISIBLE;
        // set_sector(ch[cn].x, ch[co].y);` (`arkhata.c:4117-4119`) - the
        // `ch[co].y` half is a genuine C typo (mixes the kidnappee's own
        // `x` with the player's `y`), reproduced verbatim.
        if make_invisible {
            data.misc = tick;
            if let Some(kidnappee_mut) = self.characters.get_mut(&kidnappee_id) {
                kidnappee_mut.flags.insert(CharacterFlags::INVISIBLE);
            }
            self.mark_dirty_sector(usize::from(kidnappee.x), usize::from(player.y));
        }
    }

    /// C `kidnappee_driver`'s `NT_TEXT` branch (`arkhata.c:4133-4160`),
    /// wired through the generic `analyse_text_qa` matcher.
    fn kidnappee_handle_text_message(
        &mut self,
        kidnappee_id: CharacterId,
        kidnappee_name: &str,
        data: &mut KidnappeeDriverData,
        message: &CharacterDriverMessage,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if ((ch[cn].flags & CF_INVISIBLE))` (`arkhata.c:4136`).
        if self
            .characters
            .get(&kidnappee_id)
            .is_some_and(|kidnappee| kidnappee.flags.contains(CharacterFlags::INVISIBLE))
        {
            return;
        }

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:4141-4143`).
        let tick = self.tick.0;
        if tick > data.last_talk + KIDNAPPEE_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:4145`).
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
        let Some(kidnappee) = self.characters.get(&kidnappee_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if kidnappee_id == speaker_id {
            return;
        }
        if char_dist(&kidnappee, &speaker) > KIDNAPPEE_QA_DISTANCE
            || !char_see_char(&kidnappee, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        // C `switch ((didsay = analyse_text_driver(...))) { case 2: ppd =
        // set_data(...); /* commented-out reset */ break; }` - see the
        // module doc comment: the "repeat" branch is a dead no-op in C,
        // only `didsay`'s truthiness (any nonzero match) matters below.
        let outcome = analyse_text_qa(text, kidnappee_name, &speaker.name, ARKHATA_QA);
        if let TextAnalysisOutcome::Said(reply) = &outcome {
            self.npc_quiet_say(kidnappee_id, reply);
        }
        let didsay = !matches!(outcome, TextAnalysisOutcome::NoMatch);

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:4156-4159`) - note this does *not* touch `dat->
        // last_talk`.
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `kidnappee_driver`'s `NT_GIVE` branch (`arkhata.c:4163-4171`):
    /// the only behavior it has is handing the item straight back.
    fn kidnappee_handle_give_message(
        &mut self,
        kidnappee_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&kidnappee_id)
            .and_then(|kidnappee| kidnappee.cursor_item.take())
        else {
            return;
        };
        self.npc_say(
            kidnappee_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_KIDNAPPEE;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` holds the `CF_INVISIBLE` reappear
/// timestamp (C `dat->misc = ticker`, `arkhata.c:4117`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KidnappeeDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
    #[serde(default)]
    pub misc: u64,
}
