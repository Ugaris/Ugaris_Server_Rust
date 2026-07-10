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
    let mut area_feedback = Vec::new();
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
        &mut area_feedback,
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
    let mut area_feedback = Vec::new();
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
        &mut area_feedback,
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
    let mut area_feedback = Vec::new();
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
        &mut area_feedback,
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

fn lab3_note_zone_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                lab3_note_generic:
                    name="Note"
                    sprite=11074
                    flag=IF_USE
                    flag=IF_TAKE
                    driver=192
                    arg="0300"
                ;
                "#,
        )
        .unwrap();
    loader
}

async fn run_dispatch_lab_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
) -> (Vec<(CharacterId, String)>, i32, i32, i32) {
    let config = ServerConfig {
        area_id: 22,
        ..ServerConfig::default()
    };
    let mut feedback = Vec::new();
    let mut area_feedback = Vec::new();
    let (mut executed, mut blocked, mut failed) = (0, 0, 0);
    tick_item_use_lab::dispatch_lab_outcome(
        world,
        zone_loader,
        runtime,
        &None,
        &None,
        &config,
        outcome,
        &mut feedback,
        &mut area_feedback,
        &mut executed,
        &mut blocked,
        &mut failed,
    )
    .await;
    (feedback, executed, blocked, failed)
}

#[tokio::test]
async fn dispatch_lab_outcome_lab3_teleport_door_locked_names_the_player() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 22, 10, 10);
    world.add_character(character);

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoorLocked { character_id },
    )
    .await;

    assert_eq!((executed, blocked, failed), (0, 1, 0));
    assert!(feedback
        .iter()
        .any(|(_, message)| message == "The Guard has not opened the door for thee yet, Hero."));
}

#[tokio::test]
async fn dispatch_lab_outcome_lab3_teleport_door_busy_reports_crowd_message() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoorBusy { character_id },
    )
    .await;

    assert_eq!((executed, blocked, failed), (0, 1, 0));
    assert!(feedback.iter().any(|(_, message)| message
        == "Hm. It seems there is a crowd behind the door. Please try again later."));
}

#[tokio::test]
async fn dispatch_lab_outcome_lab3_teleport_door_pluralizes_extinguished_torches() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(9),
            character_id,
            dx: 0,
            dy: 2,
            password_protected: false,
            extinguished_count: 1,
        },
    )
    .await;
    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(feedback
        .iter()
        .any(|(_, message)| message == "Thine torch extinguished due to the water."));

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(9),
            character_id,
            dx: 0,
            dy: 2,
            password_protected: false,
            extinguished_count: 2,
        },
    )
    .await;
    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(feedback
        .iter()
        .any(|(_, message)| message == "Thine torches extinguished due to the water."));

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(9),
            character_id,
            dx: 0,
            dy: 2,
            password_protected: false,
            extinguished_count: 0,
        },
    )
    .await;
    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(feedback.is_empty());
}

#[tokio::test]
async fn dispatch_lab_outcome_lab3_note_giving_blocked_says_nothing_happens() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteGivingBlocked { character_id },
    )
    .await;

    assert_eq!((executed, blocked, failed), (0, 1, 0));
    assert!(feedback
        .iter()
        .any(|(_, message)| message == "Nothing happens."));
}

#[tokio::test]
async fn dispatch_lab_outcome_lab3_note_giving_skeleton_places_note_on_cursor() {
    let mut world = World::default();
    let mut zone_loader = lab3_note_zone_loader();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 22, 10, 10);
    world.add_character(character);

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteGivingSkeleton {
            item_id: ItemId(9),
            character_id,
            note_value: 20,
        },
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(feedback.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    let note_id = character
        .cursor_item
        .expect("expected a note on the cursor");
    let note = world.items.get(&note_id).unwrap();
    assert_eq!(note.driver, ugaris_core::item_driver::IDR_LAB3_SPECIAL);
    assert_eq!(note.driver_data[1], 20);
    // C `lab3_special`'s `drdata[1]==20` sprite override.
    assert_eq!(note.sprite, 11076);
}

