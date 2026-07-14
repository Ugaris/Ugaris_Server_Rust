use super::military::*;
use super::*;
use crate::character_driver::MilitaryMasterDriverData;
use crate::clan::CLAN_BONUS_MILITARY_ADVISOR;

//-----------------------
// Military Master NPC-scoped storage blob: `process_clan_recommendation`/
// `update_clan_points` (`military.c:1654-1674,1815-1832`).

fn master_npc_with_storage(id: u32, storage_id: i32) -> Character {
    let mut master = master_npc(id);
    master.driver_state = Some(CharacterDriverState::MilitaryMaster(
        MilitaryMasterDriverData {
            storage_id,
            ..Default::default()
        },
    ));
    master
}

// C `update_clan_points`'s own `dat->last_clan_update = realtime` on
// `NT_CREATE` (`military.c:2126`) has no Rust zone-parse-time equivalent,
// so a `0` timestamp lazily stamps to `now` on the first call without
// granting any bonus yet.
#[test]
fn update_clan_points_lazily_stamps_first_tick_without_granting_bonus() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 50)
        .unwrap();

    world.update_clan_points(CharacterId(1), 1_000);

    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 0);
    let Some(CharacterDriverState::MilitaryMaster(data)) =
        world.characters[&CharacterId(1)].driver_state.clone()
    else {
        panic!("expected MilitaryMaster driver state");
    };
    assert_eq!(data.last_clan_update, 1_000);
}

// C `update_clan_points`: `realtime - dat->last_clan_update <= 60` throttle
// - no change until more than 60 seconds have passed since the last real
// update.
#[test]
fn update_clan_points_throttles_updates_to_every_sixty_seconds() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 50)
        .unwrap();

    world.update_clan_points(CharacterId(1), 1_000); // lazy-init stamp only
    world.update_clan_points(CharacterId(1), 1_030); // only 30s later: no-op
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 0);

    world.update_clan_points(CharacterId(1), 1_061); // 61s later: applies
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 50 * 20);
    let Some(CharacterDriverState::MilitaryMaster(data)) =
        world.characters[&CharacterId(1)].driver_state.clone()
    else {
        panic!("expected MilitaryMaster driver state");
    };
    // C: `dat->last_clan_update += 60;` (not stamped to `now`).
    assert_eq!(data.last_clan_update, 1_060);

    // A second call still within the same 60s window is a no-op.
    world.update_clan_points(CharacterId(1), 1_090);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 50 * 20);
}

// C: `bonus = get_clan_bonus(n, 1) * 20; if (bonus > 0) ...` - a clan with
// no Military Advisor bonus level gets nothing.
#[test]
fn update_clan_points_skips_clans_with_no_military_advisor_bonus() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();

    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 0);
}

// C: every founded clan is updated independently in the same tick.
#[test]
fn update_clan_points_updates_every_clan_independently() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let alpha = world.clan_registry.found_clan("Alpha", 0).unwrap();
    let beta = world.clan_registry.found_clan("Beta", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(alpha, CLAN_BONUS_MILITARY_ADVISOR, 10)
        .unwrap();
    world
        .clan_registry
        .set_bonus_level(beta, CLAN_BONUS_MILITARY_ADVISOR, 30)
        .unwrap();

    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    assert_eq!(world.military_master_storage.clan_pts(7, alpha), 10 * 20);
    assert_eq!(world.military_master_storage.clan_pts(7, beta), 30 * 20);
}

// Two Military Master NPCs (distinct `storage_id`s) accrue independent
// clan-point pools, matching each NPC's own `struct military_master_data`.
#[test]
fn update_clan_points_keeps_separate_npcs_storage_independent() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    assert!(world.spawn_character(master_npc_with_storage(2, 9), 12, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 50)
        .unwrap();

    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);
    // NPC 2 never ticked past its own lazy-init stamp.
    world.update_clan_points(CharacterId(2), 2_000);

    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 50 * 20);
    assert_eq!(world.military_master_storage.clan_pts(9, cnr), 0);
}

fn clan_member(id: u32, world: &mut World, cnr: u16) -> Character {
    let mut player = recruit(id);
    world.clan_registry.add_member(&mut player, cnr).unwrap();
    player
}

