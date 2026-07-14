//! Seymour NPC (`CDR_SEYMOUR`), the Seyan'Du Staff Sergeant who greets new
//! arrivals in Aston and hands out the army-enrollment quest chain
//! (`QLOG` 10-12).
//!
//! Ports `src/area/3/area3.c::seymour_driver` (`:660-958`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:106-204`, ported as
//! [`AREA3_QA`] in `world::npc::area3`, the same table `world::thomas`/
//! `world::sir_jones`/`world::astro2` share). Follows the same `World`/
//! `PlayerRuntime` split established by those siblings: the caller
//! supplies a per-player fact snapshot ([`SeymourPlayerFacts`]) up front
//! and applies the returned [`SeymourOutcomeEvent`]s afterwards, since
//! `area3_ppd.seymour_state` and the `QLOG` 10-12 quest-log entries live
//! on `crate::player::PlayerRuntime`, not `World`.
//!
//! `seymour_driver`'s eighteen-state (`0`-`17`) dialogue chain: greeting
//! -> seven lore lines -> "find a strange skull in Loisan's old Cameron
//! house" (quest 10) -> (external: player hands over
//! `IID_AREA2_ZOMBIESKULL1`, completes quest 10, sets state `10`, and
//! enrolls the player in the Imperial Army if not already ranked) ->
//! "find a silver skull in Loisan's Aston house" (quest 11) -> (external:
//! player hands over `IID_AREA2_ZOMBIESKULL2`, completes quest 11, sets
//! state `12`, awards 1 military point + 1 exp on first completion) ->
//! "find out what became of Loisan" (quest 12) -> (external: player hands
//! over `IID_AREA2_LOISANNOTE`, completes quest 12, sets state `16`,
//! awards 2 military points + 1 exp on first completion) -> "go see Kelly
//! for fighter duty".
//!
//! Deviations/gaps (documented, not silent):
//! - C's `case 12`/`case 13` bodies both just reassign `ppd->seymour_state
//!   = 14` with a `// fall through intended` comment and no `break`
//!   (`area3.c:804-807`), so a single `NT_CHAR` visit at `seymour_state ==
//!   12` or `== 13` always executes exactly `case 14`'s body. Reproduced
//!   verbatim by matching `12 | 13 | 14` as one arm, same precedent as
//!   `world::sir_jones`'s `case 10` fallthrough note.
//! - C's `set_army_rank(co, 1)` enrollment call (`area3.c:911`) writes a
//!   separate `DRD_RANK_PPD` field directly, independent of
//!   `Character.military_points`; Rust derives rank from
//!   `military_points` via [`army_rank_for_points`] instead (see that
//!   function's own doc comment). Since the enrollment only ever fires
//!   when the derived rank is currently `0`, and any `military_points` in
//!   `1..=7` derives to exactly rank `1` (matching what `set_army_rank(co,
//!   1)` would read back as), setting `military_points = 1` reproduces
//!   the observable rank/`get_army_rank_string` output without granting
//!   any exp or going through the promotion-broadcast machinery C's own
//!   `give_military_pts` would add (C doesn't call that function here
//!   either - `set_army_rank` is a raw field write).
//! - No self-defense/regen/spell-self cascade exists in C's `seymour_
//!   driver` body at all (matching `world::astro1`/`world::thomas`/
//!   `world::sir_jones`/`world::astro2`'s identical observation for area
//!   3's other "pure talker" NPCs) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`area3.c:955`) is
//!   not ported, matching the established sibling-driver precedent for
//!   stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_AREA2_LOISANNOTE, IID_AREA2_ZOMBIESKULL1, IID_AREA2_ZOMBIESKULL2};
use crate::world::*;

use super::AREA3_QA;

/// C `char_dist(cn, co) > 10` (`area3.c:714`).
const SEYMOUR_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`area3.c:232`, the shared
/// `analyse_text_driver` copy's own guard).
const SEYMOUR_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`area3.c:697`).
const SEYMOUR_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`area3.c:702`, `:842`).
const SEYMOUR_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`area3.c:951`): idle "return to post" threshold.
const SEYMOUR_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `questlog_open(co, 10)` (`area3.c:729`).
const QLOG_SEYMOUR_SKULL1: usize = 10;
/// C `questlog_open(co, 11)` (`area3.c:798`).
const QLOG_SEYMOUR_SKULL2: usize = 11;
/// C `questlog_open(co, 12)` (`area3.c:815`).
const QLOG_SEYMOUR_LOISAN: usize = 12;

