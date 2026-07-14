//! Town-mage quest-giver NPC (`CDR_GUIWYNN`), area 1's "Order of Mages"
//! two-part investigation chain (`QLOG` indices 7 and 8).
//!
//! Ports `src/area/1/gwendylon.c::guiwynn_driver` (`:4546-4889`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for every other
//! area-1 NPC file). Follows the same `World`/`PlayerRuntime` split
//! established there: the caller supplies a per-player fact snapshot
//! ([`GuiwynnPlayerFacts`]) up front and applies the returned
//! [`GuiwynnOutcomeEvent`]s afterwards, since `guiwynn_state`/
//! `guiwynn_seen_timer` (`area1_ppd` fields) and `QLOG` indices 7/8 live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - Both money rewards (`create_money_item(MONEY_AREA1_MADMAGE1/2)` +
//!   plain `give_char_item`, `gwendylon.c:4800-4805`/`4822-4827`) are a
//!   `World` cannot perform directly: unlike `world::gwendylon`'s skull
//!   rewards (which use `give_char_item_smart`'s `IF_MONEY` auto-gold-
//!   conversion branch), guiwynn calls plain `give_char_item` on the money
//!   item - a genuine C behavioral difference that leaves the reward as a
//!   literal carried "money" item object rather than converting it
//!   straight to `ch[cn].gold`. `World` also has no `ZoneLoader` template
//!   access to instantiate the "money" template in the first place. Both
//!   gaps mean this reward is entirely deferred to `ugaris-server`'s
//!   `apply_guiwynn_events`, gated on [`GuiwynnOutcomeEvent::QuestDone`]'s
//!   `times_done == 1` (C's `if (tmp == 1)`), mirroring the
//!   `complete_legacy`-returns-a-completion-summary pattern
//!   `world::lydia`/`world::gwendylon` already established for similar
//!   first-completion-only rewards.
//! - `destroy_item_byID(co, ID)` (`gwendylon.c:4794`, `4812-4815`) sweeps
//!   the player's equipment/inventory/cursor via
//!   [`World::destroy_items_by_template_id`] but not the account depot
//!   (`DRD_DEPOT_PPD`) - same documented gap as every other area-1 NPC's
//!   own `destroy_item_byID` sweep.
//! - The "This key opens the front door of the Order." line
//!   (`gwendylon.c:4663`, `4696`, `4724`) is spoken unconditionally
//!   whenever `!has_item(co, IID_AREA1_MADKEY1)` is true, *regardless* of
//!   whether the subsequent `give_char_item` call actually succeeds (C
//!   never checks the `create_item`/`give_char_item` result before
//!   speaking the line) - preserved here by always emitting the line and
//!   the [`GuiwynnOutcomeEvent::GrantKeyItem`] event together whenever the
//!   `has_item` check passes, not gating the line on the deferred grant's
//!   eventual success.
//! - C's own `guiwynn_driver` genuinely checks the trailing `NT_NPC`/
//!   `NTID_DIDSAY` self-throttle bump *twice* per tick: once in a leading
//!   pre-pass over the (not-yet-removed) message queue
//!   (`gwendylon.c:4563-4569`, same pre-pass every other area-1 NPC file
//!   has), and *again* inline in the main per-message loop right before
//!   `remove_message` (`gwendylon.c:4867-4869`) - a genuine duplicate
//!   check in the C source (harmless: it just re-assigns the same
//!   `ticker` value), not a copy-paste mistake in this port. Both checks
//!   are reproduced.
//! - The idle "return to post" gate uses a shorter `TICKS * 10` threshold
//!   (`gwendylon.c:4879`) than every other stationary area-1 NPC's own
//!   `TICKS * 30` (e.g. `world::terion`/`world::nook`/`world::lydia`), and
//!   moves toward `DX_UP` instead of `DX_RIGHT`/`DX_DOWN` - both preserved
//!   verbatim, not "fixed" to match the sibling NPCs' shape.
//! - No idle-mutterings table exists for this NPC in the C source (unlike
//!   `world::nook`/`world::lydia`) - confirmed, not a missed port.
//! - The `case 5`/`case 8` reminder lines wrap "repeat" in
//!   `COL_LIGHT_BLUE`/`COL_RESET` markers (`gwendylon.c:4670-4671`,
//!   `4700-4701`); restored via `COL_STR_LIGHT_BLUE`/`COL_STR_RESET`
//!   sentinels and `World::npc_quiet_say_bytes`, same mechanism as
//!   `world::camhermit`.

