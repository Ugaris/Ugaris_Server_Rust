//! Two-City alchemist (`CDR_TWOALCHEMIST`), "Cervik" - the spider-poison
//! quest giver, quest 31 ("Spider Poison").
//!
//! Ports `src/area/17/two.c::alchemist` (`:2950-3144`); `ch_died_driver`/
//! `ch_respawn_driver` are both plain `return 1;` no-ops for
//! `CDR_TWOALCHEMIST` in C, so no death/respawn hook exists for this NPC.
//!
//! Follows the same `World`/`PlayerRuntime` split established by
//! `world::npc::area17::two_skelly`: `alchemist_state` lives on
//! `crate::player::PlayerRuntime::twocity_ppd` (via `twocity_alchemist_
//! state`/`set_twocity_alchemist_state`), not `World`, so the caller
//! supplies a per-player fact snapshot ([`TwoAlchemistPlayerFacts`]) up
//! front and applies the returned [`TwoAlchemistOutcomeEvent`]s
//! afterwards.
//!
//! A real C quirk reproduced exactly: the quest-completion potion reward
//! (`combo_potion3`/`security_potion`, gated on `ch[co].level`) only
//! fires on the 1st/3rd/7th/10th completion (`tmp == 1 || 3 || 7 ||
//! 10`), and the "repeat" QA response can only reset `alchemist_state`
//! back to `0` while it is still `<= 4` - once a turn-in sets it to `5`,
//! nothing in this file ever resets it, so in practice the quest only
//! completes once per character. This is ported as-is (see
//! [`Self::two_alchemist_handle_give_message`]/the "repeat" branch in
//! [`Self::two_alchemist_handle_text_message`]), not "fixed", per the
//! project's own porting rules.
//!
//! Because the potion-reward decision depends on `PlayerRuntime::
//! quest_log`'s completion count (only known after `QuestLog::
//! complete_legacy` runs) and on `ZoneLoader` (item template
//! instantiation), both of which live outside `World`, the full
//! completion/reward/`say` sequence for a successful turn-in is deferred
//! to the [`TwoAlchemistOutcomeEvent::QuestDone`] event and finished by
//! the `ugaris-server`-side `area17.rs` glue - unlike `two_skelly`, whose
//! quest 30 completion has no reward branching.
use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_TWOALCHEMIST};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_AREA17_POISON;
use crate::world::*;

use super::TWOCITY_QA;

/// C `char_dist(cn, co) > 10` (`two.c:2999`).
const TWO_ALCHEMIST_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`two.c:2982`).
const TWO_ALCHEMIST_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`two.c:2987`, `:3055`).
const TWO_ALCHEMIST_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`two.c:3138`): idle "return to post" threshold.
const TWO_ALCHEMIST_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `struct alchemist_driver_data` (`two.c:2945-2948`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoAlchemistDriverData {
    pub last_talk_tick: u64,
    pub current_victim: Option<CharacterId>,
}

/// Per-player facts [`World::process_two_alchemist_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoAlchemistPlayerFacts {
    /// `PlayerRuntime::twocity_alchemist_state()`.
    pub alchemist_state: i32,
}

/// A side effect [`World::process_two_alchemist_actions`] could not apply
/// directly because it touches `PlayerRuntime`/`ZoneLoader`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoAlchemistOutcomeEvent {
    /// Write the new `twocity_ppd.alchemist_state` back.
    UpdateAlchemistState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 31)` (`two.c:3012`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 31)` plus its reward branch (`two.c:3092-
    /// 3117`) - finished server-side since it needs `QuestLog::
    /// complete_legacy`'s completion count and `ZoneLoader` item
    /// instantiation.
    QuestDone {
        player_id: CharacterId,
        alchemist_id: CharacterId,
    },
}

