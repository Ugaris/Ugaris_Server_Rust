use super::*;
use ugaris_core::world::{MiningEvent, SingleMission, MISSION_TYPE_DEMON, MISSION_TYPE_SILVER};

fn connected_player(character_id: CharacterId, session_id: u64) -> (World, ServerRuntime) {
    let mut world = World::default();
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        12,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(session_id, commands, 0);
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.character_id = Some(character_id);
    }
    (world, runtime)
}

fn metal_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                silver:
                    name="Silver"
                    description="1 unit of Silver"
                    sprite=51054
                    value=10
                    flag=IF_TAKE
                    flag=IF_USE
                    driver=61
                    arg="0101000000"
                ;
                gold:
                    name="Gold"
                    description="1 unit of Gold"
                    sprite=51053
                    value=25
                    flag=IF_TAKE
                    flag=IF_USE
                    driver=61
                    arg="0201000000"
                ;
                empty_orb:
                    name="Empty Orb"
                ;
                "#,
        )
        .unwrap();
    loader
}

fn mine_wall(id: u32, silver_base: u8, gold_base: u8, tier: u8) -> ugaris_core::entity::Item {
    let mut wall = test_item(ItemId(id), 1, ItemFlags::USED | ItemFlags::USE);
    wall.driver_data = vec![silver_base, gold_base, tier];
    wall
}

/// Brute-forces a `legacy_random_seed` whose very first
/// `World::roll_mining_event()` draw lands on `event`, so
/// `apply_mine_wall_reward` tests can deterministically force a branch
/// without depending on the LCG's internal formula.
fn find_seed_for_event(event: MiningEvent) -> u32 {
    // The orb band is only 5 wide out of a 100,000-wide roll (`mining_
    // orb_chance_base`), so a much larger search bound is needed to
    // reliably find a hit than the other, much wider bands.
    for seed in 0..500_000_u32 {
        let mut probe = World::default();
        probe.legacy_random_seed = seed;
        if probe.roll_mining_event() == event {
            return seed;
        }
    }
    panic!("no seed found for {event:?} within search bound");
}

/// Like [`find_seed_for_event`], but additionally forces
/// `apply_mine_artifact_find`'s own downstream `roll_legacy_random(100)`
/// rarity draw (the second roll after the `MiningEvent::Artifact`
/// classification itself, C `mine.c:430`) to land in `[rarity_min,
/// rarity_max)`, so each rarity-tier reward branch can be tested
/// deterministically.
fn find_seed_for_artifact_rarity(rarity_min: i32, rarity_max: i32) -> u32 {
    for seed in 0..200_000_u32 {
        let mut probe = World::default();
        probe.legacy_random_seed = seed;
        if probe.roll_mining_event() != MiningEvent::Artifact {
            continue;
        }
        let _artifact_index = probe.roll_legacy_random(12);
        let rarity = probe.roll_legacy_random(100) as i32;
        if rarity >= rarity_min && rarity < rarity_max {
            return seed;
        }
    }
    panic!("no seed found for artifact rarity [{rarity_min}, {rarity_max}) within search bound");
}

// ============================================================================
// `give_mine_item` (C `give_mine_item`, `mine.c:506-557`).
// ============================================================================

#[tokio::test]
async fn give_mine_item_merges_into_existing_inventory_pile() {
    let mut world = World::default();
    let mut loader = metal_loader();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 12, 10, 10);
    let mut existing = loader
        .instantiate_item_template("silver", Some(character_id))
        .unwrap();
    existing.value = 30;
    set_stack_count(&mut existing, 3, StackKind::SilverUnit);
    let existing_id = existing.id;
    character.inventory[30] = Some(existing_id);
    world.add_character(character);
    world.add_item(existing);

    // Roll #100 must land >= 2 to avoid the 2%-cursor branch; seed 0's
    // first draw does (verified: not 0/1).
    world.legacy_random_seed = 0;
    assert!(world.roll_legacy_random(100) >= 2);
    world.legacy_random_seed = 0;

    let result = give_mine_item(
        &mut world,
        &mut loader,
        character_id,
        StackKind::SilverUnit,
        5,
    )
    .expect("silver template should instantiate");
    match result {
        GiveMineItemResult::MergedIntoPile {
            amount,
            total,
            name,
        } => {
            assert_eq!(amount, 5);
            assert_eq!(total, 8);
            assert_eq!(name, "Silver");
        }
        other => panic!("expected a merge, got {other:?}"),
    }

    let merged = &world.items[&existing_id];
    assert_eq!(stack_count(merged), 8);
    assert_eq!(merged.value, 30 + 10 * 5);
    let character = &world.characters[&character_id];
    assert!(character.cursor_item.is_none());
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    // The freshly-created template item is never inserted (`destroy_item`).
    assert_eq!(world.items.len(), 1);
}

