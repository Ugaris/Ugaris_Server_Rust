//! Area 32 governor job-board instance-dungeon spawn:
//! `src/area/32/missions.c::start_mission`/`build_fighter`
//! (`:950-1130`/`:678-865`).
//!
//! Split out of `governor.rs` (already over the ~800-line NPC-file
//! guideline) since this is a large, mostly self-contained slice. Follows
//! the "plan in `ugaris-core` (pure map/item mutation, no `ZoneLoader`
//! needed), spawn in `ugaris-server`" split `world::pents`/`world::
//! npc::area8::fdemon_army` already established: [`World::
//! plan_start_mission`] does the busy-slice search, the existing-
//! character/junk-item cleanup, and the door/chest/entry key wiring (all
//! plain `World` map/item mutation, matching C's own single-pass
//! `start_mission` loop) and returns a [`MissionStartPlan`] describing
//! which fighters to spawn where; `ugaris-server`'s `area32.rs` turns
//! each [`FighterSpawnSpec`] into a real character via `ZoneLoader`/
//! `ServerRuntime::allocate_character_id`, mirroring `spawns.rs::
//! spawn_warp_trial_fighter`.
//!
//! Deviations (documented, not silent):
//! - C's `if (shutdown_at && realtime - shutdown_at < 10)` pre-shutdown
//!   refusal is not ported - this codebase has no `shutdown_at`
//!   wall-clock concept anywhere yet (a cosmetic ~10-second window right
//!   before a graceful shutdown, not a correctness gap).
//! - `dlog` audit-log calls have no reproduction anywhere in this
//!   codebase (see `governor.rs`'s own doc comment).

use crate::entity::CharacterValue as V;
use crate::item_driver::{
    make_item_id, skill_raise_cost_factor, DEV_ID_MISSION, IID_MISSIONCHEST, IID_MISSIONDOOR1,
    IID_MISSIONDOOR2, IID_MISSIONENTRY, IID_MISSIONFIGHTER,
};
use crate::player::MissionPpd;
use crate::world::*;

