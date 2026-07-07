//! Main quest-giver mage's daughter NPC (`CDR_LYDIA`), area 1's hungover
//! tower resident who hands the player off to `world::gwendylon` once her
//! own hangover-potion quest chain (`QLOG_LYDIA`) is done.
//!
//! Ports `src/area/1/gwendylon.c::lydia_driver` (`:3458-3703`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`/`world::terion`/`world::gwendylon`/
//! `world::greeter`/`world::jessica`/`world::jiu`/`world::forest_ranger`/
//! `world::brithildie`/`world::nook`). Follows the same `World`/
//! `PlayerRuntime` split established there: the caller supplies a
//! per-player fact snapshot ([`LydiaPlayerFacts`]) up front and applies
//! the returned [`LydiaOutcomeEvent`]s afterwards, since `lydia_state`/
//! `lydia_seen_timer` (`area1_ppd` fields) and `QLOG_LYDIA` live on
//! `crate::player::PlayerRuntime`, not `World`. `world::greeter`'s own
//! module doc comment already documented this NPC's write side as "not
//! yet ported" - this file closes that gap.
//!
//! Deviations/gaps (documented, not silent):
//! - The reward potion (`mana_potion1`/`healing_potion1` by
//!   `(ch[co].flags & (CF_WARRIOR|CF_MAGE)) == CF_MAGE`,
//!   `gwendylon.c:614-618`) is a `create_item`+`give_char_item` call
//!   `World` cannot perform directly (no `ZoneLoader`/DB access, same
//!   architectural gap as `world::gwendylon`'s `IID_CALIGARLETTER`
//!   hand-off) - queued as a [`LydiaOutcomeEvent::GrantPotion`] carrying
//!   the resolved template key (the class check itself runs here, in
//!   `World`, since `Character::flags` is visible here), applied by
//!   `ugaris-server`'s `area1.rs::apply_lydia_events` via
//!   `grant_template_item_smart`... actually via the plain (non-drop)
//!   `give_item_to_character` path, matching C's plain `give_char_item`
//!   (destroy on failure, no ground-drop fallback).
//! - `destroy_item_byID(co, ID)` (`gwendylon.c:610-611`) sweeps the
//!   player's equipment/inventory/cursor via
//!   [`World::destroy_items_by_template_id`] but not the account depot
//!   (`DRD_DEPOT_PPD`) - same documented gap as `world::gwendylon`/
//!   `world::yoakin`/`world::nook`.
//! - The state-4 reminder line wraps "repeat" in `COL_LIGHT_BLUE`/
//!   `COL_RESET` markers in C (`gwendylon.c:3548`); restored via
//!   `COL_STR_LIGHT_BLUE`/`COL_STR_RESET` sentinels and
//!   `World::npc_quiet_say_bytes`, same mechanism as `world::camhermit`.
//! - The pre-`switch` auto-reset (`gwendylon.c:3512-3514`: `realtime -
//!   ppd->lydia_seen_timer > 120 && ppd->lydia_state && ppd->lydia_state <
//!   4` resets the state to `0`) is a genuinely new pattern not seen in
//!   any other area-1 NPC ported so far - modeled here as a fact-based
//!   `new_state` pre-adjustment before the main `match`.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LYDIA, GWENDYLON_QA};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_AREA1_WOODKEY2, IID_AREA1_WOODPOTION};
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;

/// C `char_dist(cn, co) > 10` (`gwendylon.c:3502`): the `NT_CHAR` greeting
/// range.
const LYDIA_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const LYDIA_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gwendylon.c:3489`).
const LYDIA_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gwendylon.c:3494`, `:3583`).
const LYDIA_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:3672`): idle "return to post" threshold.
const LYDIA_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS * 45` (`gwendylon.c:3679`): idle-mutterings gate.
const LYDIA_MUTTER_TICKS: u64 = TICKS_PER_SECOND * 45;
/// C `realtime - ppd->lydia_seen_timer > 120` (`gwendylon.c:3512`): the
/// pre-`switch` auto-reset gate.
const LYDIA_STATE_RESET_SECONDS: i32 = 120;
/// C `realtime - ppd->lydia_seen_timer > 60` (`gwendylon.c:3547`): state
/// 4's reminder gate.
const LYDIA_REMINDER_SECONDS: i32 = 60;

