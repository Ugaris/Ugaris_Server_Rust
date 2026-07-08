use super::*;
use crate::{
    character_driver::CDR_FDEMON_ARMY,
    world::npc::area8::fdemon_army::{
        assign_profile, finalize_soldier_exp_and_level, plan_soldier_recruitment,
        scale_soldier_skill, scale_soldier_values, soldier_base_strength, soldier_equipment_items,
        FarmyData, MIS_FOLLOW, SOLDIER_PROFILES, SOLDIER_TYPE_MAGE, SOLDIER_TYPE_WARRIOR, WN_ARMS,
        WN_BODY, WN_HEAD, WN_LEGS, WN_RHAND,
    },
};

fn soldier_npc(id: u32, x: u16, y: u16, leader_cn: CharacterId) -> Character {
    let mut soldier = character(id);
    soldier.driver = CDR_FDEMON_ARMY;
    soldier.name = "Bert".into();
    soldier.x = x;
    soldier.y = y;
    // Deterministic vision in tests regardless of default (pitch-dark)
    // tile lighting - `army_follow_driver`'s viewer is the soldier, so
    // *it* needs `INFRARED` for the "leader visible" scenarios (same
    // convenience already established by `fdemon.rs`'s own
    // `fdemon_demon_npc` test helper); tests exercising the "leader not
    // visible" branch remove this flag again.
    soldier.flags |= CharacterFlags::INFRARED;
    soldier.driver_state = Some(CharacterDriverState::FdemonArmy(FarmyData {
        leader_cn,
        mission: MIS_FOLLOW,
        ..FarmyData::default()
    }));
    soldier
}

fn leader_npc(id: u32, x: u16, y: u16) -> Character {
    let mut leader = character(id);
    leader.flags |= CharacterFlags::PLAYER;
    leader.name = "Hero".into();
    leader.x = x;
    leader.y = y;
    leader
}

#[test]
fn army_follow_driver_walks_toward_visible_leader_when_far() {
    let mut world = World::default();
    let leader = leader_npc(1, 100, 100);
    world.characters.insert(leader.id, leader);
    let soldier = soldier_npc(2, 80, 100, CharacterId(1));
    let soldier_id = soldier.id;
    world.characters.insert(soldier_id, soldier);

    assert!(world.army_follow_driver(soldier_id, 10, 1));
    let moved = world.characters.get(&soldier_id).unwrap();
    assert!(moved.action != 0 || moved.x != 80);
    let Some(CharacterDriverState::FdemonArmy(dat)) = moved.driver_state else {
        panic!("expected FdemonArmy driver state");
    };
    // C `dat->lx = ch[co].x; dat->ly = ch[co].y;` - updated even though
    // the walk itself only takes one step.
    assert_eq!((dat.lx, dat.ly), (100, 100));
}

#[test]
fn army_follow_driver_stops_when_already_within_dist_of_visible_leader() {
    let mut world = World::default();
    let leader = leader_npc(1, 100, 100);
    world.characters.insert(leader.id, leader);
    // Manhattan distance 5 <= dist(10).
    let soldier = soldier_npc(2, 95, 100, CharacterId(1));
    let soldier_id = soldier.id;
    world.characters.insert(soldier_id, soldier);

    assert!(!world.army_follow_driver(soldier_id, 10, 1));
    let unmoved = world.characters.get(&soldier_id).unwrap();
    assert_eq!((unmoved.x, unmoved.y), (95, 100));
}

#[test]
fn army_follow_driver_walks_toward_last_known_position_when_leader_not_visible() {
    let mut world = World::default();
    let leader = leader_npc(1, 120, 100);
    world.characters.insert(leader.id, leader);

    let mut soldier = soldier_npc(2, 80, 100, CharacterId(1));
    // No `INFRARED`/light: at distance > 1 with pitch-dark tiles, the
    // soldier (the viewer here) cannot currently see the leader
    // (`char_see_char_nolos`).
    soldier.flags.remove(CharacterFlags::INFRARED);
    // Last-known leader position, set by a prior visible tick.
    let Some(CharacterDriverState::FdemonArmy(dat)) = soldier.driver_state.as_mut() else {
        panic!("expected FdemonArmy driver state");
    };
    dat.lx = 90;
    dat.ly = 100;
    let soldier_id = soldier.id;
    world.characters.insert(soldier_id, soldier);

    assert!(world.army_follow_driver(soldier_id, 10, 1));
    let moved = world.characters.get(&soldier_id).unwrap();
    assert!(moved.action != 0 || moved.x != 80);
}

