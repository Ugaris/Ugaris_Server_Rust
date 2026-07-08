use super::*;
use ugaris_core::item_driver::{ItemDriverRequest, IDR_PENT};

fn connect_player(runtime: &mut ServerRuntime, session_id: u64, character_id: CharacterId) {
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(session_id, commands, 0);
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.character_id = Some(character_id);
    }
}

// C `check_for_quest_completion` -> `complete_pentagram_quest` ->
// `distribute_rewards_to_player`, exercised end-to-end through
// `crate::pents::process_pentagram_activations` against a real `World::
// apply_pentagram_activate`-queued event (not a hand-built one), matching
// the `military.rs` test file's own precedent for this orchestration
// style.
#[tokio::test]
async fn process_pentagram_activations_pays_solo_solver_and_tracks_achievement_stats() {
    let area_id: u16 = 4;
    let player_id = CharacterId(1);

    let mut world = World::default();
    world.area_id = area_id;
    world.pentagram_quest.initialized = true;
    world.pentagram_quest.total_pentagrams = 1;
    world.pentagram_quest.required_activations = 1;
    world.pentagram_quest.area_pentagram_counts[3] = 1;

    let mut player_character = login_character(player_id, &login_block("Hero"), area_id, 10, 10);
    player_character.level = 10;
    assert!(world.spawn_character(player_character, 10, 10));

    let mut pent = ugaris_core::entity::Item {
        id: ItemId(7),
        name: "Pentagram".into(),
        description: String::new(),
        flags: ugaris_core::entity::ItemFlags::USED,
        sprite: 100,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
        modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
        x: 10,
        y: 10,
        carried_by: None,
        contained_in: None,
        content_id: 0,
        driver: IDR_PENT,
        driver_data: vec![3, 0, 2, 0, 0],
        serial: 0,
    };
    pent.driver = IDR_PENT;
    world.add_item(pent);

    world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_PENT,
            item_id: ItemId(7),
            character_id: player_id,
            spec: 0,
        },
        area_id,
    );

    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, player_id);

    crate::pents::process_pentagram_activations(&mut world, &mut runtime, &None).await;

    // The solve grants exp both from the direct add_pentagram_to_player
    // bonus tail and from distribute_rewards_to_player's payout.
    let character = &world.characters[&player_id];
    assert!(character.exp > 0);

    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(player.achievement_stats.pents_solved, 1);
    // area 4 -> PentArea::Earth (index 0).
    assert_eq!(player.achievement_stats.pents_per_area[0], 1);
    // pent_cnt is the lifetime activation counter, never reset by a solve.
    assert_eq!(player.pentagram_debug.pent_cnt, 1);
    // distribute_rewards_reset zeroes the accumulated per-run bonus/combo.
    assert_eq!(player.pentagram_debug.bonus, 0);
    assert_eq!(player.pentagram_debug.status, 0);

    let messages: Vec<String> = world
        .drain_pending_system_texts()
        .into_iter()
        .map(|text| text.message)
        .collect();
    assert!(messages
        .iter()
        .any(|m| m.contains("Hero solved the pentagram quest")));
    assert!(messages.iter().any(|m| m.contains("- Solved -")));
    assert!(messages.iter().any(|m| m.contains("The current record is")));
}

// A non-solving activation (required_activations not yet reached) only
// applies `add_pentagram_to_player`'s own bookkeeping/messages - no exp
// grant, no achievement, no reward fan-out.
#[tokio::test]
async fn process_pentagram_activations_leaves_no_exp_when_quest_not_yet_solved() {
    let area_id: u16 = 4;
    let player_id = CharacterId(1);

    let mut world = World::default();
    world.area_id = area_id;
    world.pentagram_quest.initialized = true;
    world.pentagram_quest.total_pentagrams = 5;
    world.pentagram_quest.required_activations = 5;
    world.pentagram_quest.area_pentagram_counts[3] = 5;

    let player_character = login_character(player_id, &login_block("Hero"), area_id, 10, 10);
    assert!(world.spawn_character(player_character, 10, 10));

    let mut pent = ugaris_core::entity::Item {
        id: ItemId(7),
        name: "Pentagram".into(),
        description: String::new(),
        flags: ugaris_core::entity::ItemFlags::USED,
        sprite: 100,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
        modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
        x: 10,
        y: 10,
        carried_by: None,
        contained_in: None,
        content_id: 0,
        driver: IDR_PENT,
        driver_data: vec![3, 0, 2, 0, 0],
        serial: 0,
    };
    pent.driver = IDR_PENT;
    world.add_item(pent);

    world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_PENT,
            item_id: ItemId(7),
            character_id: player_id,
            spec: 0,
        },
        area_id,
    );

    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, player_id);

    crate::pents::process_pentagram_activations(&mut world, &mut runtime, &None).await;

    let character = &world.characters[&player_id];
    assert_eq!(character.exp, 0);

    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(player.achievement_stats.pents_solved, 0);
    assert_eq!(player.pentagram_debug.pent_cnt, 1);

    let messages: Vec<String> = world
        .drain_pending_system_texts()
        .into_iter()
        .map(|text| text.message)
        .collect();
    assert!(messages.iter().any(|m| m.contains("You got a")));
    assert!(!messages
        .iter()
        .any(|m| m.contains("solved the pentagram quest")));
}

