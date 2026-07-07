//! Ambient lore NPC (`CDR_BRITHILDIE`), area 1's Governor's-mother
//! storyteller who unlocks the `QLOG_BRITHILDIE` bear-kill quest.
//!
//! Ports `src/area/1/gwendylon.c::brithildie_driver` (`:2474-2823`) plus
//! its shared file-local `analyse_text_driver`/`qa` table (`:98-224`,
//! already ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`/`world::terion`/`world::jessica`/
//! `world::jiu`/`world::forest_ranger`). Follows the same `World`/
//! `PlayerRuntime` split established there: the caller supplies a
//! per-player fact snapshot ([`BrithildiePlayerFacts`]) up front and
//! applies the returned [`BrithildieOutcomeEvent`]s afterwards, since
//! `brithildie_state`/`brithildie_seen_timer` (`area1_ppd` fields) live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - `BRITHILDIE_STATE_WAIT_STORY_3` (`gwendylon.c:2643-2652`) advances
//!   straight to `BRITHILDIE_STATE_STORY_4_1` on `ch[co].level >= 43`
//!   (`:2645`), *skipping* the `STORY_3_1`/`STORY_3_2`/`STORY_3_3` cases
//!   entirely - no transition anywhere in `brithildie_driver` (nor the
//!   "repeat"/"repeat all" `NT_TEXT` handlers) ever assigns those three
//!   states. They are dead code in the C source itself (their `case`
//!   labels are unreachable), preserved here exactly as written rather
//!   than "fixed" to chain through them - see [`BRITHILDIE_STATE_STORY_3_1`]/
//!   [`BRITHILDIE_STATE_STORY_3_2`]/[`BRITHILDIE_STATE_STORY_3_3`].
//! - The two consecutive `NT_CHAR` throttle checks (`gwendylon.c:2506-
//!   2514`) are both `ticker < dat->last_talk + TICKS * 10`; the second
//!   additionally requires `dat->current_victim && dat->current_victim !=
//!   co`, but since the first (unconditional) check already returns
//!   whenever that same inequality holds, the second can never be reached
//!   with a passing first check - the same dead-code shape already
//!   documented on `world::forest_ranger`'s module doc comment (apparently
//!   copy-pasted from `camhermit_driver`'s two-window pattern and never
//!   adjusted). Only the single always-reachable `TICKS * 10` gate is
//!   ported; see [`BRITHILDIE_TALK_MIN_TICKS`].
//! - `ppd->brithildie_seen_timer = realtime;` (`gwendylon.c:2725`) is
//!   unconditional (every processed `NT_CHAR`, `didsay` or not), matching
//!   `world::forest_ranger`'s/`world::camhermit`'s own unconditional
//!   `*_seen_timer` writes.
//! - `bigbadspider_dead` (`gwendylon.c:2850-2870`), the death hook that
//!   advances `BRITHILDIE_STATE_NOMORETALES_QOPEN` to `_QDONE` via
//!   `questlog_done`, is ported separately as
//!   `ugaris-server::world_events::apply_bigbadspider_death_from_hurt_event`
//!   (same split as `world::jiu`'s `riverbeast_dead` gap) - not in this
//!   file, since it fires from a different NPC's (`CDR_BIGBADSPIDER`)
//!   death, not from `brithildie_driver` itself.
//! - The `NOMORETALES_QOPEN`/`_QDONE` reminder line wraps "Repeat all" in
//!   `COL_LIGHT_BLUE`/`COL_RESET` markers (`gwendylon.c:2716-2717`);
//!   restored via `COL_STR_LIGHT_BLUE`/`COL_STR_RESET` sentinels and
//!   `World::npc_quiet_say_bytes`, same mechanism as `world::camhermit`.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_BRITHILDIE, GWENDYLON_QA};
use crate::drvlib::offset2dx;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;

/// C `char_dist(cn, co) > 15` (`gwendylon.c:2523`): the `NT_CHAR` greeting
/// range.
const BRITHILDIE_GREET_DISTANCE: i32 = 15;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const BRITHILDIE_QA_DISTANCE: i32 = 12;
/// C `TICKS * 10` (`gwendylon.c:2506`, `:2738`): the `NT_CHAR` greeting
/// throttle and the `NT_TEXT` `current_victim` reset window. See the
/// module doc comment for why only one `TICKS * 10` threshold - not two -
/// actually governs `NT_CHAR`.
const BRITHILDIE_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:2816`): idle "return to post" threshold.
const BRITHILDIE_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `#define BRITHILDIE_EXTEND_WAIT_TIME 60` (`gwendylon.c:2472`): the
/// shared reminder-line gate for every "waiting" state.
const BRITHILDIE_EXTEND_WAIT_TIME: i32 = 60;

