//! `CDR_FDEMON_ARMY` (`src/area/8/fdemon.c:44` in `drvlib.h`, driver id
//! `44`): the recruitable-soldier system. A player who reaches a high
//! enough army rank while talking to the Commander (`CDR_FDEMON_BOSS`, see
//! `fdemon_boss.rs`) can "take" up to `MAXSOLDIER` companion NPCs
//! (`army1s`=warrior/`army2s`=mage character templates) that then follow
//! the player, hold formation, fight, gain experience/rank, and produce
//! randomized "emote" chatter driven by a per-soldier personality profile.
//!
//! This module currently ports the pure, spawning-independent slices of
//! C's `take_soldiers`/`assign_profile`/`update_soldier`
//! (`fdemon.c:384-446`): the static `profile[]` table (`fdemon.c:313-320`),
//! the type/profile eligibility logic that decides *which* soldier(s)
//! become newly recruitable at a given army rank, `update_soldier`'s
//! per-skill stat scaling ([`scale_soldier_values`]), its skill-tiered
//! equipment selection ([`soldier_equipment_items`]), and its exp/level
//! recompute ([`finalize_soldier_exp_and_level`], reusing the now-ported
//! `crate::world::calc_exp`/`exp2level`, `world/exp.rs`). This is
//! deliberately reusable ahead of the actual `World`-side spawning
//! integration (real `Character`/`Item` creation via
//! `ZoneLoader::instantiate_character_template("army1s"/"army2s", ..)`,
//! `drop_char` placement, `farmy_data` follow-state initialization) - see
//! `PORTING_TODO.md`'s Area 8 entry for the full remaining scope
//! (`take_soldiers`/`drop_soldiers` spawning, `army_follow_driver`/
//! `army_front_driver`/`army_back_driver`/`army_behind_driver` formation
//! AI, `find_platoon`/`platoon_exp`'s soldier-exp half, `do_emote`/
//! `got_emote`'s emote engine, and the `it_driver` item triggers that call
//! `take_soldiers`/`drop_soldiers`).
//!
//! The companion `struct soldier` (`type`/`rank`/`base`/`profile`/`exp`/
//! `cn`/`serial`) PPD fields already round-trip through the legacy
//! `farmy_ppd` blob via `PlayerRuntime::farmy_soldier_type`/`_rank`/`_base`/
//! `_profile`/`_exp`/`_cn`/`_serial` (`player/areas_misc.rs`).
//!
//! This module additionally ports [`World::army_follow_driver`]/
//! [`World::army_back_driver`]/[`World::army_front_driver`]/
//! [`World::army_behind_driver`] (C `army_follow_driver`/
//! `army_back_driver`/`army_front_driver`/`army_behind_driver`,
//! `fdemon.c:633-705`), [`World::fdemon_army_process_text_messages`] (the
//! `NT_TEXT` mission-command reception half of C `fdemon_army`,
//! `fdemon.c:1370-1423`, minus the emote-reaction dispatch), and
//! [`World::fdemon_army_tick`] (the mission-dispatch/leader-lost-
//! disintegration slice of C `fdemon_army`, `fdemon.c:1327-1532`) - enough
//! for a recruited soldier to follow, hold position ("back"), stay close
//! ("retreat"), walk ahead of ("front"), or take up a flanking attack
//! position behind whatever the leader is facing ("behind") on command.
//! The real spawning (`take_soldiers`/`drop_soldiers`, needing
//! `ZoneLoader`) lives in `ugaris-server`'s `area8_army.rs`, matching the
//! `pents.rs`/`world/pents.rs` split precedent.
//!
//! Deviations/gaps still open in this slice (documented, not silent):
//! - Combat: `fight_driver_update`/`do_heal`/`do_bless`/
//!   `fight_driver_attack_visible` (self-defense, the heal/bless support-
//!   caster behavior gated on `V_HEAL`/`V_BLESS`) are not ported - a
//!   soldier will follow but never fight back if attacked.
//! - The whole `struct emote` personality/chat system (`do_emote`/
//!   `got_emote`, `fdemon.c:781-1325`) is not ported - `FarmyData` omits
//!   the `emote` field entirely, the `NT_TEXT` handler's `res >= 20`
//!   emote-reaction dispatch and case `7`'s emote-stats debug dump are
//!   both skipped; `regenerate_driver`/`spell_self_driver` are likewise
//!   not called per-soldier (regen already applies generically to every
//!   character - see `world/regen.rs` - so HP/endurance/mana recovery
//!   still works without this).
//! - C's `NT_CREATE`'s `if (ch[cn].arg) ch[cn].arg = NULL;` has no Rust
//!   equivalent (same precedent as every other simple NPC in this
//!   codebase).

use crate::{
    character_driver::{analyse_text_qa, CharacterDriverState, TextAnalysisOutcome, NT_TEXT},
    world::*,
};

