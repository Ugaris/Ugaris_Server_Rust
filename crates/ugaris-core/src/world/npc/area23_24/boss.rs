//! Ice Army Caves commander mission-giver (`CDR_STRATEGY_BOSS`).
//!
//! Ports C `src/area/23_24/strategy.c::strategy_boss` (`:1412-1607`): a
//! stationary NPC ("Cinciac") that greets every nearby player and steps
//! them through a 12-stage (`boss_stage` 0-11) introductory monologue
//! explaining the Areas 23/24 strategy minigame, unlocking the already-
//! ported `CDR_STRATEGY_PARSER` command table
//! (`crate::world::strategy_special::World::apply_strategy_special_command`,
//! gated on `boss_stage >= 9` by that module's own `strategy_needs_boss`)
//! for the first time - until this file, `StrategyPpd::boss_stage` could
//! never advance past its zero default, so every `#jp`/`#list`/`#info`/
//! `#raise`/`#mission`/`#enter`/`#surrender` command in that table was
//! unreachable through live gameplay.
//!
//! Deviations/gaps (documented, not silent):
//! - C reacts to a live `NT_CHAR` message (populated by `notify_area`'s
//!   sight broadcast) for the per-stage greeting. This port instead does a
//!   direct per-tick scan of nearby `CF_PLAYER` characters
//!   ([`World::strategy_boss_sighted_players`]) - the same class of
//!   "replace message-driven sighting with a direct scan" simplification
//!   already established by `world::npc::area8::fdemon_boss` and others.
//!   The `NT_TEXT` "repeat"/"military rank"/"levels and experience"
//!   detection, by contrast, *does* use the real
//!   `Character::driver_messages` queue
//!   ([`World::strategy_boss_process_text_messages`]), since nearby
//!   player speech is already reliably delivered there (`ugaris-server`'s
//!   `commands_chat.rs::push_driver_text_message` fan-out) - no
//!   replacement needed.
//! - C's `ch[cn].driver != CDR_LOSTCON` sighting-gate clause
//!   (`strategy.c:1432`) reads the *boss's own* driver field, not the
//!   sighted player's - since this function only ever runs for a real
//!   live `CDR_STRATEGY_BOSS` character, the clause is always true and is
//!   not reproduced (same "dead defensive clause" precedent documented
//!   elsewhere in this codebase, e.g. `two_guard.rs`'s own notes on
//!   similarly always-true C guards).
//! - `realtime`/`ppd->boss_timer` (wall-clock seconds) are carried here as
//!   game ticks instead (`World::tick`, scaled by `TICKS_PER_SECOND`) -
//!   same simplification precedent as `world::npc::area8::fdemon_boss`'s
//!   own `boss_timer` port (see `crate::area8`'s
//!   `FDEMON_BOSS_TIMER_THROTTLE_TICKS`), avoiding a real wall-clock
//!   dependency for a purely tick-driven engine.
//! - `NT_CREATE`'s `if (ch[cn].arg) ch[cn].arg = NULL;` has no Rust
//!   equivalent (no zone-file `arg` scratch field modeled for this
//!   driver, and it has no other observable effect) - omitted, matching
//!   every other simple dialogue NPC in this codebase (e.g. `astro1.rs`'s
//!   documented omission).
//! - C's trailing `regenerate_driver`/`spell_self_driver`/`turn`/`do_idle`
//!   calls are not ported: Cinciac is `CF_IMMORTAL`/`CF_NOATTACK` (zone
//!   data) and stationary, so none of that self-management has any
//!   observable effect worth a dedicated per-tick call here.

use crate::{
    character_driver::{CDR_STRATEGY_BOSS, NT_TEXT},
    player::StrategyPpd,
    tick::TICKS_PER_SECOND,
    world::*,
};

/// C `char_dist(cn, co) < 16` sighting range gate (`strategy.c:1431`),
/// reused for the `NT_TEXT` command gate (`strategy.c:1550`, the same
/// literal `16`).
const STRATEGY_BOSS_SIGHT_RANGE: i32 = 16;

/// Matches `world::text::notify_area`'s own broadcast radius - see
/// `world::npc::area8::fdemon_boss`'s identical constant for the same
/// rationale (a generous bounding box before the real `char_dist`/
/// `char_see_char` checks narrow it down).
const SIGHTING_SCAN_RADIUS: u16 = 32;