use std::collections::HashMap;

use crate::character_driver::{
    analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON, GWENDYLON_QA, NTID_DIDSAY, NTID_TERION,
};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_AREA1_MADKEY1;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;

/// C `char_dist(cn, co) > 16` (`gwendylon.c:4609`): the `NT_CHAR` greeting
/// range.
const GUIWYNN_GREET_DISTANCE: i32 = 16;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const GUIWYNN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gwendylon.c:4592`).
const GUIWYNN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gwendylon.c:4597`).
const GUIWYNN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 10` (`gwendylon.c:4879`): idle "return to post" threshold -
/// shorter than every other stationary area-1 NPC's own `TICKS * 30`, see
/// the module doc comment.
const GUIWYNN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `realtime - ppd->guiwynn_seen_timer > 120` (`gwendylon.c:4618`,
/// `4621`): the pre-`switch` auto-reset gate, shared by both reset `if`s.
const GUIWYNN_STATE_RESET_SECONDS: i32 = 120;
/// C `realtime - ppd->guiwynn_seen_timer > 60` (`gwendylon.c:4667`,
/// `4700`, `4728`): the reminder gate shared by states 5, 8, and 11.
const GUIWYNN_REMINDER_SECONDS: i32 = 60;
/// C `ppd->gwendy_state < 17` (`gwendylon.c:4626`): no named `#define`
/// exists in the C source for this threshold (one below
/// `GWENDYLON_STATE_FOUL_MAGICIAN_DONE == 18`), so it is named here purely
/// for readability.
const GWENDYLON_STATE_FOUL_MAGICIAN_WAIT: i32 = 17;

/// Per-player facts [`World::process_guiwynn_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiwynnPlayerFacts {
    /// `PlayerRuntime::area1_guiwynn_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_guiwynn_seen_timer()` (C `realtime` wall-clock
    /// seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
    /// `PlayerRuntime::area1_gwendy_state()`: gates state 0's quest offer.
    pub gwendy_state: i32,
    /// `PlayerRuntime::quest_log.is_done(8)` (C `questlog_isdone(co, 8)`):
    /// gates state 6's early-exit-to-11 branch.
    pub quest8_done: bool,
}

/// A side effect [`World::process_guiwynn_actions`] could not apply
/// directly because it touches `PlayerRuntime` (or, for
/// [`GuiwynnOutcomeEvent::QuestDone`]/[`GuiwynnOutcomeEvent::
/// GrantKeyItem`], needs `ZoneLoader`/quest-log access). See the module
/// doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiwynnOutcomeEvent {
    /// Write the new `area1_ppd.guiwynn_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->guiwynn_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message (`gwendylon.c:4734`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, 7)` (`gwendylon.c:4630`) or `questlog_open(co,
    /// 8)` (`gwendylon.c:4683`).
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `questlog_done(co, 7)` (`gwendylon.c:4791`) or `questlog_done(co,
    /// 8)` (`gwendylon.c:4810`) - the caller applies the exp reward, the
    /// questlog resend, and (only on first completion, C's `if (tmp ==
    /// 1)`) the `create_money_item`+plain `give_char_item` reward. See the
    /// module doc comment for why the money reward can't be resolved here.
    QuestDone {
        player_id: CharacterId,
        quest: usize,
    },
    /// C's `!has_item(co, IID_AREA1_MADKEY1)` + `create_item("mad_key1")`
    /// + plain `give_char_item` (`gwendylon.c:4658-4664`, `4691-4697`,
    ///   `4719-4725`) - deferred to `ugaris-server` since `World` has no
    ///   `ZoneLoader` template access.
    GrantKeyItem { player_id: CharacterId },
}

