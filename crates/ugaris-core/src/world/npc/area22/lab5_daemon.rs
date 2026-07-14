//! Lab 5 demon fight driver (`CDR_LAB5DAEMON`), shared by every servant/
//! master/gunned demon in the labyrinth's three demon rooms
//! (`lab5_one_servant`/`_master`/`_gunned`, `lab5_two_*`, `lab5_three_*`,
//! `zones/22/lab5.chr`).
//!
//! Ports `src/area/22/lab5.c::lab5_daemon_driver` (`:861-943`) plus its
//! `NT_CREATE` arg parser `lab5_daemon_driver_parse` (`:840-859`). Three
//! roles share this one driver, selected by each template's own
//! `arg="type=N;"`:
//! - `type=0` (servant/trash demons): plain single-victim self-defense,
//!   effectively always "aggressive" (see below).
//! - `type=1` (master demons): `CF_IMMORTAL` unless a currently-visible
//!   player wields the sacred `IID_LAB5_WEAPON` in `WN_RHAND` - this is
//!   what makes the head trophies `world::npc::area22::lab5_seyan` wants
//!   killable at all.
//! - `type=2` (gunned demons): never becomes "aggressive" via the timer
//!   (C sets `attackstart = 2147483647`, i.e. never), but unconditionally
//!   adds any visible player north of `namecoordy[0] + 25` as an enemy -
//!   this is the corridor "sniper" variant.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification as every other ported
//!   NPC (see `world::asturin`'s module doc comment): C's generic 10-slot
//!   `struct fight_driver_data` is narrowed to a single tracked `victim`.
//! - `namecoordy[0]` (`lab5.c:107`, the gunned-demon aggro-line y
//!   coordinate) now reads `World::lab5_namecoord(0)`, live-updated by
//!   the mage's own `NT_CREATE` (`world::npc::area22::lab5_mage`) - see
//!   that module's doc comment. `IDR_LAB5_ITEM`'s nameplate branch
//!   (indices 1-3, not needed by this file) is still not ported.
//! - `lab5_daemon_driver_parse`'s `type=` arg parse itself runs at spawn
//!   time ([`apply_lab5_daemon_create_message`], `ZoneLoader` has no
//!   ticker to compute `attackstart` from); the `attackstart` tail that
//!   does need the ticker still runs from the real `NT_CREATE` message on
//!   this character's first live tick instead, same "map/tick-dependent
//!   remainder deferred to first tick" precedent as `CDR_LABGNOMEDRIVER`'s
//!   own `NT_CREATE` split. That tail now does C's real `dat->attackstart
//!   += ticker` (not a flat overwrite) so `world::npc::area22::
//!   lab5_mage::ritual_demon_spawns`' pre-set relative `attackstart`
//!   (`ritual_create_char`'s `attackstart * TICKS`, `lab5.c:166`) survives
//!   through to this tick-dependent conversion to an absolute deadline,
//!   exactly like the always-`0`-pre-set zone-spawned case already did.

use crate::character_driver::next_legacy_name_value;
use crate::item_driver::IID_LAB5_WEAPON;
use crate::world::*;

/// C `WN_RHAND` (`src/common/item_id.h`/worn-slot layout): right-hand
/// weapon slot, same constant `world::npc::area8::fdemon_army` already
/// established.
const WN_RHAND: usize = 6;

