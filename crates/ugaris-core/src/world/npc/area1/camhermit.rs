//! Camp hermit NPC (`CDR_CAMHERMIT`), area 1's forest quest giver.
//!
//! Ports `src/area/1/gwendylon.c::camhermit_driver` (`:707-996`). Two
//! pieces of per-player state the dialogue needs -
//! `PlayerRuntime::area1_camhermit_state`/`area1_camhermit_seen_timer`/
//! `area1_camhermit_kills` (`area1_ppd`) and the player's
//! `quest_log.count(QLOG_HERMIT_QUEST2)` - live in `crate::player::
//! PlayerRuntime`, owned by `ugaris-server` (`ServerRuntime::players`), not
//! `World`. Following the same split already established for
//! `world::gatekeeper`'s `GateWelcomePlayerFacts`/`GateWelcomeOutcomeEvent`,
//! the caller supplies a per-player fact snapshot ([`CamhermitPlayerFacts`])
//! up front and applies the returned [`CamhermitOutcomeEvent`]s afterwards.
//!
//! Deviations/gaps (documented, not silent):
//! - `ppd->camhermit_kills` (`monster_dead`, `gwendylon.c:5200-5219`, the
//!   generic area-1 monster death hook that increments it whenever a
//!   player kills anything while `camhermit_state == CAMHERMIT_STATE_
//!   QUEST1DO`) is read here via [`CamhermitPlayerFacts::kills`] but never
//!   written by this module - `monster_dead` itself is a separate, still-
//!   unported piece of `gwendylon.c` (shared by every area-1 monster's
//!   death dispatch, not camhermit-specific). Until that lands, `kills`
//!   stays at whatever `PlayerRuntime::area1_camhermit_kills` already
//!   holds (`0` for every player today), so the `CAMHERMIT_STATE_QUEST1DO`
//!   branch will only ever take its "not enough kills yet" reminder path.
//! - The `CAMHERMIT_STATE_QUEST1DO`/`QUEST2DO` reminder line's C source
//!   wraps the word "repeat" in `COL_LIGHT_BLUE`/`COL_RESET` markers
//!   (`gwendylon.c:806-808,895-897`). `World::npc_quiet_say` broadcasts a
//!   plain UTF-8 `String` (`WorldAreaText`), which cannot represent the
//!   raw non-UTF8 color-marker byte, so the styling is dropped here (same
//!   simplification already documented on `BANK_QA`'s "account"/"explain
//!   deposit"/etc. entries) - the wording is otherwise byte-for-byte
//!   identical.
//! - `give_char_item_smart`'s `IF_MONEY` branch achievement tracking
//!   (`achievement_add_gold_earned`) needs `PlayerRuntime`/DB access
//!   `World` doesn't have - see [`CamhermitOutcomeEvent::GoldEarned`] and
//!   `World::give_char_item_smart`'s own doc comment for the established
//!   split (same shape as `World::complete_mission`'s mercenary bonus).

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_CAMHERMIT, GWENDYLON_QA};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_AREA1_SMALL_BEAR_TEETH;
use crate::quest::{
    CAMHERMIT_STATE_DONE, CAMHERMIT_STATE_ENTRY, CAMHERMIT_STATE_QUEST1DO,
    CAMHERMIT_STATE_QUEST1WAIT, CAMHERMIT_STATE_QUEST1_1, CAMHERMIT_STATE_QUEST1_2,
    CAMHERMIT_STATE_QUEST1_3, CAMHERMIT_STATE_QUEST2DO, CAMHERMIT_STATE_QUEST2DO_WAIT,
    CAMHERMIT_STATE_QUEST2WAIT, CAMHERMIT_STATE_QUEST2_1, CAMHERMIT_STATE_QUEST2_2,
    CAMHERMIT_STATE_QUEST2_3, CAMHERMIT_STATE_QUEST2_4, CAMHERMIT_STATE_QUEST2_REOPEN,
    QLOG_HERMIT_QUEST1, QLOG_HERMIT_QUEST2,
};
use crate::world::*;

