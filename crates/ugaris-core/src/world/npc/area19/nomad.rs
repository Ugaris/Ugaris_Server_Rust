//! The Nomad Plains tribe NPC (`CDR_NOMAD`), `src/area/19/nomad.c::nomad`
//! (`:927-1155`) plus its six persona sub-drivers `nomad_1`..`nomad_6`
//! (`:302-516`), their `nomad_N_repeat` resets (`:518-600`), the dice/
//! statue purchase helpers `nomad_2_text`/`nomad_6_text` (`:602-679`), the
//! quest-item turn-ins `nomad_1_give`/`nomad_4_give`/`nomad_5_give`
//! (`:681-789`), and the `Llakal Sla` dice-betting minigame `nomad_bet`/
//! `nomad_roll` (`:791-925`).
//!
//! All six personas (Kalanur the tribe recruiter/quest 32, Irakar the dice
//! seller, the game host, the two Kir monastery monks - Sarkilar's-fate
//! quest 33 and the life-teacher quest 34 - and the golden-statue seller)
//! share this one driver, differentiated at spawn time only by their own
//! `arg="nr=N;diceskill=N;minbet=N;maxbet=N;maxloss=N;"` zone-file line
//! (`world::npc::area19::nomad::parse_nomad_driver_args`, wired from
//! `crate::zone`'s `CDR_NOMAD` spawn-time branch, same "parse args at spawn
//! instead of `NT_CREATE`" precedent as `CDR_TWOSERVANT`/`CDR_TWOGUARD`).
//!
//! Split across five files to stay under the ~800-line NPC-file guideline
//! (same precedent as `world::npc::area17::guard`/`guard_messages`):
//! this file (driver data, spawn-time parsing, the `NT_CHAR` dispatch,
//! and the shared `set_nomad_state_event`/`nomad_repeat_state` helpers),
//! [`super::nomad_dialogue`] (the six `nomad_1`..`nomad_6` greeting
//! ladders themselves), [`super::nomad_text`] (`NT_TEXT`: the shared qa
//! table, `repeat`, dice/statue purchases, and the `"bet "` command
//! trigger), [`super::nomad_give`] (`NT_GIVE`: quest-item turn-ins), and
//! [`super::nomad_bet`] (the dice-betting minigame plus the `count_salt`/
//! `remove_salt` salt-currency helpers every file here needs).
//!
//! `World` cannot see `crate::player::PlayerRuntime` (where `nomad_ppd`
//! lives), so every tick pass takes a `player_facts` snapshot
//! ([`NomadPlayerFacts`]) and returns [`NomadOutcomeEvent`]s for
//! `ugaris-server` to apply - the same split every other quest-giver NPC in
//! this codebase uses. Some outcomes additionally need `ZoneLoader` (salt/
//! dice/statue item creation), which `World` also cannot see; those are
//! applied by `ugaris-server::area19::apply_nomad_events` alongside the
//! `PlayerRuntime` writes.

use std::collections::HashMap;

use crate::character_driver::{next_legacy_name_value, CDR_NOMAD};
use crate::drvlib::offset2dx;
use crate::world::*;

/// C `char_dist(cn, co) < 12` (`nomad.c:953`): `NT_CHAR` greeting range.
pub(super) const NOMAD_GREET_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`nomad.c:953`): greeting-repeat cooldown.
pub(super) const NOMAD_TALK_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`nomad.c:1149`): idle "return to post" threshold.
const NOMAD_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 10;

/// C `struct nomad_data` (`nomad.c:215-225`). `nr`/`dice_skill`/
/// `min_bet`/`max_bet`/`max_loss` are the zone-file-configured persona
/// identity/dice-game tuning (parsed once at spawn, see
/// [`parse_nomad_driver_args`]); `last_talk_tick`/`play_with`/
/// `play_timer`/`bet`/`my_throw` are this NPC instance's own mutable
/// runtime state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NomadDriverData {
    pub nr: i32,
    pub dice_skill: i32,
    pub min_bet: i32,
    pub max_bet: i32,
    pub max_loss: i32,
    pub last_talk_tick: u64,
    /// C `dat->play_with`: the player currently mid-game with this NPC, if
    /// any (`0` in C).
    pub play_with: Option<CharacterId>,
    pub play_timer: u64,
    pub bet: i32,
    pub my_throw: i32,
}