impl World {
    /// C `guiwynn_driver`'s per-tick body (`gwendylon.c:4546-4889`). `now`
    /// is C's wall-clock `realtime` (seconds).
    pub fn process_guiwynn_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, GuiwynnPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<GuiwynnOutcomeEvent> {
        let guiwynn_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_GUIWYNN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for guiwynn_id in guiwynn_ids {
            self.process_guiwynn_messages(guiwynn_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    #[allow(clippy::too_many_arguments)]
    fn process_guiwynn_messages(
        &mut self,
        guiwynn_id: CharacterId,
        player_facts: &HashMap<CharacterId, GuiwynnPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<GuiwynnOutcomeEvent>,
    ) {
        let Some(guiwynn_name) = self
            .characters
            .get(&guiwynn_id)
            .map(|guiwynn| guiwynn.name.clone())
        else {
            return;
        };
        let mut data = match self
            .characters
            .get(&guiwynn_id)
            .and_then(|guiwynn| guiwynn.driver_state.clone())
        {
            Some(CharacterDriverState::Guiwynn(data)) => data,
            _ => GuiwynnDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&guiwynn_id)
            .map(|guiwynn| std::mem::take(&mut guiwynn.driver_messages))
            .unwrap_or_default();

        // C's first pass over the (not-yet-removed) message queue
        // (`gwendylon.c:4563-4569`): any `NT_NPC`/`NTID_DIDSAY` broadcast
        // from someone else resets our own talk throttle to "just
        // talked".
        for message in &messages {
            if message.message_type == NT_NPC
                && message.dat1 == NTID_DIDSAY
                && message.dat2 != guiwynn_id.0 as i32
            {
                data.last_talk = self.tick.0;
            }
        }

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.guiwynn_handle_char_message(
                    guiwynn_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.guiwynn_handle_text_message(
                    guiwynn_id,
                    &guiwynn_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.guiwynn_handle_give_message(guiwynn_id, message, player_facts, events)
                }
                NT_NPC if message.dat1 == NTID_TERION => {
                    self.guiwynn_handle_terion_message(
                        guiwynn_id,
                        &mut data,
                        message,
                        &mut face_target,
                    );
                }
                _ => {}
            }

            // C's second, duplicate check of the same self-throttle bump
            // right before `remove_message` (`gwendylon.c:4867-4869`) -
            // see the module doc comment.
            if message.message_type == NT_NPC
                && message.dat1 == NTID_DIDSAY
                && message.dat2 != guiwynn_id.0 as i32
            {
                data.last_talk = self.tick.0;
            }
        }

        if let Some(guiwynn) = self.characters.get_mut(&guiwynn_id) {
            guiwynn.driver_state = Some(CharacterDriverState::Guiwynn(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:4876-4878`).
        if let (Some(guiwynn), Some((tx, ty))) =
            (self.characters.get(&guiwynn_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(guiwynn.x), i32::from(guiwynn.y), tx, ty) {
                if let Some(guiwynn_mut) = self.characters.get_mut(&guiwynn_id) {
                    let _ = turn(guiwynn_mut, direction as u8);
                }
            }
        }

        // C `if (ticker - dat->last_talk < TICKS*10) { do_idle(cn, TICKS);
        // return; } if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy,
        // DX_UP, ret, lastact)) return; do_idle(cn, TICKS);`
        // (`gwendylon.c:4879-4888`). The NPC's post position (C's
        // `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same substitution
        // every other stationary area-1 NPC uses.
        let last_talk = if let Some(guiwynn) = self.characters.get(&guiwynn_id) {
            match guiwynn.driver_state.as_ref() {
                Some(CharacterDriverState::Guiwynn(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if self.tick.0 < last_talk + GUIWYNN_RETURN_TO_POST_TICKS {
            return;
        }
        let Some(guiwynn) = self.characters.get(&guiwynn_id) else {
            return;
        };
        let (post_x, post_y) = (guiwynn.rest_x, guiwynn.rest_y);
        self.secure_move_driver(
            guiwynn_id,
            post_x,
            post_y,
            Direction::Up as u8,
            0,
            0,
            area_id,
        );
    }

    /// C `guiwynn_driver`'s `NT_CHAR` branch (`gwendylon.c:4576-4742`).
    #[allow(clippy::too_many_arguments)]
    fn guiwynn_handle_char_message(
        &mut self,
        guiwynn_id: CharacterId,
        data: &mut GuiwynnDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GuiwynnPlayerFacts>,
        now: i32,
        events: &mut Vec<GuiwynnOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(guiwynn) = self.characters.get(&guiwynn_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:4580-4583`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:4586-4589`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`gwendylon.c:4592-4595`).
        if tick < data.last_talk + GUIWYNN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`gwendylon.c:4597-
        // 4600`).
        if tick < data.last_talk + GUIWYNN_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:4603-4606`).
        if guiwynn_id == player_id
            || !char_see_char(&guiwynn, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 16) continue;` (`gwendylon.c:4609-
        // 4612`).
        if char_dist(&guiwynn, &player) > GUIWYNN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        // C `if (realtime - ppd->guiwynn_seen_timer > 120 &&
        // ppd->guiwynn_state && ppd->guiwynn_state <= 4) { ppd->guiwynn_state
        // = 1; }` / `if (realtime - ppd->guiwynn_seen_timer > 120 &&
        // ppd->guiwynn_state >= 6 && ppd->guiwynn_state <= 7) {
        // ppd->guiwynn_state = 6; }` (`gwendylon.c:4618-4623`).
        let mut state = facts.state;
        let seen_gap = now.saturating_sub(facts.seen_timer);
        if seen_gap > GUIWYNN_STATE_RESET_SECONDS && state > 0 && state <= 4 {
            state = 1;
        }
        if seen_gap > GUIWYNN_STATE_RESET_SECONDS && (6..=7).contains(&state) {
            state = 6;
        }

        let mut didsay = false;
        let mut new_state = state;

        match state {
            0 => {
                // C `case 0:` (`gwendylon.c:4625-4633`).
                if facts.gwendy_state >= GWENDYLON_STATE_FOUL_MAGICIAN_WAIT {
                    self.npc_quiet_say(
                        guiwynn_id,
                        &format!("Hello there, {}, please wait a moment.", player.name),
                    );
                    events.push(GuiwynnOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 7,
                    });
                    new_state = 1;
                    didsay = true;
                }
            }
            1 => {
                self.npc_quiet_say(
                    guiwynn_id,
                    &format!("I am {}, the town mage and I need your help.", guiwynn.name),
                );
                new_state = 2;
                didsay = true;
            }
            2 => {
                self.npc_quiet_say(
                    guiwynn_id,
                    "I used to teach at the Order of Mages. That is the huge building to the south-east of here. But when I came there, they attacked me.",
                );
                new_state = 3;
                didsay = true;
            }
            3 => {
                self.npc_quiet_say(
                    guiwynn_id,
                    "I could barely escape with my life. It seems they've all gone mad. I do not dare go back there, but I must know what is going on in the Order.",
                );
                new_state = 4;
                didsay = true;
            }
            4 => {
                self.npc_quiet_say(
                    guiwynn_id,
                    "If thou couldst go there and find out, I would reward thee. I am suspecting some kind of disease, or a magical attack, or some kind of poisoning, maybe with the help of an alchemistical potion. If thou findst anything out of the ordinary, bring it to me.",
                );
                new_state = 5;
                didsay = true;
                if !self.character_has_template_id(player_id, IID_AREA1_MADKEY1) {
                    events.push(GuiwynnOutcomeEvent::GrantKeyItem { player_id });
                    self.npc_quiet_say(guiwynn_id, "This key opens the front door of the Order.");
                }
            }
            5 => {
                // C `case 5:` (`gwendylon.c:4666-4674`).
                if now.saturating_sub(facts.seen_timer) > GUIWYNN_REMINDER_SECONDS {
                    self.npc_quiet_say_bytes(
                        guiwynn_id,
                        &format!(
                            "Be greeted, {}! Didst thou find out anything about the Order? Or dost thou want me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} mine offer?",
                            player.name
                        ),
                    );
                    didsay = true;
                }
            }
            6 => {
                // C `case 6:` (`gwendylon.c:4676-4686`).
                if facts.quest8_done {
                    new_state = 11;
                } else {
                    self.npc_quiet_say(
                        guiwynn_id,
                        "A Potion of Happiness? I have never heard of such a thing before. It does seem to induce madness in those who drink it. But alas, I cannot tell what it is made of.",
                    );
                    events.push(GuiwynnOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 8,
                    });
                    new_state = 7;
                    didsay = true;
                }
            }
            7 => {
                self.npc_quiet_say(
                    guiwynn_id,
                    "Couldst thou go back and try to find the recipe? I would double thine reward.",
                );
                new_state = 8;
                didsay = true;
                if !self.character_has_template_id(player_id, IID_AREA1_MADKEY1) {
                    events.push(GuiwynnOutcomeEvent::GrantKeyItem { player_id });
                    self.npc_quiet_say(guiwynn_id, "This key opens the front door of the Order.");
                }
            }
            8 => {
                // C `case 8:` (`gwendylon.c:4699-4707`).
                if now.saturating_sub(facts.seen_timer) > GUIWYNN_REMINDER_SECONDS {
                    self.npc_quiet_say_bytes(
                        guiwynn_id,
                        &format!(
                            "Be greeted, {}! Didst thou find the recipe? Or dost thou want me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} mine offer?",
                            player.name
                        ),
                    );
                    didsay = true;
                }
            }
            9 => {
                self.npc_quiet_say(
                    guiwynn_id,
                    "Oh, how sad. This madness is not curable. They will remain mad for the rest of their life. But at least I tried.",
                );
                new_state = 10;
                didsay = true;
            }
            10 => {
                self.npc_quiet_say(
                    guiwynn_id,
                    &format!(
                        "I thank thee, {}, for thine help. Mayest thou find happiness in thine life.",
                        player.name
                    ),
                );
                new_state = 11;
                didsay = true;
                if !self.character_has_template_id(player_id, IID_AREA1_MADKEY1) {
                    events.push(GuiwynnOutcomeEvent::GrantKeyItem { player_id });
                    self.npc_quiet_say(guiwynn_id, "This key opens the front door of the Order.");
                }
            }
            11
                // C `case 11:` (`gwendylon.c:4727-4732`).
                if now.saturating_sub(facts.seen_timer) > GUIWYNN_REMINDER_SECONDS => {
                    self.npc_quiet_say(guiwynn_id, &format!("Nice to see you, {}.", player.name));
                    didsay = true;
                }
            // Every other value (>= 12): no-op, matching C's `switch`
            // with no matching `case`.
            _ => {}
        }

        // C `ppd->guiwynn_seen_timer = realtime;` (`gwendylon.c:4734`):
        // unconditional.
        events.push(GuiwynnOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });
        if new_state != facts.state {
            events.push(GuiwynnOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; notify_area(..., NTID_DIDSAY, cn, 0);
        // }` (`gwendylon.c:4735-4740`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
            self.notify_area(
                guiwynn.x,
                guiwynn.y,
                NT_NPC,
                NTID_DIDSAY,
                guiwynn_id.0 as i32,
                0,
            );
        }
    }

    /// C `guiwynn_driver`'s `NT_TEXT` branch (`gwendylon.c:4745-4778`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as every other area-1 NPC's own text handler).
    #[allow(clippy::too_many_arguments)]
    fn guiwynn_handle_text_message(
        &mut self,
        guiwynn_id: CharacterId,
        guiwynn_name: &str,
        data: &mut GuiwynnDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GuiwynnPlayerFacts>,
        events: &mut Vec<GuiwynnOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let tick = self.tick.0;
        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:4748-4750`).
        if tick > data.last_talk + GUIWYNN_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:4752-4755`).
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
        if guiwynn_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(guiwynn) = self.characters.get(&guiwynn_id).cloned() else {
            return;
        };
        if char_dist(&guiwynn, &speaker) > GUIWYNN_QA_DISTANCE
            || !char_see_char(&guiwynn, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, guiwynn_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(guiwynn_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:4758-4772`): three disjoint `if`s,
            // each resetting `guiwynn_state` back to a checkpoint - safe
            // since the ranges (0-5, 6-8, 9-11) partition the full state
            // space without overlap, so at most one applies.
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    let reset_state = if facts.state <= 5 {
                        Some(0)
                    } else if (6..=8).contains(&facts.state) {
                        Some(6)
                    } else if (9..=11).contains(&facts.state) {
                        Some(9)
                    } else {
                        None
                    };
                    if let Some(new_state) = reset_state {
                        data.last_talk = 0;
                        events.push(GuiwynnOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by guiwynn's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:4774-4777`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2` branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `guiwynn_driver`'s `NT_GIVE` branch (`gwendylon.c:4781-4844`).
    fn guiwynn_handle_give_message(
        &mut self,
        guiwynn_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GuiwynnPlayerFacts>,
        events: &mut Vec<GuiwynnOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&guiwynn_id)
            .and_then(|guiwynn| guiwynn.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let facts = player_facts.get(&giver_id).copied();
        let giver_name = self
            .characters
            .get(&giver_id)
            .map(|giver| giver.name.clone())
            .unwrap_or_default();

        if template_id == crate::item_driver::IID_AREA1_MADPOTION
            && facts.is_some_and(|facts| facts.state <= 5)
        {
            // C `if (it[in].ID == IID_AREA1_MADPOTION && ppd &&
            // ppd->guiwynn_state <= 5)` (`gwendylon.c:4788-4805`).
            self.npc_quiet_say(
                guiwynn_id,
                &format!("Ahh, yes, that might be what was looking for. Thank thee, {giver_name}."),
            );
            events.push(GuiwynnOutcomeEvent::QuestDone {
                player_id: giver_id,
                quest: 7,
            });
            self.destroy_items_by_template_id(giver_id, crate::item_driver::IID_AREA1_MADPOTION);
            self.destroy_items_by_template_id(giver_id, crate::item_driver::IID_AREA1_MADKEY2);
            events.push(GuiwynnOutcomeEvent::UpdateState {
                player_id: giver_id,
                new_state: 6,
            });
            // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;`
            // (`gwendylon.c:4797-4798`).
            self.destroy_item(item_id);
        } else if template_id == crate::item_driver::IID_AREA1_MADNOTE
            && facts.is_some_and(|facts| (6..=8).contains(&facts.state))
        {
            // C `else if (it[in].ID == IID_AREA1_MADNOTE && ppd &&
            // ppd->guiwynn_state >= 6 && ppd->guiwynn_state <= 8)`
            // (`gwendylon.c:4806-4827`).
            self.npc_quiet_say(
                guiwynn_id,
                &format!(
                    "Ahh, yes, this is the recipe I was looking for. Thank thee, {giver_name}."
                ),
            );
            events.push(GuiwynnOutcomeEvent::QuestDone {
                player_id: giver_id,
                quest: 8,
            });
            self.destroy_items_by_template_id(giver_id, crate::item_driver::IID_AREA1_MADNOTE);
            self.destroy_items_by_template_id(giver_id, crate::item_driver::IID_AREA1_MADKEY2);
            self.destroy_items_by_template_id(giver_id, crate::item_driver::IID_AREA1_MADKEY3);
            self.destroy_items_by_template_id(giver_id, crate::item_driver::IID_AREA1_MADKEY4);
            self.destroy_items_by_template_id(giver_id, crate::item_driver::IID_AREA1_MADKEY5);
            events.push(GuiwynnOutcomeEvent::UpdateState {
                player_id: giver_id,
                new_state: 10,
            });
            // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;`
            // (`gwendylon.c:4819-4820`).
            self.destroy_item(item_id);
        } else {
            // C `else { quiet_say(...); if (!give_char_item(co,
            // ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].citem =
            // 0; }` (`gwendylon.c:4836-4842`) - the plain `give_char_item`,
            // not `give_char_item_smart` (same documented asymmetry as
            // every other area-1 NPC's own `NT_GIVE` handler).
            self.npc_quiet_say(
                guiwynn_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }

    /// C `guiwynn_driver`'s `NT_NPC`/`NTID_TERION` relay branch
    /// (`gwendylon.c:4846-4865`): terion's own stone-circle/yoakin-ruins
    /// broadcasts (`dat3 == 1`/`dat3 == 4`) prompt an ambient reply here,
    /// the first of which re-broadcasts `NTID_TERION` with `dat3 == 2`
    /// (picked up further down the relay chain, not by this file).
    fn guiwynn_handle_terion_message(
        &mut self,
        guiwynn_id: CharacterId,
        data: &mut GuiwynnDriverData,
        message: &CharacterDriverMessage,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let co_id = CharacterId(message.dat2.max(0) as u32);
        let Some(guiwynn) = self.characters.get(&guiwynn_id).cloned() else {
            return;
        };
        let Some(co) = self.characters.get(&co_id).cloned() else {
            return;
        };

        if message.dat3 == 1 {
            // C `if (msg->dat3 == 1)` (`gwendylon.c:4849-4855`).
            self.npc_quiet_say(
                guiwynn_id,
                "Yeah, my lad went with them. Fools, to seek the danger.",
            );
            self.notify_area(
                guiwynn.x,
                guiwynn.y,
                NT_NPC,
                NTID_TERION,
                guiwynn_id.0 as i32,
                2,
            );
            *face_target = Some((i32::from(co.x), i32::from(co.y)));
            data.last_talk = self.tick.0;
        }
        if message.dat3 == 4 {
            // C `if (msg->dat3 == 4)` (`gwendylon.c:4856-4863`).
            self.npc_quiet_say(
                guiwynn_id,
                "Yes, he's been here drinking a lot a few weeks ago. Told us that the floor in his back room collapsed, and that he was having bad dreams for several nights onwards.",
            );
            *face_target = Some((i32::from(co.x), i32::from(co.y)));
            data.last_talk = self.tick.0;
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct guiwynn_driver_data` (`src/area/1/gwendylon.c:4546-4550`): the
/// town-mage NPC's own driver memory (`CDR_GUIWYNN`, distinct from the
/// per-player `guiwynn_state`/`guiwynn_seen_timer` fields in
/// `crate::player::PlayerRuntime`'s `area1_ppd` - see `world::guiwynn`'s
/// module doc comment for the split). C's own `nighttime` field is never
/// read or written anywhere in `guiwynn_driver`'s body - dead even in C -
/// so it is not ported.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GuiwynnDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