/// C `char_dist(cn, co) > 10` (`gwendylon.c:751`): the `NT_CHAR` greeting
/// range.
const CAMHERMIT_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the local
/// `analyse_text_driver`'s own guard): the small-talk range.
const CAMHERMIT_QA_DISTANCE: i32 = 12;
/// C `TICKS * 10` (`gwendylon.c:737`).
const CAMHERMIT_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 20` (`gwendylon.c:741`).
const CAMHERMIT_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 20;
/// C `TICKS * 30` (`gwendylon.c:989`): idle "return to post" threshold.
const CAMHERMIT_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `realtime - ppd->camhermit_seen_timer > 60` (`gwendylon.c:804,891`).
const CAMHERMIT_SEEN_REMINDER_SECONDS: i32 = 60;
/// C `#define CAMHERMIT_QUEST1_KILLSNEEDED 10` (`gwendylon.c:677`).
const CAMHERMIT_QUEST1_KILLSNEEDED: i32 = 10;
/// C `#define CAMHERMIT_QUEST2_TEETHNEEDED 10` (`gwendylon.c:678`).
const CAMHERMIT_QUEST2_TEETHNEEDED: usize = 10;
/// C `#define CAMHERMIT_QUEST2_GOLD_PER_NEEDED_STACK 15` (`gwendylon.c:679`,
/// "in Gold!" - the `* 100` silver conversion happens at the call site).
const CAMHERMIT_QUEST2_GOLD_PER_NEEDED_STACK: u32 = 15;

/// Per-player facts [`World::process_camhermit_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CamhermitPlayerFacts {
    /// `PlayerRuntime::area1_camhermit_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_camhermit_seen_timer()` (C `realtime` wall-clock
    /// seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
    /// `PlayerRuntime::area1_camhermit_kills()`. See the module doc comment
    /// for why this never actually advances yet.
    pub kills: i32,
    /// `PlayerRuntime::quest_log.count(QLOG_HERMIT_QUEST2)`.
    pub quest2_done_count: u8,
}

/// A side effect [`World::process_camhermit_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamhermitOutcomeEvent {
    /// Write the new `area1_ppd.camhermit_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->camhermit_seen_timer = realtime;` after
    /// every processed `NT_CHAR` message (`gwendylon.c:918`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, ...)`.
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `questlog_done(co, ...)` - the caller must apply
    /// `PlayerRuntime::quest_log.complete_legacy` (exp reward + resend).
    QuestDone {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `questlog_reopen(co, ...)`.
    QuestReopen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `give_money`'s wealth-achievement half - see the module doc
    /// comment's last bullet.
    GoldEarned { player_id: CharacterId, amount: u32 },
}

