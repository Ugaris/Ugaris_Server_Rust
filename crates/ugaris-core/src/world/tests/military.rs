use super::*;

// C `get_army_rank_int` (`tool.c:2023-2035`): `cbrt(military_pts)` clamped
// to `[0, MAX_ARMY_RANK]`.
#[test]
fn army_rank_for_points_matches_cube_root_thresholds() {
    assert_eq!(army_rank_for_points(0), 0);
    assert_eq!(army_rank_for_points(-5), 0);
    assert_eq!(army_rank_for_points(7), 1);
    assert_eq!(army_rank_for_points(8), 2);
    assert_eq!(army_rank_for_points(999), 9);
    assert_eq!(army_rank_for_points(1000), 10);
    assert_eq!(army_rank_for_points(64_000), 40);
    // Past the max-rank cube (41^3 = 68921), the raw cube root exceeds
    // MAX_ARMY_RANK; C's `set_army_rank` clamps via `min(MAX_ARMY_RANK,
    // rank)`, so the effective rank stays capped at 40, never higher.
    assert_eq!(army_rank_for_points(68_921), MAX_ARMY_RANK);
    assert_eq!(army_rank_for_points(1_000_000), MAX_ARMY_RANK);
}

// C `tool.c:1868-1907`'s `rankname[]` table, spot-checked letter for
// letter at both ends and a couple of interior entries.
#[test]
fn army_rank_name_matches_legacy_table() {
    assert_eq!(army_rank_name(0), "nobody");
    assert_eq!(army_rank_name(1), "Private");
    assert_eq!(army_rank_name(10), "Second Lieutenant");
    assert_eq!(army_rank_name(20), "Field Marshal");
    assert_eq!(army_rank_name(40), "Avatar of Astonia");
    // Out-of-range ranks clamp instead of panicking (defensive; C's own
    // `rankname[min(MAX_ARMY_RANK, ppd->army_rank)]` never overshoots
    // since `army_rank` itself is always clamped by `set_army_rank`).
    assert_eq!(army_rank_name(999), "Avatar of Astonia");
}

#[test]
fn give_military_pts_adds_points_and_exp_without_promotion_below_threshold() {
    let mut world = World::default();
    let player = character(1);
    assert!(world.spawn_character(player, 10, 10));

    let award = world.give_military_pts(CharacterId(1), 0, 1, 3);

    assert!(!award.promoted());
    assert_eq!(award.old_rank, 0);
    assert_eq!(award.new_rank, 0);
    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.military_points, 0);
    assert_eq!(character.military_normal_exp, 1);
    assert_eq!(character.exp, 1);
    assert!(world.drain_pending_system_texts().is_empty());
    assert!(world.drain_pending_channel_broadcasts().is_empty());
}

// C `give_military_pts_no_npc` (`tool.c:3279-3306`): crossing a rank
// threshold queues the "You've been promoted..." system text.
#[test]
fn give_military_pts_promotes_and_queues_feedback_text() {
    let mut world = World::default();
    let player = character(1);
    assert!(world.spawn_character(player, 10, 10));

    let award = world.give_military_pts(CharacterId(1), 8, 0, 3);

    assert!(award.promoted());
    assert_eq!(award.old_rank, 0);
    assert_eq!(award.new_rank, 2);
    let feedback = world.drain_pending_system_texts();
    assert_eq!(feedback.len(), 1);
    assert_eq!(
        feedback[0].message,
        "You've been promoted to Private First Class. Congratulations, Character!"
    );
    // Rank 2 is below the Sergeant Major (index 9) announce threshold, so
    // no server-wide broadcast is queued.
    assert!(world.drain_pending_channel_broadcasts().is_empty());
}

// C: `if (get_army_rank_int(co) > 9)` gates the server-wide "Grats: NAME
// is a X now!" channel-6 broadcast (`tool.c:3273-3275`).
#[test]
fn give_military_pts_above_rank_nine_also_broadcasts_server_wide() {
    let mut world = World::default();
    let player = character(1);
    assert!(world.spawn_character(player, 10, 10));

    let award = world.give_military_pts(CharacterId(1), 1000, 0, 3);

    assert!(award.promoted());
    assert_eq!(award.new_rank, 10);
    let broadcasts = world.drain_pending_channel_broadcasts();
    assert_eq!(broadcasts.len(), 1);
    assert_eq!(broadcasts[0].channel, 6);
    let mut expected = b"0000000000".to_vec();
    expected.extend_from_slice(crate::text::COL_CHAT_GRATS);
    expected.extend_from_slice(b"Grats: Character is a Second Lieutenant now!");
    assert_eq!(broadcasts[0].message_bytes, expected);
}

// C `give_military_pts_no_npc`: `pts` gets the hardcore military bonus
// multiplier (`hardcore_military_exp_bonus`), distinct from the normal
// exp hardcore bonus that `give_exp` applies to `exps` internally.
#[test]
fn give_military_pts_applies_hardcore_bonus_only_to_points_not_recorded_exp() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::HARDCORE);
    assert!(world.spawn_character(player, 10, 10));
    world.settings.hardcore_military_exp_bonus = 2.0;

    world.give_military_pts(CharacterId(1), 10, 5, 3);

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.military_points, 20, "10 pts * 2.0 hardcore bonus");
    // C: `ppd->normal_exp += exps` uses the raw argument, not whatever
    // `give_exp` internally scaled the real exp total by.
    assert_eq!(character.military_normal_exp, 5);
}

#[test]
fn give_military_pts_on_unknown_character_is_a_no_op() {
    let mut world = World::default();
    let award = world.give_military_pts(CharacterId(99), 100, 5, 3);
    assert_eq!(award, MilitaryPointsAward::default());
    assert!(world.drain_pending_system_texts().is_empty());
}