impl World {
    /// C `lab5_daemon_driver`'s per-tick body (`lab5.c:861-943`).
    pub fn process_lab5_daemon_actions(&mut self, area_id: u16) -> usize {
        let daemon_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAB5DAEMON
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for daemon_id in daemon_ids {
            if self.process_lab5_daemon_tick(daemon_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    fn process_lab5_daemon_tick(&mut self, daemon_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&daemon_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Lab5Daemon(data)) => data,
            _ => Lab5DaemonDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&daemon_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        // C `int imm = -1;` (`lab5.c:863`): `None` = no change this tick.
        let mut imm: Option<bool> = None;

        for message in &messages {
            match message.message_type {
                // C `lab5_daemon_driver_parse`'s ticker-dependent tail
                // (`lab5.c:851-858`), run from the real `NT_CREATE`
                // message - see module doc comment.
                // C `dat->attackstart += ticker;` for `type` 0/1
                // (`lab5.c:852,857`); `type` 2 always resets to "never"
                // (`lab5.c:855`). See module doc comment for why this is
                // an addition, not a flat overwrite.
                NT_CREATE => match data.daemon_type {
                    2 => data.attackstart = u64::MAX,
                    _ => data.attackstart = data.attackstart.saturating_add(self.tick.0),
                },
                // C `lab5.c:879-882`.
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if let Some(text) = message.text.as_deref() {
                        self.apply_tabunga_text_notification(daemon_id, speaker_id, text);
                    }
                }
                NT_CHAR if message.dat1 > 0 => {
                    let seen_id = CharacterId(message.dat1 as u32);
                    let Some((daemon, seen)) = self
                        .characters
                        .get(&daemon_id)
                        .cloned()
                        .zip(self.characters.get(&seen_id).cloned())
                    else {
                        continue;
                    };

                    // C `lab5.c:886-896`: master immortal-toggle tracking
                    // (type==1), else unconditional `imm = 0`.
                    if data.daemon_type == 1 {
                        if seen.flags.contains(CharacterFlags::PLAYER)
                            && char_see_char(&daemon, &seen, &self.map, self.date.daylight)
                        {
                            let has_weapon = seen
                                .inventory
                                .get(WN_RHAND)
                                .copied()
                                .flatten()
                                .and_then(|item_id| self.items.get(&item_id))
                                .is_some_and(|item| item.template_id == IID_LAB5_WEAPON);
                            if !has_weapon {
                                imm = Some(true);
                            } else if imm.is_none() {
                                imm = Some(false);
                            }
                        }
                    } else {
                        imm = Some(false);
                    }

                    // C `lab5.c:898-902`: gunned demon (type==2)
                    // unconditional enemy-add for anyone north of the
                    // aggro line.
                    if data.daemon_type == 2
                        && seen.flags.contains(CharacterFlags::PLAYER)
                        && i32::from(seen.y) < self.lab5_namecoord(0).1 + 25
                        && char_see_char(&daemon, &seen, &self.map, self.date.daylight)
                    {
                        data.victim = Some(seen_id);
                    }

                    // C `standard_message_driver(cn, msg, dat->aggressive,
                    // 1)`'s `NT_CHAR` branch (`drvlib.c:2470-2476`), gated
                    // on this demon's own dynamic `aggressive` flag.
                    if data.aggressive && self.lab5_daemon_is_valid_enemy(daemon_id, seen_id) {
                        data.victim = Some(seen_id);
                    }
                }
                // C `standard_message_driver`'s `NT_SEEHIT` branch
                // (`drvlib.c:2478-2510`), always active (`helper=1`).
                NT_SEEHIT if message.dat1 > 0 && message.dat2 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    let victim_id = CharacterId(message.dat2 as u32);
                    let Some(daemon) = self.characters.get(&daemon_id).cloned() else {
                        continue;
                    };
                    let victim_is_friend = victim_id != daemon_id
                        && self
                            .characters
                            .get(&victim_id)
                            .is_some_and(|victim| victim.group == daemon.group);
                    if victim_is_friend {
                        if self.lab5_daemon_is_valid_enemy(daemon_id, attacker_id) {
                            data.victim = Some(attacker_id);
                        }
                        continue;
                    }
                    let attacker_is_friend = attacker_id != daemon_id
                        && self
                            .characters
                            .get(&attacker_id)
                            .is_some_and(|attacker| attacker.group == daemon.group);
                    if attacker_is_friend && self.lab5_daemon_is_valid_enemy(daemon_id, victim_id) {
                        data.victim = Some(victim_id);
                    }
                }
                // C `standard_message_driver`'s `NT_GOTHIT` branch
                // (`drvlib.c:2512-2538`): unconditional self-defense.
                NT_GOTHIT if message.dat1 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    let Some((daemon, attacker)) = self
                        .characters
                        .get(&daemon_id)
                        .cloned()
                        .zip(self.characters.get(&attacker_id).cloned())
                    else {
                        continue;
                    };
                    if daemon.group != attacker.group && can_attack(&daemon, &attacker, &self.map) {
                        data.victim = Some(attacker_id);
                    }
                }
                _ => {}
            }
        }

        // C `if (dat->aggressive == 0 && ticker > dat->attackstart)
        // dat->aggressive = 1;` (`lab5.c:910-912`).
        if !data.aggressive && self.tick.0 > data.attackstart {
            data.aggressive = true;
        }