/// C `realtime - ppd->boss_timer > 5`'s throttle window (`strategy.c:
/// 1433`), in game ticks - see this module's doc comment.
pub const STRATEGY_BOSS_TIMER_THROTTLE_TICKS: i64 = (TICKS_PER_SECOND * 5) as i64;

/// C `strategy_boss`'s `NT_TEXT` command detection (`strategy.c:1552-
/// 1587`): which of the three `strcasestr` keyword matches fired for a
/// given message. All three are independent `if`s in C (not an `else if`
/// chain), so a single message could in principle match more than one -
/// [`World::strategy_boss_process_text_messages`] preserves that by
/// pushing one entry per match, not one per message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyBossTextCommand {
    /// `strategy.c:1552-1569`: resets `boss_stage`/`boss_timer` to `0` for
    /// any current stage `0..=11`.
    Repeat,
    /// `strategy.c:1571-1578`: the "military rank" reward choice.
    MilitaryRank,
    /// `strategy.c:1579-1587`: the "levels and experience" reward choice.
    LevelsAndExperience,
}

/// C `strategy_boss`'s case-2 monologue text (`strategy.c:1467-1471`),
/// shared by two call sites - see [`World::strategy_boss_greet_player`]'s
/// case-1 fallthrough handling.
fn strategy_boss_case2_text() -> &'static str {
    "We've discovered these caves a few weeks ago. Each cave seems to contain a network of depots, \
several castles and platinum mines. Some ancient magic is at work here, since each castle is able to \
create artificial creatures, which can be used as workers or fighters."
}

impl World {
    /// Every live `CDR_STRATEGY_BOSS` character (C `ch_driver`'s
    /// `CDR_STRATEGY_BOSS` case, `strategy.c:1614-1616`) - the caller
    /// (`ugaris-server`'s `tick_npc::area23_24`) drives the rest of the
    /// per-tick dispatch, since it alone has the `PlayerRuntime` access
    /// [`World::strategy_boss_greet_player`]'s `ppd` parameter needs.
    pub fn strategy_boss_character_ids(&self) -> Vec<CharacterId> {
        self.characters
            .values()
            .filter(|character| {
                character.driver == CDR_STRATEGY_BOSS
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect()
    }

    /// C `strategy_boss`'s per-tick `NT_CHAR` sighting loop (`strategy.c:
    /// 1428-1543`, replaced by a direct scan - see module doc comment).
    /// Returns every visible `CF_PLAYER` character within
    /// [`STRATEGY_BOSS_SIGHT_RANGE`] tiles; callers are responsible for
    /// the `realtime - boss_timer > 5` throttle (checked inside
    /// [`World::strategy_boss_greet_player`] itself, since that function
    /// already receives `ppd`).
    pub fn strategy_boss_sighted_players(&self, boss_id: CharacterId) -> Vec<CharacterId> {
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
                    && target.x >= min_x
                    && target.x <= max_x
                    && target.y >= min_y
                    && target.y <= max_y
                    && char_dist(boss, target) < STRATEGY_BOSS_SIGHT_RANGE
            })
            .filter(|target| char_see_char(boss, target, &self.map, daylight))
            .map(|target| target.id)
            .collect()
    }