use super::FDEMON_QA;

/// C `analyse_text_driver`'s own `char_dist(cn, co) > 12` early-out
/// (`fdemon.c:209-211`), same range every other area-8 driver's `NT_TEXT`
/// handling reproduces (see `fdemon_boss.rs`'s `FDEMON_BOSS_TALK_RANGE`).
const FDEMON_ARMY_TALK_RANGE: i32 = 12;

/// C `#define MAXSOLDIER 3` (`fdemon.c:322`).
pub const MAXSOLDIER: usize = 3;

/// C `struct soldier::type` values (`fdemon.c:347`).
pub const SOLDIER_TYPE_WARRIOR: i32 = 1;
pub const SOLDIER_TYPE_MAGE: i32 = 2;

/// One row of C's `struct profile profile[]` table (`fdemon.c:301-320`).
pub struct SoldierProfile {
    pub name: &'static str,
    pub gender: char,
    pub cuddly: i32,
    pub angst: i32,
    pub bore: i32,
    pub bigmouth: i32,
    pub sprite: i32,
}

/// C `struct profile profile[]` (`fdemon.c:313-320`), digit-for-digit.
pub const SOLDIER_PROFILES: [SoldierProfile; 14] = [
    SoldierProfile {
        name: "Bert",
        gender: 'M',
        cuddly: 0,
        angst: 5,
        bore: 15,
        bigmouth: 10,
        sprite: 158,
    },
    SoldierProfile {
        name: "Josh",
        gender: 'M',
        cuddly: 20,
        angst: 10,
        bore: 20,
        bigmouth: 5,
        sprite: 160,
    },
    SoldierProfile {
        name: "Will",
        gender: 'M',
        cuddly: 10,
        angst: 20,
        bore: 5,
        bigmouth: 10,
        sprite: 162,
    },
    SoldierProfile {
        name: "James",
        gender: 'M',
        cuddly: 0,
        angst: 15,
        bore: 10,
        bigmouth: 20,
        sprite: 164,
    },
    SoldierProfile {
        name: "Carl",
        gender: 'M',
        cuddly: 25,
        angst: 5,
        bore: 5,
        bigmouth: 15,
        sprite: 166,
    },
    SoldierProfile {
        name: "Jim",
        gender: 'M',
        cuddly: 5,
        angst: 15,
        bore: 5,
        bigmouth: 10,
        sprite: 168,
    },
    SoldierProfile {
        name: "Brad",
        gender: 'M',
        cuddly: 0,
        angst: 5,
        bore: 15,
        bigmouth: 5,
        sprite: 170,
    },
    SoldierProfile {
        name: "Jenny",
        gender: 'F',
        cuddly: 25,
        angst: 25,
        bore: 5,
        bigmouth: 5,
        sprite: 176,
    },
    SoldierProfile {
        name: "Sarah",
        gender: 'F',
        cuddly: 10,
        angst: 15,
        bore: 15,
        bigmouth: 15,
        sprite: 178,
    },
    SoldierProfile {
        name: "Sue",
        gender: 'F',
        cuddly: 0,
        angst: 5,
        bore: 10,
        bigmouth: 25,
        sprite: 180,
    },
    SoldierProfile {
        name: "Peggy",
        gender: 'F',
        cuddly: 15,
        angst: 10,
        bore: 20,
        bigmouth: 5,
        sprite: 182,
    },
    SoldierProfile {
        name: "Mary",
        gender: 'F',
        cuddly: 0,
        angst: 20,
        bore: 5,
        bigmouth: 10,
        sprite: 184,
    },
    SoldierProfile {
        name: "Clara",
        gender: 'F',
        cuddly: 5,
        angst: 5,
        bore: 5,
        bigmouth: 15,
        sprite: 186,
    },
    SoldierProfile {
        name: "Beth",
        gender: 'F',
        cuddly: 1,
        angst: 10,
        bore: 15,
        bigmouth: 10,
        sprite: 188,
    },
];

/// C `assign_profile`'s emote-base assignment (`fdemon.c:384-392`): the
/// four personality-tendency fields carried from `profile[nr]` into a fresh
/// `struct emote` (`cuddly`/`angst`/`bore`/`bigmouth`; the "current" fields
/// - `lonely`/`fear`/`boredom`/`praise` - and the `likes`/`talked`/
/// `answer_*`/`last_emote` fields all start at `0` via C's `bzero`).
pub struct SoldierEmoteBase {
    pub cuddly: i32,
    pub angst: i32,
    pub bore: i32,
    pub bigmouth: i32,
}