#[tokio::test]
async fn give_mine_item_places_fresh_pile_on_empty_cursor_when_no_match() {
    let mut world = World::default();
    let mut loader = metal_loader();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        12,
        10,
        10,
    ));

    world.legacy_random_seed = 0;
    let result = give_mine_item(
        &mut world,
        &mut loader,
        character_id,
        StackKind::GoldUnit,
        4,
    )
    .expect("gold template should instantiate");
    match result {
        GiveMineItemResult::Cursor { amount, name } => {
            assert_eq!(amount, 4);
            assert_eq!(name, "Gold");
        }
        other => panic!("expected a cursor placement, got {other:?}"),
    }

    let character = &world.characters[&character_id];
    let item_id = character
        .cursor_item
        .expect("cursor should hold the new pile");
    let item = &world.items[&item_id];
    assert_eq!(item.name, "Gold");
    assert_eq!(item.value, 25 * 4);
    assert_eq!(stack_count(item), 4);
}

// ============================================================================
// `check_military_silver` wiring (C `check_military_silver`,
// `mine.c:102-134`).
// ============================================================================

#[tokio::test]
async fn apply_military_mission_silver_check_reports_progress_then_solved() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = connected_player(character_id, 1);
    {
        let player = runtime.player_for_character_mut(character_id).unwrap();
        player.set_military_took_mission(1);
        player.set_military_mission(
            0,
            SingleMission {
                mission_type: MISSION_TYPE_SILVER,
                opt1: 10,
                opt2: 0,
                pts: 5,
                exp: 50,
            },
        );
    }

    apply_military_mission_silver_check(&mut world, &mut runtime, character_id, 4);
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.military_mission(0).opt1, 6);
    assert!(!player.military_solved_mission());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(
        texts[0].message,
        "You fulfilled part of your mission, you still need 6 silver."
    );

    apply_military_mission_silver_check(&mut world, &mut runtime, character_id, 6);
    let player = runtime.player_for_character(character_id).unwrap();
    assert!(player.military_solved_mission());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(
        texts[0].message,
        "You solved your mission. Talk to the governor to claim your reward."
    );
}

#[tokio::test]
async fn apply_military_mission_silver_check_ignores_non_silver_mission_and_no_mission() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = connected_player(character_id, 1);

    // No active mission at all: no questlog resend, no message.
    apply_military_mission_silver_check(&mut world, &mut runtime, character_id, 4);
    assert!(world.drain_pending_system_texts().is_empty());

    {
        let player = runtime.player_for_character_mut(character_id).unwrap();
        player.set_military_took_mission(1);
        player.set_military_mission(
            0,
            SingleMission {
                mission_type: MISSION_TYPE_DEMON,
                opt1: 10,
                opt2: 0,
                pts: 5,
                exp: 50,
            },
        );
    }
    apply_military_mission_silver_check(&mut world, &mut runtime, character_id, 4);
    // Non-silver mission: no mutation, no message text (questlog resend
    // still happens, but that's the legacy binary payload, not asserted
    // here).
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.military_mission(0).opt1, 10);
    assert!(world.drain_pending_system_texts().is_empty());
}

// ============================================================================
// `apply_mine_cave_in` (C `handle_cave_in`, `mine.c:362-403`).
// ============================================================================

#[tokio::test]
async fn apply_mine_cave_in_pushes_collapse_feedback_and_reduces_endurance() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut victim = login_character(character_id, &login_block("Tester"), 12, 10, 10);
    victim.endurance = 1_000_000;
    world.add_character(victim);
    world.items.insert(ItemId(9), mine_wall(9, 3, 0, 8));

    world.legacy_random_seed = 99;
    let mut feedback = Vec::new();
    apply_mine_cave_in(&mut world, ItemId(9), character_id, &mut feedback);

    assert_eq!(feedback.len(), 1);
    assert!(feedback[0].1.contains("collapses"));
    assert!(world.characters[&character_id].endurance < 1_000_000);
}