    /// C `strategy_boss`'s `switch (ppd->boss_stage)` body (`strategy.c:
    /// 1443-1541`) for a single sighted player, including the throttle
    /// (`strategy.c:1433`) and lazy `init_done` reset (`:1434-1441`) C
    /// applies right before it. `boss_id` is Cinciac itself (C's `cn`,
    /// the `say()` speaker); `player_id` is the sighted player (C's
    /// `co`); `now_ticks` is `World::tick.0` as of this call.
    pub fn strategy_boss_greet_player(
        &mut self,
        boss_id: CharacterId,
        player_id: CharacterId,
        ppd: &mut StrategyPpd,
        now_ticks: i64,
    ) {
        if now_ticks.saturating_sub(i64::from(ppd.boss_timer)) <= STRATEGY_BOSS_TIMER_THROTTLE_TICKS
        {
            return;
        }

        if ppd.init_done == 0 {
            // C `bzero(ppd, sizeof(struct strategy_ppd));` - the whole
            // struct, not just the four fields set below.
            *ppd = StrategyPpd::default();
            ppd.max_level = 60;
            ppd.max_worker = 4;
            ppd.trainspeed = 1;
            ppd.eguardlvl = 50;
            ppd.init_done = 1;
        }

        let Some(player_name) = self.characters.get(&player_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(boss_name) = self.characters.get(&boss_id).map(|c| c.name.clone()) else {
            return;
        };
        let rank = army_rank_for_points(
            self.characters
                .get(&player_id)
                .map_or(0, |c| c.military_points),
        );
        let rank_name = army_rank_name(rank);

        let mut message: Option<String> = None;
        let mut timer_touched = true;

        match ppd.boss_stage {
            0 => {
                if rank < 8 {
                    message = Some(format!(
                        "Ah, {player_name}. The governer of Aston has some missions for you. \
You'd better head back there and do those first."
                    ));
                    ppd.boss_stage += 1;
                } else {
                    message = Some(format!(
                        "Welcome, {player_name}, to the Ice Army's Caves. I am {boss_name}, the \
commander in chief of the Ice Army Caves."
                    ));
                    ppd.boss_stage = 2;
                }
            }
            1 => {
                if rank >= 8 {
                    // C falls through from `case 1` straight into `case
                    // 2`'s body without an intervening `break` - see this
                    // module's doc comment.
                    message = Some(strategy_boss_case2_text().to_string());
                    ppd.boss_stage = 3;
                } else {
                    // C `break;` with no prior statement: no message, no
                    // `boss_timer` touch.
                    timer_touched = false;
                }
            }
            2 => {
                message = Some(strategy_boss_case2_text().to_string());
                ppd.boss_stage += 1;
            }
            3 => {
                message = Some(
                    "Islena's Lieutenants are trying to gain control over the castles and the \
platinum mines. If they succeed, they could spawn a vast army of these artificial creatures, and \
overrun our newly established defenses around Aston."
                        .to_string(),
                );
                ppd.boss_stage += 1;
            }
            4 => {
                message = Some(
                    "Unfortunately, we cannot spare any soldiers to fight the Lieutenants and \
their artificial armies. Therefore, we have to beat them using their own means."
                        .to_string(),
                );
                ppd.boss_stage += 1;
            }
            5 => {
                message = Some(format!(
                    "Your mission, {rank_name}, is to find out how to use the castles, the mines \
and the workers to raise an army of your own, and to defeat Islena's Lieutenants."
                ));
                ppd.boss_stage += 1;
            }
            6 => {
                message = Some(
                    "We have collected some information about the caves, and the Lieutenants you \
will encounter there. Type /mission to get a list of these caves, and the missions currently \
available."
                        .to_string(),
                );
                ppd.boss_stage += 1;
            }
            7 => {
                message = Some(
                    "You can use /enter <number> to start any of the missions listed. With \
/info, you'll be able to get some information about your understanding of the Castle's magic, \
and with /raise <number> you can choose to research one of the topics listed with /info."
                        .to_string(),
                );
                ppd.boss_stage += 1;
            }
            8 => {
                message = Some(
                    "You can also /surrender, /list, /jp and /eguard once you've started a \
mission. The artificial creatures obey specific spoken commands. So far, we discovered \
'transfer', 'mine', 'guard' and 'fight'. You will have to do some research of your own to \
utilize all the commands fully."
                        .to_string(),
                );
                ppd.boss_stage += 1;
            }
            9 => {
                message = Some(format!(
                    "Good luck, {player_name}. And report back from time to time."
                ));
                ppd.boss_stage += 1;
            }
            10 => {
                if ppd.boss_exp > 0 {
                    message = Some(format!(
                        "Ah, {player_name}. You made some progress defeating Islena's \
Lieutenants, and I have orders to reward you. Do you prefer {}military rank{} or \
{}levels and experience{}?",
                        crate::text::COL_STR_LIGHT_BLUE,
                        crate::text::COL_STR_RESET,
                        crate::text::COL_STR_LIGHT_BLUE,
                        crate::text::COL_STR_RESET,
                    ));
                    ppd.boss_stage += 1;
                    ppd.boss_msg_exp = ppd.boss_exp;
                }
            }
            11 => {
                if ppd.boss_exp > ppd.boss_msg_exp {
                    ppd.boss_stage = 10;
                }
            }
            _ => {
                timer_touched = false;
            }
        }

        if timer_touched {
            ppd.boss_timer = now_ticks as i32;
        }
        if let Some(text) = message {
            self.npc_say(boss_id, &text);
        }
    }

    /// C `strategy_boss`'s `NT_TEXT` branch (`strategy.c:1545-1589`).
    /// Drains every queued `NT_TEXT` message for `boss_id` (real ones,
    /// delivered by `ugaris-server`'s player-speech fan-out - see module
    /// doc comment) and returns every `(speaker, command)` match, for the
    /// caller (`ugaris-server`'s `area23_24.rs`, the only layer with
    /// `PlayerRuntime` access to the matched speaker's `StrategyPpd`) to
    /// apply via [`World::strategy_boss_apply_text_command`].
    pub fn strategy_boss_process_text_messages(
        &mut self,
        boss_id: CharacterId,
    ) -> Vec<(CharacterId, StrategyBossTextCommand)> {
        let mut outcome = Vec::new();
        let messages = self
            .characters
            .get_mut(&boss_id)
            .map(|boss| std::mem::take(&mut boss.driver_messages))
            .unwrap_or_default();

        let daylight = self.date.daylight;
        for message in messages {
            if message.message_type != NT_TEXT {
                continue;
            }
            let speaker_id = CharacterId(message.dat3 as u32);
            // C `if ((co = msg->dat3) == cn) { remove_message(...);
            // continue; }` (`strategy.c:1546-1549`).
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
            if !speaker.flags.contains(CharacterFlags::PLAYER)
                || char_dist(boss, speaker) >= STRATEGY_BOSS_SIGHT_RANGE
                || !char_see_char(boss, speaker, &self.map, daylight)
            {
                continue;
            }

            let lower = text.to_ascii_lowercase();
            // C: three independent `if (strcasestr(...))` checks, not an
            // `else if` chain - a message can match more than one.
            if lower.contains("repeat") {
                outcome.push((speaker_id, StrategyBossTextCommand::Repeat));
            }
            if lower.contains("military rank") {
                outcome.push((speaker_id, StrategyBossTextCommand::MilitaryRank));
            }
            if lower.contains("levels and experience") {
                outcome.push((speaker_id, StrategyBossTextCommand::LevelsAndExperience));
            }
        }
        outcome
    }

    /// Applies one [`StrategyBossTextCommand`] match from
    /// [`World::strategy_boss_process_text_messages`] (`strategy.c:1552-
    /// 1587`). `area_id` feeds the `give_military_pts_from_npc` calls'
    /// exp-gain bookkeeping, same as every other military-points award
    /// site.
    pub fn strategy_boss_apply_text_command(
        &mut self,
        boss_id: CharacterId,
        player_id: CharacterId,
        ppd: &mut StrategyPpd,
        command: StrategyBossTextCommand,
        area_id: u32,
    ) {
        match command {
            StrategyBossTextCommand::Repeat => {
                // C `switch (ppd->boss_stage) { case 0: ... case 11:
                // ppd->boss_stage = 0; ppd->boss_timer = 0; break; }`
                // (`strategy.c:1553-1569`) - every listed case does the
                // same thing, so this collapses to one range check.
                if (0..=11).contains(&ppd.boss_stage) {
                    ppd.boss_stage = 0;
                    ppd.boss_timer = 0;
                }
            }
            StrategyBossTextCommand::MilitaryRank => {
                if ppd.boss_exp > 0 {
                    self.npc_say(boss_id, "So be it.");
                    self.give_military_pts_from_npc(
                        player_id,
                        boss_id,
                        ppd.boss_exp,
                        ppd.boss_exp,
                        area_id,
                    );
                    ppd.boss_exp = 0;
                    ppd.boss_msg_exp = 0;
                    ppd.boss_stage = 10;
                }
            }
            StrategyBossTextCommand::LevelsAndExperience => {
                if ppd.boss_exp > 0 {
                    self.npc_say(boss_id, "So be it.");
                    let rank = army_rank_for_points(
                        self.characters
                            .get(&player_id)
                            .map_or(0, |c| c.military_points),
                    );
                    let pts = ppd.boss_exp / 5 + 1;
                    let exps =
                        (f64::from(ppd.boss_exp) * f64::from(rank + 5).powi(4) / 24.0) as i32;
                    self.give_military_pts_from_npc(player_id, boss_id, pts, exps, area_id);
                    ppd.boss_exp = 0;
                    ppd.boss_msg_exp = 0;
                    ppd.boss_stage = 10;
                }
            }
        }
    }
}