/// C `assign_profile(slot, nr, ppd)` (`fdemon.c:384-392`), minus the
/// `ppd`-mutation (callers write `profile` + this return value into the
/// PPD/spawned-soldier state themselves).
pub fn assign_profile(profile_index: usize) -> SoldierEmoteBase {
    let profile = &SOLDIER_PROFILES[profile_index];
    SoldierEmoteBase {
        cuddly: profile.cuddly,
        angst: profile.angst,
        bore: profile.bore,
        bigmouth: profile.bigmouth,
    }
}

/// C `update_soldier`'s `ppd->soldier[n].base = 43 + ppd->soldier[n].rank *
/// 4` (`fdemon.c:408`).
pub fn soldier_base_strength(rank: i32) -> i32 {
    43 + rank * 4
}

/// Worn inventory slot indices matching C's `WN_*` (`server.h:369-381`),
/// only the slots `update_soldier`'s equipment block actually uses.
pub const WN_HEAD: usize = 1;
pub const WN_ARMS: usize = 3;
pub const WN_BODY: usize = 4;
pub const WN_RHAND: usize = 6;
pub const WN_LEGS: usize = 7;

/// C `update_soldier`'s per-skill scaling branch (`fdemon.c:410-417`): the
/// throwaway `cc` template instance's `value[1][m]` marker (`1`/`2`/`3`,
/// from the `army1s`/`army2s` template's literal `V_*=n` fields - see the
/// module doc comment) selects which of three `base`-derived formulas the
/// real soldier's skill value becomes. Any other marker (usually `0`)
/// means "leave this skill at whatever the soldier character already
/// has" - `None` here, matching C's fallthrough (no `else` branch writes
/// `ch[co].value[1][m]`).
pub fn scale_soldier_skill(template_marker: i32, base: i32) -> Option<i32> {
    match template_marker {
        1 => Some(base / 2),
        2 => Some(base - 5),
        3 => Some(base),
        _ => None,
    }
}

/// Applies [`scale_soldier_skill`] across a full `value[1]` array (C's
/// `for (m = 0; m < V_MAX; m++)` loop, `fdemon.c:410-417`).
/// `template_markers` is the `army1s`/`army2s` template's own `value[1]`
/// (its literal `V_*=n` fields double as tier markers); `current` is the
/// actual soldier character's `value[1]` array, mutated in place. Arrays
/// shorter than `V_MAX` (43) are supported for testing convenience; extra
/// entries beyond either array's length are ignored, matching a `V_MAX`-
/// bounded loop over the shorter of the two in practice (both are always
/// `CHARACTER_VALUE_COUNT`-sized in real use).
pub fn scale_soldier_values(template_markers: &[i32], base: i32, current: &mut [i32]) {
    for (marker, value) in template_markers.iter().zip(current.iter_mut()) {
        if let Some(scaled) = scale_soldier_skill(*marker, base) {
            *value = scaled;
        }
    }
}

/// C `update_soldier`'s equipment `sprintf`+`create_item` block
/// (`fdemon.c:423-440`): returns `(worn_slot, item_template_key)` pairs to
/// instantiate and equip, keyed off the *already-scaled* `value[1]`
/// (`armor_skill`/`sword_skill`/`dagger_skill`, all C's `.../10+1` tier
/// picks - `ugaris_data/zones/generic/armor.itm`'s `sleeves`/`armor`/
/// `helmet`/`leggings`/`sword`/`dagger` templates go up to tier 10).
/// Warrior (`soldier_type == SOLDIER_TYPE_WARRIOR`) gets a five-piece
/// armor-skill-tiered kit plus a sword-skill-tiered sword; anything else
/// (mage, `SOLDIER_TYPE_MAGE`) gets only a dagger-skill-tiered dagger,
/// matching C's `if (type == 1) {...} else {...}`.
pub fn soldier_equipment_items(
    soldier_type: i32,
    armor_skill: i32,
    sword_skill: i32,
    dagger_skill: i32,
) -> Vec<(usize, String)> {
    if soldier_type == SOLDIER_TYPE_WARRIOR {
        let armor_tier = armor_skill / 10 + 1;
        let sword_tier = sword_skill / 10 + 1;
        vec![
            (WN_ARMS, format!("sleeves{armor_tier}q1")),
            (WN_BODY, format!("armor{armor_tier}q1")),
            (WN_HEAD, format!("helmet{armor_tier}q1")),
            (WN_LEGS, format!("leggings{armor_tier}q1")),
            (WN_RHAND, format!("sword{sword_tier}q1")),
        ]
    } else {
        let dagger_tier = dagger_skill / 10 + 1;
        vec![(WN_RHAND, format!("dagger{dagger_tier}q1"))]
    }
}

/// C `update_soldier`'s exp/level recompute
/// (`fdemon.c:421-422`: `ch[co].exp = ch[co].exp_used = calc_exp(co);
/// ch[co].level = exp2level(ch[co].exp);`), called after
/// [`scale_soldier_values`] has written the soldier's freshly-scaled
/// `value[1]` array so `exp`/`level` stay consistent with what a player
/// would have had to spend to reach those skill values.
pub fn finalize_soldier_exp_and_level(character: &mut crate::entity::Character) {
    let exp = crate::world::calc_exp(character);
    character.exp = exp;
    character.exp_used = exp;
    character.level = crate::world::exp2level(exp);
}

