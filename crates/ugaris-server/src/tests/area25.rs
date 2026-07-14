use super::*;
use ugaris_core::world::npc::area25::WarpmasterOutcomeEvent;

#[test]
fn reset_warp_ppd_clears_points_bonus_history_and_sets_nostepexp() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.warp_points = 3;
    player.warp_bonus_last_used = vec![40, 45, 0];
    player.warp_nostepexp = 0;
    runtime.players.insert(1, player);
    let mut loader = ZoneLoader::new();

    let applied = apply_warpmaster_events(
        &mut world,
        &mut runtime,
        &mut loader,
        vec![WarpmasterOutcomeEvent::ResetWarpPpd {
            player_id: CharacterId(2),
        }],
    );

    assert_eq!(applied, 1);
    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.warp_points, 0);
    assert!(player.warp_bonus_last_used.iter().all(|&used| used == 0));
    assert_eq!(player.warp_nostepexp, 1);
}

#[test]
fn give_keys_creates_and_gives_the_requested_key_count_then_consumes_the_ingredient() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                warped_door_key:
                  name="Warped Door Key"
                ;
            "#,
        )
        .unwrap();

    // Build a minimal player character to receive the keys.
    let mut recipient = login_character(CharacterId(2), &login_block("Godmode"), 25, 15, 15);
    recipient.cursor_item = None;
    world.add_character(recipient);

    let mut ingredient = test_item(ItemId(500), 0, ItemFlags::empty());
    ingredient.driver_data = vec![23];
    world.add_item(ingredient);

    let applied = apply_warpmaster_events(
        &mut world,
        &mut runtime,
        &mut loader,
        vec![WarpmasterOutcomeEvent::GiveKeys {
            warpmaster_id: CharacterId(1),
            player_id: CharacterId(2),
            ingredient_item_id: ItemId(500),
            count: 1,
        }],
    );

    assert_eq!(applied, 1);
    // The ingredient is consumed once at least one key was given.
    assert!(!world.items.contains_key(&ItemId(500)));
    // The recipient now carries the created key (on the cursor, the
    // first free slot `World::give_char_item` fills).
    let recipient = world.characters.get(&CharacterId(2)).unwrap();
    assert!(recipient.cursor_item.is_some());
}
