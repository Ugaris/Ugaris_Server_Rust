//! Guard Brannington NPC (`CDR_GUARDBRAN`), the town guard who greets new
//! arrivals and, once Count Brannington's family-heirloom chain is
//! complete, sends the player to investigate Arkhata for "Finding Arkhata"
//! (quest 64).
//!
//! Ports `src/area/29/brannington.c::guard_brannington_driver` (`:1834-
//! 2022`) plus the shared `analyse_text_driver`/`qa[]` table (`:86-206`,
//! ported as [`super::AREA29_QA`] in `world::npc::area29`, the same table
//! every other `brannington.c` NPC driver shares). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area29::
//! spiritbran`/`countbran`: the caller supplies a per-player fact snapshot
//! ([`GuardBranPlayerFacts`]) up front and applies the returned
//! [`GuardBranOutcomeEvent`]s afterwards, since `staffer_ppd.
//! guardbran_state`/`countbran_state`/`countbran_bits` and the `arkhata_ppd.
//! rammy_state` cross-area read (`world::npc::area29`'s own precedent for
//! reading a field owned by an unported sibling driver - see
//! `PlayerRuntime::staffer_broklin_state`/`staffer_carlos2_state`) all
//! live on `crate::player::PlayerRuntime`, not `World`.
//!
//! `guard_brannington_driver`'s nine-state (`0`-`8`) dialogue chain:
//! greeting (dialogue only if the Count's own chain hasn't started yet,
//! but the state always advances and counts as "talked") -> (`case 1`:
//! silent gate on player level `>= 45` and all three
//! `countbran_bits` jewel bits, real C fallthrough straight into `case 2`
//! once satisfied, in the same tick) -> "Count Brannington has told me
//! thou helped him" -> "my cousin is a scout... met someone up in the
//! mountains" -> "this must be investigated... I should send you" ->
//! "your mission... find out who is up there" (opens quest 64) -> waiting
//! (`case 6`: silent gate on the cross-area `arkhata_ppd.rammy_state > 0`
//! read, real C fallthrough straight into `case 7` once satisfied,
//! completing quest 64 and awarding `ACHIEVEMENT_GREAT_EXPLORER` in the
//! same tick) -> "Excellent! The Count will be most pleased... Thank you"
//! -> done (state `8`).
//!
//! Deviations/gaps (documented, not silent):
//! - Like `world::npc::area29::spiritbran`'s own `NT_TEXT` branch, this
//!   driver's own C body has no `dat->current_victim` staleness-reset
//!   preamble and no victim-mismatch early-out at all - reproduced
//!   verbatim: replies to *any* nearby player's matched small talk, not
//!   just its tracked victim.
//! - Unlike every other `brannington.c` sibling ported so far, this
//!   driver's `NT_TEXT` branch has no `case 3` ("reset me") at all - only
//!   the "repeat"/"restart" `case 2` exists (`:1969-1984`) - verified by
//!   direct re-reading of the C `switch`, not an oversight.
//! - No self-defense/regen/spell-self cascade exists in C's `guard_
//!   brannington_driver` body at all (matching every other "pure talker"
//!   Brannington NPC's identical observation) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:2021`) is not
//!   ported, matching the established `world::thomas`/`world::npc::area29::
//!   spiritbran` precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:1884`).
const GUARDBRAN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const GUARDBRAN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:1867`).
const GUARDBRAN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:1872`).
const GUARDBRAN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:2015`): idle "return to post" threshold.
const GUARDBRAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ch[co].level >= 45` (`brannington.c:1904`): the mission-briefing
/// level gate.
const GUARDBRAN_MISSION_LEVEL_REQ: u32 = 45;
/// C `ppd->countbran_bits & (1 | 2 | 4)` (`brannington.c:1904`): all three
/// jewels returned to the Count's family.
const GUARDBRAN_BITS_ALL_JEWELS: i32 = 1 | 2 | 4;
/// C questlog index 64, "Finding Arkhata".
const QLOG_GUARDBRAN: usize = 64;

/// Per-player facts [`World::process_guardbran_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardBranPlayerFacts {
    /// `PlayerRuntime::staffer_guardbran_state()`.
    pub guardbran_state: i32,
    /// `PlayerRuntime::staffer_countbran_state()`.
    pub countbran_state: i32,
    /// `PlayerRuntime::staffer_countbran_bits()`.
    pub countbran_bits: i32,
    /// `PlayerRuntime::arkhata_rammy_state()` - a cross-area read of
    /// area 37's still-unported `rammy_driver` state (see the module doc
    /// comment).
    pub rammy_state: i32,
}

/// A side effect [`World::process_guardbran_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardBranOutcomeEvent {
    /// Write the new `staffer_ppd.guardbran_state` back.
    UpdateGuardBranState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 64)` (`brannington.c:1934`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 64)` plus `achievement_award(co,
    /// ACHIEVEMENT_GREAT_EXPLORER, 1)` (`brannington.c:1940-1942`). Quest
    /// 64's own nominal exp is `60000` (not "exp awarded in driver"), so
    /// the caller applies the full `complete_legacy` exp path, same
    /// precedent as `world::npc::area28::aristocrat`'s `QuestDone`.
    QuestDone { player_id: CharacterId },
}

