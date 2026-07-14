//! James, the town drunkard NPC (`CDR_JAMES`), area 1's Lydia-quest
//! hand-off/hardcore-recruiter/paid-advice giver.
//!
//! Ports `src/area/1/gwendylon.c::james_driver` (`:2901-3173`) plus its
//! shared file-local `analyse_text_driver`/`qa` table (`:98-224`, already
//! ported once as [`GWENDYLON_QA`]/[`analyse_text_qa`] for every other
//! area-1 NPC - see `world::lydia`'s module doc comment for the shared
//! table's precedent) and the `james_raisehint`/`can_raise`/
//! `get_fight_skill_skill` helpers (`:5235-5311` and `:5311-5962`).
//! Follows the same `World`/`PlayerRuntime` split established by every
//! sibling NPC: the caller supplies a per-player fact snapshot
//! ([`JamesPlayerFacts`]) up front and applies the returned
//! [`JamesOutcomeEvent`]s afterwards, since `james_state` (`area1_ppd`)
//! lives on `crate::player::PlayerRuntime`, not `World`.
//!
//! Deviations/gaps (documented, not silent):
//! - The hardcore-invite line (`gwendylon.c:2972-2974`) and the
//!   "advice"/"buy advice" invite/fee lines (`:3013-3015`, `:3058-3060`)
//!   wrap parts of their text in `COL_LIGHT_RED`/`COL_LIGHT_BLUE`/
//!   `COL_RESET` byte markers in C; restored via `COL_STR_LIGHT_RED`/
//!   `COL_STR_LIGHT_BLUE`/`COL_STR_RESET` sentinels and
//!   `World::npc_quiet_say_bytes`, same mechanism as `world::camhermit`.
//! - `dlog(co, 0, "turned hardcore through James")` (`gwendylon.c:3133`)
//!   is dropped - no Rust `dlog` sink exists (same established gap as
//!   `world::exp`/`world::npc::trader`/`world::consistency`).
//! - The C struct `james_driver_data`'s `nighttime` field
//!   (`gwendylon.c:2895`) is never read or written anywhere in
//!   `james_driver`'s own body (confirmed: the only other `nighttime`
//!   fields, on `gwendylon_driver_data`/`yoakin_driver_data`/
//!   `guiwynn_driver_data`, are equally dead - same documented precedent
//!   on those NPCs' own module doc comments) - not carried here.
//! - **`raiseme` (`answer_code` 10, `CF_GOD`-only "raise me" debug
//!   command, `gwendylon.c:3074-3100`) and the equipment it grants
//!   (`james_create_eq`/`james_create_armor_piece`/`james_create_weapon`/
//!   `james_create_equipment`, `:5964-6074`) are deliberately NOT
//!   ported.** This is a `CF_GOD`-only developer/admin debug tool (not
//!   reachable by any real player), and porting it faithfully would
//!   require a whole new profession subsystem (`prof[].base`/`.step`,
//!   `free_prof_points`) that nothing else in this codebase needs yet -
//!   see the "Area 1" P4 task note in `PORTING_TODO.md`. The text
//!   command is recognized (via the shared `GWENDYLON_QA` table, code
//!   10) but is a no-op in Rust; `didsay` still becomes true (matching
//!   C's own `if (didsay)` gate, which only depends on
//!   `analyse_text_driver`'s return code, not on `CF_GOD` or any
//!   command-specific side effect).
//! - [`james_raisehint_advice`] therefore only implements C's
//!   `james_raisehint(cn, 0)` (the advice-only path, reachable by any
//!   player via "buy advice"): the full weighted-priority computation,
//!   the four advice-tier messages, and the balance-summary line. The
//!   `doraise == 1` tail (the `>0.90` threshold actually calling
//!   `raise_value` instead of logging, and the "Set professions"
//!   auto-learn block, `gwendylon.c:5836-5849`/`:5893-5957`) is dead code
//!   in this port since `doraise` is always `0` here - documented, not
//!   silently dropped.
//! - `get_fight_skill_skill`'s C fallback (`return 0;` i.e. `V_HP`, when
//!   wielding a `IF_WEAPON`-flagged item with none of
//!   `IF_DAGGER`/`IF_STAFF`/`IF_SWORD`/`IF_TWOHAND`, `gwendylon.c:5299`)
//!   would read `skill[0].base1 == -1` as an out-of-bounds
//!   `ch[cn].value[1][-1]` in C (undefined behavior, never actually
//!   reachable since no real weapon template lacks all four flags).
//!   [`james_fight_skill_index`] reproduces the same `0` index, but
//!   [`james_add_base_raise`] safely no-ops for it (`skill_base_
//!   attributes(Hp)` returns `None`) instead of attempting to reproduce
//!   the C UB.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON, GWENDYLON_QA};
use crate::drvlib::offset2dx;
use crate::item_driver::{bare_value, raise_cost};
use crate::legacy::INVENTORY_START_INVENTORY;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_LIGHT_RED, COL_STR_RESET};
use crate::world::character_values::{character_value_from_index, skill_base_attributes};
use crate::world::values::skill_display_name;
use crate::world::*;

