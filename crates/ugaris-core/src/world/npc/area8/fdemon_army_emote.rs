//! `CDR_FDEMON_ARMY`'s personality/chat engine - C `struct emote`/`do_emote`/
//! `got_emote` (`src/area/8/fdemon.c:323-1325`). Split out of
//! `fdemon_army.rs`/`fdemon_army_combat.rs` to keep those files within the
//! ~800-line NPC-file guideline.
//!
//! Ports:
//! - [`SoldierEmote`] (C `struct emote`, `fdemon.c:323-344`): each recruited
//!   soldier's live personality state (four base "tendencies" -
//!   `cuddly`/`angst`/`bore`/`bigmouth`, assigned once from
//!   [`super::SoldierEmoteBase`] at recruit time - plus four "current" needs
//!   that build up over time - `lonely`/`fear`/`boredom`/`praise` - and a
//!   per-platoon-slot relationship score, `likes`/`talked`).
//! - [`World::fdemon_army_do_emote`] (C `do_emote`, `fdemon.c:781-1000`): the
//!   soldier's own proactive small talk, triggered once a "current" need
//!   crosses a threshold. Ported digit-for-digit including a real C quirk:
//!   `bestscore`/`bestco`/`bestn` are declared *once* at the top of the
//!   function and never reset between the four `if` blocks (lonely/boredom/
//!   fear/praise), so a later block can silently reuse an earlier block's
//!   leftover target/score if its own platoon scan doesn't beat that
//!   leftover score - and if the very first triggered block's scan finds no
//!   candidate at all, the *whole function* returns immediately, skipping
//!   every later block even if its own threshold was also crossed this
//!   tick. Both behaviors are reproduced here via a single `best`/
//!   `best_score` pair shared across all four blocks and an early-return
//!   (`break 'emote`) exactly where C's `if (!bestco) return;` sits.
//! - [`World::fdemon_army_got_emote`] (C `got_emote`, `fdemon.c:999-1325`):
//!   the soldier's reactions to a platoon-mate's emote small talk (any of
//!   [`super::FDEMON_ARMY_EMOTE_QA`]'s `QA_YES`..`QA_COWARD` codes) - most
//!   codes check `dat->emote.answer_type`/`answer_cn`/`answer_timer` against
//!   whatever [`World::fdemon_army_do_emote`] last asked (within `TICKS*30`
//!   ticks, matching C's `ticker - dat->emote.answer_timer > TICKS * 30`),
//!   falling back to a generic "didn't ask that" reply and no state change
//!   when the pending-question context doesn't match.
//!
//! Not yet ported (documented gap): persisting `SoldierEmote` across a
//! recruit/drop/re-recruit cycle (C `take_soldiers`/`drop_soldiers` copy
//! `dat->emote` to/from `ppd->soldier[n].emote`, `fdemon.c:559-563,608-612`)
//! - a freshly (re-)spawned soldier's [`SoldierEmote`] always starts from
//!   [`super::assign_profile`]'s base tendencies with every "current"/
//!   relationship field at `0`, rather than carrying over relationship
//!   history from a prior recruitment. The legacy `DRD_FARMY_PPD` blob
//!   layout already reserves the exact byte range for this (see
//!   `crates/ugaris-core/src/player/misc.rs`'s `FARMY_PPD_BOSS_COUNTER_
//!   OFFSET` doc comment), so this is a follow-up wiring task, not a
//!   missing byte-layout fact.

use crate::{character_driver::CharacterDriverState, world::*};

use super::{
    MAXSOLDIER, QA_AFRAID, QA_BEQUIET, QA_COWARD, QA_DONTTHINKSO, QA_DOSOMETHING, QA_DOSOON,
    QA_FUNNYFACE, QA_GOAWAY, QA_GREATEST, QA_GREATSOLDIER, QA_ISTHATSO, QA_LIKESMILE, QA_NICEDAY,
    QA_NO, QA_NONEED, QA_NOTFIGHT, QA_NOTTHATBAD, QA_ONEDAY, QA_QUIETBIGMOUTH, QA_SHUTUP,
    QA_SMELLRATLING, QA_STOPBOTHER, QA_THANKS, QA_TOUGHFELLOW, QA_TURNBACK, QA_WHATSUP, QA_WHYMEAN,
    QA_YES, QA_YOUAFRAID, QA_YOUSTINK,
};

