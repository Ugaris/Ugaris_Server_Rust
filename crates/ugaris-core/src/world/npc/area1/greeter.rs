//! Town-greeter NPC (`CDR_GREETER`), area 1's tutorial welcome/civics
//! dialogue at the stronghold (Cameron, the town's Governor).
//!
//! Ports `src/area/1/gwendylon.c::greeter_driver` (`:1485-1798`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for
//! `world::camhermit`/`world::yoakin`/`world::terion`). Follows the same
//! `World`/`PlayerRuntime` split established there: the caller supplies a
//! per-player fact snapshot ([`GreeterPlayerFacts`]) up front and applies
//! the returned [`GreeterOutcomeEvent`]s afterwards, since
//! `greeter_state`/`greeter_seen_timer`/`james_state` (all `area1_ppd`
//! fields) and the `QLOG_LYDIA` quest-done flag live on
//! `crate::player::PlayerRuntime`, not `World`. Unlike `world::yoakin`'s
//! per-player state, no quest log is ever *written* here - greeter only
//! reads `QLOG_LYDIA`'s completion flag (set elsewhere, by the Lydia
//! tutorial quest driver) to decide when to stop nagging about James.
//!
//! Deviations/gaps (documented, not silent):
//! - The C `case 6`/`case 12`/`case 13` lines wrap "learn"/"repeat" in
//!   `COL_LIGHT_BLUE`/`COL_RESET` markers (`gwendylon.c:1633-1634,1686-
//!   1687,1691-1692,1706-1707`); restored via `COL_STR_LIGHT_BLUE`/
//!   `COL_STR_RESET` sentinels and `World::npc_quiet_say_bytes`, same
//!   mechanism as `world::camhermit`.
//! - `world::greeter`'s `NT_TEXT` handler follows `world::yoakin`'s
//!   `current_victim`/`last_talk` reset-then-gate shape (`gwendylon.c:
//!   1734-1741`), not `world::terion`'s ungated one - a genuine
//!   asymmetry between the C drivers, preserved here rather than
//!   unified.
use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_GREETER, GWENDYLON_QA};
use crate::drvlib::offset2dx;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;

/// C `char_dist(cn, co) > 10` (`gwendylon.c:1534`): the `NT_CHAR` greeting
/// range.
const GREETER_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const GREETER_QA_DISTANCE: i32 = 12;
/// C `TICKS * 10` (`gwendylon.c:1517`).
const GREETER_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 20` (`gwendylon.c:1522`, also reused by the `NT_TEXT`
/// handler's `current_victim` reset at `gwendylon.c:1734`).
const GREETER_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 20;
/// C `TICKS * 30` (`gwendylon.c:1791`): idle "return to post" threshold.
const GREETER_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ch[co].level > 7` (`gwendylon.c:1575,1649`): the weapon-tutorial/
/// rest-area-tutorial skip gate for high-level characters.
const GREETER_TUTORIAL_LEVEL_CAP: u32 = 7;
/// C `realtime - ppd->greeter_seen_timer > 60` (`gwendylon.c:1704`): the
/// state-13 reminder window.
const GREETER_STATE13_REMINDER_SECONDS: i32 = 60;

/// C's bare `int` state values for `ppd->greeter_state` - no `#define`
/// names exist in the C source, so these are named here purely for
/// readability.
const GREETER_STATE_ENTRY: i32 = 0;
const GREETER_STATE_WEAPON_INTRO: i32 = 1;
const GREETER_STATE_SMALL_BLADE: i32 = 2;
const GREETER_STATE_TWO_HANDED: i32 = 3;
const GREETER_STATE_FISTS: i32 = 4;
const GREETER_STATE_WEAPON_OUTRO: i32 = 5;
const GREETER_STATE_LEARN_PROMPT: i32 = 6;
const GREETER_STATE_EMPTY: i32 = 7;
const GREETER_STATE_REST_AREA: i32 = 8;
const GREETER_STATE_RECALL_SCROLLS: i32 = 9;
const GREETER_STATE_MOVEMENT: i32 = 10;
const GREETER_STATE_LOOK_GROUND: i32 = 11;
const GREETER_STATE_UNDERSTAND_PROMPT: i32 = 12;
const GREETER_STATE_REMINDER: i32 = 13;
const GREETER_STATE_DONE: i32 = 14;