/// C's bare `int` state values for `ppd->brithildie_state`
/// (`src/common/npc_states.h:51-72`).
const BRITHILDIE_STATE_ENTRY: i32 = 0;
const BRITHILDIE_STATE_WAIT_STORY_1: i32 = 1;
const BRITHILDIE_STATE_STORY_1_1: i32 = 2;
const BRITHILDIE_STATE_STORY_1_2: i32 = 3;
const BRITHILDIE_STATE_STORY_1_3: i32 = 4;
const BRITHILDIE_STATE_STORY_1_4: i32 = 5;
const BRITHILDIE_STATE_STORY_1_5: i32 = 6;
const BRITHILDIE_STATE_WAIT_STORY_2: i32 = 7;
const BRITHILDIE_STATE_STORY_2_1: i32 = 8;
const BRITHILDIE_STATE_STORY_2_2: i32 = 9;
const BRITHILDIE_STATE_STORY_2_3: i32 = 10;
const BRITHILDIE_STATE_STORY_2_4: i32 = 11;
const BRITHILDIE_STATE_WAIT_STORY_3: i32 = 12;
/// Unreachable in C - see the module doc comment.
const BRITHILDIE_STATE_STORY_3_1: i32 = 13;
/// Unreachable in C - see the module doc comment.
const BRITHILDIE_STATE_STORY_3_2: i32 = 14;
/// Unreachable in C - see the module doc comment.
const BRITHILDIE_STATE_STORY_3_3: i32 = 15;
const BRITHILDIE_STATE_WAIT_STORY_4: i32 = 16;
const BRITHILDIE_STATE_STORY_4_1: i32 = 17;
const BRITHILDIE_STATE_STORY_4_2: i32 = 18;
const BRITHILDIE_STATE_STORY_4_3: i32 = 19;
const BRITHILDIE_STATE_NOMORETALES_QOPEN: i32 = 20;
const BRITHILDIE_STATE_NOMORETALES_QDONE: i32 = 21;

/// Per-player facts [`World::process_brithildie_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrithildiePlayerFacts {
    /// `PlayerRuntime::area1_brithildie_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_brithildie_seen_timer()` (C `realtime`
    /// wall-clock seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
}

/// A side effect [`World::process_brithildie_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrithildieOutcomeEvent {
    /// Write the new `area1_ppd.brithildie_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->brithildie_seen_timer = realtime;` after
    /// every processed `NT_CHAR` message (`gwendylon.c:2725`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, QLOG_BRITHILDIE)` (`gwendylon.c:2709`).
    QuestOpen { player_id: CharacterId },
}

impl World {
    /// C `brithildie_driver`'s per-tick body (`gwendylon.c:2474-2823`).
    /// `now` is C's wall-clock `realtime` (seconds).
    pub fn process_brithildie_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, BrithildiePlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<BrithildieOutcomeEvent> {
        let brithildie_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_BRITHILDIE
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for brithildie_id in brithildie_ids {
            self.process_brithildie_messages(
                brithildie_id,
                player_facts,
                now,
                area_id,
                &mut events,
            );
        }
        events
    }

