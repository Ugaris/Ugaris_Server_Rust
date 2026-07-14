//! `CDR_NOMAD`'s `NT_TEXT` handling, split out of `nomad.rs` to stay under
//! the ~800-line NPC-file guideline (same precedent as `world::npc::
//! area17::guard`/`guard_messages`) - see `nomad.rs`'s own module doc
//! comment for the driver's full behavior.
//!
//! Ports `nomad.c`'s own `analyse_text_driver` (`:113-211`, a near-
//! identical copy of the generic `analyse_text_driver` shared across the
//! codebase, but with `char_dist(cn, co) > 11` instead of the more common
//! `> 10`) plus the `NT_TEXT` branch of `nomad` (`:986-1076`) that
//! dispatches its result, and the raw `strstr(msg->dat2, "bet ")` command
//! trigger that runs independently of the qa-table match.
//!
//! A deliberate, documented gap: C's `tabunga(cn, co, ptr)` call at the
//! very end of the `NT_TEXT` branch (`nomad.c:1076`) is a `CF_GOD`-only
//! debug stat dump (`src/system/tool.c:3837-3877`) unrelated to any
//! player-facing gameplay - not ported, same precedent as other `CF_GOD`-
//! only debug/admin tails documented elsewhere in this codebase (e.g.
//! `world::james`'s "raise me" tail).

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, CharacterDriverMessage, TextAnalysisOutcome};
use crate::world::*;

use super::nomad::{NomadDriverData, NomadOutcomeEvent, NomadPlayerFacts};
use super::{NOMAD_QA, TM_TRIBE1};

/// C `char_dist(cn, co) > 11` (`nomad.c:132`): `analyse_text_driver`'s own
/// visibility gate, one tile more permissive than the codebase's more
/// common `> 10`.
const NOMAD_TEXT_MAX_DISTANCE: i32 = 11;
/// C `char_dist(cn, co) < 12` (`nomad.c:1037`): the `"bet "` command
/// trigger's own visibility gate.
const NOMAD_BET_TRIGGER_DISTANCE: i32 = 12;

impl World {
    /// C `nomad`'s `NT_TEXT` branch (`nomad.c:986-1076`).
    pub(super) fn nomad_handle_text_message(
        &mut self,
        nomad_id: CharacterId,
        data: &mut NomadDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, NomadPlayerFacts>,
        events: &mut Vec<NomadOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(nomad) = self.characters.get(&nomad_id).cloned() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        let nr = if data.nr >= 1 && data.nr <= 6 {
            Some(data.nr as usize)
        } else {
            None
        };

        // Only run the qa-table half if the speaker has a `nomad_ppd` to
        // read/write, matching C's `if ((ppd = set_data(co, DRD_NOMAD_PPD,
        // ...)))` guard (`nomad.c:989`) - and `analyse_text_driver`'s own
        // guard clauses (`nomad.c:118-138`): ignore our own talk,
        // non-player/player-like speakers, distance, visibility.
        if let (Some(facts), Some(nr)) = (player_facts.get(&speaker_id), nr) {
            if nomad_id != speaker_id
                && speaker
                    .flags
                    .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
                && char_dist(&nomad, &speaker) <= NOMAD_TEXT_MAX_DISTANCE
                && char_see_char(&nomad, &speaker, &self.map, self.date.daylight)
            {
                match analyse_text_qa(text, &nomad.name, &speaker.name, NOMAD_QA) {
                    TextAnalysisOutcome::Said(reply) => {
                        self.npc_say(nomad_id, &reply);
                    }
                    // C `case 2:` (repeat) (`nomad.c:991-1018`).
                    TextAnalysisOutcome::Matched(2) => {
                        if let Some(new_state) =
                            World::nomad_repeat_state(data.nr, facts.nomad_state[nr])
                        {
                            data.last_talk_tick = 0;
                            self.set_nomad_state_event(events, speaker_id, nr, new_state);
                        }
                    }
                    // C `case 3/4/5/6:` (dice/statue purchase)
                    // (`nomad.c:1019-1033`).
                    TextAnalysisOutcome::Matched(res @ (3..=5)) => {
                        if data.nr == 2 {
                            data.last_talk_tick = 0;
                            self.nomad_2_text(nomad_id, speaker_id, facts, res, events);
                        }
                    }
                    TextAnalysisOutcome::Matched(6) => {
                        if data.nr == 6 {
                            data.last_talk_tick = 0;
                            self.nomad_6_text(nomad_id, speaker_id, facts, events);
                        }
                    }
                    TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
                }
            }
        }

        // C `if ((ptr = strstr(msg->dat2, "bet ")) && char_dist(cn, co) <
        // 12 && char_see_char(cn, co))` (`nomad.c:1037`).
        if let Some(bet_pos) = text.find("bet ") {
            if char_dist(&nomad, &speaker) < NOMAD_BET_TRIGGER_DISTANCE
                && char_see_char(&nomad, &speaker, &self.map, self.date.daylight)
            {
                // C `atoi(ptr + 4)` (`nomad.c:1038`).
                let val: i32 = text[bet_pos + 4..]
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0);
                if val != 0 {
                    let facts = player_facts.get(&speaker_id).copied();
                    let may_play = match data.nr {
                        1 => facts.is_some_and(|f| f.nomad_state[1] >= 9),
                        2 | 3 | 6 => facts.is_some_and(|f| f.tribe_member & TM_TRIBE1 != 0),
                        _ => false,
                    };
                    match data.nr {
                        1 | 2 | 3 | 6 => {
                            if may_play {
                                if let Some(facts) = facts {
                                    self.nomad_bet(nomad_id, data, speaker_id, val, &facts, events);
                                }
                            } else {
                                self.npc_say(nomad_id, "Sorry, I do not play with strangers.");
                            }
                        }
                        _ => {
                            self.npc_say(nomad_id, "I do not play.");
                        }
                    }
                }
            }
        }
    }

    /// C `nomad_2_text` (`nomad.c:602-645`): the dice purchase.
    fn nomad_2_text(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        facts: &NomadPlayerFacts,
        res: i32,
        events: &mut Vec<NomadOutcomeEvent>,
    ) {
        if facts.tribe_member & TM_TRIBE1 == 0 {
            return;
        }
        let (cost, template): (i32, &'static str) = match res {
            3 => (200, "dice0"),
            4 => (500, "dice1"),
            5 => (1200, "dice2"),
            _ => return,
        };
        if self.count_salt(player_id) < cost {
            self.npc_say(nomad_id, "But thou dost not have enough salt to pay.");
            return;
        }
        events.push(NomadOutcomeEvent::BuyItemWithSalt {
            nomad_id,
            player_id,
            template,
            cost,
        });
    }

    /// C `nomad_6_text` (`nomad.c:647-679`): the golden-statue purchase.
    fn nomad_6_text(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        facts: &NomadPlayerFacts,
        events: &mut Vec<NomadOutcomeEvent>,
    ) {
        if facts.tribe_member & TM_TRIBE1 == 0 {
            return;
        }
        const COST: i32 = 10000;
        if self.count_salt(player_id) < COST {
            self.npc_say(nomad_id, "But thou dost not have enough salt to pay.");
            return;
        }
        events.push(NomadOutcomeEvent::BuyItemWithSalt {
            nomad_id,
            player_id,
            template: "kir_statue",
            cost: COST,
        });
    }
}
