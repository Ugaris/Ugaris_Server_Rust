//! Two-City forbidden-territory servants (`CDR_TWOSERVANT`) - the six
//! `palace_maid1`-`palace_maid5`/`city_maid1`-style NPCs guarding the
//! palace's illegal areas, distinguished by their `nr` (`0`-`5`) arg.
//!
//! Ports `src/area/17/two.c::servant` (`:980-1322`) plus its death hook
//! `servant_dead` (`:1324-1351`, in `ugaris-server::world_events::
//! death_hooks::apply_two_servant_death_from_hurt_event`, since it needs
//! `PlayerRuntime`).
//!
//! Unlike every other Two-City NPC ported so far, this driver's own state
//! (`current_state`/`current_victim`/`nr`/`lastalert`) is entirely
//! character-local (`struct servant_data`, keyed by the servant's own
//! character id in C) - it never reads or writes a player's
//! `twocity_ppd` except a read-only `citizen_status` check in the
//! `NT_CHAR` handler, so the caller only needs a minimal per-player fact
//! snapshot ([`TwoServantPlayerFacts`]), and there is no `TwoServantDriverData`
//! writeback event at all for state bookkeeping.
//!
//! `nr` (`0`: scullery girl, `1`: maid, `2`: the governor's mistress,
//! `3`: cook, `4`: the governor's double, `5`: another citizen) is parsed
//! from the zone file's `arg="nr=N;"` at spawn time (`zone.rs`, following
//! `guard_driver`'s precedent of moving C's own `NT_CREATE`-time `arg`
//! parse to spawn time - see [`parse_two_servant_driver_args`]).
//!
//! A real, deliberately-reproduced C quirk in the `NT_CHAR` handler: the
//! per-tick body runs *two* `switch (dat->current_state) { case 0: ...;
//! dat->current_state++; case 1: break; }` blocks back to back
//! (`two.c:1048-1065` then `:1069-1080`, the second one gated behind an
//! `illegal_place(...)` check). Because the first switch already
//! increments `current_state` from `0` to `1` before the second switch
//! reads it, the second switch's own `case 0` branch ("My greetings, ...
//! how may I serve you?") is unreachable on the very first encounter -
//! only the first switch's greeting ever fires. Ported as-is (see
//! [`World::two_servant_handle_char_message`]'s own comment), not
//! "fixed". A second, related quirk: `illegal_place(ch[cn].x, ch[co].x)`
//! (`two.c:1067`) passes the *player's* `x` twice (once as the servant's
//! own `x`, once mislabeled as the player's `y`) instead of the player's
//! real `(x, y)` - also preserved digit-for-digit.
//!
//! The `threaten`/`pay bribe` QA branches (answer_codes `10`/`11`) can
//! reward a secret-passage key (`palace_key1`/`palace_key2`, new
//! `IID_AREA17_PALACEKEY1`/`IID_AREA17_PALACEKEY2` constants,
//! `item_driver::ids`) that only `ugaris-server`'s `ZoneLoader` can
//! instantiate - like `alchemist`'s potion reward, those two cases defer
//! item creation via [`TwoServantOutcomeEvent::GivePalaceKey1`]/
//! [`TwoServantOutcomeEvent::GivePalaceKey2`]; the gold-taking (`take_
//! money`) and all dialogue happen directly inside `World`, since
//! `Character::gold` is visible there (same precedent as `world::npc::
//! area17::barkeeper`'s own private `take_money` copy).

use std::collections::HashMap;

use crate::character_driver::{
    analyse_text_qa, next_legacy_name_value, TextAnalysisOutcome, CDR_TWOSERVANT,
};
use crate::drvlib::offset2dx;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;

use super::{illegal_place, TWOCITY_QA};

