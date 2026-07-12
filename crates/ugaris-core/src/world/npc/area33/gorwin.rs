//! Gorwin NPC (`CDR_TUNNELER_GORWIN`), the "Tunnel Changer" creeper who
//! runs the entrance lobby of the Long Tunnels (area 33) and lets players
//! pick which difficulty level to enter.
//!
//! Ports `src/area/33/tunnel.c::gorwin_driver` (`:1016-1354`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:114-302`, [`GORWIN_QA`]
//! below), `initialize_gorwin_ppd` (`:997-1015`), `change_tunnel_level`
//! (`:816-996`), and `handle_tunnel_info` (`:794-815`). The tunnel-door
//! item drivers (`IDR_TUNNELDOOR`/`IDR_TUNNELDOOR2`) and the procedural
//! creeper-dungeon generator (`build_fighter`/`handle_block_marker`/
//! `handle_creeper_marker`/`find_unused_sector`/`give_reward`) are a
//! separate, not-yet-ported slice - see `PORTING_TODO.md`'s Area 33 entry.
//!
//! `tunnel_ppd`/`gorwin_ppd` persistence (`PlayerRuntime::tunnel_used`/
//! `tunnel_clevel`/`gorwin_tunnel_level` and friends) was already fully
//! ported in an earlier iteration (`crate::player::tunnel`) for the
//! `/tunnel`/`/tunnels` commands - this driver only *reads* that state via
//! [`GorwinPlayerFacts`] and *writes* it via [`GorwinOutcomeEvent`], the
//! same `World`/`PlayerRuntime` split established by
//! `world::npc::area29::spiritbran`/`world::npc::area3::kassim`.
//!
//! Deviations/gaps (documented, not silent):
//! - C's unconditional `standard_message_driver(cn, msg, 0, 0)` tail call
//!   for every message is not ported: Gorwin's template carries
//!   `CF_NOATTACK|CF_IMMORTAL` (`ugaris_data/zones/33/tunnel.chr`), so its
//!   `NT_GOTHIT`/`NT_SEEHIT` self-defense branches (called with
//!   `agressive=0, helper=0`, so the `NT_CHAR`/`NT_SEEHIT` cases are
//!   already dead code) are unreachable in practice - same precedent as
//!   `world::npc::area29::spiritbran`'s identical omission.
//! - C's `NT_CREATE` handler zero-resets `last_talk`/`current_victim`/
//!   `state`/`on_break_until` (but *not* `next_break_check`) - ported for
//!   fidelity even though [`GorwinDriverData::default()`] already starts
//!   every field at zero, so it is only observable after a hypothetical
//!   respawn (Gorwin's `CF_IMMORTAL` flag means this never actually
//!   happens in practice).
//! - C's unconditional `do_idle(cn, TICKS*2)` tail call is not ported,
//!   matching the established `world::thomas`/`world::sir_jones`/
//!   `world::npc::area29::spiritbran` precedent for stationary dialogue
//!   NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::player::{MAX_TUNNEL_LEVEL, MAX_TUNNEL_USES, MIN_TUNNEL_LEVEL};
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET, COL_STR_YELLOW};
use crate::world::*;

/// C `char_dist(cn, co) > 10` (`tunnel.c:1078`, `NT_CHAR` greet gate) and
/// `char_dist(cn, co) < 10` (`tunnel.c:1246`, `NT_TEXT` reply gate) - both
/// share the same `10`-tile radius.
const GORWIN_TALK_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`tunnel.c:1060`).
const GORWIN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`tunnel.c:1066`, `1223`).
const GORWIN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 15` (`tunnel.c:1097`): next-break-roll cooldown.
const GORWIN_BREAK_CHECK_TICKS: u64 = TICKS_PER_SECOND * 15;
/// C `TICKS * 60` (`tunnel.c:1099`): break duration.
const GORWIN_BREAK_DURATION_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `TICKS * 5 * 60` (`tunnel.c:1191`): re-greet-if-idle threshold.
const GORWIN_RETALK_TICKS: u64 = TICKS_PER_SECOND * 5 * 60;
/// C `TICKS * 30` (`tunnel.c:1236`): on-break poke-again cooldown.
const GORWIN_ON_BREAK_POKE_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS` (`tunnel.c:1246`): minimum gap between replies.
const GORWIN_TEXT_REPLY_MIN_TICKS: u64 = TICKS_PER_SECOND;
/// C `TICKS * 30` (`tunnel.c:1336`): idle "return to post" threshold.
const GORWIN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS * 60` (`tunnel.c:1343`): idle-mutter threshold.
const GORWIN_MUTTER_TICKS: u64 = TICKS_PER_SECOND * 60;

/// C `#define TUNNEL_LOBBY_MIN 244` (`tunnel.h:36`).
const TUNNEL_LOBBY_MIN: u16 = 244;
/// C `#define TUNNEL_LOBBY_MAX 254` (`tunnel.h:37`).
const TUNNEL_LOBBY_MAX: u16 = 254;