        // C `if (imm == 1) ch[cn].flags |= CF_IMMORTAL; else if (imm ==
        // 0) ch[cn].flags &= ~CF_IMMORTAL;` (`lab5.c:915-919`).
        if let Some(daemon) = self.characters.get_mut(&daemon_id) {
            match imm {
                Some(true) => daemon.flags.insert(CharacterFlags::IMMORTAL),
                Some(false) => daemon.flags.remove(CharacterFlags::IMMORTAL),
                None => {}
            }
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&daemon_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((daemon, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&daemon, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&daemon_id) {
            character.driver_state = Some(CharacterDriverState::Lab5Daemon(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`
        // (`lab5.c:923-925`).
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(daemon_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`
            // (`lab5.c:926-928`).
            let arrived = self.characters.get(&daemon_id).is_some_and(|daemon| {
                daemon.x.abs_diff(data.victim_last_x) < 2
                    && daemon.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                data.victim = None;
                if let Some(character) = self.characters.get_mut(&daemon_id) {
                    character.driver_state = Some(CharacterDriverState::Lab5Daemon(data));
                }
            } else if self.secure_move_driver(
                daemon_id,
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

        // C `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return;` (`lab5.c:931-936`).
        if self.regenerate_simple_baddy(daemon_id) {
            return true;
        }
        if self.spell_self_simple_baddy(daemon_id) {
            return true;
        }

        // C `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, dat->dir,
        // ret, lastact)` (`lab5.c:937-939`): `tmpx`/`tmpy` reuse
        // `rest_x`/`rest_y`, same substitution as every other stationary
        // NPC in this file.
        let (post_x, post_y) = self
            .characters
            .get(&daemon_id)
            .map(|daemon| (daemon.rest_x, daemon.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(daemon_id, post_x, post_y, data.dir, 0, 0, area_id) {
            return true;
        }

        // C `do_idle(cn, TICKS / 2);` (`lab5.c:942`).
        self.characters
            .get_mut(&daemon_id)
            .is_some_and(|character| do_idle(character, (TICKS_PER_SECOND / 2) as i32).is_ok())
    }

    /// C `is_valid_enemy(cn, co, -1)` (`src/system/drvlib.c:897-927`).
    fn lab5_daemon_is_valid_enemy(&self, daemon_id: CharacterId, target_id: CharacterId) -> bool {
        if daemon_id == target_id {
            return false;
        }
        let Some(daemon) = self.characters.get(&daemon_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        daemon.group != target.group
            && can_attack(daemon, target, &self.map)
            && char_see_char(daemon, target, &self.map, self.date.daylight)
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct lab5_daemon_data { int attackstart; char aggressive, type,
/// dir, dummy; }` (`lab5.c:98-101`). `dummy` is padding, not ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab5DaemonDriverData {
    pub daemon_type: u8,
    pub dir: u8,
    pub attackstart: u64,
    pub aggressive: bool,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}

impl Default for Lab5DaemonDriverData {
    fn default() -> Self {
        Self {
            daemon_type: 0,
            dir: Direction::Down as u8,
            attackstart: 0,
            aggressive: false,
            victim: None,
            victim_visible: false,
            victim_last_x: 0,
            victim_last_y: 0,
        }
    }
}

/// C `lab5_daemon_driver_parse` (`lab5.c:840-859`): only reads `type=`.
fn parse_lab5_daemon_driver_args(args: &str) -> u8 {
    let mut daemon_type = 0u8;
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        if name == "type" {
            daemon_type = value.parse::<i32>().unwrap_or(0).max(0) as u8;
        }
        rest = next;
    }
    daemon_type
}

/// Spawn-time data-only half of C's `NT_CREATE`-triggered
/// `lab5_daemon_driver_parse` call (`lab5.c:875-877`, parses `type=` and
/// sets `dir` for the gunned case): `ZoneLoader` has no ticker to compute
/// `attackstart` from, so that half runs on the character's first live
/// tick instead - see module doc comment and
/// `World::process_lab5_daemon_tick`'s own `NT_CREATE` handling.
pub fn apply_lab5_daemon_create_message(character: &mut Character, args: Option<&str>) {
    let daemon_type = args
        .filter(|args| !args.is_empty())
        .map(parse_lab5_daemon_driver_args)
        .unwrap_or(0);

    let mut data = Lab5DaemonDriverData {
        daemon_type,
        ..Default::default()
    };
    // C `dat->dir = DX_LEFT;` (`lab5.c:854`, the gunned case).
    if daemon_type == 2 {
        data.dir = Direction::Left as u8;
    }

    character.driver_state = Some(CharacterDriverState::Lab5Daemon(data));
    character.push_driver_message(NT_CREATE, 0, 0, 0);
}
