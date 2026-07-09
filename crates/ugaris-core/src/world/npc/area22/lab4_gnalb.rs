//! Lab 4 gnalb guard/crazy-gnalb NPC (`CDR_LAB4GNALB`).
//!
//! Ports `src/area/22/lab4.c::lab4_gnalb_driver` (`:469-639`) plus its
//! `lab4_gnalb_driver_parse`/`_init` `NT_CREATE` setup (`:429-467`). The
//! same C function backs five roles selected by a zone-file `type=N` arg
//! (`1`=patrol guard, `2`=dead code - see below, `3`=crazy fire-staring
//! gnalb, `4`=mage, `5`=king), but only `type=1` (`lab4_gnalb_patrol`)
//! and `type=3` (`lab4_crazy_gnalb`) are ever actually spawned by
//! `zones/22/lab4.chr` - the king/mage templates that carry the crown/
//! szepter quest items spawn as plain `CDR_SIMPLEBADDY` instead (confirmed
//! by grepping the shipped zone data), so `type=4`/`5` (which the C
//! driver's own movement dispatch treats identically to the unreachable
//! `type=2` case anyway - see the tail `else` branch) are ported for
//! structural completeness but never exercised in game.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification already established
//!   for every other ported NPC (see `world::npc::area22::lab1_gnome`'s
//!   own module doc comment): C's generic 10-slot `struct
//!   fight_driver_data` is narrowed to a single tracked `victim`.
//! - `fight_driver_set_dist`/`fight_driver_set_home` (`lab4.c:458,465,
//!   576`) are not ported - same precedent as every other single-victim
//!   NPC's own module doc comment (a "leash"/aggro-range refinement never
//!   observably different from C in practice with the single-victim
//!   model, since this port has no home-leash concept at all yet).
//! - `msg->type == NT_SEEHIT && dat->type == 2` (`lab4.c:512-551`): a
//!   custom friend-help block that only matters for the never-spawned
//!   `type=2` role (C's own `xlog("CANTBETRUE"...)` in
//!   `lab4_gnalb_driver_init` confirms the author considered `type=2`
//!   itself unreachable, and it leaves `aggressive`/`helper` both `false`
//!   for that role, so even if it *were* reachable this hand-written
//!   block would already be redundant with `standard_message_driver`'s
//!   own generic `NT_SEEHIT` handling once `helper` is set - which this
//!   NPC's `type=1` role reproduces below). Not ported - dead code in the
//!   shipped game, and functionally subsumed by the generic path anyway.
//! - `standard_message_driver`'s `NT_CHAR`/`NT_SEEHIT`/`NT_GOTHIT` cases
//!   (`drvlib.c:2470-2538`), gated by this driver's own `aggressive`/
//!   `helper` fields, are reproduced inline (only observably live for
//!   `type=1`, since `type=3`/`4`/`5` all leave both flags `false`).
//! - `tabunga` (the `CF_GOD` debug stat dump `NT_TEXT` triggers) reuses
//!   the already-ported `World::apply_tabunga_text_notification`.

use crate::character_driver::{next_legacy_name_value, CDR_LAB4GNALB};
use crate::world::*;

