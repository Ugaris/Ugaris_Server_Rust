//! Underground army Commander mission-giver (`CDR_FDEMON_BOSS`).
//!
//! Ports C `src/area/8/fdemon.c::fdemon_boss` (`:1534-1900`): a 33-stage
//! `farmy_ppd.boss_stage` dialogue chain that greets every nearby player
//! and hands out sequential Defense-Station-activation missions (`world::
//! fdemon_loader_station_report` already ports the loader-side half of
//! stages 0-27; this file ports the NPC-side "what to say and when to
//! advance" half plus the open-ended stage-28+ scouting phase and
//! `platoon_exp`'s player-reward math), plus the shared `NT_TEXT` "repeat"
//! stage-reset ladder (C's `analyse_text_driver` returning code `8`, `qa[]`
//! row `{"repeat"}` - see `super::FDEMON_QA`).
//!
//! `farmy_ppd.boss_stage`/`boss_timer`/`boss_counter`/`boss_reported` are
//! `PlayerRuntime`-resident (`crates/ugaris-core/src/player/areas_misc.rs`),
//! not reachable from `World` - so, following the same split established by
//! `FdemonLoaderChanged`'s server-side dispatch (`ugaris-server`'s
//! `tick_item_use_edemon_fdemon.rs`), the actual dialogue/exp/rank side
//! effects run directly against `World` here (none of them touch
//! `PlayerRuntime`), while only the small `farmy_ppd` field delta is
//! reported back to the caller (`ugaris-server`'s `tick_npc::area8`, the
//! only layer with both `&mut World` and `&mut ServerRuntime`) to persist.
//!
//! Deviations/gaps (documented, not silent):
//! - C reacts to a live `NT_CHAR` message (populated by `notify_area`'s
//!   sight broadcast) for the per-stage greeting. This port instead does a
//!   direct per-tick scan of nearby `CF_PLAYER` characters ([`World::
//!   fdemon_boss_sighted_players`]) - the same class of "replace message-
//!   driven sighting with a direct scan" simplification already
//!   established by `world::npc::area8::fdemon_demon`/`area4::tester`/
//!   `janitor`. The `NT_TEXT` "repeat" detection, by contrast, *does* use
//!   the real `Character::driver_messages` queue ([`World::
//!   fdemon_boss_process_text_messages`]), since nearby player speech is
//!   already reliably delivered there (`ugaris-server`'s
//!   `commands_chat.rs::push_driver_text_message` fan-out) - no
//!   replacement needed.
//! - The `"take"`/`"drop"` soldier commands (`fdemon_boss`'s own `NT_TEXT`
//!   tail, calling `take_soldiers`/`drop_soldiers`) are not ported: they
//!   belong to the still-unported `CDR_FDEMON_ARMY` recruitment system
//!   (`farmy_ppd.soldier[]`/`farmy_data`) - documented gap, not silent, see
//!   `PORTING_TODO.md`'s Area 8 entry.
//! - [`World::fdemon_platoon_exp`]'s soldier-exp loop (crediting recruited
//!   soldiers' own exp/rank) is likewise unreachable without
//!   `CDR_FDEMON_ARMY`; only the always-live player-exp/rank-promotion
//!   half is ported (same class of gap already documented on
//!   `PlayerRuntime::advance_farmy_golem_kill_stage`).
//! - C's `regenerate_driver`/`spell_self_driver` self-management calls and
//!   the trailing `do_idle` are not ported - `fdemon_boss` has no self-
//!   defense cascade at all in C (unlike almost every other NPC in this
//!   codebase), and idle/regen scheduling for a stationary, never-attacked
//!   NPC has no further observable effect worth a dedicated per-tick call
//!   here (same precedent as `astro1.rs`'s documented omission).
//! - `NT_CREATE`'s `if (ch[cn].arg) ch[cn].arg = NULL;` has no Rust
//!   equivalent (no zone-file `arg` scratch field modeled for this
//!   driver) - omitted, matching every other simple ambient/dialogue NPC
//!   in this codebase.

use crate::{
    character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_FDEMON_BOSS, NT_TEXT},
    text::{COL_STR_LIGHT_BLUE, COL_STR_RESET},
    world::*,
};

use super::FDEMON_QA;

/// Matches `world::text::notify_area`'s own broadcast radius - see module
/// doc comment.
const SIGHTING_SCAN_RADIUS: u16 = 32;

/// C `char_dist(cn, co) < 16` sighting range gate (`fdemon.c:1554`).
const FDEMON_BOSS_SIGHT_RANGE: i32 = 16;