#[tokio::test]
async fn dispatch_lab_outcome_lab3_note_read_returns_canned_lore_text() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteRead {
            item_id: ItemId(9),
            character_id,
            note_value: 5,
        },
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(feedback.iter().any(|(_, message)| message
        == "These large crustaceans are too strong, but fortunately very slow."));
}

#[tokio::test]
async fn dispatch_lab_outcome_lab3_note_read_unknown_value_is_a_silent_no_op() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteRead {
            item_id: ItemId(9),
            character_id,
            note_value: 99,
        },
    )
    .await;

    // C's `default: xlog(...)` branch: no player-visible text, but the
    // item was still "used" (return 1).
    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(feedback.is_empty());
}

#[tokio::test]
async fn dispatch_lab_outcome_lab3_note_read_password_notes_reveal_and_persist_the_same_pair() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
    }

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteRead {
            item_id: ItemId(9),
            character_id,
            note_value: 20,
        },
    )
    .await;
    assert_eq!((executed, blocked, failed), (1, 0, 0));
    let first_message = feedback
        .iter()
        .find(|(id, _)| *id == character_id)
        .map(|(_, message)| message.clone())
        .expect("expected password-reveal feedback");
    assert!(first_message.starts_with("Thou can read the incomplete word \""));
    assert!(first_message.ends_with("...\"."));
    let password1 = first_message
        .trim_start_matches("Thou can read the incomplete word \"")
        .trim_end_matches("...\".")
        .to_string();
    assert!(!password1.is_empty());

    // Reading a `drdata[1]==20` note a second time must not reroll: C's
    // `lab3_init_password` only assigns `password1`/`password2` once
    // (`if (*ppd->password1) return;`).
    let (feedback_again, ..) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteRead {
            item_id: ItemId(9),
            character_id,
            note_value: 20,
        },
    )
    .await;
    let second_message = feedback_again
        .iter()
        .find(|(id, _)| *id == character_id)
        .map(|(_, message)| message.clone())
        .unwrap();
    assert_eq!(first_message, second_message);

    // Reading the `drdata[1]==21` companion note reveals the *other*
    // half of the very same persisted pair.
    let (feedback21, ..) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteRead {
            item_id: ItemId(9),
            character_id,
            note_value: 21,
        },
    )
    .await;
    let second_half_message = feedback21
        .iter()
        .find(|(id, _)| *id == character_id)
        .map(|(_, message)| message.clone())
        .expect("expected password-reveal feedback");
    let password2 = second_half_message
        .trim_start_matches("Thou can read the incomplete word \"...")
        .trim_end_matches("\".")
        .to_string();
    assert!(!password2.is_empty());

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&player.legacy_lab3_password1()),
        password1
    );
    assert_eq!(
        String::from_utf8_lossy(&player.legacy_lab3_password2()),
        password2
    );
}

fn lab5_chestbox_zone_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                lab5_manapotion:
                    name="Mana Potion"
                    sprite=11040
                    flag=IF_USE
                    flag=IF_TAKE
                    driver=190
                    arg="0C"
                ;
                "#,
        )
        .unwrap();
    loader
}

#[tokio::test]
async fn dispatch_lab_outcome_lab5_potion_drunk_destroys_item_and_broadcasts_drink_message() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 22, 10, 10);
    world.add_character(character);
    let mut potion = test_item_with_driver(ItemId(9), ugaris_core::item_driver::IDR_LAB5_ITEM);
    potion.carried_by = Some(character_id);
    world.add_item(potion);

    let config = ServerConfig {
        area_id: 22,
        ..ServerConfig::default()
    };
    let mut feedback = Vec::new();
    let mut area_feedback = Vec::new();
    let (mut executed, mut blocked, mut failed) = (0, 0, 0);
    tick_item_use_lab::dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        &None,
        &None,
        &config,
        ugaris_core::item_driver::ItemDriverOutcome::Lab5PotionDrunk {
            item_id: ItemId(9),
            character_id,
        },
        &mut feedback,
        &mut area_feedback,
        &mut executed,
        &mut blocked,
        &mut failed,
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(!world.items.contains_key(&ItemId(9)));
    assert!(area_feedback
        .iter()
        .any(|(id, message, radius)| *id == character_id
            && message == "Hero drinks a potion."
            && *radius == 10));
}

