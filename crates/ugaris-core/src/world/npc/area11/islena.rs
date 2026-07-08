//! Islena boss NPC (`CDR_PALACEISLENA`), the palace's final fight - a
//! four-line greeting dialogue that turns into an unavoidable, unkillable-
//! feeling fight the moment a player attacks her (or fails to answer).
//!
//! Ports `src/area/11/palace.c::palace_islena` (`:578-739`) plus its death
//! hook `islena_dead` (`:741-767`, ported as [`World::apply_islena_death`],
//! hooked directly into [`World::kill_character_followup`] - same
//! precedent as `CDR_PENTER`'s `World::apply_penter_demon_death`, since
//! (like that hook) this needs no `PlayerRuntime` access: the only
//! per-player state involved, `islena_state`, is read through the same
//! `PlayerFacts`/`OutcomeEvent` split as every other dialogue-driven NPC
//! for the per-tick driver, but the death hook itself only touches
//! `CharacterFlags::WON`, a plain `Character` flag).
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification as every other ported
//!   area NPC (see `world::asturin`'s module doc comment): C's generic
//!   10-slot `struct fight_driver_data` is narrowed to a single tracked
//!   `victim`. `fight_driver_add_enemy(cn, co, 1, 1)` for the `islena_state
//!   >= 10` sighting branch and `standard_message_driver(cn, msg, 0, 0)`'s
//!   own unconditional `NT_GOTHIT` self-defense branch (see that
//!   function's own doc comment in `src/system/drvlib.c:2512-2538` - the
//!   `agressive`/`helper` params it's called with here, both `0`, only
//!   gate the `NT_CHAR`/`NT_SEEHIT` cases, not `NT_GOTHIT`) both collapse
//!   into setting `data.victim`.
//! - `fight_driver_set_dist(cn, 20, 0, 30)` (`palace.c:595`, on
//!   `NT_CREATE`) is not ported, same precedent as every other
//!   single-victim NPC's own module doc comment.
//! - `fight_driver_note_hit(cn)` (`drvlib.c:2514`, inside
//!   `standard_message_driver`'s `NT_GOTHIT` case): server-side hit-rate
//!   bookkeeping with no client-visible effect and no other C reader in
//!   this file - not ported, same "dead in practice" precedent as
//!   `world::robber`'s `charlog` calls.
//! - `reset_name(co)` (`palace.c:753`) has no Rust port anywhere yet
//!   (documented gap in several other files, e.g.
//!   `world::character_values`) - cosmetic title-suffix refresh, safe to
//!   skip.
//! - The `ACHIEVEMENT_LADYKILLER` award needs the async DB-backed
//!   achievement repository `World` doesn't have access to, so
//!   `apply_islena_death` only queues the winning player's id (mirroring
//!   `pending_penter_demon_lords_demise_awards`); `ugaris-server`'s
//!   `crate::area11::process_islena_ladykiller_awards` drains it once per
//!   tick and performs the real award, called alongside the pentagram
//!   award drain in `tick_item_use_completion.rs`.

use crate::drvlib::offset2dx;
use crate::world::*;

/// C `TICKS * 5` (`palace.c:612`): dialogue-line throttle.
const ISLENA_TALK_COOLDOWN_TICKS: u64 = 5;
/// C `TICKS * 30` (`palace.c:668`): the "two different attackers" window
/// that triggers the full-heal "Power of Two" defense.
const ISLENA_HURT_WINDOW_TICKS: u64 = 30;
/// C `TICKS * 15` (`palace.c:672,697,713`): re-announcement cooldown for
/// every "I call on thee..." heal message.
const ISLENA_POWER_MSG_COOLDOWN_TICKS: u64 = 15;
/// C `1500 * POWERSCALE` (`palace.c:750`): the revenge damage a repeat
/// killer takes.
const ISLENA_REVENGE_DAMAGE: i32 = 1500 * crate::entity::POWERSCALE;

