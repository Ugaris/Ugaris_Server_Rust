//! Imp NPC (`CDR_FORESTIMP`), the treasure-hinting forest sprite who
//! kicks off the bear-hunt quest chain (`QLOG` 22, "Impish Bear Hunt")
//! and drops hints for the hermit's spider-queen quest (`QLOG` 24).
//!
//! Ports `src/area/16/forest.c::imp_driver` (`:212-420`). Unlike `world::
//! npc::area16::william`/`world::npc::area16::hermit`, the imp never
//! calls `analyse_text_driver` (no `NT_TEXT` handling at all in C) and
//! has two extra mechanics no other area-16 NPC has:
//! - An ambient invisibility toggle (C's `dat->mode`/`dat->backtime`):
//!   the imp is normally `CF_INVISIBLE` and only reveals itself for a
//!   few seconds after speaking to someone or healing a hurt player.
//! - A self-defense-adjacent heal reflex on `NT_SEEHIT`: a 1-in-4 chance
//!   to heal any player it sees drop below half HP.
//!
//! Follows the same `World`/`PlayerRuntime` split established by `world::
//! npc::area3::astro2`: the caller supplies a per-player fact snapshot
//! ([`ForestImpPlayerFacts`]) up front and applies the returned
//! [`ForestImpOutcomeEvent`]s afterwards, since `area3_ppd.imp_state`/
//! `imp_kills`/`hermit_state`/`william_state` (borrowed from `src/area/3/
//! area3.h` - C's own comment: "note: the ppd is borrowed from area3 -
//! the missions interact...") live on `crate::player::PlayerRuntime`, not
//! `World`.
//!
//! `case 7`/`case 8` and `case 9`/`case 10`'s C `switch` fallthroughs
//! (`forest.c:309-343`) are reproduced directly as nested `if`s inside
//! the `7`/`9` match arms - see [`Self::forest_imp_handle_char_message`]'s
//! inline comments for the exact mapping. `imp_kills` (`forest.c:288`,
//! incremented by the `CDR_FORESTMONSTER` sprite-306 death hook) and the
//! hardkill-weapon-forging item mutation the hint text in `case 8`
//! reads (`it[in].drdata[37]`, C's `monster_dead`/`forest.c:846-852`) are
//! ported separately - see `World::apply_forest_monster_death_driver`
//! and `ugaris-server`'s `apply_forest_monster_death_from_hurt_event`.

use std::collections::HashMap;

use crate::character_driver::CDR_FORESTIMP;
use crate::drvlib::offset2dx;
use crate::world::hurt::IID_HARDKILL;
use crate::world::*;

/// C `char_dist(cn, co) > 20` (`forest.c:261`).
const IMP_GREET_DISTANCE: i32 = 20;
/// C `TICKS * 5` (`forest.c:244`).
const IMP_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`forest.c:249`).
const IMP_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 8` (`forest.c:352`): how long the imp stays visible after
/// speaking.
const IMP_TALK_VISIBLE_TICKS: u64 = TICKS_PER_SECOND * 8;
/// C `TICKS * 3` (`forest.c:374`): how long the imp stays visible after
/// healing someone.
const IMP_HEAL_VISIBLE_TICKS: u64 = TICKS_PER_SECOND * 3;
/// C `TICKS * 30` (`forest.c:413`): idle "go home" threshold.
const IMP_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `it[in].drdata[37] < 38` (`forest.c:316`): the ritual-progress
/// threshold below which the imp keeps hinting about the hardkill
/// weapon. Note this differs from `world::npc::area3::clara`'s own `>=
/// 36` threshold for the same field - a genuine C quirk, not a typo.
const IMP_HARDKILL_HINT_THRESHOLD: u8 = 38;

/// C `struct imp_driver_data` (`forest.c:204-209`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ForestImpDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
    pub mode: i32,
    #[serde(default)]
    pub backtime: u64,
}