// C `handle_pentagram_interaction`'s `spawn_count = get_activation_spawn_
// count(); spawn_demons_at_pentagram(...)` tail, exercised end-to-end
// through a real `World::apply_pentagram_activate`-queued
// `PentagramDemonSpawnRequest` (not a hand-built one) plus
// `crate::pents::process_pentagram_demon_spawns`, matching this file's
// own precedent for orchestration-level tests. `legacy_random_seed`
// starts at `World::default()`'s `0`, which deterministically rolls a
// `Normal`-type demon and a `penter7` template name for a level-3
// pentagram (verified against the exact `legacy_random_below_from_seed`
// LCG sequence) - see the two registered templates below covering both
// outcomes so the test doesn't depend on memorizing the exact roll.
#[tokio::test]
async fn process_pentagram_demon_spawns_instantiates_penter_template_at_the_pentagram() {
    let area_id: u16 = 4;

    let mut world = World::default();
    world.area_id = area_id;
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    world.pentagram_quest.initialized = true;
    // High enough that this single activation never also crosses the
    // solve threshold - this test is only about the demon-spawn tail.
    world.pentagram_quest.required_activations = 100;
    world.pentagram_quest.area_pentagram_counts[3] = 100;
    // Only try to spawn one demon so the test only needs to predict one
    // set of demon-type/template-name RNG rolls.
    world.settings.activation_spawn_count = 1;

    let player_id = CharacterId(1);
    let player_character = login_character(player_id, &login_block("Hero"), area_id, 10, 10);
    assert!(world.spawn_character(player_character, 10, 10));

    let mut pent = ugaris_core::entity::Item {
        id: ItemId(7),
        name: "Pentagram".into(),
        description: String::new(),
        flags: ugaris_core::entity::ItemFlags::USED,
        sprite: 100,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
        modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
        x: 10,
        y: 10,
        carried_by: None,
        contained_in: None,
        content_id: 0,
        driver: IDR_PENT,
        driver_data: vec![3, 0, 2, 0, 0],
        serial: 0,
    };
    pent.driver = IDR_PENT;
    world.add_item(pent);

    world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_PENT,
            item_id: ItemId(7),
            character_id: player_id,
            spec: 0,
        },
        area_id,
    );

    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                penter6:
                  name="Demon"
                  description="Eeek! A Demon!"
                  V_HP=10
                  V_ENDURANCE=8
                  V_MANA=6
                  driver=64
                  arg="aggressive=1;helper=0;scavenger=10;startdist=20;chardist=0;stopdist=40;"
                  class=53
                ;
                penter7:
                  name="Demon"
                  description="Eeek! A Demon!"
                  V_HP=10
                  V_ENDURANCE=8
                  V_MANA=6
                  driver=64
                  arg="aggressive=1;helper=0;scavenger=10;startdist=20;chardist=0;stopdist=40;"
                  class=53
                ;
            "#,
        )
        .unwrap();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(70);

    crate::pents::process_pentagram_demon_spawns(&mut world, &mut loader, &mut runtime);

    let demon = world.characters.get(&CharacterId(70)).unwrap();
    assert_eq!(demon.driver, ugaris_core::character_driver::CDR_PENTER);
    assert_eq!(demon.name, "Demon");
    assert!(demon.hp > 0);
    assert!(matches!(
        demon.driver_state,
        Some(ugaris_core::character_driver::CharacterDriverState::SimpleBaddy(_))
    ));
    // The pentagram's own slot bookkeeping (offset 6) now points at the
    // spawned demon and its serial.
    let pent_after = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(
        u16::from_le_bytes([pent_after.driver_data[6], pent_after.driver_data[7]]),
        70
    );
}

// C `handle_demon_death`'s `achievement_award(killer_id,
// ACHIEVEMENT_DEMON_LORDS_DEMISE, 1)`, exercised end-to-end through a
// real `World::kill_character_followup`-queued award (via a live
// `CDR_PENTER` demon-lord-class death) plus
// `crate::pents::process_penter_demon_lords_demise_awards`.
#[tokio::test]
async fn process_penter_demon_lords_demise_awards_unlocks_the_achievement() {
    let area_id: u16 = 4;
    let killer_id = CharacterId(1);

    let mut world = World::default();
    world.area_id = area_id;
    world.pentagram_quest.power_levels[2] = 100; // class_index for class 260.

    let killer_character = login_character(killer_id, &login_block("Hero"), area_id, 10, 10);
    assert!(world.spawn_character(killer_character, 10, 10));

    let mut demon = login_character(CharacterId(2), &login_block("Demon"), area_id, 11, 10);
    demon.flags = ugaris_core::entity::CharacterFlags::USED;
    demon.driver = ugaris_core::character_driver::CDR_PENTER;
    demon.class = 260;
    demon.hp = 1;
    world.add_character(demon);

    // A lethal hit drives the demon's hp to/below 0, which `apply_legacy_
    // hurt` resolves into a kill and calls `kill_character_followup` -
    // the real entry point `World::apply_penter_demon_death` hooks.
    world.apply_legacy_hurt(CharacterId(2), Some(killer_id), 100_000, 1, 0, 0);

    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, killer_id);

    crate::pents::process_penter_demon_lords_demise_awards(&mut world, &mut runtime, &None).await;

    let player = runtime.player_for_character(killer_id).unwrap();
    assert!(player
        .achievement_data
        .is_unlocked(ugaris_core::achievement::AchievementType::DemonLordsDemise));

    let messages: Vec<String> = world
        .drain_pending_system_texts()
        .into_iter()
        .map(|text| text.message)
        .collect();
    assert!(messages
        .iter()
        .any(|m| m.contains("Training area power setting down to")));
}