impl World {
    /// C `guard_brannington_driver`'s per-tick body (`brannington.c:1834-
    /// 2022`).
    pub fn process_guardbran_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, GuardBranPlayerFacts>,
        area_id: u16,
    ) -> Vec<GuardBranOutcomeEvent> {
        let guardbran_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_GUARDBRAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for guardbran_id in guardbran_ids {
            self.process_guardbran_messages(guardbran_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_guardbran_messages(
        &mut self,
        guardbran_id: CharacterId,
        player_facts: &HashMap<CharacterId, GuardBranPlayerFacts>,
        area_id: u16,
        events: &mut Vec<GuardBranOutcomeEvent>,
    ) {
        let Some(guardbran_name) = self
            .characters
            .get(&guardbran_id)
            .map(|guardbran| guardbran.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::GuardBran(mut data)) = self
            .characters
            .get(&guardbran_id)
            .and_then(|guardbran| guardbran.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&guardbran_id)
            .map(|guardbran| std::mem::take(&mut guardbran.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.guardbran_handle_char_message(
                    guardbran_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.guardbran_handle_text_message(
                    guardbran_id,
                    &guardbran_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.guardbran_handle_give_message(guardbran_id, message);
                }
                _ => {}
            }
        }

        if let Some(guardbran) = self.characters.get_mut(&guardbran_id) {
            guardbran.driver_state = Some(CharacterDriverState::GuardBran(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:2011-2013`).
        if let (Some(guardbran), Some((tx, ty))) =
            (self.characters.get(&guardbran_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(guardbran.x), i32::from(guardbran.y), tx, ty)
            {
                if let Some(guardbran_mut) = self.characters.get_mut(&guardbran_id) {
                    let _ = turn(guardbran_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`brannington.c:2015-2018`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::spiritbran` already uses.
        let last_talk = if let Some(guardbran) = self.characters.get(&guardbran_id) {
            match guardbran.driver_state.as_ref() {
                Some(CharacterDriverState::GuardBran(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + GUARDBRAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(guardbran) = self.characters.get(&guardbran_id) else {
                return;
            };
            let (post_x, post_y) = (guardbran.rest_x, guardbran.rest_y);
            self.secure_move_driver(
                guardbran_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `guard_brannington_driver`'s `NT_CHAR` branch (`brannington.c:
    /// 1851-1961`), including its `case 1`->`2` and `case 6`->`7`
    /// fallthrough cascades - ported as an explicit `loop` so a single
    /// driver call can walk straight through a satisfied gate exactly like
    /// C's own `switch` fallthrough, without waiting for another tick (same
    /// mechanism as `world::npc::area29::countessabran`/`daughterbran`).
    #[allow(clippy::too_many_arguments)]
    fn guardbran_handle_char_message(
        &mut self,
        guardbran_id: CharacterId,
        data: &mut GuardBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GuardBranPlayerFacts>,
        events: &mut Vec<GuardBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(guardbran) = self.characters.get(&guardbran_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:1854-1858`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:1860-1864`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:1866-1870`).
        if tick < data.last_talk + GUARDBRAN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:1872-1875`).
        if tick < data.last_talk + GUARDBRAN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:1877-1881`).
        if guardbran_id == player_id
            || !char_see_char(&guardbran, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:1883-
        // 1887`).
        if char_dist(&guardbran, &player) > GUARDBRAN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.guardbran_state;
        let mut mark_quest_done = false;
        loop {
            match new_state {
                // C `case 0:` (`brannington.c:1894-1902`).
                0 => {
                    if facts.countbran_state == 0 {
                        self.npc_quiet_say(
                            guardbran_id,
                            "Greetings stranger, welcome to the town of Brannington. If you will, the Count would like to ask for your services. We have already informed him of your arrival, and you can find him in the mansion at the end of this street.",
                        );
                    }
                    new_state = 1;
                    didsay = true;
                    break;
                }
                // C `case 1:` (`brannington.c:1903-1908`): silent gate,
                // real fallthrough into `case 2` once satisfied.
                1 => {
                    if player.level >= GUARDBRAN_MISSION_LEVEL_REQ
                        && facts.countbran_bits & GUARDBRAN_BITS_ALL_JEWELS
                            == GUARDBRAN_BITS_ALL_JEWELS
                    {
                        new_state = 2;
                        continue;
                    }
                    break;
                }
                // C `case 2:` (`brannington.c:1909-1914`).
                2 => {
                    self.npc_quiet_say(
                        guardbran_id,
                        "Greetings! Count Brannington has told me thou helped him retrieve his family heirlooms. This time I must ask for thy help.",
                    );
                    new_state = 3;
                    didsay = true;
                    break;
                }
                // C `case 3:` (`brannington.c:1915-1920`).
                3 => {
                    self.npc_quiet_say(
                        guardbran_id,
                        "My cousin is a scout for the Count, and last night he came back in a most desperate state. He rambled about having met someone up in the mountains above the mines.",
                    );
                    new_state = 4;
                    didsay = true;
                    break;
                }
                // C `case 4:` (`brannington.c:1921-1927`).
                4 => {
                    self.npc_quiet_say(
                        guardbran_id,
                        "However my cousin is not an easily fooled man, and this must be investigated. I have discussed this with the Count and he agrees that I should send you as we are most convinced there actually is someone up there.",
                    );
                    new_state = 5;
                    didsay = true;
                    break;
                }
                // C `case 5:` (`brannington.c:1928-1935`).
                5 => {
                    self.npc_quiet_say(
                        guardbran_id,
                        &format!(
                            "Your mission, {} is to find out who is up there, and if they are friendly or hostile.",
                            army_rank_name(army_rank_for_points(player.military_points))
                        ),
                    );
                    new_state = 6;
                    didsay = true;
                    events.push(GuardBranOutcomeEvent::QuestOpen { player_id });
                    break;
                }
                // C `case 6:` (`brannington.c:1936-1945`): silent gate,
                // real fallthrough into `case 7` once satisfied.
                6 => {
                    if facts.rammy_state > 0 {
                        new_state = 7;
                        mark_quest_done = true;
                        continue;
                    }
                    break;
                }
                // C `case 7:` (`brannington.c:1946-1951`).
                7 => {
                    self.npc_quiet_say(
                        guardbran_id,
                        &format!(
                            "Excellent! The Count will be most pleased to hear this. Thank you, {}!",
                            player.name
                        ),
                    );
                    new_state = 8;
                    didsay = true;
                    break;
                }
                // C `case 8: break;` (`brannington.c:1952-1953`): all done.
                _ => break,
            }
        }

        if mark_quest_done {
            events.push(GuardBranOutcomeEvent::QuestDone { player_id });
        }

        if new_state != facts.guardbran_state {
            events.push(GuardBranOutcomeEvent::UpdateGuardBranState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:1955-1959`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `guard_brannington_driver`'s `NT_TEXT` branch (`brannington.c:
    /// 1964-1990`), wired through the generic `analyse_text_qa` matcher.
    /// This branch has no victim-staleness-reset preamble and no
    /// victim-mismatch early-out, and no `case 3` "reset me" at all (see
    /// the module doc comment).
    #[allow(clippy::too_many_arguments)]
    fn guardbran_handle_text_message(
        &mut self,
        guardbran_id: CharacterId,
        guardbran_name: &str,
        data: &mut GuardBranDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GuardBranPlayerFacts>,
        events: &mut Vec<GuardBranOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:1967`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if guardbran_id == speaker_id {
            return;
        }
        let Some(guardbran) = self.characters.get(&guardbran_id).cloned() else {
            return;
        };
        if char_dist(&guardbran, &speaker) > GUARDBRAN_QA_DISTANCE
            || !char_see_char(&guardbran, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let guardbran_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.guardbran_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, guardbran_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(guardbran_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:1970-1983`): reset back to the
            // nearest chain start.
            TextAnalysisOutcome::Matched(2) => {
                let reset_state = if guardbran_state <= 1 {
                    Some(0)
                } else if (2..=6).contains(&guardbran_state) {
                    Some(2)
                } else if (7..=8).contains(&guardbran_state) {
                    Some(7)
                } else {
                    None
                };
                if let Some(reset_state) = reset_state {
                    data.last_talk = 0;
                    events.push(GuardBranOutcomeEvent::UpdateGuardBranState {
                        player_id: speaker_id,
                        new_state: reset_state,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the area-shared `4`/`5` gold/silver
            // trade codes, consumed only by `broklin_driver`) is unhandled
            // by guardbran's own C `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington.c:1985-1988`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `guard_brannington_driver`'s `NT_GIVE` branch (`brannington.c:
    /// 1993-2003`): the guard never wants anything, always hands the item
    /// back.
    fn guardbran_handle_give_message(
        &mut self,
        guardbran_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&guardbran_id)
            .and_then(|guardbran| guardbran.cursor_item.take())
        else {
            return;
        };

        // C's own fallback line, `quiet_say` (`:1997`) - matches the same
        // exact wording every other `brannington.c` NPC uses.
        self.npc_quiet_say(
            guardbran_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_GUARDBRAN, CDR_LOSTCON};

/// C `struct guard_brannington_data` (`src/area/29/brannington.c:1829-
/// 1832`, inline local declaration mirrored on `world::npc::area29::
/// spiritbran`'s `struct spirit_brannington_data` shape).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GuardBranDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_GUARDBRAN`] to `ugaris-server`'s `apply_guardbran_events`.
pub const fn qlog_guardbran() -> usize {
    QLOG_GUARDBRAN
}