/// C `nomad_parse` (`nomad.c:282-300`): reads the zone-file `arg=` string
/// at spawn time (see the module doc comment for why this happens here
/// instead of on the first `NT_CREATE` message, matching
/// `parse_two_servant_driver_args`'s precedent). Unknown keys are silently
/// ignored (C only `elog`s them).
pub fn parse_nomad_driver_args(args: &str) -> NomadDriverData {
    let mut data = NomadDriverData::default();
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "nr" => data.nr = parsed,
            "diceskill" => data.dice_skill = parsed,
            "minbet" => data.min_bet = parsed,
            "maxbet" => data.max_bet = parsed,
            "maxloss" => data.max_loss = parsed,
            _ => {} // C: `elog(...)` - log-only.
        }
        rest = next;
    }
    data
}

/// Per-player facts [`World::process_nomad_actions`] needs from
/// `crate::player::PlayerRuntime`'s `nomad_ppd`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NomadPlayerFacts {
    /// `PlayerRuntime::nomad_state(nr)` for every `nr` (indexed directly,
    /// C `ppd->nomad_state[MAXNOMAD]`).
    pub nomad_state: [i32; 10],
    /// `PlayerRuntime::nomad_win(nr)` for every `nr`.
    pub nomad_win: [i32; 10],
    /// `PlayerRuntime::nomad_tribe_member()`.
    pub tribe_member: i32,
    /// `PlayerRuntime::nomad_open_bet()`.
    pub open_bet: i32,
    /// `PlayerRuntime::nomad_open_roll()`.
    pub open_roll: (i32, i32, i32),
}

/// A side effect [`World::process_nomad_actions`] could not apply directly
/// because it touches `PlayerRuntime` and/or `ZoneLoader`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NomadOutcomeEvent {
    /// C `ppd->nomad_state[nr]++`/`=N` (every `nomad_N`/`nomad_N_repeat`).
    UpdateNomadState {
        player_id: CharacterId,
        nr: usize,
        new_state: i32,
    },
    /// C `questlog_open(co, 32/33/34)`.
    QuestOpen {
        player_id: CharacterId,
        quest_id: i32,
    },
    /// C `questlog_done(co, 32/33/34)`.
    QuestDone {
        player_id: CharacterId,
        quest_id: i32,
    },
    /// C `ppd->tribe_member |= TM_TRIBE1` (`nomad_1_give`, `nomad.c:705`).
    SetTribeMember { player_id: CharacterId, flag: i32 },
    /// C `ppd->nomad_win[nr] += / -= dat->bet` (`nomad_roll`).
    AdjustNomadWin {
        player_id: CharacterId,
        nr: usize,
        delta: i32,
    },
    /// C `ppd->open_bet`/`open_roll1/2/3` writes (`nomad_bet`/`nomad_roll`).
    SetOpenBet {
        player_id: CharacterId,
        bet: i32,
        roll1: i32,
        roll2: i32,
        roll3: i32,
    },
    /// C `give_exp(co, diff/10 or diff/2)` (`nomad_5_give`).
    GiveExp {
        player_id: CharacterId,
        base_exp: i64,
    },
    /// C `nomad_1_give`'s wolf/white-wolf-skin trade-in (`nomad.c:710-
    /// 734`): destroys `skin_item_id` and hands the player a freshly
    /// created "salt" stack worth `amount` ounces.
    GiveSaltForSkin {
        nomad_id: CharacterId,
        player_id: CharacterId,
        skin_item_id: ItemId,
        amount: u32,
    },
    /// C `nomad_2_text`/`nomad_6_text`'s dice/golden-statue purchase
    /// (`nomad.c:602-679`): creates `template`, hands it to the player,
    /// and (only on success) removes `cost` ounces of salt.
    BuyItemWithSalt {
        nomad_id: CharacterId,
        player_id: CharacterId,
        template: &'static str,
        cost: i32,
    },
    /// C `nomad_roll`'s loss branch (`nomad.c:904-922`): creates a fresh
    /// "salt" stack worth `amount` ounces for the player; `nr` is needed
    /// to also apply `AdjustNomadWin` on success (C's own `ppd->nomad_win
    /// [nr] -= dat->bet` sits inside the same `if (give_char_item(...))`
    /// branch).
    PaySaltWinnings {
        nomad_id: CharacterId,
        player_id: CharacterId,
        amount: i32,
        nr: usize,
    },
}

