use super::*;

fn labexit_zone_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                labexit:
                    sprite=1060
                    flag=IF_USE
                    driver=102
                ;
                "#,
        )
        .unwrap();
    loader
}

#[test]
fn create_lab_exit_drops_a_tagged_gate_at_the_killers_feet() {
    let mut world = World::default();
    let mut zone_loader = labexit_zone_loader();
    let killer = login_character(CharacterId(7), &login_block("Hero"), 22, 20, 20);
    assert!(world.spawn_character(killer, 20, 20));

    assert!(crate::lab::create_lab_exit(
        &mut world,
        &mut zone_loader,
        CharacterId(7),
        20
    ));

    let gate = world
        .items
        .values()
        .find(|item| item.driver == ugaris_core::item_driver::IDR_LABEXIT)
        .expect("expected a spawned lab exit gate");
    // C `drop_item_extended(in, ch[cn].x, ch[cn].y, 4)`: the killer's
    // own tile is `MOVEBLOCK`ed by their own character, so the gate
    // lands on a nearby free tile within the search radius, not
    // literally under their feet.
    assert!(gate.x.abs_diff(20) <= 4);
    assert!(gate.y.abs_diff(20) <= 4);
    assert_eq!(&gate.driver_data[0..4], &7u32.to_le_bytes());
    assert_eq!(gate.driver_data[4], 20);
}

#[test]
fn create_lab_exit_fails_silently_for_an_unknown_killer() {
    let mut world = World::default();
    let mut zone_loader = labexit_zone_loader();

    assert!(!crate::lab::create_lab_exit(
        &mut world,
        &mut zone_loader,
        CharacterId(7),
        20
    ));
    assert!(world.items.is_empty());
}

#[test]
fn apply_pending_lab_exit_spawns_drains_the_queue() {
    let mut world = World::default();
    let mut zone_loader = labexit_zone_loader();
    let killer = login_character(CharacterId(7), &login_block("Hero"), 22, 20, 20);
    assert!(world.spawn_character(killer, 20, 20));
    world.queue_lab_exit_spawn(CharacterId(7), 20);

    assert_eq!(
        crate::lab::apply_pending_lab_exit_spawns(&mut world, &mut zone_loader),
        1
    );
    assert!(world.drain_pending_lab_exit_spawns().is_empty());
}

fn labexit_use_outcome(character_id: CharacterId) -> ugaris_core::item_driver::ItemDriverOutcome {
    ugaris_core::item_driver::ItemDriverOutcome::LabExitUse {
        item_id: ItemId(9),
        character_id,
        lab_nr: 20,
        frame: 227,
        target_area: 3,
        target_x: 183,
        target_y: 199,
    }
}

#[tokio::test]
async fn dispatch_lab_outcome_labexit_use_awards_exp_and_teleports_same_area() {
    let mut world = World::default();
    let mut zone_loader = labexit_zone_loader();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Hero"), 3, 10, 10);
    character.level = 20;
    assert!(world.spawn_character(character, 10, 10));
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
    }
    // C `labexit`'s `change_area(cn, 3, 183, 199)`: this test's own
    // server serves area 3, matching the target, so no cross-area
    // hand-off is needed.
    let config = ServerConfig {
        area_id: 3,
        ..ServerConfig::default()
    };
    let mut feedback = Vec::new();
    let (mut executed, mut blocked, mut failed) = (0, 0, 0);

    tick_item_use_lab::dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        &None,
        &None,
        &config,
        labexit_use_outcome(character_id),
        &mut feedback,
        &mut executed,
        &mut blocked,
        &mut failed,
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(feedback
        .iter()
        .any(|(_, message)| message.contains("Congratulations, Hero")));
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.lab_solved_bits & (1 << 20), 1 << 20);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (183, 199));
    // C `give_exp(cn, level_value(20) / 5)`.
    assert_eq!(character.exp, ugaris_core::world::level_value(20) / 5);
}

#[tokio::test]
async fn dispatch_lab_outcome_labexit_use_does_not_regrant_exp_on_repeat_visit() {
    let mut world = World::default();
    let mut zone_loader = labexit_zone_loader();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 3, 10, 10);
    assert!(world.spawn_character(character, 10, 10));
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
        player.lab_solved_bits |= 1 << 20;
    }
    let config = ServerConfig {
        area_id: 3,
        ..ServerConfig::default()
    };
    let mut feedback = Vec::new();
    let (mut executed, mut blocked, mut failed) = (0, 0, 0);

    tick_item_use_lab::dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        &None,
        &None,
        &config,
        labexit_use_outcome(character_id),
        &mut feedback,
        &mut executed,
        &mut blocked,
        &mut failed,
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(feedback.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.exp, 0);
}

#[tokio::test]
async fn dispatch_lab_outcome_labexit_use_reports_down_message_without_repositories() {
    // Cross-area (this test's own server serves area 22, the target is
    // Aston/area 3) without a registered `AreaRepository`/
    // `CharacterRepository` pair falls back to C's own `labexit`
    // "Sorry, Aston is down. Please try again soon." message.
    let mut world = World::default();
    let mut zone_loader = labexit_zone_loader();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 22, 10, 10);
    world.add_character(character);
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
    }
    let config = ServerConfig {
        area_id: 22,
        ..ServerConfig::default()
    };
    let mut feedback = Vec::new();
    let (mut executed, mut blocked, mut failed) = (0, 0, 0);

    tick_item_use_lab::dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        &None,
        &None,
        &config,
        labexit_use_outcome(character_id),
        &mut feedback,
        &mut executed,
        &mut blocked,
        &mut failed,
    )
    .await;

    assert_eq!((executed, blocked, failed), (0, 1, 0));
    assert!(feedback
        .iter()
        .any(|(_, message)| message == "Sorry, Aston is down. Please try again soon."));
}
