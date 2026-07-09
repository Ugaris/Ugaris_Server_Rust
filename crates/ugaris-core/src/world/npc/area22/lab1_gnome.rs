//! Lab 1 torch-gnome NPC (`CDR_LABGNOMEDRIVER`), the guard/master triad
//! that gates the labyrinth's first level.
//!
//! Ports `src/area/22/lab1.c::labgnome_driver` (`:259-386`) plus its
//! `labgnome_died_driver` death hook (`:388-406`). The same C function
//! backs three roles selected by zone-file args (`fighter=1`, `master=1`,
//! or neither): a torch-guard grunt that patrols a fixed set of nearby
//! torches and re-lights/defends them, a plain fighter variant (wider
//! aggro range, no torch duty), and the room's `CF_IMMORTAL` "Immortal
//! Master" who can only be hurt by the `IDR_DEATHFIBRIN` staff
//! (`world::item_driver::area22_lab::deathfibrin_driver` in
//! `ugaris-server`'s item-outcome dispatch - see that driver's own doc
//! comment for the coupling).
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification already established
//!   for every other ported NPC (see `world::asturin`'s module doc
//!   comment): C's generic 10-slot `struct fight_driver_data` is narrowed
//!   to a single tracked `victim`, fed by the torch-attacker `NT_NPC`/
//!   `NTID_LABGNOMETORCH` message, `standard_message_driver`'s `NT_CHAR`/
//!   `NT_SEEHIT`/`NT_GOTHIT` cases (parameterized by this driver's own
//!   `aggressive`/`helper` args, unlike most single-victim NPCs which
//!   hardcode one mode), and the "someone is blocking my path to the
//!   torch" tail of `use_and_attack_driver`.
//! - `fight_driver_set_dist(cn, 30, 0, 60)` / `(cn, 14, 0, 14)` / `(cn,
//!   15, 0, 15)` (`lab1.c:276,278,281`, one per role) are not ported -
//!   same precedent as every other single-victim NPC's own module doc
//!   comment (never observably different from C in practice with the
//!   single-victim model).
//! - `use_and_attack_driver`'s custom `attack_check_target` pathfinder
//!   predicate (letting the pathfinder route through doors and idle
//!   players) is not reproduced; this port reuses the generic
//!   `World::setup_walk_toward_use_item` walk-then-use helper (same one
//!   `world::lampghost`'s lamp-walk uses) for the "reach the torch"
//!   half, and a plain `pathfinder` direction probe for the "who's
//!   blocking me" tail - any character (not just idle players, C's own
//!   `attack_check_target` filter) standing on the probed tile becomes
//!   the new victim. A torch behind a closed door the gnome cannot path
//!   through is therefore never reached by this port either way, same
//!   observable end state as C.
//! - `scan_gnometorches`'s `scan_gnometorch_check_target` pathfinder
//!   reachability pre-filter (skip torches whose own tile holds a
//!   driver-2 item) is not reproduced - torches are registered purely by
//!   line-of-sight (`World::map`'s `can_see`, matching C's own
//!   `los_can_see(cn, ..., 15)` call) within the same `[-15,+15]` box.
//!   `lab1.chr`'s actual torch placements are all in open floor tiles,
//!   so this is never observably different in the shipped zone data.
//!   The `MAX_GNOMETORCH = 10` cap and the farthest-first sort (`db -
//!   da`, C's own descending-order comparator - the *last* unlit torch
//!   found while re-scanning the list each tick therefore ends up being
//!   the *nearest* one, matching C's `dat->usetarget` overwrite loop)
//!   ARE both reproduced digit-for-digit.
//! - `labgnome_died_driver`'s `dat->master` branch (`create_lab_exit(co,
//!   20)`, spawning the `"labexit"` reward item at the killer's
//!   position) is not ported yet: `create_lab_exit` is shared verbatim
//!   by all five `src/area/22/lab*.c` files and its `IDR_LABEXIT` use-
//!   side (`set_solved_lab` + `change_area(cn, 3, 183, 199)`) is
//!   likewise still an inert stub in `ugaris-server`'s completed-action
//!   dispatch (`ItemDriverOutcome::LabExitUse` currently only increments
//!   the executed counter - see `crates/ugaris-server/src/
//!   tick_item_use_completion.rs`). Wiring the full "kill a lab master,
//!   get a labexit, use it to solve the level and warp to Aston" loop is
//!   tracked as a single follow-up slice shared by all five lab areas
//!   rather than five one-off half-features; `apply_labgnome_death_driver`
//!   below still ports the `dat->text` speech branch, which IS fully
//!   self-contained.
//! - `tabunga` (the `CF_GOD` debug stat dump `NT_TEXT` triggers) reuses
//!   the already-ported `World::apply_tabunga_text_notification`
//!   (`world::text`), same as `CDR_SIMPLEBADDY`'s own generic `NT_TEXT`
//!   handling.