/// C `char_dist(cn, co) > 15` (`gwendylon.c:2950`): the `NT_CHAR`
/// greeting range.
const JAMES_GREET_DISTANCE: i32 = 15;
/// C `char_dist(cn, co) > 12` (`gwendylon.c:145`, the shared
/// `analyse_text_driver` copy's own guard).
const JAMES_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`gwendylon.c:2933`).
const JAMES_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`gwendylon.c:2938`, `:3035`).
const JAMES_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`gwendylon.c:3166`): idle "return to post" threshold.
const JAMES_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `#define AF1_STORAGE_HINT (1u << 1)` (`src/area/1/area1.h:21`).
const AF1_STORAGE_HINT: i32 = 1 << 1;

/// Per-player facts [`World::process_james_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JamesPlayerFacts {
    /// `PlayerRuntime::area1_james_state()`.
    pub james_state: i32,
    /// `PlayerRuntime::area1_lydia_state()`, read by James's own state 0
    /// (`ppd->lydia_state >= 6` skip-ahead) and state 3 (`ppd->lydia_state
    /// < 6` gate).
    pub lydia_state: i32,
    /// `PlayerRuntime::area1_flags()` (`area1_ppd.flags`), read for
    /// `AF1_STORAGE_HINT`.
    pub area1_flags: i32,
}