// ============================================================================
// `spawn_normal_golem`/`spawn_rare_golem` (`mine.c:571-647`).
// ============================================================================

fn golem_loader() -> ZoneLoader {
    let mut loader = metal_loader();
    loader
        .load_character_templates_str(
            r#"
                miner1:
                  name="Silver Golem"
                  description="A silver golem."
                  sprite=36
                  flag=CF_ALIVE
                  V_HP=20
                  V_ENDURANCE=15
                  V_MANA=5
                  driver=7
                ;
                "#,
        )
        .unwrap();
    loader
}

#[tokio::test]
async fn spawn_normal_golem_sets_stats_from_template_values() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = golem_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(50);
    world.items.insert(ItemId(9), mine_wall(9, 3, 0, 1));
    world.items.get_mut(&ItemId(9)).unwrap().x = 7;
    world.items.get_mut(&ItemId(9)).unwrap().y = 8;

    assert!(spawn_normal_golem(
        &mut world,
        &mut loader,
        &mut runtime,
        ItemId(9),
        1,
    ));

    let golem = world.characters.get(&CharacterId(50)).unwrap();
    assert_eq!((golem.x, golem.y), (7, 8));
    assert_eq!(golem.name, "Silver Golem");
    assert_eq!(golem.dir, Direction::RightDown as u8);
    assert_eq!(golem.hp, 20 * POWERSCALE);
    assert_eq!(golem.endurance, 15 * POWERSCALE);
    assert_eq!(golem.mana, 5 * POWERSCALE);
}

#[tokio::test]
async fn spawn_rare_golem_boosts_level_and_hp_but_leaves_endurance_mana_untouched() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = golem_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(60);
    world.items.insert(ItemId(9), mine_wall(9, 3, 0, 1));
    world.items.get_mut(&ItemId(9)).unwrap().x = 4;
    world.items.get_mut(&ItemId(9)).unwrap().y = 5;

    assert!(spawn_rare_golem(
        &mut world,
        &mut loader,
        &mut runtime,
        ItemId(9),
        1,
    ));

    let golem = world.characters.get(&CharacterId(60)).unwrap();
    assert_eq!((golem.x, golem.y), (4, 5));
    // hp = template V_HP(20) * POWERSCALE * rare_golem_hp_multiplier(2).
    assert_eq!(golem.hp, 20 * POWERSCALE * 2);
    // Level boosted (default rare_golem_level_boost = 2, template has no
    // explicit `level=` so its raw default is 0). Endurance/mana are
    // never explicitly re-set by `spawn_rare_golem` (unlike the normal
    // variant) but already hold the template's full computed values from
    // instantiation, so they end up identical to what an explicit
    // (redundant) assignment would have produced.
    assert_eq!(golem.level, 2);
    assert_eq!(golem.endurance, 15 * POWERSCALE);
    assert_eq!(golem.mana, 5 * POWERSCALE);
}

// ============================================================================
// `spawn_keyholder_golem` (C `keyholder_door`'s golem-spawn tail,
// `mine.c:1196-1208`).
// ============================================================================

fn keyholder_golem_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                keyholder_golem3:
                  name="Gold Golem"
                  description="An ancient golem made of stone and gold."
                  sprite=258
                  flag=CF_INFRARED
                  V_HP=85
                  V_ENDURANCE=10
                  V_MANA=0
                  driver=107
                ;
                "#,
        )
        .unwrap();
    loader
}

#[tokio::test]
async fn spawn_keyholder_golem_sets_stats_dir_post_and_victim_from_template() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = keyholder_golem_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(70);
    let player_id = CharacterId(1);

    spawn_keyholder_golem(&mut world, &mut loader, &mut runtime, player_id, 3, 3, 5);

    let golem = world.characters.get(&CharacterId(70)).unwrap();
    // C `2 + (n%3)*8 + 5, 231 + (n/3)*8 + 3` vs. the player's own
    // `2 + (n%3)*8 + 1, 231 + (n/3)*8 + 3` (`mine.c:1187,1204-1207`): 4
    // tiles east of the player's teleport target, same row.
    assert_eq!((golem.x, golem.y), (7, 5));
    assert_eq!(golem.name, "Gold Golem");
    assert_eq!(golem.dir, Direction::LeftUp as u8);
    assert_eq!(golem.hp, 85 * POWERSCALE);
    assert_eq!(golem.endurance, 10 * POWERSCALE);
    assert_eq!(golem.mana, 0);
    assert_eq!((golem.rest_x, golem.rest_y), (7, 5));
    match &golem.driver_state {
        Some(CharacterDriverState::GolemKeyhold(data)) => {
            assert_eq!(data.victim, Some(player_id));
        }
        other => panic!("expected GolemKeyhold driver state, got {other:?}"),
    }
}

