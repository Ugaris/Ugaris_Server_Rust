//! Hermit NPC (`CDR_FORESTHERMIT`), the old man who sends players to slay
//! the spider queen (`QLOG` 24, "The Spider Queen").
//!
//! Ports `src/area/16/forest.c::hermit_driver` (`:636-815`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:83-202`, ported as
//! [`FOREST_QA`] in `world::npc::area16`, the same table `world::npc::
//! area16::william` shares). Follows the same `World`/`PlayerRuntime`
//! split established by `world::npc::area3::astro2`: the caller supplies
//! a per-player fact snapshot ([`HermitPlayerFacts`]) up front and applies
//! the returned [`HermitOutcomeEvent`]s afterwards, since
//! `area3_ppd.hermit_state` (borrowed from `src/area/3/area3.h` - C's own
//! comment: "note: the ppd is borrowed from area3 - the missions
//! interact...") lives on `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - `HermitOutcomeEvent::QuestDone`'s exp reward is applied via
//!   `QuestLog::complete_legacy`/`World::give_exp`, same precedent as
//!   every other quest-completion exp grant in this codebase - C's own
//!   `questlog_done` call carries no separate money/item reward here.
//! - No self-defense/regen/spell-self cascade exists in C's `hermit_
//!   driver` body at all (matching `world::astro1`/`world::thomas`'s
//!   identical observation for area 3's "pure talker" NPCs) - this port
//!   omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`forest.c:814`) is
//!   not ported, matching the established `world::thomas`/`world::astro2`
//!   precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_FORESTHERMIT};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::FOREST_QA;