    fn process_brithildie_messages(
        &mut self,
        brithildie_id: CharacterId,
        player_facts: &HashMap<CharacterId, BrithildiePlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<BrithildieOutcomeEvent>,
    ) {
        let Some(brithildie_name) = self
            .characters
            .get(&brithildie_id)
            .map(|brithildie| brithildie.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Brithildie(mut data)) = self
            .characters
            .get(&brithildie_id)
            .and_then(|brithildie| brithildie.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&brithildie_id)
            .map(|brithildie| std::mem::take(&mut brithildie.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.brithildie_handle_char_message(
                    brithildie_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.brithildie_handle_text_message(
                    brithildie_id,
                    &brithildie_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.brithildie_handle_give_message(brithildie_id, message),
                _ => {}
            }
        }

        if let Some(brithildie) = self.characters.get_mut(&brithildie_id) {
            brithildie.driver_state = Some(CharacterDriverState::Brithildie(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:2812-2814`).
        if let (Some(brithildie), Some((tx, ty))) =
            (self.characters.get(&brithildie_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(brithildie.x), i32::from(brithildie.y), tx, ty)
            {
                if let Some(brithildie_mut) = self.characters.get_mut(&brithildie_id) {
                    let _ = turn(brithildie_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHTDOWN,
        // ret, lastact)) return; } do_idle(cn, TICKS);` (`gwendylon.c:2816-
        // 2822`). The NPC's post position (C's `tmpx`/`tmpy`) reuses
        // `rest_x`/`rest_y`, the same substitution `world::forest_ranger`
        // already uses for other stationary NPCs' spawn tiles.
        let last_talk = if let Some(brithildie) = self.characters.get(&brithildie_id) {
            match brithildie.driver_state.as_ref() {
                Some(CharacterDriverState::Brithildie(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + BRITHILDIE_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(brithildie) = self.characters.get(&brithildie_id) else {
                return;
            };
            let (post_x, post_y) = (brithildie.rest_x, brithildie.rest_y);
            self.secure_move_driver(
                brithildie_id,
                post_x,
                post_y,
                Direction::RightDown as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `brithildie_driver`'s `NT_CHAR` branch (`gwendylon.c:2489-2732`).
    #[allow(clippy::too_many_arguments)]
    fn brithildie_handle_char_message(
        &mut self,
        brithildie_id: CharacterId,
        data: &mut BrithildieDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, BrithildiePlayerFacts>,
        now: i32,
        events: &mut Vec<BrithildieOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(brithildie) = self.characters.get(&brithildie_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:2493-2497`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:2499-2503`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*10) continue;`
        // (`gwendylon.c:2506-2509`) - see the module doc comment for why
        // the C source's second, `current_victim`-gated check
        // (`gwendylon.c:2511-2514`) is unreachable dead code and not
        // ported.
        if tick < data.last_talk + BRITHILDIE_TALK_MIN_TICKS {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:2517-2520`).
        if brithildie_id == player_id
            || !char_see_char(&brithildie, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 15) continue;` (`gwendylon.c:2522-
        // 2526`).
        if char_dist(&brithildie, &player) > BRITHILDIE_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.state;
        let level = player.level as i32;

        match facts.state {
            BRITHILDIE_STATE_ENTRY => {
                // C `case BRITHILDIE_STATE_ENTRY:` (`gwendylon.c:2534-
                // 2549`).
                self.npc_quiet_say(
                    brithildie_id,
                    "Hello traveller! Thou must have met mine eldest son Walter, the Governor of this town. Have thee also seen my second son Seymour? He has joined the imperial army. Oh, I am so proud of my boys, how much they have accomplished!",
                );
                didsay = true;
                new_state = if level < 9 {
                    BRITHILDIE_STATE_WAIT_STORY_1
                } else if level < 20 {
                    BRITHILDIE_STATE_STORY_1_1
                } else if level < 37 {
                    BRITHILDIE_STATE_WAIT_STORY_2
                } else {
                    BRITHILDIE_STATE_STORY_2_1
                };
            }
            BRITHILDIE_STATE_WAIT_STORY_1 => {
                // C `case BRITHILDIE_STATE_WAIT_STORY_1:` (`gwendylon.c:
                // 2551-2563`).
                if (9..20).contains(&level) {
                    new_state = BRITHILDIE_STATE_STORY_1_1;
                } else if level >= 20 {
                    // Argh shouldnt be here (C comment, `gwendylon.c:
                    // 2556`).
                    new_state = BRITHILDIE_STATE_WAIT_STORY_2;
                } else if now.saturating_sub(facts.seen_timer) > BRITHILDIE_EXTEND_WAIT_TIME {
                    self.npc_quiet_say(
                        brithildie_id,
                        "Come back another day, I may have a story for you.",
                    );
                    didsay = true;
                }
            }
            BRITHILDIE_STATE_STORY_1_1 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "The house next door once belonged to a hunter. Just as proud as Yoakin and his brother are. One day he found a piece of gold on a baby bear he had slain.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_1_2;
            }
            BRITHILDIE_STATE_STORY_1_2 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "It was stuck between the cub's front teeth. The coin was old, and for reasons unknown the hunter was certain it had come from a hidden treasure.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_1_3;
            }
            BRITHILDIE_STATE_STORY_1_3 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "From that day, the man spent a good ten years of his life in search of the gold. It became an obsession. And whilst gone for months at the time on each trip, he missed the birth of his first child, and his second.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_1_4;
            }
            BRITHILDIE_STATE_STORY_1_4 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "Filled with grief his wife passed away and the children he never learned to know had to fend for themselves.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_1_5;
            }
            BRITHILDIE_STATE_STORY_1_5 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "Rumor has it he eventually found the treasure, but soon realized he gained no happiness from it. He now built a cabin, secluded in the forest. And there he lives now with anger and regret as his only companions.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_WAIT_STORY_2;
            }
            BRITHILDIE_STATE_WAIT_STORY_2 => {
                // C `case BRITHILDIE_STATE_WAIT_STORY_2:` (`gwendylon.c:
                // 2602-2611`).
                if level >= 39 {
                    new_state = BRITHILDIE_STATE_STORY_2_1;
                } else if now.saturating_sub(facts.seen_timer) > BRITHILDIE_EXTEND_WAIT_TIME {
                    self.npc_quiet_say(
                        brithildie_id,
                        "Come back another day, I may have a story for you.",
                    );
                    didsay = true;
                }
            }
            BRITHILDIE_STATE_STORY_2_1 => {
                self.npc_quiet_say(brithildie_id, "Welcome back to my humble home, friend.");
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_2_2;
            }
            BRITHILDIE_STATE_STORY_2_2 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "People shun the northern side of the river. If there is any crossing there, it is long forgotten by the folks of this town. It is said that one day long ago, a man came from that very side of the river. His robes were strange and his head was bald.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_2_3;
            }
            BRITHILDIE_STATE_STORY_2_3 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "This man claimed he had not eaten solid food for years, only meditated and survived on rainwater. How unlikely that may seem, he had a special aura about him. He taught the people of Cameron the true meaning of peace.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_2_4;
            }
            BRITHILDIE_STATE_STORY_2_4 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "This was long before the mansion became a ruin, and he is nearly forgotten. But in the atmosphere of our town his teachings live on.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_WAIT_STORY_3;
            }
            BRITHILDIE_STATE_WAIT_STORY_3 => {
                // C `case BRITHILDIE_STATE_WAIT_STORY_3:` (`gwendylon.c:
                // 2643-2652`) - jumps straight to `STORY_4_1`, skipping
                // `STORY_3_1..3_3` entirely. See the module doc comment.
                if level >= 43 {
                    new_state = BRITHILDIE_STATE_STORY_4_1;
                } else if now.saturating_sub(facts.seen_timer) > BRITHILDIE_EXTEND_WAIT_TIME {
                    self.npc_quiet_say(
                        brithildie_id,
                        "Come back another day, I may have a story for you.",
                    );
                    didsay = true;
                }
            }
            BRITHILDIE_STATE_STORY_3_1 => {
                self.npc_quiet_say(
                    brithildie_id,
                    &format!("I am pleased to see thee again {}.", player.name),
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_3_2;
            }
            BRITHILDIE_STATE_STORY_3_2 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "This town used to be ruled by a city council of our most respected citizens. Not by a Governor as today. The last council ended over a century ago under strange circumstances. They simply disappeared, and along with them, most knowledge of their work as well.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_3_3;
            }
            BRITHILDIE_STATE_STORY_3_3 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "Reskin seems to think that his tenancy occupation is a well-kept secret. The reality is, we allow it. Cannot very well have our only barkeep banished from town. What he does not know is how old the structure is, it dates back to the council times. I believe the room is still there. Seemingly kept intact by some spell.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_WAIT_STORY_4;
            }
            BRITHILDIE_STATE_WAIT_STORY_4 => {
                // C `case BRITHILDIE_STATE_WAIT_STORY_4:` (`gwendylon.c:
                // 2678-2687`).
                if level >= 45 {
                    new_state = BRITHILDIE_STATE_STORY_4_1;
                } else if now.saturating_sub(facts.seen_timer) > BRITHILDIE_EXTEND_WAIT_TIME {
                    self.npc_quiet_say(
                        brithildie_id,
                        "Come back another day, I may have a story for you.",
                    );
                    didsay = true;
                }
            }
            BRITHILDIE_STATE_STORY_4_1 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "I have just hear such a story! You certainly will like it!",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_4_2;
            }
            BRITHILDIE_STATE_STORY_4_2 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "Thus far no one has known why the western part of our forest appears to be dying. The decay has long been blamed on magic. Last night, at the tavern, a stranger spoke of a monster that had ventured there. There description was vague at best. Something crawling, evil, and with green legs.",
                );
                didsay = true;
                new_state = BRITHILDIE_STATE_STORY_4_3;
            }
            BRITHILDIE_STATE_STORY_4_3 => {
                self.npc_quiet_say(
                    brithildie_id,
                    "This was a fortnight ago, ever since people all across town have suffered from nightmares. The forest has acquired an eerie gloom to it, there must be something unholy at work there now.",
                );
                didsay = true;
                events.push(BrithildieOutcomeEvent::QuestOpen { player_id });
                new_state = BRITHILDIE_STATE_NOMORETALES_QOPEN;
            }
            BRITHILDIE_STATE_NOMORETALES_QOPEN | BRITHILDIE_STATE_NOMORETALES_QDONE => {
                // C `case BRITHILDIE_STATE_NOMORETALES_QOPEN: case
                // BRITHILDIE_STATE_NOMORETALES_QDONE:` (`gwendylon.c:2713-
                // 2722`).
                if now.saturating_sub(facts.seen_timer) > BRITHILDIE_EXTEND_WAIT_TIME {
                    self.npc_quiet_say_bytes(
                        brithildie_id,
                        &format!(
                            "Hail thee {}! I have no more tales to tell. If you wish me to repeat all my tales say {COL_STR_LIGHT_BLUE}Repeat all{COL_STR_RESET}.",
                            player.name
                        ),
                    );
                    didsay = true;
                }
            }
            // Every other value: no-op, matching C's `switch` with no
            // matching `case`.
            _ => {}
        }

        if new_state != facts.state {
            events.push(BrithildieOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }
        // C `ppd->brithildie_seen_timer = realtime;` (`gwendylon.c:2725`):
        // unconditional, regardless of `didsay`.
        events.push(BrithildieOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:2727-2731`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `brithildie_driver`'s `NT_TEXT` branch (`gwendylon.c:2735-2790`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::forest_ranger`'s/`world::yoakin`'s text handlers).
    fn brithildie_handle_text_message(
        &mut self,
        brithildie_id: CharacterId,
        brithildie_name: &str,
        data: &mut BrithildieDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, BrithildiePlayerFacts>,
        events: &mut Vec<BrithildieOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:2738-2740`).
        let tick = self.tick.0;
        if tick > data.last_talk + BRITHILDIE_TALK_MIN_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:2742-2745`).
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
        if brithildie_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(brithildie) = self.characters.get(&brithildie_id).cloned() else {
            return;
        };
        if char_dist(&brithildie, &speaker) > BRITHILDIE_QA_DISTANCE
            || !char_see_char(&brithildie, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, brithildie_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(brithildie_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:2748-2773`): four disjoint `if`s,
            // each resetting `brithildie_state` back to a checkpoint and
            // zeroing `last_talk` - at most one applies since the ranges
            // don't overlap. The `WAIT_STORY_4` branch is commented out in
            // C itself ("Quest not yet implemented", `gwendylon.c:2764-
            // 2768`) and is not ported.
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    let reset_state = if facts.state == BRITHILDIE_STATE_WAIT_STORY_1 {
                        Some(BRITHILDIE_STATE_ENTRY)
                    } else if facts.state == BRITHILDIE_STATE_WAIT_STORY_2 {
                        Some(BRITHILDIE_STATE_STORY_1_1)
                    } else if facts.state == BRITHILDIE_STATE_WAIT_STORY_3 {
                        Some(BRITHILDIE_STATE_STORY_2_1)
                    } else if facts.state == BRITHILDIE_STATE_NOMORETALES_QOPEN {
                        Some(BRITHILDIE_STATE_STORY_4_1)
                    } else {
                        None
                    };
                    if let Some(new_state) = reset_state {
                        data.last_talk = 0;
                        events.push(BrithildieOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state,
                        });
                    }
                }
                didsay = true;
            }
            // C `case 13:` (`gwendylon.c:2776-2784`): "repeat all" -
            // rewinds only from `NOMORETALES_QOPEN` back to
            // `WAIT_STORY_2`.
            TextAnalysisOutcome::Matched(13) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    if facts.state == BRITHILDIE_STATE_NOMORETALES_QOPEN {
                        data.last_talk = 0;
                        events.push(BrithildieOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: BRITHILDIE_STATE_WAIT_STORY_2,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by brithildie's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:2786-2789`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2`/`case 13` branches
        // above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `brithildie_driver`'s `NT_GIVE` branch (`gwendylon.c:2793-2804`).
    /// Unlike `world::terion`/`world::forest_ranger`'s give-back handlers,
    /// C's own `brithildie_driver` calls plain `give_char_item`, not
    /// `give_char_item_smart` - no ground-drop fallback on a full
    /// inventory, ported here via `World::give_char_item` rather than
    /// "fixed" to match its siblings (same documented asymmetry as
    /// `world::jessica`'s `NT_GIVE` handler).
    fn brithildie_handle_give_message(
        &mut self,
        brithildie_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&brithildie_id)
            .and_then(|brithildie| brithildie.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            brithildie_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct brithildie_driver_data` (`src/area/1/gwendylon.c:2467-2470`):
/// the ambient lore NPC's own driver memory (`CDR_BRITHILDIE`, distinct
/// from the per-player `brithildie_state`/`brithildie_seen_timer` fields
/// in `crate::player::PlayerRuntime`'s `area1_ppd` - see
/// `world::brithildie`'s module doc comment for the split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BrithildieDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
