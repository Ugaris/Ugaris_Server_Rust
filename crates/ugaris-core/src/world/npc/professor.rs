//! `CDR_PROFESSOR` generic profession-teacher NPC.
//!
//! Ports `src/common/professor.c`'s `professor_driver`/
//! `professor_driver_parse`/`learn_prof`/`improve_prof`/`count_prof`
//! (`:216-333`) plus the file-local `analyse_text_driver`/`qa` table
//! (`:67-214`, wired through the shared
//! [`crate::character_driver::analyse_text_qa`] matcher, same pattern as
//! `world::bank`/`world::merchant`) and the `src/system/prof.c` profession
//! metadata table (`struct prof prof[P_MAX]`, `:35-56`) needed by
//! `learn_prof`/`improve_prof`/`free_prof_points` (only entries `0..=10`,
//! the eleven real professions any zone-placed `teach0`-`teach10` NPC can
//! actually teach - C's remaining `P_MAX`(20) slots are `"Demon"`/`"empty"`
//! placeholder rows this driver never reaches, see `PROF_TABLE`
//! (`world/text.rs`) for the parallel `(name, max)` table `/values`'s
//! `show_prof_info` already uses).
//!
//! Eleven live "Teacher" NPCs (`teach0`-`teach10`, one per profession) are
//! placed on Area 3's map (`ugaris_data/zones/3/above3_generic.chr`/
//! `above3.map`), each configured via its own `arg="nr=N;quest=0;
//! option=NNN;cost=50;"` zone-file argument - this is genuinely reachable
//! player-facing content, not dead code (unlike some other "C's own driver
//! is empty" P4 closures).
//!
//! `achievement_check_profession` (`professor.c:291`/`:329`) is queued via
//! [`ProfessorAchievementCheck`]/[`World::drain_pending_professor_
//! achievement_checks`] rather than applied directly, since the actual
//! Rust achievement state (`AccountAchievements`) lives on
//! `crate::player::PlayerRuntime`, outside `World`'s visibility - same
//! `pending_*`/`drain_pending_*` convention as `world::exp::
//! LevelAchievementCheck`. Everything else `learn_prof`/`improve_prof`
//! touch (`Character.professions`/`.gold`/`.flags`) is resolved
//! synchronously in `World`, since none of it needs `PlayerRuntime`.
//!
//! Deviations/gaps (documented, not silent):
//! - `struct professor_driver_data::last_talk` (`professor.c:217`) is
//!   never read or written anywhere in `professor_driver`'s own body -
//!   dropped here, same precedent as `world::james`'s dead `nighttime`
//!   field.
//! - `dlog`/direct DB writes have no Rust sink in this codebase (existing
//!   gap, not introduced here) - not applicable, `professor_driver` has
//!   none.
//! - `NT_CHAR`'s `remove_message`-on-guard-failure semantics are a no-op
//!   in this port: the whole `driver_messages` buffer is drained
//!   unconditionally every tick regardless of which guard clauses reject
//!   any individual message, matching every other message-driven NPC in
//!   this codebase.

use crate::character_driver::{
    analyse_text_qa, mem_add_driver, mem_check_driver, next_legacy_name_value, TextAnalysisOutcome,
    TextQaEntry, CDR_PROFESSOR,
};
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};
use crate::world::*;