/// C `analyse_text_driver`'s own `char_dist(cn, co) > 12` early-out
/// (`fdemon.c:209-211`), gating whether *any* small-talk (including the
/// "repeat" detection) can match at all.
const FDEMON_BOSS_TALK_RANGE: i32 = 12;

/// C `realtime - ppd->boss_timer > 5`'s throttle window (`fdemon.c:1556`).
pub const FDEMON_BOSS_TIMER_THROTTLE_SECONDS: i32 = 5;

/// Per-player facts snapshot for [`World::fdemon_boss_greet_player`] - the
/// caller (`ugaris-server`, the only layer with `PlayerRuntime` access)
/// builds one from the sighted player's `farmy_ppd` fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FdemonBossPlayerFacts {
    pub boss_stage: i32,
    pub boss_counter: i32,
    pub boss_reported: i32,
}

/// What changed for one player after a sighting - `None` fields mean
/// "unchanged"; the caller only writes back what's `Some`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FdemonBossStageUpdate {
    pub new_stage: Option<i32>,
    pub new_counter: Option<i32>,
    pub new_reported: Option<i32>,
    /// C sets `ppd->boss_timer = realtime;` in every case *except* the
    /// four "waiting for mission solve" stages (`5`/`8`/`11`/`14`/`17`/
    /// `20`/`23`/`26`) and case `30`'s nothing-new-found early exit.
    pub timer_touched: bool,
}

