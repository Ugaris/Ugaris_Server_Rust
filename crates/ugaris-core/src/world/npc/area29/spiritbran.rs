//! Spirit of Brannington NPC (`CDR_SPIRITBRAN`), the ghost who explains the
//! necromancer plot and runs "The Brannington Holy Relic" (quest 44).
//!
//! Ports `src/area/29/brannington.c::spirit_brannington_driver` (`:1128-
//! 1308`) plus its shared `analyse_text_driver`/`qa[]` table (`:86-206`),
//! ported as [`super::AREA29_QA`] in `world::npc::area29` (the same table
//! every other `brannington.c` NPC driver shares). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area28::
//! aristocrat`/`yoatin`: the caller supplies a per-player fact snapshot
//! ([`SpiritBranPlayerFacts`]) up front and applies the returned
//! [`SpiritBranOutcomeEvent`]s afterwards, since `staffer_ppd.
//! spiritbran_state` and the `QLOG` 44 quest-log entry live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! `spirit_brannington_driver`'s six-state (`0`-`5`) dialogue chain:
//! greeting (opens quest 44) -> "a Necromancer has revived their ancestors"
//! -> "retrieve the Brannington Holy Relic" -> "also kill this Necromancer,
//! ask Count Brannington about his jewelry" -> (waiting: state `4`) ->
//! (`NT_GIVE`: hand in `IID_STAFF_HOLYRELIC`, quest 44 done, state jumps to
//! `5`, first completion only grants one save via
//! [`SpiritBranOutcomeEvent::QuestDone`]'s `times_done == 1` gate - same
//! precedent as `world::npc::area28::aristocrat`'s money reward, but here
//! rewarding `Character::saves` instead of an item since `ch[co].saves`
//! lives on `World`, not `PlayerRuntime`) -> done.
//!
//! Deviations/gaps (documented, not silent):
//! - Like `world::npc::area28::aristocrat`/`yoatin`'s `NT_TEXT` branch,
//!   this driver's own C body has no `dat->current_victim` staleness-reset
//!   preamble and no victim-mismatch early-out at all - reproduced
//!   verbatim: replies to *any* nearby player's matched small talk, not
//!   just its tracked victim.
//! - Like `world::npc::area28::aristocrat`'s own `case 3` (but unlike
//!   `world::npc::area26::smugglecom`'s silent one), this driver's `case 3`
//!   (`:1240-1245`) speaks a visible `say(cn, "reset done")` line (not
//!   `quiet_say`) before wiping the state - ported via
//!   [`crate::world::World::npc_say`].
//! - No self-defense/regen/spell-self cascade exists in C's `spirit_
//!   brannington_driver` body at all (matching `world::astro1`/
//!   `world::npc::area28::aristocrat`/`yoatin`'s identical observation for
//!   other "pure talker" NPCs) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:1307`) is not
//!   ported, matching the established `world::thomas`/`world::sir_jones`
//!   precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:1177`).
const SPIRITBRAN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const SPIRITBRAN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:1160`).
const SPIRITBRAN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:1165`).
const SPIRITBRAN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:1301`): idle "return to post" threshold.
const SPIRITBRAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ch[co].saves < 10` (`brannington.c:1270`): the save cap.
const SPIRITBRAN_SAVE_CAP: u8 = 10;

/// Per-player facts [`World::process_spiritbran_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiritBranPlayerFacts {
    /// `PlayerRuntime::staffer_spiritbran_state()`.
    pub spiritbran_state: i32,
}

