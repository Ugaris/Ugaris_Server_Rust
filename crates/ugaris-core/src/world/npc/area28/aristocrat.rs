//! Aristocrat NPC (`CDR_ARISTOCRAT`), the robbed noble in Brannington
//! Forest who runs "The Family Heirloom" quest chain (quest 38).
//!
//! Ports `src/area/28/brannington_forest.c::aristocrat_driver` (`:234-424`)
//! plus its shared `analyse_text_driver`/`qa[]` table (`:75-199`), ported as
//! [`super::AREA28_QA`] in `world::npc::area28` (the same table
//! `world::npc::area28::yoatin` shares). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area26::
//! smugglecom`/`rouven`: the caller supplies a per-player fact snapshot
//! ([`AristocratPlayerFacts`]) up front and applies the returned
//! [`AristocratOutcomeEvent`]s afterwards, since `staffer_ppd.
//! aristocrat_state` and the `QLOG` 38 quest-log entry live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! `aristocrat_driver`'s nine-state (`0`-`8`) dialogue chain: greeting ->
//! "you look like an adventurer" -> "don't growl at me" -> "wildlife at a
//! lake north of here" -> "a native attacked me" -> "I lost my Amulet" ->
//! "please retrieve it" -> (`NT_GIVE`: hand in `IID_STAFF_ARIAMULET`, quest
//! 38 done, state jumps to `8`, first completion only grants 1000g via
//! `AristocratOutcomeEvent::QuestDone`'s `times_done == 1` gate - same
//! precedent as `world::npc::area3::astro2`'s money reward) -> done.
//!
//! Deviations/gaps (documented, not silent):
//! - Unlike `world::thomas`/`world::sir_jones`'s `NT_TEXT` branch (but like
//!   `world::npc::area26::smugglecom`/`rouven`'s), this driver's own C body
//!   has no `dat->current_victim` staleness-reset preamble and no victim-
//!   mismatch early-out at all - reproduced verbatim: replies to *any*
//!   nearby player's matched small talk, not just its tracked victim.
//! - Unlike `world::npc::area26::smugglecom`'s own silent `case 3` ("reset
//!   me"), this driver's `case 3` (`:355-360`) *does* speak a visible
//!   `say(cn, "reset done")` line (not `quiet_say`) before wiping the
//!   state - ported via [`crate::world::World::npc_say`], matching C's own
//!   choice of the area-fanned, non-quote-filtered variant.
//! - No self-defense/regen/spell-self cascade exists in C's `aristocrat_
//!   driver` body at all (matching `world::astro1`/`world::npc::area26::
//!   smugglecom`'s identical observation for other "pure talker" NPCs) -
//!   this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:423`) is not
//!   ported, matching the established `world::thomas`/`world::sir_jones`
//!   precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA28_QA;

/// C `char_dist(cn, co) > 10` (`brannington_forest.c:283`).
const ARISTOCRAT_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington_forest.c:120`, the shared
/// `analyse_text_driver` copy's own guard).
const ARISTOCRAT_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington_forest.c:266`).
const ARISTOCRAT_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington_forest.c:271`).
const ARISTOCRAT_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington_forest.c:417`): idle "return to post"
/// threshold.
const ARISTOCRAT_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// Per-player facts [`World::process_aristocrat_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AristocratPlayerFacts {
    /// `PlayerRuntime::staffer_aristocrat_state()`.
    pub aristocrat_state: i32,
}