/// One newly-eligible recruit slot, as planned by [`plan_soldier_recruitment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoldierRecruitPlan {
    pub slot: usize,
    pub soldier_type: i32,
    pub profile: usize,
}

/// C `take_soldiers`'s type/profile eligibility loop (`fdemon.c:467-514`),
/// minus the actual character spawning (`create_char`/`update_soldier`/
/// `update_char`/`drop_char`/`set_data`) - see the module doc comment for
/// what's still deferred. Returns one [`SoldierRecruitPlan`] per slot that
/// is *newly* recruitable this call (`existing_type[slot] == 0` and the
/// slot's rank threshold is met); slots already occupied
/// (`existing_type[slot] != 0`) or not yet unlocked are `None`, matching
/// C's `!ppd->soldier[n].type` guard - a caller re-invoking this with the
/// same `existing_type`/`existing_profile` state is a no-op, so it is safe
/// to call every time the player's army rank might have changed rather
/// than only once.
///
/// `existing_profile` still participates in slot 1/2's uniqueness `do
/// {...} while` loops (`fdemon.c:494-499,507-512`) even for slots not
/// newly assigned this call, exactly like C reading `ppd->soldier[m].
/// profile` regardless of whether that slot was assigned in a prior call
/// or this one.
pub fn plan_soldier_recruitment(
    army_rank: i32,
    is_warrior: bool,
    is_male: bool,
    existing_type: [i32; MAXSOLDIER],
    existing_profile: [i32; MAXSOLDIER],
    mut random_below: impl FnMut(u32) -> u32,
) -> [Option<SoldierRecruitPlan>; MAXSOLDIER] {
    let total = SOLDIER_PROFILES.len() as u32;
    let half = total / 2;
    let mut profile = existing_profile;
    let mut plans: [Option<SoldierRecruitPlan>; MAXSOLDIER] = [None, None, None];

    // n == 0 (fdemon.c:468-480): rank > 0, no uniqueness check (first slot).
    // Note the profile roll here is `RANDOM(ARRAYSIZE(profile)) / 2` (full-
    // range roll, then halved) - NOT `RANDOM(ARRAYSIZE(profile) / 2)` like
    // slots 1/2 below. Ported digit-for-digit despite the asymmetry.
    if army_rank > 0 && existing_type[0] == 0 {
        let soldier_type = if is_warrior {
            SOLDIER_TYPE_MAGE
        } else {
            SOLDIER_TYPE_WARRIOR
        };
        let pro = if is_male {
            random_below(total) / 2 + half
        } else {
            random_below(total) / 2
        };
        profile[0] = pro as i32;
        plans[0] = Some(SoldierRecruitPlan {
            slot: 0,
            soldier_type,
            profile: pro as usize,
        });
    }

    // n == 1 (fdemon.c:481-501): rank > 4, unique against slot 0.
    if army_rank > 4 && existing_type[1] == 0 {
        let soldier_type = if is_warrior {
            SOLDIER_TYPE_WARRIOR
        } else {
            SOLDIER_TYPE_MAGE
        };
        let mut pro;
        loop {
            pro = if is_male {
                random_below(half)
            } else {
                random_below(half) + half
            };
            if profile[0] != pro as i32 {
                break;
            }
        }
        profile[1] = pro as i32;
        plans[1] = Some(SoldierRecruitPlan {
            slot: 1,
            soldier_type,
            profile: pro as usize,
        });
    }

    // n == 2 (fdemon.c:502-514): rank > 6, full-range pick, unique against
    // slots 0 and 1 (no gender restriction, matching C).
    if army_rank > 6 && existing_type[2] == 0 {
        let mut pro;
        loop {
            pro = random_below(total);
            if profile[0] != pro as i32 && profile[1] != pro as i32 {
                break;
            }
        }
        plans[2] = Some(SoldierRecruitPlan {
            slot: 2,
            soldier_type: SOLDIER_TYPE_WARRIOR,
            profile: pro as usize,
        });
    }

    plans
}

/// C `#define MIS_FOLLOW 1` .. `#define MIS_FRONT 5` (`fdemon.c:627-631`).
pub const MIS_FOLLOW: i32 = 1;
pub const MIS_BACK: i32 = 2;
pub const MIS_RETREAT: i32 = 3;
pub const MIS_BEHIND: i32 = 4;
pub const MIS_FRONT: i32 = 5;

