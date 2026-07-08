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
