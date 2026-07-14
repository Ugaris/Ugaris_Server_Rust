//! Lab 3 password-gate guard (`CDR_LAB3PASSGUARD`).
//!
//! Ports `src/area/22/lab3.c::lab3_passguard_driver` (`:82-307`). There is
//! exactly one guard instance in the whole game (`ugaris_data/zones/22/
//! lab3.chr:398`), stationed in front of the password-protected teleport
//! door `IDR_LAB3_SPECIAL` gates (`drdata[0]==1`, `drdata[3]!=0` -
//! `lab3_special`, ported in `item_driver::area22_lab::lab3_special_driver`
//! + `World::apply_lab3_teleport_door`).
//!
//! Deviations/gaps (documented, not silent):
//! - C's `static int talk` (`lab3.c:83`) is process-lifetime, not
//!   per-character: the *first* `CDR_LAB3PASSGUARD` ever created
//!   (server-wide, across every respawn after the guard's very first
//!   creation) latches `dat->talk = 1` forever; every later creation
//!   (i.e. every respawn after the guard's first-ever death) sees the
//!   static already `1` and never sets its own fresh `dat->talk`, so the
//!   respawned guard is permanently mute - a real, reproduced C bug, not
//!   a simplification. Modeled as [`crate::world::World::
//!   lab3_passguard_talk_latched`] (in-memory only, resets on server
//!   restart exactly like C's own `static int`).
//! - The two duplicate `if (msg->type == NT_TEXT)` blocks (`:109-112` and
//!   `:224-273`) both call `tabunga(cn, co, ...)` on every text message
//!   (the first block does nothing else - the C `if`-chain simply falls
//!   through both blocks for any `NT_TEXT` message); this port calls
//!   [`crate::world::World::apply_tabunga_text_notification`] once, a
//!   harmless dedup of an observably identical double call.
//! - `fight_driver_set_dist(cn, 10, 0, 12)`/`fight_driver_add_enemy(cn,
//!   co, 1, 1)`/`fight_driver_update`/`fight_driver_attack_visible`/
//!   `fight_driver_follow_invisible` (C's generic 10-slot `struct
//!   fight_driver_data` enemy list) are replaced by a single tracked
//!   `co`/`serial`/`attacking`/`pursuing` pair, the same narrowing
//!   established by `world::npc::area22::lab2_deamon`'s own module doc
//!   comment. The `stop_dist=12` "give up if the victim strays too far
//!   from home" trim (`fight_driver_update`'s own `dat->stop_dist` check)
//!   is reproduced directly via [`dist_from_home`] instead of via the
//!   enemy-list mechanism it lived on in C.
//! - `standard_message_driver(cn, msg, 0, 0)` is not reproduced (dead
//!   code for `agressive=0, helper=0`), same precedent as every other
//!   NPC's own module doc comment in this directory.
//! - `IDR_LAB3_SPECIAL`'s note-reading branch (`lab3_special`'s
//!   `drdata[0]==3`, cases `20`/`21`) is what actually calls
//!   `lab3_init_password` and populates
//!   [`crate::player::PlayerRuntime::legacy_lab3_password1`]/
//!   `legacy_lab3_password2` (`ugaris-server`'s `tick_item_use_lab.rs`,
//!   since the random pick + `PlayerRuntime` write need the server layer).
//!   Until a player reads one of those two notes, both fields stay empty;
//!   C's own `strcasestr(str, "")` (empty needle) always matches, so this
//!   port's `contains` check behaves identically to C in that state - not
//!   a behavioral deviation.

use crate::direction::Direction;
use crate::drvlib::{char_dist, offset2dx};
use crate::path::pathfinder;
use crate::see::char_see_char;
use crate::world::*;

/// C `char_dist(cn, co) > 10` (`lab3.c:160`): far-away talkstep reset gate.
const LAB3_PASSGUARD_TALK_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`lab3.c:172`): minimum gap between greetings.
const LAB3_PASSGUARD_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `dat->stop_dist` set by `fight_driver_set_dist(cn, 10, 0, 12)`
/// (`lab3.c:102`): give up the chase once the victim strays this far from
/// the guard's home post.
const LAB3_PASSGUARD_STOP_DIST: i32 = 12;
/// C `TICKS * 5` (`lab3.c:300`): idle "return to post" threshold.
const LAB3_PASSGUARD_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `dat->last_talk = ticker + 10 * TICKS` (`lab3.c:268`): the "BLUB"
/// confusion penalty.
const LAB3_PASSGUARD_BLUB_PENALTY_TICKS: u64 = TICKS_PER_SECOND * 10;