/// C `struct mission_data`'s `build_fighter`/`start_mission`-only fields
/// (`missions.c:449-470`), parallel to `governor::MISSION_TEMPLATES` -
/// same index order (thief, spy, beast, ruffian, vampire, graverobber,
/// hide-and-seek), kept in a separate table rather than growing that
/// struct so `governor.rs` (already over the file-size guideline) does
/// not need to change.
pub struct MissionFighterData {
    pub basename: &'static str,
    pub basedesc: &'static str,
    pub bossname: &'static str,
    pub bossdesc: &'static str,
    pub bigbossname: &'static str,
    pub bigbossdesc: &'static str,
    pub sprite: i32,
    pub bosssprite: i32,
    /// C `strengthname[3]` (easy/normal/hard suffix, e.g. `" Apprentice"`/
    /// `""`/`" Master"`).
    pub strength_names: [&'static str; 3],
    pub temp: &'static str,
    pub bosstemp: &'static str,
    pub itemname: Option<&'static str>,
    pub area: u16,
    pub char_flags: CharacterFlags,
}

/// C `struct mission_data *mdtab[]` (`missions.c:632-633`).
pub const MISSION_FIGHTER_DATA: [MissionFighterData; 7] = [
    MissionFighterData {
        basename: "Thief",
        basedesc: "A thief belonging to the famous gang 'The Pickers'.",
        bossname: "Sacewan",
        bossdesc: "The boss of the gang 'The Pickers'.",
        bigbossname: "Thief Lord",
        bigbossdesc: "Sacewan's boss.",
        sprite: 312,
        bosssprite: 312,
        strength_names: [" Apprentice", "", " Master"],
        temp: "mis_warrior",
        bosstemp: "mis_seyan",
        itemname: Some("Documents"),
        area: 0,
        char_flags: CharacterFlags::ALIVE,
    },
    MissionFighterData {
        basename: "Bodyguard",
        basedesc: "One of the spy's hirelings.",
        bossname: "Spy",
        bossdesc: "The spy who stole the book.",
        bigbossname: "Spy Lord",
        bigbossdesc: "Uh-oh. This is the spies's boss. Looks tough.",
        sprite: 312,
        bosssprite: 312,
        strength_names: [" Trainee", "", " Master"],
        temp: "mis_warrior",
        bosstemp: "mis_seyan",
        itemname: Some("Book"),
        area: 1,
        char_flags: CharacterFlags::ALIVE,
    },
    MissionFighterData {
        basename: "Swamp Beast",
        basedesc: "A vicious looking beast.",
        bossname: "Beasty Boss",
        bossdesc: "This looks like the swamp beasts leader.",
        bigbossname: "Beast Lord",
        bigbossdesc: "A very vicious looking beast. Must be the leader's leader.",
        sprite: 300,
        bosssprite: 300,
        strength_names: [" Youngling", "", " Elder"],
        temp: "mis_beast",
        bosstemp: "mis_beast",
        itemname: None,
        area: 2,
        char_flags: CharacterFlags::ALIVE,
    },
    MissionFighterData {
        basename: "Ruffian",
        basedesc: "A ruffian under the command of Gorinion.",
        bossname: "Gorinion",
        bossdesc: "The ruffian's leader.",
        bigbossname: "Ruffian Lord",
        bigbossdesc: "My, oh, my, what a hunk.",
        sprite: 15,
        bosssprite: 13,
        strength_names: [" Newbie", "", " Bottomkicker"],
        temp: "mis_warrior",
        bosstemp: "mis_mage",
        itemname: None,
        area: 3,
        char_flags: CharacterFlags::ALIVE,
    },
    MissionFighterData {
        basename: "Vampire",
        basedesc: "White skin, fangs, blood on its lips. This is a vampire, alright.",
        bossname: "Vampire Master",
        bossdesc: "White skin, fangs, blood on its lips. This is a vampire, alright.",
        bigbossname: "Vampire Lord",
        bigbossdesc: "White skin, fangs, blood on its lips. This is a vampire, alright.",
        sprite: 23,
        bosssprite: 23,
        strength_names: [" Youngling", "", " Elder"],
        temp: "mis_seyan",
        bosstemp: "mis_seyan",
        itemname: None,
        area: 4,
        char_flags: CharacterFlags::UNDEAD,
    },
    MissionFighterData {
        basename: "Zombie",
        basedesc: "Rotten flesh, decaying bones, weird grin. Looks like a Zombie.",
        bossname: "Zombie Boss",
        bossdesc: "Rotten flesh, decaying bones, weird grin, lots of muscle. Looks like a Zombie Boss.",
        bigbossname: "Zombie Lord",
        bigbossdesc: "Rotten flesh, decaying bones, weird grin, lots of muscle. Looks like the Zombie Bosses boss.",
        sprite: 9,
        bosssprite: 9,
        strength_names: [" Slave", "", " Master"],
        temp: "mis_warrior",
        bosstemp: "mis_warrior",
        itemname: Some("Spoon of Doom"),
        area: 4,
        char_flags: CharacterFlags::UNDEAD,
    },
    MissionFighterData {
        basename: "Skeleton",
        basedesc: "My, what a bony guy. Uh, on second look: It's just bones.",
        bossname: "Skeleton Master",
        bossdesc: "My, what a bony guy. Uh, on second look: It's just bones.",
        bigbossname: "Skeleton Lord",
        bigbossdesc: "My, what a bony guy. Uh, on second look: It's just bones.",
        sprite: 8,
        bosssprite: 8,
        strength_names: [" Weakling", "", " Strongling"],
        temp: "mis_warrior",
        bosstemp: "mis_warrior",
        itemname: Some("Ring"),
        area: 5,
        char_flags: CharacterFlags::UNDEAD,
    },
];

/// One fighter [`World::plan_start_mission`] wants
/// `ugaris-server`'s `area32.rs::spawn_mission_fighters` to instantiate.
/// `key_id: 0` means no `mis_key` item (C `keyID` param `0`).
#[derive(Debug, Clone, PartialEq)]
pub struct FighterSpawnSpec {
    pub x: u16,
    pub y: u16,
    pub diff: i32,
    pub key_id: u32,
    pub key_name: &'static str,
    pub name: String,
    pub temp: &'static str,
    pub desc: &'static str,
    /// C `ch[cn].deaths = fID` (`1`=easy/`2`=normal/`3`=hard/`4`=boss):
    /// the still-unported `mission_fighter_dead` death hook's kill-
    /// counter tag - kept for forward compatibility, unused by this
    /// slice.
    pub fighter_kind: u8,
    pub sprite: i32,
    pub has_special_item: bool,
    pub extra_flags: CharacterFlags,
}

/// Return value of [`World::plan_start_mission`].
pub struct MissionStartPlan {
    pub entry: (u16, u16),
    pub fighters: Vec<FighterSpawnSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionStartError {
    /// C's own busy-slice refusal message ("...this job is unavailable
    /// right now...").
    AllSlicesBusy,
}

/// C `build_fighter`'s per-skill stat formula (`missions.c:693-768`),
/// applied only where `markers[v] != 0` (the freshly-instantiated
/// template's own bare `value[1]`) **and**
/// [`crate::item_driver::skill_raise_cost_factor`]`(v) != 0` - C's `for (n
/// = 0; n < V_MAX; n++) { if (!skill[n].cost) continue; if
/// (!ch[cn].value[1][n]) continue; ... }` loop. Every other index keeps
/// its template marker value unchanged.
pub fn build_fighter_stat_values(markers: &[i16], diff: i32) -> Vec<i16> {
    markers
        .iter()
        .enumerate()
        .map(|(index, &marker)| {
            if marker == 0 || skill_raise_cost_factor(index) == 0 {
                marker
            } else {
                build_fighter_stat_value(index, diff)
            }
        })
        .collect()
}

fn build_fighter_stat_value(index: usize, diff: i32) -> i16 {
    let val =
        if index == V::Hp as usize || index == V::Endurance as usize || index == V::Mana as usize {
            (diff - 15).max(10)
        } else if index == V::Wisdom as usize {
            (diff - 25).max(10)
        } else if index == V::Intelligence as usize
            || index == V::Agility as usize
            || index == V::Strength as usize
        {
            (diff - 5).max(10)
        } else if index == V::Hand as usize
            || index == V::Attack as usize
            || index == V::Parry as usize
            || index == V::Immunity as usize
        {
            diff.max(1)
        } else if index == V::ArmorSkill as usize {
            ((diff / 10) * 10).max(1)
        } else if index == V::Tactics as usize {
            (diff - 5).max(1)
        } else if index == V::Warcry as usize {
            (diff - 15).max(1)
        } else if index == V::Surround as usize
            || index == V::BodyControl as usize
            || index == V::SpeedSkill as usize
        {
            (diff - 20).max(1)
        } else if index == V::Percept as usize {
            (diff - 10).max(1)
        } else if index == V::Bless as usize
            || index == V::Fireball as usize
            || index == V::Freeze as usize
            || index == V::MagicShield as usize
        {
            (diff - 5).max(1)
        } else {
            (diff - 30).max(1)
        };
    val.min(250) as i16
}

/// C `build_fighter`'s big-boss special-item tier ladder
/// (`missions.c:802-837`): `(strength, base)` fed into
/// `World::create_special_item`.
pub fn special_item_tier_for_level(level: i32) -> (i32, i32) {
    const TIERS: [(i32, i32, i32); 10] = [
        (10, 3, 1),
        (17, 4, 10),
        (24, 6, 20),
        (31, 7, 30),
        (38, 8, 40),
        (45, 10, 50),
        (52, 12, 60),
        (60, 14, 70),
        (66, 16, 80),
        (74, 18, 90),
    ];
    for (bound, strength, base) in TIERS {
        if level < bound {
            return (strength, base);
        }
    }
    (20, 90)
}

/// C `mission_status(cn, ppd)` (`missions.c:867-895`).
pub fn mission_status_lines(ppd: &MissionPpd, title: &str, md: &MissionFighterData) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = 3;
    lines.push(format!("#{line}0{title}"));
    line += 1;