#[tokio::test]
async fn spawn_keyholder_golem_does_nothing_for_unknown_template() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = keyholder_golem_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(70);

    // Only `keyholder_golem3` exists in the loader; door `golem_nr: 9`
    // has no matching template.
    spawn_keyholder_golem(
        &mut world,
        &mut loader,
        &mut runtime,
        CharacterId(1),
        9,
        3,
        5,
    );

    assert!(world.characters.is_empty());
}

// ============================================================================
// `apply_mine_wall_reward` end to end (C `handle_mining_result`,
// `mine.c:222-279`).
// ============================================================================

#[tokio::test]
async fn apply_mine_wall_reward_silver_branch_grants_item_and_achievement() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = connected_player(character_id, 1);
    let mut loader = metal_loader();
    world.items.insert(ItemId(9), mine_wall(9, 3, 0, 1));

    world.legacy_random_seed = find_seed_for_event(MiningEvent::Silver);
    let mut feedback = Vec::new();
    apply_mine_wall_reward(
        &mut world,
        &mut loader,
        &mut runtime,
        &None,
        12,
        ItemId(9),
        character_id,
        &mut feedback,
    )
    .await;

    assert_eq!(feedback.len(), 1);
    assert!(feedback[0].1.starts_with("You found"));
    let character = &world.characters[&character_id];
    assert!(character.cursor_item.is_some());
    let player = runtime.player_for_character(character_id).unwrap();
    assert!(player.achievement_stats.silver_mined > 0);
}

#[tokio::test]
async fn apply_mine_wall_reward_golem_branch_spawns_a_character() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = connected_player(character_id, 1);
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = golem_loader();
    let mut wall = mine_wall(9, 3, 0, 1);
    wall.x = 5;
    wall.y = 6;
    world.items.insert(ItemId(9), wall);
    runtime.set_next_character_id(90);

    world.legacy_random_seed = find_seed_for_event(MiningEvent::Golem);
    let mut feedback = Vec::new();
    apply_mine_wall_reward(
        &mut world,
        &mut loader,
        &mut runtime,
        &None,
        12,
        ItemId(9),
        character_id,
        &mut feedback,
    )
    .await;

    assert!(world.characters.get(&CharacterId(90)).is_some());
}

#[tokio::test]
async fn apply_mine_wall_reward_cavein_branch_reduces_endurance() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = connected_player(character_id, 1);
    let mut loader = metal_loader();
    world.characters.get_mut(&character_id).unwrap().endurance = 1_000_000;
    world.items.insert(ItemId(9), mine_wall(9, 3, 0, 8));

    world.legacy_random_seed = find_seed_for_event(MiningEvent::CaveIn);
    let mut feedback = Vec::new();
    apply_mine_wall_reward(
        &mut world,
        &mut loader,
        &mut runtime,
        &None,
        12,
        ItemId(9),
        character_id,
        &mut feedback,
    )
    .await;

    assert!(!feedback.is_empty());
    assert!(world.characters[&character_id].endurance < 1_000_000);
}

// ============================================================================
// `apply_mine_orb_find` (C `handle_orb_find`, `mine.c:328-360`).
// ============================================================================

#[tokio::test]
async fn apply_mine_wall_reward_orb_branch_grants_a_named_orb_on_cursor() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = connected_player(character_id, 1);
    let mut loader = metal_loader();
    world.items.insert(ItemId(9), mine_wall(9, 3, 0, 1));

    world.legacy_random_seed = find_seed_for_event(MiningEvent::Orb);
    let mut feedback = Vec::new();
    apply_mine_wall_reward(
        &mut world,
        &mut loader,
        &mut runtime,
        &None,
        12,
        ItemId(9),
        character_id,
        &mut feedback,
    )
    .await;

    // The orb itself lands on the (guaranteed-empty, since digging
    // requires it) cursor via the plain `World::give_char_item`.
    let character = &world.characters[&character_id];
    let orb_id = character
        .cursor_item
        .expect("orb should be placed on the cursor");
    let orb = &world.items[&orb_id];
    assert!(orb.name.starts_with("Orb of 5 "));
    assert_eq!(orb.driver_data[1], 5);

    let texts = world.drain_pending_system_text_bytes();
    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0].character_id, character_id);
    let first = String::from_utf8_lossy(&texts[0].message);
    assert!(first.contains("Odds bodkins!"));
    assert!(first.contains("mystical orb of cerulean radiance"));
    let second = String::from_utf8_lossy(&texts[1].message);
    assert!(second.starts_with("Thou hast received: "));
    assert!(second.contains("+5"));
}