/// Per-player facts [`World::process_lab3_passguard_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lab3PassguardPlayerFacts {
    /// `PlayerRuntime::legacy_lab3_guard_talkstep()`.
    pub guard_talkstep: u8,
    /// `PlayerRuntime::legacy_lab3_password1()`, nul-padded to 8 bytes.
    pub password1: [u8; 8],
    /// `PlayerRuntime::legacy_lab3_password2()`, nul-padded to 8 bytes.
    pub password2: [u8; 8],
}

/// A side effect [`World::process_lab3_passguard_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lab3PassguardOutcomeEvent {
    /// Write the new `ppd->guard_talkstep` back.
    SetGuardTalkstep { player_id: CharacterId, value: u8 },
}

impl World {
    /// C `lab3_passguard_driver`'s per-tick body (`lab3.c:82-307`).
    pub fn process_lab3_passguard_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, Lab3PassguardPlayerFacts>,
        area_id: u16,
    ) -> Vec<Lab3PassguardOutcomeEvent> {
        let guard_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAB3PASSGUARD
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for guard_id in guard_ids {
            self.process_lab3_passguard_tick(guard_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_lab3_passguard_tick(
        &mut self,
        guard_id: CharacterId,
        player_facts: &HashMap<CharacterId, Lab3PassguardPlayerFacts>,
        area_id: u16,
        events: &mut Vec<Lab3PassguardOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Lab3Passguard(mut data)) = self
            .characters
            .get(&guard_id)
            .and_then(|guard| guard.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&guard_id)
            .map(|guard| std::mem::take(&mut guard.driver_messages))
            .unwrap_or_default();

        let mut talkdir: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                // C `if (msg->type == NT_CREATE) { fight_driver_set_dist
                // (...); if (!talk) { talk = 1; dat->talk = 1; } }`
                // (`:101-107`).
                NT_CREATE => {
                    if !self.lab3_passguard_talk_latched {
                        self.lab3_passguard_talk_latched = true;
                        data.talk = true;
                    }
                }
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if let Some(text) = message.text.as_deref() {
                        self.apply_tabunga_text_notification(guard_id, speaker_id, text);
                    }
                    self.lab3_passguard_handle_text(
                        guard_id,
                        &mut data,
                        message,
                        player_facts,
                        events,
                        &mut talkdir,
                    );
                }
                NT_GIVE => {
                    // C `if (msg->type == NT_GIVE && ch[cn].citem) {
                    // destroy_item(ch[cn].citem); ch[cn].citem = 0; }`
                    // (`:114-118`).
                    if let Some(item_id) = self
                        .characters
                        .get(&guard_id)
                        .and_then(|guard| guard.cursor_item)
                    {
                        self.destroy_item(item_id);
                        if let Some(guard) = self.characters.get_mut(&guard_id) {
                            guard.cursor_item = None;
                        }
                    }
                }
                NT_CHAR => {
                    self.lab3_passguard_handle_char(
                        guard_id,
                        &mut data,
                        message,
                        player_facts,
                        events,
                        &mut talkdir,
                    );
                }
                _ => {}
            }
        }

        if let Some(guard) = self.characters.get_mut(&guard_id) {
            guard.driver_state = Some(CharacterDriverState::Lab3Passguard(data));
        }

        // C: `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
        // 0)) return; if (fight_driver_follow_invisible(cn)) return;`
        // (`:280-286`), narrowed to the single tracked `co` (see module
        // doc comment).
        if data.attacking && self.lab3_passguard_pursue(guard_id, &mut data, area_id) {
            if let Some(guard) = self.characters.get_mut(&guard_id) {
                guard.driver_state = Some(CharacterDriverState::Lab3Passguard(data));
            }
            return;
        }

        // C: `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return;` (`:289-294`).
        if self.regenerate_simple_baddy(guard_id) {
            if let Some(guard) = self.characters.get_mut(&guard_id) {
                guard.driver_state = Some(CharacterDriverState::Lab3Passguard(data));
            }
            return;
        }
        if self.spell_self_simple_baddy(guard_id) {
            if let Some(guard) = self.characters.get_mut(&guard_id) {
                guard.driver_state = Some(CharacterDriverState::Lab3Passguard(data));
            }
            return;
        }

        // C `if (talkdir) turn(cn, talkdir);` (`:297-299`).
        if let (Some(guard), Some((tx, ty))) = (self.characters.get(&guard_id).cloned(), talkdir) {
            if let Some(direction) = offset2dx(i32::from(guard.x), i32::from(guard.y), tx, ty) {
                if let Some(guard_mut) = self.characters.get_mut(&guard_id) {
                    let _ = turn(guard_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*5 < ticker) { if
        // (secure_move_driver(cn, tmpx, tmpy, DX_RIGHT, ret, lastact))
        // return; }` (`:300-304`). `tmpx`/`tmpy` reuse `rest_x`/`rest_y`,
        // the same substitution every other stationary NPC in this
        // codebase uses.
        if data.last_talk_tick + LAB3_PASSGUARD_RETURN_TO_POST_TICKS < self.tick.0 {
            let (post_x, post_y) = self
                .characters
                .get(&guard_id)
                .map(|guard| (guard.rest_x, guard.rest_y))
                .unwrap_or_default();
            if self.secure_move_driver(
                guard_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            ) {
                if let Some(guard) = self.characters.get_mut(&guard_id) {
                    guard.driver_state = Some(CharacterDriverState::Lab3Passguard(data));
                }
            }
        }
        // C `do_idle(cn, TICKS);` (`:306`) - not modeled, same precedent
        // as every other stationary dialogue-only NPC in this codebase.
    }

    /// C `lab3_passguard_driver`'s `NT_CHAR` branch (`:120-222`).
    #[allow(clippy::too_many_arguments)]
    fn lab3_passguard_handle_char(
        &mut self,
        guard_id: CharacterId,
        data: &mut Lab3PassguardDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab3PassguardPlayerFacts>,
        events: &mut Vec<Lab3PassguardOutcomeEvent>,
        talkdir: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!dat->talk) { remove_message; continue; }` (`:124-127`).
        if !data.talk {
            return;
        }
        // C `if (ch[cn].x != tmpx || ch[cn].y != tmpy) { remove_message;
        // continue; }` (`:130-133`): only greet while at the guard post.
        if guard.x != guard.rest_x || guard.y != guard.rest_y {
            return;
        }
        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`:136-139`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`:142-145`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) { remove_message;
        // continue; }` (`:148-151`).
        if guard_id == player_id || !char_see_char(&guard, &player, &self.map, self.date.daylight) {
            return;
        }
        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        // C `if (char_dist(cn, co) > 10) { ...reset...; remove_message;
        // continue; }` (`:160-169`).
        if char_dist(&guard, &player) > LAB3_PASSGUARD_TALK_DISTANCE {
            if facts.guard_talkstep == 20 {
                self.npc_say(
                    guard_id,
                    &format!(
                        "Do not forget thine password, {}. Next time I will ask you again.",
                        player.name
                    ),
                );
            } else if facts.guard_talkstep != 0 {
                self.npc_say(
                    guard_id,
                    &format!("Thou art wise, {}, very wise.", player.name),
                );
            }
            events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                player_id,
                value: 0,
            });
            return;
        }

        // C `if (ticker < dat->last_talk + 5*TICKS) { remove_message;
        // continue; }` (`:172-175`).
        let tick = self.tick.0;
        if tick < data.last_talk_tick + LAB3_PASSGUARD_TALK_MIN_TICKS {
            return;
        }

        let mut didsay = false;
        let mut trigger_fight = false;
        // C `switch (ppd->guard_talkstep) { ... }` (`:178-216`).
        match facts.guard_talkstep {
            0 => {
                self.npc_say(
                    guard_id,
                    &format!(
                        "Halt, {}, say the password, or leave this place immediately!",
                        player.name
                    ),
                );
                didsay = true;
                events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                    player_id,
                    value: 1,
                });
            }
            1 => {
                didsay = true;
                events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                    player_id,
                    value: 2,
                });
            }
            2 => {
                didsay = true;
                events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                    player_id,
                    value: 3,
                });
            }
            3 => {
                self.npc_say(
                    guard_id,
                    &format!(
                        "I'll count up to three, then I will kill thee, {}. So move, or say the password!",
                        player.name
                    ),
                );
                didsay = true;
                events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                    player_id,
                    value: 4,
                });
            }
            4 => {
                self.npc_say(guard_id, "One.");
                didsay = true;
                events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                    player_id,
                    value: 5,
                });
            }
            5 => {
                self.npc_say(guard_id, "Two.");
                didsay = true;
                events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                    player_id,
                    value: 6,
                });
            }
            6 => {
                self.npc_say(guard_id, &format!("Three! {}, I'm coming!", player.name));
                events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                    player_id,
                    value: 0,
                });
                trigger_fight = true;
            }
            // C `case 20: break;` (`:214-215`): password mode - silent.
            _ => {}
        }

        if trigger_fight {
            // C `fight_driver_add_enemy(cn, co, 1, 1)` (`:211`).
            data.co = Some(player_id);
            data.serial = player.serial;
            data.attacking = true;
            data.pursuing = true;
            data.victim_last_x = player.x;
            data.victim_last_y = player.y;
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir =
        // offset2dx(...); }` (`:218-221`).
        if didsay {
            data.last_talk_tick = tick;
            *talkdir = Some((i32::from(player.x), i32::from(player.y)));
        }
    }

    /// C `lab3_passguard_driver`'s second `NT_TEXT` branch (`:224-273`),
    /// the actual password-check logic (the first `NT_TEXT` block only
    /// calls `tabunga` - see the module doc comment for the dedup).
    #[allow(clippy::too_many_arguments)]
    fn lab3_passguard_handle_text(
        &mut self,
        guard_id: CharacterId,
        data: &mut Lab3PassguardDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab3PassguardPlayerFacts>,
        events: &mut Vec<Lab3PassguardOutcomeEvent>,
        talkdir: &mut Option<(i32, i32)>,
    ) {
        // C `if (msg->dat1 == LOG_INFO) { remove_message; continue; }`
        // (`:232-235`): no emotes.
        if message.dat1 == i32::from(crate::log_text::LOG_INFO) {
            return;
        }
        // C `if (!dat->talk) { remove_message; continue; }` (`:238-241`).
        if !data.talk {
            return;
        }
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`:244-247`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        // C `if (char_dist(cn, co) > 10) { remove_message; continue; }`
        // (`:250-253`).
        if char_dist(&guard, &speaker) > LAB3_PASSGUARD_TALK_DISTANCE {
            return;
        }
        let Some(facts) = player_facts.get(&speaker_id) else {
            return;
        };
        let Some(text) = message.text.as_deref() else {
            return;
        };

        // C `sprintf(password, "%s%s", ppd->password1, ppd->password2);
        // if (strcasestr(str, password)) { ... } else if (strcasestr(str,
        // "BLUB")) { ... } else if (strcasestr(str, "REPEAT")) { ... }`
        // (`:261-272`).
        let mut password = trim_nul(&facts.password1);
        password.extend(trim_nul(&facts.password2));
        let password_lower = String::from_utf8_lossy(&password).to_lowercase();
        let text_lower = text.to_lowercase();

        if text_lower.contains(&password_lower) {
            self.npc_say(
                guard_id,
                &format!(
                    "Fine, {}, I will open the door for thee. Mayest thou pass the last gate.",
                    speaker.name
                ),
            );
            events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                player_id: speaker_id,
                value: 20,
            });
        } else if text_lower.contains("blub") {
            self.npc_say(guard_id, "What?");
            data.last_talk_tick = self.tick.0 + LAB3_PASSGUARD_BLUB_PENALTY_TICKS;
        } else if text_lower.contains("repeat") {
            self.npc_say(guard_id, "I'll repeat.");
            events.push(Lab3PassguardOutcomeEvent::SetGuardTalkstep {
                player_id: speaker_id,
                value: 0,
            });
        }
        let _ = talkdir;
    }

    /// C: "fighting" tail (`:280-286`), narrowed to the single tracked
    /// `co` - see the module doc comment. Returns `true` if an attack or
    /// pursuit move consumed this tick's action (matching C's `return`
    /// after `fight_driver_attack_visible`/`fight_driver_follow_
    /// invisible` returning nonzero).
    fn lab3_passguard_pursue(
        &mut self,
        guard_id: CharacterId,
        data: &mut Lab3PassguardDriverData,
        area_id: u16,
    ) -> bool {
        let Some(co) = data.co else {
            data.attacking = false;
            data.pursuing = false;
            return false;
        };
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return false;
        };
        let valid = self.characters.get(&co).is_some_and(|victim| {
            victim.flags.contains(CharacterFlags::USED) && victim.serial == data.serial
        });
        if !valid {
            data.attacking = false;
            data.pursuing = false;
            data.co = None;
            return false;
        }
        let victim = self.characters.get(&co).cloned().unwrap();

        // C `fight_driver_update`'s `stop_dist` trim (`drvlib.c:2201-2205`,
        // configured via `fight_driver_set_dist(cn, 10, 0, 12)`): give up
        // once the victim strays too far from the guard's home post.
        if dist_from_home(&victim, guard.rest_x, guard.rest_y) > LAB3_PASSGUARD_STOP_DIST {
            data.attacking = false;
            data.pursuing = false;
            data.co = None;
            return false;
        }

        if !data.pursuing {
            return false;
        }

        let visible = char_see_char(&guard, &victim, &self.map, self.date.daylight);
        if visible {
            data.victim_last_x = victim.x;
            data.victim_last_y = victim.y;
            return self.attack_driver_direct(guard_id, co, area_id);
        }

        // C `fight_driver_follow_invisible`: "we're at his last position
        // but didn't find him there - give up" (`drvlib.c:2309-2313`).
        let arrived = self.characters.get(&guard_id).is_some_and(|guard| {
            guard.x.abs_diff(data.victim_last_x) < 2 && guard.y.abs_diff(data.victim_last_y) < 2
        });
        if arrived {
            data.pursuing = false;
            return false;
        }

        let (fx, fy) = (usize::from(guard.x), usize::from(guard.y));
        let (tx, ty) = (
            usize::from(data.victim_last_x),
            usize::from(data.victim_last_y),
        );
        let path = pathfinder(&self.map, fx, fy, tx, ty, 0, None);
        let path = if path.direction.is_none() {
            pathfinder(&self.map, fx, fy, tx, ty, 1, None)
        } else {
            path
        };
        let Some(direction) = path.direction else {
            data.pursuing = false;
            return false;
        };
        self.walk_or_use_driver(guard_id, direction, area_id)
    }
}