/// A side effect [`World::process_spiritbran_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiritBranOutcomeEvent {
    /// Write the new `staffer_ppd.spiritbran_state` back.
    UpdateSpiritBranState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 44)`.
    QuestOpen { player_id: CharacterId },
    /// C `tmp = questlog_done(co, 44); ... if (tmp == 1 && !(ch[co].flags &
    /// CF_HARDCORE) && ch[co].saves < 10) { ch[co].saves++; log_char(co,
    /// LOG_SYSTEM, 0, "You received one save."); }` (`brannington.c:1268-
    /// 1273`) - the `times_done == 1` gate is applied by the caller
    /// (`ugaris-server`'s `apply_spiritbran_events`), same precedent as
    /// `world::npc::area28::aristocrat`'s `QuestDone` gold reward.
    QuestDone { player_id: CharacterId },
    /// C `case 3:` (`brannington.c:1240-1245`): the god-only "reset me"
    /// state wipe.
    ResetSpiritBran { player_id: CharacterId },
}

impl World {
    /// C `spirit_brannington_driver`'s per-tick body (`brannington.c:1128-
    /// 1308`).
    pub fn process_spiritbran_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, SpiritBranPlayerFacts>,
        area_id: u16,
    ) -> Vec<SpiritBranOutcomeEvent> {
        let spiritbran_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SPIRITBRAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for spiritbran_id in spiritbran_ids {
            self.process_spiritbran_messages(spiritbran_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_spiritbran_messages(
        &mut self,
        spiritbran_id: CharacterId,
        player_facts: &HashMap<CharacterId, SpiritBranPlayerFacts>,
        area_id: u16,
        events: &mut Vec<SpiritBranOutcomeEvent>,
    ) {
        let Some(spiritbran_name) = self
            .characters
            .get(&spiritbran_id)
            .map(|spiritbran| spiritbran.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::SpiritBran(mut data)) = self
            .characters
            .get(&spiritbran_id)
            .and_then(|spiritbran| spiritbran.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&spiritbran_id)
            .map(|spiritbran| std::mem::take(&mut spiritbran.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.spiritbran_handle_char_message(
                    spiritbran_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.spiritbran_handle_text_message(
                    spiritbran_id,
                    &spiritbran_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.spiritbran_handle_give_message(
                    spiritbran_id,
                    message,
                    player_facts,
                    events,
                ),
                _ => {}
            }
        }

        if let Some(spiritbran) = self.characters.get_mut(&spiritbran_id) {
            spiritbran.driver_state = Some(CharacterDriverState::SpiritBran(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:1297-1299`).
        if let (Some(spiritbran), Some((tx, ty))) =
            (self.characters.get(&spiritbran_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(spiritbran.x), i32::from(spiritbran.y), tx, ty)
            {
                if let Some(spiritbran_mut) = self.characters.get_mut(&spiritbran_id) {
                    let _ = turn(spiritbran_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_UP, ret,
        // lastact)) return; }` (`brannington.c:1301-1305`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area28::aristocrat` already uses.
        let last_talk = if let Some(spiritbran) = self.characters.get(&spiritbran_id) {
            match spiritbran.driver_state.as_ref() {
                Some(CharacterDriverState::SpiritBran(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + SPIRITBRAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(spiritbran) = self.characters.get(&spiritbran_id) else {
                return;
            };
            let (post_x, post_y) = (spiritbran.rest_x, spiritbran.rest_y);
            self.secure_move_driver(
                spiritbran_id,
                post_x,
                post_y,
                Direction::Up as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `spirit_brannington_driver`'s `NT_CHAR` branch (`brannington.c:
    /// 1144-1225`).
    #[allow(clippy::too_many_arguments)]
    fn spiritbran_handle_char_message(
        &mut self,
        spiritbran_id: CharacterId,
        data: &mut SpiritBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SpiritBranPlayerFacts>,
        events: &mut Vec<SpiritBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(spiritbran) = self.characters.get(&spiritbran_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:1147-1151`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:1153-1157`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:1159-1163`).
        if tick < data.last_talk + SPIRITBRAN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:1165-1168`).
        if tick < data.last_talk + SPIRITBRAN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:1170-1174`).
        if spiritbran_id == player_id
            || !char_see_char(&spiritbran, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:1176-
        // 1180`).
        if char_dist(&spiritbran, &player) > SPIRITBRAN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.spiritbran_state;
        match facts.spiritbran_state {
            // C `case 0:` (`brannington.c:1187-1194`).
            0 => {
                self.npc_quiet_say(
                    spiritbran_id,
                    &format!(
                        "Greetings {}, I have watched thee from below here, and have now need for thy strength!",
                        player.name
                    ),
                );
                events.push(SpiritBranOutcomeEvent::QuestOpen { player_id });
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`brannington.c:1195-1200`).
            1 => {
                self.npc_quiet_say(
                    spiritbran_id,
                    "Unbeknownst to everyone in Brannington, a Necromancer has revived their ancestors, and is trying to take over the town with his army of undead.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`brannington.c:1201-1206`).
            2 => {
                self.npc_quiet_say(
                    spiritbran_id,
                    "If you can retrieve the Brannington Holy Relic which the necromancer has taken, then his army will no longer obey him, and will be trapped here forever.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`brannington.c:1207-1213`).
            3 => {
                self.npc_quiet_say(
                    spiritbran_id,
                    "Also, I wish for you to kill this Necromancer. However, in order to get into the crypt, you will have to ask Count Brannington about his jewelry. You will need his help to open the crypt doors.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4: break;` (`brannington.c:1214-1215`): waiting for
            // the holy relic.
            4 => {}
            // C `case 5: break;` (`brannington.c:1216-1217`): all done.
            5 => {}
            _ => {}
        }

        if new_state != facts.spiritbran_state {
            events.push(SpiritBranOutcomeEvent::UpdateSpiritBranState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:1219-1223`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `spirit_brannington_driver`'s `NT_TEXT` branch (`brannington.c:
    /// 1228-1252`), wired through the generic `analyse_text_qa` matcher
    /// (same pattern as `world::npc::area28::aristocrat`/`yoatin`'s text
    /// handler). This branch has no victim-staleness-reset preamble and no
    /// victim-mismatch early-out (see the module doc comment).
    #[allow(clippy::too_many_arguments)]
    fn spiritbran_handle_text_message(
        &mut self,
        spiritbran_id: CharacterId,
        spiritbran_name: &str,
        data: &mut SpiritBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SpiritBranPlayerFacts>,
        events: &mut Vec<SpiritBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:1231`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if spiritbran_id == speaker_id {
            return;
        }
        let Some(spiritbran) = self.characters.get(&spiritbran_id).cloned() else {
            return;
        };
        if char_dist(&spiritbran, &speaker) > SPIRITBRAN_QA_DISTANCE
            || !char_see_char(&spiritbran, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let spiritbran_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.spiritbran_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, spiritbran_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(spiritbran_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:1234-1239`): reset back to the
            // greeting if not yet past it.
            TextAnalysisOutcome::Matched(2) => {
                if spiritbran_state <= 4 {
                    data.last_talk = 0;
                    events.push(SpiritBranOutcomeEvent::UpdateSpiritBranState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`brannington.c:1240-1245`): the god-only "reset
            // me" wipe, which speaks a visible `say(cn, "reset done")` line
            // first (see the module doc comment).
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(spiritbran_id, "reset done");
                    events.push(SpiritBranOutcomeEvent::ResetSpiritBran {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the area-shared `4`/`5` gold/silver
            // trade codes, consumed only by `broklin_driver`, not yet
            // ported) is unhandled by spirit's own C `switch` but still
            // counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington.c:1247-1250`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `spirit_brannington_driver`'s `NT_GIVE` branch (`brannington.c:
    /// 1255-1289`).
    fn spiritbran_handle_give_message(
        &mut self,
        spiritbran_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SpiritBranPlayerFacts>,
        events: &mut Vec<SpiritBranOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&spiritbran_id)
            .and_then(|spiritbran| spiritbran.cursor_item.take())
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

        // C `if (it[in].ID == IID_STAFF_HOLYRELIC && ppd &&
        // ppd->spiritbran_state < 5)` (`brannington.c:1262`).
        if item.template_id == IID_STAFF_HOLYRELIC
            && is_player
            && facts.is_some_and(|facts| facts.spiritbran_state < 5)
        {
            self.npc_quiet_say(
                spiritbran_id,
                &format!(
                    "The Branningtons owe you much great hero. I give thee Ishtar's blessings... May your journeys be full of adventure and glory, {}!",
                    giver.name
                ),
            );
            events.push(SpiritBranOutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_STAFF_HOLYRELIC);
            events.push(SpiritBranOutcomeEvent::UpdateSpiritBranState {
                player_id: giver_id,
                new_state: 5,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`brannington.c:1276-1281`): hand the
        // item back to the giver. C uses `quiet_say` here (not `say`) -
        // found while porting the sibling `world::npc::area29::countbran`
        // driver and cross-checking every `brannington.c` occurrence of
        // this exact fallback line: all ten use `quiet_say`, unlike
        // `brannington_forest.c`'s aristocrat/yoatin, which use `say`.
        self.npc_quiet_say(
            spiritbran_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_LOSTCON, CDR_SPIRITBRAN};
use crate::item_driver::IID_STAFF_HOLYRELIC;

/// C `struct spirit_brannington_data` (`src/area/29/brannington.c:1128-
/// 1134`, inline local declaration mirrored on `world::npc::area28::
/// aristocrat`'s `struct aristocrat_data` shape).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpiritBranDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`SPIRITBRAN_SAVE_CAP`] to `ugaris-server`'s `apply_spiritbran_
/// events` (C `ch[co].saves < 10`, `brannington.c:1270`).
pub const fn spiritbran_save_cap() -> u8 {
    SPIRITBRAN_SAVE_CAP
}