/// C `lydia_mutterings[]` (`gwendylon.c:3684-3697`): the hungover
/// idle-muttering table, 12 entries, indexed by `RANDOM(12)`.
const LYDIA_MUTTERINGS: [&str; 12] = [
    "Ohhh, my head... never again.",
    "Why is everything so LOUD?",
    "I swear I will never drink again. I mean it this time.",
    "Is it possible to die from a headache? Asking for myself.",
    "The light... it burns...",
    "James and his 'one more drink.' This is his fault.",
    "Water. I need water. And silence. Mostly silence.",
    "Was that a bird? It sounded like a cannon.",
    "I think the room is spinning. Or I am. Hard to tell.",
    "Father's potions cure everything except poor life choices.",
    "If anyone mentions ale right now, I will scream. Then regret screaming.",
    "Note to self: 'party like there's no tomorrow' assumes you survive till tomorrow.",
];

/// Per-player facts [`World::process_lydia_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LydiaPlayerFacts {
    /// `PlayerRuntime::area1_lydia_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_lydia_seen_timer()` (C `realtime` wall-clock
    /// seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
}

/// A side effect [`World::process_lydia_actions`] could not apply
/// directly because it touches `PlayerRuntime` (or, for
/// [`LydiaOutcomeEvent::GrantPotion`], needs `ZoneLoader`). See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LydiaOutcomeEvent {
    /// Write the new `area1_ppd.lydia_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->lydia_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message (`gwendylon.c:3568`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, 0)` (`gwendylon.c:3518`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, QLOG_LYDIA)` (`gwendylon.c:3606`) plus
    /// `achievement_award(co, ACHIEVEMENT_A_HELPING_HAND, 1)`
    /// (`gwendylon.c:3607`) - the caller applies both.
    QuestDone { player_id: CharacterId },
    /// C's class-conditional `create_item(...)`/`give_char_item(co, in)`
    /// reward (`gwendylon.c:614-621` - see the module doc comment for why
    /// this is deferred to `ugaris-server`).
    GrantPotion {
        player_id: CharacterId,
        template: &'static str,
    },
}