impl World {
    /// C `alchemist`'s per-tick body (`two.c:2950-3144`).
    pub fn process_two_alchemist_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TwoAlchemistPlayerFacts>,
        area_id: u16,
    ) -> Vec<TwoAlchemistOutcomeEvent> {
        let alchemist_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TWOALCHEMIST
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for alchemist_id in alchemist_ids {
            self.process_two_alchemist_tick(alchemist_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_two_alchemist_tick(
        &mut self,
        alchemist_id: CharacterId,
        player_facts: &HashMap<CharacterId, TwoAlchemistPlayerFacts>,
        area_id: u16,
        events: &mut Vec<TwoAlchemistOutcomeEvent>,
    ) {
        let Some(alchemist_name) = self.characters.get(&alchemist_id).map(|c| c.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::TwoAlchemist(mut data)) = self
            .characters
            .get(&alchemist_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&alchemist_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.two_alchemist_handle_char_message(
                    alchemist_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.two_alchemist_handle_text_message(
                    alchemist_id,
                    &alchemist_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.two_alchemist_handle_give_message(
                    alchemist_id,
                    message,
                    player_facts,
                    events,
                ),
                _ => {}
            }
        }

        if let Some(alchemist) = self.characters.get_mut(&alchemist_id) {
            alchemist.driver_state = Some(CharacterDriverState::TwoAlchemist(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`two.c:3134-3136`).
        if let (Some(alchemist), Some((tx, ty))) =
            (self.characters.get(&alchemist_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(alchemist.x), i32::from(alchemist.y), tx, ty)
            {
                if let Some(alchemist_mut) = self.characters.get_mut(&alchemist_id) {
                    let _ = turn(alchemist_mut, direction as u8);
                }
            }
        }

        let data = match self
            .characters
            .get(&alchemist_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::TwoAlchemist(data)) => *data,
            _ => return,
        };

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (secure_move_
        // driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret, lastact))
        // return; } do_idle(cn, TICKS);` (`two.c:3138-3143`). `tmpx`/
        // `tmpy` reuse `rest_x`/`rest_y`, the same substitution every
        // other stationary NPC in this codebase makes.
        if data.last_talk_tick + TWO_ALCHEMIST_RETURN_TO_POST_TICKS < self.tick.0 {
            let (post_x, post_y) = self
                .characters
                .get(&alchemist_id)
                .map(|alchemist| (alchemist.rest_x, alchemist.rest_y))
                .unwrap_or_default();
            if self.secure_move_driver(
                alchemist_id,
                post_x,
                post_y,
                Direction::Left as u8,
                0,
                0,
                area_id,
            ) {
                return;
            }
        }
        // C `do_idle(cn, TICKS);` (`two.c:3143`) - not modeled, same
        // precedent as every other stationary dialogue-only NPC in this
        // codebase: it has no observable effect in this message-driven
        // architecture.
    }

    /// C `alchemist`'s `NT_CHAR` branch (`two.c:2966-3049`).
    #[allow(clippy::too_many_arguments)]
    fn two_alchemist_handle_char_message(
        &mut self,
        alchemist_id: CharacterId,
        data: &mut TwoAlchemistDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoAlchemistPlayerFacts>,
        events: &mut Vec<TwoAlchemistOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(alchemist) = self.characters.get(&alchemist_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`two.c:2970-2973`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`two.c:2976-2979`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`two.c:2982-2985`).
        if tick < data.last_talk_tick + TWO_ALCHEMIST_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->
        // current_victim && dat->current_victim != co) continue;`
        // (`two.c:2987-2990`).
        if tick < data.last_talk_tick + TWO_ALCHEMIST_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`two.c:2992-2996`).
        if alchemist_id == player_id
            || !char_see_char(&alchemist, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`two.c:2998-3002`).
        if char_dist(&alchemist, &player) > TWO_ALCHEMIST_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->alchemist_state) { ... }` (`two.c:3008-3042`).
        match facts.alchemist_state {
            0 => {
                self.npc_say(
                    alchemist_id,
                    &format!(
                        "Too much sulphur. Yes, too much sulphur. Oh, hello, {}. I am {}, the alchemist.",
                        player.name, alchemist.name
                    ),
                );
                events.push(TwoAlchemistOutcomeEvent::QuestOpen { player_id });
                events.push(TwoAlchemistOutcomeEvent::UpdateAlchemistState {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
            }
            1 => {
                self.npc_say(
                    alchemist_id,
                    &format!(
                        "If thou art not too busy, {}, thou couldst do me a favor. I need spider poison for my experiments, but I am too busy to go looking for it. If thou wouldst bring me some, I'd reward thee.",
                        player.name
                    ),
                );
                events.push(TwoAlchemistOutcomeEvent::UpdateAlchemistState {
                    player_id,
                    new_state: 2,
                });
                didsay = true;
            }
            2 => {
                self.npc_say(
                    alchemist_id,
                    "I know of some poisonous spiders who live underground, in a plantage the ancients built to grow food. When I went there some years ago, the lighting was beginning to fail. Anyway. The entrance is close to the transport portal.",
                );
                events.push(TwoAlchemistOutcomeEvent::UpdateAlchemistState {
                    player_id,
                    new_state: 3,
                });
                didsay = true;
            }
            3 => {
                self.npc_say(
                    alchemist_id,
                    "Thou wilt need to look for bright red spiders, those are the only ones who have the poison I need.",
                );
                events.push(TwoAlchemistOutcomeEvent::UpdateAlchemistState {
                    player_id,
                    new_state: 4,
                });
                didsay = true;
            }
            // `alchemist_state == 4`/`5`: silent (`two.c:3038-3041`).
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`two.c:3043-3047`).
        if didsay {
            data.last_talk_tick = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `alchemist`'s `NT_TEXT` branch (`two.c:3052-3077`), wired through
    /// the generic `analyse_text_qa` matcher (same pattern as `world::
    /// npc::area17::two_skelly`'s text handler).
    #[allow(clippy::too_many_arguments)]
    fn two_alchemist_handle_text_message(
        &mut self,
        alchemist_id: CharacterId,
        alchemist_name: &str,
        data: &mut TwoAlchemistDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoAlchemistPlayerFacts>,
        events: &mut Vec<TwoAlchemistOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->
        // current_victim) dat->current_victim = 0;` (`two.c:3055-3057`).
        let tick = self.tick.0;
        if tick > data.last_talk_tick + TWO_ALCHEMIST_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
        {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`two.c:3059-3062`).
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

        // C `analyse_text_driver`'s own guard clauses (`two.c:126-144`):
        // ignore our own talk, non-players/player-likes, not-visible.
        if alchemist_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(alchemist) = self.characters.get(&alchemist_id).cloned() else {
            return;
        };
        if !char_see_char(&alchemist, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let alchemist_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.alchemist_state)
            .unwrap_or(0);

        let mut didsay = false;
        // C's `analyse_text_driver` calls `say(cn, qa[q].answer, ...)`
        // directly (`two.c:206`), same as `two_skelly`.
        match analyse_text_qa(text, alchemist_name, &speaker.name, TWOCITY_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(alchemist_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`two.c:3065-3071`) - only resets
            // while `alchemist_state <= 4`; once turned in (`5`), the
            // repeat command can no longer restart the greeting ladder.
            TextAnalysisOutcome::Matched(2) => {
                if alchemist_state <= 4 {
                    data.last_talk_tick = 0;
                    events.push(TwoAlchemistOutcomeEvent::UpdateAlchemistState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`two.c:3073-3076`) - note this does *not* touch `dat->
        // last_talk` (except the explicit reset inside the `case 2`
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `alchemist`'s `NT_GIVE` branch (`two.c:3080-3126`): a spider
    /// poison turn-in (while `alchemist_state <= 4`) clears the alchemist's
    /// cursor item, destroys any matching poison in the giver's own
    /// inventory (matching C's belt-and-suspenders `destroy_item_byID`
    /// call), marks `alchemist_state = 5`, and defers the quest-
    /// completion/reward/`say` sequence to
    /// [`TwoAlchemistOutcomeEvent::QuestDone`] since it needs
    /// `PlayerRuntime::quest_log`'s completion count and `ZoneLoader`.
    /// Anything else is handed straight back (falling back to destroying
    /// it if the player's inventory is full), matching C's plain
    /// `give_char_item` (not `give_char_item_smart`).
    fn two_alchemist_handle_give_message(
        &mut self,
        alchemist_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoAlchemistPlayerFacts>,
        events: &mut Vec<TwoAlchemistOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get(&alchemist_id)
            .and_then(|alchemist| alchemist.cursor_item)
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            return;
        };
        let alchemist_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.alchemist_state)
            .unwrap_or(0);

        if template_id == IID_AREA17_POISON && alchemist_state <= 4 {
            // C `ppd->alchemist_state = 5; tmp = questlog_done(co, 31);
            // destroy_item_byID(co, IID_AREA17_POISON); destroy_item(ch
            // [cn].citem); ch[cn].citem = 0;` (`two.c:3090-3097`), plus
            // the reward `say`/`create_item` cascade (`:3099-3117`),
            // finished by the server-side `QuestDone` handler.
            events.push(TwoAlchemistOutcomeEvent::UpdateAlchemistState {
                player_id: giver_id,
                new_state: 5,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA17_POISON);
            if let Some(alchemist) = self.characters.get_mut(&alchemist_id) {
                alchemist.cursor_item = None;
            }
            self.destroy_item(item_id);
            events.push(TwoAlchemistOutcomeEvent::QuestDone {
                player_id: giver_id,
                alchemist_id,
            });
        } else {
            // C `else { say("Thou hast better use..."); if (!give_char_
            // item(co, ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].
            // citem = 0; }` (`two.c:3118-3124`).
            self.npc_say(
                alchemist_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
            if let Some(alchemist) = self.characters.get_mut(&alchemist_id) {
                alchemist.cursor_item = None;
            }
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;