/// C `struct gnalb_path gnalb_path[]` (`lab4.c:347-416`), transcribed
/// digit-for-digit including the two dead `{0,0,{0,0,0,0}}` sentinel
/// entries at index `0` and `46` (`dat->path` is a raw index into this
/// table, and C's own nearest-node scan at `lab4_gnalb_driver_init`
/// iterates `1..max_gnalb_path` without skipping index `46`).
const GNALB_PATH: [(u16, u16, [u16; 4]); 64] = [
    (0, 0, [0, 0, 0, 0]),       // 0
    (52, 228, [45, 2, 0, 0]),   // 1
    (56, 231, [1, 3, 0, 0]),    // 2
    (61, 233, [2, 4, 0, 0]),    // 3
    (65, 234, [3, 5, 0, 0]),    // 4
    (68, 236, [4, 6, 0, 0]),    // 5
    (73, 237, [5, 7, 0, 0]),    // 6
    (77, 238, [6, 8, 0, 0]),    // 7
    (81, 239, [7, 9, 0, 0]),    // 8
    (83, 239, [8, 10, 0, 0]),   // 9
    (88, 238, [9, 11, 0, 0]),   // 10
    (89, 236, [10, 12, 0, 0]),  // 11
    (90, 233, [11, 13, 0, 0]),  // 12
    (90, 230, [12, 14, 0, 0]),  // 13
    (89, 227, [13, 15, 0, 0]),  // 14
    (90, 225, [14, 16, 0, 0]),  // 15
    (92, 223, [15, 17, 47, 0]), // 16 -- cross 2 --
    (92, 220, [16, 18, 0, 0]),  // 17
    (93, 217, [17, 19, 0, 0]),  // 18
    (92, 213, [18, 20, 0, 0]),  // 19
    (89, 209, [19, 21, 0, 0]),  // 20
    (87, 206, [20, 22, 0, 0]),  // 21
    (86, 203, [21, 23, 0, 0]),  // 22
    (86, 201, [22, 24, 63, 0]), // 23 -- cross 1 --
    (82, 200, [23, 25, 0, 0]),  // 24
    (78, 199, [24, 26, 0, 0]),  // 25
    (74, 197, [25, 27, 0, 0]),  // 26
    (71, 197, [26, 28, 0, 0]),  // 27
    (69, 199, [27, 29, 0, 0]),  // 28
    (67, 201, [28, 30, 0, 0]),  // 29
    (66, 205, [29, 31, 0, 0]),  // 30
    (67, 207, [30, 32, 0, 0]),  // 31
    (67, 210, [31, 33, 0, 0]),  // 32
    (68, 213, [32, 34, 0, 0]),  // 33
    (67, 216, [33, 35, 0, 0]),  // 34
    (67, 218, [34, 36, 0, 0]),  // 35
    (65, 220, [35, 37, 0, 0]),  // 36
    (62, 218, [36, 38, 0, 0]),  // 37
    (58, 215, [37, 39, 0, 0]),  // 38
    (54, 213, [38, 40, 0, 0]),  // 39
    (51, 213, [39, 41, 0, 0]),  // 40
    (49, 214, [40, 42, 0, 0]),  // 41
    (48, 217, [41, 43, 0, 0]),  // 42
    (48, 219, [42, 44, 0, 0]),  // 43
    (49, 222, [43, 45, 0, 0]),  // 44
    (50, 225, [44, 1, 0, 0]),   // 45
    (0, 0, [0, 0, 0, 0]),       // 46
    (95, 226, [16, 48, 0, 0]),  // 47 -- cross 2 --
    (97, 227, [47, 49, 0, 0]),  // 48
    (99, 229, [48, 50, 0, 0]),  // 49
    (102, 229, [49, 51, 0, 0]), // 50
    (104, 230, [50, 52, 0, 0]), // 51
    (106, 228, [51, 53, 0, 0]), // 52
    (108, 225, [52, 54, 0, 0]), // 53
    (107, 221, [53, 55, 0, 0]), // 54
    (106, 217, [54, 56, 0, 0]), // 55
    (105, 213, [55, 57, 0, 0]), // 56
    (104, 209, [56, 58, 0, 0]), // 57
    (103, 205, [57, 59, 0, 0]), // 58
    (100, 201, [58, 60, 0, 0]), // 59
    (97, 199, [59, 61, 0, 0]),  // 60
    (93, 199, [60, 62, 0, 0]),  // 61
    (90, 200, [61, 63, 0, 0]),  // 62
    (87, 199, [62, 23, 0, 0]),  // 63 -- cross 1 --
];

/// C `struct lab4_gnalb_driver_data` (`lab4.c:420-427`). `dummyA` is
/// never read anywhere in the C driver's body - dead even in C, same
/// precedent as other ported NPCs' own dead fields - so it is not
/// ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab4GnalbDriverData {
    /// C `char type` (`lab4.c:421`): `1`=guard, `2`=dead code, `3`=young/
    /// crazy, `4`=mage, `5`=king. See module doc comment.
    pub gnalb_type: u8,
    pub aggressive: bool,
    pub helper: bool,
    /// C `dat->path`/`dat->lastpath`: indices into [`GNALB_PATH`].
    pub path: u16,
    pub lastpath: u16,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}

impl Default for Lab4GnalbDriverData {
    fn default() -> Self {
        Self {
            gnalb_type: 0,
            aggressive: false,
            helper: false,
            path: 0,
            lastpath: 0,
            victim: None,
            victim_visible: false,
            victim_last_x: 0,
            victim_last_y: 0,
        }
    }
}

/// C `lab4_gnalb_driver_parse` (`lab4.c:429-439`).
fn parse_lab4_gnalb_driver_args(args: &str) -> Lab4GnalbDriverData {
    let mut data = Lab4GnalbDriverData::default();
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        if name == "type" {
            data.gnalb_type = value.parse::<u8>().unwrap_or(0);
        }
        rest = next;
    }
    data
}