/// Per-player facts [`World::process_forest_imp_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForestImpPlayerFacts {
    /// `PlayerRuntime::area3_imp_state()`.
    pub imp_state: i32,
    /// `PlayerRuntime::area3_hermit_state()` (`case 5`/`7`/`8`/`9`
    /// gates).
    pub hermit_state: i32,
    /// `questlog_isdone(co, 23)` (`forest.c:298`, `case 5`).
    pub quest23_done: bool,
    /// `has_item(co, IID_HARDKILL)` (`forest.c:316`) - `true` iff any
    /// inventory/cursor slot (not just the worn right hand, unlike
    /// `world::npc::area3::clara`'s own check) carries the hardkill
    /// weapon.
    pub has_hardkill_item: bool,
    /// `it[in].drdata[37]` (`forest.c:316`) - only meaningful when
    /// `has_hardkill_item` is `true`.
    pub hardkill_ritual_progress: u8,
}

/// A side effect [`World::process_forest_imp_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForestImpOutcomeEvent {
    /// Write the new `area3_ppd.imp_state` back.
    UpdateImpState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->imp_kills = 0` (`forest.c:288`, `case 3`).
    ResetImpKills { player_id: CharacterId },
    /// C `ppd->william_state = 3` (`forest.c:295`, `case 4`).
    UpdateWilliamState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_done(co, 22)` (`forest.c:289`, `case 3`).
    QuestDoneBearHunt { player_id: CharacterId },
}

