//! Tavern-keeper/alchemy-turn-in NPC (`CDR_RESKIN`), area 1's bartender in
//! the Cameron tavern's hidden back room.
//!
//! Ports `src/area/1/gwendylon.c::reskin_driver` (`:4098-4417`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`/`world::terion`/`world::gwendylon`).
//! Follows the same `World`/`PlayerRuntime` split established there: the
//! caller supplies a per-player fact snapshot ([`ReskinPlayerFacts`]) up
//! front and applies the returned [`ReskinOutcomeEvent`]s afterwards,
//! since `reskin_state`/`reskin_seen_timer`/`reskin_got_bits` (all
//! `area1_ppd` fields), the `QLOG_RESKIN` (17) quest log entry, and the
//! `firstkill_ppd` bit for class 16 (the "Guild Master", `check_first_kill
//! (co, 16)`) all live on `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - `struct reskin_driver_data`'s `last_walk`/`pos` fields (`gwendylon.c:
//!   4100-4101`) are never read or written anywhere in `reskin_driver`'s
//!   body - dead even in C, same precedent as `world::terion`'s own
//!   `TerionDriverData` - so they are not ported.
//! - `case 7`'s `check_first_kill(co, 16)` branch (`gwendylon.c:4234-
//!   4241`) never sets `didsay = 1` even though it calls `quiet_say` - a
//!   genuine C asymmetry (every other dialogue-producing case in this
//!   `switch` does set it). Preserved as-is rather than "fixed", same
//!   precedent as `world::terion`'s own documented asymmetries: no
//!   `last_talk`/`current_victim`/`NTID_DIDSAY` broadcast fires for this
//!   turn even though a line was said.
//! - The `case 3` reminder line wraps "repeat" in `COL_LIGHT_BLUE`/
//!   `COL_RESET` markers in C (`gwendylon.c:4204`); dropped here for the
//!   same reason documented on `world::camhermit`'s module doc comment
//!   (`World::npc_quiet_say` broadcasts a plain UTF-8 `String`).
//! - The `NT_GIVE` branch's non-money item hand-back (`gwendylon.c:4369-
//!   4372,4378-4381`) uses C's plain `give_char_item` (hand-then-
//!   overflow-inventory, no drop-to-ground fallback); `World::
//!   give_char_item_smart` is used in its place, same substitution
//!   `world::terion`'s own `NT_GIVE` branch already established (that
//!   function's superset behavior - it additionally tries a ground drop
//!   before destroying - is a strict improvement over silently destroying
//!   an item C would have dropped, and never changes the common,
//!   non-full-inventory case).
//! - `check_first_kill(co, 16)`'s "class 16" argument is not named in the
//!   C source (no `#define` exists for it) - it is simply "whichever NPC
//!   template has `class == 16`", ported here as a bare per-player fact
//!   ([`ReskinPlayerFacts::killed_guild_master`],
//!   `PlayerRuntime::has_first_kill(16)`) without further interpretation.

use std::collections::HashMap;

use crate::character_driver::{
    analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON, CDR_RESKIN, GWENDYLON_QA, NTID_DIDSAY,
    NTID_TERION,
};
use crate::drvlib::offset2dx;
use crate::item_driver::{drdata, IID_ALCHEMY_INGREDIENT};
use crate::quest::GWENDYLON_STATE_FIRST_SKULL_DONE;
use crate::world::*;