/// Per-player facts [`World::process_greeter_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GreeterPlayerFacts {
    /// `PlayerRuntime::area1_greeter_state()`.
    pub state: i32,
    /// `PlayerRuntime::area1_greeter_seen_timer()` (C `realtime` wall-clock
    /// seconds at last processed `NT_CHAR`).
    pub seen_timer: i32,
    /// `PlayerRuntime::area1_james_state()`: gates the state 6 "James
    /// needs help" reminder sentence.
    pub james_state: i32,
    /// `PlayerRuntime::quest_log.is_done(QLOG_LYDIA)` (C `questlog_isdone
    /// (co, QLOG_LYDIA)`, `gwendylon.c:1684,1700`): gates the state
    /// 12/13 -> 14 "stop nagging" transitions.
    pub lydia_quest_done: bool,
}

/// A side effect [`World::process_greeter_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreeterOutcomeEvent {
    /// Write the new `area1_ppd.greeter_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->greeter_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message (`gwendylon.c:1720`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
}

impl World {
    /// C `greeter_driver`'s per-tick body (`gwendylon.c:1485-1798`). `now`
    /// is C's wall-clock `realtime` (seconds).
    pub fn process_greeter_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, GreeterPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<GreeterOutcomeEvent> {
        let greeter_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_GREETER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for greeter_id in greeter_ids {
            self.process_greeter_messages(greeter_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_greeter_messages(
        &mut self,
        greeter_id: CharacterId,
        player_facts: &HashMap<CharacterId, GreeterPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<GreeterOutcomeEvent>,
    ) {
        let Some(greeter_name) = self
            .characters
            .get(&greeter_id)
            .map(|greeter| greeter.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Greeter(mut data)) = self
            .characters
            .get(&greeter_id)
            .and_then(|greeter| greeter.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&greeter_id)
            .map(|greeter| std::mem::take(&mut greeter.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.greeter_handle_char_message(
                    greeter_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.greeter_handle_text_message(
                    greeter_id,
                    &greeter_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.greeter_handle_give_message(greeter_id, message),
                _ => {}
            }
        }

        if let Some(greeter) = self.characters.get_mut(&greeter_id) {
            greeter.driver_state = Some(CharacterDriverState::Greeter(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:1787-1789`).
        if let (Some(greeter), Some((tx, ty))) =
            (self.characters.get(&greeter_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(greeter.x), i32::from(greeter.y), tx, ty) {
                if let Some(greeter_mut) = self.characters.get_mut(&greeter_id) {
                    let _ = turn(greeter_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; } do_idle(cn, TICKS);` (`gwendylon.c:1791-
        // 1797`). The NPC's post position (C's `tmpx`/`tmpy`) reuses
        // `rest_x`/`rest_y`, the same substitution `world::terion`/
        // `world::yoakin`/`world::camhermit` already use for other
        // stationary NPCs' spawn tiles.
        let last_talk = if let Some(greeter) = self.characters.get(&greeter_id) {
            match greeter.driver_state.as_ref() {
                Some(CharacterDriverState::Greeter(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + GREETER_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(greeter) = self.characters.get(&greeter_id) else {
                return;
            };
            let (post_x, post_y) = (greeter.rest_x, greeter.rest_y);
            self.secure_move_driver(
                greeter_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `greeter_driver`'s `NT_CHAR` branch (`gwendylon.c:1501-1728`).
    #[allow(clippy::too_many_arguments)]
    fn greeter_handle_char_message(
        &mut self,
        greeter_id: CharacterId,
        data: &mut GreeterDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GreeterPlayerFacts>,
        now: i32,
        events: &mut Vec<GreeterOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(greeter) = self.characters.get(&greeter_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:1505-1508`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:1511-1514`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*10) continue;`
        // (`gwendylon.c:1517-1520`).
        if tick < data.last_talk + GREETER_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*20 && dat->current_victim
        // != co) continue;` (`gwendylon.c:1522-1525`).
        if tick < data.last_talk + GREETER_TALK_VICTIM_TICKS
            && data.current_victim.map_or(0, |victim| victim.0) != player_id.0
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:1528-1531`).
        if greeter_id == player_id
            || !char_see_char(&greeter, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`gwendylon.c:1534-
        // 1537`).
        if char_dist(&greeter, &player) > GREETER_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let is_warrior = player.flags.contains(CharacterFlags::WARRIOR);
        let is_mage = player.flags.contains(CharacterFlags::MAGE);
        let warrior_only = is_warrior && !is_mage;
        let mage_only = is_mage && !is_warrior;
        let seyan_du = is_warrior && is_mage;

        let mut didsay = false;
        let mut new_state = facts.state;

        match facts.state {
            GREETER_STATE_ENTRY => {
                // C `case 0:` (`gwendylon.c:1544-1571`).
                if warrior_only {
                    self.npc_quiet_say(
                        greeter_id,
                        &format!(
                            "Hail {}! and welcome to the land of Astonia. I am {}, the Governor of this town, Cameron. I see thou art a mighty Warrior.",
                            player.name, greeter.name
                        ),
                    );
                    new_state = GREETER_STATE_WEAPON_INTRO;
                    didsay = true;
                } else if mage_only {
                    self.npc_quiet_say(
                        greeter_id,
                        &format!(
                            "Hail {}! and welcome to land of Astonia. I am {}, the Governor of this town, Cameron. I see thou art a wise Mage.",
                            player.name, greeter.name
                        ),
                    );
                    new_state = GREETER_STATE_WEAPON_INTRO;
                    didsay = true;
                } else if seyan_du {
                    self.npc_quiet_say(
                        greeter_id,
                        &format!(
                            "Hail thee {}! I have not seen a Seyan'Du in quite some time! Thou most certainly possess more knowledge than me, I shall save my ranting for another traveler! Fare thee well {}.",
                            player.name, player.name
                        ),
                    );
                    new_state = GREETER_STATE_DONE;
                    didsay = true;
                }
            }
            GREETER_STATE_WEAPON_INTRO => {
                // C `case 1:` (`gwendylon.c:1573-1582`).
                if player.level > GREETER_TUTORIAL_LEVEL_CAP {
                    new_state = GREETER_STATE_UNDERSTAND_PROMPT;
                } else {
                    self.npc_quiet_say(
                        greeter_id,
                        "Before thine journey begins, I advise choosing which weapon thou shalt favor.",
                    );
                    new_state = GREETER_STATE_SMALL_BLADE;
                    didsay = true;
                }
            }
            GREETER_STATE_SMALL_BLADE => {
                // C `case 2:` (`gwendylon.c:1584-1599`).
                if warrior_only {
                    self.npc_quiet_say(
                        greeter_id,
                        "A sword is a small blade. This allows thee to carry a torch to light up in dark places.",
                    );
                    new_state = GREETER_STATE_TWO_HANDED;
                    didsay = true;
                } else if mage_only {
                    self.npc_quiet_say(
                        greeter_id,
                        "A dagger is a small blade. This allows thee to carry a torch to light up in dark places.",
                    );
                    new_state = GREETER_STATE_TWO_HANDED;
                    didsay = true;
                }
            }
            GREETER_STATE_TWO_HANDED => {
                // C `case 3:` (`gwendylon.c:1601-1616`).
                if warrior_only {
                    self.npc_quiet_say(
                        greeter_id,
                        "Thou canst also choose to use a two-handed sword. This does slightly more damage, but does not leave a hand to spare for carrying torches.",
                    );
                    new_state = GREETER_STATE_FISTS;
                    didsay = true;
                } else if mage_only {
                    self.npc_quiet_say(
                        greeter_id,
                        "Thou canst also use a Staff. This does slightly more damage, but does not leave a hand to spare for carrying torches",
                    );
                    new_state = GREETER_STATE_FISTS;
                    didsay = true;
                }
            }
            GREETER_STATE_FISTS => {
                // C `case 4:` (`gwendylon.c:1618-1624`).
                self.npc_quiet_say(
                    greeter_id,
                    "A third option is to fight with thine fists. Without any weapon a pugilist does less damage, but may also parry more incoming hits and dispense experience more freely without a need to fulfil any weapon requirements.",
                );
                new_state = GREETER_STATE_WEAPON_OUTRO;
                didsay = true;
            }
            GREETER_STATE_WEAPON_OUTRO => {
                // C `case 5:` (`gwendylon.c:1626-1630`).
                self.npc_quiet_say(
                    greeter_id,
                    "May the weapon path thou chooseth bring thee fortune. Each has its merits!",
                );
                new_state = GREETER_STATE_LEARN_PROMPT;
                didsay = true;
            }
            GREETER_STATE_LEARN_PROMPT => {
                // C `case 6:` (`gwendylon.c:1632-1642`).
                self.npc_quiet_say_bytes(
                    greeter_id,
                    &format!(
                        "If thee wishes to learn more about the basics of gameplay, say {COL_STR_LIGHT_BLUE}learn{COL_STR_RESET}."
                    ),
                );
                if facts.james_state == 0 {
                    self.npc_quiet_say(
                        greeter_id,
                        "Otherwise, I hear the poor James whimpering, go north-east, I'm sure he could use the assistance of a hero to be.",
                    );
                }
                new_state = GREETER_STATE_REMINDER;
                didsay = true;
            }
            GREETER_STATE_EMPTY => {
                // C `case 7:` (`gwendylon.c:1644-1646`): empty case.
            }
            GREETER_STATE_REST_AREA => {
                // C `case 8:` (`gwendylon.c:1648-1658`).
                if player.level > GREETER_TUTORIAL_LEVEL_CAP {
                    new_state = GREETER_STATE_UNDERSTAND_PROMPT;
                } else {
                    self.npc_quiet_say(
                        greeter_id,
                        "The blue square on which thou art standing is a rest area. Thou can not be attacked while standing here, and\t if thou wishes to leave the game or use a scroll of Recall thou shall return to the last blue square that thou hast stood upon.",
                    );
                    new_state = GREETER_STATE_RECALL_SCROLLS;
                    didsay = true;
                }
            }
            GREETER_STATE_RECALL_SCROLLS => {
                // C `case 9:` (`gwendylon.c:1660-1665`).
                self.npc_quiet_say(
                    greeter_id,
                    "Recall scrolls can be purchased at the nearby shop. I would suggest carrying one with thee at all times as they are sure to save thee from certain peril.",
                );
                new_state = GREETER_STATE_MOVEMENT;
                didsay = true;
            }
            GREETER_STATE_MOVEMENT => {
                // C `case 10:` (`gwendylon.c:1667-1673`).
                self.npc_quiet_say(
                    greeter_id,
                    "To move, left-click anywhere on the screen, and your character will move there, given it is possible. Sometimes, your character cannot move to the position you've clicked on, and nothing will happen.",
                );
                new_state = GREETER_STATE_LOOK_GROUND;
                didsay = true;
            }
            GREETER_STATE_LOOK_GROUND => {
                // C `case 11:` (`gwendylon.c:1675-1681`).
                self.npc_quiet_say(
                    greeter_id,
                    "Right clicking on the ground will look at the ground, telling you what area you are in, the specific location, and in dangerous areas, it will also tell you if the area is dangerous for you or just right for you.",
                );
                new_state = GREETER_STATE_UNDERSTAND_PROMPT;
                didsay = true;
            }
            GREETER_STATE_UNDERSTAND_PROMPT => {
                // C `case 12:` (`gwendylon.c:1683-1697`).
                if facts.lydia_quest_done {
                    self.npc_quiet_say_bytes(
                        greeter_id,
                        &format!(
                            "Didst thou understand? Or dost thou want me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} mine words?"
                        ),
                    );
                    new_state = GREETER_STATE_DONE;
                } else {
                    self.npc_quiet_say_bytes(
                        greeter_id,
                        &format!(
                            "Didst thou understand? Or dost thou want me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} mine words? Otherwise, be off now. Like I said before. James is audibly in requirement of aid!"
                        ),
                    );
                    new_state = GREETER_STATE_REMINDER;
                }
                didsay = true;
            }
            GREETER_STATE_REMINDER => {
                // C `case 13:` (`gwendylon.c:1699-1714`).
                if facts.lydia_quest_done {
                    new_state = GREETER_STATE_DONE;
                } else if now - facts.seen_timer > GREETER_STATE13_REMINDER_SECONDS {
                    self.npc_quiet_say_bytes(
                        greeter_id,
                        &format!(
                            "Hail, {}! Didst thou understand? Or dost thou want me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} mine words? Otherwise, be off now. Like I said before. James is audibly in requirement of aid!",
                            player.name
                        ),
                    );
                    didsay = true;
                }
            }
            // C `case 14:` (`gwendylon.c:1716-1718`): no-op, don't talk
            // anymore, just react on repeat.
            _ => {}
        }

        if new_state != facts.state {
            events.push(GreeterOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }
        // C `ppd->greeter_seen_timer = realtime;` (`gwendylon.c:1720`):
        // unconditional once every gating check above has passed.
        events.push(GreeterOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:1722-1726`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `greeter_driver`'s `NT_TEXT` branch (`gwendylon.c:1731-1765`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as `world::camhermit`/`world::yoakin`/`world::terion`'s text
    /// handlers).
    fn greeter_handle_text_message(
        &mut self,
        greeter_id: CharacterId,
        greeter_name: &str,
        data: &mut GreeterDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GreeterPlayerFacts>,
        events: &mut Vec<GreeterOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*20 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:1734-1736`).
        let tick = self.tick.0;
        if tick > data.last_talk + GREETER_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:1738-1741`).
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
        if greeter_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(greeter) = self.characters.get(&greeter_id).cloned() else {
            return;
        };
        if char_dist(&greeter, &speaker) > GREETER_QA_DISTANCE
            || !char_see_char(&greeter, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, greeter_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(greeter_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:1744-1750`): `ppd->greeter_state
            // <= 14` is always true (14 is the max reachable state), so
            // this always resets to state 0.
            TextAnalysisOutcome::Matched(2) => {
                data.last_talk = 0;
                events.push(GreeterOutcomeEvent::UpdateState {
                    player_id: speaker_id,
                    new_state: GREETER_STATE_ENTRY,
                });
                didsay = true;
            }
            // C `case 12:` (`gwendylon.c:1752-1758`): only rewinds if the
            // player is exactly at the "empty" state 7 checkpoint or has
            // already passed the understand-prompt state 13.
            TextAnalysisOutcome::Matched(12) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    if facts.state == GREETER_STATE_EMPTY || facts.state >= GREETER_STATE_REMINDER {
                        data.last_talk = 0;
                        events.push(GreeterOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: GREETER_STATE_REST_AREA,
                        });
                    }
                }
                didsay = true;
            }
            // Every other matched code is unhandled by greeter's own C
            // `switch` (only meaningful to `gwendylon_driver`'s bigger
            // one) but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:1761-1764`) - note this does *not* touch
        // `dat->last_talk` (except inside the `case 2`/`case 12` branches
        // above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `greeter_driver`'s `NT_GIVE` branch (`gwendylon.c:1768-1779`).
    fn greeter_handle_give_message(
        &mut self,
        greeter_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&greeter_id)
            .and_then(|greeter| greeter.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            greeter_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        self.give_char_item_smart(giver_id, item_id, true);
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct greeter_driver_data` (`src/area/1/gwendylon.c`, just above
/// `greeter_driver` at `:1485`): the town-greeter NPC's own driver memory
/// (`CDR_GREETER`, distinct from the per-player `greeter_state`/
/// `greeter_seen_timer` fields in `crate::player::PlayerRuntime`'s
/// `area1_ppd` - see `world::greeter`'s module doc comment for the
/// split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GreeterDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
