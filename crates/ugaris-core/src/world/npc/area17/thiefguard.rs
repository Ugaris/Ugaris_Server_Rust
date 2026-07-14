//! Two-City thieves-guild entrance guard (`CDR_TWOTHIEFGUARD`), the sewer
//! gatekeeper standing between Exkordon and the still-unported
//! `CDR_TWOTHIEFMASTER` guild master.
//!
//! Ports `src/area/17/two.c::thiefguard` (`:1541-1727`); C's
//! `ch_died_driver`/`ch_respawn_driver` dispatch for `CDR_TWOTHIEFGUARD`
//! are plain `return 1;` no-ops (same as `CDR_TWOSANWYN`/
//! `CDR_TWOALCHEMIST`), so no death/respawn hook exists for this NPC.
//!
//! Unlike every other Two-City NPC ported so far, this driver adds
//! visiting players as combat enemies of its own accord (`fight_driver_
//! add_enemy(cn, co, 1, 1)`, unconditional on talk cooldowns) whenever a
//! player with `thief_state < 3` (hasn't yet been welcomed as a guild
//! member) is spotted south of `y = 27` (i.e. actually inside the guild
//! sewers, not just at the entrance) - reproduced via the already-ported
//! driver-independent `Character::fight_driver` slot (`add_simple_baddy_
//! enemy_unchecked`, same machinery `world::npc::area8::fdemon_army_
//! combat`/`world::npc_messages`'s `aggressive=1` callers already use).
//!
//! A second, real C quirk: C's own per-tick tail calls `fight_driver_
//! attack_visible(cn, 0)` (full movement allowed for the attack task
//! itself) but *never* calls `fight_driver_follow_invisible` at all
//! (`two.c:1721-1723`, unlike `simple_baddy_driver`/`lostcon_driver`/
//! `guard_driver`) - ported via [`World::fight_driver_attack_visible_and_
//! follow`]'s new `may_follow_invisible: false` parameter (see that
//! function's own doc comment for the shared precedent, also needed by
//! `CDR_FDEMON_ARMY`'s combat fallback, which turned out to share the
//! exact same omission once this was investigated).
//!
//! The `thief_state == 50`/`51` greeting-ladder branches ("I have heard
//! of you. Thou art the one who killed the old guild master.") are ported
//! digit-for-digit but are dead code in practice until the still-unported
//! `CDR_TWOTHIEFMASTER`'s `thiefmaster_dead` death hook (`two.c:2207`,
//! `ppd->thief_state = 50`) exists to ever set that state - a documented
//! gap, not a silent one.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_TWOTHIEFGUARD};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::TWOCITY_QA;

/// C `char_dist(cn, co) > 10` (`two.c:1596`).
const TWO_THIEFGUARD_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`two.c:1579`).
const TWO_THIEFGUARD_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`two.c:1584`, `:1637`).
const TWO_THIEFGUARD_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`two.c:1718`): idle "return to post" threshold.
const TWO_THIEFGUARD_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ch[co].y < 27` (`two.c:1574`): only players actually inside the
/// guild sewers (not just at the entrance) provoke the guard.
const TWO_THIEFGUARD_HOSTILE_Y_BOUNDARY: u16 = 27;
/// C `ppd->thief_state < 3` (`two.c:1574`).
const TWO_THIEFGUARD_HOSTILE_STATE_CEILING: i32 = 3;
/// C `take_money(co, 10000)` (`two.c:1682`).
const TWO_THIEFGUARD_FEE_GOLD: u32 = 10000;

/// C `struct thiefguard_data` (`two.c:1541-1544`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoThiefGuardDriverData {
    pub last_talk_tick: u64,
    pub current_victim: Option<CharacterId>,
}

/// Per-player facts [`World::process_two_thiefguard_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoThiefGuardPlayerFacts {
    /// `PlayerRuntime::twocity_thief_state()`.
    pub thief_state: i32,
}

/// A side effect [`World::process_two_thiefguard_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoThiefGuardOutcomeEvent {
    /// Write the new `twocity_ppd.thief_state` back.
    UpdateThiefState {
        player_id: CharacterId,
        new_state: i32,
    },
}