/// A side effect [`World::process_james_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JamesOutcomeEvent {
    /// Write the new `area1_ppd.james_state` back.
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->flags |= AF1_STORAGE_HINT;` (`gwendylon.c:2964`).
    SetStorageHint { player_id: CharacterId },
    /// C `questlog_open(co, QLOG_LYDIA)` (`gwendylon.c:2986`) - James's
    /// own state-0 greeting opens the same quest Lydia's own state-0
    /// greeting does (both are the player's very first hint about the
    /// hangover-potion quest chain).
    QuestOpen { player_id: CharacterId },
}

impl World {
    /// C `james_driver`'s per-tick body (`gwendylon.c:2901-3173`).
    pub fn process_james_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, JamesPlayerFacts>,
        area_id: u16,
    ) -> Vec<JamesOutcomeEvent> {
        let james_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_JAMES
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for james_id in james_ids {
            self.process_james_messages(james_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_james_messages(
        &mut self,
        james_id: CharacterId,
        player_facts: &HashMap<CharacterId, JamesPlayerFacts>,
        area_id: u16,
        events: &mut Vec<JamesOutcomeEvent>,
    ) {
        let Some(james_name) = self
            .characters
            .get(&james_id)
            .map(|james| james.name.clone())
        else {
            return;
        };
        let mut data = match self
            .characters
            .get(&james_id)
            .and_then(|james| james.driver_state.clone())
        {
            Some(CharacterDriverState::James(data)) => data,
            _ => JamesDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&james_id)
            .map(|james| std::mem::take(&mut james.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.james_handle_char_message(
                    james_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.james_handle_text_message(
                    james_id,
                    &james_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.james_handle_give_message(james_id, message),
                _ => {}
            }
        }

        if let Some(james) = self.characters.get_mut(&james_id) {
            james.driver_state = Some(CharacterDriverState::James(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`gwendylon.c:3162-3164`).
        if let (Some(james), Some((tx, ty))) =
            (self.characters.get(&james_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(james.x), i32::from(james.y), tx, ty) {
                if let Some(james_mut) = self.characters.get_mut(&james_id) {
                    let _ = turn(james_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHTDOWN,
        // ret, lastact)) return; }` (`gwendylon.c:3166-3170`). The NPC's
        // post position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the
        // same substitution every other stationary area-1 NPC uses.
        let last_talk = match self.characters.get(&james_id) {
            Some(james) => match james.driver_state.as_ref() {
                Some(CharacterDriverState::James(data)) => data.last_talk,
                _ => return,
            },
            None => return,
        };
        if last_talk + JAMES_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(james) = self.characters.get(&james_id) else {
                return;
            };
            let (post_x, post_y) = (james.rest_x, james.rest_y);
            self.secure_move_driver(
                james_id,
                post_x,
                post_y,
                Direction::RightDown as u8,
                0,
                0,
                area_id,
            );
        }
        // C `do_idle(cn, TICKS);` (`gwendylon.c:3172`) - not modeled, same
        // precedent as every other stationary dialogue-only area-1 NPC
        // (`world::terion`/`world::brithildie`/...): it has no observable
        // effect in this message-driven architecture.
    }

    /// C `james_driver`'s `NT_CHAR` branch (`gwendylon.c:2916-3029`).
    #[allow(clippy::too_many_arguments)]
    fn james_handle_char_message(
        &mut self,
        james_id: CharacterId,
        data: &mut JamesDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JamesPlayerFacts>,
        events: &mut Vec<JamesOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(james) = self.characters.get(&james_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`gwendylon.c:2920-2924`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`gwendylon.c:2926-2930`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`gwendylon.c:2932-2936`).
        if tick < data.last_talk + JAMES_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`gwendylon.c:2938-
        // 2941`).
        if tick < data.last_talk + JAMES_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`gwendylon.c:2943-2947`).
        if james_id == player_id || !char_see_char(&james, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 15) continue;` (`gwendylon.c:2949-
        // 2953`).
        if char_dist(&james, &player) > JAMES_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.james_state;

        // C `if (has_empty_inventory(co) && !(ppd->flags &
        // AF1_STORAGE_HINT)) { ... }` (`gwendylon.c:2959-2966`).
        if james_has_empty_inventory(&player) && facts.area1_flags & AF1_STORAGE_HINT == 0 {
            self.npc_quiet_say(
                james_id,
                &format!(
                    "Shouldst thou need new equipment, {}, try the chests in the western corner. If thou hast not used them yet, they should contain all thou needst.",
                    player.name
                ),
            );
            events.push(JamesOutcomeEvent::SetStorageHint { player_id });
            didsay = true;
        }

        // C `switch (ppd->james_state) { ... }` (`gwendylon.c:2968-3022`).
        match facts.james_state {
            0 => {
                // C `if ((ch[co].flags & CF_PAID) && ch[co].exp == 0 &&
                // !(ch[co].flags & CF_HARDCORE)) { quiet_say(...); }`
                // (`gwendylon.c:2970-2975`) - unconditional (not gated by
                // the branch below), COL markers dropped, see the module
                // doc comment.
                if player.flags.contains(CharacterFlags::PAID)
                    && player.exp == 0
                    && !player.flags.contains(CharacterFlags::HARDCORE)
                {
                    self.npc_quiet_say_bytes(
                        james_id,
                        &format!(
                            "{COL_STR_LIGHT_RED}Hello, {}. Dost thou wish to become a {COL_STR_LIGHT_BLUE}Hardcore{COL_STR_LIGHT_RED} character?",
                            player.name
                        ),
                    );
                }
                // C `if (ppd->lydia_state >= 6) { ppd->james_state = 3;
                // break; }` (`gwendylon.c:2976-2979`) - no `didsay`.
                if facts.lydia_state >= 6 {
                    new_state = 3;
                } else {
                    self.npc_quiet_say(
                        james_id,
                        &format!(
                            "Ah, hello there, {}. I am {}. Couldst thou do me a favour? Last night I accompanied the mages' daughter Lydia to a party. At some point I passed out!",
                            player.name, james.name
                        ),
                    );
                    new_state = 1;
                    didsay = true;
                    events.push(JamesOutcomeEvent::QuestOpen { player_id });
                }
            }
            1 => {
                self.npc_quiet_say(
                    james_id,
                    "I dare not go back and apologize myself, couldst thou go visit Lydia and make sure she got home safely without my protection?",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`gwendylon.c:2996-3001`): the C `quiet_say` for
            // this state is commented out in the legacy source itself (a
            // dead line, not a missed port - `// quiet_say(cn, "The
            // robbers are west of here, ...");`), so no text is spoken
            // here even though the state still advances and `didsay`
            // still becomes true.
            2 => {
                new_state = 3;
                didsay = true;
            }
            3 => {
                // C `if (ppd->lydia_state < 6) break;` (`gwendylon.c:3004-
                // 3006`) - no `didsay`.
                if facts.lydia_state >= 6 {
                    self.npc_quiet_say(
                        james_id,
                        &format!("Ah, {}. I am glad that thou could help Lydia.", player.name),
                    );
                    new_state = 4;
                    didsay = true;
                }
            }
            4 => {
                self.npc_quiet_say_bytes(
                    james_id,
                    &format!(
                        "If you ever need {COL_STR_LIGHT_BLUE}advice{COL_STR_RESET} on how to raise your character, I'd be happy to help you - for a small fee."
                    ),
                );
                new_state = 5;
                didsay = true;
            }
            // 5: break (no-op).
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`gwendylon.c:3023-3027`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
        if new_state != facts.james_state {
            events.push(JamesOutcomeEvent::UpdateState {
                player_id,
                new_state,
            });
        }
    }

    /// C `james_driver`'s `NT_TEXT` branch (`gwendylon.c:3032-3140`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern
    /// as every other area-1 NPC's text handler).
    #[allow(clippy::too_many_arguments)]
    fn james_handle_text_message(
        &mut self,
        james_id: CharacterId,
        james_name: &str,
        data: &mut JamesDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JamesPlayerFacts>,
        events: &mut Vec<JamesOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`gwendylon.c:3035-3037`).
        let tick = self.tick.0;
        if tick > data.last_talk + JAMES_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`gwendylon.c:3039-3042`).
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
        if james_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(james) = self.characters.get(&james_id).cloned() else {
            return;
        };
        if char_dist(&james, &speaker) > JAMES_QA_DISTANCE
            || !char_see_char(&james, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let mut didsay = false;
        match analyse_text_qa(text, james_name, &speaker.name, GWENDYLON_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(james_id, &reply);
                didsay = true;
            }
            // C `case 2: // repeat` (`gwendylon.c:3045-3051`).
            TextAnalysisOutcome::Matched(2) => {
                if let Some(facts) = player_facts.get(&speaker_id) {
                    if (0..=3).contains(&facts.james_state) {
                        events.push(JamesOutcomeEvent::UpdateState {
                            player_id: speaker_id,
                            new_state: 0,
                        });
                        data.last_talk = 0;
                    }
                }
                didsay = true;
            }
            // C `case 3: // advice` (`gwendylon.c:3053-3062`).
            TextAnalysisOutcome::Matched(3) => {
                if speaker.level > 70 {
                    self.npc_quiet_say(
                        james_id,
                        &format!(
                            "I'm afraid I cannot help thee, {}. Thou art much wiser than I am.",
                            speaker.name
                        ),
                    );
                } else {
                    let fee = james_advice_fee_gold(speaker.level);
                    self.npc_quiet_say_bytes(
                        james_id,
                        &format!(
                            "I'll help thee for the small fee of {fee:.2}G, {}. Say {COL_STR_LIGHT_BLUE}buy advice{COL_STR_RESET} if thou wantst it.",
                            speaker.name
                        ),
                    );
                }
                didsay = true;
            }
            // C `case 4: // buy advice` (`gwendylon.c:3064-3072`).
            TextAnalysisOutcome::Matched(4) => {
                if speaker.level > 70 {
                    self.npc_quiet_say(
                        james_id,
                        &format!(
                            "I'm afraid I cannot help thee, {}. Thou art much wiser than I am.",
                            speaker.name
                        ),
                    );
                } else {
                    let cost = james_advice_cost_money(speaker.level);
                    if self.james_take_money(speaker_id, cost) {
                        self.james_raisehint_advice(speaker_id);
                    } else {
                        self.npc_quiet_say(
                            james_id,
                            &format!("Thou dost not have enough money, {}.", speaker.name),
                        );
                    }
                }
                didsay = true;
            }
            // C `case 10: // raise me` (`gwendylon.c:3074-3100`) - the
            // `CF_GOD`-only debug command; deliberately not ported, see
            // the module doc comment. `didsay` still becomes true since
            // C's own `if (didsay)` only depends on the returned code,
            // not on `CF_GOD` or any command-specific effect.
            TextAnalysisOutcome::Matched(10) => {
                didsay = true;
            }
            // C `case 11: // hardcore` (`gwendylon.c:3102-3114`).
            TextAnalysisOutcome::Matched(11) => {
                self.npc_quiet_say(
                    james_id,
                    "Hardcore is an option. A hardcore character does not earn saves when he levels, and he loses a lot more experience on death than a normal character. But he can train his skills higher than any other character.",
                );
                self.npc_quiet_say(
                    james_id,
                    "Since death is a lot harder on hardcore characters, thou must be aware that the gods (game management) will ignore any complaints about deaths caused by lag, other players luring monsters to thee or other reasons which are not entirely fair, but not real bugs either.",
                );
                self.npc_quiet_say(
                    james_id,
                    "So, the rule is: It is thy choice to become a hardcore character, and thou must live with the consequences of that choice. The gods wilt not help thee. Dost thou accept these rules? [ I accept the rules and wish to become a hardcore character ]",
                );
                didsay = true;
            }
            // C `case 12: // i accept the rules ...` (`gwendylon.c:3116-
            // 3134`).
            TextAnalysisOutcome::Matched(12) => {
                if !speaker.flags.contains(CharacterFlags::PAID) {
                    self.npc_quiet_say(
                        james_id,
                        "But thou art not a paying player. Thou mayest not become a hardcore character.",
                    );
                } else if speaker.flags.contains(CharacterFlags::HARDCORE) {
                    self.npc_quiet_say(james_id, "But thou art a hardcore character already.");
                } else if speaker.exp != 0 {
                    self.npc_quiet_say(
                        james_id,
                        "But thou hast already earned experience. Thou mayest not become a hardcore character.",
                    );
                } else {
                    self.npc_quiet_say(
                        james_id,
                        &format!(
                            "So be it. Good luck, {}. Mayest thou never regret this decision.",
                            speaker.name
                        ),
                    );
                    if let Some(target) = self.characters.get_mut(&speaker_id) {
                        target.flags.insert(CharacterFlags::HARDCORE);
                        target.saves = 0;
                    }
                    // C `dlog(co, 0, "turned hardcore through James");`
                    // (`gwendylon.c:3133`) - dropped, see the module doc
                    // comment.
                }
                didsay = true;
            }
            // Every other matched code (e.g. `9`, promise/word/oath) is
            // unhandled by James's own C `switch` but still counts as
            // `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`gwendylon.c:3136-3139`).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `james_driver`'s `NT_GIVE` branch (`gwendylon.c:3143-3154`): the
    /// plain (non-drop-fallback) give, matching C's own plain
    /// `give_char_item` (same pattern as every sibling NPC's own
    /// catch-all give handler).
    fn james_handle_give_message(
        &mut self,
        james_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&james_id)
            .and_then(|james| james.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            james_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }

    /// C `take_money(cn, val)` (`src/system/tool.c:3820-3826`), a private
    /// copy matching every other NPC's own inline `take_money` copy (see
    /// `world::gatekeeper::gate_take_money`'s doc comment for the same
    /// precedent).
    fn james_take_money(&mut self, player_id: CharacterId, amount: u32) -> bool {
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

    /// C `james_raisehint(cn, 0)` (`gwendylon.c:5311-5962`) - the
    /// advice-only path. See the module doc comment for why `doraise ==
    /// 1` is not ported.
    fn james_raisehint_advice(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return;
        };
        let seyan = character.flags.contains(CharacterFlags::WARRIOR)
            && character.flags.contains(CharacterFlags::MAGE);
        // C `int class_type = 0; if (CF_MAGE) class_type = 1; if
        // (CF_MAGE|CF_WARRIOR) class_type = 2;` (`gwendylon.c:5317-5324`).
        let class_type = if seyan {
            2
        } else if character.flags.contains(CharacterFlags::MAGE) {
            1
        } else {
            0
        };

        let mut raise = [0.0_f64; CHARACTER_VALUE_COUNT];
        let blessmod = if bare_value(&character, CharacterValue::Bless as usize) != 0 {
            1.4
        } else {
            1.0
        };

        // C's flash/fire mutual-exclusivity pick (`gwendylon.c:5335-
        // 5352`): `flash`/`fire` default to `1` (both true) and are only
        // overridden when one value clearly dominates the other; if
        // neither dominates, a `RANDOM(2)` tie-break picks one.
        let flash_bare = bare_value(&character, CharacterValue::Flash as usize);
        let fire_bare = bare_value(&character, CharacterValue::Fireball as usize);
        let mut flash = true;
        let mut fire = true;
        if flash_bare > 1 && flash_bare > fire_bare {
            fire = false;
        }
        if fire_bare > 1 && fire_bare > flash_bare {
            flash = false;
        }
        if flash && fire {
            if legacy_random_below_from_seed(&mut self.legacy_random_seed, 2) != 0 {
                flash = false;
            } else {
                fire = false;
            }
        }
        let warcry_class = class_type == 0 || class_type == 2;

        let fight_idx = james_fight_skill_index(&character, &self.items);
        let fight_value = character_value_from_index(fight_idx).unwrap_or(CharacterValue::Hp);

        // ---- offense (`gwendylon.c:5356-5492`) ----
        james_add_raise(&mut raise, &character, seyan, CharacterValue::Attack, 1.0);
        if bare_value(&character, CharacterValue::Attack as usize) == 0 {
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Bless, 16.0);
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Heal, 16.0);
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Freeze, 16.0);
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::MagicShield,
                16.0,
            );
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Flash, 16.0);
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Fireball,
                16.0,
            );
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Pulse, 16.0);
        }
        james_add_raise(&mut raise, &character, seyan, fight_value, 2.0);
        james_add_raise(&mut raise, &character, seyan, CharacterValue::Tactics, 4.0);
        if bare_value(&character, CharacterValue::Attack as usize) == 0 {
            james_add_base_raise(&mut raise, &character, seyan, fight_value, 10.0 / blessmod);
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::MagicShield,
                10.0 / 2.0 / blessmod,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Bless,
                40.0 / 3.0 / 3.0,
            );
        } else {
            james_add_base_raise(&mut raise, &character, seyan, fight_value, 10.0 / blessmod);
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Attack,
                10.0 / 2.0 / blessmod,
            );
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Tactics,
                20.0 / blessmod,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Bless,
                40.0 / 3.5 / 3.0,
            );
        }

        // ---- defense (`gwendylon.c:5494-5612`) ----
        james_add_raise(&mut raise, &character, seyan, CharacterValue::Parry, 1.0);
        if bare_value(&character, CharacterValue::Parry as usize) == 0 {
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::MagicShield,
                1.0,
            );
        }
        james_add_raise(&mut raise, &character, seyan, fight_value, 2.0);
        james_add_raise(&mut raise, &character, seyan, CharacterValue::Tactics, 4.0);
        if bare_value(&character, CharacterValue::Parry as usize) == 0 {
            james_add_base_raise(&mut raise, &character, seyan, fight_value, 10.0 / blessmod);
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::MagicShield,
                10.0 / 2.0 / blessmod,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Bless,
                40.0 / 2.0 / 3.0,
            );
        } else {
            james_add_base_raise(&mut raise, &character, seyan, fight_value, 10.0 / blessmod);
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Parry,
                10.0 / 2.0 / blessmod,
            );
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Tactics,
                20.0 / blessmod,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Bless,
                40.0 / 3.5 / 3.0,
            );
        }

        // ---- immunity (`gwendylon.c:5614-5678`) ----
        james_add_raise(&mut raise, &character, seyan, CharacterValue::Immunity, 1.0);
        james_add_raise(&mut raise, &character, seyan, CharacterValue::Tactics, 5.0);
        if bare_value(&character, CharacterValue::Tactics as usize) == 0 {
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Immunity,
                5.0 / blessmod,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Bless,
                20.0 / 3.0,
            );
        } else {
            // C re-adds the same immunity-base spread even in this
            // branch (`gwendylon.c:5643-5654`), then additionally spreads
            // onto tactics's own base attributes - verbatim, not a typo.
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Immunity,
                5.0 / blessmod,
            );
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Tactics,
                25.0 / blessmod,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Bless,
                20.0 / 1.2 / 3.0,
            );
        }

        // ---- flash (`gwendylon.c:5680-5702`) ----
        if flash && flash_bare != 0 && class_type != 2 {
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Flash, 1.0);
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Flash,
                5.0 / blessmod,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Bless,
                20.0 / 3.0,
            );
        }

        // ---- fire (`gwendylon.c:5704-5726`) ----
        if fire && fire_bare != 0 && class_type != 2 {
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Fireball, 1.0);
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Fireball,
                5.0 / blessmod,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Bless,
                20.0 / 3.0,
            );
        }

        // ---- pulse (`gwendylon.c:5728-5750`) ----
        let pulse_bare = bare_value(&character, CharacterValue::Pulse as usize);
        if pulse_bare != 0 && class_type != 2 {
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Pulse, 1.0);
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Pulse,
                5.0 / blessmod,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Bless,
                20.0 / 3.0,
            );
        }

        // ---- warcry (`gwendylon.c:5752-5770`) ----
        let warcry_bare = bare_value(&character, CharacterValue::Warcry as usize);
        if warcry_class && warcry_bare != 0 {
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Warcry, 2.0);
            james_add_base_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::Warcry,
                10.0 / blessmod,
            );
        }

        // ---- warrior-only new skills (`gwendylon.c:5772-5784`) ----
        if class_type == 0 || class_type == 2 {
            james_add_raise(&mut raise, &character, seyan, CharacterValue::Surround, 3.0);
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::BodyControl,
                3.0,
            );
            james_add_raise(
                &mut raise,
                &character,
                seyan,
                CharacterValue::SpeedSkill,
                3.0,
            );
        }

        // ---- rage/warcry dampening (`gwendylon.c:5786-5792`) ----
        if james_can_raise(&character, CharacterValue::Rage as usize) {
            raise[CharacterValue::Rage as usize] *= 0.7;
        }
        if james_can_raise(&character, CharacterValue::Warcry as usize) {
            raise[CharacterValue::Warcry as usize] *= 0.8;
        }

        // ---- misc (`gwendylon.c:5794-5827`) ----
        // Index-based loop kept: mirrors C's `for (v = 0; v < V_MAX; v++)`
        // reading and writing `raise[v]` in place (`gwendylon.c:5795-5827`).
        #[allow(clippy::needless_range_loop)]
        for idx in 0..CHARACTER_VALUE_COUNT {
            let value = character_value_from_index(idx).unwrap();
            let weight = match value {
                CharacterValue::Hp => {
                    if class_type == 0 {
                        2.0
                    } else {
                        5.0
                    }
                }
                CharacterValue::Mana => {
                    if class_type == 1 {
                        1.0
                    } else {
                        3.0
                    }
                }
                CharacterValue::Endurance => {
                    if warcry_class {
                        4.0
                    } else {
                        8.0
                    }
                }
                CharacterValue::ArmorSkill => 1.25,
                CharacterValue::Duration | CharacterValue::Rage => 4.0,
                CharacterValue::Profession => 2.0,
                CharacterValue::Regenerate | CharacterValue::Meditate => 4.0,
                _ => continue,
            };
            if raise[idx] == 0.0 && james_can_raise(&character, idx) {
                let current = bare_value(&character, idx);
                let cost = f64::from(raise_cost(idx, current, seyan));
                raise[idx] = 1.0 / (cost * weight);
            }
        }

        // ---- threshold messages + balance summary (`gwendylon.c:5829-
        // 5959`) ----
        let mr = raise.iter().copied().fold(0.0_f64, f64::max);
        if mr == 0.0 {
            return;
        }

        let mut done = [false; CHARACTER_VALUE_COUNT];
        let mut messages = Vec::new();

        for idx in 0..CHARACTER_VALUE_COUNT {
            if raise[idx] / mr > 0.90 && !done[idx] {
                messages.push(format!(
                    "You should definitely raise {}.",
                    james_value_name(idx)
                ));
                done[idx] = true;
            }
        }
        for idx in 0..CHARACTER_VALUE_COUNT {
            if raise[idx] / mr > 0.80 && !done[idx] {
                messages.push(format!(
                    "You should consider raising {}.",
                    james_value_name(idx)
                ));
                done[idx] = true;
            }
        }
        for idx in 0..CHARACTER_VALUE_COUNT {
            if raise[idx] / mr > 0.65 && !done[idx] {
                messages.push(format!(
                    "You might raise {}, but you probably shouldn't.",
                    james_value_name(idx)
                ));
                done[idx] = true;
            }
        }
        for idx in 0..CHARACTER_VALUE_COUNT {
            if raise[idx] != 0.0
                && raise[idx] / mr < 0.30
                && !done[idx]
                && idx != CharacterValue::Freeze as usize
                && idx != CharacterValue::Fireball as usize
                && idx != CharacterValue::Flash as usize
                && idx != CharacterValue::Pulse as usize
            {
                messages.push(format!(
                    "You should not raise {} for a while.",
                    james_value_name(idx)
                ));
                done[idx] = true;
            }
        }

        let mut sum = 0.0_f64;
        let mut cnt = 0_u32;
        // Index-based loop kept: mirrors C's balance-summary pass over
        // `raise[v]` (`gwendylon.c:5870-5874`).
        #[allow(clippy::needless_range_loop)]
        for idx in 0..CHARACTER_VALUE_COUNT {
            if raise[idx] != 0.0 && idx != CharacterValue::Freeze as usize {
                sum += raise[idx] / mr;
                cnt += 1;
            }
        }
        if cnt > 0 {
            sum /= f64::from(cnt);
            let line = if sum >= 0.90 {
                "Your character seems to be very well balanced indeed."
            } else if sum > 0.80 {
                "Your character seems to be very well balanced."
            } else if sum > 0.70 {
                "Your character seems to be well balanced."
            } else if sum > 0.60 {
                "Your character seems to be fairly well balanced."
            } else if sum > 0.30 {
                "Your character seems to be somewhat unbalanced."
            } else {
                "Your character seems to be very unbalanced."
            };
            messages.push(line.to_string());
        }

        messages.push(
            "Please rely on your own judgement, too. I am just James the drunkard, and I might very well be wrong..."
                .to_string(),
        );

        for message in messages {
            self.queue_system_text(character_id, message);
        }
    }
}