impl World {
    /// C `imp_driver`'s per-tick body (`forest.c:212-420`).
    pub fn process_forest_imp_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ForestImpPlayerFacts>,
        area_id: u16,
    ) -> Vec<ForestImpOutcomeEvent> {
        let imp_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_FORESTIMP
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for imp_id in imp_ids {
            self.process_forest_imp_messages(imp_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_forest_imp_messages(
        &mut self,
        imp_id: CharacterId,
        player_facts: &HashMap<CharacterId, ForestImpPlayerFacts>,
        area_id: u16,
        events: &mut Vec<ForestImpOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::ForestImp(mut data)) = self
            .characters
            .get(&imp_id)
            .and_then(|imp| imp.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&imp_id)
            .map(|imp| std::mem::take(&mut imp.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;
        let mut healed_and_returned = false;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.forest_imp_handle_char_message(
                    imp_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.forest_imp_handle_give_message(imp_id),
                NT_SEEHIT => {
                    // C `if (do_heal(cn, co)) { remove_message(cn, msg);
                    // return; }` (`forest.c:376-379`): success ends the
                    // whole per-tick body early, skipping every message
                    // still queued after this one and the idle/movement
                    // tail below.
                    if self.forest_imp_handle_seehit_message(imp_id, &mut data, message) {
                        healed_and_returned = true;
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(imp) = self.characters.get_mut(&imp_id) {
            imp.driver_state = Some(CharacterDriverState::ForestImp(data));
        }
        if healed_and_returned {
            return;
        }

        // C `if (dat->mode == 1 && ticker > dat->backtime) dat->mode =
        // 0;` (`forest.c:389-391`).
        let tick = self.tick.0;
        let mode = match self
            .characters
            .get(&imp_id)
            .and_then(|imp| imp.driver_state.as_ref())
        {
            Some(CharacterDriverState::ForestImp(data)) => {
                let mut mode = data.mode;
                if mode == 1 && tick > data.backtime {
                    mode = 0;
                    if let Some(imp) = self.characters.get_mut(&imp_id) {
                        if let Some(CharacterDriverState::ForestImp(data)) =
                            imp.driver_state.as_mut()
                        {
                            data.mode = 0;
                        }
                    }
                }
                mode
            }
            _ => return,
        };

        // C `if (dat->mode == 0) { ... CF_INVISIBLE on ... } else if
        // (dat->mode == 1) { ... CF_INVISIBLE off ... }` (`forest.c:393-
        // 403`).
        if let Some(imp) = self.characters.get(&imp_id) {
            let (x, y) = (imp.x, imp.y);
            let is_invisible = imp.flags.contains(CharacterFlags::INVISIBLE);
            if mode == 0 && !is_invisible {
                if let Some(imp_mut) = self.characters.get_mut(&imp_id) {
                    imp_mut.flags.insert(CharacterFlags::INVISIBLE);
                }
                self.mark_dirty_sector(usize::from(x), usize::from(y));
            } else if mode == 1 && is_invisible {
                if let Some(imp_mut) = self.characters.get_mut(&imp_id) {
                    imp_mut.flags.remove(CharacterFlags::INVISIBLE);
                }
                self.mark_dirty_sector(usize::from(x), usize::from(y));
            }
        }

        // C `if (talkdir) turn(cn, talkdir);` (`forest.c:405-407`).
        if let (Some(imp), Some((tx, ty))) = (self.characters.get(&imp_id).cloned(), face_target) {
            if let Some(direction) = offset2dx(i32::from(imp.x), i32::from(imp.y), tx, ty) {
                if let Some(imp_mut) = self.characters.get_mut(&imp_id) {
                    let _ = turn(imp_mut, direction as u8);
                }
            }
        }

        // C `if (spell_self_driver(cn)) return;` (`forest.c:409-411`):
        // `spell_self_driver` (self-buff casting) has no Rust port
        // anywhere in this codebase yet - same documented gap as every
        // other NPC driver's identical comment (e.g. `world::npc::area1::
        // asturin`'s own `secure_move_driver`-preceding comment).

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`forest.c:413-417`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other area-3-family driver uses.
        let last_talk = match self
            .characters
            .get(&imp_id)
            .and_then(|imp| imp.driver_state.as_ref())
        {
            Some(CharacterDriverState::ForestImp(data)) => data.last_talk,
            _ => return,
        };
        if last_talk + IMP_RETURN_TO_POST_TICKS < tick {
            let Some(imp) = self.characters.get(&imp_id) else {
                return;
            };
            let (post_x, post_y) = (imp.rest_x, imp.rest_y);
            self.secure_move_driver(
                imp_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `imp_driver`'s `NT_CHAR` branch (`forest.c:227-355`).
    #[allow(clippy::too_many_lines)]
    fn forest_imp_handle_char_message(
        &mut self,
        imp_id: CharacterId,
        data: &mut ForestImpDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ForestImpPlayerFacts>,
        events: &mut Vec<ForestImpOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(imp) = self.characters.get(&imp_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`forest.c:231-234`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`forest.c:237-240`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`forest.c:243-246`).
        if tick < data.last_talk + IMP_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`forest.c:248-252`).
        if tick < data.last_talk + IMP_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`forest.c:254-258`).
        if imp_id == player_id || !char_see_char(&imp, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 20) continue;` (`forest.c:260-263`).
        if char_dist(&imp, &player) > IMP_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        // C `switch (ppd->imp_state) { ... }` (`forest.c:270-346`).
        match facts.imp_state {
            0 => {
                self.npc_quiet_say(
                    imp_id,
                    "A human. Now this is interesting. What could a human be doing here?",
                );
                events.push(ForestImpOutcomeEvent::UpdateImpState {
                    player_id,
                    new_state: 1,
                });
                didsay = true;
            }
            1 => {
                self.npc_quiet_say(
                    imp_id,
                    &format!(
                        "Should I tell {} about the treasure? Ah, let's wait and see.",
                        himname(&player)
                    ),
                );
                events.push(ForestImpOutcomeEvent::UpdateImpState {
                    player_id,
                    new_state: 2,
                });
                didsay = true;
            }
            // `imp_state == 2`: waiting for `imp_kills` to reach 21 (the
            // `CDR_FORESTMONSTER` death hook drives this).
            3 => {
                self.npc_quiet_say(
                    imp_id,
                    &format!(
                        "Nicely done, {}. Thou might not be bright, but at least thou knowest how to fight.",
                        player.name
                    ),
                );
                events.push(ForestImpOutcomeEvent::UpdateImpState {
                    player_id,
                    new_state: 4,
                });
                events.push(ForestImpOutcomeEvent::ResetImpKills { player_id });
                events.push(ForestImpOutcomeEvent::QuestDoneBearHunt { player_id });
                didsay = true;
            }
            4 => {
                self.npc_quiet_say(imp_id, "Now do be a dear and run back to William.");
                events.push(ForestImpOutcomeEvent::UpdateImpState {
                    player_id,
                    new_state: 5,
                });
                events.push(ForestImpOutcomeEvent::UpdateWilliamState {
                    player_id,
                    new_state: 3,
                });
                didsay = true;
            }
            5 => {
                // C `if (questlog_isdone(co, 23)) { ppd->imp_state = 11;
                // break; }` (`forest.c:298-301`) - no `say`, no `didsay`.
                if facts.quest23_done {
                    events.push(ForestImpOutcomeEvent::UpdateImpState {
                        player_id,
                        new_state: 11,
                    });
                }
            }
            6 => {
                self.npc_quiet_say(
                    imp_id,
                    "Hullo human! There is more to be done here I know thine worth. Find him who is old and in need of thy help.",
                );
                events.push(ForestImpOutcomeEvent::UpdateImpState {
                    player_id,
                    new_state: 7,
                });
                didsay = true;
            }
            7 => {
                // C `case 7: if (ppd->hermit_state > 3) { ppd->imp_state++;
                // // fall thru... } else { break; } case 8: ...`
                // (`forest.c:309-328`) - the fallthrough is reproduced
                // directly here: on `hermit_state > 3` we immediately run
                // `case 8`'s own body against the *new* state instead of
                // waiting for the next `NT_CHAR` message.
                if facts.hermit_state > 3 {
                    if facts.hermit_state == 4
                        && (!facts.has_hardkill_item
                            || facts.hardkill_ritual_progress < IMP_HARDKILL_HINT_THRESHOLD)
                    {
                        if facts.has_hardkill_item {
                            self.npc_quiet_say(
                                imp_id,
                                "Listen, human, for this might save thine life: The spider queen is beyond the strength of thine holy weapon. Thou needst find another stone circle. Find the skeleton ruin and go eastward.",
                            );
                        } else {
                            self.npc_quiet_say(
                                imp_id,
                                "Listen, human, for this might save thine life: Thou needst a holy weapon, otherwise thine task will remain unfulfilled.",
                            );
                        }
                        didsay = true;
                    }
                    events.push(ForestImpOutcomeEvent::UpdateImpState {
                        player_id,
                        new_state: 9,
                    });
                }
            }
            9 => {
                // C `case 9: if (ppd->hermit_state > 4) { ppd->imp_state++;
                // // fall thru... } else { break; } case 10: say(...);
                // ppd->imp_state++; didsay = 1; break;` (`forest.c:329-
                // 343`).
                if facts.hermit_state > 4 {
                    self.npc_quiet_say(
                        imp_id,
                        &format!(
                            "Thou art truly worthy, dear human called {}. Now, I shall tell thee how to find the treasure. Find the southernmost stone where the single skeleton lurks. Dig a hole and thou shalt find the key to a treasure.",
                            player.name
                        ),
                    );
                    events.push(ForestImpOutcomeEvent::UpdateImpState {
                        player_id,
                        new_state: 11,
                    });
                    didsay = true;
                }
            }
            // `imp_state == 8`/`10` (transient, never observed at the
            // start of a tick - `case 8`/`case 10` are only ever reached
            // via the `case 7`/`case 9` fallthroughs above, which always
            // finish past them in the same pass) or `11` (done): no-op.
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; dat->mode = 1; dat->backtime = ticker
        // + TICKS*8; }` (`forest.c:347-353`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
            data.mode = 1;
            data.backtime = tick + IMP_TALK_VISIBLE_TICKS;
        }
    }

    /// C `imp_driver`'s `NT_GIVE` branch (`forest.c:357-366`): the imp
    /// has no turn-in item at all - any offered item is silently
    /// destroyed.
    fn forest_imp_handle_give_message(&mut self, imp_id: CharacterId) {
        let Some(item_id) = self
            .characters
            .get_mut(&imp_id)
            .and_then(|imp| imp.cursor_item.take())
        else {
            return;
        };
        self.destroy_item(item_id);
    }

    /// C `imp_driver`'s `NT_SEEHIT` branch (`forest.c:368-381`): a
    /// 1-in-4 chance to reveal itself and heal a player it just saw drop
    /// below half HP. Returns `true` iff `do_heal` succeeded, matching
    /// C's `if (do_heal(cn, co)) { remove_message(cn, msg); return; }`
    /// early-return.
    fn forest_imp_handle_seehit_message(
        &mut self,
        imp_id: CharacterId,
        data: &mut ForestImpDriverData,
        message: &CharacterDriverMessage,
    ) -> bool {
        let victim_id = CharacterId(message.dat2.max(0) as u32);
        if message.dat2 <= 0 {
            return false;
        }
        let Some(victim) = self.characters.get(&victim_id).cloned() else {
            return false;
        };
        // C `if (co && (ch[co].flags & CF_PLAYER) && ch[co].hp <
        // ch[co].value[1][V_HP] * POWERSCALE / 2 && !RANDOM(4))`
        // (`forest.c:370`).
        if !victim.flags.contains(CharacterFlags::PLAYER) {
            return false;
        }
        // C `ch[co].value[1][V_HP]` (`forest.c:370`) - the *modified*
        // max-hp row, distinct from `do_heal`'s own `values[0]` (base)
        // missing-hp calculation.
        let half_hp = character_value_present(&victim, CharacterValue::Hp) * POWERSCALE / 2;
        if victim.hp >= half_hp {
            return false;
        }
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, 4) != 0 {
            return false;
        }

        // C `ch[cn].flags &= ~CF_INVISIBLE; set_sector(...); dat->mode =
        // 1; dat->backtime = ticker + TICKS*3; say(cn, "Ooh. Don't die,
        // dear human.");` (`forest.c:371-375`).
        let (x, y) = (self.characters.get(&imp_id).map(|imp| (imp.x, imp.y))).unwrap_or_default();
        if let Some(imp_mut) = self.characters.get_mut(&imp_id) {
            imp_mut.flags.remove(CharacterFlags::INVISIBLE);
        }
        self.mark_dirty_sector(usize::from(x), usize::from(y));
        data.mode = 1;
        data.backtime = self.tick.0 + IMP_HEAL_VISIBLE_TICKS;
        self.npc_quiet_say(imp_id, "Ooh. Don't die, dear human.");

        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(target) = self.characters.get(&victim_id).cloned() else {
            return false;
        };
        self.characters.get_mut(&imp_id).is_some_and(|caster| {
            do_heal(caster, &target, None, &self.map, weather_movement_percent).is_ok()
        })
    }
}

/// C `himname(int cn)` (`src/system/tool.c:1528-1535`).
fn himname(character: &Character) -> &'static str {
    if character.flags.contains(CharacterFlags::MALE) {
        "him"
    } else if character.flags.contains(CharacterFlags::FEMALE) {
        "her"
    } else {
        "it"
    }
}

/// Hardkill-weapon fact lookup helper for callers building
/// [`ForestImpPlayerFacts`]: C `has_item(co, IID_HARDKILL)` (`forest.c:
/// 316`), which - unlike `world::npc::area3::clara::clara_hardkill_
/// weapon_facts`'s own worn-right-hand-only check - scans every
/// inventory/cursor slot (C `has_item`'s own `for (n = 0; n <
/// INVENTORYSIZE; n++)` loop plus its trailing `ch[cn].citem` check).
/// Returns `(has_hardkill_item, ritual_progress)`.
pub fn imp_hardkill_weapon_facts(world: &World, player_id: CharacterId) -> (bool, u8) {
    let Some(character) = world.characters.get(&player_id) else {
        return (false, 0);
    };
    let item_id = character
        .inventory
        .iter()
        .flatten()
        .copied()
        .find(|item_id| {
            world
                .items
                .get(item_id)
                .is_some_and(|item| item.template_id == IID_HARDKILL)
        })
        .or_else(|| {
            character.cursor_item.filter(|item_id| {
                world
                    .items
                    .get(item_id)
                    .is_some_and(|item| item.template_id == IID_HARDKILL)
            })
        });
    let Some(item_id) = item_id else {
        return (false, 0);
    };
    let progress = world
        .items
        .get(&item_id)
        .and_then(|item| item.driver_data.get(37).copied())
        .unwrap_or(0);
    (true, progress)
}