/// C `char_dist(cn, co) > 10` (`two.c:1036`).
const TWO_SERVANT_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`two.c:1019`).
const TWO_SERVANT_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`two.c:1024`, `:1094`).
const TWO_SERVANT_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`two.c:1315`): idle "return to post" threshold.
const TWO_SERVANT_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS * 30` (`two.c:1298`): `NT_GOTHIT` alert cooldown.
const TWO_SERVANT_ALERT_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `take_money(co, 2000)` (`two.c:1221`, `:1268`).
const SERVANT_PAY_BRIBE_COST_SMALL: u32 = 2000;
/// C `take_money(co, 5000)` (`two.c:1233`, `:1260`).
const SERVANT_PAY_BRIBE_COST_LARGE: u32 = 5000;

/// C `struct servant_data` (`two.c:960-966`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoServantDriverData {
    pub last_talk_tick: u64,
    pub current_victim: Option<CharacterId>,
    pub current_state: i32,
    pub nr: i32,
    pub lastalert: u64,
}

/// Per-player facts [`World::process_two_servant_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoServantPlayerFacts {
    /// `PlayerRuntime::twocity_citizen_status()`.
    pub citizen_status: i32,
}

/// A side effect [`World::process_two_servant_actions`] could not apply
/// directly because it needs `ZoneLoader` to instantiate a new item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoServantOutcomeEvent {
    /// C `in = create_item("palace_key1"); if (in && !give_char_item(co,
    /// in)) destroy_item(in);` (`two.c:1200-1203`, `:1250-1253`).
    GivePalaceKey1 { player_id: CharacterId },
    /// C `in = create_item("palace_key2"); if (in && !give_char_item(co,
    /// in)) destroy_item(in);` (`two.c:1236-1239`).
    GivePalaceKey2 { player_id: CharacterId },
}

/// C `servant_parse(cn, dat)` (`two.c:968-978`): parses `arg="nr=N;"` at
/// spawn time (`zone.rs`), following `guard_driver`'s precedent - see
/// this module's own doc comment. C's own unknown-arg branch is a bare
/// `elog(...)`, not ported (same precedent as `parse_two_guard_driver_
/// args`).
pub fn parse_two_servant_driver_args(args: &str) -> TwoServantDriverData {
    let mut data = TwoServantDriverData::default();
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        if name == "nr" {
            data.nr = value.parse::<i32>().unwrap_or(0);
        }
        rest = next;
    }
    data
}

/// C `Sirname(cn)` (`src/system/tool.c:1538-1546`), used by the `pay
/// bribe`/`nr == 0` reply (`two.c:1226`) instead of the giver's real
/// name - same precedent as `world::npc::area11::islena`'s own private
/// copy.
fn servant_sirname(character: &Character) -> &'static str {
    if character.flags.contains(CharacterFlags::MALE) {
        "Sir"
    } else if character.flags.contains(CharacterFlags::FEMALE) {
        "Lady"
    } else {
        "Neuter"
    }
}