// C `process_clan_recommendation` (`military.c:1654-1674`): grants
// `ppd->current_pts += 5` and deducts 12000 from the clan's banked
// points once the clan has banked more than 12000.
#[test]
fn process_clan_recommendation_grants_points_and_deducts_clan_pool_above_threshold() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 700)
        .unwrap(); // 700 * 20 = 14000 > 12000 in a single tick
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 14_000);

    let mut player_char = clan_member(2, &mut world, cnr);
    player_char.name = "Godmode".into();
    assert!(world.spawn_character(player_char, 10, 10));
    let mut player = PlayerRuntime::connected(2, 0);

    let greeting =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");

    assert_eq!(
        greeting.as_deref(),
        Some("Be greeted, Godmode. You've been recommended by your clan!")
    );
    assert_eq!(player.military_current_pts(), 5);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 2_000);
}

// C: `dat->storage_data.clan_pts[clan_nr] > 12000` - exactly at (or
// below) the threshold is not enough.
#[test]
fn process_clan_recommendation_is_a_no_op_at_or_below_threshold() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 600)
        .unwrap(); // 600 * 20 = 12000, exactly at the threshold
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 12_000);

    let player_char = clan_member(2, &mut world, cnr);
    assert!(world.spawn_character(player_char, 10, 10));
    let mut player = PlayerRuntime::connected(2, 0);

    let greeting =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");

    assert_eq!(greeting, None);
    assert_eq!(player.military_current_pts(), 0);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 12_000);
}

// C: `!(clan_nr = get_char_clan(co))` - a non-clan-member player is a
// silent no-op regardless of the clan pool.
#[test]
fn process_clan_recommendation_is_a_no_op_for_non_clan_members() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 700)
        .unwrap();
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    let player_char = recruit(2); // no clan membership
    assert!(world.spawn_character(player_char, 10, 10));
    let mut player = PlayerRuntime::connected(2, 0);

    let greeting =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");

    assert_eq!(greeting, None);
    assert_eq!(player.military_current_pts(), 0);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 14_000);
}

// C: `dat->last_recom != ch[co].ID` - the same player is only ever
// recommended once per NPC lifetime, even if the clan pool refills above
// threshold again.
#[test]
fn process_clan_recommendation_does_not_repeat_for_the_same_player() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 700)
        .unwrap();
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    let player_char = clan_member(2, &mut world, cnr);
    assert!(world.spawn_character(player_char, 10, 10));
    let mut player = PlayerRuntime::connected(2, 0);

    let first =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");
    assert!(first.is_some());
    assert_eq!(player.military_current_pts(), 5);

    // Refill the pool above threshold again, then greet the same player.
    world.update_clan_points(CharacterId(1), 1_121);
    let second =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");

    assert_eq!(second, None);
    assert_eq!(player.military_current_pts(), 5); // unchanged
}

// A different player at the same NPC can still be recommended after
// another player already was.
#[test]
fn process_clan_recommendation_allows_a_different_player_after_another_was_recommended() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 1300)
        .unwrap(); // 1300 * 20 = 26000: enough for two 12000 deductions
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    let first_char = clan_member(2, &mut world, cnr);
    assert!(world.spawn_character(first_char, 10, 10));
    let mut first_player = PlayerRuntime::connected(2, 0);
    let first_outcome = world.process_clan_recommendation(
        CharacterId(1),
        CharacterId(2),
        &mut first_player,
        "Alice",
    );
    assert!(first_outcome.is_some());

    let second_char = clan_member(3, &mut world, cnr);
    assert!(world.spawn_character(second_char, 11, 10));
    let mut second_player = PlayerRuntime::connected(3, 0);
    let second_outcome = world.process_clan_recommendation(
        CharacterId(1),
        CharacterId(3),
        &mut second_player,
        "Bob",
    );

    assert!(second_outcome.is_some());
    assert_eq!(second_player.military_current_pts(), 5);
}

//-----------------------
// Military Master NPC-scoped quest statistics: `World::record_mission_
// offered` (`accept_mission`'s `quests_given[difficulty]++`,
// `military.c:1348`) and `World::complete_mission`'s `quests_solved`/
// `pts_given`/`exp_given[difficulty]` bumps (`military.c:1382,1407,1411`).