/// C `#define AT_YESNO 1` .. `#define AT_AFFIRM 6` (`fdemon.c:774-779`).
const AT_YESNO: i32 = 1;
const AT_THANKS: i32 = 2;
const AT_INSULT: i32 = 3;
const AT_RELAX: i32 = 4;
const AT_ENCOURAGE: i32 = 5;
const AT_AFFIRM: i32 = 6;

/// A pending "yes/no"-style question's timeout: C `ticker - dat->emote.
/// answer_timer > TICKS * 30` (`fdemon.c:1002` etc.), 30 seconds.
const ANSWER_TIMEOUT_TICKS: i64 = TICKS_PER_SECOND as i64 * 30;

/// C `struct emote` (`fdemon.c:323-344`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SoldierEmote {
    pub cuddly: i32,
    pub lonely: i32,
    pub angst: i32,
    pub fear: i32,
    pub bore: i32,
    pub boredom: i32,
    pub bigmouth: i32,
    pub praise: i32,
    /// `int likes[MAXSOLDIER + 1]`, indexed by platoon slot (`0..MAXSOLDIER`
    /// = fellow soldiers, `MAXSOLDIER` = the leader).
    pub likes: [i32; MAXSOLDIER + 1],
    pub talked: [i32; MAXSOLDIER + 1],
    pub answer_timer: i64,
    /// `int answer_cn`: the platoon member id (`0` = none) [`World::
    /// fdemon_army_do_emote`] last addressed a pending question to.
    pub answer_cn: i32,
    pub answer_type: i32,
    pub last_emote: i64,
}

impl Default for SoldierEmote {
    fn default() -> Self {
        SoldierEmote {
            cuddly: 0,
            lonely: 0,
            angst: 0,
            fear: 0,
            bore: 0,
            boredom: 0,
            bigmouth: 0,
            praise: 0,
            likes: [0; MAXSOLDIER + 1],
            talked: [0; MAXSOLDIER + 1],
            answer_timer: 0,
            answer_cn: 0,
            answer_type: 0,
            last_emote: 0,
        }
    }
}