/// C `has_empty_inventory(cn)` (`gwendylon.c:2874-2890`).
fn james_has_empty_inventory(character: &Character) -> bool {
    for slot in character.inventory.iter().take(12) {
        if slot.is_some() {
            return false;
        }
    }
    for slot in character
        .inventory
        .iter()
        .take(INVENTORY_SIZE)
        .skip(INVENTORY_START_INVENTORY)
    {
        if slot.is_some() {
            return false;
        }
    }
    true
}

/// C `can_raise(cn, v)` (`gwendylon.c:5235-5264`).
fn james_can_raise(character: &Character, value: usize) -> bool {
    let bare = bare_value(character, value);
    if bare == 0 {
        return false;
    }
    if !character.flags.contains(CharacterFlags::ARCH) && bare > 49 {
        return false;
    }
    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    if seyan && bare > 99 {
        return false;
    }
    if bare > 114 {
        return false;
    }
    if value == CharacterValue::Profession as usize && bare > 99 {
        return false;
    }
    true
}

/// C `get_fight_skill_skill(cn)` (`gwendylon.c:5266-5300`). See the
/// module doc comment for the `0`-index fallback quirk.
fn james_fight_skill_index(character: &Character, items: &HashMap<ItemId, Item>) -> usize {
    let Some(item_id) = character
        .inventory
        .get(worn_slot::RIGHT_HAND)
        .copied()
        .flatten()
    else {
        return CharacterValue::Hand as usize;
    };
    let Some(item) = items.get(&item_id) else {
        return CharacterValue::Hand as usize;
    };
    if !item.flags.intersects(ItemFlags::WEAPON) {
        return CharacterValue::Hand as usize;
    }
    if item.flags.contains(ItemFlags::DAGGER) {
        return CharacterValue::Dagger as usize;
    }
    if item.flags.contains(ItemFlags::STAFF) {
        return CharacterValue::Staff as usize;
    }
    if item.flags.contains(ItemFlags::SWORD) {
        return CharacterValue::Sword as usize;
    }
    if item.flags.contains(ItemFlags::TWOHAND) {
        return CharacterValue::TwoHand as usize;
    }
    0
}

