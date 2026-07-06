//! Hunter NPC (`CDR_YOAKIN`), area 1's bear-hunt quest giver at the
//! knight castle.
//!
//! Ports `src/area/1/gwendylon.c::yoakin_driver` (`:996-1217`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`). Follows the same `World`/`PlayerRuntime` split
//! established there: the caller supplies a per-player fact snapshot
//! ([`YoakinPlayerFacts`]) up front and applies the returned
//! [`YoakinOutcomeEvent`]s afterwards, since `yoakin_state`/
//! `yoakin_seen_timer`/`logain_state`/`shrike_state`/`shrike_fails` (all
//! `area1_ppd` fields) and the `QLOG_YOAKIN` quest-log count live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - `destroy_item_byID(co, IID_AREA1_BIGBEAR_TOOTH)`
//!   (`gwendylon.c:1155`) sweeps every matching item in the player's
//!   equipment/inventory/cursor *and* their account depot
//!   (`DRD_DEPOT_PPD`). [`World::destroy_items_by_template_id`] only
//!   covers the first three (depot storage lives in `ugaris-server`'s
//!   `PlayerRuntime`/DB layer, outside `World`) - a stray big-bear-tooth
//!   parked in the depot at turn-in time will not be swept here, unlike
//!   on the C server. Rare in practice (the item only exists to be handed
//!   straight to Yoakin) and consistent with this codebase's existing
//!   depot-blind-spot precedent elsewhere.
//! - The C `case 4` reminder line wraps "repeat" in `COL_LIGHT_BLUE`/
//!   `COL_RESET` markers (`gwendylon.c:1095-1097`); dropped here for the
//!   same reason documented on `world::camhermit`'s module doc comment
//!   (`World::npc_quiet_say` broadcasts a plain UTF-8 `String`).
//! - The bear-tooth reward (`create_money_item(MONEY_AREA1_BEARTOOTH)` +
//!   `give_char_item_smart`, `gwendylon.c:1163-1166`) is ported as the
//!   direct effect `give_char_item_smart`'s own `IF_MONEY` branch already
//!   produces (credit `character.gold`, queue the "received gold" system
//!   text, done) rather than round-tripping through a throwaway `Item`
//!   that would be destroyed on the very next line - behaviorally
//!   identical, since a money item is never placed anywhere observable
//!   before `give_char_item_smart` consumes it.

use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    analyse_text_qa, TextAnalysisOutcome, YoakinDriverData, CDR_YOAKIN, GWENDYLON_QA,
};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_AREA1_BIGBEAR_TOOTH, IID_SHRIKE_TALISMAN};
use crate::quest::quest_exp::MONEY_AREA1_BEARTOOTH;

/// C `char_dist(cn, co) > 10` (`gwendylon.c:1048`): the `NT_CHAR` greeting
/// range.
const YOAKIN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const YOAKIN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gwendylon.c:1031`).
const YOAKIN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gwendylon.c:1036`, `:1118`).
const YOAKIN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:1210`): idle "return to post" threshold.
const YOAKIN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `realtime - ppd->yoakin_seen_timer > 120` (`gwendylon.c:1057`): the
/// intro-chain (states 1-3) reset-to-1 window.
const YOAKIN_RESET_SECONDS: i32 = 120;
/// C `realtime - ppd->yoakin_seen_timer > 60` (`gwendylon.c:1093`): the
/// state-4 "did you find the bear" reminder window.
const YOAKIN_STATE4_REMINDER_SECONDS: i32 = 60;
/// C `ppd->logain_state < 6` (`gwendylon.c:1076`): the "mad knights"
/// quest-completion gate for state 2 -> 3.
const YOAKIN_LOGAIN_STATE_REQUIRED: i32 = 6;

/// C's bare `int` state values for `ppd->yoakin_state` - no `#define`
/// names exist in the C source (unlike `CAMHERMIT_STATE_*`), so these are
/// named here purely for readability.
const YOAKIN_STATE_ENTRY: i32 = 0;
const YOAKIN_STATE_WARNED: i32 = 1;
const YOAKIN_STATE_KNIGHTS_GATE: i32 = 2;
const YOAKIN_STATE_QUEST_GIVEN: i32 = 3;
const YOAKIN_STATE_QUEST_DO: i32 = 4;
const YOAKIN_STATE_DONE: i32 = 5;