#[test]
fn army_follow_driver_returns_false_without_a_driver_state_or_leader() {
    let mut world = World::default();
    let mut soldier = character(2);
    soldier.driver = CDR_FDEMON_ARMY;
    soldier.x = 80;
    soldier.y = 100;
    let soldier_id = soldier.id;
    world.characters.insert(soldier_id, soldier);

    // No `FdemonArmy` driver state at all.
    assert!(!world.army_follow_driver(soldier_id, 10, 1));
}

#[test]
fn fdemon_army_tick_disintegrates_when_leader_lost() {
    let mut world = World::default();
    // Leader never inserted - C's `!(ch[dat->leader_cn].flags)` case.
    let soldier = soldier_npc(2, 80, 100, CharacterId(1));
    let soldier_id = soldier.id;
    world.characters.insert(soldier_id, soldier);

    assert!(world.fdemon_army_tick(soldier_id, 1));
    assert!(world.characters.get(&soldier_id).is_none());
}

#[test]
fn fdemon_army_tick_disintegrates_when_leader_group_differs() {
    let mut world = World::default();
    let mut leader = leader_npc(1, 100, 100);
    leader.group = 5;
    world.characters.insert(leader.id, leader);

    let mut soldier = soldier_npc(2, 80, 100, CharacterId(1));
    soldier.group = 9; // different group - "lost" our master.
    let soldier_id = soldier.id;
    world.characters.insert(soldier_id, soldier);

    assert!(world.fdemon_army_tick(soldier_id, 1));
    assert!(world.characters.get(&soldier_id).is_none());
}

#[test]
fn fdemon_army_tick_follows_leader_of_the_same_group() {
    let mut world = World::default();
    let mut leader = leader_npc(1, 100, 100);
    leader.group = 5;
    world.characters.insert(leader.id, leader);

    let mut soldier = soldier_npc(2, 80, 100, CharacterId(1));
    soldier.group = 5;
    let soldier_id = soldier.id;
    world.characters.insert(soldier_id, soldier);

    assert!(!world.fdemon_army_tick(soldier_id, 1));
    let moved = world.characters.get(&soldier_id).unwrap();
    assert!(moved.action != 0 || moved.x != 80);
}

// The remaining tests exercise `fdemon_army`'s pure helpers (no `World`
// needed) - moved here from an in-file `#[cfg(test)]` module to keep
// `fdemon_army.rs` itself under the ~800-line NPC-file guideline, same
// "tests live under `world::tests`" convention as every other area-8 file.

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
    let plans = plan_soldier_recruitment(7, true, true, existing_type, existing_profile, |below| {
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
fn finalize_soldier_exp_and_level_recomputes_exp_used_and_level_from_scaled_values() {
    let base = soldier_base_strength(1); // 47
    let mut soldier = character(9);
    soldier.level = 1;
    soldier.exp = 999;
    soldier.exp_used = 999;
    // A slice of army1s's template markers (V_HP=2, V_ENDURANCE=1,
    // V_SWORD=3), same fixture as `scale_soldier_values_...` above.
    let template_markers = [2, 1, 0, 3, 3];
    let mut scaled = [0i32; 5];
    scale_soldier_values(&template_markers, base, &mut scaled);
    for (v, value) in scaled.iter().enumerate() {
        soldier.values[1][v] = *value as i16;
    }
    // values[1][..5] = [42(-5), 23(/2), 0(untouched), 47(base), 47(base)]

    finalize_soldier_exp_and_level(&mut soldier);

    let expected_exp = crate::world::calc_exp(&soldier);
    assert_eq!(soldier.exp, expected_exp);
    assert_eq!(soldier.exp_used, expected_exp);
    assert_eq!(soldier.level, crate::world::exp2level(expected_exp));
    assert!(
        soldier.exp > 0,
        "scaled skill values must produce nonzero exp"
    );
}

#[test]
fn already_occupied_slots_are_never_replanned_regardless_of_rank() {
    let existing_type = [
        SOLDIER_TYPE_WARRIOR,
        SOLDIER_TYPE_MAGE,
        SOLDIER_TYPE_WARRIOR,
    ];
    let existing_profile = [0, 1, 2];
    let plans = plan_soldier_recruitment(20, true, true, existing_type, existing_profile, |_| {
        panic!("no RNG rolls expected when every slot is already occupied")
    });
    assert!(plans.iter().all(Option::is_none));
}