    let kill_line = |lines: &mut Vec<String>, line: &mut i32, done: i32, total: i32, name: &str| {
        if total > done {
            let n = total - done;
            lines.push(format!(
                "#{}0- Kill {} {}{}{}",
                *line,
                n,
                md.basename,
                name,
                if n != 1 { "s" } else { "" }
            ));
            *line += 1;
        }
    };
    kill_line(
        &mut lines,
        &mut line,
        ppd.kill_easy[0],
        ppd.kill_easy[1],
        md.strength_names[0],
    );
    kill_line(
        &mut lines,
        &mut line,
        ppd.kill_normal[0],
        ppd.kill_normal[1],
        md.strength_names[1],
    );
    kill_line(
        &mut lines,
        &mut line,
        ppd.kill_hard[0],
        ppd.kill_hard[1],
        md.strength_names[2],
    );

    if ppd.kill_boss[1] > ppd.kill_boss[0] {
        let n = ppd.kill_boss[1] - ppd.kill_boss[0];
        lines.push(format!(
            "#{line}0- Kill {n} {}{}",
            md.bossname,
            if n != 1 { "s" } else { "" }
        ));
        line += 1;
    }
    if ppd.find_item[1] > ppd.find_item[0] {
        lines.push(format!("#{line}0- Find {}", md.itemname.unwrap_or("")));
        line += 1;
    }
    while line < 9 {
        lines.push(format!("#{line}0"));
        line += 1;
    }
    lines
}

/// C `mission_fighter_dead`'s kill-counter `switch (nr)` (`missions.c:
/// 1865-1876`), `nr` being `ch[cn].deaths` (the dying fighter's `fID`
/// fighter-tier tag, see [`FighterSpawnSpec::fighter_kind`]). Every arm
/// is `min(done + 1, total)`, so an already-complete counter (or an
/// out-of-range `fighter_kind`, C's own `switch` default: no `case`, a
/// silent no-op) never overflows past its `[1]` total.
pub fn record_mission_fighter_kill(ppd: &mut MissionPpd, fighter_kind: u8) {
    let counter = match fighter_kind {
        1 => &mut ppd.kill_easy,
        2 => &mut ppd.kill_normal,
        3 => &mut ppd.kill_hard,
        4 => &mut ppd.kill_boss,
        _ => return,
    };
    counter[0] = (counter[0] + 1).min(counter[1]);
}

/// C `mission_done(cn, ppd)` (`missions.c:922-940`): once every objective
/// (`kill_easy`/`kill_normal`/`kill_hard`/`kill_boss`/`find_item`) is
/// complete, promote `ppd->active` to `ppd->solved` and clear `active`.
/// Returns `true` only when this call is the one that flips a still-
/// active job to solved (C's own `if (ppd->active) { ... }` guard - a
/// mission with `active == 0` already, or one that just got its very
/// first slot rolled with everything still at `0/0`, never re-triggers
/// the "finished the job" message on every subsequent call once already
/// solved).
pub fn try_solve_mission(ppd: &mut MissionPpd) -> bool {
    if ppd.kill_easy[1] > ppd.kill_easy[0]
        || ppd.kill_normal[1] > ppd.kill_normal[0]
        || ppd.kill_hard[1] > ppd.kill_hard[0]
        || ppd.kill_boss[1] > ppd.kill_boss[0]
        || ppd.find_item[1] > ppd.find_item[0]
    {
        return false;
    }
    if ppd.active != 0 {
        ppd.solved = ppd.active;
        ppd.active = 0;
        return true;
    }
    false
}

fn write_mission_key_id(driver_data: &mut Vec<u8>, key_id: u32) {
    if driver_data.len() < 5 {
        driver_data.resize(5, 0);
    }
    driver_data[1..5].copy_from_slice(&key_id.to_le_bytes());
}

impl World {
    /// C `start_mission(cn, co, idx, ppd)` (`missions.c:950-1130`), minus
    /// `build_fighter`'s actual character creation (deferred to
    /// `ugaris-server`, see the module doc comment) and the trailing
    /// `mission_status`/`teleport_char_driver` calls (both callable
    /// directly by `World`, left to the caller in `governor.rs` since
    /// they don't belong to "planning").
    pub fn plan_start_mission(
        &mut self,
        idx: usize,
        ppd: &mut MissionPpd,
    ) -> Result<MissionStartPlan, MissionStartError> {
        let slot = ppd.sm[idx];
        let md_index = slot.mdidx.clamp(0, MISSION_FIGHTER_DATA.len() as i32 - 1) as usize;
        let md = &MISSION_FIGHTER_DATA[md_index];

        let mut chosen = None;
        for n in 0..6u16 {
            if n == 5 && md.area == 5 {
                break;
            }
            let fx = 1 + md.area * 41;
            let fy = 1 + n * 41;
            let tx = fx + 40;
            let ty = fy + 40;
            let mut busy = false;
            'scan: for x in fx..=tx {
                for y in fy..=ty {
                    let Some(tile) = self.map.tile(usize::from(x), usize::from(y)) else {
                        continue;
                    };
                    if tile.character == 0 {
                        continue;
                    }
                    let occupant = CharacterId(u32::from(tile.character));
                    if self
                        .characters
                        .get(&occupant)
                        .is_some_and(|character| character.flags.contains(CharacterFlags::PLAYER))
                    {
                        busy = true;
                        break 'scan;
                    }
                }
            }
            if !busy {
                chosen = Some((fx, fy, tx, ty));
                break;
            }
        }
        let Some((fx, fy, tx, ty)) = chosen else {
            return Err(MissionStartError::AllSlicesBusy);
        };