/// `raise[value] += 1.0 / (raise_cost(value, ...) * divisor)`, gated by
/// `can_raise` - the single repeated shape of nearly every line in C
/// `james_raisehint` (`gwendylon.c:5311-5827`).
fn james_add_raise(
    raise: &mut [f64; CHARACTER_VALUE_COUNT],
    character: &Character,
    seyan: bool,
    value: CharacterValue,
    divisor: f64,
) {
    let idx = value as usize;
    if james_can_raise(character, idx) {
        let current = bare_value(character, idx);
        let cost = f64::from(raise_cost(idx, current, seyan));
        raise[idx] += 1.0 / (cost * divisor);
    }
}

/// Spreads `james_add_raise` onto `value`'s three C `skill[].base1/2/3`
/// attributes (`skill_base_attributes`); a no-op for values with no base
/// attributes (powers, attributes, `Armor`/`Weapon`/`Light`, `Cold`,
/// `Profession` - C's `-1,-1,-1` rows).
fn james_add_base_raise(
    raise: &mut [f64; CHARACTER_VALUE_COUNT],
    character: &Character,
    seyan: bool,
    value: CharacterValue,
    divisor: f64,
) {
    let Some((base1, base2, base3)) = skill_base_attributes(value) else {
        return;
    };
    james_add_raise(raise, character, seyan, base1, divisor);
    james_add_raise(raise, character, seyan, base2, divisor);
    james_add_raise(raise, character, seyan, base3, divisor);
}