/// C `char_dist(cn, co) > 16` (`gwendylon.c:4162`): the `NT_CHAR` greeting
/// range.
const RESKIN_GREET_DISTANCE: i32 = 16;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const RESKIN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gwendylon.c:4145`).
const RESKIN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gwendylon.c:4150`).
const RESKIN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:4410`): idle "return to post" threshold.
const RESKIN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `realtime - ppd->reskin_seen_timer > 600` (`gwendylon.c:4201`):
/// seconds, not ticks (`realtime` is a Unix timestamp).
const RESKIN_SEEN_REMINDER_SECONDS: i32 = 600;
/// C `ppd->terion_state < 4` (`gwendylon.c:4176`) - named here to match
/// `world::terion`'s own `TERION_STATE_HORDES_GREET`, the state Terion's
/// own skelly-story intro finishes at.
const TERION_STATE_HORDES_GREET: i32 = 4;
/// C `ppd->logain_state > 8` (`gwendylon.c:4207`) - no named constant
/// exists in the C source for this threshold.
const LOGAIN_STATE_RESKIN_UNLOCK: i32 = 8;
/// C's `0x1FFFFFE` bitmask (`gwendylon.c:4350`): all alchemy-ingredient
/// types 1-24 collected.
const RESKIN_ALL_INGREDIENTS_MASK: u32 = 0x1FFF_FFE;

/// Reskin's bare `int` state values for `ppd->reskin_state` - no
/// `#define` names exist in the C source, so these are named here purely
/// for readability.
const RESKIN_STATE_ENTRY: i32 = 0;
const RESKIN_STATE_BEER_SHORTAGE: i32 = 1;
const RESKIN_STATE_ASK_INGREDIENTS: i32 = 2;
const RESKIN_STATE_WAIT_INGREDIENTS: i32 = 3;
const RESKIN_STATE_FAVOR_ASK: i32 = 4;
const RESKIN_STATE_NEW_MASTER: i32 = 5;
const RESKIN_STATE_ENTRANCE_HINT: i32 = 6;
const RESKIN_STATE_WAIT_GUILDMASTER_TALK: i32 = 7;
const RESKIN_STATE_ALCHEMY_RECIPE: i32 = 8;
const RESKIN_STATE_DONE: i32 = 9;

/// Per-player facts [`World::process_reskin_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReskinPlayerFacts {
    /// `PlayerRuntime::area1_reskin_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_reskin_seen_timer()`: a Unix timestamp
    /// (`realtime`), not a tick count.
    pub seen_timer: i32,
    /// `PlayerRuntime::area1_gwendy_state()`: gates the state 0 -> 1
    /// transition.
    pub gwendy_state: i32,
    /// `PlayerRuntime::area1_terion_state()`: gates the state 0 -> 1
    /// transition.
    pub terion_state: i32,
    /// `PlayerRuntime::area1_logain_state()`: gates the state 3 -> 4
    /// transition.
    pub logain_state: i32,
    /// `PlayerRuntime::area1_reskin_got_bits()` as `u32`: the
    /// already-turned-in alchemy-ingredient-type bitmask.
    pub got_bits: u32,
    /// `PlayerRuntime::has_first_kill(16)` (`check_first_kill(co, 16)`,
    /// `gwendylon.c:4235`).
    pub killed_guild_master: bool,
}

/// A side effect [`World::process_reskin_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReskinOutcomeEvent {
    /// Write the new `area1_ppd.reskin_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// Write the new `area1_ppd.reskin_seen_timer` back.
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// Write the new `area1_ppd.reskin_got_bits` back.
    UpdateGotBits { player_id: CharacterId, value: i32 },
    /// C `questlog_open(co, 17)` (`gwendylon.c:4216`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 17)` (`gwendylon.c:4237`).
    QuestDone { player_id: CharacterId },
    /// C `give_money`'s wealth-achievement half - see `world::camhermit`'s
    /// module doc comment for why this is a separate event.
    GoldEarned { player_id: CharacterId, amount: u32 },
    /// C `achievement_award(co, ACHIEVEMENT_WELL_PAID_GATHERER, 1)`
    /// (`gwendylon.c:4351`).
    WellPaidGathererAchievement { player_id: CharacterId },
}