impl World {
    /// Every live `CDR_FDEMON_BOSS` character (C `ch_driver`'s
    /// `CDR_FDEMON_BOSS` case, `fdemon.c:3021,3024-3026`) - the caller
    /// (`ugaris-server`'s `tick_npc::area8`) drives the rest of the
    /// per-tick dispatch, since it alone has the `PlayerRuntime` access
    /// [`World::fdemon_boss_greet_player`]/[`fdemon_boss_repeat_reset`]'s
    /// results need to be persisted through.
    pub fn fdemon_boss_character_ids(&self) -> Vec<CharacterId> {
        self.characters
            .values()
            .filter(|character| {
                character.driver == CDR_FDEMON_BOSS
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect()
    }

    /// C `ch_driver`'s `CDR_FDEMON_BOSS` per-tick `NT_CHAR` sighting loop
    /// (`fdemon.c:1550-1778`, replaced by a direct scan - see module doc
    /// comment). Returns every visible `CF_PLAYER` character within
    /// [`FDEMON_BOSS_SIGHT_RANGE`] tiles whose driver isn't `CDR_LOSTCON`;
    /// callers are responsible for the `realtime - boss_timer > 5` throttle
    /// (`PlayerRuntime`-resident data this module can't read) before
    /// calling [`World::fdemon_boss_greet_player`].
    pub fn fdemon_boss_sighted_players(&self, boss_id: CharacterId) -> Vec<CharacterId> {
        let Some(boss) = self.characters.get(&boss_id) else {
            return Vec::new();
        };
        let daylight = self.date.daylight;
        let min_x = boss.x.saturating_sub(SIGHTING_SCAN_RADIUS);
        let max_x = boss.x.saturating_add(SIGHTING_SCAN_RADIUS);
        let min_y = boss.y.saturating_sub(SIGHTING_SCAN_RADIUS);
        let max_y = boss.y.saturating_add(SIGHTING_SCAN_RADIUS);
        self.characters
            .values()
            .filter(|target| {
                target.id != boss_id
                    && target.flags.contains(CharacterFlags::PLAYER)
                    && target.driver != CDR_LOSTCON
                    && target.x >= min_x
                    && target.x <= max_x
                    && target.y >= min_y
                    && target.y <= max_y
                    && char_dist(boss, target) < FDEMON_BOSS_SIGHT_RANGE
            })
            .filter(|target| char_see_char(boss, target, &self.map, daylight))
            .map(|target| target.id)
            .collect()
    }

    /// C `fdemon_boss`'s `switch (ppd->boss_stage)` body (`fdemon.c:
    /// 1558-1776`) for a single sighted player, already throttle-gated by
    /// the caller. `player_id` is the sighted player (C's `co`); `boss_id`
    /// is the Commander itself (C's `cn`, the `say()` speaker).
    pub fn fdemon_boss_greet_player(
        &mut self,
        boss_id: CharacterId,
        player_id: CharacterId,
        facts: FdemonBossPlayerFacts,
        area_id: u32,
    ) -> FdemonBossStageUpdate {
        let Some(name) = self.characters.get(&player_id).map(|c| c.name.clone()) else {
            return FdemonBossStageUpdate::default();
        };
        let rank_name = army_rank_name(army_rank_for_points(
            self.characters
                .get(&player_id)
                .map_or(0, |c| c.military_points),
        ))
        .to_string();

        let FdemonBossPlayerFacts {
            boss_stage,
            boss_counter,
            boss_reported,
        } = facts;
        let mut update = FdemonBossStageUpdate {
            timer_touched: true,
            ..Default::default()
        };

        match boss_stage {
            0 => {
                if army_rank_for_points(
                    self.characters
                        .get(&player_id)
                        .map_or(0, |c| c.military_points),
                ) < 2
                {
                    self.npc_say(boss_id, &format!("Ah, {name}. The governer of Aston has some missions for you. You'd better head back there and do those first."));
                } else {
                    self.npc_say(boss_id, &format!("Welcome, {name}, to our underground headquarters. I am the commander of the underground army. We are trying to stop the demon's progress here, before they invade Aston again."));
                    update.new_stage = Some(1);
                }
            }
            1 => {
                self.npc_say_bytes(boss_id, &format!(
                    "Unfortunately, I have a lack of good leaders. But you can {COL_STR_LIGHT_BLUE}take{COL_STR_RESET} some men to explore the underground and solve your missions. Just be sure to {COL_STR_LIGHT_BLUE}drop{COL_STR_RESET} them off here again before you leave."
                ));
                update.new_stage = Some(2);
            }
            2 => {
                self.npc_say(boss_id, "These soldiers have been trained to obey some easy commands: 'Follow' makes them follow you. 'Front' makes them walk in front of you. With 'Back', they'll take one step back. They follow you more closely if you order a 'retreat'. And you can make them attack your enemy from 'behind'.");
                update.new_stage = Some(3);
            }
            3 => {
                self.npc_say(boss_id, "It is up to you if you want their help on your missions. I recommend you take them along, the enemies are numerous, and some are quite dangerous.");
                update.new_stage = Some(4);
            }
            4 => {
                self.npc_say(boss_id, &format!("Alright, {rank_name}, your first mission is to reach the Ancient Defense Station number 1, and refuel it. You can find some of the power-crystals of the ancients in the big hall to the south-east. Take plenty, they're growing fast."));
                update.new_stage = Some(5);
            }
            5 | 8 | 11 | 14 | 17 | 20 | 23 | 26 => {
                // "waiting for mission solve" - no message, no timer touch.
                update.timer_touched = false;
            }
            6 => {
                self.fdemon_platoon_exp(boss_id, player_id, 1000, 2, area_id);
                update.new_stage = Some(7);
            }
            7 => {
                self.npc_say(boss_id, "Your next mission is to activate Defense Station 3. That is the next station north-west of the one you activated in your last mission.");
                update.new_stage = Some(8);
            }
            9 => {
                self.fdemon_platoon_exp(boss_id, player_id, 1000, 2, area_id);
                update.new_stage = Some(10);
            }
            10 => {
                self.npc_say(boss_id, &format!("I know, {name}, you're tired of doing these simple missions, but they have to be done. We can't hold the demons back without those Defense Stations. {rank_name}, your mission is to activate Defense Station 2. It is located north-east of station 1."));
                update.new_stage = Some(11);
            }
            12 => {
                self.fdemon_platoon_exp(boss_id, player_id, 1000, 2, area_id);
                update.new_stage = Some(13);
            }
            13 => {
                self.npc_say(boss_id, &format!("Our knowledge of the underground system here is pretty limited, {name}. Your next mission is to find and activate the Defense Stations 4 and 5. I assume that these are pretty close to number 2 and 3."));
                update.new_stage = Some(14);
                update.new_counter = Some(0);
            }
            15 => {
                self.fdemon_platoon_exp(boss_id, player_id, 2000, 4, area_id);
                update.new_stage = Some(16);
            }
            16 => {
                self.npc_say(boss_id, &format!("I've been getting reports about some beings we called 'Fire Golems', {name}. It seems these beasts are very hard to kill. I've lost many good men to them. The few who made it back reported that only an attack in the back had any success. Your next mission is to slay one of those 'Fire Golems'. You can find them north-west of Defense Station 3. Oh, {name}, may I remind you, that our soldiers have been trained to obey the commands 'follow', 'retreat', 'behind', 'front' and 'back'?"));
                update.new_stage = Some(17);
            }
            18 => {
                self.fdemon_platoon_exp(boss_id, player_id, 3000, 4, area_id);
                update.new_stage = Some(19);
            }
            19 => {
                self.npc_say(boss_id, &format!("Scouts have found a room made by the ancients where a lot of small containers are stored. It is located in the north-western part of the underground. I want you to go there, aquire some of these containers and find out what they do. Good luck, {name}."));
                update.new_stage = Some(20);
                update.new_counter = Some(0);
            }
            21 => {
                self.fdemon_platoon_exp(boss_id, player_id, 4000, 5, area_id);
                update.new_stage = Some(22);
            }
            22 => {
                self.npc_say(boss_id, &format!("Now that we know what these containers can hold, we need to find out what to do with that golem blood, {name}. Your mission, {rank_name}, is to find a use for it."));
                update.new_stage = Some(23);
                update.new_counter = Some(0);
            }
            24 => {
                self.fdemon_platoon_exp(boss_id, player_id, 4000, 5, area_id);
                update.new_stage = Some(25);
            }
            25 => {
                self.npc_say(boss_id, &format!("So that's how we can pass those lava fields. Well, since you can cross them now, {name}, I want you to activate Defense Station 6. It is located north-east of number 5."));
                update.new_stage = Some(26);
                update.new_counter = Some(0);
            }
            27 => {
                self.fdemon_platoon_exp(boss_id, player_id, 4000, 5, area_id);
                update.new_stage = Some(28);
            }
            28 | 29 => {
                // C `case 28: ppd->boss_stage++; ppd->boss_counter = 0; //
                // fall thru intended` falls straight into `case 29`'s body
                // - both starting stages produce the exact same observable
                // message/stage-30 transition, so they're one match arm;
                // only the `boss_counter` reset is `case 28`-specific.
                self.npc_say(boss_id, &format!("I do not have a specific mission for you at the moment, {name}, but I want you to scout the whole underground and find all the Defense Stations. When you find one, activate it, and report back from time to time. You don't have to come back for every single new station you find, though."));
                update.new_stage = Some(30);
                if boss_stage == 28 {
                    update.new_counter = Some(0);
                }
            }
            30 => {
                let mut cnt = 0i32;
                let mut cnt2 = 0i32;
                for n in 0..32 {
                    let bit = 1i32 << n;
                    if boss_counter & bit != 0 {
                        if boss_reported & bit == 0 {
                            cnt += 1;
                        }
                        cnt2 += 1;
                    }
                }
                if cnt2 >= 26 {
                    update.new_stage = Some(31);
                }
                if cnt == 0 {
                    update.timer_touched = false;
                } else {
                    self.npc_say(boss_id, &format!("Ah, {name}. I hear you have found {cnt} new Defense Stations. So you've found {cnt2} stations now."));
                    self.fdemon_platoon_exp(boss_id, player_id, 2000 * cnt, 2 * cnt, area_id);
                    update.new_reported = Some(boss_counter);
                }
            }
            31 => {
                self.npc_say(
                    boss_id,
                    &format!(
                        "It seems we know all stations here now. Thou wert most helpful, {name}."
                    ),
                );
                update.new_stage = Some(32);
            }
            32 => {
                self.npc_say(boss_id, "I do not have any more missions for thee. But thou might want to explore the underworld further. This is but one part of it, and I feel that the cause of all this evil lies further to the north.");
                update.new_stage = Some(33);
            }
            _ => {
                // No C `case` past 32 - stage stuck, no message, no timer.
                update.timer_touched = false;
            }
        }

        update
    }

    /// C `platoon_exp(cn, cm, amount, pts, ppd)` (`fdemon.c:719-772`)'s
    /// always-live player-reward half (see module doc comment for the
    /// deferred soldier-exp loop).
    pub(crate) fn fdemon_platoon_exp(
        &mut self,
        boss_id: CharacterId,
        player_id: CharacterId,
        amount: i32,
        pts: i32,
        area_id: u32,
    ) {
        let Some(name) = self.characters.get(&player_id).map(|c| c.name.clone()) else {
            return;
        };
        self.npc_say(boss_id, &format!("Well done, {name}."));

        // C's soldier-exp loop (`ppd->soldier[n]`) is unreachable: no
        // `CDR_FDEMON_ARMY` recruits exist yet - see module doc comment.

        let Some(level) = self.characters.get(&player_id).map(|c| c.level) else {
            return;
        };
        let units = (amount + 1999) / 2000;
        let exp_cap = i64::from(level_value(level)) / 5;
        let per_unit_base = (i64::from(amount) * 4) / i64::from(units.max(1));
        let per_unit_exp = exp_cap.min(per_unit_base);
        self.give_exp(player_id, i64::from(units) * per_unit_exp, area_id);

        let Some(character) = self.characters.get_mut(&player_id) else {
            return;
        };
        let old_rank = army_rank_for_points(character.military_points);
        character.military_points = character.military_points.saturating_add(pts);
        let new_rank = army_rank_for_points(character.military_points);

        if new_rank < 24 && new_rank > old_rank {
            let rank_name = army_rank_name(new_rank);
            self.npc_say(
                boss_id,
                &format!("You've been promoted to {rank_name}. Congratulations, {name}!"),
            );
        }
    }

    /// C `fdemon_boss`'s `NT_TEXT` branch (`fdemon.c:1780-1863`, the
    /// "repeat" half only - see module doc comment for the deferred
    /// "take"/"drop" tail), wired through the generic `analyse_text_qa`
    /// matcher (same pattern as `world/trader.rs::trader_qa_reply`/
    /// `gatekeeper.rs::gate_welcome_handle_text_message`). Drains every
    /// queued `NT_TEXT` message for `boss_id` (real ones, delivered by
    /// `ugaris-server`'s player-speech fan-out - see module doc comment),
    /// replies to ordinary small talk directly, and returns the list of
    /// speakers whose message matched "repeat" (C's `analyse_text_driver`
    /// return code `8`) for the caller to apply [`fdemon_boss_repeat_reset`]
    /// to (needs that speaker's `PlayerRuntime`-resident `farmy_ppd`).
    pub fn fdemon_boss_process_text_messages(&mut self, boss_id: CharacterId) -> Vec<CharacterId> {
        let Some(boss_name) = self.characters.get(&boss_id).map(|c| c.name.clone()) else {
            return Vec::new();
        };
        let messages = self
            .characters
            .get_mut(&boss_id)
            .map(|boss| std::mem::take(&mut boss.driver_messages))
            .unwrap_or_default();

        let daylight = self.date.daylight;
        let mut repeat_requests = Vec::new();
        for message in messages {
            if message.message_type != NT_TEXT {
                continue;
            }
            let speaker_id = CharacterId(message.dat3 as u32);
            if speaker_id == boss_id {
                continue;
            }
            let Some(text) = message.text.as_deref() else {
                continue;
            };
            let (Some(boss), Some(speaker)) = (
                self.characters.get(&boss_id),
                self.characters.get(&speaker_id),
            ) else {
                continue;
            };
            // C `analyse_text_driver`'s own `char_dist(cn, co) > 12` /
            // `!char_see_char(cn, co)` early-outs (`fdemon.c:209-215`).
            if char_dist(boss, speaker) > FDEMON_BOSS_TALK_RANGE
                || !char_see_char(boss, speaker, &self.map, daylight)
            {
                continue;
            }
            let speaker_name = speaker.name.clone();

            match analyse_text_qa(text, &boss_name, &speaker_name, FDEMON_QA) {
                TextAnalysisOutcome::Said(reply) => {
                    self.npc_say(boss_id, &reply);
                }
                // C `case 1: say(cn, "I'm %s.", ch[cn].name); default:
                // return qa[q].answer_code;` (`fdemon.c:285-289`) - see
                // `super::FDEMON_QA`'s own doc comment.
                TextAnalysisOutcome::Matched(1) => {
                    self.npc_say(boss_id, &format!("I'm {boss_name}."));
                }
                TextAnalysisOutcome::Matched(8) => {
                    repeat_requests.push(speaker_id);
                }
                TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
            }
        }
        repeat_requests
    }
}

/// C `fdemon_boss`'s `NT_TEXT` "repeat" stage-reset ladder (`fdemon.c:
/// 1788-1861`), run once per player whose spoken text matched "repeat"
/// (see [`World::fdemon_boss_process_text_messages`]). Pure stage math
/// only - returns `Some((new_stage, new_timer))` when C's ladder actually
/// resets something, `None` for the "exp give-out"/no-op stages (`6`/`9`/
/// `12`/`15`/`18`/`21`/`24`/`27`/`28`) and any stage `>= 33`.
pub fn fdemon_boss_repeat_reset(boss_stage: i32) -> Option<(i32, i32)> {
    match boss_stage {
        0..=5 => Some((0, 0)),
        7 | 8 => Some((7, 0)),
        10 | 11 => Some((10, 0)),
        13 | 14 => Some((13, 0)),
        16 | 17 => Some((16, 0)),
        19 | 20 => Some((19, 0)),
        22 | 23 => Some((22, 0)),
        25 | 26 => Some((25, 0)),
        29 | 30 => Some((29, 0)),
        31 | 32 => Some((31, 0)),
        _ => None,
    }
}