impl World {
    /// C `do_emote(cn, dat)` (`fdemon.c:781-1000`) - see the module doc
    /// comment for the shared-`bestscore`-across-blocks quirk this
    /// reproduces digit-for-digit.
    pub fn fdemon_army_do_emote(&mut self, soldier_id: CharacterId) {
        let Some(soldier) = self.characters.get(&soldier_id).cloned() else {
            return;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = soldier.driver_state.clone() else {
            return;
        };
        let platoon = dat.platoon;
        let mut emote = dat.emote;
        let own_gender = soldier.flags & (CharacterFlags::MALE | CharacterFlags::FEMALE);
        let tick = self.tick.0 as i64;
        let hour = self.date.hour;

        let mut best_score = -99999i32;
        let mut best: Option<(usize, CharacterId)> = None;
        let mut lines: Vec<String> = Vec::new();

        'emote: {
            if emote.lonely > 5000 {
                for slot in 0..=MAXSOLDIER {
                    let co = platoon[slot];
                    if co.0 == 0 {
                        continue;
                    }
                    let Some(other) = self.characters.get(&co) else {
                        continue;
                    };
                    let other_gender =
                        other.flags & (CharacterFlags::MALE | CharacterFlags::FEMALE);
                    if other_gender == own_gender {
                        continue;
                    }
                    let score = emote.likes[slot] + emote.talked[slot];
                    if score > best_score {
                        best_score = score;
                        best = Some((slot, co));
                    }
                }
                let Some((slot, co)) = best else {
                    break 'emote;
                };
                let Some(target_name) = self.characters.get(&co).map(|c| c.name.clone()) else {
                    break 'emote;
                };
                if emote.likes[slot] < 10 {
                    lines.push(if hour > 6 && hour < 20 {
                        format!("Oh, what a nice day it is, {target_name}, isn't it?")
                    } else {
                        format!("The nights here are scary, {target_name}, aren't they?")
                    });
                    if emote.talked[slot] > -2 {
                        emote.likes[slot] += 1;
                    }
                    emote.talked[slot] -= 1;
                    emote.answer_timer = tick;
                    emote.answer_cn = co.0 as i32;
                    emote.answer_type = AT_YESNO;
                } else if emote.likes[slot] < 20 {
                    lines.push(format!("I like the way you smile, {target_name}."));
                    if emote.talked[slot] > -2 {
                        emote.likes[slot] += 1;
                    }
                    emote.talked[slot] -= 1;
                    emote.answer_timer = tick;
                    emote.answer_cn = co.0 as i32;
                    emote.answer_type = AT_THANKS;
                }
                emote.lonely /= 2;
                emote.last_emote = tick;
            }

            if emote.boredom > 10000 {
                for slot in 0..=MAXSOLDIER {
                    let co = platoon[slot];
                    if co.0 == 0 || co == soldier_id {
                        continue;
                    }
                    let score = emote.talked[slot] - emote.likes[slot];
                    if score > best_score {
                        best_score = score;
                        best = Some((slot, co));
                    }
                }
                let Some((slot, co)) = best else {
                    break 'emote;
                };
                let Some(target_name) = self.characters.get(&co).map(|c| c.name.clone()) else {
                    break 'emote;
                };
                if emote.likes[slot] < 0 {
                    lines.push(format!("You stink, {target_name}."));
                    if emote.talked[slot] > -2 {
                        emote.likes[slot] -= 1;
                    }
                    emote.talked[slot] -= 1;
                    emote.answer_timer = tick;
                    emote.answer_cn = co.0 as i32;
                    emote.answer_type = AT_INSULT;
                } else if emote.likes[slot] < 10 {
                    lines.push(format!("Oh, come on, do something, {target_name}."));
                    if emote.talked[slot] > -2 {
                        emote.likes[slot] -= 1;
                    }
                    emote.talked[slot] -= 1;
                    emote.answer_timer = tick;
                    emote.answer_cn = co.0 as i32;
                    emote.answer_type = AT_RELAX;
                } else if emote.likes[slot] < 20 {
                    lines.push(format!("You have a funny face, {target_name}."));
                    if emote.talked[slot] > -2 {
                        emote.likes[slot] -= 1;
                    }
                    emote.talked[slot] -= 1;
                    emote.answer_timer = tick;
                    emote.answer_cn = co.0 as i32;
                    emote.answer_type = AT_RELAX;
                }
                // C: the `likes[n] < 0` branch and the (unreachable, C's own
                // dead `else if (likes[n] < 0)`) second branch are identical
                // conditions - kept verbatim above as a single `< 0` arm.
                emote.boredom /= 2;
                emote.last_emote = tick;
            }

            if emote.fear > 1000 {
                for slot in 0..=MAXSOLDIER {
                    let co = platoon[slot];
                    if co.0 == 0 || co == soldier_id {
                        continue;
                    }
                    let score = emote.likes[slot] + emote.talked[slot];
                    if score > best_score {
                        best_score = score;
                        best = Some((slot, co));
                    }
                }
                let Some((slot, co)) = best else {
                    break 'emote;
                };
                let Some(target_name) = self.characters.get(&co).map(|c| c.name.clone()) else {
                    break 'emote;
                };
                if emote.likes[slot] < 10 {
                    lines.push(format!(
                        "Shouldn't we turn back? What do you think, {target_name}?"
                    ));
                    if emote.talked[slot] > -2 {
                        emote.likes[slot] += 1;
                    }
                    emote.talked[slot] -= 1;
                    emote.answer_timer = tick;
                    emote.answer_cn = co.0 as i32;
                    emote.answer_type = AT_ENCOURAGE;
                } else if emote.likes[slot] < 20 {
                    lines.push(format!("I'm afraid, {target_name}."));
                    if emote.talked[slot] > -2 {
                        emote.likes[slot] += 1;
                    }
                    emote.talked[slot] -= 1;
                    emote.answer_timer = tick;
                    emote.answer_cn = co.0 as i32;
                    emote.answer_type = AT_ENCOURAGE;
                }
                emote.fear /= 2;
                emote.last_emote = tick;
            }

            if emote.praise > 100 {
                for slot in 0..=MAXSOLDIER {
                    let co = platoon[slot];
                    if co.0 == 0 {
                        continue;
                    }
                    let Some(other) = self.characters.get(&co) else {
                        continue;
                    };
                    let other_gender =
                        other.flags & (CharacterFlags::MALE | CharacterFlags::FEMALE);
                    if other_gender == own_gender {
                        continue;
                    }
                    let mut seed = self.legacy_random_seed;
                    let roll = legacy_random_below_from_seed(&mut seed, 30) as i32;
                    self.legacy_random_seed = seed;
                    let score = emote.likes[slot] + emote.talked[slot] + roll;
                    if score > best_score {
                        best_score = score;
                        best = Some((slot, co));
                    }
                }
                let Some((slot, co)) = best else {
                    break 'emote;
                };
                let Some(target_name) = self.characters.get(&co).map(|c| c.name.clone()) else {
                    break 'emote;
                };
                if emote.likes[slot] < 10 {
                    lines.push(format!("Ha! I'm the greatest, right, {target_name}?"));
                    if emote.talked[slot] > -2 {
                        emote.likes[slot] += 1;
                    }
                    emote.talked[slot] -= 1;
                    emote.answer_timer = tick;
                    emote.answer_cn = co.0 as i32;
                    emote.answer_type = AT_AFFIRM;
                } else if emote.likes[slot] < 20 {
                    lines.push(format!("I'm becoming a great soldier, {target_name}."));
                    if emote.talked[slot] > -2 {
                        emote.likes[slot] += 1;
                    }
                    emote.talked[slot] -= 1;
                    emote.answer_timer = tick;
                    emote.answer_cn = co.0 as i32;
                    emote.answer_type = AT_AFFIRM;
                }
                emote.praise /= 2;
                emote.last_emote = tick;
            }
        }

        for line in lines {
            self.npc_say(soldier_id, &line);
        }
        if let Some(CharacterDriverState::FdemonArmy(dat)) = self
            .characters
            .get_mut(&soldier_id)
            .and_then(|c| c.driver_state.as_mut())
        {
            dat.emote = emote;
        }
    }