use crate::character_driver::{next_legacy_name_value, CDR_LABGNOMEDRIVER, NTID_LABGNOMETORCH};
use crate::world::*;

/// C `#define MAX_GNOMETORCH 10` (`lab1.c:169`).
const MAX_GNOMETORCH: usize = 10;
/// C `scan_gnometorches`'s fixed radius (`lab1.c:211-214`), also reused
/// for `los_can_see(cn, ..., 15)`.
const GNOMETORCH_SCAN_RADIUS: i32 = 15;

/// C `struct labgnome_driver_data` (`lab1.c:171-183`). `outch` is never
/// read anywhere in `labgnome_driver`'s body - dead even in C, same
/// precedent as other ported NPCs' own dead fields - so it is not
/// ported.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LabGnomeDriverData {
    pub fighter: bool,
    pub master: bool,
    pub text: bool,
    pub aggressive: bool,
    pub helper: bool,
    /// C `dat->dir` (`DX_DOWN`/`DX_RIGHT`/`DX_LEFT`): idle-facing direction
    /// computed once from the spawn room's `MF_SIGHTBLOCK` neighbors.
    pub dir: u8,
    /// C `dat->torch[MAX_GNOMETORCH]`/`dat->numtorch`: fixed at creation
    /// by `scan_gnometorches`, farthest-first (see module doc comment).
    pub torches: Vec<ItemId>,
    /// C `dat->usetarget`.
    pub usetarget: Option<ItemId>,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}

impl Default for LabGnomeDriverData {
    fn default() -> Self {
        Self {
            fighter: false,
            master: false,
            text: false,
            aggressive: false,
            helper: false,
            dir: Direction::Down as u8,
            torches: Vec::new(),
            usetarget: None,
            victim: None,
            victim_visible: false,
            victim_last_x: 0,
            victim_last_y: 0,
        }
    }
}

/// C `labgnome_driver_parse` (`lab1.c:235-257`).
fn parse_labgnome_driver_args(args: &str) -> LabGnomeDriverData {
    let mut data = LabGnomeDriverData::default();
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "fighter" => data.fighter = parsed != 0,
            "text" => data.text = parsed != 0,
            "master" => data.master = parsed != 0,
            "aggressive" => data.aggressive = parsed != 0,
            "helper" => data.helper = parsed != 0,
            _ => {}
        }
        rest = next;
    }
    data
}

