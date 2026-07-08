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
//! become newly recruitable at a given army rank, and `update_soldier`'s
//! per-skill stat scaling ([`scale_soldier_values`]) plus its
//! skill-tiered equipment selection ([`soldier_equipment_items`]). This is
//! deliberately reusable ahead of the actual `World`-side spawning
//! integration (real `Character`/`Item` creation via
//! `ZoneLoader::instantiate_character_template("army1s"/"army2s", ..)`,
//! `calc_exp`/`exp2level` (not yet ported to core - see `world/exp.rs`),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_table_has_fourteen_entries_matching_c() {
        assert_eq!(SOLDIER_PROFILES.len(), 14);
        assert_eq!(SOLDIER_PROFILES[0].name, "Bert");
        assert_eq!(SOLDIER_PROFILES[0].sprite, 158);
        assert_eq!(SOLDIER_PROFILES[13].name, "Beth");
        assert_eq!(SOLDIER_PROFILES[13].sprite, 188);
    }

    #[test]
    fn assign_profile_carries_the_four_tendency_fields() {
        let emote = assign_profile(4); // Carl: cuddly 25, angst 5, bore 5, bigmouth 15
        assert_eq!(emote.cuddly, 25);
        assert_eq!(emote.angst, 5);
        assert_eq!(emote.bore, 5);
        assert_eq!(emote.bigmouth, 15);
    }

    #[test]
    fn soldier_base_strength_matches_c_formula() {
        assert_eq!(soldier_base_strength(1), 47);
        assert_eq!(soldier_base_strength(4), 59);
    }

    #[test]
    fn rank_zero_recruits_nobody() {
        let plans = plan_soldier_recruitment(0, true, true, [0; 3], [0; 3], |_| 0);
        assert!(plans.iter().all(Option::is_none));
    }

    #[test]
    fn rank_one_recruits_only_slot_zero_with_gendered_profile_range() {
        // Male: profile = RANDOM(14) / 2 + 7, i.e. upper half of the table.
        let plans = plan_soldier_recruitment(
            1,
            /* is_warrior */ true,
            /* is_male */ true,
            [0; 3],
            [0; 3],
            |below| {
                assert_eq!(below, 14);
                5
            },
        );
        assert_eq!(plans[1], None);
        assert_eq!(plans[2], None);
        let slot0 = plans[0].expect("slot 0 should be recruitable at rank 1");
        assert_eq!(slot0.slot, 0);
        // is_warrior true -> mage (C: `if (ch[cn].flags & CF_WARRIOR) type=2`).
        assert_eq!(slot0.soldier_type, SOLDIER_TYPE_MAGE);
        assert_eq!(slot0.profile, 5 / 2 + 7);

        // Female: profile = RANDOM(14) / 2, lower half.
        let plans = plan_soldier_recruitment(1, true, false, [0; 3], [0; 3], |below| {
            assert_eq!(below, 14);
            9
        });
        assert_eq!(plans[0].unwrap().profile, 9 / 2);
    }

    #[test]
    fn rank_five_recruits_slot_one_avoiding_slot_zero_profile() {
        // Slot 0 already recruited with profile 9 (upper half) in a
        // previous call; is_male=false here means slot 1's own roll is also
        // `RANDOM(7) + 7` (upper half), so a same-value roll can collide.
        let existing_type = [SOLDIER_TYPE_MAGE, 0, 0];
        let existing_profile = [9, 0, 0];
        // First roll (2 -> pro=9) collides with slot 0's profile (9),
        // second roll (5 -> pro=12) doesn't.
        let mut calls = 0u32;
        let rolls = [2u32, 5u32];
        let plans =
            plan_soldier_recruitment(5, false, false, existing_type, existing_profile, |below| {
                assert_eq!(below, 7);
                let v = rolls[calls as usize];
                calls += 1;
                v
            });
        assert_eq!(plans[0], None); // already occupied, not re-planned
        let slot1 = plans[1].expect("slot 1 should be recruitable at rank 5");
        assert_eq!(slot1.profile, 12);
        assert_eq!(calls, 2, "must re-roll past the colliding profile");
        // is_warrior false -> mage for slot 1 (C: `else type=2`).
        assert_eq!(slot1.soldier_type, SOLDIER_TYPE_MAGE);
        assert_eq!(plans[2], None);
    }

    #[test]
    fn rank_seven_recruits_slot_two_full_range_avoiding_both_prior_slots() {
        let existing_type = [SOLDIER_TYPE_WARRIOR, SOLDIER_TYPE_MAGE, 0];
        let existing_profile = [1, 9, 0];
        let rolls = [1u32, 9u32, 4u32];
        let mut calls = 0usize;
        let plans =
            plan_soldier_recruitment(7, true, true, existing_type, existing_profile, |below| {
                assert_eq!(below, 14);
                let v = rolls[calls];
                calls += 1;
                v
            });
        assert_eq!(plans[0], None);
        assert_eq!(plans[1], None);
        let slot2 = plans[2].expect("slot 2 should be recruitable at rank 7");
        assert_eq!(slot2.profile, 4);
        assert_eq!(slot2.soldier_type, SOLDIER_TYPE_WARRIOR);
        assert_eq!(calls, 3, "must re-roll past both colliding profiles");
    }

    #[test]
    fn scale_soldier_skill_matches_c_three_branch_formula() {
        assert_eq!(scale_soldier_skill(1, 47), Some(23)); // 47/2 = 23 (int div)
        assert_eq!(scale_soldier_skill(2, 47), Some(42)); // 47-5
        assert_eq!(scale_soldier_skill(3, 47), Some(47));
        assert_eq!(scale_soldier_skill(0, 47), None);
        assert_eq!(scale_soldier_skill(4, 47), None);
    }

    #[test]
    fn scale_soldier_values_applies_army1s_markers_and_leaves_others_untouched() {
        // A slice of the real army1s template markers (fdemon.chr):
        // V_HP=2, V_ENDURANCE=1, V_MANA=0, V_ARMORSKILL=3, V_SWORD=3.
        let template_markers = [2, 1, 0, 3, 3];
        let base = soldier_base_strength(1); // 47
        let mut current = [999, 999, 999, 999, 999];
        scale_soldier_values(&template_markers, base, &mut current);
        assert_eq!(current[0], 42); // marker 2 -> base-5
        assert_eq!(current[1], 23); // marker 1 -> base/2
        assert_eq!(current[2], 999); // marker 0 -> untouched
        assert_eq!(current[3], 47); // marker 3 -> base
        assert_eq!(current[4], 47); // marker 3 -> base
    }

    #[test]
    fn soldier_equipment_items_warrior_gets_five_piece_armor_skill_tiered_kit() {
        let items = soldier_equipment_items(SOLDIER_TYPE_WARRIOR, 23, 47, 999);
        assert_eq!(
            items,
            vec![
                (WN_ARMS, "sleeves3q1".to_string()),
                (WN_BODY, "armor3q1".to_string()),
                (WN_HEAD, "helmet3q1".to_string()),
                (WN_LEGS, "leggings3q1".to_string()),
                (WN_RHAND, "sword5q1".to_string()),
            ]
        );
    }

    #[test]
    fn soldier_equipment_items_mage_gets_only_a_dagger_skill_tiered_dagger() {
        let items = soldier_equipment_items(SOLDIER_TYPE_MAGE, 999, 999, 12);
        assert_eq!(items, vec![(WN_RHAND, "dagger2q1".to_string())]);
    }

    #[test]
    fn already_occupied_slots_are_never_replanned_regardless_of_rank() {
        let existing_type = [
            SOLDIER_TYPE_WARRIOR,
            SOLDIER_TYPE_MAGE,
            SOLDIER_TYPE_WARRIOR,
        ];
        let existing_profile = [0, 1, 2];
        let plans =
            plan_soldier_recruitment(20, true, true, existing_type, existing_profile, |_| {
                panic!("no RNG rolls expected when every slot is already occupied")
            });
        assert!(plans.iter().all(Option::is_none));
    }
}
