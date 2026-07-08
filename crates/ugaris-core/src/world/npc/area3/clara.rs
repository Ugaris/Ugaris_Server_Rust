//! Clara, the swamp-outpost commander NPC (`CDR_SWAMPCLARA`), ported from
//! `src/area/15/swamp.c::clara_driver` (`:510-764`) plus its shared
//! `analyse_text_driver`/local `qa[]` copy (`:85-197`, ported as
//! [`CLARA_QA`] - see that constant's own doc comment for why it is *not*
//! `world::npc::area3::AREA3_QA`) and `monster_dead`'s Clara-facing half
//! (`:766-800`, already ported separately as [`clara_state_after_swamp_
//! monster_death`], wired live in `ugaris-server`'s `world_events::
//! death_hooks::apply_swamp_monster_death_from_hurt_event`).
//!
//! C's own comment on `clara_driver` - "note: the ppd is borrowed from
//! area3 - the missions interact..." - only shares the `area3_ppd` struct
//! layout (`clara_state`/`kelly_state` fields) with `src/area/3/area3.c`;
//! `swamp.c` is a *separate* C translation unit with its own local `qa[]`
//! table and its own local `analyse_text_driver` copy (different guard
//! clauses - see [`CLARA_QA`]'s doc comment), so this file lives under
//! `world/npc/area3/` (matching the shared PPD data, and every other
//! sibling driver that reads/writes `kelly_state`/`clara_state`) rather
//! than a new `world/npc/area15/`.
//!
//! [`clara_dialogue_step`] ports the entire sixteen-state (`0`-`15`)
//! dialogue switch as a pure function (greeting -> gated on `world::kelly`
//! reaching state 15 -> status report -> gated on `world::kelly` reaching
//! state 18 -> the hardkill-weapon quest (`QLOG` 21): find the man who
//! knows how to hurt the immune swamp beast, forge the `IID_HARDKILL`
//! weapon via three stone-circle rituals (`World::apply_swamp_monster_
//! death_driver`), slay the beast, report back - completing quest 21 and
//! awarding military points at both the weapon-forging and kill-report
//! milestones). [`World::process_clara_actions`] is the runtime message-
//! loop integration this module doc comment's earlier revision left as a
//! documented gap: `NT_CHAR` sighting (calls `clara_dialogue_step`,
//! applies the reward directly via `World::give_military_pts_from_npc`,
//! same precedent as `world::kelly`'s per-shrine reward), `NT_TEXT`
//! small-talk/`repeat` (via [`CLARA_QA`] + the generic `analyse_text_qa`
//! matcher), and `NT_GIVE` (Clara always gives everything straight back -
//! she has no turn-in item, unlike `world::kelly`/`world::seymour`).
//! `PlayerRuntime`-touching side effects ([`ClaraOutcomeEvent`]) are
//! applied by `ugaris-server`'s `area3.rs::apply_clara_events`.

#[allow(unused_imports)]
use crate::world::*;

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClaraDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaraDialogueContext<'a> {
    pub player_name: &'a str,
    pub clara_name: &'a str,
    pub army_rank: &'a str,
    pub kelly_state: i32,
    pub clara_state: i32,
    pub has_hardkill_item: bool,
    pub hardkill_ritual_progress: u8,
    pub questlog_21_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaraDialogueOutcome {
    pub clara_state: i32,
    pub text: Option<String>,
    pub open_questlog: Option<u16>,
    pub complete_questlog: Option<u16>,
    pub military_points: i32,
    pub military_exp: i32,
}