#[tokio::test]
async fn dispatch_lab_outcome_lab5_chestbox_open_grants_reward_and_marks_it_opened() {
    let mut world = World::default();
    let mut zone_loader = lab5_chestbox_zone_loader();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 22, 10, 10);
    world.add_character(character);
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
    }

    let (feedback, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab5ChestboxOpen {
            item_id: ItemId(11),
            character_id,
            reward: 6,
        },
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    assert!(feedback
        .iter()
        .any(|(_, message)| message == "You received a Mana Potion."));
    let player = runtime.player_for_character(character_id).unwrap();
    assert!(player.lab5_chestbox_already_opened(11));
    assert!(!player.lab5_chestbox_already_opened(12));
}

#[tokio::test]
async fn dispatch_lab_outcome_lab5_ritual_start_persists_daemon_and_state() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 22, 10, 10);
    world.add_character(character);
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
    }

    let (_, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab5RitualStart {
            character_id,
            daemon: 2,
        },
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.lab5_ritual_daemon, 2);
    assert_eq!(player.lab5_ritual_state, 1);
    assert!(world
        .drain_pending_system_texts()
        .iter()
        .any(|text| text.message.contains("The Ritual of Beronath started.")));
}

#[tokio::test]
async fn dispatch_lab_outcome_lab5_ritual_hurt_at_item_resets_daemon_and_state() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 22, 10, 10);
    assert!(world.spawn_character(character, 10, 10));
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
        player.lab5_ritual_daemon = 1;
        player.lab5_ritual_state = 1;
    }
    let mut plate = test_item_with_driver(ItemId(9), ugaris_core::item_driver::IDR_LAB5_ITEM);
    plate.x = 90;
    plate.y = 28;
    world.add_item(plate);

    let (_, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab5RitualHurtAtItem {
            item_id: ItemId(9),
            character_id,
            stored_daemon: 1,
        },
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.lab5_ritual_daemon, 0);
    assert_eq!(player.lab5_ritual_state, 0);
}

#[tokio::test]
async fn dispatch_lab_outcome_lab5_entrance_ritual_hurt_forced_message_is_queued_first() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 22, 10, 10);
    assert!(world.spawn_character(character, 10, 10));
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
        player.lab5_ritual_daemon = 1;
        player.lab5_ritual_state = 3;
    }

    let (_, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab5EntranceRitualHurt {
            character_id,
            entrance_index: 2,
            stored_daemon: 1,
            forced_message: true,
        },
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "Mathor tells you: \"Sorry. But a strange power forced me.\""));
    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.lab5_ritual_daemon, 0);
    assert_eq!(player.lab5_ritual_state, 0);
}

#[tokio::test]
async fn dispatch_lab_outcome_lab5_no_potion_door_pass_teleports_the_player() {
    let mut world = World::default();
    let mut zone_loader = ZoneLoader::new();
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Hero"), 22, 10, 10);
    assert!(world.spawn_character(character, 10, 10));

    let (_, executed, blocked, failed) = run_dispatch_lab_outcome(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        ugaris_core::item_driver::ItemDriverOutcome::Lab5NoPotionDoorPass {
            character_id,
            target_x: 50,
            target_y: 60,
        },
    )
    .await;

    assert_eq!((executed, blocked, failed), (1, 0, 0));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (50, 60));
}