/// Spawn-time data-only half of C's `NT_CREATE` handler (parses `type=`
/// only); the map-dependent remainder (nearest-path-node lookup for
/// `type=1`) runs on the character's first live tick instead - see
/// `World::lab4_gnalb_finish_create`, same precedent as
/// `world::npc::area22::lab1_gnome::apply_labgnome_create_message`.
pub fn apply_lab4_gnalb_create_message(character: &mut Character, args: Option<&str>) {
    let data = args
        .filter(|args| !args.is_empty())
        .map(parse_lab4_gnalb_driver_args)
        .unwrap_or_default();
    character.driver_state = Some(CharacterDriverState::Lab4Gnalb(data));
    character.push_driver_message(NT_CREATE, 0, 0, 0);
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_LAB4GNALB`
    /// characters (C `ch_driver`'s `CDR_LAB4GNALB` case, `lab4.c:703-705`).
    pub fn process_lab4_gnalb_actions(&mut self, area_id: u16) -> usize {
        let gnalb_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAB4GNALB
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for gnalb_id in gnalb_ids {
            if self.process_lab4_gnalb_tick(gnalb_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `lab4_gnalb_driver`'s per-tick body (`lab4.c:469-639`).
    fn process_lab4_gnalb_tick(&mut self, gnalb_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&gnalb_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Lab4Gnalb(data)) => data,
            _ => Lab4GnalbDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&gnalb_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                // C `lab4.c:485-488`.
                NT_CREATE => {
                    self.lab4_gnalb_finish_create(gnalb_id, &mut data);
                }
                // C `lab4.c:490-500`: destroy whatever we're handed,
                // unconditionally.
                NT_GIVE => {
                    if let Some(item_id) = self
                        .characters
                        .get(&gnalb_id)
                        .and_then(|character| character.cursor_item)
                    {
                        self.destroy_item(item_id);
                    }
                }
                // C `lab4.c:502-510`: debug echo only.
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if let Some(text) = message.text.as_deref() {
                        self.apply_tabunga_text_notification(gnalb_id, speaker_id, text);
                    }
                }
                // C `standard_message_driver`'s `NT_CHAR` branch
                // (`drvlib.c:2470-2476`), gated on this NPC's own
                // `aggressive` arg (live only for `type=1`).
                NT_CHAR if data.aggressive && message.dat1 > 0 => {
                    let seen_id = CharacterId(message.dat1 as u32);
                    if self.lab4_gnalb_is_valid_enemy(gnalb_id, seen_id) {
                        data.victim = Some(seen_id);
                    }
                }
                // C `standard_message_driver`'s `NT_SEEHIT` branch
                // (`drvlib.c:2478-2510`), gated on this NPC's own
                // `helper` arg (live only for `type=1`).
                NT_SEEHIT if data.helper && message.dat1 > 0 && message.dat2 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    let victim_id = CharacterId(message.dat2 as u32);
                    let Some(gnalb) = self.characters.get(&gnalb_id).cloned() else {
                        continue;
                    };
                    let victim_is_friend = victim_id != gnalb_id
                        && self
                            .characters
                            .get(&victim_id)
                            .is_some_and(|victim| victim.group == gnalb.group);
                    if victim_is_friend {
                        if self.lab4_gnalb_is_valid_enemy(gnalb_id, attacker_id) {
                            data.victim = Some(attacker_id);
                        }
                        continue;
                    }
                    let attacker_is_friend = attacker_id != gnalb_id
                        && self
                            .characters
                            .get(&attacker_id)
                            .is_some_and(|attacker| attacker.group == gnalb.group);
                    if attacker_is_friend && self.lab4_gnalb_is_valid_enemy(gnalb_id, victim_id) {
                        data.victim = Some(victim_id);
                    }
                }
                // C `standard_message_driver`'s `NT_GOTHIT` branch
                // (`drvlib.c:2512-2538`): unconditional self-defense.
                NT_GOTHIT if message.dat1 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    let Some((gnalb, attacker)) = self
                        .characters
                        .get(&gnalb_id)
                        .cloned()
                        .zip(self.characters.get(&attacker_id).cloned())
                    else {
                        continue;
                    };
                    if gnalb.group != attacker.group && can_attack(&gnalb, &attacker, &self.map) {
                        data.victim = Some(attacker_id);
                    }
                }
                _ => {}
            }
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&gnalb_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((gnalb, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&gnalb, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&gnalb_id) {
            character.driver_state = Some(CharacterDriverState::Lab4Gnalb(data));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(gnalb_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`.
            let arrived = self.characters.get(&gnalb_id).is_some_and(|gnalb| {
                gnalb.x.abs_diff(data.victim_last_x) < 2 && gnalb.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                data.victim = None;
                if let Some(character) = self.characters.get_mut(&gnalb_id) {
                    character.driver_state = Some(CharacterDriverState::Lab4Gnalb(data));
                }
            } else if self.secure_move_driver(
                gnalb_id,
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
        // return;` (`lab4.c:567-572`).
        if self.regenerate_simple_baddy(gnalb_id) {
            return true;
        }
        if self.spell_self_simple_baddy(gnalb_id) {
            return true;
        }

        // C `lab4.c:575-595`: guard patrol.
        if data.gnalb_type == 1 && data.path != 0 {
            return self.lab4_gnalb_patrol(gnalb_id, area_id, &mut data);
        }

        // C `lab4.c:598-631`: crazy-gnalb idle chatter + fireplace poke.
        if data.gnalb_type == 3 {
            return self.lab4_gnalb_crazy_idle(gnalb_id, area_id);
        }

        // C `lab4.c:633-638`: default idle wander.
        let (post_x, post_y) = self
            .characters
            .get(&gnalb_id)
            .map(|gnalb| (gnalb.rest_x, gnalb.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            gnalb_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }
        self.characters
            .get_mut(&gnalb_id)
            .is_some_and(|character| do_idle(character, (TICKS_PER_SECOND / 2) as i32).is_ok())
    }

    /// C `is_valid_enemy(cn, co, -1)` (`src/system/drvlib.c:897-927`).
    fn lab4_gnalb_is_valid_enemy(&self, gnalb_id: CharacterId, target_id: CharacterId) -> bool {
        if gnalb_id == target_id {
            return false;
        }
        let Some(gnalb) = self.characters.get(&gnalb_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        gnalb.group != target.group
            && can_attack(gnalb, target, &self.map)
            && char_see_char(gnalb, target, &self.map, self.date.daylight)
    }

    /// C `lab4_gnalb_driver_init` (`lab4.c:441-467`), called from the
    /// `NT_CREATE` handler. `fight_driver_set_dist` is not ported - see
    /// module doc comment.
    fn lab4_gnalb_finish_create(&mut self, gnalb_id: CharacterId, data: &mut Lab4GnalbDriverData) {
        // C `lab4.c:444-452`: the nearest-path-node lookup, `type=1`
        // guards only.
        if data.gnalb_type == 1 {
            if let Some(gnalb) = self.characters.get(&gnalb_id) {
                let (gx, gy) = (gnalb.x, gnalb.y);
                let mut mindist = i32::MAX;
                let mut nearest = data.path;
                for (index, &(px, py, _)) in GNALB_PATH.iter().enumerate().skip(1) {
                    let dist = map_dist(gx, gy, px, py);
                    if dist < mindist {
                        mindist = dist;
                        nearest = index as u16;
                    }
                }
                data.path = nearest;
            }
        }

        // C `lab4.c:454-466`: `type==1` guards get `aggressive`/`helper`;
        // `type==2` is dead code (`xlog("CANTBETRUE"...)`, see module doc
        // comment) and leaves both flags at their zero-initialized
        // `false`; every other type (`3`/`4`/`5`) explicitly sets both
        // `false` too.
        if data.gnalb_type == 1 {
            data.aggressive = true;
            data.helper = true;
        } else if data.gnalb_type != 2 {
            data.aggressive = false;
            data.helper = false;
        }
    }

    /// C `lab4.c:575-595`: `swap_move_driver` toward the current path
    /// node, then a random branch pick once arrived (`< 4` tiles away).
    /// C always `return`s after this whole block regardless of which
    /// path was taken, so this always reports "acted", same as the
    /// caller's own unconditional `return` after `type=3`'s block below.
    fn lab4_gnalb_patrol(
        &mut self,
        gnalb_id: CharacterId,
        area_id: u16,
        data: &mut Lab4GnalbDriverData,
    ) -> bool {
        let (target_x, target_y, next) = GNALB_PATH[usize::from(data.path)];

        if self.lab4_gnalb_swap_move(gnalb_id, target_x, target_y, area_id) {
            if let Some(character) = self.characters.get_mut(&gnalb_id) {
                character.driver_state = Some(CharacterDriverState::Lab4Gnalb(*data));
            }
            return true;
        }

        let Some(gnalb) = self.characters.get(&gnalb_id) else {
            return true;
        };
        if map_dist(gnalb.x, gnalb.y, target_x, target_y) < 4 {
            // C's `do { p = RANDOM(4); } while (next[p] == 0 ||
            // (next[1] != 0 && next[p] == dat->lastpath));` (`lab4.c:584-
            // 589`).
            let branching = next[1] != 0;
            let mut picked;
            loop {
                let p = legacy_random_below_from_seed(&mut self.legacy_random_seed, 4) as usize;
                picked = next[p];
                if picked != 0 && !(branching && picked == data.lastpath) {
                    break;
                }
            }
            data.lastpath = data.path;
            data.path = picked;
        } else if let Some(character) = self.characters.get_mut(&gnalb_id) {
            let _ = do_idle(character, (TICKS_PER_SECOND / 2) as i32);
        }

        if let Some(character) = self.characters.get_mut(&gnalb_id) {
            character.driver_state = Some(CharacterDriverState::Lab4Gnalb(*data));
        }
        true
    }

    /// C `swap_move_driver(cn, tx, ty, 1)` (`lab4.c:578`): walks toward a
    /// tile, falling back to a pathfinder pass that ignores blocking
    /// characters (the "swap" semantics) if the direct route is blocked -
    /// same two-`setup_walk_toward` pattern as
    /// `world::npc::area11::palace_guard`'s own `palace_guard_move`.
    fn lab4_gnalb_swap_move(
        &mut self,
        gnalb_id: CharacterId,
        target_x: u16,
        target_y: u16,
        area_id: u16,
    ) -> bool {
        self.setup_walk_toward(
            gnalb_id,
            usize::from(target_x),
            usize::from(target_y),
            1,
            area_id,
            false,
        ) || self.setup_walk_toward(
            gnalb_id,
            usize::from(target_x),
            usize::from(target_y),
            1,
            area_id,
            true,
        )
    }

    /// C `lab4.c:598-631`: the crazy gnalb's random murmurs plus its
    /// `do_use(cn, DX_RIGHT, 0)` fireplace poke. C always `return`s after
    /// this whole block, so this always reports "acted".
    fn lab4_gnalb_crazy_idle(&mut self, gnalb_id: CharacterId, area_id: u16) -> bool {
        match legacy_random_below_from_seed(&mut self.legacy_random_seed, 50) {
            0 => {
                self.npc_whisper(gnalb_id, "Me saw right in Fire.");
            }
            1 => {
                self.npc_whisper(gnalb_id, "Me not crazy. In me house me saw in fire.");
            }
            2 => {
                self.npc_whisper(gnalb_id, "Me will get it out.");
            }
            3 => {
                self.npc_whisper(gnalb_id, "Fire hot, but me not crazy.");
            }
            4 => {
                self.npc_whisper(gnalb_id, "Tell mage me saw in fire, me not crazy.");
            }
            // C `if (do_use(cn, DX_RIGHT, 0)) return;` (`lab4.c:620`).
            10..=14 => {
                if self.lab4_gnalb_use_right(gnalb_id) {
                    return true;
                }
            }
            _ => {}
        }

        // C `if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT,
        // ret, lastact)) return; do_idle(cn, TICKS / 2); return;`
        // (`lab4.c:625-629`): `tmpx`/`tmpy` reuse `rest_x`/`rest_y`, same
        // substitution as every other stationary NPC in this file.
        let (post_x, post_y) = self
            .characters
            .get(&gnalb_id)
            .map(|gnalb| (gnalb.rest_x, gnalb.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            gnalb_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }
        if let Some(character) = self.characters.get_mut(&gnalb_id) {
            let _ = do_idle(character, (TICKS_PER_SECOND / 2) as i32);
        }
        true
    }

    /// C `do_use(cn, DX_RIGHT, 0)` (`lab4.c:620`): trigger whatever
    /// usable item sits directly to this NPC's right (the fireplace),
    /// same "use whatever's there" pattern as
    /// `world::npc::area1::balltrap::balltrap_fire_left`.
    fn lab4_gnalb_use_right(&mut self, gnalb_id: CharacterId) -> bool {
        let Some(gnalb) = self.characters.get(&gnalb_id).cloned() else {
            return false;
        };
        let (dx, dy) = Direction::Right.delta();
        let Some(x) = offset_coordinate(usize::from(gnalb.x), dx) else {
            return false;
        };
        let Some(y) = offset_coordinate(usize::from(gnalb.y), dy) else {
            return false;
        };
        let item_id = self.map.tile(x, y).map(|tile| tile.item).unwrap_or(0);
        if item_id == 0 {
            return false;
        }
        let Some(item) = self.items.get(&ItemId(item_id)).cloned() else {
            return false;
        };
        if !item.flags.contains(ItemFlags::USE) {
            return false;
        }
        let Some(character) = self.characters.get_mut(&gnalb_id) else {
            return false;
        };
        do_use(
            character,
            &self.map,
            &item,
            Direction::Right as u8,
            0,
            self.settings.weather_movement_percent,
        )
        .is_ok()
    }
}