/// Per-player facts [`World::process_seymour_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeymourPlayerFacts {
    /// `PlayerRuntime::area3_seymour_state()`.
    pub seymour_state: i32,
    /// `PlayerRuntime::quest_log.is_done(QLOG_SEYMOUR_SKULL2)` (C
    /// `questlog_isdone(co, 11)`).
    pub quest11_done: bool,
    /// `PlayerRuntime::quest_log.is_done(QLOG_SEYMOUR_LOISAN)` (C
    /// `questlog_isdone(co, 12)`).
    pub quest12_done: bool,
}

/// A side effect [`World::process_seymour_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeymourOutcomeEvent {
    /// Write the new `area3_ppd.seymour_state` back.
    UpdateSeymourState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, ...)`.
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `tmp = questlog_done(co, 12); ... if (tmp == 1) { give_military_
    /// pts(cn, co, 2, 1); }` (`area3.c:894-900`). `seymour_id` is needed
    /// for the conditional `give_military_pts_from_npc` promotion
    /// announcement (`World::give_military_pts_from_npc`'s `master_id`
    /// parameter).
    LoisanNoteQuestDone {
        player_id: CharacterId,
        seymour_id: CharacterId,
    },
    /// C `questlog_done(co, 10);` (`area3.c:907`) - return value unused,
    /// no conditional point reward for this one.
    ZombieSkull1QuestDone { player_id: CharacterId },
    /// C `tmp = questlog_done(co, 11); ... if (tmp == 1) { give_military_
    /// pts(cn, co, 1, 1); }` (`area3.c:921-926`). `seymour_id` is needed
    /// for the conditional `give_military_pts_from_npc` promotion
    /// announcement, same as [`SeymourOutcomeEvent::LoisanNoteQuestDone`].
    ZombieSkull2QuestDone {
        player_id: CharacterId,
        seymour_id: CharacterId,
    },
}