/// A side effect [`World::process_aristocrat_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AristocratOutcomeEvent {
    /// Write the new `staffer_ppd.aristocrat_state` back.
    UpdateAristocratState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 38)`.
    QuestOpen { player_id: CharacterId },
    /// C `tmp = questlog_done(co, 38); ... if (tmp == 1 && (in =
    /// create_money_item(1000 * 100))) { give_char_item(co, in); }`
    /// (`brannington_forest.c:380-388`) - applied via the standard
    /// `complete_legacy` flow (real quest-table exp), with the manual gold
    /// reward gated on `completion.times_done == 1`, same precedent as
    /// `world::npc::area3::astro2`'s `QuestDone` money reward.
    QuestDone { player_id: CharacterId },
    /// C `case 3:` (`brannington_forest.c:355-360`): the god-only "reset
    /// me" state wipe.
    ResetAristocrat { player_id: CharacterId },
}

impl World {
    /// C `aristocrat_driver`'s per-tick body (`brannington_forest.c:234-424`).
    pub fn process_aristocrat_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, AristocratPlayerFacts>,
        area_id: u16,
    ) -> Vec<AristocratOutcomeEvent> {
        let aristocrat_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ARISTOCRAT
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for aristocrat_id in aristocrat_ids {
            self.process_aristocrat_messages(aristocrat_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_aristocrat_messages(
        &mut self,
        aristocrat_id: CharacterId,
        player_facts: &HashMap<CharacterId, AristocratPlayerFacts>,
        area_id: u16,
        events: &mut Vec<AristocratOutcomeEvent>,
    ) {
        let Some(aristocrat_name) = self
            .characters
            .get(&aristocrat_id)
            .map(|aristocrat| aristocrat.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Aristocrat(mut data)) = self
            .characters
            .get(&aristocrat_id)
            .and_then(|aristocrat| aristocrat.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&aristocrat_id)
            .map(|aristocrat| std::mem::take(&mut aristocrat.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.aristocrat_handle_char_message(
                    aristocrat_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.aristocrat_handle_text_message(
                    aristocrat_id,
                    &aristocrat_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.aristocrat_handle_give_message(
                    aristocrat_id,
                    message,
                    player_facts,
                    events,
                ),
                _ => {}
            }
        }

        if let Some(aristocrat) = self.characters.get_mut(&aristocrat_id) {
            aristocrat.driver_state = Some(CharacterDriverState::Aristocrat(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington_forest.c:413-415`).
        if let (Some(aristocrat), Some((tx, ty))) =
            (self.characters.get(&aristocrat_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(aristocrat.x), i32::from(aristocrat.y), tx, ty)
            {
                if let Some(aristocrat_mut) = self.characters.get_mut(&aristocrat_id) {
                    let _ = turn(aristocrat_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret,
        // lastact)) return; }` (`brannington_forest.c:417-421`). The NPC's
        // post position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the
        // same substitution `world::npc::area26::smugglecom` already uses.
        let last_talk = if let Some(aristocrat) = self.characters.get(&aristocrat_id) {
            match aristocrat.driver_state.as_ref() {
                Some(CharacterDriverState::Aristocrat(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + ARISTOCRAT_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(aristocrat) = self.characters.get(&aristocrat_id) else {
                return;
            };
            let (post_x, post_y) = (aristocrat.rest_x, aristocrat.rest_y);
            self.secure_move_driver(
                aristocrat_id,
                post_x,
                post_y,
                Direction::Left as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `aristocrat_driver`'s `NT_CHAR` branch (`brannington_forest.c:249-
    /// 340`).
    #[allow(clippy::too_many_arguments)]
    fn aristocrat_handle_char_message(
        &mut self,
        aristocrat_id: CharacterId,
        data: &mut AristocratDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, AristocratPlayerFacts>,
        events: &mut Vec<AristocratOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(aristocrat) = self.characters.get(&aristocrat_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington_forest.c:253-257`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington_forest.c:259-263`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington_forest.c:265-269`).
        if tick < data.last_talk + ARISTOCRAT_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington_forest.c:271-274`).
        if tick < data.last_talk + ARISTOCRAT_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington_forest.c:277-280`).
        if aristocrat_id == player_id
            || !char_see_char(&aristocrat, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;`
        // (`brannington_forest.c:283-286`).
        if char_dist(&aristocrat, &player) > ARISTOCRAT_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.aristocrat_state;
        match facts.aristocrat_state {
            // C `case 0:` (`brannington_forest.c:293-298`).
            0 => {
                self.npc_quiet_say(aristocrat_id, "Greetings stranger!");
                events.push(AristocratOutcomeEvent::QuestOpen { player_id });
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`brannington_forest.c:299-303`).
            1 => {
                self.npc_quiet_say(
                    aristocrat_id,
                    "Say! You look like quite a buoyant adventurer.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`brannington_forest.c:304-308`).
            2 => {
                self.npc_quiet_say(
                    aristocrat_id,
                    "Oh no, I didn't mean it that way! Please don't growl at me!",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`brannington_forest.c:309-313`).
            3 => {
                self.npc_quiet_say(
                    aristocrat_id,
                    "I was watching the local wildlife at a large lake north of here...",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`brannington_forest.c:314-318`).
            4 => {
                self.npc_quiet_say(
                    aristocrat_id,
                    "When one of the larger natives suddenly lurched out of the water and attacked me.",
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5:` (`brannington_forest.c:319-323`).
            5 => {
                self.npc_quiet_say(
                    aristocrat_id,
                    "I managed to escape with my life, but alas my Amulet was lost.",
                );
                new_state = 6;
                didsay = true;
            }
            // C `case 6:` (`brannington_forest.c:324-328`).
            6 => {
                self.npc_quiet_say(
                    aristocrat_id,
                    "I would reward you well if you could retrieve this family heirloom for me.",
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7: break;` (`brannington_forest.c:329-330`): waiting
            // for the player to hand in the amulet.
            7 => {}
            // C `case 8: break;` (`brannington_forest.c:331-332`): quest
            // chain done.
            8 => {}
            _ => {}
        }

        if new_state != facts.aristocrat_state {
            events.push(AristocratOutcomeEvent::UpdateAristocratState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington_forest.c:334-338`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `aristocrat_driver`'s `NT_TEXT` branch (`brannington_forest.c:343-
    /// 367`), wired through the generic `analyse_text_qa` matcher (same
    /// pattern as `world::npc::area26::smugglecom`'s text handler). This
    /// branch has no victim-staleness-reset preamble and no victim-mismatch
    /// early-out (see the module doc comment).
    #[allow(clippy::too_many_arguments)]
    fn aristocrat_handle_text_message(
        &mut self,
        aristocrat_id: CharacterId,
        aristocrat_name: &str,
        data: &mut AristocratDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, AristocratPlayerFacts>,
        events: &mut Vec<AristocratOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington_forest.c:346`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses
        // (`brannington_forest.c:101-126`): ignore our own talk, non-
        // players, distance > 12, not-visible.
        if aristocrat_id == speaker_id {
            return;
        }
        let Some(aristocrat) = self.characters.get(&aristocrat_id).cloned() else {
            return;
        };
        if char_dist(&aristocrat, &speaker) > ARISTOCRAT_QA_DISTANCE
            || !char_see_char(&aristocrat, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let aristocrat_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.aristocrat_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, aristocrat_name, &speaker.name, AREA28_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(aristocrat_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington_forest.c:349-354`): reset back to
            // the greeting if not yet past it.
            TextAnalysisOutcome::Matched(2) => {
                if aristocrat_state <= 7 {
                    data.last_talk = 0;
                    events.push(AristocratOutcomeEvent::UpdateAristocratState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`brannington_forest.c:355-360`): the god-only
            // "reset me" wipe, which unlike `world::npc::area26::
            // smugglecom`'s own silent `case 3` speaks a visible `say(cn,
            // "reset done")` line first (see the module doc comment).
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(aristocrat_id, "reset done");
                    events.push(AristocratOutcomeEvent::ResetAristocrat {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code is unhandled by aristocrat's own C
            // `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington_forest.c:362-365`) - note this does *not* touch
        // `dat->last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `aristocrat_driver`'s `NT_GIVE` branch (`brannington_forest.c:370-
    /// 403`).
    fn aristocrat_handle_give_message(
        &mut self,
        aristocrat_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, AristocratPlayerFacts>,
        events: &mut Vec<AristocratOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&aristocrat_id)
            .and_then(|aristocrat| aristocrat.cursor_item.take())
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

        // C `if (it[in].ID == IID_STAFF_ARIAMULET && ppd &&
        // ppd->aristocrat_state <= 7)` (`brannington_forest.c:377`).
        if item.template_id == IID_STAFF_ARIAMULET
            && is_player
            && facts.is_some_and(|facts| facts.aristocrat_state <= 7)
        {
            self.npc_quiet_say(
                aristocrat_id,
                "Yes! Many thanks adventurer! Please accept this reward.",
            );
            events.push(AristocratOutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_STAFF_ARIAMULET);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_ARIKEY);
            events.push(AristocratOutcomeEvent::UpdateAristocratState {
                player_id: giver_id,
                new_state: 8,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`brannington_forest.c:390-395`): hand
        // the item back to the giver.
        self.npc_say(
            aristocrat_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_ARISTOCRAT, CDR_LOSTCON};
use crate::item_driver::{IID_STAFF_ARIAMULET, IID_STAFF_ARIKEY};

/// C `struct aristocrat_data` (`src/area/28/brannington_forest.c:228-232`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AristocratDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