fn james_value_name(idx: usize) -> &'static str {
    character_value_from_index(idx)
        .map(skill_display_name)
        .unwrap_or("Unknown")
}

/// C `ch[co].level * ch[co].level * ch[co].level / 100.0` (`gwendylon.c:
/// 3058-3060`), the displayed "advice" fee (gold is stored in hundredths,
/// same convention as every other `{:.2}G` formatter in this codebase).
fn james_advice_fee_gold(level: u32) -> f64 {
    let cube = u64::from(level)
        .saturating_mul(u64::from(level))
        .saturating_mul(u64::from(level));
    cube as f64 / 100.0
}

/// C `take_money(co, ch[co].level * ch[co].level * ch[co].level)`
/// (`gwendylon.c:3067`) - the raw money amount actually charged (not
/// divided by 100, unlike the displayed fee above).
fn james_advice_cost_money(level: u32) -> u32 {
    let cube = u64::from(level)
        .saturating_mul(u64::from(level))
        .saturating_mul(u64::from(level));
    cube.min(u64::from(u32::MAX)) as u32
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct james_driver_data` (`src/area/1/gwendylon.c:2892-2896`): the
/// town-drunkard NPC's own driver memory (`CDR_JAMES`, distinct from the
/// per-player `james_state` field in `crate::player::PlayerRuntime`'s
/// `area1_ppd` - see `world::james`'s module doc comment for the split).
/// C's own `nighttime` field is never read or written anywhere in
/// `james_driver`'s own body - dropped, see the module doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JamesDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