/// C `char_dist(cn, co) > 10` in `professor_driver`'s `NT_CHAR` greeting
/// guard (`professor.c:371`).
const PROFESSOR_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` in `analyse_text_driver`'s own guard
/// (`professor.c:130`).
const PROFESSOR_QA_DISTANCE: i32 = 12;
/// C `mem_check_driver(cn, co, 7)`/`mem_add_driver(cn, co, 7)`
/// (`professor.c:377`/`:386`).
const PROFESSOR_GREET_MEMORY_SLOT: usize = 7;

/// C `struct prof prof[P_MAX]` (`src/system/prof.c:35-56`), entries
/// `0..=10` only - see the module doc comment. `(name, base, max, step)`.
const PROF_STATS: [(&str, i32, i32, i32); 11] = [
    ("Athlete", 6, 30, 3),
    ("Alchemist", 10, 50, 10),
    ("Miner", 4, 20, 2),
    ("Assassin", 10, 50, 5),
    ("Thief", 6, 30, 3),
    ("Light Warrior", 6, 30, 3),
    ("Dark Warrior", 6, 30, 3),
    ("Trader", 4, 20, 2),
    ("Mercenary", 4, 20, 2),
    ("Clan Warrior", 6, 30, 3),
    ("Herbalist", 10, 30, 10),
];

/// `src/system/server.h`'s `P_LIGHT`/`P_DARK` indices, used by
/// `learn_prof`'s mutual-exclusion guard (`professor.c:268-275`).
const P_LIGHT: usize = 5;
const P_DARK: usize = 6;

/// A queued `achievement_check_profession(co, nr, ch[co].prof[nr])` call
/// (`professor.c:291`/`:329`) - see the module doc comment for why this is
/// deferred rather than applied inline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfessorAchievementCheck {
    pub player_id: CharacterId,
    pub profession: i32,
    pub level: i32,
}

/// C `struct professor_driver_data` (`professor.c:216-224`), minus the
/// dead `last_talk` field - see the module doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProfessorDriverData {
    pub dir: i32,
    pub nr: i32,
    pub quest: i32,
    pub quest_option: i32,
    pub improve_cost: i32,
}

/// C `professor_driver_parse` (`professor.c:226-244`).
pub fn parse_professor_driver_args(args: &str) -> ProfessorDriverData {
    let mut data = ProfessorDriverData::default();
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "dir" => data.dir = parsed,
            "nr" => data.nr = parsed,
            "quest" => data.quest = parsed,
            "option" => data.quest_option = parsed,
            "cost" => data.improve_cost = parsed,
            _ => {}
        }
        rest = next;
    }
    data
}

/// C `free_prof_points(co)` (`src/system/prof.c:97-104`).
fn free_prof_points(character: &Character) -> i32 {
    let used: i32 = character.professions.iter().map(|&p| i32::from(p)).sum();
    i32::from(character.values[1][CharacterValue::Profession as usize]) - used
}

impl World {
    pub fn drain_pending_professor_achievement_checks(&mut self) -> Vec<ProfessorAchievementCheck> {
        self.pending_professor_achievement_checks
            .drain(..)
            .collect()
    }

    /// C `learn_prof(cn, co, nr)` (`src/common/professor.c:257-295`).
    /// Returns `true` on success, matching C's own `int` return.
    fn learn_prof(&mut self, professor_id: CharacterId, player_id: CharacterId, nr: usize) -> bool {
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return false;
        };
        let Some(&(name, base, _max, _step)) = PROF_STATS.get(nr) else {
            return false;
        };

        if player.professions.get(nr).copied().unwrap_or(0) != 0 {
            self.npc_say(
                professor_id,
                &format!("But thou knowest the ways of the {name} already!"),
            );
            return false;
        }
        if !player.flags.contains(CharacterFlags::PAID) && count_prof(&player) > 0 {
            self.npc_say(
                professor_id,
                "Only paying players may learn more than one profession.",
            );
            return false;
        }
        if nr == P_LIGHT && player.professions.get(P_DARK).copied().unwrap_or(0) != 0 {
            self.npc_say(
                professor_id,
                "Thou hast learned Master of Dark already, thou mayest not learn Master of Light.",
            );
            return false;
        }
        if nr == P_DARK && player.professions.get(P_LIGHT).copied().unwrap_or(0) != 0 {
            self.npc_say(
                professor_id,
                "Thou hast learned Master of Light already, thou mayest not learn Master of Dark.",
            );
            return false;
        }

        let cnt = free_prof_points(&player);
        if cnt < base {
            self.npc_say(
                professor_id,
                &format!(
                    "Thou have not the required profession points. Thou needst {base}, but thou hast only {cnt}."
                ),
            );
            return false;
        }

        if let Some(player_mut) = self.characters.get_mut(&player_id) {
            player_mut.professions[nr] = base as i16;
            player_mut.flags.insert(CharacterFlags::PROF);
        }
        self.update_character(player_id);
        self.npc_say(
            professor_id,
            &format!("Thou hast learnt the art of {name}."),
        );

        if player.flags.contains(CharacterFlags::PLAYER) {
            self.pending_professor_achievement_checks
                .push(ProfessorAchievementCheck {
                    player_id,
                    profession: nr as i32,
                    level: base,
                });
        }
        true
    }

    /// C `improve_prof(cn, co, nr)` (`src/common/professor.c:297-333`).
    fn improve_prof(
        &mut self,
        professor_id: CharacterId,
        player_id: CharacterId,
        nr: usize,
    ) -> bool {
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return false;
        };
        let Some(&(name, _base, max, step_cfg)) = PROF_STATS.get(nr) else {
            return false;
        };
        let current = i32::from(player.professions.get(nr).copied().unwrap_or(0));

        if current == 0 {
            self.npc_say(
                professor_id,
                &format!(
                    "But thou knowest not the ways of the {name}. Thou must learn them first."
                ),
            );
            return false;
        }
        if current >= max {
            self.npc_say(
                professor_id,
                &format!("Thou hast reached mastery in the art of the {name} already."),
            );
            return false;
        }

        let cnt = free_prof_points(&player);
        let step = step_cfg.min(max - current);

        if !player.flags.contains(CharacterFlags::PAID) && current + step >= 20 {
            self.npc_say(
                professor_id,
                &format!("Only paying players may raise this profession higher than {current}."),
            );
            return false;
        }
        if cnt < step {
            self.npc_say(
                professor_id,
                &format!("Thou needst have at least {step_cfg} unused profession points."),
            );
            return false;
        }

        let new_level = current + step;
        if let Some(player_mut) = self.characters.get_mut(&player_id) {
            player_mut.professions[nr] = new_level as i16;
            player_mut.flags.insert(CharacterFlags::PROF);
        }
        self.update_character(player_id);
        self.npc_say(
            professor_id,
            &format!("Thy profession {name} was improved to {new_level}."),
        );

        if player.flags.contains(CharacterFlags::PLAYER) {
            self.pending_professor_achievement_checks
                .push(ProfessorAchievementCheck {
                    player_id,
                    profession: nr as i32,
                    level: new_level,
                });
        }
        true
    }

    /// C `professor_driver`'s per-tick body (`professor.c:335-516`).
    pub fn process_professor_actions(&mut self, area_id: u16) {
        let professor_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_PROFESSOR
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for professor_id in professor_ids {
            self.process_professor_messages(professor_id);
            self.process_professor_tick_action(professor_id, area_id);
        }
    }

    fn process_professor_messages(&mut self, professor_id: CharacterId) {
        let Some(professor) = self.characters.get_mut(&professor_id) else {
            return;
        };
        let professor_name = professor.name.clone();
        let messages = std::mem::take(&mut professor.driver_messages);

        let mut destroy_cursor = false;
        let mut greet_targets: Vec<CharacterId> = Vec::new();
        let mut text_events: Vec<(CharacterId, String)> = Vec::new();

        for message in messages {
            match message.message_type {
                NT_CHAR => {
                    greet_targets.push(CharacterId(message.dat1.max(0) as u32));
                }
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if speaker_id == professor_id {
                        continue;
                    }
                    if let Some(text) = message.text.as_deref() {
                        text_events.push((speaker_id, text.to_string()));
                    }
                }
                NT_GIVE => {
                    destroy_cursor = true;
                }
                _ => {}
            }
        }

        // C `professor_driver`'s `NT_CHAR` greeting branch
        // (`professor.c:354-387`).
        for player_id in greet_targets {
            let Some(professor) = self.characters.get(&professor_id).cloned() else {
                return;
            };
            let Some(player) = self.characters.get(&player_id).cloned() else {
                continue;
            };
            if professor_id == player_id
                || !char_see_char(&professor, &player, &self.map, self.date.daylight)
            {
                continue;
            }
            if player.values[1][CharacterValue::Profession as usize] == 0 {
                continue;
            }
            if char_dist(&professor, &player) > PROFESSOR_GREET_DISTANCE {
                continue;
            }
            if mem_check_driver(
                &professor.driver_memory,
                PROFESSOR_GREET_MEMORY_SLOT,
                player_id.0,
            ) {
                continue;
            }
            let data = match professor.driver_state.as_ref() {
                Some(CharacterDriverState::Professor(data)) => *data,
                _ => continue,
            };
            let Some(&(prof_name, ..)) = PROF_STATS.get(data.nr as usize) else {
                continue;
            };
            self.npc_say_bytes(
                professor_id,
                &format!(
                    "Hello {}! I am a professor at Aston University, and I {COL_STR_LIGHT_BLUE}teach{COL_STR_RESET} {COL_STR_LIGHT_BLUE}{prof_name}{COL_STR_RESET}.",
                    player.name
                ),
            );
            if let Some(professor_mut) = self.characters.get_mut(&professor_id) {
                mem_add_driver(
                    &mut professor_mut.driver_memory,
                    PROFESSOR_GREET_MEMORY_SLOT,
                    player_id.0,
                );
            }
        }

        // C `professor_driver`'s `NT_TEXT` branch (`professor.c:390-492`).
        for (speaker_id, text) in text_events {
            self.professor_handle_text(professor_id, &professor_name, speaker_id, &text);
        }

        // C `professor_driver`'s `NT_GIVE` branch (`professor.c:495-503`):
        // the given item simply vanishes.
        if destroy_cursor {
            let cursor = self
                .characters
                .get_mut(&professor_id)
                .and_then(|professor| professor.cursor_item.take());
            if let Some(item_id) = cursor {
                self.destroy_item(item_id);
            }
        }
    }

    /// C `professor_driver`'s `NT_TEXT` `switch (ret)` body
    /// (`professor.c:393-491`), where `ret = analyse_text_driver(...)`.
    fn professor_handle_text(
        &mut self,
        professor_id: CharacterId,
        professor_name: &str,
        speaker_id: CharacterId,
        text: &str,
    ) {
        let Some(professor) = self.characters.get(&professor_id).cloned() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if char_dist(&professor, &speaker) > PROFESSOR_QA_DISTANCE
            || !char_see_char(&professor, &speaker, &self.map, self.date.daylight)
        {
            return;
        }
        let data = match professor.driver_state.as_ref() {
            Some(CharacterDriverState::Professor(data)) => *data,
            _ => return,
        };
        let Some(&(prof_name, prof_base, _max, prof_step)) = PROF_STATS.get(data.nr as usize)
        else {
            return;
        };

        match analyse_text_qa(text, professor_name, &speaker.name, PROFESSOR_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(professor_id, &reply);
            }
            // C `answer_code == 1` -> `say(cn, "I'm %s.", ch[cn].name)`.
            TextAnalysisOutcome::Matched(1) => {
                self.npc_say(professor_id, &format!("I'm {professor_name}."));
            }
            // C `case 2:` ("teach"/"repeat", `professor.c:394-409`) -
            // `dat->quest` is always `0` in every zone-placed
            // `teach0`-`teach10` template, but the `switch` is kept for
            // fidelity.
            TextAnalysisOutcome::Matched(2) => match data.quest {
                0 => {
                    self.npc_say(
                        professor_id,
                        &format!(
                            "If thou wishest to learn the art of the {prof_name}, thou must pay {} gold coins and {prof_base} profession points. Say {COL_STR_LIGHT_BLUE}learn{COL_STR_RESET} if this is thy wish. Thou canst also {COL_STR_LIGHT_BLUE}improve{COL_STR_RESET} thy knowledge of this art for the fee of {} gold coins and {prof_step} profession points.",
                            data.quest_option,
                            data.improve_cost * prof_step,
                        ),
                    );
                }
                _ => {
                    self.npc_say(professor_id, "You've found bug #418a");
                }
            },
            // C `case 3:` (profession-description words, always describes
            // the professor's own profession, `data.nr` - not the word the
            // player said, a real C quirk preserved verbatim,
            // `professor.c:410-460`).
            TextAnalysisOutcome::Matched(3) => {
                self.npc_say(professor_id, professor_description_text(data.nr));
            }
            // C `case 4:` ("learn", `professor.c:462-479`).
            TextAnalysisOutcome::Matched(4) => match data.quest {
                0 => {
                    if speaker.gold < (data.quest_option as u32).saturating_mul(100) {
                        self.npc_say(
                            professor_id,
                            &format!("But thou cannot afford my fee of {}G.", data.quest_option),
                        );
                    } else if self.learn_prof(professor_id, speaker_id, data.nr as usize) {
                        if let Some(speaker_mut) = self.characters.get_mut(&speaker_id) {
                            speaker_mut.gold = speaker_mut
                                .gold
                                .saturating_sub((data.quest_option as u32).saturating_mul(100));
                            speaker_mut.flags.insert(CharacterFlags::ITEMS);
                        }
                    }
                }
                _ => {
                    self.npc_say(professor_id, "You've found bug #418a");
                }
            },
            // C `case 5:` ("improve", `professor.c:480-490`).
            TextAnalysisOutcome::Matched(5) => {
                let fee = (data.improve_cost * prof_step) as u32;
                if speaker.gold < fee.saturating_mul(100) {
                    self.npc_say(
                        professor_id,
                        &format!("But thou cannot afford my fee of {fee}G."),
                    );
                } else if self.improve_prof(professor_id, speaker_id, data.nr as usize) {
                    if let Some(speaker_mut) = self.characters.get_mut(&speaker_id) {
                        speaker_mut.gold = speaker_mut.gold.saturating_sub(fee.saturating_mul(100));
                        speaker_mut.flags.insert(CharacterFlags::ITEMS);
                    }
                }
            }
            TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
        }
    }

    /// C `professor_driver`'s tail (`professor.c:511-515`):
    /// `secure_move_driver`/`do_idle`, `ret`/`lastact` passed as `0` - same
    /// simplification already accepted for this class of driver (see
    /// `world::npc::area37::nop`'s module doc comment).
    fn process_professor_tick_action(&mut self, professor_id: CharacterId, area_id: u16) {
        let Some(professor) = self.characters.get(&professor_id).cloned() else {
            return;
        };
        let Some(CharacterDriverState::Professor(data)) = professor.driver_state else {
            return;
        };
        self.secure_move_driver(
            professor_id,
            professor.rest_x,
            professor.rest_y,
            data.dir as u8,
            0,
            0,
            area_id,
        );
    }
}

/// C `count_prof(cn)` (`src/common/professor.c:246-255`).
fn count_prof(character: &Character) -> i32 {
    character.professions.iter().filter(|&&p| p != 0).count() as i32
}

/// C `professor_driver`'s `case 3:` `switch (dat->nr)` (`professor.c:
/// 410-460`).
fn professor_description_text(nr: i32) -> &'static str {
    match nr {
        0 => "The art of the athlete are fast, precise movements. Skilled athletes make better use of their endurance and move faster than untrained humans.",
        1 => "The alchemist can create better potions, calling on the powers of the moons and the seasons at any time.",
        2 => "A skilled miner will make better use of every vein of precious metal he finds. He will also not exhaust as fast as an unskilled miner.",
        3 => "The assassin is especially skilled at attacking an enemy from the side or behind, and he can backstab an unware opponent from behind.",
        4 => "A skilled thief can remain unseen even when next to another person. But when he uses this skill of stealth he cannot do anything but wait or walk, and the effort of remaining unseen will drain his endurance.",
        5 => "A master of light will receive a bonus to his basic abilities during the day. If he masters this skill he will also be able to see all undead creatures in the dark.",
        6 => "A master of dark will receive a bonus to his basic abilities during the night. If he masters this skill he will also be able to see all living creatures in the dark.",
        7 => "A skilled trader will get better prices when dealing with merchants.",
        8 => "Those skilled in the art of the mercenary will advance in military rank faster. They will also collect pay for their missions.",
        9 => "A clan master has received special training in the art of clan warfare. He will be at an advantage in any fight in the clan catacombs.",
        10 => "A herbalist knows the art of making plants ripe faster. Any flower, berry or mushroom he picks will grow back in less time.",
        _ => "You've found bug #418b",
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

/// C `struct qa qa[]` from `src/common/professor.c:73-102`.
pub const PROFESSOR_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm fine!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hello"],
        answer: Some("Hello, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hi"],
        answer: Some("Hi, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["greetings"],
        answer: Some("Greetings, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hail"],
        answer: Some("And hail to you, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["help"],
        answer: Some("Sorry, I'm just a merchant, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what", "is", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["buy"],
        answer: Some("Hey %s, use 'buy %s'!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["sell"],
        answer: Some("Hey %s, use 'sell %s'!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["what", "is", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["who", "are", "you"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["teach"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["learn"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["improve"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["athlete"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["alchemist"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["miner"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["assassin"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["thief"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["light"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["light", "warrior"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["dark"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["dark", "warrior"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["trader"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["mercenary"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["clan"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["herbalist"],
        answer: None,
        answer_code: 3,
    },
];