/// C `gorwin_mutterings[]` (`tunnel.c:52-65`).
const GORWIN_MUTTERINGS: &[&str] = &[
    "I wonder if they notice the extra gold...",
    "Being a creeper is honest work... mostly.",
    "Note to self: count the gold BEFORE giving change.",
    "These tunnels could use some decoration. Maybe some skulls.",
    "Heh heh heh...",
    "Was that 100 gold per level or 200? Eh, close enough.",
    "I should ask for a raise. Do creepers get raises?",
    "The left path... or was it the right? I always forget.",
    "Shiny gold... precious gold...",
    "They say never trust a creeper. Smart advice, really.",
    "I wonder what the surface looks like these days.",
    "One gold for me, one gold for them... wait, other way around.",
];
/// C `gorwin_fourth_wall[]` (`tunnel.c:69-75`).
const GORWIN_FOURTH_WALL: &[&str] = &[
    "You know, I've been standing in this tunnel for... how long has the server been up?",
    "Sometimes I feel like I'm just... following a script. Weird.",
    "Do you ever get the feeling someone is watching us? Through a screen, perhaps?",
    "I had the strangest dream. I was just... code. Lines and lines of code.",
    "Tick, tick, tick... twenty-four times a second. Anyone else hear that?",
];
/// C `gorwin_wrong_names[]` (`tunnel.c:79-81`).
const GORWIN_WRONG_NAMES: &[&str] = &[
    "adventurer",
    "brave warrior",
    "tunnel enthusiast",
    "valued customer",
    "friend",
    "stranger",
    "hero",
    "champion",
];