/// C `char_dist(cn, co) > 10` (`forest.c:685`).
const HERMIT_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`forest.c:668`).
const HERMIT_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`forest.c:673`).
const HERMIT_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`forest.c:808`): idle "return to post" threshold.
const HERMIT_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `struct hermit_driver_data` (`forest.c:630-633`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ForestHermitDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Per-player facts [`World::process_forest_hermit_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForestHermitPlayerFacts {
    /// `PlayerRuntime::area3_hermit_state()`.
    pub hermit_state: i32,
}

/// A side effect [`World::process_forest_hermit_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForestHermitOutcomeEvent {
    /// Write the new `area3_ppd.hermit_state` back.
    UpdateHermitState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 24)` (`forest.c:700`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 24)` (`forest.c:738`).
    QuestDone { player_id: CharacterId },
}

impl World {
    /// C `hermit_driver`'s per-tick body (`forest.c:636-815`).
    pub fn process_forest_hermit_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ForestHermitPlayerFacts>,
        area_id: u16,
    ) -> Vec<ForestHermitOutcomeEvent> {
        let hermit_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_FORESTHERMIT
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for hermit_id in hermit_ids {
            self.process_forest_hermit_messages(hermit_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_forest_hermit_messages(
        &mut self,
        hermit_id: CharacterId,
        player_facts: &HashMap<CharacterId, ForestHermitPlayerFacts>,
        area_id: u16,
        events: &mut Vec<ForestHermitOutcomeEvent>,
    ) {
        let Some(hermit_name) = self
            .characters
            .get(&hermit_id)
            .map(|hermit| hermit.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::ForestHermit(mut data)) = self
            .characters
            .get(&hermit_id)
            .and_then(|hermit| hermit.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&hermit_id)
            .map(|hermit| std::mem::take(&mut hermit.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.forest_hermit_handle_char_message(
                    hermit_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.forest_hermit_handle_text_message(
                    hermit_id,
                    &hermit_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.forest_hermit_handle_give_message(hermit_id, message),
                _ => {}
            }
        }

        if let Some(hermit) = self.characters.get_mut(&hermit_id) {
            hermit.driver_state = Some(CharacterDriverState::ForestHermit(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`forest.c:804-806`).
        if let (Some(hermit), Some((tx, ty))) =
            (self.characters.get(&hermit_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(hermit.x), i32::from(hermit.y), tx, ty) {
                if let Some(hermit_mut) = self.characters.get_mut(&hermit_id) {
                    let _ = turn(hermit_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`forest.c:808-812`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other area-3-family driver uses.
        let last_talk = match self
            .characters
            .get(&hermit_id)
            .and_then(|hermit| hermit.driver_state.as_ref())
        {
            Some(CharacterDriverState::ForestHermit(data)) => data.last_talk,
            _ => return,
        };
        if last_talk + HERMIT_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(hermit) = self.characters.get(&hermit_id) else {
                return;
            };
            let (post_x, post_y) = (hermit.rest_x, hermit.rest_y);
            self.secure_move_driver(
                hermit_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `hermit_driver`'s `NT_CHAR` branch (`forest.c:652-757`).
    fn forest_hermit_handle_char_message(
        &mut self,
        hermit_id: CharacterId,
        data: &mut ForestHermitDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestHermitPlayerFacts>,
        events: &mut Vec<ForestHermitOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(hermit) = self.characters.get(&hermit_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`forest.c:656-659`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`forest.c:661-664`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`forest.c:667-670`).
        if tick < data.last_talk + HERMIT_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`forest.c:672-676`).
        if tick < data.last_talk + HERMIT_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`forest.c:678-682`).
        if hermit_id == player_id || !char_see_char(&hermit, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`forest.c:684-688`).
        if char_dist(&hermit, &player) > HERMIT_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->hermit_state) { ... }` (`forest.c:694-750`).
        match facts.hermit_state {
            0 => {
                self.npc_quiet_say(
                    hermit_id,
                    &format!(
                        "My greetings to thee, {}. 'Tis most fortunate to see such a formidable hero as thyself. Be aware that I am in dire need of thine help.",
                        player.name
                    ),
                );
                events.push(ForestHermitOutcomeEvent::QuestOpen { player_id });
                events.push(ForestHermitOutcomeEvent::UpdateHermitState {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
            }
            1 => {
                self.npc_quiet_say(
                    hermit_id,
                    "Not long ago, some foul demons invaded this once so peaceful forest. They did not linger for long, but after they left the spiders in the western part of the forest started to grow and grow and grow.",
                );
                events.push(ForestHermitOutcomeEvent::UpdateHermitState {
                    player_id,
                    new_state: 2,
                });
                didsay = true;
            }
            2 => {
                self.npc_quiet_say(
                    hermit_id,
                    &format!(
                        "They did not only grow in size, but also in aggressiveness. Before, they used to feed on other insects, but now they lust human blood. Therefore I lay this quest upon thee, {}, to go to their lair and slay their queen.",
                        player.name
                    ),
                );
                events.push(ForestHermitOutcomeEvent::UpdateHermitState {
                    player_id,
                    new_state: 3,
                });
                didsay = true;
            }
            3 => {
                self.npc_quiet_say(
                    hermit_id,
                    &format!(
                        "Be wary, and prepare thyself well, for the queen can only be slain by a holy weapon of sufficient strength. Now go, {}, and do what needs be done. Thou canst reach their lair by going south and turning north-west at the old ruin.",
                        player.name
                    ),
                );
                events.push(ForestHermitOutcomeEvent::UpdateHermitState {
                    player_id,
                    new_state: 4,
                });
                didsay = true;
            }
            // `hermit_state == 4`: waiting for the spider queen kill.
            5 => {
                self.npc_quiet_say(
                    hermit_id,
                    &format!(
                        "I thank thee, {}, for thy brave deed. Forever shall I keep the memory of thy courage in my heart.",
                        player.name
                    ),
                );
                events.push(ForestHermitOutcomeEvent::UpdateHermitState {
                    player_id,
                    new_state: 6,
                });
                events.push(ForestHermitOutcomeEvent::QuestDone { player_id });
                didsay = true;
            }
            6 => {
                self.npc_quiet_say(
                    hermit_id,
                    &format!(
                        "I know not why these demons have come, nor whence they came from. But I ask thee, {}, fight them whereever they show their ugly hides.",
                        player.name
                    ),
                );
                events.push(ForestHermitOutcomeEvent::UpdateHermitState {
                    player_id,
                    new_state: 7,
                });
                didsay = true;
            }
            // `hermit_state == 4` or `7` (or any other value): no-op,
            // matching C's empty `case 4:`/`case 7: break;`.
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`forest.c:751-755`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `hermit_driver`'s `NT_TEXT` branch (`forest.c:760-785`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::npc::area3::astro2`'s text handler).
    fn forest_hermit_handle_text_message(
        &mut self,
        hermit_id: CharacterId,
        hermit_name: &str,
        data: &mut ForestHermitDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestHermitPlayerFacts>,
        events: &mut Vec<ForestHermitOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`forest.c:763-765`).
        let tick = self.tick.0;
        if tick > data.last_talk + HERMIT_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`forest.c:767-770`).
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

        // C `analyse_text_driver`'s own guard clauses (`forest.c:112-124`):
        // ignore our own talk, non-players/player-likes, not-visible (no
        // active distance check - the `char_dist(cn,co)>16` guard is
        // commented out in C, `forest.c:125`).
        if hermit_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(hermit) = self.characters.get(&hermit_id).cloned() else {
            return;
        };
        if !char_see_char(&hermit, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let hermit_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.hermit_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, hermit_name, &speaker.name, FOREST_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(hermit_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`forest.c:772-779`): only
            // `hermit_state <= 4` resets to `0` - states 5-7 are
            // untouched, matching C's missing `else` branch.
            TextAnalysisOutcome::Matched(2) => {
                if hermit_state <= 4 {
                    data.last_talk = 0;
                    events.push(ForestHermitOutcomeEvent::UpdateHermitState {
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
        // (`forest.c:781-784`) - note this does *not* touch `dat->
        // last_talk` (except the explicit reset inside the `case 2`
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `hermit_driver`'s `NT_GIVE` branch (`forest.c:787-796`): the
    /// hermit has no turn-in item at all - any offered item is silently
    /// destroyed (unlike `world::npc::area3::astro2`, which gives
    /// unrecognized items back).
    fn forest_hermit_handle_give_message(
        &mut self,
        hermit_id: CharacterId,
        _message: &CharacterDriverMessage,
    ) {
        let Some(item_id) = self
            .characters
            .get_mut(&hermit_id)
            .and_then(|hermit| hermit.cursor_item.take())
        else {
            return;
        };
        self.destroy_item(item_id);
    }
}