/// Per-player facts [`World::process_islena_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IslenaPlayerFacts {
    /// `PlayerRuntime::islena_state`.
    pub islena_state: i32,
}

/// A side effect [`World::process_islena_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IslenaOutcomeEvent {
    /// Write the new `islena_state` back (dialogue advance, or the
    /// `NT_GOTHIT`/`NT_SPELL(V_FREEZE)` branch pinning it to `10`).
    UpdateState {
        player_id: CharacterId,
        new_state: i32,
    },
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_PALACEISLENA`
    /// characters (C `ch_driver`'s `CDR_PALACEISLENA` case, `palace.c:776-
    /// 778`).
    pub fn process_islena_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, IslenaPlayerFacts>,
        area_id: u16,
    ) -> Vec<IslenaOutcomeEvent> {
        let islena_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_PALACEISLENA
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for islena_id in islena_ids {
            self.process_islena_tick(islena_id, player_facts, area_id, &mut events);
        }
        events
    }

    /// C `palace_islena`'s per-tick body (`palace.c:578-739`).
    fn process_islena_tick(
        &mut self,
        islena_id: CharacterId,
        player_facts: &HashMap<CharacterId, IslenaPlayerFacts>,
        area_id: u16,
        events: &mut Vec<IslenaOutcomeEvent>,
    ) -> bool {
        let mut data = match self
            .characters
            .get(&islena_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Islena(data)) => data,
            _ => IslenaDriverData::default(),
        };

        let mut washit = false;
        let mut talkdir: Option<Direction> = None;

        let messages = self
            .characters
            .get_mut(&islena_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_CHAR if message.dat1 > 0 => {
                    self.islena_handle_char_message(
                        islena_id,
                        message,
                        player_facts,
                        &mut data,
                        &mut talkdir,
                        events,
                    );
                }
                NT_TEXT => {
                    if let (Some(text), speaker_id) = (
                        message.text.as_deref(),
                        CharacterId(message.dat3.max(0) as u32),
                    ) {
                        self.apply_tabunga_text_notification(islena_id, speaker_id, text);
                    }
                }
                NT_GOTHIT | NT_SPELL if message.dat1 > 0 => {
                    // C `((msg->type == NT_GOTHIT) || (msg->type ==
                    // NT_SPELL && msg->dat2 == V_FREEZE)) && (co =
                    // msg->dat1)` (`palace.c:659`).
                    if message.message_type == NT_SPELL
                        && message.dat2 != CharacterValue::Freeze as i32
                    {
                        continue;
                    }
                    let attacker_id = CharacterId(message.dat1 as u32);
                    let Some(attacker) = self.characters.get(&attacker_id).cloned() else {
                        continue;
                    };
                    if !attacker.flags.contains(CharacterFlags::PLAYER) {
                        continue;
                    }
                    washit = true;
                    // C `if (ppd->islena_state < 10) say(...); ppd->
                    // islena_state = 10;` (`palace.c:663-666`).
                    let current_state = player_facts
                        .get(&attacker_id)
                        .map(|facts| facts.islena_state)
                        .unwrap_or(0);
                    if current_state < 10 {
                        self.npc_say(
                            islena_id,
                            "Thou wilt not hear? So be it. This is thy death!",
                        );
                    }
                    events.push(IslenaOutcomeEvent::UpdateState {
                        player_id: attacker_id,
                        new_state: 10,
                    });
                    // C `palace.c:668-680`: the "Power of Two" full heal
                    // when a second, different attacker hits within 30
                    // ticks of the last recorded hit.
                    if self.tick.0.saturating_sub(data.last_hurt_time)
                        > ISLENA_HURT_WINDOW_TICKS * TICKS_PER_SECOND
                        || data.last_hurt_by == Some(attacker_id)
                    {
                        data.last_hurt_time = self.tick.0;
                        data.last_hurt_by = Some(attacker_id);
                    } else {
                        self.islena_power_heal(
                            islena_id,
                            &mut data,
                            "I call on thee, the Power of Two! Save me from this treacherous attack!",
                        );
                    }
                    // C `standard_message_driver`'s own unconditional
                    // `NT_GOTHIT` self-defense (see module doc comment) -
                    // `NT_SPELL` freeze messages don't reach that switch
                    // case in C, but since this port's single-victim model
                    // has no separate spell-message path, treat both the
                    // same way here (never observably different: a
                    // freezer is always a valid enemy candidate too).
                    if let Some(islena) = self.characters.get(&islena_id).cloned() {
                        if islena.group != attacker.group
                            && can_attack(&islena, &attacker, &self.map)
                        {
                            data.victim = Some(attacker_id);
                        }
                    }
                }
                _ => {}
            }
        }

        // C `fight_driver_update(cn)`.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&islena_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((islena, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&islena, &victim, &self.map, self.date.daylight) {
                        data.victim_visible = true;
                        data.victim_last_x = victim.x;
                        data.victim_last_y = victim.y;
                    } else {
                        data.victim_visible = false;
                    }
                }
                _ => {
                    data.victim = None;
                    data.victim_visible = false;
                }
            }
        }

        if let Some(character) = self.characters.get_mut(&islena_id) {
            character.driver_state = Some(CharacterDriverState::Islena(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) { ...; return; }`
        // (`palace.c:694-707`).
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(islena_id, victim_id, area_id) {
                    let action_idle = self
                        .characters
                        .get(&islena_id)
                        .is_some_and(|islena| islena.action == action::IDLE);
                    if washit && action_idle {
                        let mut data = match self
                            .characters
                            .get(&islena_id)
                            .and_then(|character| character.driver_state.clone())
                        {
                            Some(CharacterDriverState::Islena(data)) => data,
                            _ => IslenaDriverData::default(),
                        };
                        self.islena_power_heal(
                            islena_id,
                            &mut data,
                            "I call on thee, the Power of none! Save me from this treacherous attack!",
                        );
                        if let Some(character) = self.characters.get_mut(&islena_id) {
                            character.driver_state = Some(CharacterDriverState::Islena(data));
                        }
                    }
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`.
            let arrived = self.characters.get(&islena_id).is_some_and(|islena| {
                islena.x.abs_diff(data.victim_last_x) < 2
                    && islena.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                if let Some(CharacterDriverState::Islena(state)) = self
                    .characters
                    .get_mut(&islena_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    state.victim = None;
                }
            } else if self.secure_move_driver(
                islena_id,
                data.victim_last_x,
                data.victim_last_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            ) {
                return true;
            }
        }

        // C `palace.c:712-721`: unconditional re-heal fallback when hit
        // but no fight action was taken this tick.
        if washit {
            let mut data = match self
                .characters
                .get(&islena_id)
                .and_then(|character| character.driver_state.clone())
            {
                Some(CharacterDriverState::Islena(data)) => data,
                _ => IslenaDriverData::default(),
            };
            self.islena_power_heal(
                islena_id,
                &mut data,
                "I call on thee, the Power of None! Save me from this treacherous attack!",
            );
            if let Some(character) = self.characters.get_mut(&islena_id) {
                character.driver_state = Some(CharacterDriverState::Islena(data));
            }
        }

        // C `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)` (`palace.c:723`): C's `ret`/`lastact` params are the
        // caller's own last-action outcome, which this per-message-loop
        // port has no equivalent of (never observably different in
        // practice - every other ported NPC that calls `secure_move_
        // driver` passes `0, 0` too, see `world::asturin`'s own module doc
        // comment).
        let Some((post_x, post_y)) = self
            .characters
            .get(&islena_id)
            .map(|islena| (islena.rest_x, islena.rest_y))
        else {
            return false;
        };
        if self.secure_move_driver(
            islena_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `if (regenerate_driver(cn)) return;` / `if (spell_self_
        // driver(cn)) return;` (`palace.c:727-732`).
        if self.regenerate_simple_baddy(islena_id) {
            return true;
        }
        if self.spell_self_simple_baddy(islena_id) {
            return true;
        }

        if let Some(direction) = talkdir {
            if let Some(islena) = self.characters.get_mut(&islena_id) {
                let _ = turn(islena, direction as u8);
            }
        }

        // C `do_idle(cn, TICKS);` (`palace.c:738`).
        self.idle_simple_baddy(islena_id)
    }

    /// C `palace_islena`'s `NT_CHAR` branch (`palace.c:599-649`): the
    /// four-line greeting/warning dialogue, or an instant enemy-add once
    /// `islena_state >= 10`.
    fn islena_handle_char_message(
        &mut self,
        islena_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, IslenaPlayerFacts>,
        data: &mut IslenaDriverData,
        talkdir: &mut Option<Direction>,
        events: &mut Vec<IslenaOutcomeEvent>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(islena) = self.characters.get(&islena_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if ((ch[co].flags & CF_PLAYER) && char_see_char(cn, co) && ...)`
        // (`palace.c:602-603`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if !char_see_char(&islena, &player, &self.map, self.date.daylight) {
            return;
        }
        let state = player_facts
            .get(&player_id)
            .map(|facts| facts.islena_state)
            .unwrap_or(0);

        // C `if (ppd->islena_state >= 10) { fight_driver_add_enemy(cn, co,
        // 1, 1); remove_message(cn, msg); continue; }` (`palace.c:606-610`).
        if state >= 10 {
            data.victim = Some(player_id);
            return;
        }

        // C `if (ticker < dat->last_talk + TICKS * 5) { remove_message(cn,
        // msg); continue; }` (`palace.c:612-615`).
        if self.tick.0
            < data
                .last_talk
                .saturating_add(ISLENA_TALK_COOLDOWN_TICKS * TICKS_PER_SECOND)
        {
            return;
        }

        let mut didsay = true;
        match state {
            0 => self.npc_say(
                islena_id,
                &format!(
                    "Ah, {}. I have heard of thee. Thou hast quite a reputation among my minions.",
                    player.name
                ),
            ),
            1 => self.npc_say(
                islena_id,
                &format!(
                    "Thou hast been led by the nose by Ishtar, {}. I was quite content living here among my creatures, but Ishtar had to stir up hatred among the human population against me.",
                    player.name
                ),
            ),
            2 => self.npc_say(
                islena_id,
                "It was never my wish to continue the war of the last eon. So why don't we set the enmity aside? I will forgive thee all the trouble thou hast caused me.",
            ),
            3 => self.npc_say(
                islena_id,
                &format!(
                    "None of us must die today if thou wilt just leave me be, {}.",
                    player.name
                ),
            ),
            _ => {
                didsay = false;
                false
            }
        };

        if didsay {
            data.last_talk = self.tick.0;
            *talkdir = offset2dx(
                i32::from(islena.x),
                i32::from(islena.y),
                i32::from(player.x),
                i32::from(player.y),
            );
            events.push(IslenaOutcomeEvent::UpdateState {
                player_id,
                new_state: state + 1,
            });
        }
    }

    /// C's shared "full-heal + throttled announcement" block, inlined at
    /// three call sites in `palace_islena` (`palace.c:672-679,697-704,713-
    /// 720`) with only the message text differing.
    fn islena_power_heal(
        &mut self,
        islena_id: CharacterId,
        data: &mut IslenaDriverData,
        message: &str,
    ) {
        if self.tick.0.saturating_sub(data.last_power_msg)
            > ISLENA_POWER_MSG_COOLDOWN_TICKS * TICKS_PER_SECOND
        {
            self.npc_say(islena_id, message);
        }
        if let Some(islena) = self.characters.get_mut(&islena_id) {
            islena.hp = character_value(islena, CharacterValue::Hp) * POWERSCALE;
            islena.mana = character_value(islena, CharacterValue::Mana) * POWERSCALE;
            islena.endurance = character_value(islena, CharacterValue::Endurance) * POWERSCALE;
            islena.lifeshield = character_value(islena, CharacterValue::MagicShield) * POWERSCALE;
        }
        data.last_power_msg = self.tick.0;
    }

    /// C `islena_dead` (`palace.c:741-767`), dispatched from `ch_died_
    /// driver`'s `CDR_PALACEISLENA` case - called by
    /// [`World::kill_character_followup`] for every `CDR_PALACEISLENA`
    /// death that has a killer (`if (!co) return;`).
    pub(crate) fn apply_islena_death(&mut self, islena_id: CharacterId, killer_id: CharacterId) {
        let Some(killer) = self.characters.get(&killer_id).cloned() else {
            return;
        };

        if killer.flags.contains(CharacterFlags::WON) {
            // C `palace.c:748-750`: already slew her once before - this
            // is a revenge hit, not a fresh win.
            self.npc_say(islena_id, "Why again? Why? Thou shalt die with me!");
            self.apply_legacy_hurt(killer_id, Some(islena_id), ISLENA_REVENGE_DAMAGE, 1, 95, 95);
            return;
        }

        if let Some(killer_mut) = self.characters.get_mut(&killer_id) {
            killer_mut.flags.insert(CharacterFlags::WON);
        }
        // C `reset_name(co)` (`palace.c:753`) - not ported, see module doc
        // comment.
        self.npc_say(islena_id, "So it must end? Why me?");

        let title = islena_sirname(&killer);
        self.queue_system_text(
            killer_id,
            format!(
                "From now on, thou shalt be known as {title} {}. Thou hast slain Islena and the purpose of thy days is fulfilled. All shall admire thine persistence, braveness and power. But still, some doubts remain. How can a mere human succeed were Ishtar failed?",
                killer.name
            ),
        );
        self.queue_islena_grats(&killer.name, title);

        if killer.flags.contains(CharacterFlags::PLAYER) {
            self.pending_islena_ladykiller_awards.push(killer_id);
        }
    }

    /// C `sprintf(buf, "0000000000" COL_MAUVE "Grats: %s is a %s now!",
    /// ch[co].name, Sirname(co)); server_chat(6, buf);` (`palace.c:761-
    /// 762`).
    fn queue_islena_grats(&mut self, name: &str, title: &str) {
        let mut message = b"0000000000".to_vec();
        message.extend_from_slice(crate::text::COL_MAUVE);
        message.extend_from_slice(format!("Grats: {name} is a {title} now!").as_bytes());
        self.queue_channel_broadcast(6, message);
    }

    /// Drains the `CharacterId`s queued by [`World::apply_islena_death`].
    /// Call once per tick alongside `ugaris-server`'s achievement-award
    /// drains - see the module doc comment.
    pub fn drain_pending_islena_ladykiller_awards(&mut self) -> Vec<CharacterId> {
        std::mem::take(&mut self.pending_islena_ladykiller_awards)
    }
}

/// C `Sirname(cn)` (`src/system/tool.c:1538-1546`).
fn islena_sirname(character: &Character) -> &'static str {
    if character.flags.contains(CharacterFlags::MALE) {
        "Sir"
    } else if character.flags.contains(CharacterFlags::FEMALE) {
        "Lady"
    } else {
        "Neuter"
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_PALACEISLENA;

/// C `struct islena_data` (`src/area/11/palace.c:570-576`), plus this
/// port's own single-victim self-defense tracking (see module doc
/// comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IslenaDriverData {
    pub last_talk: u64,
    pub last_hurt_time: u64,
    pub last_hurt_by: Option<CharacterId>,
    pub last_power_msg: u64,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}