// ============================================================================
// `apply_mine_artifact_find` (C `handle_artifact_find`, `mine.c:405-504`).
// ============================================================================

fn artifact_test_setup(character_id: CharacterId) -> (World, ServerRuntime) {
    let (mut world, runtime) = connected_player(character_id, 1);
    world.characters.get_mut(&character_id).unwrap().level = 30;
    world.items.insert(ItemId(9), mine_wall(9, 3, 0, 1));
    (world, runtime)
}

#[tokio::test]
async fn apply_mine_wall_reward_artifact_common_branch_grants_a_pittance_of_exp() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = artifact_test_setup(character_id);
    let mut loader = metal_loader();

    world.legacy_random_seed = find_seed_for_artifact_rarity(0, 50);
    let mut feedback = Vec::new();
    apply_mine_wall_reward(
        &mut world,
        &mut loader,
        &mut runtime,
        &None,
        12,
        ItemId(9),
        character_id,
        &mut feedback,
    )
    .await;

    // level_value(30) / 750 = 113521 / 750 = 151.
    assert_eq!(world.characters[&character_id].exp, 151);
    let texts = world.drain_pending_system_text_bytes();
    assert!(texts
        .iter()
        .any(|t| String::from_utf8_lossy(&t.message).contains("unearthed a relic from the")));
    assert!(texts
        .iter()
        .any(|t| String::from_utf8_lossy(&t.message).contains("but a pittance")));
}

#[tokio::test]
async fn apply_mine_wall_reward_artifact_uncommon_branch_grants_silver() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = artifact_test_setup(character_id);
    let mut loader = metal_loader();

    world.legacy_random_seed = find_seed_for_artifact_rarity(50, 80);
    let mut feedback = Vec::new();
    apply_mine_wall_reward(
        &mut world,
        &mut loader,
        &mut runtime,
        &None,
        12,
        ItemId(9),
        character_id,
        &mut feedback,
    )
    .await;

    assert!(world.characters[&character_id].gold > 0);
    let texts = world.drain_pending_system_text_bytes();
    assert!(texts
        .iter()
        .any(|t| String::from_utf8_lossy(&t.message).contains("gold coins")));
}

#[tokio::test]
async fn apply_mine_wall_reward_artifact_rare_branch_grants_military_points() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = artifact_test_setup(character_id);
    let mut loader = metal_loader();

    world.legacy_random_seed = find_seed_for_artifact_rarity(80, 95);
    let mut feedback = Vec::new();
    apply_mine_wall_reward(
        &mut world,
        &mut loader,
        &mut runtime,
        &None,
        12,
        ItemId(9),
        character_id,
        &mut feedback,
    )
    .await;

    // min(level / 3, 10) = min(10, 10) = 10.
    assert_eq!(world.characters[&character_id].military_points, 10);
    let texts = world.drain_pending_system_text_bytes();
    assert!(texts
        .iter()
        .any(|t| String::from_utf8_lossy(&t.message).contains("Huzzah!")));
}

#[tokio::test]
async fn apply_mine_wall_reward_artifact_very_rare_branch_grants_exp_and_gold() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = artifact_test_setup(character_id);
    let mut loader = metal_loader();

    world.legacy_random_seed = find_seed_for_artifact_rarity(95, 100);
    let mut feedback = Vec::new();
    apply_mine_wall_reward(
        &mut world,
        &mut loader,
        &mut runtime,
        &None,
        12,
        ItemId(9),
        character_id,
        &mut feedback,
    )
    .await;

    // level_value(30) / 250 = 113521 / 250 = 454.
    assert_eq!(world.characters[&character_id].exp, 454);
    assert!(world.characters[&character_id].gold > 0);
    let texts = world.drain_pending_system_text_bytes();
    assert!(texts
        .iter()
        .any(|t| String::from_utf8_lossy(&t.message).contains("By Ishtar's light!")));
}