impl World {
    /// C `reskin_driver`'s per-tick body (`gwendylon.c:4105-4417`).
    pub fn process_reskin_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ReskinPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<ReskinOutcomeEvent> {
        let reskin_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_RESKIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for reskin_id in reskin_ids {
            self.process_reskin_messages(reskin_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    #[allow(clippy::too_many_arguments)]
    fn process_reskin_messages(
        &mut self,
        reskin_id: CharacterId,
        player_facts: &HashMap<CharacterId, ReskinPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<ReskinOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Reskin(mut data)) = self
            .characters
            .get(&reskin_id)
            .and_then(|reskin| reskin.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&reskin_id)
            .map(|reskin| std::mem::take(&mut reskin.driver_messages))
            .unwrap_or_default();

        // C's first pass over the (not-yet-removed) message queue
        // (`gwendylon.c:4116-4122`): any `NT_NPC`/`NTID_DIDSAY` broadcast
        // from someone else resets our own talk throttle to "just
        // talked".
        for message in &messages {
            if message.message_type == NT_NPC
                && message.dat1 == NTID_DIDSAY
                && message.dat2 != reskin_id.0 as i32
            {
                data.last_talk = self.tick.0;
            }
        }

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.reskin_handle_char_message(
                    reskin_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.reskin_handle_text_message(
                    reskin_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.reskin_handle_give_message(reskin_id, message, player_facts, events)
                }
                NT_NPC => {
                    self.reskin_handle_npc_message(reskin_id, &mut data, message, &mut face_target)
                }
                _ => {}
            }
        }

        if let Some(reskin) = self.characters.get_mut(&reskin_id) {
            reskin.driver_state = Some(CharacterDriverState::Reskin(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:4406-4408`).
        if let (Some(reskin), Some((tx, ty))) =
            (self.characters.get(&reskin_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(reskin.x), i32::from(reskin.y), tx, ty) {
                if let Some(reskin_mut) = self.characters.get_mut(&reskin_id) {
                    let _ = turn(reskin_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; } do_idle(cn, TICKS);` (`gwendylon.c:4410-
        // 4416`). The NPC's post position (C's `tmpx`/`tmpy`) reuses
        // `rest_x`/`rest_y`, the same substitution `world::terion`/
        // `world::camhermit`/`world::yoakin` already use for other
        // stationary NPCs' spawn tiles.
        let last_talk = if let Some(reskin) = self.characters.get(&reskin_id) {
            match reskin.driver_state.as_ref() {
                Some(CharacterDriverState::Reskin(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + RESKIN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(reskin) = self.characters.get(&reskin_id) else {
                return;
            };
            let (post_x, post_y) = (reskin.rest_x, reskin.rest_y);
            self.secure_move_driver(
                reskin_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
        let _ = now;
    }

    /// C `reskin_driver`'s `NT_CHAR` branch (`gwendylon.c:4128-4264`).
    #[allow(clippy::too_many_arguments)]
    fn reskin_handle_char_message(
        &mut self,
        reskin_id: CharacterId,
        data: &mut ReskinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ReskinPlayerFacts>,
        now: i32,
        events: &mut Vec<ReskinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(reskin) = self.characters.get(&reskin_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:4132-4136`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:4138-4142`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`gwendylon.c:4144-4148`).
        if tick < data.last_talk + RESKIN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`gwendylon.c:4150-4153`).
        if tick < data.last_talk + RESKIN_TALK_VICTIM_TICKS
            && data.current_victim.map_or(0, |victim| victim.0) != player_id.0
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:4155-4159`).
        if reskin_id == player_id || !char_see_char(&reskin, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 16) continue;` (`gwendylon.c:4161-
        // 4165`).
        if char_dist(&reskin, &player) > RESKIN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.state;

        match facts.state {
            RESKIN_STATE_ENTRY => {
                // C `case 0:` (`gwendylon.c:4172-4183`).
                if facts.gwendy_state < GWENDYLON_STATE_FIRST_SKULL_DONE {
                    // dont offer alchemy quests before skull 2 - silent.
                } else if facts.terion_state < TERION_STATE_HORDES_GREET {
                    // wait for terion to finish skelly2 story - silent.
                } else {
                    self.npc_quiet_say(
                        reskin_id,
                        &format!(
                            "Hello, {}! I am {}, the bartender.",
                            player.name, reskin.name
                        ),
                    );
                    new_state = RESKIN_STATE_BEER_SHORTAGE;
                    didsay = true;
                }
            }
            RESKIN_STATE_BEER_SHORTAGE => {
                self.npc_quiet_say(
                    reskin_id,
                    &format!(
                        "We have a shortage of beer at the moment, {}, so I'm spending some of my time doing alchemistical studies. So far I've found out that only a potion brewed of at least one flower, one berry and one mushroom will have any effect.",
                        player.name
                    ),
                );
                new_state = RESKIN_STATE_ASK_INGREDIENTS;
                didsay = true;
            }
            RESKIN_STATE_ASK_INGREDIENTS => {
                self.npc_quiet_say(
                    reskin_id,
                    "In spite of the beer shortage, people still visit my tavern, so I can't go out to find ingredients as often as I'd like. If thou happenst to come across any new flower, berry or mushroom and bring it to me, I'd pay thee handsomely.",
                );
                new_state = RESKIN_STATE_WAIT_INGREDIENTS;
                didsay = true;
            }
            RESKIN_STATE_WAIT_INGREDIENTS => {
                // C `case 3:` (`gwendylon.c:4200-4210`).
                if now.saturating_sub(facts.seen_timer) > RESKIN_SEEN_REMINDER_SECONDS {
                    self.npc_quiet_say(
                        reskin_id,
                        &format!(
                            "Hello again, {}! Didst thou find any new ingredients? Or dost thou want me to repeat mine offer?",
                            player.name
                        ),
                    );
                    didsay = true;
                } else if facts.logain_state > LOGAIN_STATE_RESKIN_UNLOCK {
                    new_state = RESKIN_STATE_FAVOR_ASK;
                }
            }
            RESKIN_STATE_FAVOR_ASK => {
                self.npc_quiet_say(
                    reskin_id,
                    &format!(
                        "Oh, {}, couldst thou do me another favor? I've rented the back room to some, uh, not so respectable members of the society, and they are giving me trouble.",
                        player.name
                    ),
                );
                events.push(ReskinOutcomeEvent::QuestOpen { player_id });
                new_state = RESKIN_STATE_NEW_MASTER;
                didsay = true;
            }
            RESKIN_STATE_NEW_MASTER => {
                self.npc_quiet_say(
                    reskin_id,
                    "A few days ago, a new master took over their, uh, organization, and he's been threatening me. If thou couldst talk to him, I'd appreciate it. I'd even give thee a nice reward.",
                );
                new_state = RESKIN_STATE_ENTRANCE_HINT;
                didsay = true;
            }
            RESKIN_STATE_ENTRANCE_HINT => {
                self.npc_quiet_say(
                    reskin_id,
                    "Thou canst find them by going through that hidden entrance there, in the western corner of the room, between these two barrels. To open it, use the bear head hanging on the wall.",
                );
                new_state = RESKIN_STATE_WAIT_GUILDMASTER_TALK;
                didsay = true;
            }
            RESKIN_STATE_WAIT_GUILDMASTER_TALK => {
                // C `case 7:` (`gwendylon.c:4234-4241`): note `didsay` is
                // never set here even though a line is said - see the
                // module doc comment.
                if facts.killed_guild_master {
                    self.npc_quiet_say(
                        reskin_id,
                        &format!(
                            "Oh, thank you for talking to the Guild Master, {}.",
                            player.name
                        ),
                    );
                    events.push(ReskinOutcomeEvent::QuestDone { player_id });
                    new_state = RESKIN_STATE_ALCHEMY_RECIPE;
                }
            }
            RESKIN_STATE_ALCHEMY_RECIPE => {
                // C `case 8:` (`gwendylon.c:4242-4252`).
                if (player.flags & (CharacterFlags::WARRIOR | CharacterFlags::MAGE))
                    == CharacterFlags::WARRIOR
                {
                    self.npc_quiet_say(
                        reskin_id,
                        "Thou canst make a potion to raise thine abilities by using Adygalah, Chrysado, Domari, Beelough and any mushroom.",
                    );
                } else {
                    self.npc_quiet_say(
                        reskin_id,
                        "Thou canst make a potion to raise thine abilities by using two parts Elithah, one part Firuba, one part Beelough and any mushroom.",
                    );
                }
                new_state = RESKIN_STATE_DONE;
                didsay = true;
            }
            // Every other value (>= 10): no-op, matching C's `switch`
            // with no matching `case` (`case 9:` is also a no-op).
            _ => {}
        }

        if new_state != facts.state {
            events.push(ReskinOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }

        // C `ppd->reskin_seen_timer = realtime;` (`gwendylon.c:4256`,
        // unconditional inside `if (ppd)`).
        events.push(ReskinOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; notify_area(..., NTID_DIDSAY, cn, 0);
        // }` (`gwendylon.c:4257-4262`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
            self.notify_area(
                reskin.x,
                reskin.y,
                NT_NPC,
                NTID_DIDSAY,
                reskin_id.0 as i32,
                0,
            );
        }
    }

    /// C `reskin_driver`'s `NT_TEXT` branch (`gwendylon.c:4267-4291`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::camhermit`/`world::yoakin`/`world::terion`/
    /// `world::gwendylon`'s text handlers).
    fn reskin_handle_text_message(
        &mut self,
        reskin_id: CharacterId,
        data: &mut ReskinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ReskinPlayerFacts>,
        events: &mut Vec<ReskinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        let Some(reskin_name) = self
            .characters
            .get(&reskin_id)
            .map(|reskin| reskin.name.clone())
        else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`gwendylon.c:136-
        // 149`): ignore our own talk, non-players, distance > 12,
        // not-visible.
        if reskin_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(reskin) = self.characters.get(&reskin_id).cloned() else {
            return;
        };
        if char_dist(&reskin, &speaker) > RESKIN_QA_DISTANCE
            || !char_see_char(&reskin, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, &reskin_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(reskin_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:4272-4285`): three disjoint `if`s,
            // each resetting `reskin_state` back to a checkpoint and
            // zeroing `last_talk` - at most one applies since the ranges
            // don't overlap once evaluated in order (same precedent as
            // `world::terion`'s own four-range reset).
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    let reset_state = if facts.state <= RESKIN_STATE_FAVOR_ASK {
                        Some(RESKIN_STATE_ENTRY)
                    } else if (RESKIN_STATE_FAVOR_ASK..=RESKIN_STATE_WAIT_GUILDMASTER_TALK)
                        .contains(&facts.state)
                    {
                        Some(RESKIN_STATE_FAVOR_ASK)
                    } else if (RESKIN_STATE_ALCHEMY_RECIPE..=RESKIN_STATE_DONE)
                        .contains(&facts.state)
                    {
                        Some(RESKIN_STATE_ALCHEMY_RECIPE)
                    } else {
                        None
                    };
                    if let Some(new_state) = reset_state {
                        data.last_talk = 0;
                        events.push(ReskinOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by reskin's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:4287-4290`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2` branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `reskin_driver`'s `NT_GIVE` branch (`gwendylon.c:4294-4383`).
    fn reskin_handle_give_message(
        &mut self,
        reskin_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ReskinPlayerFacts>,
        events: &mut Vec<ReskinOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&reskin_id)
            .and_then(|reskin| reskin.cursor_item.take())
        else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            self.give_char_item_smart(giver_id, item_id, true);
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };

        // C `if (it[in].ID == IID_ALCHEMY_INGREDIENT)` (`gwendylon.c:
        // 4299`).
        if item.template_id == IID_ALCHEMY_INGREDIENT {
            let ingredient_type = u32::from(drdata(&item, 0));
            let bit = 1u32 << ingredient_type;
            let got_bits = player_facts
                .get(&giver_id)
                .map_or(0, |facts| facts.got_bits);

            // C `if (!(ppd->reskin_got_bits & bit))` (`gwendylon.c:4300`).
            if got_bits & bit == 0 {
                if reskin_alchemy_level_gate(ingredient_type, giver.level) {
                    // C's ten level-gated "cannot pay at the moment"
                    // branches (`gwendylon.c:4301-4340`) - falls through
                    // to the generic hand-back below (no `return` in C
                    // here).
                    let word = if matches!(ingredient_type, 21 | 22 | 23 | 24) {
                        "stone"
                    } else {
                        "mushroom"
                    };
                    self.npc_quiet_say(
                        reskin_id,
                        &format!(
                            "Oh, a very nice {word}, {}. But I'm afraid I cannot pay for it at the moment.",
                            giver.name
                        ),
                    );
                } else {
                    // C `else` payment branch (`gwendylon.c:4341-4364`).
                    let new_bits = got_bits | bit;
                    events.push(ReskinOutcomeEvent::UpdateGotBits {
                        player_id: giver_id,
                        value: new_bits as i32,
                    });
                    self.npc_quiet_say(
                        reskin_id,
                        &format!(
                            "Ah, a nice {} thou found there. Here, this is for thy trouble.",
                            item.name
                        ),
                    );
                    let gold_amount = item.value.saturating_mul(5);
                    if let Some(character) = self.characters.get_mut(&giver_id) {
                        character.gold = character.gold.saturating_add(gold_amount);
                        character.flags.insert(CharacterFlags::ITEMS);
                    }
                    self.queue_system_text_bytes(giver_id, give_money_message(gold_amount));
                    events.push(ReskinOutcomeEvent::GoldEarned {
                        player_id: giver_id,
                        amount: gold_amount,
                    });
                    if new_bits == RESKIN_ALL_INGREDIENTS_MASK {
                        events.push(ReskinOutcomeEvent::WellPaidGathererAchievement {
                            player_id: giver_id,
                        });
                    }
                    self.destroy_item(item_id);
                    return;
                }
            } else {
                // C `else` (already turned in) branch (`gwendylon.c:4366-
                // 4374`).
                self.npc_quiet_say(
                    reskin_id,
                    &format!(
                        "Oh, I'm sorry, {}, but thou brought me this one before.",
                        giver.name
                    ),
                );
                self.give_char_item_smart(giver_id, item_id, true);
                return;
            }
        }

        // C's generic hand-back fallback (`gwendylon.c:4376-4381`):
        // reached for non-alchemy items, and for alchemy items declined
        // by the level gate above.
        self.npc_quiet_say(
            reskin_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        self.give_char_item_smart(giver_id, item_id, true);
    }

    /// C `reskin_driver`'s `NT_NPC` branch (`gwendylon.c:4385-4394`): the
    /// `CDR_TERION` broadcast (`terion_driver`'s `TERION_STATE_RESKIN_BEER`
    /// case, `gwendylon.c:1360-1364`) asking about beer.
    fn reskin_handle_npc_message(
        &mut self,
        reskin_id: CharacterId,
        data: &mut ReskinDriverData,
        message: &CharacterDriverMessage,
        face_target: &mut Option<(i32, i32)>,
    ) {
        if message.dat1 != NTID_TERION || message.dat3 != 5 {
            return;
        }
        let terion_id = CharacterId(message.dat2.max(0) as u32);
        let Some(terion) = self.characters.get(&terion_id).cloned() else {
            return;
        };
        self.npc_quiet_say(
            reskin_id,
            "No Terion, no beer. But what thou sayst is true.",
        );
        *face_target = Some((i32::from(terion.x), i32::from(terion.y)));
        data.last_talk = self.tick.0;
    }
}

/// C's ten level-gated "cannot pay at the moment" `if`/`else if` branches
/// (`gwendylon.c:4301-4340`): each alchemy-ingredient `drdata[0]` type has
/// its own minimum-level threshold below which Reskin declines payment.
/// Types with no listed threshold (anything outside these ten values) are
/// never declined.
fn reskin_alchemy_level_gate(ingredient_type: u32, level: u32) -> bool {
    match ingredient_type {
        24 => level < 80,
        23 => level < 10,
        21 => level < 30,
        22 => level < 60,
        16 => level < 25,
        15 => level < 23,
        14 => level < 20,
        13 => level < 18,
        12 => level < 16,
        11 => level < 14,
        _ => false,
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct reskin_driver_data` (`src/area/1/gwendylon.c:4098-4103`): the
/// tavern-keeper NPC's own driver memory (`CDR_RESKIN`, distinct from the
/// per-player `reskin_state` field in `crate::player::PlayerRuntime`'s
/// `area1_ppd` - see `world::reskin`'s module doc comment for the split).
/// The C struct's `last_walk`/`pos` fields are never read or written
/// anywhere in `reskin_driver`'s body - dead even in C - so they are not
/// ported.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReskinDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