        ppd.active = (idx + 1) as i32;
        ppd.solved = 0;
        ppd.md_idx = md_index as i32;
        ppd.mcnt += 1;
        let key_id = make_item_id(DEV_ID_MISSION, (ppd.mcnt * 3) as u32);

        // C: count key-carrying fighter markers.
        let mut key1 = 0i32;
        let mut key2 = 0i32;
        let mut key3 = 0i32;
        for x in fx..=tx {
            for y in fy..=ty {
                let Some(item) = self.tile_item(x, y) else {
                    continue;
                };
                if item.template_id != IID_MISSIONFIGHTER {
                    continue;
                }
                match item.driver_data.first().copied().unwrap_or(0) {
                    5 => key1 += 1,
                    6 => key2 += 1,
                    7 => key3 += 1,
                    _ => {}
                }
            }
        }
        let mut key1 = self.roll_legacy_random(key1.max(0) as u32) as i32 + 1;
        let mut key2 = self.roll_legacy_random(key2.max(0) as u32) as i32 + 1;
        let mut key3 = self.roll_legacy_random(key3.max(0) as u32) as i32 + 1;

        let mut fighters = Vec::new();
        let mut easy = 0;
        let mut normal = 0;
        let mut hard = 0;
        let mut boss = 0;
        let mut chest = 0;
        let mut entry = (0u16, 0u16);

