//! Kassim NPC (`CDR_KASSIM`), Aston's jewelry engraver.
//!
//! Ports `src/area/3/area3.c::kassim_driver` (`:327-656`) plus the shared
//! `AREA3_QA` table's `explain`(9) entry this driver needs
//! (`area3.c:122`). `DRD_ENGRAVE_DATA` (C `struct engrave_data { char
//! text[80]; }`, `area3.c:318-320`) - the transient pending-inscription
//! text stashed on the *speaking player* while the engraving transaction
//! is in progress - is ported as a new `CharacterDriverState::Engrave`
//! variant: same "player, not NPC" precedent as `ClanFoundData`
//! (`character_driver.rs`), since the player is mid-transaction with
//! Kassim, not Kassim himself. `area3_ppd.kassim_state`/`kassim_seen_timer`
//! /`kassim_item_wait_starttime` live on `crate::player::PlayerRuntime`
//! (C wall-clock `realtime` seconds, not tick count, for the two timer
//! fields) - same split as `world::lydia`: the caller supplies a
//! per-player fact snapshot ([`KassimPlayerFacts`]) plus `now` up front
//! and applies the returned [`KassimOutcomeEvent`]s afterwards.
//!
//! Deviations/gaps (documented, not silent):
//! - The `"engrave: "`(8) qa-table row is genuinely dead code in C:
//!   `kassim_driver`'s `NT_TEXT` branch special-cases
//!   `strcasestr(msg->dat2, "engrave:")` *before* ever calling
//!   `analyse_text_driver` (`area3.c:479-499`), so `analyse_text_qa`
//!   never sees that literal text - not ported (see `AREA3_QA`'s own doc
//!   comment).
//! - `KASSIM_STATE_EXPLAIN`/`ITEM_GOT`/`TAKE_MONEY` are logically-
//!   unreachable `elog`-only branches in C's own `NT_CHAR` switch (their
//!   real transitions happen in the `NT_TEXT`/`NT_GIVE` branches instead,
//!   `area3.c:397-400,436-444`) - omitted here too, matching C's own
//!   dead-branch comments.
//! - C's `NT_GIVE` branch only sets `didsay` (and therefore only turns to
//!   face/remembers `current_victim`) for the "already engraved", "not
//!   wearable", and "engrave data missing" (error) sub-branches - *not*
//!   for the successful engrave, the "cannot pay", or the "wrong state"
//!   fallback branches, even though all of those also `quiet_say`
//!   (`area3.c:539-611`). This looks like an oversight in the original C
//!   but is preserved exactly per the porting rules (`AGENTS.md`: "copy
//!   ... stupid-looking edge cases").

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::entity::ItemFlags;
use crate::world::*;

use super::AREA3_QA;

/// C `char_dist(cn, co) > 10` (`area3.c:379`).
const KASSIM_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`area3.c:232`, the shared
/// `analyse_text_driver` copy's own guard).
const KASSIM_QA_DISTANCE: i32 = 12;
/// C `TICKS * 10` (`area3.c:361`).
const KASSIM_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 20` (`area3.c:367`, `:470`).
const KASSIM_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 20;
/// C `TICKS * 30` (`area3.c:629`): idle "return to post" threshold.
const KASSIM_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS * 60` (`area3.c:636`): idle-muttering threshold.
const KASSIM_MUTTER_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `#define KASSIM_TIME_REPEAT_ENTRY 120` (`area3.c:322`): wall-clock
/// seconds before Kassim repeats his greeting.
const KASSIM_TIME_REPEAT_ENTRY: i32 = 120;
/// C `#define KASSIM_INSCRIPTION_MIN_LETTER 3` (`area3.c:323`).
const KASSIM_INSCRIPTION_MIN_LETTERS: usize = 3;
/// C `#define KASSIM_TIME_WAIT_ITEM 120` (`area3.c:324`): wall-clock
/// seconds before Kassim gives up waiting for the item.
const KASSIM_TIME_WAIT_ITEM: i32 = 120;
/// C `#define KASSIM_GOLD_NEEDED_TO_ENGRAVE 500 * 100` (`area3.c:325`).
const KASSIM_GOLD_NEEDED_TO_ENGRAVE: u32 = 500 * 100;