impl World {
    /// C `lydia_driver`'s per-tick body (`gwendylon.c:3458-3703`). `now`
    /// is C's wall-clock `realtime` (seconds).
    pub fn process_lydia_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, LydiaPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<LydiaOutcomeEvent> {
        let lydia_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LYDIA
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for lydia_id in lydia_ids {
            self.process_lydia_messages(lydia_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_lydia_messages(
        &mut self,
        lydia_id: CharacterId,
        player_facts: &HashMap<CharacterId, LydiaPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<LydiaOutcomeEvent>,
    ) {
        let Some(lydia_name) = self
            .characters
            .get(&lydia_id)
            .map(|lydia| lydia.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Lydia(mut data)) = self
            .characters
            .get(&lydia_id)
            .and_then(|lydia| lydia.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&lydia_id)
            .map(|lydia| std::mem::take(&mut lydia.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.lydia_handle_char_message(
                    lydia_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.lydia_handle_text_message(
                    lydia_id,
                    &lydia_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.lydia_handle_give_message(lydia_id, message, player_facts, events),
                _ => {}
            }
        }

        if let Some(lydia) = self.characters.get_mut(&lydia_id) {
            lydia.driver_state = Some(CharacterDriverState::Lydia(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:3667-3669`).
        if let (Some(lydia), Some((tx, ty))) =
            (self.characters.get(&lydia_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(lydia.x), i32::from(lydia.y), tx, ty) {
                if let Some(lydia_mut) = self.characters.get_mut(&lydia_id) {
                    let _ = turn(lydia_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`gwendylon.c:3672-3676`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary area-1 NPC uses.
        let last_talk = if let Some(lydia) = self.characters.get(&lydia_id) {
            match lydia.driver_state.as_ref() {
                Some(CharacterDriverState::Lydia(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + LYDIA_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(lydia) = self.characters.get(&lydia_id) else {
                return;
            };
            let (post_x, post_y) = (lydia.rest_x, lydia.rest_y);
            let moved = self.secure_move_driver(
                lydia_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
            if moved {
                return;
            }
        }

        // C's "Lydia idle mutterings" block (`gwendylon.c:3679-3698`).
        self.lydia_idle_chatter(lydia_id, last_talk);
    }

    /// C `lydia_driver`'s `NT_CHAR` branch (`gwendylon.c:3474-3576`).
    #[allow(clippy::too_many_arguments)]
    fn lydia_handle_char_message(
        &mut self,
        lydia_id: CharacterId,
        data: &mut LydiaDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, LydiaPlayerFacts>,
        now: i32,
        events: &mut Vec<LydiaOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(lydia) = self.characters.get(&lydia_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:3478-3481`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:3483-3486`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`gwendylon.c:3489-3492`).
        if tick < data.last_talk + LYDIA_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`gwendylon.c:3494-3497`) - note, like
        // `world::nook`, no `dat->current_victim &&` truthy guard here.
        if tick < data.last_talk + LYDIA_TALK_VICTIM_TICKS
            && data.current_victim.map_or(0, |victim| victim.0) != player_id.0
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:3499-3502`).
        if lydia_id == player_id || !char_see_char(&lydia, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`gwendylon.c:3504-
        // 3507`).
        if char_dist(&lydia, &player) > LYDIA_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        // C `if (realtime - ppd->lydia_seen_timer > 120 &&
        // ppd->lydia_state && ppd->lydia_state < 4) { ppd->lydia_state =
        // 0; }` (`gwendylon.c:3512-3514`) - see the module doc comment.
        let mut state = facts.state;
        if now.saturating_sub(facts.seen_timer) > LYDIA_STATE_RESET_SECONDS
            && state > 0
            && state < 4
        {
            state = 0;
        }

        let mut didsay = false;
        let mut new_state = state;

        // C `switch (ppd->lydia_state) { ... }` (`gwendylon.c:3516-3562`).
        match state {
            0 => {
                self.npc_quiet_say(
                    lydia_id,
                    &format!(
                        "Oohh, my head. Hu? Ah, hello {}. I am {}.",
                        player.name, lydia.name
                    ),
                );
                new_state = 1;
                didsay = true;
                events.push(LydiaOutcomeEvent::QuestOpen { player_id });
            }
            1 => {
                self.npc_quiet_say(
                    lydia_id,
                    "I am sorry, I am no good company today. I went to a party in the village last night and now I got a horrible headache. I had a potion with me which should help cure the hangover.",
                );
                new_state = 2;
                didsay = true;
            }
            2 => {
                self.npc_quiet_say(
                    lydia_id,
                    &format!(
                        "But on my way back, I got ambushed. Some thieves stole the potion. James that drunkard was supposed to bring me home, but he passed out. Thou wouldn't happen to have the time to hunt them down and bring me the potion, {}?",
                        player.name
                    ),
                );
                new_state = 3;
                didsay = true;
            }
            3 => {
                self.npc_quiet_say(
                    lydia_id,
                    "I would be grateful indeed. My head is killing me. They ambushed me right here in front of the tower and fled west with their loot.",
                );
                new_state = 4;
                didsay = true;
            }
            4 => {
                // C `if (realtime - ppd->lydia_seen_timer > 60) { ... }`
                // (`gwendylon.c:3546-3552`).
                if now.saturating_sub(facts.seen_timer) > LYDIA_REMINDER_SECONDS {
                    self.npc_quiet_say_bytes(
                        lydia_id,
                        &format!(
                            "Hello again, {}! Didst thou find the potion? Or dost thou want me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} mine offer?",
                            player.name
                        ),
                    );
                    didsay = true;
                    self.notify_area(
                        lydia.x,
                        lydia.y,
                        NT_NPC,
                        NTID_TUTORIAL,
                        0,
                        player_id.0 as i32,
                    );
                }
            }
            5 => {
                self.npc_quiet_say(lydia_id, "Here, this might save thy life.");
                new_state = 6;
                didsay = true;
            }
            6 => {
                self.npc_quiet_say(
                    lydia_id,
                    "Gwendylon, my father, is currently looking for help. If thou art looking for adventures, it might be wise to visit him. He lives next door.",
                );
                new_state = 7;
                didsay = true;
            }
            // 7: break (no-op, "Quest done, don't talk anymore :-)").
            _ => {}
        }

        // C `ppd->lydia_seen_timer = realtime;` (`gwendylon.c:3565`):
        // unconditional.
        events.push(LydiaOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });
        if new_state != facts.state {
            events.push(LydiaOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:3566-3570`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `lydia_driver`'s `NT_TEXT` branch (`gwendylon.c:3581-3617`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::nook`'s text handler).
    fn lydia_handle_text_message(
        &mut self,
        lydia_id: CharacterId,
        lydia_name: &str,
        data: &mut LydiaDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, LydiaPlayerFacts>,
        events: &mut Vec<LydiaOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:3583-3585`).
        let tick = self.tick.0;
        if tick > data.last_talk + LYDIA_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:3587-3590`).
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
        if lydia_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(lydia) = self.characters.get(&lydia_id).cloned() else {
            return;
        };
        if char_dist(&lydia, &speaker) > LYDIA_QA_DISTANCE
            || !char_see_char(&lydia, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, lydia_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(lydia_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:3592-3603`): two independent
            // `if`s (not an else-if chain in C), reset against the
            // original snapshotted `facts.state` - safe since the ranges
            // ([0,4] vs [6,7]) don't overlap, so at most one applies. See
            // the module doc comment.
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    if facts.state <= 4 {
                        data.last_talk = 0;
                        events.push(LydiaOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: 1,
                        });
                    }
                    if (6..=7).contains(&facts.state) {
                        data.last_talk = 0;
                        events.push(LydiaOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: 6,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by lydia's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:3613-3616`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2` branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `lydia_driver`'s `NT_GIVE` branch (`gwendylon.c:3620-3660`).
    fn lydia_handle_give_message(
        &mut self,
        lydia_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, LydiaPlayerFacts>,
        events: &mut Vec<LydiaOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&lydia_id)
            .and_then(|lydia| lydia.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let facts = player_facts.get(&giver_id).copied();

        if template_id == IID_AREA1_WOODPOTION && facts.is_some_and(|facts| facts.state <= 4) {
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_quiet_say(
                lydia_id,
                &format!("Ah. That feels so much better. Thank thee, {giver_name}."),
            );
            events.push(LydiaOutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA1_WOODPOTION);
            self.destroy_items_by_template_id(giver_id, IID_AREA1_WOODKEY2);

            events.push(LydiaOutcomeEvent::UpdateState {
                player_id: giver_id,
                new_state: 5,
            });

            // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;`
            // (`gwendylon.c:3608-3609`).
            self.destroy_item(item_id);

            // C `if ((ch[co].flags & (CF_WARRIOR | CF_MAGE)) ==
            // (CF_MAGE)) { in = create_item("mana_potion1"); } else { in =
            // create_item("healing_potion1"); }` (`gwendylon.c:3614-618`)
            // - pure-mage check (mage set AND warrior not set), not just
            // "has the mage flag" - see the module doc comment.
            let template = self
                .characters
                .get(&giver_id)
                .map(|giver| {
                    if giver.flags.contains(CharacterFlags::MAGE)
                        && !giver.flags.contains(CharacterFlags::WARRIOR)
                    {
                        "mana_potion1"
                    } else {
                        "healing_potion1"
                    }
                })
                .unwrap_or("healing_potion1");
            events.push(LydiaOutcomeEvent::GrantPotion {
                player_id: giver_id,
                template,
            });
        } else {
            // C `else { quiet_say(...); if (!give_char_item(co,
            // ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].citem =
            // 0; }` (`gwendylon.c:3651-3656`) - the plain `give_char_item`,
            // not `give_char_item_smart` (same documented asymmetry as
            // `world::nook`'s own `NT_GIVE` handler).
            self.npc_quiet_say(
                lydia_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }

    /// C's "Lydia idle mutterings" block (`gwendylon.c:3679-3698`): once
    /// every 45 seconds, on a `RANDOM(20)` 1-in-20 hit, murmur a random
    /// hungover line.
    fn lydia_idle_chatter(&mut self, lydia_id: CharacterId, last_talk: u64) {
        let tick = self.tick.0;
        if last_talk + LYDIA_MUTTER_TICKS >= tick {
            return;
        }
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, 20) != 0 {
            return;
        }
        let index = legacy_random_below_from_seed(
            &mut self.legacy_random_seed,
            LYDIA_MUTTERINGS.len() as u32,
        ) as usize;
        self.npc_murmur(lydia_id, LYDIA_MUTTERINGS[index]);

        if let Some(CharacterDriverState::Lydia(data)) = self
            .characters
            .get_mut(&lydia_id)
            .and_then(|lydia| lydia.driver_state.as_mut())
        {
            data.last_talk = tick;
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct lydia_driver_data` (`src/area/1/gwendylon.c:3453-3456`): the
/// hangover-quest NPC's own driver memory (`CDR_LYDIA`, distinct from the
/// per-player `lydia_state`/`lydia_seen_timer` fields in
/// `crate::player::PlayerRuntime`'s `area1_ppd` - see `world::lydia`'s
/// module doc comment for the split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LydiaDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