impl World {
    /// C `servant`'s per-tick body (`two.c:980-1322`).
    pub fn process_two_servant_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TwoServantPlayerFacts>,
        area_id: u16,
    ) -> Vec<TwoServantOutcomeEvent> {
        let servant_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TWOSERVANT
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for servant_id in servant_ids {
            self.process_two_servant_tick(servant_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_two_servant_tick(
        &mut self,
        servant_id: CharacterId,
        player_facts: &HashMap<CharacterId, TwoServantPlayerFacts>,
        area_id: u16,
        events: &mut Vec<TwoServantOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::TwoServant(mut data)) = self
            .characters
            .get(&servant_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&servant_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.two_servant_handle_char_message(
                    servant_id,
                    &mut data,
                    message,
                    player_facts,
                    &mut face_target,
                ),
                NT_TEXT => self.two_servant_handle_text_message(
                    servant_id,
                    &mut data,
                    message,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.two_servant_handle_give_message(servant_id),
                NT_GOTHIT => self.two_servant_handle_gothit_message(servant_id, &mut data, message),
                _ => {}
            }
        }

        if let Some(servant) = self.characters.get_mut(&servant_id) {
            servant.driver_state = Some(CharacterDriverState::TwoServant(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`two.c:1311-1313`).
        if let (Some(servant), Some((tx, ty))) =
            (self.characters.get(&servant_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(servant.x), i32::from(servant.y), tx, ty) {
                if let Some(servant_mut) = self.characters.get_mut(&servant_id) {
                    let _ = turn(servant_mut, direction as u8);
                }
            }
        }

        let data = match self
            .characters
            .get(&servant_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::TwoServant(data)) => *data,
            _ => return,
        };

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (secure_move_
        // driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret, lastact))
        // return; } do_idle(cn, TICKS);` (`two.c:1315-1321`). `tmpx`/
        // `tmpy` reuse `rest_x`/`rest_y`, the same substitution every
        // other stationary NPC in this codebase makes.
        if data.last_talk_tick + TWO_SERVANT_RETURN_TO_POST_TICKS < self.tick.0 {
            let (post_x, post_y) = self
                .characters
                .get(&servant_id)
                .map(|servant| (servant.rest_x, servant.rest_y))
                .unwrap_or_default();
            if self.secure_move_driver(
                servant_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            ) {
                // Mirrors the C early return; nothing follows, but keep the
                // ported control flow explicit.
                #[allow(clippy::needless_return)]
                return;
            }
        }
        // C `do_idle(cn, TICKS);` (`two.c:1321`) - not modeled, same
        // precedent as every other stationary dialogue-only NPC in this
        // codebase: it has no observable effect in this message-driven
        // architecture.
    }

    /// C `servant`'s `NT_CHAR` branch (`two.c:1003-1088`). Reproduces the
    /// double-switch/`current_state`-increment-before-second-read quirk
    /// and the `illegal_place(ch[cn].x, ch[co].x)` mislabeled-coordinate
    /// quirk digit-for-digit - see this module's own doc comment.
    fn two_servant_handle_char_message(
        &mut self,
        servant_id: CharacterId,
        data: &mut TwoServantDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoServantPlayerFacts>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(servant) = self.characters.get(&servant_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`two.c:1006-1010`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message;
        // continue; }` (`two.c:1012-1016`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`two.c:1018-1022`).
        if tick < data.last_talk_tick + TWO_SERVANT_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->
        // current_victim != co) continue;` (`two.c:1024-1027`).
        if tick < data.last_talk_tick + TWO_SERVANT_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`two.c:1029-1033`).
        if servant_id == player_id
            || !char_see_char(&servant, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`two.c:1035-1039`).
        if char_dist(&servant, &player) > TWO_SERVANT_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        // C `if (dat->current_victim != co) dat->current_state = 0;`
        // (`two.c:1045-1047`).
        if data.current_victim != Some(player_id) {
            data.current_state = 0;
        }

        let mut didsay = false;
        // C `switch (dat->current_state) { case 0: ...; dat->
        // current_state++; break; case 1: break; }` (`two.c:1048-1065`).
        if data.current_state == 0 {
            if data.nr == 4 {
                self.npc_say(
                    servant_id,
                    "Now, what do we have here? I do not think thine presence here is appropriate. GUARDS!",
                );
                self.two_city_call_guard(servant_id, player_id);
            } else {
                self.npc_say_bytes(
                    servant_id,
                    &format!(
                        "Uh, hello, {}. Thou art not supposed to be here. ({COL_STR_LIGHT_BLUE}chat{COL_STR_RESET} {COL_STR_LIGHT_BLUE}bribe{COL_STR_RESET} {COL_STR_LIGHT_BLUE}threaten{COL_STR_RESET})",
                        player.name
                    ),
                );
            }
            data.current_state += 1;
            didsay = true;
        }

        // C `if (illegal_place(ch[cn].x, ch[co].x) > ppd->citizen_status)
        // {} else { switch (dat->current_state) { case 0: ...; break;
        // case 1: break; } }` (`two.c:1067-1081`). Note `data.
        // current_state` was already bumped above (if it started at 0),
        // so this second `case 0` never actually fires in practice - a
        // real C quirk, preserved as-is.
        if illegal_place(servant.x, player.x) <= facts.citizen_status && data.current_state == 0 {
            self.npc_say_bytes(
                servant_id,
                &format!(
                    "My greetings, {}. How may I serve you? ({COL_STR_LIGHT_BLUE}chat{COL_STR_RESET} {COL_STR_LIGHT_BLUE}bribe{COL_STR_RESET} {COL_STR_LIGHT_BLUE}threaten{COL_STR_RESET})",
                    player.name
                ),
            );
            data.current_state += 1;
            didsay = true;
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...; dat->
        // current_victim = co; }` (`two.c:1082-1086`).
        if didsay {
            data.last_talk_tick = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `servant`'s `NT_TEXT` branch (`two.c:1091-1284`), wired through
    /// the generic `analyse_text_qa` matcher (same pattern as `world::
    /// npc::area17::two_skelly`/`alchemist`/`sanwyn`/`barkeeper`'s text
    /// handlers).
    fn two_servant_handle_text_message(
        &mut self,
        servant_id: CharacterId,
        data: &mut TwoServantDriverData,
        message: &CharacterDriverMessage,
        events: &mut Vec<TwoServantOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->
        // current_victim) dat->current_victim = 0;` (`two.c:1094-1096`).
        let tick = self.tick.0;
        if tick > data.last_talk_tick + TWO_SERVANT_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
        {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`two.c:1098-1101`).
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
        let Some(servant_name) = self.characters.get(&servant_id).map(|c| c.name.clone()) else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`two.c:126-144`):
        // ignore our own talk, non-players/player-likes, not-visible.
        if servant_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(servant) = self.characters.get(&servant_id).cloned() else {
            return;
        };
        if !char_see_char(&servant, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let mut didsay = false;
        // C's `analyse_text_driver` calls `say(cn, qa[q].answer, ...)`
        // directly (`two.c:206`), same as every sibling driver.
        match analyse_text_qa(text, &servant_name, &speaker.name, TWOCITY_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(servant_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`two.c:1104-1106`).
            TextAnalysisOutcome::Matched(2) => {
                data.current_state = 0;
                didsay = true;
            }
            // C `case 8:` (chat) (`two.c:1107-1138`).
            TextAnalysisOutcome::Matched(8) => {
                match data.nr {
                    0 => {
                        self.npc_say(
                            servant_id,
                            "I spend my days scrubbing pots and pans. Thou wouldst believe not how dirty they can get. Sometimes it takes me an hour to clean one of the pans.",
                        );
                    }
                    1 => {
                        self.npc_say(
                            servant_id,
                            "The governor, he is a cruel man. Do be very careful here in Exkordon.",
                        );
                    }
                    2 => {
                        self.npc_say(
                            servant_id,
                            "I am the governor's mistress. Oh, I wish I could leave him, oh, I wish.",
                        );
                    }
                    3 => {
                        self.npc_say(
                            servant_id,
                            "It looks like it'll rain soon, doesn't it? The farmers sure could use a good rain.",
                        );
                    }
                    5 => {
                        // C `if (hour > 6 && hour < 23)` (`two.c:1127`).
                        if self.date.hour > 6 && self.date.hour < 23 {
                            self.npc_say(
                                servant_id,
                                &format!(
                                    "So nice of thee to come and visit, {}. Life has been dull, and thou art most interesting.",
                                    speaker.name
                                ),
                            );
                        } else {
                            self.npc_say(
                                servant_id,
                                &format!(
                                    "Why dost thou disturb me in the middle of the night, {}?",
                                    speaker.name
                                ),
                            );
                        }
                    }
                    _ => {} // `nr == 4`: no reply (`didsay` stays truthy, see below).
                };
                didsay = true;
            }
            // C `case 9:` (bribe) (`two.c:1139-1179`).
            TextAnalysisOutcome::Matched(9) => {
                match data.nr {
                    0 => {
                        self.npc_say_bytes(
                            servant_id,
                            &format!("It is nice of thee to offer money, and I could use it, oh yes, I could, but I cannot give thee anything in return. ({COL_STR_LIGHT_BLUE}pay bribe{COL_STR_RESET} of 20G)"),
                        );
                    }
                    1 => {
                        self.npc_say_bytes(
                            servant_id,
                            &format!(
                                "Listen, {}, I know of a secret passage, which connects two store rooms. Thou couldst use it to avoid the guards. I even have the key, which unlocks this door. ({COL_STR_LIGHT_BLUE}pay bribe{COL_STR_RESET} of 50G)",
                                speaker.name
                            ),
                        );
                    }
                    2 => {
                        if speaker.flags.contains(CharacterFlags::MALE) {
                            self.npc_say_bytes(
                                servant_id,
                                &format!(
                                    "Thou art most handsome, {}. For a kiss, I would tell thee how thou canst reach the governor's private rooms through a secret passage. ({COL_STR_LIGHT_BLUE}pay bribe{COL_STR_RESET} - a kiss)",
                                    speaker.name
                                ),
                            );
                        } else {
                            self.npc_say(
                                servant_id,
                                "Thou darest offer me money? Thou art most common, wench.",
                            );
                        }
                    }
                    3 => {
                        self.npc_say_bytes(
                            servant_id,
                            &format!(
                                "Well, there is something I could tell thee, {}, which thou mightst find worth thy money. ({COL_STR_LIGHT_BLUE}pay bribe{COL_STR_RESET} of 50G)",
                                speaker.name
                            ),
                        );
                    }
                    5 => {
                        self.npc_say_bytes(
                            servant_id,
                            &format!("Ah, money. Money is always welcome! ({COL_STR_LIGHT_BLUE}pay bribe{COL_STR_RESET} of 20G)"),
                        );
                    }
                    _ => {} // `nr == 4`.
                };
                didsay = true;
            }
            // C `case 10:` (threaten) (`two.c:1180-1217`).
            TextAnalysisOutcome::Matched(10) => {
                match data.nr {
                    0 => {
                        self.npc_say(
                            servant_id,
                            "No, please, don't kill me! Please! Have mercy, I am just a poor scullery girl!",
                        );
                    }
                    1 => {
                        self.npc_say(
                            servant_id,
                            &format!(
                                "I shall tell the guards about thee, {}. Now go.",
                                speaker.name
                            ),
                        );
                        self.two_city_call_guard(servant_id, speaker_id);
                    }
                    2 => {
                        if speaker.flags.contains(CharacterFlags::MALE) {
                            self.npc_say(
                                servant_id,
                                &format!(
                                    "Uh, thou likest it rough, don't thou, {}? Well, I do not.",
                                    speaker.name
                                ),
                            );
                            self.two_city_call_guard(servant_id, speaker_id);
                        } else {
                            self.npc_say(
                                servant_id,
                                "Uh, thou seemest most determined, lady. I shall relent to thy wishes, then. There is a secret passage to the governors private rooms. It starts in the room behind the southern door leading north-west in the corridor in front of my room. Here's the key.",
                            );
                            events.push(TwoServantOutcomeEvent::GivePalaceKey1 {
                                player_id: speaker_id,
                            });
                        }
                    }
                    3 => {
                        self.npc_say(
                            servant_id,
                            "But, but... I'm just a cook. Why kill me? Please, have mercy!",
                        );
                    }
                    5 => {
                        self.npc_say(servant_id, "GUARDS!");
                        self.two_city_call_guard(servant_id, speaker_id);
                    }
                    _ => {} // `nr == 4`.
                };
                didsay = true;
            }
            // C `case 11:` (pay bribe) (`two.c:1218-1277`).
            TextAnalysisOutcome::Matched(11) => {
                match data.nr {
                    0 => {
                        if self.two_servant_take_money(speaker_id, SERVANT_PAY_BRIBE_COST_SMALL) {
                            self.npc_say(
                                servant_id,
                                &format!(
                                    "Ooh. I thank thee, noble {}, I thank thee! But wait. There is one thing I can tell thee: Avoid the governors study at all cost. It is behind the door leading south-east from the small hallway.",
                                    servant_sirname(&speaker)
                                ),
                            );
                        } else {
                            self.npc_say(servant_id, "Oh, how mean! First thou offerest me money and now thou canst not pay!");
                        }
                    }
                    1 => {
                        if self.two_servant_take_money(speaker_id, SERVANT_PAY_BRIBE_COST_LARGE) {
                            self.npc_say(
                                servant_id,
                                "The passage starts in the store room at the north-eastern end of the corridor in front of this room. Here's the key.",
                            );
                            events.push(TwoServantOutcomeEvent::GivePalaceKey2 {
                                player_id: speaker_id,
                            });
                        } else {
                            self.npc_say(servant_id, "No money no key.");
                        }
                    }
                    2 => {
                        if speaker.flags.contains(CharacterFlags::MALE) {
                            self.npc_say(
                                servant_id,
                                "Ooh, thou art so cute. I shall relent to thy wishes, then. There is a secret passage to the governors private rooms. It starts in the room behind the southern door leading north-west in the corridor in front of my room. Here's the key.",
                            );
                            events.push(TwoServantOutcomeEvent::GivePalaceKey1 {
                                player_id: speaker_id,
                            });
                        } else {
                            self.npc_say(servant_id, "Hu?");
                        }
                    }
                    3 => {
                        if self.two_servant_take_money(speaker_id, SERVANT_PAY_BRIBE_COST_LARGE) {
                            self.npc_say(
                                servant_id,
                                "The governor, he likes to... Well... Eat strawberry pies.",
                            );
                        } else {
                            self.npc_say(
                                servant_id,
                                "Uh, I'm afraid thou dost not have enough money.",
                            );
                        }
                    }
                    5 => {
                        if self.two_servant_take_money(speaker_id, SERVANT_PAY_BRIBE_COST_SMALL) {
                            self.npc_say(
                                servant_id,
                                "Gladly I accept thine noble gift, stranger. Didst thou know that there is a secret entrance to the palace in the sewers?",
                            );
                        } else {
                            self.npc_say(
                                servant_id,
                                "Uh, I'm afraid thou dost not have enough money.",
                            );
                        }
                    }
                    _ => {} // `nr == 4`.
                };
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`two.c:1280-1283`) - note this does *not* touch `dat->
        // last_talk`.
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `servant`'s `NT_GIVE` branch (`two.c:1287-1295`): unlike most
    /// sibling drivers, *any* item handed over is destroyed unconditionally
    /// - there is no give-back fallback (same precedent as `world::npc::
    /// area17::barkeeper`'s own `NT_GIVE` branch).
    fn two_servant_handle_give_message(&mut self, servant_id: CharacterId) {
        let Some(item_id) = self
            .characters
            .get(&servant_id)
            .and_then(|servant| servant.cursor_item)
        else {
            return;
        };
        if let Some(servant) = self.characters.get_mut(&servant_id) {
            servant.cursor_item = None;
        }
        self.destroy_item(item_id);
    }

    /// C `servant`'s `NT_GOTHIT` branch (`two.c:1296-1303`).
    fn two_servant_handle_gothit_message(
        &mut self,
        servant_id: CharacterId,
        data: &mut TwoServantDriverData,
        message: &CharacterDriverMessage,
    ) {
        let attacker_id = CharacterId(message.dat1.max(0) as u32);
        let tick = self.tick.0;
        // C `if (!dat->lastalert || ticker - dat->lastalert > TICKS*30)`
        // (`two.c:1298`).
        if data.lastalert != 0
            && tick.saturating_sub(data.lastalert) <= TWO_SERVANT_ALERT_COOLDOWN_TICKS
        {
            return;
        }
        self.npc_say(servant_id, "Guards! HELP!");
        self.two_city_call_guard(servant_id, attacker_id);
        data.lastalert = tick;
    }

    /// C `take_money(cn, val)` (`src/system/tool.c:3820-3826`), a private
    /// copy matching every other NPC's own inline `take_money` copy (see
    /// `world::gatekeeper::gate_take_money`'s doc comment for the same
    /// precedent).
    fn two_servant_take_money(&mut self, player_id: CharacterId, amount: u32) -> bool {
        let Some(player) = self.characters.get_mut(&player_id) else {
            return false;
        };
        if player.gold < amount {
            return false;
        }
        player.gold -= amount;
        player.flags.insert(CharacterFlags::ITEMS);
        true
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;