/// C `#define KASSIM_STATE_ENTRY 0` (`src/common/npc_states.h:108`).
const KASSIM_STATE_ENTRY: i32 = 0;
/// C `#define KASSIM_STATE_ENGRAVE_TEXT 2` (`npc_states.h:110`).
const KASSIM_STATE_ENGRAVE_TEXT: i32 = 2;
/// C `#define KASSIM_STATE_ITEM_WAIT 3` (`npc_states.h:111`).
const KASSIM_STATE_ITEM_WAIT: i32 = 3;
/// C `#define KASSIM_STATE_ENGRAVE 6` (`npc_states.h:114`).
const KASSIM_STATE_ENGRAVE: i32 = 6;

/// Kassim's own idle mutterings (`area3.c:637-650`) - genuinely new
/// relative to every prior area-3 driver.
const KASSIM_MUTTERINGS: &[&str] = &[
    "Such crude jewelry they bring me... but my craft elevates even the humblest piece.",
    "Steady hands, steady heart. That is the engraver's way.",
    "I once engraved a ring so fine, the wearer wept. True story.",
    "Three letters minimum. Is that truly so hard to remember?",
    "The grain of the metal speaks to me. Each piece has a voice.",
    "Five hundred gold for my services. A bargain, really. My talent is priceless.",
    "Engrave, polish, admire. The eternal cycle.",
    "I wonder what became of that ring I engraved for the old emperor...",
    "These tools have been in my family for generations. Well, I bought them last week. But still.",
    "Another day, another inscription. 'I love you.' So original.",
    "My finest work was invisible to the naked eye. Nobody noticed. That's the tragedy of genius.",
    "Do NOT touch the tools. I can see you looking at them.",
];

/// Per-player facts [`World::process_kassim_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KassimPlayerFacts {
    /// `PlayerRuntime::area3_kassim_state()`.
    pub kassim_state: i32,
    /// `PlayerRuntime::area3_kassim_seen_timer()` (C wall-clock `realtime`
    /// seconds at last processed `NT_CHAR`).
    pub kassim_seen_timer: i32,
    /// `PlayerRuntime::area3_kassim_item_wait_starttime()`.
    pub kassim_item_wait_starttime: i32,
}

/// A side effect [`World::process_kassim_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KassimOutcomeEvent {
    /// Write the new `area3_ppd.kassim_state` back.
    UpdateKassimState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C's unconditional `ppd->kassim_seen_timer = realtime;` after every
    /// processed `NT_CHAR` message (`area3.c:456`).
    UpdateSeenTimer { player_id: CharacterId, value: i32 },
    /// C `ppd->kassim_item_wait_starttime = realtime;` (`area3.c:414`).
    UpdateItemWaitStart { player_id: CharacterId, value: i32 },
}