/// C `struct farmy_data` (`fdemon.c:370-382`), minus the `emote` field -
/// see the module doc comment for why the emote system is deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FarmyData {
    pub leader_cn: CharacterId,
    pub lx: u16,
    pub ly: u16,
    pub mission: i32,
    pub opt1: i32,
    pub opt2: i32,
    pub timer: i64,
    pub closeup: bool,
    /// C `int platoon[MAXSOLDIER + 1]`: slots `0..MAXSOLDIER` are the
    /// platoon's soldier character ids (`CharacterId(0)` for an empty
    /// slot), slot `MAXSOLDIER` is the leader.
    pub platoon: [CharacterId; MAXSOLDIER + 1],
}

impl Default for FarmyData {
    fn default() -> Self {
        FarmyData {
            leader_cn: CharacterId(0),
            lx: 0,
            ly: 0,
            mission: MIS_FOLLOW,
            opt1: 0,
            opt2: 0,
            timer: 0,
            closeup: false,
            platoon: [CharacterId(0); MAXSOLDIER + 1],
        }
    }
}

impl World {
    /// C `army_follow_driver(cn, dat, dist)` (`fdemon.c:633-655`): walks
    /// one step toward the leader (`min_dist=2` once the leader is
    /// visible, matching C's fixed `pathfinder(...,2,...)` call - `dist`
    /// only gates the "already close enough, don't move" early-out) or
    /// toward the last-known leader position (`min_dist=0`) when the
    /// leader isn't currently visible. Returns whether a walk action was
    /// queued (C's `return 1`/`return 0`).
    pub fn army_follow_driver(
        &mut self,
        character_id: CharacterId,
        dist: i32,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = character.driver_state.clone() else {
            return false;
        };
        let Some(leader) = self.characters.get(&dat.leader_cn) else {
            return false;
        };

        let daylight = self.date.daylight;
        if char_see_char(character, leader, &self.map, daylight) {
            let (lx, ly) = (leader.x, leader.y);
            let (cx, cy) = (character.x, character.y);
            if let Some(CharacterDriverState::FdemonArmy(dat)) = self
                .characters
                .get_mut(&character_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                dat.lx = lx;
                dat.ly = ly;
            }
            let manhattan =
                (i32::from(cx) - i32::from(lx)).abs() + (i32::from(cy) - i32::from(ly)).abs();
            if manhattan <= dist {
                return false;
            }
            self.setup_walk_toward(
                character_id,
                usize::from(lx),
                usize::from(ly),
                2,
                area_id,
                false,
            )
        } else {
            let (cx, cy) = (character.x, character.y);
            if cx == dat.lx && cy == dat.ly {
                return false;
            }
            self.setup_walk_toward(
                character_id,
                usize::from(dat.lx),
                usize::from(dat.ly),
                0,
                area_id,
                false,
            )
        }
    }

    /// C `army_back_driver(cn, dat)` (`fdemon.c:675-686`): if the soldier
    /// is still standing at the guard post recorded when the "back"
    /// command was issued (`dat->opt1`/`dat->opt2`), take exactly one
    /// step in the direction opposite its current facing (C `(ch[cn].dir
    /// + 3) % 8 + 1`, see [`opposite_direction`]) and return `true`
    /// immediately on success. Otherwise (already moved off the guard
    /// post, or the backward step is blocked) fall back to a timeout:
    /// after 5 seconds with no progress revert the mission to
    /// `MIS_FOLLOW` and return `false`; before that, idle for half a
    /// second and return whether the idle was queued (C `return
    /// do_idle(cn, TICKS/2)`).
    pub fn army_back_driver(&mut self, character_id: CharacterId, area_id: u16) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = character.driver_state.clone() else {
            return false;
        };