        for x in fx..=tx {
            for y in fy..=ty {
                if let Some(tile) = self.map.tile(usize::from(x), usize::from(y)) {
                    if tile.character != 0 {
                        let occupant = CharacterId(u32::from(tile.character));
                        let is_player = self.characters.get(&occupant).is_some_and(|character| {
                            character.flags.contains(CharacterFlags::PLAYER)
                        });
                        if !is_player {
                            self.remove_and_destroy_character(occupant);
                        }
                    }
                }

                let Some(item_id) = self.tile_item_id(x, y) else {
                    continue;
                };
                let Some(item) = self.items.get(&item_id) else {
                    continue;
                };
                if item.flags.contains(ItemFlags::TAKE) || item.content_id != 0 {
                    self.destroy_item(item_id);
                    continue;
                }
                let template_id = item.template_id;
                let driver_data_0 = item.driver_data.first().copied().unwrap_or(0);
                let item_pos = (item.x, item.y);

                if template_id == IID_MISSIONFIGHTER {
                    match driver_data_0 {
                        1 => {
                            fighters.push(FighterSpawnSpec {
                                x,
                                y,
                                diff: slot.difficulty / 3,
                                key_id: 0,
                                key_name: "",
                                name: format!("{}{}", md.basename, md.strength_names[0]),
                                temp: md.temp,
                                desc: md.basedesc,
                                fighter_kind: 1,
                                sprite: md.sprite,
                                has_special_item: false,
                                extra_flags: md.char_flags,
                            });
                            easy += 1;
                        }
                        2 => {
                            fighters.push(FighterSpawnSpec {
                                x,
                                y,
                                diff: slot.difficulty / 3 + 1,
                                key_id: 0,
                                key_name: "",
                                name: format!("{}{}", md.basename, md.strength_names[1]),
                                temp: md.temp,
                                desc: md.basedesc,
                                fighter_kind: 2,
                                sprite: md.sprite,
                                has_special_item: false,
                                extra_flags: md.char_flags,
                            });
                            normal += 1;
                        }
                        3 => {
                            fighters.push(FighterSpawnSpec {
                                x,
                                y,
                                diff: slot.difficulty / 3 + 2,
                                key_id: 0,
                                key_name: "",
                                name: format!("{}{}", md.basename, md.strength_names[2]),
                                temp: md.temp,
                                desc: md.basedesc,
                                fighter_kind: 3,
                                sprite: md.sprite,
                                has_special_item: false,
                                extra_flags: md.char_flags,
                            });
                            hard += 1;
                        }
                        4 => {
                            if self.roll_legacy_random(10) != 0 {
                                fighters.push(FighterSpawnSpec {
                                    x,
                                    y,
                                    diff: slot.difficulty / 3 + 3,
                                    key_id: 0,
                                    key_name: "",
                                    name: md.bossname.to_string(),
                                    temp: md.bosstemp,
                                    desc: md.bossdesc,
                                    fighter_kind: 4,
                                    sprite: md.bosssprite,
                                    has_special_item: false,
                                    extra_flags: md.char_flags,
                                });
                            } else {
                                fighters.push(FighterSpawnSpec {
                                    x,
                                    y,
                                    diff: slot.difficulty / 3 + 5,
                                    key_id: 0,
                                    key_name: "",
                                    name: md.bigbossname.to_string(),
                                    temp: md.bosstemp,
                                    desc: md.bigbossdesc,
                                    fighter_kind: 4,
                                    sprite: md.bosssprite,
                                    has_special_item: true,
                                    extra_flags: md.char_flags,
                                });
                            }
                            boss += 1;
                        }
                        5 => {
                            key1 -= 1;
                            fighters.push(FighterSpawnSpec {
                                x,
                                y,
                                diff: slot.difficulty / 3 + 1,
                                key_id: if key1 == 0 { key_id } else { 0 },
                                key_name: "Door Key I",
                                name: format!("{}{}", md.basename, md.strength_names[1]),
                                temp: md.temp,
                                desc: md.basedesc,
                                fighter_kind: 2,
                                sprite: md.sprite,
                                has_special_item: false,
                                extra_flags: md.char_flags,
                            });
                            normal += 1;
                        }
                        6 => {
                            key2 -= 1;
                            fighters.push(FighterSpawnSpec {
                                x,
                                y,
                                diff: slot.difficulty / 3 + 1,
                                key_id: if key2 == 0 { key_id + 1 } else { 0 },
                                key_name: "Door Key II",
                                name: format!("{}{}", md.basename, md.strength_names[1]),
                                temp: md.temp,
                                desc: md.basedesc,
                                fighter_kind: 2,
                                sprite: md.sprite,
                                has_special_item: false,
                                extra_flags: md.char_flags,
                            });
                            normal += 1;
                        }
                        7 => {
                            key3 -= 1;
                            let carries_key = md.itemname.is_some() && key3 == 0;
                            fighters.push(FighterSpawnSpec {
                                x,
                                y,
                                diff: slot.difficulty / 3 + 1,
                                key_id: if carries_key { key_id + 2 } else { 0 },
                                key_name: "Chest Key",
                                name: format!("{}{}", md.basename, md.strength_names[1]),
                                temp: md.temp,
                                desc: md.basedesc,
                                fighter_kind: 2,
                                sprite: md.sprite,
                                has_special_item: false,
                                extra_flags: md.char_flags,
                            });
                            normal += 1;
                        }
                        _ => {}
                    }
                }
                if template_id == IID_MISSIONCHEST && md.itemname.is_some() {
                    if let Some(item) = self.items.get_mut(&item_id) {
                        write_mission_key_id(&mut item.driver_data, key_id + 2);
                    }
                    chest += 1;
                }
                if template_id == IID_MISSIONDOOR1 {
                    if let Some(item) = self.items.get_mut(&item_id) {
                        write_mission_key_id(&mut item.driver_data, key_id);
                    }
                }
                if template_id == IID_MISSIONDOOR2 {
                    if let Some(item) = self.items.get_mut(&item_id) {
                        write_mission_key_id(&mut item.driver_data, key_id + 1);
                    }
                }
                if template_id == IID_MISSIONENTRY {
                    entry = item_pos;
                    if let Some(item) = self.items.get_mut(&item_id) {
                        item.sprite = 0;
                    }
                }
            }
        }

        ppd.kill_easy = [0, easy];
        ppd.kill_normal = [0, normal];
        ppd.kill_hard = [0, hard];
        ppd.kill_boss = [0, boss];
        ppd.find_item = [0, chest];

        Ok(MissionStartPlan { entry, fighters })
    }

    fn tile_item_id(&self, x: u16, y: u16) -> Option<ItemId> {
        let tile = self.map.tile(usize::from(x), usize::from(y))?;
        (tile.item != 0).then_some(ItemId(tile.item))
    }

    fn tile_item(&self, x: u16, y: u16) -> Option<&Item> {
        let item_id = self.tile_item_id(x, y)?;
        self.items.get(&item_id)
    }

    fn remove_and_destroy_character(&mut self, character_id: CharacterId) {
        let carried: Vec<ItemId> = self
            .characters
            .get(&character_id)
            .map(|character| {
                character
                    .inventory
                    .iter()
                    .flatten()
                    .copied()
                    .chain(character.cursor_item)
                    .collect()
            })
            .unwrap_or_default();
        for item_id in carried {
            self.destroy_item(item_id);
        }
        self.remove_character(character_id);
    }
}