/// C `struct qa qa[]` (`tunnel.c:114-197`).
const GORWIN_QA: &[crate::character_driver::TextQaEntry] = &[
    crate::character_driver::TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm shimmering with energy! How about you, %s?"),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["hello"],
        answer: Some("Welcome to the tunnels, %s! I'm Gorwin, your friendly neighborhood creeper."),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["hi"],
        answer: Some("Greetings, %s! Don't be alarmed by my appearance, I'm here to help!"),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["who", "are", "you"],
        answer: Some(
            "I'm Gorwin the Levelshifter, a friendly creeper who can adjust the tunnel difficulty for you.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["bye"],
        answer: Some("Safe travels through the tunnels, %s!"),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["tunnel"],
        answer: None,
        answer_code: 1,
    },
    crate::character_driver::TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 9,
    },
    crate::character_driver::TextQaEntry {
        words: &["level"],
        answer: None,
        answer_code: 10,
    },
    crate::character_driver::TextQaEntry {
        words: &["change"],
        answer: Some(
            "To change your tunnel level, just say 'level X'. You can reset to (your level - 10) for free, or pay a fee for other levels.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["help"],
        answer: Some(
            "I can tell you about the tunnels, change the difficulty level, or offer advice. What do you need?",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["advice"],
        answer: Some(
            "It's usually best to start with a level close to your own and adjust as needed. Don't forget to collect your rewards!",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["creepers"],
        answer: Some(
            "Creepers are the magical creatures you'll face in the tunnels. They adapt to your chosen difficulty level. Be prepared for a challenge!",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["rewards"],
        answer: Some(
            "The tunnels offer two types of rewards: experience and military rank. You can choose which type of reward you want at the end of each section.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["experience"],
        answer: Some(
            "Choosing the experience reward will grant you valuable XP to help you level up faster. The amount decreases with each completion of the same level.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["military", "rank"],
        answer: Some("Opting for military rank will boost your standing in the army. It's a great way to climb the ranks quickly!"),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["military"],
        answer: Some("Military rank is one of the rewards you can choose in the tunnels. It boosts your standing in the army."),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["exit"],
        answer: Some(
            "To exit the tunnels, you'll need to find an exit door. Remember, you can't change your level while inside the tunnels.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["max", "uses"],
        answer: Some(
            "You can complete up to 10 tunnels at each level. After that, you won't receive any more rewards for that level.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["experience", "reward"],
        answer: Some("The experience reward is calculated based on the tunnel's level, and your own"),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["military", "reward"],
        answer: Some(
            "Military rank rewards scale with tunnel level. At level 10 you get about 11 points for first completion, scaling up to about 110 points at level 100. The exact formula is: (100 + level*level/10) / (completions + 9). Higher level tunnels give significantly more military rank, but rewards decrease with each completion.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["exit", "door"],
        answer: Some(
            "If you choose to exit through the central door instead of taking a reward, you'll be teleported out of the tunnel but won't receive any reward. It's useful if you do not want to surpass a level or millitary rank.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["fee"],
        answer: Some(
            "There's a small fee for changing your tunnel level. It's calculated as 100 gold multiplied by the difference between your current level and the desired level.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["reward", "decrease"],
        answer: Some(
            "Both experience and military rank rewards decrease with each completion of a tunnel at the same level. This is calculated by dividing the base reward by (completions + 9). For example, military rewards go from about 11 points for the first completion, to 5 points for the second, then 4, 3, and so on. This encourages you to progress to higher levels as you grow stronger.",
        ),
        answer_code: 0,
    },
    crate::character_driver::TextQaEntry {
        words: &["first", "completion"],
        answer: Some(
            "For your first completion of a tunnel level, the military reward would be 100 / 9, which is about 11 points.",
        ),
        answer_code: 0,
    },
];

/// Per-player facts [`World::process_gorwin_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone)]
pub struct GorwinPlayerFacts {
    /// `PlayerRuntime::gorwin_tunnel_level()`.
    pub gorwin_tunnel_level: i32,
    /// `PlayerRuntime::tunnel_clevel()`.
    pub tunnel_clevel: i32,
    /// `PlayerRuntime::tunnel_used(level)` for every `level` in
    /// `0..=MAX_TUNNEL_LEVEL`, indexed by `level` directly (`tunnel_ppd::
    /// used[204]`, `tunnel.h:8`).
    pub tunnel_used: Vec<u8>,
}

impl GorwinPlayerFacts {
    fn used_at(&self, level: i32) -> u8 {
        if level < 0 {
            return 0;
        }
        self.tunnel_used.get(level as usize).copied().unwrap_or(0)
    }
}

/// A side effect [`World::process_gorwin_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GorwinOutcomeEvent {
    /// Writes only `gorwin_ppd::tunnel_level` (C `initialize_gorwin_ppd`,
    /// `tunnel.c:997-1015`, and the "waiting" auto-promote branch,
    /// `tunnel.c:1172-1180` - neither touches `tunnel_ppd::clevel`).
    SetGorwinTunnelLevel { player_id: CharacterId, level: i32 },
    /// Writes both `gorwin_ppd::tunnel_level` and `tunnel_ppd::clevel` to
    /// the same `level` (C `change_tunnel_level`'s three level-changing
    /// branches, `tunnel.c:879-880`, `905-906`, `976-977`).
    SetTunnelLevelBoth { player_id: CharacterId, level: i32 },
}

/// C `find_next_available_level` (`tunnel.c:516-525`): pure, so it takes
/// the already-snapshotted [`GorwinPlayerFacts`] instead of `World`.
fn find_next_available_level(
    facts: &GorwinPlayerFacts,
    start_level: i32,
    max_level: i32,
) -> Option<i32> {
    let upper = MAX_TUNNEL_LEVEL.min(max_level);
    ((start_level + 1)..=upper).find(|&level| facts.used_at(level) < MAX_TUNNEL_USES)
}

/// C `is_player_in_tunnel` (`tunnel.c:774-792`), narrowed to the pure
/// area/position check (the caller already knows `co` is a player).
fn gorwin_is_player_in_tunnel(x: u16, y: u16, area_id: u16) -> bool {
    if area_id != 33 {
        return false;
    }
    !(TUNNEL_LOBBY_MIN..=TUNNEL_LOBBY_MAX).contains(&x)
        || !(TUNNEL_LOBBY_MIN..=TUNNEL_LOBBY_MAX).contains(&y)
}

/// C `isdigit(*arg)` gate + `atoi(arg)` (`tunnel.c:1261-1262`): parses the
/// maximal leading run of ASCII digits, requiring at least one.
fn parse_leading_i32(text: &str) -> Option<i32> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<i32>().ok()
}

impl World {
    /// C `gorwin_driver`'s per-tick body (`tunnel.c:1016-1354`).
    pub fn process_gorwin_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, GorwinPlayerFacts>,
        area_id: u16,
    ) -> Vec<GorwinOutcomeEvent> {
        let gorwin_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TUNNELER_GORWIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for gorwin_id in gorwin_ids {
            self.process_gorwin_messages(gorwin_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_gorwin_messages(
        &mut self,
        gorwin_id: CharacterId,
        player_facts: &HashMap<CharacterId, GorwinPlayerFacts>,
        area_id: u16,
        events: &mut Vec<GorwinOutcomeEvent>,
    ) {
        let Some(gorwin_name) = self.characters.get(&gorwin_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::Gorwin(mut data)) = self
            .characters
            .get(&gorwin_id)
            .and_then(|character| character.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&gorwin_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                // C `if (msg->type == NT_CREATE) { ... }` (`tunnel.c:1032-1037`).
                NT_CREATE => {
                    data.last_talk = 0;
                    data.current_victim = None;
                    data.state = 0;
                    data.on_break_until = 0;
                }
                NT_CHAR => self.gorwin_handle_char_message(
                    gorwin_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.gorwin_handle_text_message(
                    gorwin_id,
                    &gorwin_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                    area_id,
                ),
                _ => {}
            }
        }

        if let Some(gorwin) = self.characters.get_mut(&gorwin_id) {
            gorwin.driver_state = Some(CharacterDriverState::Gorwin(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`tunnel.c:1332-1334`).
        if let (Some(gorwin), Some((tx, ty))) =
            (self.characters.get(&gorwin_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(gorwin.x), i32::from(gorwin.y), tx, ty) {
                if let Some(gorwin_mut) = self.characters.get_mut(&gorwin_id) {
                    let _ = turn(gorwin_mut, direction as u8);
                }
            }
        }

        let (last_talk, rest_x, rest_y) = match self.characters.get(&gorwin_id) {
            Some(gorwin) => (
                match gorwin.driver_state.as_ref() {
                    Some(CharacterDriverState::Gorwin(data)) => data.last_talk,
                    _ => return,
                },
                gorwin.rest_x,
                gorwin.rest_y,
            ),
            None => return,
        };

        let tick = self.tick.0;
        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`tunnel.c:1336-1340`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::spiritbran` already uses.
        let moved = if last_talk + GORWIN_RETURN_TO_POST_TICKS < tick {
            self.secure_move_driver(
                gorwin_id,
                rest_x,
                rest_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            )
        } else {
            false
        };
        if moved {
            return;
        }

        // C `if (dat->last_talk + TICKS*60 < ticker && !RANDOM(25)) { ...
        // dat->last_talk = ticker; }` (`tunnel.c:1343-1351`).
        if last_talk + GORWIN_MUTTER_TICKS < tick && self.roll_legacy_random(25) == 0 {
            let line = if self.roll_legacy_random(50) == 0 {
                GORWIN_FOURTH_WALL
                    [self.roll_legacy_random(GORWIN_FOURTH_WALL.len() as u32) as usize]
            } else {
                GORWIN_MUTTERINGS[self.roll_legacy_random(GORWIN_MUTTERINGS.len() as u32) as usize]
            };
            self.npc_murmur(gorwin_id, line);
            if let Some(CharacterDriverState::Gorwin(data)) = self
                .characters
                .get_mut(&gorwin_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                data.last_talk = tick;
            }
        }
    }

    /// C `gorwin_driver`'s `NT_CHAR` branch (`tunnel.c:1039-1216`).
    #[allow(clippy::too_many_arguments)]
    fn gorwin_handle_char_message(
        &mut self,
        gorwin_id: CharacterId,
        data: &mut GorwinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GorwinPlayerFacts>,
        events: &mut Vec<GorwinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(gorwin) = self.characters.get(&gorwin_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if ((ch[co].flags & CF_PLAYER) && ch[co].driver !=
        // CDR_LOSTCON) initialize_gorwin_ppd(co);` (`tunnel.c:1042-1045`).
        if player.flags.contains(CharacterFlags::PLAYER) && player.driver != CDR_LOSTCON {
            if let Some(facts) = player_facts.get(&player_id) {
                if facts.gorwin_tunnel_level == 0 {
                    let level = if facts.tunnel_clevel >= MIN_TUNNEL_LEVEL {
                        facts.tunnel_clevel
                    } else {
                        ((player.level as i32) - 10).clamp(MIN_TUNNEL_LEVEL, MAX_TUNNEL_LEVEL)
                    };
                    events.push(GorwinOutcomeEvent::SetGorwinTunnelLevel { player_id, level });
                }
            }
        }

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`tunnel.c:1048-1051`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message;
        // continue; }` (`tunnel.c:1054-1057`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`tunnel.c:1060-1063`).
        if tick < data.last_talk + GORWIN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->
        // current_victim && dat->current_victim != co) continue;`
        // (`tunnel.c:1066-1069`).
        if tick < data.last_talk + GORWIN_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`tunnel.c:1072-1075`).
        if gorwin_id == player_id || !char_see_char(&gorwin, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`tunnel.c:1078-1081`).
        if char_dist(&gorwin, &player) > GORWIN_TALK_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        // C `if (dat->on_break_until > ticker) { remove_message;
        // continue; }` (`tunnel.c:1090-1093`).
        if data.on_break_until > tick {
            return;
        }

        // C `if (dat->state > 8 && ticker > dat->next_break_check) { ...
        // }` (`tunnel.c:1096-1107`).
        if data.state > 8 && tick > data.next_break_check {
            data.next_break_check = tick + GORWIN_BREAK_CHECK_TICKS;
            if self.roll_legacy_random(100) == 0 {
                data.on_break_until = tick + GORWIN_BREAK_DURATION_TICKS;
                self.npc_say(
                    gorwin_id,
                    "You know what? I need a break. Even creepers need rest. Come back in a minute.",
                );
                data.last_talk = tick;
                *face_target = Some((i32::from(player.x), i32::from(player.y)));
                data.current_victim = None;
                return;
            }
        }

        let mut didsay = false;

        if data.state <= 8 {
            let text = match data.state {
                0 => {
                    let name = if self.roll_legacy_random(100) < 5 {
                        GORWIN_WRONG_NAMES[self.roll_legacy_random(GORWIN_WRONG_NAMES.len() as u32) as usize]
                    } else {
                        player.name.as_str()
                    };
                    format!(
                        "Welcome to the magical tunnels, {name}! I'm Gorwin, and I'm here to help you navigate the challenges ahead."
                    )
                }
                1 => format!(
                    "The tunnels are a magical labyrinth that adapts to your skills. You can complete up to {MAX_TUNNEL_USES} tunnels at each level."
                ),
                2 => format!(
                    "You'll face {COL_STR_LIGHT_BLUE}creepers{COL_STR_RESET} and overcome obstacles as you progress through each tunnel."
                ),
                3 => format!(
                    "At the end of each tunnel, you'll find pillars offering {COL_STR_LIGHT_BLUE}experience{COL_STR_RESET} or {COL_STR_LIGHT_BLUE}military rank{COL_STR_RESET} as rewards."
                ),
                4 => format!(
                    "The {COL_STR_LIGHT_BLUE}experience{COL_STR_RESET} reward is based on the tunnel's level and decreases with each completion."
                ),
                5 => format!(
                    "The {COL_STR_LIGHT_BLUE}military rank{COL_STR_RESET} reward is based on the amount of completions and decreases with each completion."
                ),
                6 => "If you choose to exit through the central door instead of taking a reward, you'll be teleported out but receive no reward.".to_string(),
                7 => format!(
                    "The minimum tunnel level is {MIN_TUNNEL_LEVEL}, and the maximum is your current level ({}).",
                    player.level
                ),
                _ => format!(
                    "To change your tunnel level, just say '{COL_STR_LIGHT_BLUE}level X{COL_STR_RESET}', where X is the desired level. There's a fee (100G per level) based on the level difference."
                ),
            };
            self.npc_say_bytes(gorwin_id, &text);
            data.state += 1;
            didsay = true;
        } else if facts.gorwin_tunnel_level > 0
            && facts.gorwin_tunnel_level <= MAX_TUNNEL_LEVEL
            && facts.gorwin_tunnel_level <= (player.level as i32)
        {
            // C `struct tunnel_ppd *tppd = ...; if (tppd && tppd->
            // used[ppd->tunnel_level] >= MAX_TUNNEL_USES) { ... }`
            // (`tunnel.c:1170-1188`).
            if facts.used_at(facts.gorwin_tunnel_level) >= MAX_TUNNEL_USES {
                if let Some(next) =
                    find_next_available_level(facts, facts.gorwin_tunnel_level, player.level as i32)
                {
                    self.npc_say_bytes(
                        gorwin_id,
                        &format!(
                            "I see thou hast mastered level {}, {}! I've already set thy tunnel level to {COL_STR_YELLOW} {next}{COL_STR_RESET}. Ready for a new challenge?",
                            facts.gorwin_tunnel_level, player.name
                        ),
                    );
                    events.push(GorwinOutcomeEvent::SetGorwinTunnelLevel {
                        player_id,
                        level: next,
                    });
                } else {
                    self.npc_say(
                        gorwin_id,
                        &format!(
                            "Remarkable, {}! Thou hast conquered every tunnel level available to thee. There is nothing more I can teach thee. Thou art a true master of the depths!",
                            player.name
                        ),
                    );
                }
                didsay = true;
            }
        }

        // C `if (!didsay && dat->last_talk + TICKS*5*60 < ticker) { ...
        // }` (`tunnel.c:1191-1209`).
        if !didsay && data.last_talk + GORWIN_RETALK_TICKS < tick {
            if self.roll_legacy_random(100) < 3 {
                self.npc_say(
                    gorwin_id,
                    "Wait, have we met? I'm Gorwin! Let me tell you about the tunnels.",
                );
                data.state = 0;
            } else {
                let name = if self.roll_legacy_random(100) < 5 {
                    GORWIN_WRONG_NAMES
                        [self.roll_legacy_random(GORWIN_WRONG_NAMES.len() as u32) as usize]
                } else {
                    player.name.as_str()
                };
                self.npc_say_bytes(
                    gorwin_id,
                    &format!(
                        "Welcome back, {name}! Do you need any information about the tunnels or changing your {COL_STR_LIGHT_BLUE}level{COL_STR_RESET}? Or doest thou need me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} my introduction?"
                    ),
                );
            }
            didsay = true;
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`tunnel.c:1211-1215`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `gorwin_driver`'s `NT_TEXT` branch (`tunnel.c:1219-1325`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn gorwin_handle_text_message(
        &mut self,
        gorwin_id: CharacterId,
        gorwin_name: &str,
        data: &mut GorwinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GorwinPlayerFacts>,
        events: &mut Vec<GorwinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
        area_id: u16,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let tick = self.tick.0;

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->
        // current_victim) dat->current_victim = 0;` (`tunnel.c:1222-1225`).
        if tick > data.last_talk + GORWIN_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`tunnel.c:1228-1231`).
        if let Some(victim) = data.current_victim {
            if victim != speaker_id {
                return;
            }
        }

        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        // C `if (co != cn && (ch[co].flags & CF_PLAYER)) { ... }`
        // (`tunnel.c:1233`).
        if speaker_id == gorwin_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `if (dat->on_break_until > ticker) { ...; remove_message;
        // continue; }` (`tunnel.c:1235-1242`).
        if data.on_break_until > tick {
            if tick > data.last_talk + GORWIN_ON_BREAK_POKE_TICKS {
                self.npc_say(gorwin_id, "I told you, I'm on break! Come back later.");
                data.last_talk = tick;
            }
            return;
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(gorwin) = self.characters.get(&gorwin_id).cloned() else {
            return;
        };
        // C `if (ticker > dat->last_talk + TICKS && char_dist(cn,co) < 10
        // && char_see_char(cn,co)) { ... }` (`tunnel.c:1246`).
        if tick <= data.last_talk + GORWIN_TEXT_REPLY_MIN_TICKS
            || char_dist(&gorwin, &speaker) >= GORWIN_TALK_DISTANCE
            || !char_see_char(&gorwin, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        // C `ppd = set_data(co, DRD_GORWIN_PPD, ...); if (!ppd) {
        // remove_message; continue; }` (`tunnel.c:1247-1251`).
        let Some(facts) = player_facts.get(&speaker_id) else {
            return;
        };

        let mut didsay = false;

        // C `arg = strcasestr(ptr, "level"); if (arg && (arg == ptr ||
        // !isalpha(*(arg - 1)))) { ... }` (`tunnel.c:1254-1255`).
        let lower = text.to_ascii_lowercase();
        let level_command = lower.find("level").and_then(|pos| {
            let preceded_ok = pos == 0 || !text.as_bytes()[pos - 1].is_ascii_alphabetic();
            preceded_ok.then(|| text[pos + "level".len()..].trim_start())
        });

        if let Some(rest) = level_command {
            if let Some(level) = parse_leading_i32(rest) {
                if gorwin_is_player_in_tunnel(speaker.x, speaker.y, area_id) {
                    self.npc_say(
                        gorwin_id,
                        "You can't change your tunnel level while inside a tunnel. Please exit first.",
                    );
                } else {
                    self.gorwin_change_tunnel_level(gorwin_id, speaker_id, facts, level, events);
                }
                didsay = true;
            } else {
                self.npc_say(
                    gorwin_id,
                    &format!(
                        "Please specify a level between {MIN_TUNNEL_LEVEL} and your current level ({}).",
                        speaker.level
                    ),
                );
                didsay = true;
            }
        } else {
            match analyse_text_qa(text, gorwin_name, &speaker.name, GORWIN_QA) {
                TextAnalysisOutcome::Said(reply) => {
                    self.npc_say(gorwin_id, &reply);
                    didsay = true;
                }
                // C `case 1:` -> `handle_tunnel_info(cn, co);`
                // (`tunnel.c:1281-1284`).
                TextAnalysisOutcome::Matched(1) => {
                    self.gorwin_handle_tunnel_info(gorwin_id, &speaker);
                    didsay = true;
                }
                // C `case 9:` (`tunnel.c:1285-1290`, the "repeat" reset).
                TextAnalysisOutcome::Matched(9) => {
                    data.state = 0;
                    data.last_talk = 0;
                    self.npc_say(
                        gorwin_id,
                        "Certainly! Let me explain everything about the tunnels again.",
                    );
                    didsay = true;
                }
                // C `case 10:` (`tunnel.c:1291-1315`, the "level" range
                // info, with a 5% chance to lie about the numbers).
                TextAnalysisOutcome::Matched(10) => {
                    if self.roll_legacy_random(100) < 5 {
                        let display_min = MIN_TUNNEL_LEVEL + self.roll_legacy_random(10) as i32;
                        let display_max =
                            (speaker.level as i32) + self.roll_legacy_random(20) as i32 - 10;
                        self.npc_say(
                            gorwin_id,
                            &format!(
                                "I can adjust the tunnel's difficulty for you. You can choose between {display_min} and {display_max}. You can always reset to (your level - 10) for free, or pay 100g per level difference for other changes."
                            ),
                        );
                        self.npc_say(gorwin_id, "...actually, don't quote me on those numbers.");
                    } else {
                        self.npc_say(
                            gorwin_id,
                            &format!(
                                "I can adjust the tunnel's difficulty for you. You can choose between {MIN_TUNNEL_LEVEL} and {}. You can always reset to (your level - 10) for free, or pay 100g per level difference for other changes.",
                                speaker.level
                            ),
                        );
                    }
                    didsay = true;
                }
                TextAnalysisOutcome::Matched(_) => {}
                // C's fallthrough `return 2;` -> the switch's default
                // `case 2:` (`tunnel.c:1277-1280`).
                TextAnalysisOutcome::NoMatch => {
                    self.npc_say(
                        gorwin_id,
                        "I'm sorry, I didn't quite catch that. Could you rephrase?",
                    );
                    didsay = true;
                }
            }
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`tunnel.c:1319-1322`).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `handle_tunnel_info` (`tunnel.c:794-815`).
    fn gorwin_handle_tunnel_info(&mut self, gorwin_id: CharacterId, speaker: &Character) {
        self.npc_say(
            gorwin_id,
            "The tunnels are a magical labyrinth that adapts to your skills.",
        );
        self.npc_say_bytes(
            gorwin_id,
            &format!(
                "You can complete up to {MAX_TUNNEL_USES} tunnels at each level, facing {COL_STR_LIGHT_BLUE}creepers{COL_STR_RESET} and obstacles."
            ),
        );
        self.npc_say_bytes(
            gorwin_id,
            &format!(
                "Rewards include {COL_STR_LIGHT_BLUE}experience{COL_STR_RESET} and {COL_STR_LIGHT_BLUE}military rank{COL_STR_RESET}. The reward amount decreases with each completion."
            ),
        );
        let divider = self.settings.tunnel_exp_base_value_divider;
        self.npc_say(
            gorwin_id,
            &format!("Experience reward is based on the tunnel level value divided by {divider:.6}, then divided by (completions + 9)."),
        );
        let mill_base = self.settings.tunnel_mill_exp_base_value;
        self.npc_say(
            gorwin_id,
            &format!(
                "Military rank reward starts at {mill_base} points divided by (completions + 9). For a fresh tunnel, that's about 11 points."
            ),
        );
        self.npc_say(
            gorwin_id,
            "You can also exit without a reward using the central door.",
        );
        self.npc_say_bytes(
            gorwin_id,
            &format!(
                "Your tunnel level can be between {MIN_TUNNEL_LEVEL} and your current level ({}). Change it by saying '{COL_STR_LIGHT_BLUE}level X{COL_STR_RESET}'.",
                speaker.level
            ),
        );
        self.npc_say_bytes(
            gorwin_id,
            &format!(
                "There's a fee for changing levels. Any other questions about the {COL_STR_LIGHT_BLUE}tunnels{COL_STR_RESET}, {COL_STR_LIGHT_BLUE}rewards{COL_STR_RESET}, or {COL_STR_LIGHT_BLUE}levels{COL_STR_RESET}?"
            ),
        );
    }

    /// C `change_tunnel_level` (`tunnel.c:816-996`).
    fn gorwin_change_tunnel_level(
        &mut self,
        gorwin_id: CharacterId,
        player_id: CharacterId,
        facts: &GorwinPlayerFacts,
        level: i32,
        events: &mut Vec<GorwinOutcomeEvent>,
    ) {
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };
        let char_level = player.level as i32;
        let default_level = (char_level - 10).max(MIN_TUNNEL_LEVEL);
        let max_allowed = MAX_TUNNEL_LEVEL.min(char_level);

        // C `if (level < MIN_TUNNEL_LEVEL || level > min(MAX_TUNNEL_LEVEL,
        // ch[co].level)) { ... }` (`tunnel.c:837-841`).
        if level < MIN_TUNNEL_LEVEL || level > max_allowed {
            self.npc_say(
                gorwin_id,
                &format!("I can only set your tunnel level between {MIN_TUNNEL_LEVEL} and {max_allowed} (your max allowed level)."),
            );
            return;
        }

        // C `if (level == gorwin_ppd->tunnel_level) { ... }`
        // (`tunnel.c:844-863`).
        if level == facts.gorwin_tunnel_level {
            let quip = match self.roll_legacy_random(5) {
                0 => format!("Thou... thou dost realize thy tunnel level is already {level}, right?"),
                1 => format!(
                    "I could set it to {level} for you, but it's already {level}. Math isn't thy strong suit, is it?"
                ),
                2 => format!("Let me check... yep, {level}. That's where you already are. Brilliant request."),
                3 => format!("Ah yes, level {level}. A fine choice. Also thy current one. Coincidence?"),
                _ => format!("Processing thy request to change from {level} to {level}... done! That'll be extra."),
            };
            self.npc_say(gorwin_id, &quip);
            // C `if (RANDOM(2) && ch[co].gold >= 500) { ... }`
            // (`tunnel.c:856-861`).
            if self.roll_legacy_random(2) == 1 && player.gold >= 500 {
                let petty_fee = (self.roll_legacy_random(5) as i32 + 1) * 100;
                if let Some(p) = self.characters.get_mut(&player_id) {
                    p.gold = p.gold.saturating_sub(petty_fee as u32);
                }
                self.npc_say(
                    gorwin_id,
                    &format!("That'll be {} gold for wasting my time.", petty_fee / 100),
                );
            }
            return;
        }

        // C `if (tunnel_ppd->used[level] >= MAX_TUNNEL_USES) { ... }`
        // (`tunnel.c:866-889`).
        if facts.used_at(level) >= MAX_TUNNEL_USES {
            if let Some(next) = find_next_available_level(facts, level, char_level) {
                let quip = match self.roll_legacy_random(3) {
                    0 => format!(
                        "Level {level}? Thou hast already squeezed every last drop from that one. I'll set thee to {next} instead. You're welcome."
                    ),
                    1 => format!(
                        "All {MAX_TUNNEL_USES} completions at level {level} used up. Fine, I'll bump thee to {next}. Do I look like thy personal secretary?"
                    ),
                    _ => format!("Nothing left at level {level} for thee. I've moved thee to {next}. The things I do around here..."),
                };
                self.npc_say(gorwin_id, &quip);
                events.push(GorwinOutcomeEvent::SetTunnelLevelBoth {
                    player_id,
                    level: next,
                });
            } else {
                self.npc_say(
                    gorwin_id,
                    &format!("Level {level} is fully completed, and there's nothing beyond it for thee. Thou art truly done here."),
                );
            }
            return;
        }

        // C `if (abs(gorwin_ppd->tunnel_level - level) == 1) { ... }`
        // (`tunnel.c:892-901`) - flavor only, falls through.
        if (facts.gorwin_tunnel_level - level).abs() == 1 {
            const TINY_QUIPS: [&str; 4] = [
                "One whole level, eh? Barely worth the paperwork.",
                "A single level difference. Living dangerously, I see.",
                "Plus one level. Truly a bold strategic move.",
                "Just one level? I had to get up for this?",
            ];
            let quip = TINY_QUIPS[self.roll_legacy_random(4) as usize];
            self.npc_say(gorwin_id, quip);
        }

        // C `if (level == default_level) { ... }` (`tunnel.c:904-920`).
        if level == default_level {
            events.push(GorwinOutcomeEvent::SetTunnelLevelBoth { player_id, level });
            if self.roll_legacy_random(100) < 2 && player.gold >= 10000 {
                if let Some(p) = self.characters.get_mut(&player_id) {
                    p.gold = p.gold.saturating_sub(10000);
                }
                self.npc_say(
                    gorwin_id,
                    &format!("I've reset your tunnel level to {level} for the low price of 100 gold. Good luck in the tunnels!"),
                );
                self.npc_say(
                    gorwin_id,
                    "...wait, was that supposed to be free? Hmm. Well, too late now!",
                );
            } else {
                self.npc_say(
                    gorwin_id,
                    &format!("I've reset your tunnel level to {level} (your level - 10) for free. Good luck in the tunnels!"),
                );
            }
            return;
        }

        // C `fee = abs(gorwin_ppd->tunnel_level - level) * 10000;` plus the
        // fudge-roll easter eggs (`tunnel.c:923-961`).
        let fee = (facts.gorwin_tunnel_level - level).abs() * 10000;
        let fudge_roll = self.roll_legacy_random(100) as i32;
        let mut actual_fee = fee;
        let mut fudge_msg: Option<&str> = None;
        if fudge_roll < 3 {
            actual_fee = fee + (self.roll_legacy_random(5) as i32 + 2) * 10000;
            fudge_msg =
                Some("Hmm, prices went up recently... supply chain issues, you understand.");
        } else if fudge_roll < 6 {
            actual_fee = fee + (self.roll_legacy_random(3) as i32 + 1) * 1000;
            fudge_msg = Some("Plus a small... administrative fee. Don't worry about it.");
        } else if fudge_roll < 8 {
            actual_fee = (fee - (self.roll_legacy_random(3) as i32 + 1) * 10000).max(100);
            fudge_msg = Some("Tell you what, I like your face. I'll give you a little discount.");
        } else if fudge_roll < 9 {
            actual_fee = self.roll_legacy_random(5) as i32 * 100 + 100;
            fudge_msg =
                Some("Shh, don't tell anyone, but I may have... miscounted. Your lucky day!");
        } else if fudge_roll < 10 {
            actual_fee = fee * 2;
            fudge_msg = Some("That'll be the standard fee. Trust me, I counted twice.");
        } else if fudge_roll < 12 {
            let rounded = (fee / 100_000) * 100_000;
            if rounded > 0 && rounded < fee {
                actual_fee = rounded;
                fudge_msg = Some(
                    "That'll be... let me round that down for you. I'll keep the change, though.",
                );
            }
        } else if fudge_roll < 13 {
            actual_fee = 0;
            fudge_msg = Some("You know what, this one's on the house. Don't ask why.");
        }

        let counts_out_loud = self.roll_legacy_random(100) < 3 && actual_fee > 0;

        // C `if (ch[co].gold < actual_fee) { ... }` (`tunnel.c:966-971`).
        if i64::from(player.gold) < i64::from(actual_fee) {
            self.npc_say(
                gorwin_id,
                &format!(
                    "You need {} gold to change your tunnel level to {level}. You currently have {} gold.",
                    actual_fee / 100,
                    player.gold / 100
                ),
            );
            self.npc_say(
                gorwin_id,
                &format!(
                    "You can always reset to level {default_level} (your level - 10) for free!"
                ),
            );
            return;
        }

        if actual_fee > 0 {
            if let Some(p) = self.characters.get_mut(&player_id) {
                p.gold = p.gold.saturating_sub(actual_fee as u32);
            }
        }
        events.push(GorwinOutcomeEvent::SetTunnelLevelBoth { player_id, level });

        if counts_out_loud {
            self.npc_say(
                gorwin_id,
                &format!(
                    "Let's see... one, two, skip a few... {}! Yes, that's correct. Probably.",
                    actual_fee / 100
                ),
            );
        }
        if let Some(msg) = fudge_msg {
            self.npc_say(gorwin_id, msg);
        }
        if actual_fee > 0 {
            self.npc_say(
                gorwin_id,
                &format!(
                    "For the small fee of {} gold your tunnel level has been set to {level}. Good luck in the tunnels!",
                    actual_fee / 100
                ),
            );
        } else {
            self.npc_say(
                gorwin_id,
                &format!("Your tunnel level has been set to {level}. Good luck in the tunnels!"),
            );
        }
        self.npc_say(gorwin_id, &format!("Remember, you can always reset to level {default_level} (your level - 10) for free!"));
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_LOSTCON, CDR_TUNNELER_GORWIN};

/// C `struct gorwin_driver_data` (`tunnel.c:43-49`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GorwinDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
    #[serde(default)]
    pub state: i32,
    #[serde(default)]
    pub on_break_until: u64,
    #[serde(default)]
    pub next_break_check: u64,
}