        if i32::from(character.x) == dat.opt1 && i32::from(character.y) == dat.opt2 {
            let direction = opposite_direction(character.dir);
            let weather_movement_percent = self.settings.weather_movement_percent;
            let earthmud_extra_cost = self.earthmud_extra_movement_cost(character_id);
            let walked = Direction::try_from(direction).is_ok_and(|direction| {
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| {
                        do_walk(
                            character,
                            &mut self.map,
                            direction as u8,
                            area_id,
                            weather_movement_percent,
                            earthmud_extra_cost,
                        )
                        .is_ok()
                    })
            });
            if walked {
                return true;
            }
        }

        if self.tick.0 as i64 - dat.timer > TICKS_PER_SECOND as i64 * 5 {
            if let Some(CharacterDriverState::FdemonArmy(dat)) = self
                .characters
                .get_mut(&character_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                dat.mission = MIS_FOLLOW;
            }
            false
        } else {
            self.characters
                .get_mut(&character_id)
                .is_some_and(|character| do_idle(character, TICKS_PER_SECOND as i32 / 2).is_ok())
        }
    }

    /// C `army_front_driver(cn, dat, dist)` (`fdemon.c:657-673`): walks
    /// one step toward a point 4 tiles ahead of the leader in its current
    /// facing direction (C `dx2offset(ch[co].dir,...)`, [`Direction::
    /// delta`]-equivalent, times 4, added to the leader's position).
    /// Returns whether a walk action was queued (C `return 1`/`return
    /// 0`): `false` if the leader isn't visible, the soldier is already
    /// within `dist` tiles of the target point, or no path is found.
    pub fn army_front_driver(
        &mut self,
        character_id: CharacterId,
        dist: i32,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = character.driver_state.clone() else {
            return false;
        };
        let Some(leader) = self.characters.get(&dat.leader_cn) else {
            return false;
        };

        let daylight = self.date.daylight;
        if !char_see_char(character, leader, &self.map, daylight) {
            return false;
        }
        let Ok(leader_direction) = Direction::try_from(leader.dir) else {
            return false;
        };
        let (dx, dy) = leader_direction.delta();
        let target_x = i32::from(leader.x) + i32::from(dx) * 4;
        let target_y = i32::from(leader.y) + i32::from(dy) * 4;
        if target_x < 0 || target_y < 0 {
            return false;
        }

        let manhattan =
            (i32::from(character.x) - target_x).abs() + (i32::from(character.y) - target_y).abs();
        if manhattan <= dist {
            return false;
        }

        self.setup_walk_toward(
            character_id,
            target_x as usize,
            target_y as usize,
            2,
            area_id,
            false,
        )
    }

    /// C `army_behind_driver(cn, dat)` (`fdemon.c:688-705`): positions the
    /// soldier directly behind whatever character (`co`) the leader is
    /// currently facing, then attacks it. `co` is found by looking up the
    /// map tile immediately in front of the leader (C's
    /// `dx2offset(ch[cc].dir, ...)`); the soldier's target tile is one
    /// step behind `co` in `co`'s own facing direction (C's `(ch[co].dir
    /// + 3) % 8 + 1`, the same [`opposite_direction`] helper
    /// [`World::army_back_driver`] uses). If the soldier isn't already
    /// standing there, C's `move_driver(cn, tx, ty, 0)` (ported as
    /// [`World::setup_walk_toward`], itself exactly `pathfinder` +
    /// `walk_or_use_driver`, i.e. `move_driver`'s own definition) is
    /// tried first; on success this returns `true` immediately without
    /// attacking this tick, matching C's early `return 1`. If the move
    /// fails, the soldier says "cannot go there" and its mission reverts
    /// to `MIS_FOLLOW`, but - matching C's lack of an early return there
    /// - execution still falls through to the attack attempt below.
    /// Returns whether an attack was queued (C's final `return
    /// do_attack(cn, ch[co].dir, co)`), or `false` if the leader or `co`
    /// can no longer be resolved. C's random `AC_ATTACK1 + RANDOM(3)`
    /// variant pick is not reproduced (matching the pre-existing
    /// `action::ATTACK1`-only simplification already used by every other
    /// `do_attack` caller in this codebase, e.g.
    /// `setup_simple_baddy_attack_driver`/`attack_driver_direct` in
    /// `world/npc_fight.rs`).
    pub fn army_behind_driver(&mut self, character_id: CharacterId, area_id: u16) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = character.driver_state.clone() else {
            return false;
        };
        let Some(leader) = self.characters.get(&dat.leader_cn) else {
            return false;
        };
        let Ok(leader_direction) = Direction::try_from(leader.dir) else {
            return false;
        };
        let (fdx, fdy) = leader_direction.delta();
        let Some(front_x) = offset_coordinate(usize::from(leader.x), fdx) else {
            return false;
        };
        let Some(front_y) = offset_coordinate(usize::from(leader.y), fdy) else {
            return false;
        };
        let target_tile_character = self
            .map
            .tile(front_x, front_y)
            .map(|tile| tile.character)
            .unwrap_or(0);
        if target_tile_character == 0 {
            return false;
        }
        let target_id = CharacterId(u32::from(target_tile_character));
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        let (target_x, target_y) = (target.x, target.y);
        let target_dir = target.dir;
        let Ok(behind_direction) = Direction::try_from(opposite_direction(target_dir)) else {
            return false;
        };
        let (bdx, bdy) = behind_direction.delta();
        let Some(behind_x) = offset_coordinate(usize::from(target_x), bdx) else {
            return false;
        };
        let Some(behind_y) = offset_coordinate(usize::from(target_y), bdy) else {
            return false;
        };

        let (cx, cy) = (character.x, character.y);
        if usize::from(cx) != behind_x || usize::from(cy) != behind_y {
            if self.setup_walk_toward(character_id, behind_x, behind_y, 0, area_id, false) {
                return true;
            }
            self.npc_say(character_id, "cannot go there");
            self.set_fdemon_army_mission(character_id, MIS_FOLLOW);
        }

        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(attacker) = self.characters.get_mut(&character_id) else {
            return false;
        };
        crate::do_action::do_attack(
            attacker,
            &self.map,
            &target,
            target_dir,
            action::ATTACK1,
            weather_movement_percent,
        )
        .is_ok()
    }

    /// C `fdemon_army`'s `NT_TEXT` message branch (`fdemon.c:1370-1423`),
    /// minus the `res >= 20` emote-reaction dispatch (`got_emote`) and
    /// case `7`'s emote-stats debug dump - both meaningless without the
    /// (unported) emote system, see the module doc comment. Only platoon
    /// members (the leader or a fellow recruited soldier, C
    /// `find_platoon`) can address this soldier at all; only the
    /// leader's own speech can issue a mission command.
    pub fn fdemon_army_process_text_messages(&mut self, soldier_id: CharacterId) {
        let Some(soldier_name) = self.characters.get(&soldier_id).map(|c| c.name.clone()) else {
            return;
        };
        let messages = self
            .characters
            .get_mut(&soldier_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        let daylight = self.date.daylight;
        for message in messages {
            if message.message_type != NT_TEXT {
                continue;
            }
            let speaker_id = CharacterId(message.dat3 as u32);
            if speaker_id == soldier_id {
                continue;
            }
            let Some(text) = message.text.as_deref() else {
                continue;
            };

            let Some(CharacterDriverState::FdemonArmy(dat)) = self
                .characters
                .get(&soldier_id)
                .and_then(|character| character.driver_state.clone())
            else {
                continue;
            };
            // C `if ((friend = find_platoon(co, dat)) == -1) { remove_
            // message(...); continue; }` - only platoon members (leader
            // or fellow soldiers) may address this soldier.
            if !dat.platoon.contains(&speaker_id) {
                continue;
            }

            let (Some(soldier), Some(speaker)) = (
                self.characters.get(&soldier_id),
                self.characters.get(&speaker_id),
            ) else {
                continue;
            };
            // C `analyse_text_driver`'s own `char_dist(cn, co) > 12` /
            // `!char_see_char(cn, co)` early-outs (`fdemon.c:209-215`).
            if char_dist(soldier, speaker) > FDEMON_ARMY_TALK_RANGE
                || !char_see_char(soldier, speaker, &self.map, daylight)
            {
                continue;
            }
            let speaker_name = speaker.name.clone();
            let speaker_military_points = speaker.military_points;
            let (soldier_x, soldier_y) = (soldier.x, soldier.y);

            let outcome = analyse_text_qa(text, &soldier_name, &speaker_name, FDEMON_QA);

            // C `if (co != dat->leader_cn) { remove_message(...); continue; }`
            // - only the leader's own speech may issue a mission command;
            // a fellow soldier's matching text is dropped here (C's own
            // `find_platoon` check above already filters out anyone not
            // on the platoon at all).
            if speaker_id != dat.leader_cn {
                continue;
            }

            let rank_name =
                army_rank_name(army_rank_for_points(speaker_military_points)).to_string();

            match outcome {
                TextAnalysisOutcome::Matched(2) => {
                    self.npc_say(soldier_id, &format!("Sir! Yes, Sir, {rank_name}, Sir!"));
                    self.set_fdemon_army_mission(soldier_id, MIS_FOLLOW);
                }
                TextAnalysisOutcome::Matched(3) => {
                    self.npc_say(soldier_id, &format!("Will do, {rank_name}."));
                    let tick = self.tick.0 as i64;
                    if let Some(CharacterDriverState::FdemonArmy(dat)) = self
                        .characters
                        .get_mut(&soldier_id)
                        .and_then(|character| character.driver_state.as_mut())
                    {
                        dat.mission = MIS_BACK;
                        dat.opt1 = i32::from(soldier_x);
                        dat.opt2 = i32::from(soldier_y);
                        dat.timer = tick;
                    }
                }
                TextAnalysisOutcome::Matched(4) => {
                    self.npc_say(soldier_id, &format!("Aye Aye {rank_name}, Sir."));
                    self.set_fdemon_army_mission(soldier_id, MIS_RETREAT);
                }
                TextAnalysisOutcome::Matched(5) => {
                    self.npc_say(soldier_id, &format!("So be it, {rank_name}."));
                    self.set_fdemon_army_mission(soldier_id, MIS_FRONT);
                }
                TextAnalysisOutcome::Matched(6) => {
                    self.npc_say(soldier_id, &format!("I'll go rub his back, {rank_name}."));
                    self.set_fdemon_army_mission(soldier_id, MIS_BEHIND);
                }
                TextAnalysisOutcome::Said(_)
                | TextAnalysisOutcome::Matched(_)
                | TextAnalysisOutcome::NoMatch => {}
            }
        }
    }

    fn set_fdemon_army_mission(&mut self, soldier_id: CharacterId, mission: i32) {
        if let Some(CharacterDriverState::FdemonArmy(dat)) = self
            .characters
            .get_mut(&soldier_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            dat.mission = mission;
        }
    }

    /// Every live `CDR_FDEMON_ARMY` character (C `ch_driver`'s
    /// `CDR_FDEMON_ARMY` case, `fdemon.c:3021,3070,3084` - only its
    /// `CDT_DRIVER` `fdemon_army(cn, ret, lastact)` tick call is ported
    /// here).
    pub fn fdemon_army_character_ids(&self) -> Vec<CharacterId> {
        self.characters
            .values()
            .filter(|character| {
                character.driver == crate::character_driver::CDR_FDEMON_ARMY
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect()
    }

    /// C `fdemon_army(cn, ret, lastact)` (`fdemon.c:1327-1532`) - the
    /// leader-lost-disintegration guard plus both mission-dispatch
    /// `switch (dat->mission)` blocks (`FOLLOW`/`BACK`/`RETREAT`/`FRONT`/
    /// `BEHIND`, the last via [`World::army_behind_driver`]), minus
    /// the combat/heal/bless self-defense fallback that sits between them
    /// in C and the trailing emote/regen/idle tail (also documented
    /// gaps). Returns `true` if the soldier disintegrated (leader lost -
    /// C's `remove_destroy_char(cn)`), matching [`World::
    /// remove_character`]'s own "did it exist" contract for the caller.
    pub fn fdemon_army_tick(&mut self, character_id: CharacterId, area_id: u16) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::FdemonArmy(dat)) = character.driver_state.clone() else {
            return false;
        };
        let group = character.group;

        // C `if (dat->leader_cn) { if (!(ch[dat->leader_cn].flags) ||
        // ch[dat->leader_cn].group != ch[cn].group) { remove_destroy_char
        // (cn); return; } ... }` - `dat->leader_cn` is always set at spawn
        // time by `take_soldiers` in this port (see `area8_army.rs`), so
        // the `if (dat->leader_cn)` outer guard is unreachable here.
        let leader_lost = match self.characters.get(&dat.leader_cn) {
            None => true,
            Some(leader) => leader.group != group,
        };
        if leader_lost {
            self.remove_character(character_id);
            return true;
        }

        // C's first `switch (dat->mission)` block (`fdemon.c:1447-1481`).
        match dat.mission {
            MIS_FOLLOW => {
                if self.army_follow_driver(character_id, 10, area_id) {
                    if let Some(CharacterDriverState::FdemonArmy(dat)) = self
                        .characters
                        .get_mut(&character_id)
                        .and_then(|character| character.driver_state.as_mut())
                    {
                        dat.closeup = true;
                    }
                    return false;
                }
            }
            MIS_BACK => {
                if self.army_back_driver(character_id, area_id) {
                    return false;
                }
            }
            MIS_RETREAT => {
                if self.army_follow_driver(character_id, 3, area_id) {
                    return false;
                }
            }
            MIS_BEHIND => {
                if self.army_behind_driver(character_id, area_id) {
                    return false;
                }
            }
            MIS_FRONT => {
                if self.army_front_driver(character_id, 10, area_id) {
                    return false;
                }
            }
            _ => {}
        }

        // C's combat/heal/bless/self-defense fallback
        // (`fight_driver_update`/`do_heal`/`do_bless`/
        // `fight_driver_attack_visible`) between the two mission-dispatch
        // blocks is not ported - see the module doc comment.

        // C's second `switch (dat->mission)` block (`fdemon.c:1500-1514`)
        // only has `FOLLOW`/`FRONT` cases.
        match dat.mission {
            MIS_FOLLOW => {
                if self.army_follow_driver(character_id, 3, area_id) {
                    return false;
                }
                if let Some(CharacterDriverState::FdemonArmy(dat)) = self
                    .characters
                    .get_mut(&character_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    dat.closeup = false;
                }
            }
            MIS_FRONT => {
                if self.army_front_driver(character_id, 3, area_id) {
                    return false;
                }
            }
            _ => {}
        }

        // Emotes/regen/`do_idle` tail not ported - see the module doc
        // comment (regen already applies generically to every character).
        false
    }
}

/// C `(ch[cn].dir + 3) % 8 + 1` (`fdemon.c:676`): the direction opposite
/// `dir` - used by [`World::army_back_driver`] to step backward from a
/// held guard post.
fn opposite_direction(dir: u8) -> u8 {
    (u32::from(dir) + 3) as u8 % 8 + 1
}

// Tests for this module's pure functions live in `world::tests::
// fdemon_army` (alongside the `World`-based `army_follow_driver`/
// `fdemon_army_tick` tests) - same "no in-file test module" convention as
// every other area-8 NPC file (`fdemon_boss.rs`/`fdemon_demon.rs`), to
// keep this driver/parser/QA file under the ~800-line guideline.