    /// C `got_emote(cn, co, slot, nr, dat)` (`fdemon.c:999-1325`): `speaker_id`
    /// is C's `co`, `slot` is C's `slot` (the [`super::MAXSOLDIER`]`+1`-sized
    /// platoon index returned by `find_platoon`), `code` is C's `nr` (one of
    /// [`super::QA_YES`]..[`super::QA_COWARD`], from [`super::
    /// FDEMON_ARMY_EMOTE_QA`]).
    pub fn fdemon_army_got_emote(
        &mut self,
        soldier_id: CharacterId,
        speaker_id: CharacterId,
        slot: usize,
        code: i32,
    ) {
        let Some(CharacterDriverState::FdemonArmy(dat)) = self
            .characters
            .get(&soldier_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        if slot >= dat.emote.likes.len() {
            return;
        }
        let mut emote = dat.emote;
        let tick = self.tick.0 as i64;
        let speaker_cn = speaker_id.0 as i32;
        let expired = |emote: &SoldierEmote, at: i32| {
            emote.answer_type != at
                || emote.answer_cn != speaker_cn
                || tick - emote.answer_timer > ANSWER_TIMEOUT_TICKS
        };
        let clear_answer = |emote: &mut SoldierEmote| {
            emote.talked[slot] += 1;
            emote.answer_cn = 0;
            emote.answer_timer = 0;
        };

        let line: Option<String> = match code {
            QA_YES => {
                if expired(&emote, AT_YESNO) {
                    Some(format!("Yes what, {speaker_name}?"))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 2;
                    None
                }
            }
            QA_NO => {
                if expired(&emote, AT_YESNO) {
                    Some(format!("No what, {speaker_name}?"))
                } else {
                    clear_answer(&mut emote);
                    None
                }
            }
            QA_NICEDAY => {
                let line = if emote.likes[slot] > 0 {
                    emote.likes[slot] += 1;
                    format!("Yes, {speaker_name}.")
                } else {
                    format!("No, {speaker_name}.")
                };
                emote.talked[slot] += 1;
                emote.lonely -= 200;
                Some(line)
            }
            QA_THANKS => {
                if expired(&emote, AT_THANKS) {
                    Some(format!("Thanks? Thanks for what, {speaker_name}?"))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 2;
                    None
                }
            }
            QA_YOUSTINK => {
                let line = if emote.likes[slot] > 0 {
                    emote.likes[slot] -= 5;
                    format!("Why are you so mean to me, {speaker_name}?")
                } else {
                    emote.likes[slot] -= 1;
                    format!("And you smell like a ratling, {speaker_name}!")
                };
                emote.talked[slot] += 1;
                emote.boredom -= 2000;
                Some(line)
            }
            QA_GOAWAY => {
                let line = if emote.likes[slot] > 0 {
                    emote.likes[slot] -= 2;
                    format!("What's up with you, {speaker_name}?")
                } else {
                    emote.likes[slot] -= 1;
                    format!("Shut up, {speaker_name}!")
                };
                emote.talked[slot] += 1;
                emote.boredom -= 2000;
                Some(line)
            }
            QA_DOSOMETHING => {
                let line = if emote.likes[slot] > 0 {
                    format!("Yeah, I hope we'll do something soon, {speaker_name}.")
                } else {
                    emote.likes[slot] -= 1;
                    format!("Stop bothering me, {speaker_name}!")
                };
                emote.talked[slot] += 1;
                emote.boredom -= 2000;
                Some(line)
            }
            QA_FUNNYFACE => {
                let line = if emote.likes[slot] > 0 {
                    format!("I'm bored too, {speaker_name}, let's not fight.")
                } else {
                    emote.likes[slot] -= 1;
                    format!("Oh boy! Please be quiet, {speaker_name}.")
                };
                emote.talked[slot] += 1;
                emote.boredom -= 2000;
                Some(line)
            }
            QA_LIKESMILE => {
                let line = if emote.likes[slot] > 5 {
                    emote.likes[slot] += 1;
                    format!("Why, thank you, {speaker_name}.")
                } else {
                    format!("Is that so, {speaker_name}?")
                };
                emote.talked[slot] += 1;
                emote.lonely -= 200;
                Some(line)
            }
            QA_TURNBACK => {
                let line = if emote.likes[slot] > -5 {
                    emote.likes[slot] += 1;
                    format!("It's not that bad, {speaker_name}.")
                } else {
                    emote.likes[slot] -= 1;
                    format!("Are you afraid, {speaker_name}?")
                };
                emote.talked[slot] += 1;
                emote.fear -= 20;
                Some(line)
            }
            QA_GREATEST => {
                let line = if emote.likes[slot] > 5 {
                    format!("You're a tough fellow, {speaker_name}!")
                } else {
                    emote.likes[slot] -= 2;
                    format!("Oh, be quiet, {speaker_name}, you bigmouth!")
                };
                emote.talked[slot] += 1;
                emote.praise += 10;
                Some(line)
            }
            QA_GREATSOLDIER => {
                let line = if emote.likes[slot] > 0 {
                    format!("One day you'll be a great soldier, {speaker_name}.")
                } else {
                    emote.likes[slot] -= 2;
                    format!("I don't think so, {speaker_name}.")
                };
                emote.talked[slot] += 1;
                emote.praise += 10;
                Some(line)
            }
            QA_AFRAID => {
                let line = if emote.likes[slot] > 0 {
                    emote.likes[slot] += 2;
                    format!("There's no need to be afraid, {speaker_name}.")
                } else {
                    emote.likes[slot] -= 2;
                    format!("Shut up you, {speaker_name}, you coward!")
                };
                emote.talked[slot] += 1;
                emote.fear -= 20;
                Some(line)
            }
            QA_WHYMEAN => {
                if expired(&emote, AT_INSULT) {
                    Some(format!("Mean? Why am I mean, {speaker_name}?"))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 3;
                    None
                }
            }
            QA_SMELLRATLING => {
                if expired(&emote, AT_INSULT) {
                    Some(format!("Oh yeah? Is that so, {speaker_name}?"))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] -= 1;
                    None
                }
            }
            QA_WHATSUP => {
                if expired(&emote, AT_INSULT) {
                    Some(format!("Oh, nothing, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 1;
                    None
                }
            }
            QA_SHUTUP => {
                if expired(&emote, AT_INSULT) {
                    Some(format!("Why? I didn't say anything, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] -= 1;
                    None
                }
            }
            QA_DOSOON => {
                if expired(&emote, AT_RELAX) {
                    Some(format!("Oh? Oh, that's fine, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 1;
                    None
                }
            }
            QA_STOPBOTHER => {
                if expired(&emote, AT_RELAX) {
                    Some(format!("I am not bothering you, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] -= 1;
                    None
                }
            }
            QA_NOTFIGHT => {
                if expired(&emote, AT_RELAX) {
                    Some(format!("I wasn't trying to pick a fight, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 1;
                    None
                }
            }
            QA_BEQUIET => {
                if expired(&emote, AT_RELAX) {
                    Some(format!("But I am quiet, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] -= 1;
                    None
                }
            }
            QA_ISTHATSO => {
                if expired(&emote, AT_THANKS) {
                    Some(format!("Hu? What is how, {speaker_name}?"))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] -= 3;
                    None
                }
            }
            QA_NOTTHATBAD => {
                if expired(&emote, AT_ENCOURAGE) {
                    Some(format!("I didn't say it is, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 2;
                    None
                }
            }
            QA_YOUAFRAID => {
                if expired(&emote, AT_ENCOURAGE) {
                    Some(format!("Afraid? Me? That's silly, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] -= 2;
                    None
                }
            }
            QA_NONEED => {
                // C: same fallback text as `QA_YOUAFRAID` - kept verbatim.
                if expired(&emote, AT_ENCOURAGE) {
                    Some(format!("Afraid? Me? That's silly, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 2;
                    None
                }
            }
            QA_COWARD => {
                if expired(&emote, AT_ENCOURAGE) {
                    Some(format!("Why are you calling me a coward, {speaker_name}?"))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] -= 2;
                    None
                }
            }
            QA_TOUGHFELLOW => {
                if expired(&emote, AT_AFFIRM) {
                    Some(format!("That's nice to hear, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 2;
                    None
                }
            }
            QA_QUIETBIGMOUTH => {
                if expired(&emote, AT_AFFIRM) {
                    Some(format!("What? But I didn't say anything, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] -= 2;
                    None
                }
            }
            QA_ONEDAY => {
                if expired(&emote, AT_AFFIRM) {
                    Some(format!("That's very nice to hear, {speaker_name}."))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] += 2;
                    None
                }
            }
            QA_DONTTHINKSO => {
                if expired(&emote, AT_AFFIRM) {
                    Some(format!("You don't think what, {speaker_name}?"))
                } else {
                    clear_answer(&mut emote);
                    emote.likes[slot] -= 1;
                    None
                }
            }
            _ => return,
        };

        if let Some(CharacterDriverState::FdemonArmy(dat)) = self
            .characters
            .get_mut(&soldier_id)
            .and_then(|c| c.driver_state.as_mut())
        {
            dat.emote = emote;
        }
        if let Some(line) = line {
            self.npc_say(soldier_id, &line);
        }
    }

    /// C `fdemon_army`'s emote-stats debug command (`case 7`,
    /// `fdemon.c:1414-1421`): reports every `struct emote` field as a single
    /// `say()` line. Only reachable via [`super::FDEMON_QA`]'s `"emote"`
    /// entry (`answer_code: 7`), gated the same way commands `2..=6` are -
    /// only the leader's own speech may trigger it.
    pub fn fdemon_army_emote_stats_line(&self, soldier_id: CharacterId) -> Option<String> {
        let CharacterDriverState::FdemonArmy(dat) =
            self.characters.get(&soldier_id)?.driver_state.clone()?
        else {
            return None;
        };
        let e = dat.emote;
        Some(format!(
            "cuddly={}, lonely={}, angst={}, fear={}, bore={}, boredom={}, bigmouth={}, praise={}, \
             like={}/{}/{} {}, replied={}/{}/{} {}",
            e.cuddly,
            e.lonely,
            e.angst,
            e.fear,
            e.bore,
            e.boredom,
            e.bigmouth,
            e.praise,
            e.likes[0],
            e.likes[1],
            e.likes[2],
            e.likes[3],
            e.talked[0],
            e.talked[1],
            e.talked[2],
            e.talked[3],
        ))
    }
}