/// Spawn-time data-only half of C's `NT_CREATE` handler (`lab1.c:273-283`
/// up through the `fight_driver_set_dist`/`CF_IMMORTAL` branch), called
/// from `ZoneLoader::instantiate_character_template` the same way
/// `apply_lab2_undead_create_message`/`apply_palace_guard_create_message`
/// are. The map-dependent remainder (idle-facing direction, torch scan)
/// runs on the character's first live tick instead - see
/// `World::process_labgnome_tick`'s own `NT_CREATE` handling below,
/// mirroring C's own timing (the `NT_CREATE` message is only ever
/// processed on a character's next tick, never synchronously at
/// `create_char` time).
pub fn apply_labgnome_create_message(character: &mut Character, args: Option<&str>) {
    let mut data = args
        .filter(|args| !args.is_empty())
        .map(|args| parse_labgnome_driver_args(args))
        .unwrap_or_default();
    // C `ch[cn].flags |= CF_IMMORTAL;` inside the `dat->master` branch
    // (`lab1.c:279`).
    if data.master {
        character.flags.insert(CharacterFlags::IMMORTAL);
    }
    data.dir = Direction::Down as u8;
    character.driver_state = Some(CharacterDriverState::LabGnome(data));
    character.push_driver_message(NT_CREATE, 0, 0, 0);
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_LABGNOMEDRIVER`
    /// characters (C `ch_driver`'s `CDR_LABGNOMEDRIVER` case, `lab1.c:592-
    /// 600`).
    pub fn process_labgnome_actions(&mut self, area_id: u16) -> usize {
        let gnome_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LABGNOMEDRIVER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for gnome_id in gnome_ids {
            if self.process_labgnome_tick(gnome_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `labgnome_driver`'s per-tick body (`lab1.c:259-386`).
    fn process_labgnome_tick(&mut self, gnome_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&gnome_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::LabGnome(data)) => data,
            _ => LabGnomeDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&gnome_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                // C `lab1.c:273-295`: finish the map-dependent half of
                // `NT_CREATE` init (see `apply_labgnome_create_message`'s
                // own doc comment).
                NT_CREATE => {
                    self.labgnome_finish_create(gnome_id, &mut data);
                }
                // C `lab1.c:298-309`: a torch we're guarding was turned
                // off by someone.
                NT_NPC if message.dat1 == NTID_LABGNOMETORCH => {
                    let torch_id = ItemId(message.dat2.max(0) as u32);
                    let attacker_id = CharacterId(message.dat3.max(0) as u32);
                    if data.torches.contains(&torch_id)
                        && self.labgnome_is_valid_enemy(gnome_id, attacker_id)
                    {
                        data.victim = Some(attacker_id);
                        self.npc_shout(
                            gnome_id,
                            &format!(
                                "Hurgha. Master me told protecting torch. Prepare to die {}!",
                                self.characters
                                    .get(&attacker_id)
                                    .map(|attacker| attacker.name.as_str())
                                    .unwrap_or_default()
                            ),
                        );
                    }
                }
                // C `lab1.c:311-314`.
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if let Some(text) = message.text.as_deref() {
                        self.apply_tabunga_text_notification(gnome_id, speaker_id, text);
                    }
                }
                // C `standard_message_driver`'s `NT_CHAR` branch
                // (`drvlib.c:2470-2476`), gated on this NPC's own
                // `aggressive` arg.
                NT_CHAR if data.aggressive && message.dat1 > 0 => {
                    let seen_id = CharacterId(message.dat1 as u32);
                    if self.labgnome_is_valid_enemy(gnome_id, seen_id) {
                        data.victim = Some(seen_id);
                    }
                }
                // C `standard_message_driver`'s `NT_SEEHIT` branch
                // (`drvlib.c:2478-2510`), gated on this NPC's own
                // `helper` arg.
                NT_SEEHIT if data.helper && message.dat1 > 0 && message.dat2 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    let victim_id = CharacterId(message.dat2 as u32);
                    let Some(gnome) = self.characters.get(&gnome_id).cloned() else {
                        continue;
                    };
                    let victim_is_friend = victim_id != gnome_id
                        && self
                            .characters
                            .get(&victim_id)
                            .is_some_and(|victim| victim.group == gnome.group);
                    if victim_is_friend {
                        if self.labgnome_is_valid_enemy(gnome_id, attacker_id) {
                            data.victim = Some(attacker_id);
                        }
                        continue;
                    }
                    let attacker_is_friend = attacker_id != gnome_id
                        && self
                            .characters
                            .get(&attacker_id)
                            .is_some_and(|attacker| attacker.group == gnome.group);
                    if attacker_is_friend && self.labgnome_is_valid_enemy(gnome_id, victim_id) {
                        data.victim = Some(victim_id);
                    }
                }
                // C `standard_message_driver`'s `NT_GOTHIT` branch
                // (`drvlib.c:2512-2538`): unconditional self-defense.
                NT_GOTHIT if message.dat1 > 0 => {
                    let attacker_id = CharacterId(message.dat1 as u32);
                    let Some((gnome, attacker)) = self
                        .characters
                        .get(&gnome_id)
                        .cloned()
                        .zip(self.characters.get(&attacker_id).cloned())
                    else {
                        continue;
                    };
                    if gnome.group != attacker.group && can_attack(&gnome, &attacker, &self.map) {
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
                .get(&gnome_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((gnome, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&gnome, &victim, &self.map, self.date.daylight) {
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

        if let Some(character) = self.characters.get_mut(&gnome_id) {
            character.driver_state = Some(CharacterDriverState::LabGnome(data.clone()));
        }

        // C `if (fight_driver_attack_visible(cn, 0)) return;`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(gnome_id, victim_id, area_id) {
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) return;`.
            let arrived = self.characters.get(&gnome_id).is_some_and(|gnome| {
                gnome.x.abs_diff(data.victim_last_x) < 2 && gnome.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                data.victim = None;
                if let Some(character) = self.characters.get_mut(&gnome_id) {
                    character.driver_state = Some(CharacterDriverState::LabGnome(data.clone()));
                }
            } else if self.secure_move_driver(
                gnome_id,
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

        // C `lab1.c:329-351`: re-pick the nearest unlit tracked torch,
        // then walk to and light it (or fight whoever's in the way).
        if data.usetarget.is_none()
            || self
                .items
                .get(&data.usetarget.unwrap())
                .is_some_and(|item| item.driver_data.first().copied().unwrap_or(0) == 1)
        {
            data.usetarget = None;
            for &torch_id in &data.torches {
                let unlit = self
                    .items
                    .get(&torch_id)
                    .is_some_and(|item| item.driver_data.first().copied().unwrap_or(0) == 0);
                if unlit {
                    data.usetarget = Some(torch_id);
                    self.npc_say(gnome_id, "Master me told keeping torch burning.");
                }
            }
            if let Some(character) = self.characters.get_mut(&gnome_id) {
                character.driver_state = Some(CharacterDriverState::LabGnome(data.clone()));
            }
        }

        if let Some(torch_id) = data.usetarget {
            let (used, blocker) = self.labgnome_use_and_attack(gnome_id, torch_id, area_id);
            if used {
                return true;
            }
            if let Some(blocker_id) = blocker {
                if self.labgnome_is_valid_enemy(gnome_id, blocker_id) {
                    data.victim = Some(blocker_id);
                    if let Some(character) = self.characters.get_mut(&gnome_id) {
                        character.driver_state = Some(CharacterDriverState::LabGnome(data.clone()));
                    }
                    self.npc_shout(
                        gnome_id,
                        &format!(
                            "You're in my way {}! Die!",
                            self.characters
                                .get(&blocker_id)
                                .map(|blocker| blocker.name.as_str())
                                .unwrap_or_default()
                        ),
                    );
                }
            }
        }

        // C `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return;` (`lab1.c:354-359`).
        if self.regenerate_simple_baddy(gnome_id) {
            return true;
        }
        if self.spell_self_simple_baddy(gnome_id) {
            return true;
        }

        // C `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, dat->dir,
        // ret, lastact)` (`lab1.c:360-362`): `tmpx`/`tmpy` reuse
        // `rest_x`/`rest_y`, same substitution as every other stationary
        // NPC in this file.
        let (post_x, post_y) = self
            .characters
            .get(&gnome_id)
            .map(|gnome| (gnome.rest_x, gnome.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(gnome_id, post_x, post_y, data.dir, 0, 0, area_id) {
            return true;
        }

        // C `lab1.c:365-382`: idle nonsense, only for non-master gnomes.
        if !data.master {
            match legacy_random_below_from_seed(&mut self.legacy_random_seed, 500) {
                0 => {
                    self.npc_say(gnome_id, "Grmadasd.");
                }
                1 => {
                    self.npc_say(gnome_id, "Huas. Grkasd Wod.");
                }
                2 => {
                    if !data.torches.is_empty() {
                        self.npc_say(
                            gnome_id,
                            &format!("Me have to protect {} torch. Hungrfa.", data.torches.len()),
                        );
                    }
                }
                3 => {
                    self.npc_say(gnome_id, "Me have dark here feeling.");
                }
                _ => {}
            }
        }

        // C `do_idle(cn, TICKS / 2);` (`lab1.c:385`).
        self.characters
            .get_mut(&gnome_id)
            .is_some_and(|character| do_idle(character, (TICKS_PER_SECOND / 2) as i32).is_ok())
    }

    /// C `is_valid_enemy(cn, co, -1)` (`src/system/drvlib.c:897-927`).
    fn labgnome_is_valid_enemy(&self, gnome_id: CharacterId, target_id: CharacterId) -> bool {
        if gnome_id == target_id {
            return false;
        }
        let Some(gnome) = self.characters.get(&gnome_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        gnome.group != target.group
            && can_attack(gnome, target, &self.map)
            && char_see_char(gnome, target, &self.map, self.date.daylight)
    }

    /// The map-dependent remainder of C's `NT_CREATE` handler (`lab1.c:
    /// 276-294`): idle-facing direction from the spawn room's
    /// `MF_SIGHTBLOCK` neighbors, plus (for plain guard gnomes only) the
    /// fixed torch scan.
    fn labgnome_finish_create(&mut self, gnome_id: CharacterId, data: &mut LabGnomeDriverData) {
        let Some(gnome) = self.characters.get(&gnome_id).cloned() else {
            return;
        };

        // C `lab1.c:286-294`.
        let mut dir = Direction::Down as u8;
        if gnome.x > 0
            && self
                .map
                .tile(usize::from(gnome.x) - 1, usize::from(gnome.y))
                .is_some_and(|tile| tile.flags.contains(MapFlags::SIGHTBLOCK))
        {
            dir = Direction::Right as u8;
        }
        if self
            .map
            .tile(usize::from(gnome.x) + 1, usize::from(gnome.y))
            .is_some_and(|tile| tile.flags.contains(MapFlags::SIGHTBLOCK))
        {
            dir = Direction::Left as u8;
        }
        data.dir = dir;

        // C `lab1.c:275-283`: only plain guard gnomes (neither `fighter`
        // nor `master`) scan for torches to protect.
        if !data.fighter && !data.master {
            data.torches = self.labgnome_scan_torches(&gnome);
        }
    }

    /// C `scan_gnometorches` (`lab1.c:208-233`): see module doc comment
    /// for the pathfinder-reachability-filter gap.
    fn labgnome_scan_torches(&self, gnome: &Character) -> Vec<ItemId> {
        let gx = i32::from(gnome.x);
        let gy = i32::from(gnome.y);
        let sx = (gx - GNOMETORCH_SCAN_RADIUS).max(0) as usize;
        let sy = (gy - GNOMETORCH_SCAN_RADIUS).max(0) as usize;
        let ex = (gx + GNOMETORCH_SCAN_RADIUS).min(self.map.width() as i32 - 1) as usize;
        let ey = (gy + GNOMETORCH_SCAN_RADIUS).min(self.map.height() as i32 - 1) as usize;

        let mut candidates: Vec<(ItemId, i32)> = self
            .items
            .values()
            .filter(|item| {
                item.driver == IDR_LABTORCH
                    && usize::from(item.x) >= sx
                    && usize::from(item.x) <= ex
                    && usize::from(item.y) >= sy
                    && usize::from(item.y) <= ey
                    && self.map.can_see(
                        usize::from(gnome.x),
                        usize::from(gnome.y),
                        usize::from(item.x),
                        usize::from(item.y),
                        GNOMETORCH_SCAN_RADIUS as usize,
                    )
            })
            .map(|item| {
                let dx = i32::from(item.x) - gx;
                let dy = i32::from(item.y) - gy;
                (item.id, dx * dx + dy * dy)
            })
            .collect();

        // C `qsortproc_gnometorch`'s `db - da` comparator: farthest
        // first. Tie-break by `ItemId` for determinism (an
        // implementation detail with no in-game observable meaning, same
        // precedent as `world::lampghost`'s own tie-break substitution).
        candidates.sort_by(|(a_id, a_dist), (b_id, b_dist)| {
            b_dist.cmp(a_dist).then_with(|| a_id.0.cmp(&b_id.0))
        });
        candidates.truncate(MAX_GNOMETORCH);
        candidates.into_iter().map(|(item_id, _)| item_id).collect()
    }

    /// C `use_and_attack_driver(cn, dat->usetarget, 0, &co)` (`lab1.c:93-
    /// 162`), narrowed per the module doc comment: returns `(used, blocker)`
    /// where `used` is C's own return value and `blocker` is `co` when the
    /// walk failed but a character was found standing where the gnome
    /// needed to step.
    fn labgnome_use_and_attack(
        &mut self,
        gnome_id: CharacterId,
        item_id: ItemId,
        area_id: u16,
    ) -> (bool, Option<CharacterId>) {
        let Some(item) = self.items.get(&item_id).cloned() else {
            return (false, None);
        };
        let Some(gnome) = self.characters.get(&gnome_id).cloned() else {
            return (false, None);
        };

        if let Some(direction) = adjacent_use_direction(
            gnome.x,
            gnome.y,
            usize::from(item.x),
            usize::from(item.y),
            item.flags.contains(ItemFlags::FRONTWALL),
        ) {
            let Some(gnome_mut) = self.characters.get_mut(&gnome_id) else {
                return (false, None);
            };
            let used = do_use(
                gnome_mut,
                &self.map,
                &item,
                direction as u8,
                0,
                self.settings.weather_movement_percent,
            )
            .is_ok();
            return (used, None);
        }

        if self.setup_walk_toward_use_item(
            gnome_id,
            usize::from(item.x),
            usize::from(item.y),
            item.flags,
            area_id,
        ) {
            return (true, None);
        }

        // C `lab1.c:144-158`: a direction toward the torch existed but
        // neither `do_walk` nor `do_use` succeeded - see who's standing
        // in the way.
        let path = pathfinder(
            &self.map,
            usize::from(gnome.x),
            usize::from(gnome.y),
            usize::from(item.x),
            usize::from(item.y),
            1,
            None,
        );
        let Some(direction) = path.direction else {
            return (false, None);
        };
        let (dx, dy) = direction.delta();
        let bx = i32::from(gnome.x) + i32::from(dx);
        let by = i32::from(gnome.y) + i32::from(dy);
        let (Ok(bx), Ok(by)) = (usize::try_from(bx), usize::try_from(by)) else {
            return (false, None);
        };
        let blocker = self
            .map
            .tile(bx, by)
            .map(|tile| tile.character)
            .filter(|&character_id| character_id != 0)
            .map(|character_id| CharacterId(u32::from(character_id)));
        (false, blocker)
    }

    /// C `labgnome_died_driver` (`lab1.c:388-406`): see module doc
    /// comment for the not-yet-ported `create_lab_exit` reward branch.
    pub fn apply_labgnome_death_driver(&mut self, gnome_id: CharacterId, killer_id: CharacterId) {
        let Some(CharacterDriverState::LabGnome(data)) = self
            .characters
            .get(&gnome_id)
            .and_then(|character| character.driver_state.clone())
        else {
            return;
        };

        if data.text {
            let killer_name = self
                .characters
                .get(&killer_id)
                .map(|killer| killer.name.clone())
                .unwrap_or_default();
            self.npc_say(
                gnome_id,
                &format!(
                    "Arrrggh. {killer_name} me killed, but {killer_name} never kills master \
                     behind door. Master can be killed only by Deathfibrin."
                ),
            );
        }
    }

    /// C `deathfibrin_scan` (`lab1.c:440-458`): find a live, visible
    /// "Immortal Master" (a `CDR_LABGNOMEDRIVER` named exactly "Immortal
    /// Master", matching `lab1.chr`'s own `labgnome_master` template)
    /// within an 8-tile box. C returns the first row-major match; this
    /// port's `HashMap` iteration order is non-deterministic, but every
    /// shipped lab room only ever has one master, so this is never
    /// observably different in practice.
    pub(crate) fn deathfibrin_scan(&self, character_id: CharacterId) -> Option<CharacterId> {
        let character = self.characters.get(&character_id)?;
        let cx = i32::from(character.x);
        let cy = i32::from(character.y);
        let sx = (cx - 8).max(0) as u16;
        let sy = (cy - 8).max(0) as u16;
        let ex = (cx + 8).min(self.map.width() as i32 - 1) as u16;
        let ey = (cy + 8).min(self.map.height() as i32 - 1) as u16;
        self.characters
            .values()
            .find(|candidate| {
                candidate.driver == CDR_LABGNOMEDRIVER
                    && candidate.name == "Immortal Master"
                    && candidate.x >= sx
                    && candidate.x <= ex
                    && candidate.y >= sy
                    && candidate.y <= ey
                    && char_see_char(character, candidate, &self.map, self.date.daylight)
            })
            .map(|candidate| candidate.id)
    }

    /// C `map[ch[cn].x+ch[cn].y*MAXMAP].light` (`lab1.c:523-524`), the
    /// debug value C's own "no immortal close enough" message prints.
    pub(crate) fn deathfibrin_tile_light(&self, character_id: CharacterId) -> u8 {
        let Some(character) = self.characters.get(&character_id) else {
            return 0;
        };
        self.map
            .tile(usize::from(character.x), usize::from(character.y))
            .map(|tile| tile.light.clamp(0, 255) as u8)
            .unwrap_or(0)
    }
}