impl World {
    /// C `nomad`'s per-tick body (`nomad.c:927-1155`).
    pub fn process_nomad_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, NomadPlayerFacts>,
        area_id: u16,
    ) -> Vec<NomadOutcomeEvent> {
        let nomad_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_NOMAD
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for nomad_id in nomad_ids {
            self.process_nomad_tick(nomad_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_nomad_tick(
        &mut self,
        nomad_id: CharacterId,
        player_facts: &HashMap<CharacterId, NomadPlayerFacts>,
        area_id: u16,
        events: &mut Vec<NomadOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Nomad(mut data)) = self
            .characters
            .get(&nomad_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&nomad_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        let mut talk_dir_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.nomad_handle_char_message(
                    nomad_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut talk_dir_target,
                ),
                NT_TEXT => self.nomad_handle_text_message(
                    nomad_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                ),
                NT_GIVE => {
                    self.nomad_handle_give_message(nomad_id, &data, message, player_facts, events)
                }
                NT_NPC => self.nomad_handle_npc_message(
                    nomad_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                ),
                _ => {}
            }
        }

        if let Some(nomad) = self.characters.get_mut(&nomad_id) {
            nomad.driver_state = Some(CharacterDriverState::Nomad(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`nomad.c:1146-1148`).
        if let (Some(nomad), Some((tx, ty))) =
            (self.characters.get(&nomad_id).cloned(), talk_dir_target)
        {
            if let Some(direction) = offset2dx(i32::from(nomad.x), i32::from(nomad.y), tx, ty) {
                if let Some(nomad_mut) = self.characters.get_mut(&nomad_id) {
                    let _ = turn(nomad_mut, direction as u8);
                }
            }
        }

        // C `if (spell_self_driver(cn)) return; if (regenerate_driver(cn))
        // return;` (`nomad.c:1139-1144`) - these two checks happen
        // *before* the `talkdir`/return-to-post tail in C, but neither has
        // an observable ordering effect relative to `turn`/`secure_move_
        // driver` in this message-driven architecture (matching every
        // other NPC file's precedent of running the idle-driver fallbacks
        // after the message loop).
        if self.spell_self_simple_baddy(nomad_id) {
            return;
        }
        if self.regenerate_simple_baddy(nomad_id) {
            return;
        }

        let data = match self
            .characters
            .get(&nomad_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::Nomad(data)) => *data,
            _ => return,
        };

        // C `if (ticker - dat->lasttalk > TICKS*10 && secure_move_driver
        // (cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret, lastact)) return;`
        // (`nomad.c:1149-1152`). `tmpx`/`tmpy` reuse `rest_x`/`rest_y`,
        // same substitution every other stationary NPC in this codebase
        // makes.
        if self.tick.0.saturating_sub(data.last_talk_tick) > NOMAD_RETURN_TO_POST_TICKS {
            let (post_x, post_y) = self
                .characters
                .get(&nomad_id)
                .map(|nomad| (nomad.rest_x, nomad.rest_y))
                .unwrap_or_default();
            let _ = self.secure_move_driver(
                nomad_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
        // C `do_idle(cn, TICKS);` (`nomad.c:1154`) - not modeled, same
        // precedent as every other stationary dialogue-only NPC in this
        // codebase: it has no observable effect in this message-driven
        // architecture.
    }

    /// C `nomad`'s `NT_CHAR` branch (`nomad.c:951-984`): dispatches to the
    /// speaking persona's own `nomad_N` sub-driver based on `dat->nr`.
    #[allow(clippy::too_many_arguments)]
    fn nomad_handle_char_message(
        &mut self,
        nomad_id: CharacterId,
        data: &mut NomadDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, NomadPlayerFacts>,
        events: &mut Vec<NomadOutcomeEvent>,
        talk_dir_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(nomad) = self.characters.get(&nomad_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if ((ch[co].flags & CF_PLAYER) && ticker - dat->lasttalk >
        // TICKS*5 && char_dist(cn, co) < 12 && char_see_char(cn, co) &&
        // (ppd = set_data(...)))` (`nomad.c:953-954`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let tick = self.tick.0;
        if tick.saturating_sub(data.last_talk_tick) <= NOMAD_TALK_COOLDOWN_TICKS {
            return;
        }
        if char_dist(&nomad, &player) >= NOMAD_GREET_DISTANCE {
            return;
        }
        if !char_see_char(&nomad, &player, &self.map, self.date.daylight) {
            return;
        }
        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        if data.nr < 1 || data.nr > 6 {
            return;
        }
        let nr = data.nr as usize;

        let didsay = match data.nr {
            1 => self.nomad_1(
                nomad_id,
                player_id,
                &player.name,
                &nomad.name,
                facts,
                nr,
                events,
            ),
            2 => self.nomad_2(
                nomad_id,
                player_id,
                &player.name,
                &nomad.name,
                facts,
                nr,
                events,
            ),
            3 => self.nomad_3(
                nomad_id,
                player_id,
                &player.name,
                &nomad.name,
                facts,
                nr,
                events,
            ),
            4 => self.nomad_4(nomad_id, player_id, &player, &nomad.name, facts, nr, events),
            5 => self.nomad_5(
                nomad_id,
                player_id,
                &player.name,
                &nomad.name,
                facts,
                nr,
                events,
            ),
            6 => self.nomad_6(
                nomad_id,
                player_id,
                &player.name,
                &nomad.name,
                facts,
                nr,
                events,
            ),
            _ => false,
        };

        // C `dat->lasttalk = ticker; if (didsay) { talkdir = ...; }`
        // (`nomad.c:979-983`) - `lasttalk` updates unconditionally once
        // the gate above passes, even on a silent (`didsay == 0`) result.
        data.last_talk_tick = tick;
        if didsay {
            *talk_dir_target = Some((i32::from(player.x), i32::from(player.y)));
        }
    }

    pub(super) fn set_nomad_state_event(
        &self,
        events: &mut Vec<NomadOutcomeEvent>,
        player_id: CharacterId,
        nr: usize,
        new_state: i32,
    ) {
        events.push(NomadOutcomeEvent::UpdateNomadState {
            player_id,
            nr,
            new_state,
        });
    }

    /// C `nomad_1_repeat`..`nomad_6_repeat` (`nomad.c:518-600`), dispatched
    /// from [`super::nomad_text`]'s `NT_TEXT` handler on a "repeat" match.
    pub(super) fn nomad_repeat_state(nr_driver: i32, current_state: i32) -> Option<i32> {
        match nr_driver {
            1 => {
                if (0..=8).contains(&current_state) {
                    Some(0)
                } else if (9..=10).contains(&current_state) {
                    Some(9)
                } else {
                    None
                }
            }
            2 | 3 => {
                if (0..=2).contains(&current_state) {
                    Some(0)
                } else {
                    None
                }
            }
            4 | 5 => {
                if (0..=3).contains(&current_state) {
                    Some(0)
                } else if (4..=5).contains(&current_state) {
                    Some(4)
                } else {
                    None
                }
            }
            6 => {
                if (0..=3).contains(&current_state) {
                    Some(0)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