/// C `dist_from_home(cn, co)` (`src/system/drvlib.c:2366-2377`), the
/// Chebyshev-like weighted distance used to trim `fight_driver_update`'s
/// enemy list against `dat->stop_dist`.
fn dist_from_home(character: &Character, home_x: u16, home_y: u16) -> i32 {
    let dx = (i32::from(character.x) - i32::from(home_x)).abs();
    let dy = (i32::from(character.y) - i32::from(home_y)).abs();
    if dx > dy {
        (dx << 1) + dy
    } else {
        (dy << 1) + dx
    }
}

/// Trims trailing nul bytes from a fixed-size C-string field.
fn trim_nul(field: &[u8; 8]) -> Vec<u8> {
    let len = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    field[..len].to_vec()
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct lab3_passguard_driver_data` (`lab3.c:75-80`). `current_victim`/
/// `current_victim_serial` are declared in C but never read/written
/// anywhere in `lab3_passguard_driver` (dead fields) and are not ported;
/// `co`/`serial`/`attacking`/`pursuing`/`victim_last_x`/`victim_last_y`
/// are this port's narrowed stand-in for C's generic `struct
/// fight_driver_data`/`DRD_FIGHTDRIVER` slot - see the module doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab3PassguardDriverData {
    #[serde(default)]
    pub talk: bool,
    #[serde(default)]
    pub last_talk_tick: u64,
    #[serde(default)]
    pub co: Option<CharacterId>,
    #[serde(default)]
    pub serial: u32,
    #[serde(default)]
    pub attacking: bool,
    #[serde(default)]
    pub pursuing: bool,
    #[serde(default)]
    pub victim_last_x: u16,
    #[serde(default)]
    pub victim_last_y: u16,
}