/// Per-player facts [`World::process_yoakin_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YoakinPlayerFacts {
    /// `PlayerRuntime::area1_yoakin_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_yoakin_seen_timer()` (C `realtime` wall-clock
    /// seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
    /// `PlayerRuntime::area1_logain_state()`: gates the state 2 -> 3
    /// transition on the "mad knights" quest being done.
    pub logain_state: i32,
    /// `PlayerRuntime::quest_log.count(QLOG_YOAKIN)`, sampled *before*
    /// this tick's completion is applied - `0` here means C's
    /// `questlog_done`'s return value would be `1` (first completion),
    /// gating the bear-tooth gold reward.
    pub quest_done_count: u8,
    /// `PlayerRuntime::area1_shrike_state()`.
    pub shrike_state: i32,
    /// `PlayerRuntime::area1_shrike_fails()`.
    pub shrike_fails: i32,
    /// `ch[co].level`, needed for the shrike-talisman exp reward formula.
    pub level: u32,
}

/// A side effect [`World::process_yoakin_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoakinOutcomeEvent {
    /// Write the new `area1_ppd.yoakin_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->yoakin_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message (`gwendylon.c:1104`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, 5)`.
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 5)` - the caller must apply
    /// `PlayerRuntime::quest_log.complete_legacy` (exp reward + resend).
    QuestDone { player_id: CharacterId },
    /// C `give_char_item_smart(co, in, 1)`'s `IF_MONEY` branch for the
    /// `create_money_item(MONEY_AREA1_BEARTOOTH)` reward - see the module
    /// doc comment's last bullet for why this is applied directly rather
    /// than via a throwaway `Item`.
    GoldEarned { player_id: CharacterId, amount: u32 },
    /// Write the new `area1_ppd.shrike_state` back (`gwendylon.c:1179`).
    UpdateShrikeState {
        player_id: CharacterId,
        new_state: i32,
    },
}

impl World {
    /// C `yoakin_driver`'s per-tick body (`gwendylon.c:996-1217`). `now`
    /// is C's wall-clock `realtime` (seconds).
    pub fn process_yoakin_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, YoakinPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<YoakinOutcomeEvent> {
        let yoakin_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_YOAKIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for yoakin_id in yoakin_ids {
            self.process_yoakin_messages(yoakin_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_yoakin_messages(
        &mut self,
        yoakin_id: CharacterId,
        player_facts: &HashMap<CharacterId, YoakinPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<YoakinOutcomeEvent>,
    ) {
        let Some(yoakin_name) = self
            .characters
            .get(&yoakin_id)
            .map(|yoakin| yoakin.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Yoakin(mut data)) = self
            .characters
            .get(&yoakin_id)
            .and_then(|yoakin| yoakin.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&yoakin_id)
            .map(|yoakin| std::mem::take(&mut yoakin.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.yoakin_handle_char_message(
                    yoakin_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.yoakin_handle_text_message(
                    yoakin_id,
                    &yoakin_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.yoakin_handle_give_message(
                    yoakin_id,
                    message,
                    player_facts,
                    area_id,
                    events,
                ),
                _ => {}
            }
        }

        if let Some(yoakin) = self.characters.get_mut(&yoakin_id) {
            yoakin.driver_state = Some(CharacterDriverState::Yoakin(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:1206-1208`).
        if let (Some(yoakin), Some((tx, ty))) =
            (self.characters.get(&yoakin_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(yoakin.x), i32::from(yoakin.y), tx, ty) {
                if let Some(yoakin_mut) = self.characters.get_mut(&yoakin_id) {
                    let _ = turn(yoakin_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; } do_idle(cn, TICKS);` (`gwendylon.c:1210-
        // 1216`). The NPC's post position (C's `tmpx`/`tmpy`) reuses
        // `rest_x`/`rest_y`, the same substitution `world::camhermit`
        // already uses for other stationary NPCs' spawn tiles.
        let last_talk = if let Some(yoakin) = self.characters.get(&yoakin_id) {
            match yoakin.driver_state.as_ref() {
                Some(CharacterDriverState::Yoakin(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + YOAKIN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(yoakin) = self.characters.get(&yoakin_id) else {
                return;
            };
            let (post_x, post_y) = (yoakin.rest_x, yoakin.rest_y);
            self.secure_move_driver(
                yoakin_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `yoakin_driver`'s `NT_CHAR` branch (`gwendylon.c:1012-1111`).
    fn yoakin_handle_char_message(
        &mut self,
        yoakin_id: CharacterId,
        data: &mut YoakinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, YoakinPlayerFacts>,
        now: i32,
        events: &mut Vec<YoakinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(yoakin) = self.characters.get(&yoakin_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:1016-1019`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:1021-1025`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`gwendylon.c:1030-1034`).
        if tick < data.last_talk + YOAKIN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`gwendylon.c:1036-1039`) - a plain `!=`, so
        // `None` (C's `0`) compares equal to a real `player_id` only if
        // that id itself were `0` (never true for a live character).
        if tick < data.last_talk + YOAKIN_TALK_VICTIM_TICKS
            && data.current_victim.map_or(0, |victim| victim.0) != player_id.0
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:1041-1045`).
        if yoakin_id == player_id || !char_see_char(&yoakin, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`gwendylon.c:1047-
        // 1051`).
        if char_dist(&yoakin, &player) > YOAKIN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `if (realtime - ppd->yoakin_seen_timer > 120 &&
        // ppd->yoakin_state && ppd->yoakin_state < 4) { ppd->yoakin_state
        // = 1; }` (`gwendylon.c:1057-1059`).
        let mut new_state = if now.saturating_sub(facts.seen_timer) > YOAKIN_RESET_SECONDS
            && facts.state != YOAKIN_STATE_ENTRY
            && facts.state < YOAKIN_STATE_QUEST_DO
        {
            YOAKIN_STATE_WARNED
        } else {
            facts.state
        };

        if new_state == YOAKIN_STATE_ENTRY {
            self.npc_quiet_say(
                yoakin_id,
                &format!("Hail {}! I am {}, the hunter.", player.name, yoakin.name),
            );
            events.push(YoakinOutcomeEvent::QuestOpen { player_id });
            didsay = true;
            new_state = YOAKIN_STATE_WARNED;
        } else if new_state == YOAKIN_STATE_WARNED {
            self.npc_quiet_say(
                yoakin_id,
                &format!(
                    "Be careful in the forest around the village, {}. There are wolves and bears about which can be very aggressive.",
                    player.name
                ),
            );
            didsay = true;
            new_state = YOAKIN_STATE_KNIGHTS_GATE;
        } else if new_state == YOAKIN_STATE_KNIGHTS_GATE {
            if facts.logain_state >= YOAKIN_LOGAIN_STATE_REQUIRED {
                self.npc_quiet_say(
                    yoakin_id,
                    &format!(
                        "Greetings again, {}. Lately, there have been reports of a huge mother bear on the path to the city.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = YOAKIN_STATE_QUEST_GIVEN;
            }
        } else if new_state == YOAKIN_STATE_QUEST_GIVEN {
            self.npc_quiet_say(
                yoakin_id,
                "This bear has been killing several travellers, and I put a price on its head. So if thou happen to kill it, bring me its teeth as proof.",
            );
            didsay = true;
            new_state = YOAKIN_STATE_QUEST_DO;
        } else if new_state == YOAKIN_STATE_QUEST_DO {
            if now.saturating_sub(facts.seen_timer) > YOAKIN_STATE4_REMINDER_SECONDS {
                self.npc_quiet_say(
                    yoakin_id,
                    &format!(
                        "Hail, {}! Didst thou find that big mother bear? Or dost thou want me to repeat mine offer?",
                        player.name
                    ),
                );
                didsay = true;
            }
        }
        // `YOAKIN_STATE_DONE` and any other value: no-op, matching C's
        // empty `case 5: break;`.

        if new_state != facts.state {
            events.push(YoakinOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }
        // C `ppd->yoakin_seen_timer = realtime;` (`gwendylon.c:1104`):
        // unconditional, regardless of `didsay`.
        events.push(YoakinOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:1106-1110`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `yoakin_driver`'s `NT_TEXT` branch (`gwendylon.c:1114-1141`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::camhermit`'s text handler).
    fn yoakin_handle_text_message(
        &mut self,
        yoakin_id: CharacterId,
        yoakin_name: &str,
        data: &mut YoakinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, YoakinPlayerFacts>,
        events: &mut Vec<YoakinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:1118-1120`).
        let tick = self.tick.0;
        if tick > data.last_talk + YOAKIN_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:1122-1125`).
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
        if yoakin_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(yoakin) = self.characters.get(&yoakin_id).cloned() else {
            return;
        };
        if char_dist(&yoakin, &speaker) > YOAKIN_QA_DISTANCE
            || !char_see_char(&yoakin, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, yoakin_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(yoakin_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:1128-1134`).
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    if facts.state <= YOAKIN_STATE_QUEST_DO {
                        events.push(YoakinOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: YOAKIN_STATE_KNIGHTS_GATE,
                        });
                        data.last_talk = 0;
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by yoakin's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:1137-1140`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2` branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `yoakin_driver`'s `NT_GIVE` branch (`gwendylon.c:1144-1193`).
    fn yoakin_handle_give_message(
        &mut self,
        yoakin_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, YoakinPlayerFacts>,
        area_id: u16,
        events: &mut Vec<YoakinOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&yoakin_id)
            .and_then(|yoakin| yoakin.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let facts = player_facts.get(&giver_id).copied();

        if template_id == IID_AREA1_BIGBEAR_TOOTH
            && facts.is_some_and(|facts| facts.state <= YOAKIN_STATE_QUEST_DO)
        {
            let facts = facts.expect("checked above");
            self.npc_quiet_say(
                yoakin_id,
                &format!(
                    "Thank thee, {}. The forest will be safer now.",
                    self.characters
                        .get(&giver_id)
                        .map(|giver| giver.name.clone())
                        .unwrap_or_default()
                ),
            );
            events.push(YoakinOutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA1_BIGBEAR_TOOTH);
            events.push(YoakinOutcomeEvent::UpdateState {
                player_id: giver_id,
                new_state: YOAKIN_STATE_DONE,
            });
            self.destroy_item(item_id);

            // C `if (tmp == 1)` (`gwendylon.c:1162-1167`): only reward
            // gold the first time this quest is completed. Unlike
            // camhermit's `CAMHERMIT_QUEST2_GOLD_PER_NEEDED_STACK` (a
            // "Gold" unit needing an explicit `* 100` conversion to
            // silver at its call site), `MONEY_AREA1_BEARTOOTH` is fed to
            // `create_money_item` directly - already silver units.
            if facts.quest_done_count == 0 {
                if let Some(giver) = self.characters.get_mut(&giver_id) {
                    let amount = MONEY_AREA1_BEARTOOTH.max(0) as u32;
                    giver.gold = giver.gold.saturating_add(amount);
                    giver.flags.insert(CharacterFlags::ITEMS);
                    self.queue_system_text_bytes(giver_id, give_money_message(amount));
                    events.push(YoakinOutcomeEvent::GoldEarned {
                        player_id: giver_id,
                        amount,
                    });
                }
            }
        } else if template_id == IID_SHRIKE_TALISMAN
            && facts.is_some_and(|facts| facts.shrike_state == 0)
        {
            let facts = facts.expect("checked above");
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_emote(yoakin_id, "turns deadly pale and starts to tremble");
            self.npc_quiet_say(
                yoakin_id,
                &format!("I... I thank thee, {giver_name}. I'd have never thought... Thank thee!"),
            );
            let level_val = level_value(facts.level);
            if facts.shrike_fails != 0 {
                self.npc_quiet_say(
                    yoakin_id,
                    "And I forgive thee trying to kill me. My wounds were almost fatal, but I survived.",
                );
                self.give_exp(
                    giver_id,
                    i64::from((level_val / 5).min(143_462)),
                    u32::from(area_id),
                );
            } else {
                self.give_exp(
                    giver_id,
                    i64::from((level_val / 5).min(286_925)),
                    u32::from(area_id),
                );
            }
            events.push(YoakinOutcomeEvent::UpdateShrikeState {
                player_id: giver_id,
                new_state: 1,
            });
            self.destroy_item(item_id);
        } else {
            self.npc_quiet_say(
                yoakin_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            self.give_char_item_smart(giver_id, item_id, true);
        }
    }
}