impl World {
    /// C `camhermit_driver`'s per-tick body (`gwendylon.c:707-996`).
    /// `now` is C's wall-clock `realtime` (seconds).
    pub fn process_camhermit_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CamhermitPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<CamhermitOutcomeEvent> {
        let camhermit_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_CAMHERMIT
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for camhermit_id in camhermit_ids {
            self.process_camhermit_messages(camhermit_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_camhermit_messages(
        &mut self,
        camhermit_id: CharacterId,
        player_facts: &HashMap<CharacterId, CamhermitPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<CamhermitOutcomeEvent>,
    ) {
        let Some(camhermit_name) = self
            .characters
            .get(&camhermit_id)
            .map(|camhermit| camhermit.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Camhermit(mut data)) = self
            .characters
            .get(&camhermit_id)
            .and_then(|camhermit| camhermit.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&camhermit_id)
            .map(|camhermit| std::mem::take(&mut camhermit.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.camhermit_handle_char_message(
                    camhermit_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.camhermit_handle_text_message(
                    camhermit_id,
                    &camhermit_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.camhermit_handle_give_message(camhermit_id, message),
                _ => {}
            }
        }

        if let Some(camhermit) = self.characters.get_mut(&camhermit_id) {
            camhermit.driver_state = Some(CharacterDriverState::Camhermit(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:987`).
        if let (Some(camhermit), Some((tx, ty))) =
            (self.characters.get(&camhermit_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(camhermit.x), i32::from(camhermit.y), tx, ty)
            {
                if let Some(camhermit_mut) = self.characters.get_mut(&camhermit_id) {
                    let _ = turn(camhermit_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; } do_idle(cn, TICKS);` (`gwendylon.c:989-994`).
        // The NPC's post position (C's `tmpx`/`tmpy`) reuses `rest_x`/
        // `rest_y`, the same substitution `world::gatekeeper`/`world::bank`
        // already use for other stationary NPCs' spawn tiles.
        let last_talk = if let Some(camhermit) = self.characters.get(&camhermit_id) {
            match camhermit.driver_state.as_ref() {
                Some(CharacterDriverState::Camhermit(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + CAMHERMIT_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(camhermit) = self.characters.get(&camhermit_id) else {
                return;
            };
            let (post_x, post_y) = (camhermit.rest_x, camhermit.rest_y);
            self.secure_move_driver(
                camhermit_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `camhermit_driver`'s `NT_CHAR` branch (`gwendylon.c:713-921`).
    fn camhermit_handle_char_message(
        &mut self,
        camhermit_id: CharacterId,
        data: &mut CamhermitDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CamhermitPlayerFacts>,
        now: i32,
        events: &mut Vec<CamhermitOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(camhermit) = self.characters.get(&camhermit_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:719-722`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:724-727`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*10) continue;`
        // (`gwendylon.c:730-733`).
        if tick < data.last_talk + CAMHERMIT_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*20 && dat->current_victim
        // != co) continue;` (`gwendylon.c:735-738`) - a plain `!=`, unlike
        // `world::gatekeeper`'s truthy-gated version, so `None` (C's `0`)
        // compares equal to a real `player_id` only if that id itself were
        // `0` (never true for a live character).
        if tick < data.last_talk + CAMHERMIT_TALK_VICTIM_TICKS
            && data.current_victim.map_or(0, |victim| victim.0) != player_id.0
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:740-743`).
        if camhermit_id == player_id
            || !char_see_char(&camhermit, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`gwendylon.c:745-748`).
        if char_dist(&camhermit, &player) > CAMHERMIT_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.state;
        let reminder = format!(
            "Hail, {}! Didst thou understand? Or dost thou want me to repeat mine words?",
            player.name
        );

        if facts.state == CAMHERMIT_STATE_ENTRY {
            self.npc_quiet_say(
                camhermit_id,
                "Who enters my domain! You shall fear the wrath of... Oh, its just an adventurer!",
            );
            didsay = true;
            new_state = CAMHERMIT_STATE_QUEST1WAIT;
        } else if facts.state == CAMHERMIT_STATE_QUEST1WAIT {
            if player.level >= 9 {
                new_state = CAMHERMIT_STATE_QUEST1_1;
            }
        } else if facts.state == CAMHERMIT_STATE_QUEST1_1 {
            self.npc_quiet_say(
                camhermit_id,
                "Those villagers keep bothering me, but adventurers who fight the evil beasts have my favor.",
            );
            didsay = true;
            new_state = CAMHERMIT_STATE_QUEST1_2;
        } else if facts.state == CAMHERMIT_STATE_QUEST1_2 {
            self.npc_quiet_say(
                camhermit_id,
                "Those big bears have become a problem, back when it was only mama bear around one could simply avoid danger, but now anyone who travel in the forest is in great peril.",
            );
            didsay = true;
            new_state = CAMHERMIT_STATE_QUEST1_3;
        } else if facts.state == CAMHERMIT_STATE_QUEST1_3 {
            self.npc_quiet_say(
                camhermit_id,
                "They have gathered near a huge cavern in the western forest. If thou could go there and kill at least 10 of them I would be grateful, and sleep safely at night without them lurking about.",
            );
            didsay = true;
            events.push(CamhermitOutcomeEvent::QuestOpen {
                player_id,
                quest: QLOG_HERMIT_QUEST1,
            });
            new_state = CAMHERMIT_STATE_QUEST1DO;
        } else if facts.state == CAMHERMIT_STATE_QUEST1DO {
            if facts.kills >= CAMHERMIT_QUEST1_KILLSNEEDED {
                self.npc_quiet_say(
                    camhermit_id,
                    "Thou hast brought some fear into those beasts, I thank thee.",
                );
                didsay = true;
                events.push(CamhermitOutcomeEvent::QuestDone {
                    player_id,
                    quest: QLOG_HERMIT_QUEST1,
                });
                new_state = CAMHERMIT_STATE_QUEST2WAIT;
            } else if now.saturating_sub(facts.seen_timer) > CAMHERMIT_SEEN_REMINDER_SECONDS {
                self.npc_quiet_say(camhermit_id, &reminder);
                didsay = true;
            }
        } else if facts.state == CAMHERMIT_STATE_QUEST2WAIT
            || facts.state == CAMHERMIT_STATE_QUEST2_1
        {
            self.npc_quiet_say(
                camhermit_id,
                &format!("Could I ask another favor from thee {}?", player.name),
            );
            didsay = true;
            new_state = CAMHERMIT_STATE_QUEST2_2;
        } else if facts.state == CAMHERMIT_STATE_QUEST2_2 {
            self.npc_quiet_say(
                camhermit_id,
                "I want revenge on the bears. Last night a group of baby bears disturbed my sleep.",
            );
            didsay = true;
            new_state = CAMHERMIT_STATE_QUEST2_3;
        } else if facts.state == CAMHERMIT_STATE_QUEST2_3 {
            self.npc_quiet_say(
                camhermit_id,
                &format!(
                    "Go kill those bears, and as proof bring me their teeth. I shall pay thee for bringing me {CAMHERMIT_QUEST2_TEETHNEEDED} teeth to create a necklace."
                ),
            );
            didsay = true;
            new_state = CAMHERMIT_STATE_QUEST2_4;
        } else if facts.state == CAMHERMIT_STATE_QUEST2_4 {
            self.npc_quiet_say(
                camhermit_id,
                "From their teeth I shall create necklaces to wear myself and sell to the wanderers of the forest. To serve as a constant reminder to the bears of what happens to those who anger me.",
            );
            didsay = true;
            events.push(CamhermitOutcomeEvent::QuestOpen {
                player_id,
                quest: QLOG_HERMIT_QUEST2,
            });
            new_state = CAMHERMIT_STATE_QUEST2DO;
        } else if facts.state == CAMHERMIT_STATE_QUEST2DO_WAIT {
            if now.saturating_sub(facts.seen_timer) > CAMHERMIT_SEEN_REMINDER_SECONDS {
                new_state = CAMHERMIT_STATE_QUEST2DO;
            }
        } else if facts.state == CAMHERMIT_STATE_QUEST2DO {
            if self.count_item_inventory_by_template(player_id, IID_AREA1_SMALL_BEAR_TEETH)
                >= CAMHERMIT_QUEST2_TEETHNEEDED
            {
                let mut teeth_stack_cnt: u32 = 0;
                while self.collect_camhermit_teeth(player_id) {
                    teeth_stack_cnt += 1;
                }
                if teeth_stack_cnt > 0 {
                    let gold_amount =
                        CAMHERMIT_QUEST2_GOLD_PER_NEEDED_STACK * 100 * teeth_stack_cnt;
                    self.npc_quiet_say(
                        camhermit_id,
                        "These teeth will make a fine necklace. Here is thy payment.",
                    );
                    if let Some(character) = self.characters.get_mut(&player_id) {
                        character.gold = character.gold.saturating_add(gold_amount);
                        character.flags.insert(CharacterFlags::ITEMS);
                    }
                    self.queue_system_text_bytes(player_id, give_money_message(gold_amount));
                    events.push(CamhermitOutcomeEvent::GoldEarned {
                        player_id,
                        amount: gold_amount,
                    });
                    didsay = true;
                    events.push(CamhermitOutcomeEvent::QuestDone {
                        player_id,
                        quest: QLOG_HERMIT_QUEST2,
                    });
                    new_state = CAMHERMIT_STATE_DONE;
                } else {
                    // Unreachable in practice (the outer count check
                    // already guarantees >= 10 total), kept for parity
                    // with C's own dead defensive branch.
                    self.npc_quiet_say(
                        camhermit_id,
                        "You tried to cheat on me? Thy quest shall be reseted.",
                    );
                    didsay = true;
                    new_state = CAMHERMIT_STATE_QUEST2WAIT;
                }
            } else {
                self.npc_quiet_say(camhermit_id, &reminder);
                new_state = CAMHERMIT_STATE_QUEST2DO_WAIT;
                didsay = true;
            }
        } else if facts.state == CAMHERMIT_STATE_QUEST2_REOPEN {
            if facts.quest2_done_count >= 10 {
                self.npc_quiet_say(
                    camhermit_id,
                    "I have now more necklaces than there art people who walk amongst these trees, begone! I have no more work for thee!",
                );
                didsay = true;
                new_state = CAMHERMIT_STATE_DONE;
            } else {
                self.npc_quiet_say(
                    camhermit_id,
                    &format!(
                        "I shall pay thee for bringing me {CAMHERMIT_QUEST2_TEETHNEEDED} teeth to create a necklace."
                    ),
                );
                didsay = true;
                events.push(CamhermitOutcomeEvent::QuestReopen {
                    player_id,
                    quest: QLOG_HERMIT_QUEST2,
                });
                new_state = CAMHERMIT_STATE_QUEST2DO;
            }
        }
        // `CAMHERMIT_STATE_DONE` and any other value: no-op, matching C's
        // empty `case CAMHERMIT_STATE_DONE: break;`.

        if new_state != facts.state {
            events.push(CamhermitOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }
        // C `ppd->camhermit_seen_timer = realtime;` (`gwendylon.c:918`):
        // unconditional, regardless of `didsay`.
        events.push(CamhermitOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:920-924`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `camhermit_driver`'s `NT_TEXT` branch (`gwendylon.c:927-957`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::gatekeeper::gate_welcome_handle_text_message`).
    fn camhermit_handle_text_message(
        &mut self,
        camhermit_id: CharacterId,
        camhermit_name: &str,
        data: &mut CamhermitDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CamhermitPlayerFacts>,
        events: &mut Vec<CamhermitOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`gwendylon.c:136-
        // 149`): ignore our own talk, non-players, distance > 12,
        // not-visible.
        if camhermit_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(camhermit) = self.characters.get(&camhermit_id).cloned() else {
            return;
        };
        if char_dist(&camhermit, &speaker) > CAMHERMIT_QA_DISTANCE
            || !char_see_char(&camhermit, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, camhermit_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(camhermit_id, &reply);
                didsay = true;
            }
            // C `case 2: // Repeat` (`gwendylon.c:931-943`).
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    if facts.state == CAMHERMIT_STATE_QUEST1DO {
                        events.push(CamhermitOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: CAMHERMIT_STATE_QUEST1_1,
                        });
                        data.last_talk = 0;
                    } else if facts.state == CAMHERMIT_STATE_QUEST2DO {
                        events.push(CamhermitOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: CAMHERMIT_STATE_QUEST2_1,
                        });
                        data.last_talk = 0;
                    } else if facts.state >= CAMHERMIT_STATE_DONE {
                        events.push(CamhermitOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: CAMHERMIT_STATE_QUEST2_REOPEN,
                        });
                        data.last_talk = 0;
                    }
                }
                didsay = true;
            }
            // Every other matched code (3/4/9/10/11/12/13, meaningful only
            // to `gwendylon_driver`'s own bigger `switch`) is unhandled by
            // camhermit's C `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:954-957`) - note this does *not* touch
        // `dat->last_talk`, unlike the `NT_CHAR` branch.
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `camhermit_driver`'s `NT_GIVE` branch (`gwendylon.c:959-971`).
    fn camhermit_handle_give_message(
        &mut self,
        camhermit_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&camhermit_id)
            .and_then(|camhermit| camhermit.cursor_item.take())
        else {
            return;
        };

        self.npc_quiet_say(
            camhermit_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        // C `if (!give_char_item_smart(co, ch[cn].citem, 1)) { destroy_item
        // (ch[cn].citem); }` - `give_char_item_smart` already destroys the
        // item internally on every failure path (no space, `IF_NODROP`,
        // drop failure), so no extra destroy call is needed here.
        self.give_char_item_smart(giver_id, item_id, true);
    }

    /// C `count_item_inventory(co, itemID)` (`src/system/tool.c:199-208`):
    /// counts inventory slots `30..INVENTORYSIZE` only (not the cursor),
    /// one per matching stack regardless of stack size.
    fn count_item_inventory_by_template(
        &self,
        character_id: CharacterId,
        template_id: u32,
    ) -> usize {
        let Some(character) = self.characters.get(&character_id) else {
            return 0;
        };
        character
            .inventory
            .iter()
            .skip(INVENTORY_START_INVENTORY)
            .flatten()
            .filter(|item_id| {
                self.items
                    .get(item_id)
                    .is_some_and(|item| item.template_id == template_id)
            })
            .count()
    }

    /// C `collect_teeth(cn, co)` (`src/area/1/gwendylon.c:681-699`): scans
    /// `character_id`'s inventory slots `30..INVENTORYSIZE` once in order,
    /// destroying every `IID_AREA1_SMALL_BEAR_TEETH` item found, stopping
    /// as soon as 10 have been destroyed (returns `true`). If a full pass
    /// ends with fewer than 10 found, every teeth item that pass *did*
    /// find has still been destroyed - a genuine C quirk (the reward loop
    /// below calls this repeatedly while it returns `true`; the final,
    /// short pass that returns `false` still silently eats whatever
    /// remainder, 0..9, teeth items were left) - preserved digit-for-digit
    /// per the porting rule, not "fixed".
    fn collect_camhermit_teeth(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let candidate_item_ids: Vec<ItemId> = character
            .inventory
            .iter()
            .skip(INVENTORY_START_INVENTORY)
            .flatten()
            .copied()
            .collect();

        let mut cnt = 0usize;
        for item_id in candidate_item_ids {
            if self
                .items
                .get(&item_id)
                .is_some_and(|item| item.template_id == IID_AREA1_SMALL_BEAR_TEETH)
            {
                cnt += 1;
                self.destroy_item(item_id);
                if cnt >= CAMHERMIT_QUEST2_TEETHNEEDED {
                    return true;
                }
            }
        }
        false
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct camhermit_driver_data` (`src/area/1/gwendylon.c:702-705`): the
/// forest hermit NPC's own driver memory (`CDR_CAMHERMIT`, distinct from
/// the per-player `camhermit_state`/`camhermit_seen_timer`/`camhermit_kills`
/// fields in `crate::player::PlayerRuntime`'s `area1_ppd` - see
/// `world::camhermit`'s module doc comment for the split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CamhermitDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