impl World {
    /// C `seymour_driver`'s per-tick body (`area3.c:665-958`).
    pub fn process_seymour_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, SeymourPlayerFacts>,
        area_id: u16,
    ) -> Vec<SeymourOutcomeEvent> {
        let seymour_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SEYMOUR
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for seymour_id in seymour_ids {
            self.process_seymour_messages(seymour_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_seymour_messages(
        &mut self,
        seymour_id: CharacterId,
        player_facts: &HashMap<CharacterId, SeymourPlayerFacts>,
        area_id: u16,
        events: &mut Vec<SeymourOutcomeEvent>,
    ) {
        let Some(seymour_name) = self
            .characters
            .get(&seymour_id)
            .map(|seymour| seymour.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Seymour(mut data)) = self
            .characters
            .get(&seymour_id)
            .and_then(|seymour| seymour.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&seymour_id)
            .map(|seymour| std::mem::take(&mut seymour.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.seymour_handle_char_message(
                    seymour_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.seymour_handle_text_message(
                    seymour_id,
                    &seymour_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.seymour_handle_give_message(seymour_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(seymour) = self.characters.get_mut(&seymour_id) {
            seymour.driver_state = Some(CharacterDriverState::Seymour(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`area3.c:947-949`).
        if let (Some(seymour), Some((tx, ty))) =
            (self.characters.get(&seymour_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(seymour.x), i32::from(seymour.y), tx, ty) {
                if let Some(seymour_mut) = self.characters.get_mut(&seymour_id) {
                    let _ = turn(seymour_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`area3.c:951-955`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::thomas`/`world::sir_jones`/`world::astro2`
        // already use.
        let last_talk = if let Some(seymour) = self.characters.get(&seymour_id) {
            match seymour.driver_state.as_ref() {
                Some(CharacterDriverState::Seymour(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + SEYMOUR_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(seymour) = self.characters.get(&seymour_id) else {
                return;
            };
            let (post_x, post_y) = (seymour.rest_x, seymour.rest_y);
            self.secure_move_driver(
                seymour_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `seymour_driver`'s `NT_CHAR` branch (`area3.c:680-836`).
    fn seymour_handle_char_message(
        &mut self,
        seymour_id: CharacterId,
        data: &mut SeymourDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SeymourPlayerFacts>,
        events: &mut Vec<SeymourOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(seymour) = self.characters.get(&seymour_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`area3.c:684-688`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`area3.c:690-694`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`area3.c:696-700`).
        if tick < data.last_talk + SEYMOUR_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`area3.c:702-705`).
        if tick < data.last_talk + SEYMOUR_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`area3.c:707-711`).
        if seymour_id == player_id
            || !char_see_char(&seymour, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`area3.c:713-717`).
        if char_dist(&seymour, &player) > SEYMOUR_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.seymour_state;
        match facts.seymour_state {
            // C `case 0:` (`area3.c:724-732`).
            0 => {
                self.npc_quiet_say(
                    seymour_id,
                    &format!(
                        "Welcome to Aston, {}! I am {}, Staff Sergeant of the Seyan'Du, the late emperor's personal guard.",
                        player.name, seymour.name
                    ),
                );
                events.push(SeymourOutcomeEvent::QuestOpen {
                    player_id,
                    quest: QLOG_SEYMOUR_SKULL1,
                });
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`area3.c:733-738`).
            1 => {
                self.npc_quiet_say(
                    seymour_id,
                    "There are but few of us left. Most died in the defense when the underworld opened and the monsters stormed the imperial palace.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`area3.c:739-744`).
            2 => {
                self.npc_quiet_say(
                    seymour_id,
                    "Do be wary when thou approachest the palace. It is still in ruins, and monsters appear there from time to time.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`area3.c:745-750`).
            3 => {
                self.npc_quiet_say(
                    seymour_id,
                    "Some greedy folks tried to loot it, but few have returned with their spoils. Ah, be that as it may. I talk too much.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`area3.c:751-757`).
            4 => {
                self.npc_quiet_say(
                    seymour_id,
                    "My captain said I was to hire strong looking adventurers for Imperial service. Never before have the Seyan'Du asked for help. But now we do. Oh, what has become of this world?",
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5:` (`area3.c:758-762`).
            5 => {
                self.npc_quiet_say(
                    seymour_id,
                    "Our emperor murdered, nine of ten killed. Oh my, oh my.",
                );
                new_state = 6;
                didsay = true;
            }
            // C `case 6:` (`area3.c:763-770`).
            6 => {
                self.npc_quiet_say(
                    seymour_id,
                    &format!(
                        "{}, the Seyan'Du are offering you rank and status, in exchange for some missions. The first of these missions is to find out more about a certain Loisan.",
                        player.name
                    ),
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7:` (`area3.c:771-777`).
            7 => {
                self.npc_quiet_say(
                    seymour_id,
                    "It seems he lived in Cameron for a while and moved here a few weeks later. After he left Cameron, skeletons have been haunting that place. Now, he has left Aston, and we're having trouble with zombies.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8:` (`area3.c:778-784`).
            8 => {
                self.npc_quiet_say(
                    seymour_id,
                    "As a first step, I want you to go back to Cameron and search Loisan's house there. I have heard rumors that he was working on strange human skulls, and I want you to acquire one of those and bring it to me.",
                );
                new_state = 9;
                didsay = true;
            }
            // C `case 9: break;` (`area3.c:785-786`) - waiting for the
            // player to hand over `IID_AREA2_ZOMBIESKULL1`.
            9 => {}
            // C `case 10:` (`area3.c:787-801`).
            10 => {
                if facts.quest11_done {
                    new_state = 12;
                } else {
                    let rank_name = army_rank_name(army_rank_for_points(player.military_points));
                    self.npc_quiet_say(
                        seymour_id,
                        &format!(
                            "Your next mission, {rank_name}, is to search Loisan's house here in Aston. It is on this street, on the western side. As far as we know, he's been using silver skulls here, and I want you to bring me one of those. You might also want to talk to the Governor of Aston for additional missions.",
                        ),
                    );
                    events.push(SeymourOutcomeEvent::QuestOpen {
                        player_id,
                        quest: QLOG_SEYMOUR_SKULL2,
                    });
                    new_state = 11;
                    didsay = true;
                }
            }
            // C `case 11: break;` (`area3.c:802-803`) - waiting for the
            // player to hand over `IID_AREA2_ZOMBIESKULL2`.
            11 => {}
            // C `case 12:`/`case 13:` fall through into `case 14`'s body
            // with no intervening `break` (`area3.c:804-818`) - see the
            // module doc comment's deviation note.
            12..=14 => {
                if facts.quest12_done {
                    new_state = 16;
                } else {
                    self.npc_quiet_say(
                        seymour_id,
                        "Alright, now that we have the skulls he was using, it would be nice to know what became of Loisan. If you can find him, his body, or proof of his whereabouts, bring it to me.",
                    );
                    events.push(SeymourOutcomeEvent::QuestOpen {
                        player_id,
                        quest: QLOG_SEYMOUR_LOISAN,
                    });
                    new_state = 15;
                    didsay = true;
                }
            }
            // C `case 15: break;` (`area3.c:819-820`) - waiting for the
            // player to hand over `IID_AREA2_LOISANNOTE`.
            15 => {}
            // C `case 16:` (`area3.c:821-826`).
            16 => {
                self.npc_quiet_say(
                    seymour_id,
                    "Kelly, my superior, mentioned that she needs some fighters. Please go to her and offer your service. And do not forget to report to the Governor from time to time.",
                );
                new_state = 17;
                didsay = true;
            }
            // C `case 17: break;` (`area3.c:827-828`).
            17 => {}
            _ => {}
        }

        if new_state != facts.seymour_state {
            events.push(SeymourOutcomeEvent::UpdateSeymourState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`area3.c:830-834`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `seymour_driver`'s `NT_TEXT` branch (`area3.c:839-880`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::thomas`'s text handler).
    #[allow(clippy::too_many_arguments)]
    fn seymour_handle_text_message(
        &mut self,
        seymour_id: CharacterId,
        seymour_name: &str,
        data: &mut SeymourDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SeymourPlayerFacts>,
        events: &mut Vec<SeymourOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`area3.c:842-844`).
        let tick = self.tick.0;
        if tick > data.last_talk + SEYMOUR_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`area3.c:846-849`).
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

        // C `analyse_text_driver`'s own guard clauses (`area3.c:223-238`):
        // ignore our own talk, non-players, distance > 12, not-visible.
        if seymour_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(seymour) = self.characters.get(&seymour_id).cloned() else {
            return;
        };
        if char_dist(&seymour, &speaker) > SEYMOUR_QA_DISTANCE
            || !char_see_char(&seymour, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let seymour_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.seymour_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, seymour_name, &speaker.name, AREA3_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(seymour_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat/restart) (`area3.c:852-874`): five
            // mutually exclusive state buckets covering the full `0..=17`
            // range, each resetting `dat->last_talk = 0` and rewinding
            // `seymour_state` to the bucket's start.
            TextAnalysisOutcome::Matched(2) => {
                data.last_talk = 0;
                let new_state = match seymour_state {
                    0..=9 => 0,
                    10..=11 => 10,
                    12..=13 => 12,
                    14..=15 => 14,
                    _ => 16,
                };
                events.push(SeymourOutcomeEvent::UpdateSeymourState {
                    player_id: speaker_id,
                    new_state,
                });
                didsay = true;
            }
            // Every other matched code is unhandled by seymour's own C
            // `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`area3.c:876-879`) - note this does *not* touch
        // `dat->last_talk` (except the explicit reset inside the `case 2`
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `seymour_driver`'s `NT_GIVE` branch (`area3.c:883-939`).
    fn seymour_handle_give_message(
        &mut self,
        seymour_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, SeymourPlayerFacts>,
        events: &mut Vec<SeymourOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&seymour_id)
            .and_then(|seymour| seymour.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let Some(giver_name) = self.characters.get(&giver_id).map(|c| c.name.clone()) else {
            self.destroy_item(item_id);
            return;
        };
        let seymour_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.seymour_state)
            .unwrap_or(-1);

        if template_id == IID_AREA2_LOISANNOTE && seymour_state == 15 {
            // C `if (it[in].ID == IID_AREA2_LOISANNOTE && ppd->
            // seymour_state == 15) { say("So he is dead. Ah, well. Thank
            // you, %s."); tmp = questlog_done(co, 12); destroy_item_byID
            // (co, IID_AREA2_LOISANNOTE); ppd->seymour_state = 16; if
            // (tmp == 1) { give_military_pts(cn, co, 2, 1); }
            // destroy_item(ch[cn].citem); ch[cn].citem = 0; }`
            // (`area3.c:890-904`).
            self.npc_quiet_say(
                seymour_id,
                &format!("So he is dead. Ah, well. Thank you, {giver_name}."),
            );
            events.push(SeymourOutcomeEvent::LoisanNoteQuestDone {
                player_id: giver_id,
                seymour_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA2_LOISANNOTE);
            events.push(SeymourOutcomeEvent::UpdateSeymourState {
                player_id: giver_id,
                new_state: 16,
            });
            self.destroy_item(item_id);
        } else if template_id == IID_AREA2_ZOMBIESKULL1 && seymour_state == 9 {
            // C `} else if (it[in].ID == IID_AREA2_ZOMBIESKULL1 && ppd->
            // seymour_state == 9) { say("A strange skull indeed, %s.");
            // questlog_done(co, 10); ppd->seymour_state = 10; if (!
            // get_army_rank_int(co)) { set_army_rank(co, 1); say("Welcome
            // to the Imperial Army, %s %s!"); } destroy_item(ch[cn].
            // citem); ch[cn].citem = 0; }` (`area3.c:905-917`).
            self.npc_quiet_say(
                seymour_id,
                &format!("A strange skull indeed, {giver_name}."),
            );
            events.push(SeymourOutcomeEvent::ZombieSkull1QuestDone {
                player_id: giver_id,
            });
            events.push(SeymourOutcomeEvent::UpdateSeymourState {
                player_id: giver_id,
                new_state: 10,
            });
            if let Some(character) = self.characters.get(&giver_id) {
                if army_rank_for_points(character.military_points) == 0 {
                    if let Some(character) = self.characters.get_mut(&giver_id) {
                        character.military_points = 1;
                        character.flags.insert(CharacterFlags::UPDATE);
                    }
                    self.npc_quiet_say(
                        seymour_id,
                        &format!(
                            "Welcome to the Imperial Army, {} {giver_name}!",
                            army_rank_name(1)
                        ),
                    );
                }
            }
            self.destroy_item(item_id);
        } else if template_id == IID_AREA2_ZOMBIESKULL2 && seymour_state == 11 {
            // C `} else if (it[in].ID == IID_AREA2_ZOMBIESKULL2 && ppd->
            // seymour_state == 11) { say("Ah. Well done, %s."); tmp =
            // questlog_done(co, 11); ppd->seymour_state = 12; if (tmp ==
            // 1) { give_military_pts(cn, co, 1, 1); } destroy_item(ch[cn]
            // .citem); ch[cn].citem = 0; }` (`area3.c:918-930`).
            self.npc_quiet_say(seymour_id, &format!("Ah. Well done, {giver_name}."));
            events.push(SeymourOutcomeEvent::ZombieSkull2QuestDone {
                player_id: giver_id,
                seymour_id,
            });
            events.push(SeymourOutcomeEvent::UpdateSeymourState {
                player_id: giver_id,
                new_state: 12,
            });
            self.destroy_item(item_id);
        } else {
            // C `else { say("Thou hast better use for this than I do.
            // Well, if there is a use for it at all."); if (!give_char_
            // item(co, ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].
            // citem = 0; }` (`area3.c:931-937`).
            self.npc_quiet_say(
                seymour_id,
                "Thou hast better use for this than I do. Well, if there is a use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_SEYMOUR;

/// C `struct seymour_driver_data` (`src/area/3/area3.c:660-663`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeymourDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