// C: `dat->storage_data.quests_given[difficulty]++;` - called once per
// successful mission acceptance, keyed by the accepting NPC's own
// `storage_id`.
#[test]
fn record_mission_offered_increments_quests_given_for_its_difficulty() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));

    world.record_mission_offered(CharacterId(1), 2);
    world.record_mission_offered(CharacterId(1), 2);
    world.record_mission_offered(CharacterId(1), 0);

    assert_eq!(
        world.military_master_storage.quest_stats(7, 2),
        (2, 0, 0, 0)
    );
    assert_eq!(
        world.military_master_storage.quest_stats(7, 0),
        (1, 0, 0, 0)
    );
}

// A `master_id` with no live `CDR_MILITARY_MASTER` driver state is a
// silent no-op (mirrors every other storage-scoped `World` method's own
// guard in this module).
#[test]
fn record_mission_offered_is_a_no_op_for_a_non_master_character() {
    let mut world = World::default();
    world.add_character(character(1));

    world.record_mission_offered(CharacterId(1), 0);

    assert_eq!(
        world.military_master_storage.quest_stats(0, 0),
        (0, 0, 0, 0)
    );
}

// C `complete_mission`: `quests_solved[difficulty]++`, `pts_given[
// difficulty] += mis[difficulty].pts` (the mission's raw point *cost*,
// not the larger formula-adjusted `military_pts_awarded`), `exp_given[
// difficulty] += mis[difficulty].exp`.
#[test]
fn complete_mission_records_quest_stats_on_its_master_npc() {
    let mut world = World::default();
    world.add_character(character(1));
    assert!(world.spawn_character(master_npc_with_storage(2, 9), 10, 10));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(3); // difficulty 2
    player.set_military_solved_mission(true);
    player.set_military_mission(2, demon_mission(20, 40));

    let result = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(2));

    let CompleteMissionResult::Completed(outcome) = result else {
        panic!("expected Completed, got {result:?}");
    };
    assert_eq!(outcome.difficulty, 2);

    // (quests_given, quests_solved, exp_given, pts_given)
    assert_eq!(
        world.military_master_storage.quest_stats(9, 2),
        (0, 1, 40, 20)
    );
}

// A second completion at a different difficulty accumulates
// independently, and the counters are keyed per-`storage_id` (a
// different Master NPC's own blob stays untouched).
#[test]
fn complete_mission_accumulates_stats_across_difficulties_and_keeps_npcs_independent() {
    let mut world = World::default();
    world.add_character(character(1));
    assert!(world.spawn_character(master_npc_with_storage(2, 9), 10, 10));
    assert!(world.spawn_character(master_npc_with_storage(3, 40), 11, 10));

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1); // difficulty 0
    player.set_military_solved_mission(true);
    player.set_military_mission(0, demon_mission(10, 10));
    let _ = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(2));

    player.set_military_took_mission(2); // difficulty 1
    player.set_military_solved_mission(true);
    player.set_military_mission(1, demon_mission(5, 8));
    let _ = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(2));

    assert_eq!(
        world.military_master_storage.quest_stats(9, 0),
        (0, 1, 10, 10)
    );
    assert_eq!(
        world.military_master_storage.quest_stats(9, 1),
        (0, 1, 8, 5)
    );
    // The other Master NPC's storage_id (40) was never touched.
    assert_eq!(
        world.military_master_storage.quest_stats(40, 0),
        (0, 0, 0, 0)
    );
}

// A `master_id` with no live `CDR_MILITARY_MASTER` driver state is a
// silent no-op for the stats bump - `complete_mission`'s own character/
// exp/points mutation still applies normally.
#[test]
fn complete_mission_stats_are_a_no_op_for_a_non_master_character() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);
    player.set_military_solved_mission(true);
    player.set_military_mission(0, demon_mission(10, 10));

    let result = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(999));

    let CompleteMissionResult::Completed(outcome) = result else {
        panic!("expected Completed, got {result:?}");
    };
    assert_eq!(outcome.exp_awarded, 10);
    assert_eq!(
        world.military_master_storage.quest_stats(0, 0),
        (0, 0, 0, 0)
    );
}
