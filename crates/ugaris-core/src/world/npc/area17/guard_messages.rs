//! `CDR_TWOGUARD` message-type handlers, split out of `guard.rs` to stay
//! under the ~800-line NPC-file guideline (same precedent as
//! `world::npc::area8::fdemon_army_movement`, split out of
//! `fdemon_army.rs` for the same reason) - see `guard.rs`'s own module
//! doc comment for the driver's full behavior/deviation notes.

use super::guard::{
    ALERT_COOLDOWN_TICKS, FINE_TIMEOUT_TICKS, FINE_WARN_INTERVAL_TICKS, LEAVE_TIMEOUT_GUEST_TICKS,
    LEAVE_TIMEOUT_TICKS, LEAVE_WARN_INTERVAL_TICKS, NOFIGHT_COOLDOWN_TICKS,
};
use crate::character_driver::{
    analyse_text_qa, TextAnalysisOutcome, NTID_TWOCITY, NTID_TWOCITY_PICK,
};
use crate::drvlib::offset2dx;
use crate::world::npc::area17::{
    illegal_place, TwoGuardDriverData, TwoGuardOutcomeEvent, TwoGuardPlayerFacts, CS_CITIZEN,
    CS_ENEMY, CS_GUEST, CS_HONOR, LS_CLEAN, LS_DEAD, LS_FINE, TWOCITY_QA,
};
use crate::world::*;

/// C `char_dist(cn, co) < 16` (`two.c:444`): guest-pass intro speech
/// range.
const GUEST_INTRO_DISTANCE: i32 = 16;