impl World {
    /// C `thiefguard`'s per-tick body (`two.c:1546-1727`).
    pub fn process_two_thiefguard_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TwoThiefGuardPlayerFacts>,
        area_id: u16,
    ) -> Vec<TwoThiefGuardOutcomeEvent> {
        let thiefguard_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TWOTHIEFGUARD
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for thiefguard_id in thiefguard_ids {
            self.process_two_thiefguard_tick(thiefguard_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_two_thiefguard_tick(
        &mut self,
        thiefguard_id: CharacterId,
        player_facts: &HashMap<CharacterId, TwoThiefGuardPlayerFacts>,
        area_id: u16,
        events: &mut Vec<TwoThiefGuardOutcomeEvent>,
    ) {
        let Some(thiefguard_name) = self.characters.get(&thiefguard_id).map(|c| c.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::TwoThiefGuard(mut data)) = self
            .characters
            .get(&thiefguard_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&thiefguard_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.two_thiefguard_handle_char_message(
                    thiefguard_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.two_thiefguard_handle_text_message(
                    thiefguard_id,
                    &thiefguard_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.two_thiefguard_handle_give_message(thiefguard_id),
                _ => {}
            }
        }

        if let Some(thiefguard) = self.characters.get_mut(&thiefguard_id) {
            thiefguard.driver_state = Some(CharacterDriverState::TwoThiefGuard(data));
        }

        // C `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
        // 0)) return;` (`two.c:1719-1723`). No `fight_driver_follow_
        // invisible` call exists in C here - see the module doc comment.
        if let Some(thiefguard) = self.characters.get(&thiefguard_id).cloned() {
            let mut seed = self.legacy_random_seed;
            let attacked = self.fight_driver_attack_visible_and_follow(
                thiefguard_id,
                &thiefguard,
                area_id,
                FightDriverSuppressions::default(),
                false,
                &mut |below| legacy_random_below_from_seed(&mut seed, below),
            );
            self.legacy_random_seed = seed;
            if attacked {
                return;
            }
        }
        // C `if (spell_self_driver(cn)) return;` (`two.c:1724-1726`).
        if self.spell_self_simple_baddy(thiefguard_id) {
            return;
        }

        // C `if (talkdir) turn(cn, talkdir);` (`two.c:1728-1730`).
        if let (Some(thiefguard), Some((tx, ty))) =
            (self.characters.get(&thiefguard_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(thiefguard.x), i32::from(thiefguard.y), tx, ty)
            {
                if let Some(thiefguard_mut) = self.characters.get_mut(&thiefguard_id) {
                    let _ = turn(thiefguard_mut, direction as u8);
                }
            }
        }

        let data = match self
            .characters
            .get(&thiefguard_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::TwoThiefGuard(data)) => *data,
            _ => return,
        };

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (secure_move_
        // driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret, lastact))
        // return; }` (`two.c:1732-1736`). `tmpx`/`tmpy` reuse `rest_x`/
        // `rest_y`, same substitution every other stationary NPC in this
        // codebase makes.
        if data.last_talk_tick + TWO_THIEFGUARD_RETURN_TO_POST_TICKS < self.tick.0 {
            let (post_x, post_y) = self
                .characters
                .get(&thiefguard_id)
                .map(|thiefguard| (thiefguard.rest_x, thiefguard.rest_y))
                .unwrap_or_default();
            if self.secure_move_driver(
                thiefguard_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            ) {
                // Mirrors the C early return; nothing follows, but keep the
                // ported control flow explicit.
                #[allow(clippy::needless_return)]
                return;
            }
        }
        // C `do_idle(cn, TICKS);` (`two.c:1738`) - not modeled, same
        // precedent as every other stationary dialogue-only NPC in this
        // codebase: it has no observable effect in this message-driven
        // architecture.
    }

    /// C `thiefguard`'s `NT_CHAR` branch (`two.c:1563-1656`).
    #[allow(clippy::too_many_arguments)]
    fn two_thiefguard_handle_char_message(
        &mut self,
        thiefguard_id: CharacterId,
        data: &mut TwoThiefGuardDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoThiefGuardPlayerFacts>,
        events: &mut Vec<TwoThiefGuardOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(thiefguard) = self.characters.get(&thiefguard_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`two.c:1566-1570`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        let facts = player_facts.get(&player_id).copied();

        // C `if (ppd && ppd->thief_state < 3 && ch[co].y < 27 &&
        // char_see_char(cn, co)) { fight_driver_add_enemy(cn, co, 1, 1); }`
        // (`two.c:1572-1575`) - unconditional on the talk-cooldown/victim
        // checks below, since it happens before any of them can `continue`.
        if let Some(facts) = facts {
            if facts.thief_state < TWO_THIEFGUARD_HOSTILE_STATE_CEILING
                && player.y < TWO_THIEFGUARD_HOSTILE_Y_BOUNDARY
                && char_see_char(&thiefguard, &player, &self.map, self.date.daylight)
            {
                let tick = self.tick.0 as i32;
                if let Some(thiefguard_mut) = self.characters.get_mut(&thiefguard_id) {
                    let _ = add_simple_baddy_enemy_unchecked(thiefguard_mut, player_id, 1, tick);
                }
            }
        }

        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`two.c:1577-1581`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`two.c:1583-1587`).
        if tick < data.last_talk_tick + TWO_THIEFGUARD_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`two.c:1589-1592`) - unlike `sanwyn`/
        // `two_skelly`, this check does *not* additionally require
        // `dat->current_victim` to be nonzero first.
        if tick < data.last_talk_tick + TWO_THIEFGUARD_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`two.c:1594-1598`).
        if thiefguard_id == player_id
            || !char_see_char(&thiefguard, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`two.c:1600-1604`).
        if char_dist(&thiefguard, &player) > TWO_THIEFGUARD_GREET_DISTANCE {
            return;
        }

        let Some(facts) = facts else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->thief_state) { ... }` (`two.c:1607-1645`).
        match facts.thief_state {
            0 => {
                self.npc_say(
                    thiefguard_id,
                    &format!(
                        "HALT! Who's there? Ah, a stranger in our wonderful town. Hello, {}, and welcome to the thieves guild. Thou mayest not enter...",
                        player.name
                    ),
                );
                events.push(TwoThiefGuardOutcomeEvent::UpdateThiefState {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
            }
            1 => {
                self.npc_say(
                    thiefguard_id,
                    "...unless thou wert to become a member. If this is thy wish, thou wilt have to pay a fee of 100G.",
                );
                events.push(TwoThiefGuardOutcomeEvent::UpdateThiefState {
                    player_id,
                    new_state: 2,
                });
                didsay = true;
            }
            // `thief_state == 2`: waiting for the player to pay the fee
            // (`two.c:1618-1620`).
            2 => {}
            3 => {
                self.npc_say(
                    thiefguard_id,
                    &format!(
                        "Thou might want to talk to the guild master now, {}. He's in the room behind me.",
                        player.name
                    ),
                );
                events.push(TwoThiefGuardOutcomeEvent::UpdateThiefState {
                    player_id,
                    new_state: 4,
                });
                didsay = true;
            }
            // `thief_state == 4`: done (`two.c:1626-1628`).
            4 => {}
            // `thief_state == 50`/`51`: reachable only once the still-
            // unported `CDR_TWOTHIEFMASTER`'s `thiefmaster_dead` sets
            // `thief_state = 50` - see the module doc comment.
            50 => {
                self.npc_say(
                    thiefguard_id,
                    &format!(
                        "Ah, {}. I have heard of you. Thou art the one who killed the old guild master.",
                        player.name
                    ),
                );
                events.push(TwoThiefGuardOutcomeEvent::UpdateThiefState {
                    player_id,
                    new_state: 51,
                });
                didsay = true;
            }
            51 => {
                self.npc_say(
                    thiefguard_id,
                    "Well, thou hast done us and our new master a favor with that. Not that we'd pay thee anything for it, but we won't hold any grudges either.",
                );
                events.push(TwoThiefGuardOutcomeEvent::UpdateThiefState {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
            }
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`two.c:1647-1651`).
        if didsay {
            data.last_talk_tick = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `thiefguard`'s `NT_TEXT` branch (`two.c:1659-1701`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::npc::area17::sanwyn`/`two_skelly`/`alchemist`'s text
    /// handlers).
    #[allow(clippy::too_many_arguments)]
    fn two_thiefguard_handle_text_message(
        &mut self,
        thiefguard_id: CharacterId,
        thiefguard_name: &str,
        data: &mut TwoThiefGuardDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoThiefGuardPlayerFacts>,
        events: &mut Vec<TwoThiefGuardOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`two.c:1662-1664`).
        let tick = self.tick.0;
        if tick > data.last_talk_tick + TWO_THIEFGUARD_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
        {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`two.c:1666-1669`).
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
        if thiefguard_id == speaker_id
            || !speaker
                .flags
                .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return;
        }
        let Some(thiefguard) = self.characters.get(&thiefguard_id).cloned() else {
            return;
        };
        if !char_see_char(&thiefguard, &speaker, &self.map, self.date.daylight) {
            return;
        }

        let thief_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.thief_state)
            .unwrap_or(0);

        let mut didsay = false;
        // C's `analyse_text_driver` calls `say(cn, qa[q].answer, ...)`
        // directly (`two.c:206`), same as its Two-City siblings.
        match analyse_text_qa(text, thiefguard_name, &speaker.name, TWOCITY_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(thiefguard_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat) (`two.c:1670-1676`): only resets
            // `thief_state` to `0` while it is `<= 2` - unlike `sanwyn`'s
            // "repeat" this has no other branch for higher states, it's
            // simply a no-op there (still `didsay = 1` regardless, since
            // `analyse_text_driver` returns nonzero for *any* matched
            // qa row - matched-but-inert, same C semantics as `two_
            // barkeeper`'s dead-code `case 2` inner branch).
            TextAnalysisOutcome::Matched(2) => {
                if thief_state <= 2 {
                    data.last_talk_tick = 0;
                    events.push(TwoThiefGuardOutcomeEvent::UpdateThiefState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                didsay = true;
            }
            // C `case 15:` ("pay a fee") (`two.c:1678-1692`).
            TextAnalysisOutcome::Matched(15) => {
                if thief_state == 2 {
                    if self.two_thiefguard_take_money(speaker_id, TWO_THIEFGUARD_FEE_GOLD) {
                        self.npc_say(
                            thiefguard_id,
                            &format!(
                                "I welcome thee, {}, as a member of the thieves guild. Thou shalt be as dear to me as my brother - whom I killed when he was cheating me with the winnings of our enterprise.",
                                speaker.name
                            ),
                        );
                        events.push(TwoThiefGuardOutcomeEvent::UpdateThiefState {
                            player_id: speaker_id,
                            new_state: 3,
                        });
                    } else {
                        self.npc_say(thiefguard_id, "Thou dost not have enough money.");
                    }
                } else {
                    self.npc_say(thiefguard_id, "Hu?");
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`two.c:1699-1701`) - note this does *not* touch `dat->
        // last_talk` (except the explicit reset inside the "repeat"
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `thiefguard`'s `NT_GIVE` branch (`two.c:1704-1710`): any item
    /// handed over simply vanishes - no give-back fallback exists,
    /// matching C's plain `destroy_item` (same precedent as `world::npc::
    /// area17::barkeeper`'s own unconditional-destroy `NT_GIVE` handler).
    fn two_thiefguard_handle_give_message(&mut self, thiefguard_id: CharacterId) {
        let Some(item_id) = self
            .characters
            .get(&thiefguard_id)
            .and_then(|thiefguard| thiefguard.cursor_item)
        else {
            return;
        };
        if let Some(thiefguard) = self.characters.get_mut(&thiefguard_id) {
            thiefguard.cursor_item = None;
        }
        self.destroy_item(item_id);
    }

    /// C `take_money(co, 10000)` (`two.c:1682`, `tool.c:3820-3827`): plain
    /// `Character::gold` deduction, no bank fallback - same precedent as
    /// `world::npc::area17::barkeeper`/`servant`'s own private `take_
    /// money` copies.
    fn two_thiefguard_take_money(&mut self, player_id: CharacterId, amount: u32) -> bool {
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
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;