pub fn clara_dialogue_step(context: ClaraDialogueContext<'_>) -> ClaraDialogueOutcome {
    let mut state = context.clara_state;
    let mut open_questlog = None;
    let mut complete_questlog = None;
    let mut military_points = 0;
    let mut military_exp = 0;
    let text = match state {
        0 => {
            state += 1;
            Some(format!(
                "Greetings, {}! I am {}, First Sergeant of the Seyan'Du and commander of this outpost.",
                context.player_name, context.clara_name
            ))
        }
        1 if context.kelly_state >= 15 => {
            state += 1;
            clara_dialogue_step_text_after_fallthrough(&mut state, context)
        }
        1 => None,
        2 => clara_dialogue_step_text_after_fallthrough(&mut state, context),
        3 => {
            state += 1;
            Some(
                "Under the current circumstances, I do not recommend sending reinforcements to secure the road. We cannot afford to bind our forces here. Now go back to Aston and deliver this report."
                    .to_string(),
            )
        }
        4 => {
            state += 1;
            Some(format!(
                "Afterwards come back here, I have more work for thee. That will be all, {}. Dismissed!",
                context.army_rank
            ))
        }
        5 if context.kelly_state >= 18 => {
            state += 1;
            open_questlog = Some(21);
            state += 1;
            Some(format!(
                "I have a difficult mission for thee, {}. The main reason we had to retreat to this camp was one huge swamp beast. It seemed to be immune to our attacks.",
                context.player_name
            ))
        }
        5 => None,
        6 => {
            open_questlog = Some(21);
            state += 1;
            Some(format!(
                "I have a difficult mission for thee, {}. The main reason we had to retreat to this camp was one huge swamp beast. It seemed to be immune to our attacks.",
                context.player_name
            ))
        }
        7 => {
            state += 1;
            Some(
                "I want thee to find a way to slay it. I have heard rumors about a man who used to live with the swamp beasts north-east of this camp. Mayhap he knows a way to injure this beast."
                    .to_string(),
            )
        }
        8 => {
            state += 1;
            Some(format!(
                "Dismissed, {}. And good luck. Thou wilt need it.",
                context.army_rank
            ))
        }
        9 if context.has_hardkill_item => {
            if context.questlog_21_count == 0 {
                military_points = 4;
                military_exp = EXP_AREA15_HARDKILL;
            }
            state += 1;
            clara_hardkill_report_text(&mut state, context)
        }
        9 => None,
        10 => clara_hardkill_report_text(&mut state, context),
        11 if context.has_hardkill_item && context.hardkill_ritual_progress >= 36 => {
            state += 1;
            state += 1;
            Some("Now that thou knowest how to kill that beast, please go and do it.".to_string())
        }
        11 => None,
        12 => {
            state += 1;
            Some("Now that thou knowest how to kill that beast, please go and do it.".to_string())
        }
        13 => None,
        14 => {
            complete_questlog = Some(21);
            if context.questlog_21_count == 1 {
                military_points = 8;
                military_exp = 1;
            }
            state += 1;
            Some(format!("Well done indeed, {}!", context.player_name))
        }
        15 => {
            state += 1;
            Some(format!(
                "The swamp will be safer now, but more dangers await thee on thy travels. May Ishtar be with thee, {}.",
                context.player_name
            ))
        }
        _ => None,
    };

    ClaraDialogueOutcome {
        clara_state: state,
        text,
        open_questlog,
        complete_questlog,
        military_points,
        military_exp,
    }
}

fn clara_dialogue_step_text_after_fallthrough(
    state: &mut i32,
    context: ClaraDialogueContext<'_>,
) -> Option<String> {
    *state += 1;
    Some(format!(
        "I assume thou hast been sent from Aston, {}, to report on our status. The road through the swamp is no longer secure and we have been under attack from beasts emerging from the swamp.",
        context.army_rank
    ))
}

fn clara_hardkill_report_text(
    state: &mut i32,
    context: ClaraDialogueContext<'_>,
) -> Option<String> {
    *state += 1;
    if context.has_hardkill_item && context.hardkill_ritual_progress < 36 {
        Some(format!(
            "So that is how one can kill them. Thou wilt need to find all three stone circles and perform the ritual in each one, then, {}.",
            context.player_name
        ))
    } else {
        Some("So that is how one can kill them.".to_string())
    }
}