impl World {
    /// C `guard_driver`'s `NT_CHAR` branch (`two.c:388-491`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn two_guard_handle_char(
        &mut self,
        guard_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoGuardPlayerFacts>,
        now: i32,
        events: &mut Vec<TwoGuardOutcomeEvent>,
        data: &mut TwoGuardDriverData,
    ) {
        if data.busy {
            return;
        }
        let player_id = CharacterId(message.dat1 as u32);
        if data.victim.is_some() && data.victim != Some(player_id) {
            return;
        }
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        if now.saturating_sub(facts.last_attack) <= 2 {
            return;
        }
        if facts.current_guard != 0
            && facts.current_guard != guard_id.0 as i32
            && now.saturating_sub(facts.current_guard_time) <= 3
        {
            return;
        }
        if !char_see_char(&guard, &player, &self.map, self.date.daylight) {
            return;
        }
        let place = illegal_place(player.x, player.y);
        if place == 0 {
            return;
        }

        let mut update = TwoGuardOutcomeEvent {
            player_id,
            legal_status: facts.legal_status,
            legal_fine: facts.legal_fine,
            citizen_status: facts.citizen_status,
            current_guard: facts.current_guard,
            current_guard_time: facts.current_guard_time,
            last_attack: facts.last_attack,
            guard_intro: facts.guard_intro,
            bank_gold_deduction: None,
        };
        let mut changed = false;

        if facts.legal_status == LS_DEAD {
            data.victim = Some(player_id);
            data.victim_timeout = self.tick.0;
            data.victim_visible = true;
            data.victim_last_x = player.x;
            data.victim_last_y = player.y;
        } else if place > facts.citizen_status {
            data.victim = Some(player_id);
            data.victim_timeout = self.tick.0;
            update.current_guard = guard_id.0 as i32;
            update.current_guard_time = now;
            changed = true;
            data.victim_visible = true;
            data.victim_last_x = player.x;
            data.victim_last_y = player.y;

            if data.leave_state == 0 {
                self.npc_say(
                    guard_id,
                    &format!(
                        "Hey! {}! You have no business in there! Get out at once!",
                        player.name
                    ),
                );
                data.leave_state = 1;
                data.leave_timeout = self.tick.0;
                data.lastsay = self.tick.0;
                if let Some(direction) = offset2dx(
                    i32::from(guard.x),
                    i32::from(guard.y),
                    i32::from(player.x),
                    i32::from(player.y),
                ) {
                    if let Some(character) = self.characters.get_mut(&guard_id) {
                        let _ = turn(character, direction as u8);
                    }
                }
            }
            if data.leave_state == 1 {
                let timeout = if place == CS_GUEST {
                    LEAVE_TIMEOUT_GUEST_TICKS
                } else {
                    LEAVE_TIMEOUT_TICKS
                };
                if data.lastsay.saturating_add(LEAVE_WARN_INTERVAL_TICKS) < self.tick.0 {
                    self.npc_say(
                        guard_id,
                        &format!("Get out, {}, or I'll have to kill you!", player.name),
                    );
                    data.lastsay = self.tick.0;
                }
                if data.leave_timeout.saturating_add(timeout) < self.tick.0 {
                    data.leave_state = 2;
                    self.npc_say(guard_id, "You had ample time to leave, now you die!");
                }
            }
            if data.leave_state == 2 {
                update.last_attack = now;
                changed = true;
            }
        } else if facts.legal_status == LS_CLEAN {
            if facts.guard_intro == 0 && char_dist(&guard, &player) < GUEST_INTRO_DISTANCE {
                self.npc_say(
                    guard_id,
                    &format!(
                        "Listen carefully, {}. Thou art here on a guest pass. Any crime here, and thou wilt lose the right to enter our city.",
                        player.name
                    ),
                );
                update.guard_intro = 1;
                changed = true;
            }
        } else if facts.legal_status == LS_FINE {
            data.victim = Some(player_id);
            data.victim_timeout = self.tick.0;
            update.current_guard = guard_id.0 as i32;
            update.current_guard_time = now;
            changed = true;
            data.victim_visible = true;
            data.victim_last_x = player.x;
            data.victim_last_y = player.y;

            if data.fine_state == 0 {
                self.npc_say(
                    guard_id,
                    &format!(
                        "Hey, {}, you owe the city {:.2}G! Say pay to pay it!",
                        player.name,
                        f64::from(facts.legal_fine) / 100.0
                    ),
                );
                data.fine_state = 1;
                data.fine_timeout = self.tick.0;
                data.lastsay = self.tick.0;
                if let Some(direction) = offset2dx(
                    i32::from(guard.x),
                    i32::from(guard.y),
                    i32::from(player.x),
                    i32::from(player.y),
                ) {
                    if let Some(character) = self.characters.get_mut(&guard_id) {
                        let _ = turn(character, direction as u8);
                    }
                }
            }
            if data.fine_state == 1 {
                if data.lastsay.saturating_add(FINE_WARN_INTERVAL_TICKS) < self.tick.0 {
                    self.npc_say(
                        guard_id,
                        &format!("Come on, {}, pay or I'll have to kill you!", player.name),
                    );
                    data.lastsay = self.tick.0;
                }
                if data.fine_timeout.saturating_add(FINE_TIMEOUT_TICKS) < self.tick.0 {
                    data.fine_state = 2;
                    self.npc_say(guard_id, "You had ample time to pay, now you die!");
                } else if let Some(direction) = offset2dx(
                    i32::from(guard.x),
                    i32::from(guard.y),
                    i32::from(player.x),
                    i32::from(player.y),
                ) {
                    if let Some(character) = self.characters.get_mut(&guard_id) {
                        let _ = turn(character, direction as u8);
                    }
                }
            }
            if data.fine_state == 2 {
                update.last_attack = now;
                changed = true;
            }
        }

        if changed {
            events.push(update);
        }
    }

    /// C `guard_driver`'s `NT_TEXT` branch (`two.c:493-573`), wired
    /// through the shared `TWOCITY_QA` table.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn two_guard_handle_text(
        &mut self,
        guard_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoGuardPlayerFacts>,
        now: i32,
        events: &mut Vec<TwoGuardOutcomeEvent>,
        data: &mut TwoGuardDriverData,
    ) {
        let player_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        if guard_id == player_id || !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if !char_see_char(&guard, &player, &self.map, self.date.daylight) {
            return;
        }
        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        match analyse_text_qa(text, &guard.name, &player.name, TWOCITY_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(guard_id, &reply);
            }
            TextAnalysisOutcome::Matched(2) => {
                // C `case 2:` (`two.c:497-503`): reset `guard_intro`.
                events.push(TwoGuardOutcomeEvent {
                    player_id,
                    legal_status: facts.legal_status,
                    legal_fine: facts.legal_fine,
                    citizen_status: facts.citizen_status,
                    current_guard: facts.current_guard,
                    current_guard_time: facts.current_guard_time,
                    last_attack: facts.last_attack,
                    guard_intro: 0,
                    bank_gold_deduction: None,
                });
            }
            TextAnalysisOutcome::Matched(3) => {
                // C `case 3:` (`two.c:504-544`): "pay".
                self.two_guard_handle_pay(guard_id, player_id, &player, facts, events, data);
            }
            TextAnalysisOutcome::Matched(code @ (5 | 6 | 7 | 12)) => {
                // C `case 12/5/6/7:` (`two.c:545-570`): god-only
                // citizen-status admin commands.
                if player.flags.contains(CharacterFlags::GOD) {
                    let (citizen_status, legal_status, legal_fine) = match code {
                        12 => (CS_ENEMY, facts.legal_status, facts.legal_fine),
                        5 => (CS_GUEST, LS_CLEAN, 0),
                        6 => (CS_CITIZEN, facts.legal_status, facts.legal_fine),
                        _ => (CS_HONOR, facts.legal_status, facts.legal_fine),
                    };
                    events.push(TwoGuardOutcomeEvent {
                        player_id,
                        legal_status,
                        legal_fine,
                        citizen_status,
                        current_guard: facts.current_guard,
                        current_guard_time: facts.current_guard_time,
                        last_attack: facts.last_attack,
                        guard_intro: facts.guard_intro,
                        bank_gold_deduction: None,
                    });
                }
            }
            TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
        }

        // C `tabunga(cn, co, (char *)msg->dat2)` (`two.c:572`).
        self.apply_tabunga_text_notification(guard_id, player_id, text);
        let _ = now;
    }

    /// C `case 3:` of `guard_driver`'s `NT_TEXT` switch (`two.c:504-544`):
    /// the "pay" command, including the direct-gold and bank-fallback
    /// branches.
    fn two_guard_handle_pay(
        &mut self,
        guard_id: CharacterId,
        player_id: CharacterId,
        player: &Character,
        facts: &TwoGuardPlayerFacts,
        events: &mut Vec<TwoGuardOutcomeEvent>,
        data: &mut TwoGuardDriverData,
    ) {
        if facts.legal_status != LS_FINE {
            return;
        }
        let mut new_legal_status = facts.legal_status;
        let mut new_legal_fine = facts.legal_fine;
        let mut bank_gold_deduction = None;

        if player.gold >= facts.legal_fine.max(0) as u32 {
            self.npc_say(guard_id, &format!("Wise choice, {}.", player.name));
            if let Some(character) = self.characters.get_mut(&player_id) {
                character.gold = character
                    .gold
                    .saturating_sub(facts.legal_fine.max(0) as u32);
                character.flags.insert(CharacterFlags::ITEMS);
            }
            new_legal_status = LS_CLEAN;
            new_legal_fine = 0;
        } else {
            let need = facts.legal_fine - player.gold as i32;
            if need <= facts.bank_gold {
                self.npc_say(
                    guard_id,
                    &format!(
                        "Wise choice, {} (took {:.2}G from bank account).",
                        player.name,
                        f64::from(need) / 100.0
                    ),
                );
                bank_gold_deduction = Some(need);
                if let Some(character) = self.characters.get_mut(&player_id) {
                    character.gold = 0;
                    character.flags.insert(CharacterFlags::ITEMS);
                }
                new_legal_status = LS_CLEAN;
                new_legal_fine = 0;
            } else {
                self.npc_say(
                    guard_id,
                    &format!(
                        "Gosh, {}'s broke. Well, {}'ll die then.",
                        hename(player),
                        hename(player)
                    ),
                );
            }
        }

        let mut still_illegal = true;
        if new_legal_status == LS_CLEAN {
            still_illegal = illegal_place(player.x, player.y) > facts.citizen_status;
            if !still_illegal {
                // C `fight_driver_remove_enemy(cn, co)` (`two.c:540`).
                if data.victim == Some(player_id) {
                    data.victim = None;
                }
                data.nofight_timer = self.tick.0;
                // C `player_driver_stop(ch[co].player, 1)` (`two.c:542`)
                // is not ported - see `guard.rs`'s module doc comment.
            }
        }
        let _ = still_illegal;

        events.push(TwoGuardOutcomeEvent {
            player_id,
            legal_status: new_legal_status,
            legal_fine: new_legal_fine,
            citizen_status: facts.citizen_status,
            current_guard: facts.current_guard,
            current_guard_time: facts.current_guard_time,
            last_attack: facts.last_attack,
            guard_intro: facts.guard_intro,
            bank_gold_deduction,
        });
    }

    /// C `guard_driver`'s `NT_GOTHIT` branch (`two.c:574-599`) plus the
    /// `standard_message_driver(cn, msg, 0, 0)` fallback's `NT_GOTHIT`
    /// case (`drvlib.c:2512-2538`), which is the only one of the four
    /// `standard_message_driver` cases this driver's call actually
    /// reaches (`agressive=0`/`helper=0` no-op the `NT_CHAR`/`NT_SEEHIT`
    /// cases) - see `guard.rs`'s module doc comment.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn two_guard_handle_gothit(
        &mut self,
        guard_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoGuardPlayerFacts>,
        now: i32,
        events: &mut Vec<TwoGuardOutcomeEvent>,
        data: &mut TwoGuardDriverData,
    ) {
        let attacker_id = CharacterId(message.dat1 as u32);
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        let Some(attacker) = self.characters.get(&attacker_id).cloned() else {
            return;
        };

        // C `two.c:576-581`.
        let half_hp = character_value(&guard, CharacterValue::Hp) * POWERSCALE / 2;
        if guard.hp < half_hp
            && (data.lastalert == 0
                || self.tick.0.saturating_sub(data.lastalert) > ALERT_COOLDOWN_TICKS)
        {
            self.npc_say(guard_id, "Help! Officer under attack!");
            self.two_city_call_guard(guard_id, attacker_id);
            data.lastalert = self.tick.0;
        }

        // C `two.c:582-598`.
        if attacker.flags.contains(CharacterFlags::PLAYER) {
            if let Some(facts) = player_facts.get(&attacker_id) {
                let mut update = TwoGuardOutcomeEvent {
                    player_id: attacker_id,
                    legal_status: facts.legal_status,
                    legal_fine: facts.legal_fine,
                    citizen_status: facts.citizen_status,
                    current_guard: facts.current_guard,
                    current_guard_time: facts.current_guard_time,
                    last_attack: now,
                    guard_intro: facts.guard_intro,
                    bank_gold_deduction: None,
                };
                if facts.legal_status != LS_DEAD && now.saturating_sub(facts.last_attack) > 10 {
                    update.legal_status = LS_FINE;
                    update.legal_fine = facts.legal_fine + 2000;
                    if facts.citizen_status == CS_GUEST {
                        self.npc_say(
                            guard_id,
                            "We do not allow strangers to commit any crime here. Leave at once!",
                        );
                        update.citizen_status = CS_ENEMY;
                    } else {
                        self.npc_say(
                            guard_id,
                            &format!(
                                "Hey {}! Fine for attacking a city guard: 20G! Say pay to pay it!",
                                attacker.name
                            ),
                        );
                    }
                }
                events.push(update);
            }
        }

        // C `standard_message_driver`'s `NT_GOTHIT` case (`drvlib.c:2512-
        // 2538`), gated on `dat->nofight_timer` (`two.c:660`).
        if self.tick.0.saturating_sub(data.nofight_timer) > NOFIGHT_COOLDOWN_TICKS
            && guard.group != attacker.group
            && can_attack(&guard, &attacker, &self.map)
        {
            data.victim = Some(attacker_id);
            data.victim_timeout = self.tick.0;
            data.victim_visible = char_see_char(&guard, &attacker, &self.map, self.date.daylight);
            if data.victim_visible {
                data.victim_last_x = attacker.x;
                data.victim_last_y = attacker.y;
            }
        }
    }

    /// C `guard_driver`'s `NT_SEEHIT` branch (`two.c:600-624`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn two_guard_handle_seehit(
        &mut self,
        guard_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoGuardPlayerFacts>,
        now: i32,
        events: &mut Vec<TwoGuardOutcomeEvent>,
        data: &mut TwoGuardDriverData,
    ) {
        let attacker_id = CharacterId(message.dat1.max(0) as u32);
        let victim_id = CharacterId(message.dat2.max(0) as u32);
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        if victim_id == guard_id || attacker_id.0 == 0 || victim_id.0 == 0 {
            return;
        }
        let Some(victim) = self.characters.get(&victim_id).cloned() else {
            return;
        };
        if victim.group != guard.group {
            return;
        }
        let Some(attacker) = self.characters.get(&attacker_id).cloned() else {
            return;
        };
        if !attacker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if !char_see_char(&guard, &attacker, &self.map, self.date.daylight) {
            return;
        }
        let Some(facts) = player_facts.get(&attacker_id) else {
            return;
        };

        if facts.legal_status != LS_DEAD && now.saturating_sub(facts.last_attack) > 10 {
            let mut update = TwoGuardOutcomeEvent {
                player_id: attacker_id,
                legal_status: LS_FINE,
                legal_fine: facts.legal_fine + 7500,
                citizen_status: facts.citizen_status,
                current_guard: facts.current_guard,
                current_guard_time: facts.current_guard_time,
                last_attack: now,
                guard_intro: facts.guard_intro,
                bank_gold_deduction: None,
            };
            if facts.citizen_status == CS_GUEST {
                self.npc_say(
                    guard_id,
                    "Protect the innocent! We do not allow strangers to commit any crime here. Leave at once!",
                );
                update.citizen_status = CS_ENEMY;
            } else {
                self.npc_say(
                    guard_id,
                    &format!(
                        "Protect the innocent! Stop at once and pay your fine, {}!",
                        attacker.name
                    ),
                );
            }
            events.push(update);
        }

        if self.tick.0.saturating_sub(data.nofight_timer) > NOFIGHT_COOLDOWN_TICKS {
            data.victim = Some(attacker_id);
            data.victim_timeout = self.tick.0;
            data.victim_visible = true;
            data.victim_last_x = attacker.x;
            data.victim_last_y = attacker.y;
        }
    }

    /// C `guard_driver`'s `NT_NPC` branch (`two.c:626-658`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn two_guard_handle_npc(
        &mut self,
        guard_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TwoGuardPlayerFacts>,
        now: i32,
        events: &mut Vec<TwoGuardOutcomeEvent>,
        data: &mut TwoGuardDriverData,
    ) {
        if message.dat1 == NTID_TWOCITY {
            let packed = message.dat3;
            data.tx = (packed.rem_euclid(MAX_MAP as i32)) as u16;
            data.ty = (packed.div_euclid(MAX_MAP as i32)) as u16;
            data.good_tx_try = self.tick.0;
            return;
        }
        if message.dat1 != NTID_TWOCITY_PICK {
            return;
        }

        let player_id = CharacterId(message.dat2.max(0) as u32);
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if !char_see_char(&guard, &player, &self.map, self.date.daylight) {
            return;
        }
        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };
        if now.saturating_sub(facts.last_attack) <= 60 {
            return;
        }

        let mut update = TwoGuardOutcomeEvent {
            player_id,
            legal_status: facts.legal_status,
            legal_fine: facts.legal_fine,
            citizen_status: facts.citizen_status,
            current_guard: facts.current_guard,
            current_guard_time: facts.current_guard_time,
            last_attack: facts.last_attack,
            guard_intro: facts.guard_intro,
            bank_gold_deduction: None,
        };
        let mut changed = false;

        if illegal_place(player.x, player.y) > facts.citizen_status {
            self.npc_say(guard_id, "Thou shalt not steal! Now thou wilt die!");
            data.victim = Some(player_id);
            data.victim_timeout = self.tick.0;
            data.victim_visible = true;
            data.victim_last_x = player.x;
            data.victim_last_y = player.y;
            update.last_attack = now;
            changed = true;
        }
        if facts.legal_status != LS_DEAD {
            update.legal_status = LS_FINE;
            update.legal_fine = facts.legal_fine + 3000;
            changed = true;
            if facts.citizen_status == CS_GUEST {
                self.npc_say(
                    guard_id,
                    "Hey! Stop thief! We do not allow strangers to commit any crime here. Leave at once!",
                );
                update.citizen_status = CS_ENEMY;
                // C omits `ppd->last_attack = realtime;` in this branch
                // (`two.c:646-649`) - a real quirk, preserved as-is.
                update.last_attack = facts.last_attack.max(update.last_attack.min(now));
            } else {
                self.npc_say(guard_id, "Hey! Stop thief! Fine for breaking a lock: 30G.");
                update.last_attack = now;
            }
        }

        if changed {
            events.push(update);
        }
    }
}