impl World {
    /// C `kassim_driver`'s per-tick body (`area3.c:327-656`). `now` is
    /// C's wall-clock `realtime` (seconds).
    pub fn process_kassim_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, KassimPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<KassimOutcomeEvent> {
        let kassim_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_KASSIM
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for kassim_id in kassim_ids {
            self.process_kassim_messages(kassim_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    fn process_kassim_messages(
        &mut self,
        kassim_id: CharacterId,
        player_facts: &HashMap<CharacterId, KassimPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<KassimOutcomeEvent>,
    ) {
        let Some(kassim_name) = self
            .characters
            .get(&kassim_id)
            .map(|kassim| kassim.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Kassim(mut data)) = self
            .characters
            .get(&kassim_id)
            .and_then(|kassim| kassim.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&kassim_id)
            .map(|kassim| std::mem::take(&mut kassim.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.kassim_handle_char_message(
                    kassim_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.kassim_handle_text_message(
                    kassim_id,
                    &kassim_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.kassim_handle_give_message(
                    kassim_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                _ => {}
            }
        }

        if let Some(kassim) = self.characters.get_mut(&kassim_id) {
            kassim.driver_state = Some(CharacterDriverState::Kassim(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`area3.c:625-627`).
        if let (Some(kassim), Some((tx, ty))) =
            (self.characters.get(&kassim_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(kassim.x), i32::from(kassim.y), tx, ty) {
                if let Some(kassim_mut) = self.characters.get_mut(&kassim_id) {
                    let _ = turn(kassim_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`area3.c:629-633`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::thomas`/`world::lydia` already use.
        let last_talk = if let Some(kassim) = self.characters.get(&kassim_id) {
            match kassim.driver_state.as_ref() {
                Some(CharacterDriverState::Kassim(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + KASSIM_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(kassim) = self.characters.get(&kassim_id) else {
                return;
            };
            let (post_x, post_y) = (kassim.rest_x, kassim.rest_y);
            let moved = self.secure_move_driver(
                kassim_id,
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

        // C `if (dat->last_talk + TICKS*60 < ticker && !RANDOM(25)) { ...
        // murmur(...); dat->last_talk = ticker; }` (`area3.c:636-653`).
        if last_talk + KASSIM_MUTTER_TICKS < self.tick.0 {
            let mut seed = self.legacy_random_seed;
            let fires = legacy_random_below_from_seed(&mut seed, 25) == 0;
            let idx =
                legacy_random_below_from_seed(&mut seed, KASSIM_MUTTERINGS.len() as u32) as usize;
            self.legacy_random_seed = seed;
            if fires {
                self.npc_murmur(kassim_id, KASSIM_MUTTERINGS[idx]);
                if let Some(kassim) = self.characters.get_mut(&kassim_id) {
                    if let Some(CharacterDriverState::Kassim(data)) = kassim.driver_state.as_mut() {
                        data.last_talk = self.tick.0;
                    }
                }
            }
        }
    }

    /// C `kassim_driver`'s `NT_CHAR` branch (`area3.c:345-463`).
    #[allow(clippy::too_many_arguments)]
    fn kassim_handle_char_message(
        &mut self,
        kassim_id: CharacterId,
        data: &mut KassimDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, KassimPlayerFacts>,
        now: i32,
        events: &mut Vec<KassimOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(kassim) = self.characters.get(&kassim_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`area3.c:349-352`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`area3.c:355-358`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*10) continue;`
        // (`area3.c:361-364`).
        if tick < data.last_talk + KASSIM_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*20 && dat->
        // current_victim != co) continue;` (`area3.c:367-370`).
        if tick < data.last_talk + KASSIM_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`area3.c:373-376`).
        if kassim_id == player_id || !char_see_char(&kassim, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`area3.c:379-382`).
        if char_dist(&kassim, &player) > KASSIM_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        match facts.kassim_state {
            // C `case KASSIM_STATE_ENTRY:` (`area3.c:389-395`).
            KASSIM_STATE_ENTRY => {
                if now.saturating_sub(facts.kassim_seen_timer) > KASSIM_TIME_REPEAT_ENTRY {
                    self.npc_quiet_say_bytes(
                        kassim_id,
                        &format!(
                            "Hello and welcome to my humble abode, dost thou want me to engrave a piece of jewellery for thee? Let me {}explain{} it.",
                            crate::text::COL_STR_LIGHT_BLUE,
                            crate::text::COL_STR_RESET,
                        ),
                    );
                    didsay = true;
                }
            }
            // C `case KASSIM_STATE_ENGRAVE_TEXT:` (`area3.c:402-424`).
            KASSIM_STATE_ENGRAVE_TEXT => {
                let engrave_text = match self
                    .characters
                    .get(&player_id)
                    .and_then(|player| player.driver_state.clone())
                {
                    Some(CharacterDriverState::Engrave(engrave_data)) => Some(engrave_data.text),
                    _ => None,
                };
                match engrave_text {
                    Some(text) if text.len() >= KASSIM_INSCRIPTION_MIN_LETTERS => {
                        self.npc_quiet_say(
                            kassim_id,
                            "Hand me the item thou wishes to have engraved.",
                        );
                        didsay = true;
                        events.push(KassimOutcomeEvent::UpdateItemWaitStart {
                            player_id,
                            value: now,
                        });
                        events.push(KassimOutcomeEvent::UpdateKassimState {
                            player_id,
                            new_state: KASSIM_STATE_ITEM_WAIT,
                        });
                    }
                    Some(_) => {
                        self.npc_quiet_say(
                            kassim_id,
                            "The inscription shall at least count three letters, thou may try again.",
                        );
                        didsay = true;
                        events.push(KassimOutcomeEvent::UpdateKassimState {
                            player_id,
                            new_state: KASSIM_STATE_ENTRY,
                        });
                    }
                    None => {
                        self.npc_quiet_say(kassim_id, "Something went wrong, thou may try again.");
                        didsay = true;
                        events.push(KassimOutcomeEvent::UpdateKassimState {
                            player_id,
                            new_state: KASSIM_STATE_ENTRY,
                        });
                    }
                }
            }
            // C `case KASSIM_STATE_ITEM_WAIT:` (`area3.c:426-434`).
            KASSIM_STATE_ITEM_WAIT => {
                if now.saturating_sub(facts.kassim_item_wait_starttime) > KASSIM_TIME_WAIT_ITEM {
                    self.npc_quiet_say(kassim_id, "Cancel engravement, no item got.");
                    didsay = true;
                    events.push(KassimOutcomeEvent::UpdateKassimState {
                        player_id,
                        new_state: KASSIM_STATE_ENTRY,
                    });
                }
            }
            // C `case KASSIM_STATE_ENGRAVE:` (`area3.c:446-453`).
            KASSIM_STATE_ENGRAVE => {
                let engrave_text = match self
                    .characters
                    .get(&player_id)
                    .and_then(|player| player.driver_state.clone())
                {
                    Some(CharacterDriverState::Engrave(engrave_data)) => engrave_data.text,
                    _ => String::new(),
                };
                self.npc_quiet_say(
                    kassim_id,
                    &format!(
                        "Thine item now bears the inscription '{engrave_text}', may it bring thee much joy."
                    ),
                );
                didsay = true;
                events.push(KassimOutcomeEvent::UpdateKassimState {
                    player_id,
                    new_state: KASSIM_STATE_ENTRY,
                });
            }
            // `KASSIM_STATE_EXPLAIN`/`ITEM_GOT`/`TAKE_MONEY`: logically-
            // unreachable `elog`-only branches - see the module doc
            // comment.
            _ => {}
        }

        // C `ppd->kassim_seen_timer = realtime;` (`area3.c:456`) -
        // unconditional, unlike `dat->last_talk` below.
        events.push(KassimOutcomeEvent::UpdateSeenTimer {
            player_id,
            value: now,
        });

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`area3.c:458-462`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `kassim_driver`'s `NT_TEXT` branch (`area3.c:467-526`).
    #[allow(clippy::too_many_arguments)]
    fn kassim_handle_text_message(
        &mut self,
        kassim_id: CharacterId,
        kassim_name: &str,
        data: &mut KassimDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, KassimPlayerFacts>,
        events: &mut Vec<KassimOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*20 && dat->current_victim)
        // dat->current_victim = 0;` (`area3.c:470-472`).
        let tick = self.tick.0;
        if tick > data.last_talk + KASSIM_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`area3.c:474-477`).
        if let Some(current_victim) = data.current_victim {
            if current_victim != speaker_id {
                return;
            }
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };

        let mut didsay = false;

        // C `if ((ptr = strcasestr((char *)msg->dat2, "engrave:")) &&
        // (engrave_dta = set_data(co, DRD_ENGRAVE_DATA, ...))) { ... }`
        // (`area3.c:479-498`). Note: this branch never sets `didsay`, so
        // Kassim does not turn to face the speaker or remember them as
        // `current_victim` just from an "engrave:" command.
        if let Some(pos) = text.to_ascii_lowercase().find("engrave:") {
            let inscription: String = text[pos + "engrave:".len()..]
                .trim_start()
                .chars()
                .take_while(|&c| c != '"')
                .take(79)
                .collect();
            if let Some(speaker) = self.characters.get_mut(&speaker_id) {
                speaker.driver_state = Some(CharacterDriverState::Engrave(EngraveDriverData {
                    text: inscription,
                }));
            }
            let kassim_state = player_facts
                .get(&speaker_id)
                .map(|facts| facts.kassim_state)
                .unwrap_or(-1);
            if kassim_state == KASSIM_STATE_ENTRY {
                events.push(KassimOutcomeEvent::UpdateKassimState {
                    player_id: speaker_id,
                    new_state: KASSIM_STATE_ENGRAVE_TEXT,
                });
            }
        } else {
            // C `analyse_text_driver`'s own guard clauses
            // (`area3.c:223-238`): ignore our own talk, non-players,
            // distance > 12, not-visible.
            let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
                return;
            };
            if kassim_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
                return;
            }
            let Some(kassim) = self.characters.get(&kassim_id).cloned() else {
                return;
            };
            if char_dist(&kassim, &speaker) > KASSIM_QA_DISTANCE
                || !char_see_char(&kassim, &speaker, &self.map, self.date.daylight)
            {
                return;
            }

            match analyse_text_qa(text, kassim_name, &speaker.name, AREA3_QA) {
                TextAnalysisOutcome::Said(reply) => {
                    self.npc_quiet_say(kassim_id, &reply);
                    didsay = true;
                }
                // C `case 2: break;` (repeat) - no kassim-specific action.
                TextAnalysisOutcome::Matched(2) => {
                    didsay = true;
                }
                // C `case 9:` (explain) (`area3.c:504-518`).
                TextAnalysisOutcome::Matched(9) => {
                    let kassim_state = player_facts
                        .get(&speaker_id)
                        .map(|facts| facts.kassim_state)
                        .unwrap_or(-1);
                    if kassim_state == KASSIM_STATE_ENTRY {
                        self.npc_quiet_say(
                            kassim_id,
                            &format!(
                                "To make use of my services, simply say 'engrave: <engraving text> and hand me the item thou wishes to have engraved. I will charge thee {}g for the job.",
                                KASSIM_GOLD_NEEDED_TO_ENGRAVE / 100
                            ),
                        );
                        events.push(KassimOutcomeEvent::UpdateKassimState {
                            player_id: speaker_id,
                            new_state: KASSIM_STATE_ENTRY,
                        });
                    }
                    // C's `else { elog(...); }` branch: no speech, no
                    // state change, but `didsay` still ends up truthy
                    // (code 9) - matches the general `Matched(_)`
                    // fallthrough below.
                    didsay = true;
                }
                TextAnalysisOutcome::Matched(_) => {
                    didsay = true;
                }
                TextAnalysisOutcome::NoMatch => {}
            }
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`area3.c:522-525`) - note this does *not* touch
        // `dat->last_talk`.
        if didsay {
            if let Some(speaker) = self.characters.get(&speaker_id) {
                *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            }
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `kassim_driver`'s `NT_GIVE` branch (`area3.c:529-617`).
    fn kassim_handle_give_message(
        &mut self,
        kassim_id: CharacterId,
        data: &mut KassimDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, KassimPlayerFacts>,
        events: &mut Vec<KassimOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&kassim_id)
            .and_then(|kassim| kassim.cursor_item.take())
        else {
            return;
        };

        let kassim_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.kassim_state)
            .unwrap_or(-1);
        let mut didsay = false;

        if kassim_state == KASSIM_STATE_ITEM_WAIT {
            let item_flags = self
                .items
                .get(&item_id)
                .map(|item| item.flags)
                .unwrap_or(ItemFlags::empty());
            if item_flags.intersects(ItemFlags::WEAR) {
                if item_flags.contains(ItemFlags::ENGRAVED) {
                    // C `if (it[in].flags & IF_ENGRAVED) { ... }`
                    // (`area3.c:539-549`).
                    self.npc_quiet_say(
                        kassim_id,
                        "I am sorry, this piece already has an engraving, I do not have the heart to overwrite such splendid artwork!. Thou may repeat the procedure with another item.",
                    );
                    didsay = true;
                    events.push(KassimOutcomeEvent::UpdateKassimState {
                        player_id: giver_id,
                        new_state: KASSIM_STATE_ENTRY,
                    });
                    if !self.give_char_item(giver_id, item_id) {
                        self.destroy_item(item_id);
                    }
                } else {
                    let gold = self
                        .characters
                        .get(&giver_id)
                        .map(|character| character.gold)
                        .unwrap_or(0);
                    if gold < KASSIM_GOLD_NEEDED_TO_ENGRAVE {
                        // C `if (ch[co].gold < KASSIM_GOLD_NEEDED_TO_ENGRAVE)
                        // { ... }` (`area3.c:556-563`) - `didsay` is *not*
                        // set here (see the module doc comment).
                        self.npc_quiet_say(
                            kassim_id,
                            "Thou cannot pay for mine services, please come again when you can afford it.",
                        );
                        events.push(KassimOutcomeEvent::UpdateKassimState {
                            player_id: giver_id,
                            new_state: KASSIM_STATE_ENTRY,
                        });
                        if !self.give_char_item(giver_id, item_id) {
                            self.destroy_item(item_id);
                        }
                    } else {
                        // C `ch[co].gold -= KASSIM_GOLD_NEEDED_TO_ENGRAVE;`
                        // (`area3.c:566`).
                        if let Some(giver) = self.characters.get_mut(&giver_id) {
                            giver.gold -= KASSIM_GOLD_NEEDED_TO_ENGRAVE;
                        }
                        let engrave_text = match self
                            .characters
                            .get(&giver_id)
                            .and_then(|player| player.driver_state.clone())
                        {
                            Some(CharacterDriverState::Engrave(engrave_data)) => {
                                Some(engrave_data.text)
                            }
                            _ => None,
                        };
                        if let Some(text) = engrave_text {
                            // C `strcpy(it[in].description, engrave_dta->
                            // text); it[in].flags |= IF_ENGRAVED;`
                            // (`area3.c:570-572`) - `didsay` is *not* set
                            // here either (see the module doc comment).
                            if let Some(item) = self.items.get_mut(&item_id) {
                                item.description = text;
                                item.flags.insert(ItemFlags::ENGRAVED);
                            }
                            if !self.give_char_item(giver_id, item_id) {
                                self.destroy_item(item_id);
                            }
                            events.push(KassimOutcomeEvent::UpdateKassimState {
                                player_id: giver_id,
                                new_state: KASSIM_STATE_ENGRAVE,
                            });
                        } else {
                            // C's unreachable `else` (`set_data` never
                            // fails in practice) - `area3.c:579-589`.
                            self.npc_quiet_say(
                                kassim_id,
                                "Something went wrong, thou may try again.",
                            );
                            didsay = true;
                            events.push(KassimOutcomeEvent::UpdateKassimState {
                                player_id: giver_id,
                                new_state: KASSIM_STATE_ENTRY,
                            });
                            if !self.give_char_item(giver_id, item_id) {
                                self.destroy_item(item_id);
                            }
                        }
                    }
                }
            } else {
                // C `else { quiet_say(cn, "Only wearable items ..."); }`
                // (`area3.c:592-602`).
                self.npc_quiet_say(
                    kassim_id,
                    "Only wearable items can be engraved. Thou may repeat the procedure with another item.",
                );
                didsay = true;
                events.push(KassimOutcomeEvent::UpdateKassimState {
                    player_id: giver_id,
                    new_state: KASSIM_STATE_ENTRY,
                });
                if !self.give_char_item(giver_id, item_id) {
                    self.destroy_item(item_id);
                }
            }
        } else {
            // C `else { quiet_say(cn, "Thou hast better use ..."); }`
            // (`area3.c:603-610`) - `didsay` is *not* set here either.
            self.npc_quiet_say(
                kassim_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`area3.c:613-616`).
        if didsay {
            if let Some(giver) = self.characters.get(&giver_id) {
                *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            }
            data.current_victim = Some(giver_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_KASSIM;

/// C `struct kassim_driver_data` (`src/area/3/area3.c:313-316`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KassimDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// C `struct engrave_data` (`src/area/3/area3.c:318-320`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EngraveDriverData {
    pub text: String,
}