pub fn clara_replay_state_after_text_analysis(clara_state: i32, didsay: i32) -> i32 {
    if didsay != 2 {
        return clara_state;
    }
    match clara_state {
        ..=5 => 0,
        6..=9 => 6,
        10..=11 => 10,
        12..=13 => 12,
        15..=16 => 15,
        _ => clara_state,
    }
}

pub fn clara_state_after_swamp_monster_death(
    clara_state: i32,
    killer_is_player: bool,
    monster_is_hardkill: bool,
) -> i32 {
    if killer_is_player && monster_is_hardkill && (12..=13).contains(&clara_state) {
        14
    } else {
        clara_state
    }
}

// ---- runtime NPC message-loop integration ----

use std::collections::HashMap;

use crate::drvlib::offset2dx;
use crate::world::hurt::IID_HARDKILL;

/// C `char_dist(cn, co) > 10` (`swamp.c:559`): the `NT_CHAR` greeting
/// distance gate.
const CLARA_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`swamp.c:542`).
const CLARA_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`swamp.c:547`, `:694`).
const CLARA_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`swamp.c:757`): idle "return to post" threshold.
const CLARA_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `struct qa qa[]` (`swamp.c:85-92`) - `clara_driver`'s own file-local
/// small-talk table. This is a smaller, *distinct* array from
/// `world::npc::area3::AREA3_QA`: `swamp.c` and `area3.c` are two separate
/// C translation units that each define their own file-local `qa[]`/
/// `analyse_text_driver`; only the `area3_ppd` struct layout itself is
/// actually shared between them (per `clara_driver`'s own C comment -
/// "note: the ppd is borrowed from area3 - the missions interact...").
/// Missing every one of `AREA3_QA`'s extra rows (`restart`/`please
/// repeat`/`please restart`/`aye`/`nay`/`shortcut to caligar`/`explain`/
/// `list`/`money`/the 80-row raise-lower block) - those belong only to
/// `area3.c`'s own copy, read by its own six NPC drivers.
const CLARA_QA: &[TextQaEntry] = &[
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
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
];

/// Per-player facts [`World::process_clara_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World`'s pure per-tick pass
/// cannot see: `area3_ppd.clara_state`/`kelly_state` live on
/// `PlayerRuntime`, and - since "does *this* player currently carry the
/// hardkill weapon" depends on which `Character` owns which `Item`, a
/// cross-cutting `World` fact - the caller snapshots the `WN_RHAND` check
/// once per player up front too, same shape as every other area-3 sibling
/// driver's facts struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaraPlayerFacts {
    /// `PlayerRuntime::area3_clara_state()`.
    pub clara_state: i32,
    /// `PlayerRuntime::area3_kelly_state()`, needed for the `case 1`/
    /// `case 5` gates (`ppd->kelly_state >= 15`/`>= 18`).
    pub kelly_state: i32,
    /// `(in = ch[co].item[WN_RHAND]) && it[in].ID == IID_HARDKILL`
    /// (`swamp.c:631`, `:640`, `:652`).
    pub has_hardkill_item: bool,
    /// `it[in].drdata[37]` (`swamp.c:640`, `:652`) - only meaningful when
    /// `has_hardkill_item` is `true`.
    pub hardkill_ritual_progress: u8,
    /// `PlayerRuntime::quest_log.count(21)` (C `questlog_count(co, 21)`).
    pub questlog_21_count: i32,
}

/// A side effect [`World::process_clara_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaraOutcomeEvent {
    /// Write the new `area3_ppd.clara_state` back.
    UpdateClaraState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 21)` (`swamp.c:614`).
    QuestOpen { player_id: CharacterId },
    /// C `questlog_done(co, 21)` (`swamp.c:666`) - the exp/resend half;
    /// the conditional `give_military_pts` reward is applied directly in
    /// `World` (see [`World::clara_handle_char_message`]'s doc comment).
    QuestDone { player_id: CharacterId },
}

impl World {
    /// C `ch_driver`'s `CDR_SWAMPCLARA` dispatch (`swamp.c:804-806`) ->
    /// `clara_driver`'s per-tick body (`swamp.c:510-764`).
    pub fn process_clara_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ClaraPlayerFacts>,
        area_id: u16,
    ) -> Vec<ClaraOutcomeEvent> {
        let clara_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SWAMPCLARA
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for clara_id in clara_ids {
            self.process_clara_messages(clara_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_clara_messages(
        &mut self,
        clara_id: CharacterId,
        player_facts: &HashMap<CharacterId, ClaraPlayerFacts>,
        area_id: u16,
        events: &mut Vec<ClaraOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Clara(mut data)) = self
            .characters
            .get(&clara_id)
            .and_then(|clara| clara.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&clara_id)
            .map(|clara| std::mem::take(&mut clara.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.clara_handle_char_message(
                    clara_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.clara_handle_text_message(
                    clara_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.clara_handle_give_message(clara_id, message),
                _ => {}
            }
        }

        if let Some(clara) = self.characters.get_mut(&clara_id) {
            clara.driver_state = Some(CharacterDriverState::Clara(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`swamp.c:753-755`).
        if let (Some(clara), Some((tx, ty))) =
            (self.characters.get(&clara_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(clara.x), i32::from(clara.y), tx, ty) {
                if let Some(clara_mut) = self.characters.get_mut(&clara_id) {
                    let _ = turn(clara_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`swamp.c:757-761`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other area-3 sibling driver uses.
        let last_talk = match self
            .characters
            .get(&clara_id)
            .and_then(|clara| clara.driver_state.as_ref())
        {
            Some(CharacterDriverState::Clara(data)) => data.last_talk,
            _ => return,
        };
        if last_talk + CLARA_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(clara) = self.characters.get(&clara_id) else {
                return;
            };
            let (post_x, post_y) = (clara.rest_x, clara.rest_y);
            self.secure_move_driver(
                clara_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `clara_driver`'s `NT_CHAR` branch (`swamp.c:525-687`), driven by
    /// the pure [`clara_dialogue_step`] state machine for the actual
    /// dialogue switch. The military-points/exp reward `clara_dialogue_
    /// step` computes (`case 9`'s hardkill-report bonus, `case 14`'s
    /// completion bonus) is applied directly here via `World::
    /// give_military_pts_from_npc` (touches only `Character` fields), same
    /// precedent as `world::kelly`'s per-shrine reward.
    fn clara_handle_char_message(
        &mut self,
        clara_id: CharacterId,
        data: &mut ClaraDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ClaraPlayerFacts>,
        events: &mut Vec<ClaraOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(clara) = self.characters.get(&clara_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`swamp.c:530-533`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`swamp.c:536-539`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`swamp.c:542-545`).
        if tick < data.last_talk + CLARA_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`swamp.c:547-550`).
        if tick < data.last_talk + CLARA_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`swamp.c:552-556`).
        if clara_id == player_id || !char_see_char(&clara, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`swamp.c:558-562`).
        if char_dist(&clara, &player) > CLARA_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id).copied() else {
            return;
        };

        let rank_name = army_rank_name(army_rank_for_points(player.military_points));
        let outcome = clara_dialogue_step(ClaraDialogueContext {
            player_name: &player.name,
            clara_name: &clara.name,
            army_rank: rank_name,
            kelly_state: facts.kelly_state,
            clara_state: facts.clara_state,
            has_hardkill_item: facts.has_hardkill_item,
            hardkill_ritual_progress: facts.hardkill_ritual_progress,
            questlog_21_count: facts.questlog_21_count,
        });

        if let Some(text) = &outcome.text {
            self.npc_quiet_say(clara_id, text);
        }
        if outcome.clara_state != facts.clara_state {
            events.push(ClaraOutcomeEvent::UpdateClaraState {
                player_id,
                new_state: outcome.clara_state,
            });
        }
        if outcome.open_questlog.is_some() {
            events.push(ClaraOutcomeEvent::QuestOpen { player_id });
        }
        if outcome.complete_questlog.is_some() {
            events.push(ClaraOutcomeEvent::QuestDone { player_id });
        }
        if outcome.military_points != 0 {
            self.give_military_pts_from_npc(
                player_id,
                clara_id,
                outcome.military_points,
                outcome.military_exp,
                u32::from(self.area_id),
            );
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`swamp.c:682-686`).
        if outcome.text.is_some() {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `clara_driver`'s `NT_TEXT` branch (`swamp.c:691-732`), wired
    /// through `swamp.c`'s own local `analyse_text_driver`/`qa[]` copy
    /// (ported as [`CLARA_QA`] + the generic `analyse_text_qa` matcher -
    /// see that constant's doc comment for why this is *not*
    /// `world::npc::area3::AREA3_QA`). Unlike `area3.c`'s copy (reused by
    /// `world::kelly`/etc.), `swamp.c`'s own local `analyse_text_driver`
    /// (`:101-197`) checks `CF_PLAYER | CF_PLAYERLIKE` (not just
    /// `CF_PLAYER`) and has its own `char_dist(cn,co) > 16` guard
    /// commented out (`:120`) - i.e. genuinely no active distance check
    /// here, unlike `area3.c`'s sibling copy.
    fn clara_handle_text_message(
        &mut self,
        clara_id: CharacterId,
        data: &mut ClaraDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ClaraPlayerFacts>,
        events: &mut Vec<ClaraOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`swamp.c:694-696`).
        let tick = self.tick.0;
        if tick > data.last_talk + CLARA_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`swamp.c:698-701`).
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

        // C `analyse_text_driver`'s own guard clauses (`swamp.c:112-124`):
        // ignore our own talk, non-players/player-likes, not-visible (no
        // active distance check - see this fn's own doc comment).
        if clara_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(clara) = self.characters.get(&clara_id).cloned() else {
            return;
        };
        if !char_see_char(&clara, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let clara_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.clara_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, &clara.name, &speaker.name, CLARA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(clara_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`swamp.c:704-726`): five mutually
            // exclusive state buckets, ported as [`clara_replay_state_
            // after_text_analysis`].
            TextAnalysisOutcome::Matched(2) => {
                let new_state = clara_replay_state_after_text_analysis(clara_state, 2);
                if new_state != clara_state {
                    data.last_talk = 0;
                    events.push(ClaraOutcomeEvent::UpdateClaraState {
                        player_id: speaker_id,
                        new_state,
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
        // (`swamp.c:728-731`) - note this does *not* touch `dat->
        // last_talk` (except the explicit reset inside the `case 2`
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `clara_driver`'s `NT_GIVE` branch (`swamp.c:735-745`): Clara has
    /// no turn-in item of her own (unlike `world::kelly`/`world::
    /// seymour`) - she always gives everything handed to her straight
    /// back.
    fn clara_handle_give_message(
        &mut self,
        clara_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&clara_id)
            .and_then(|clara| clara.cursor_item.take())
        else {
            return;
        };
        // C `quiet_say(cn, "Thou hast better use for this than I do. Well,
        // if there is use for it at all."); if (!give_char_item(co,
        // ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].citem = 0;`
        // (`swamp.c:739-743`).
        self.npc_quiet_say(
            clara_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

/// Hardkill-weapon fact lookup helper for callers building
/// [`ClaraPlayerFacts`]: C `(in = ch[co].item[WN_RHAND]) && it[in].ID ==
/// IID_HARDKILL` (`swamp.c:631`, `:640`, `:652`). Returns
/// `(has_hardkill_item, ritual_progress)`.
pub fn clara_hardkill_weapon_facts(world: &World, player_id: CharacterId) -> (bool, u8) {
    let Some(item_id) = world
        .characters
        .get(&player_id)
        .and_then(|character| character.inventory.get(worn_slot::RIGHT_HAND))
        .copied()
        .flatten()
    else {
        return (false, 0);
    };
    let Some(item) = world.items.get(&item_id) else {
        return (false, 0);
    };
    if item.template_id != IID_HARDKILL {
        return (false, 0);
    }
    let progress = item.driver_data.get(37).copied().unwrap_or(0);
    (true, progress)
}
