use super::*;

// -- lab5_item -------------------------------------------------------------

fn lab5_item(id: u32, drdata: Vec<u8>) -> Item {
    let mut lab5 = item(id, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB5_ITEM);
    lab5.driver_data = drdata;
    lab5
}

fn lab5_request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_LAB5_ITEM,
        item_id: ItemId(item_id),
        character_id: CharacterId(character_id),
        spec: 0,
    }
}

#[test]
fn lab5_obelisk_fully_heals_and_reports_for_sound() {
    let mut actor = character(1);
    actor.hp = 1;
    actor.mana = 1;
    actor.endurance = 1;
    actor.lifeshield = 1;
    actor.values[0][CharacterValue::Hp as usize] = 100;
    actor.values[0][CharacterValue::Mana as usize] = 50;
    actor.values[0][CharacterValue::Endurance as usize] = 60;
    actor.values[0][CharacterValue::MagicShield as usize] = 30;
    let mut obelisk = lab5_item(7, vec![1]);

    let outcome = execute_item_driver(&mut actor, &mut obelisk, lab5_request(7, 1), 22, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5Obelisk {
            character_id: CharacterId(1)
        }
    );
    assert_eq!(actor.hp, 100 * POWERSCALE);
    assert_eq!(actor.mana, 50 * POWERSCALE);
    assert_eq!(actor.endurance, 60 * POWERSCALE);
    assert_eq!(actor.lifeshield, 30 * POWERSCALE);
}

#[test]
fn lab5_combopotion_heals_lifeshield_only_when_magicshield_present() {
    let mut actor = character(1);
    actor.values[0][CharacterValue::Hp as usize] = 100;
    actor.values[0][CharacterValue::Mana as usize] = 50;
    actor.values[0][CharacterValue::Endurance as usize] = 60;
    actor.values[0][CharacterValue::MagicShield as usize] = 30;
    // C reads `value[1][V_MAGICSHIELD]` for the gate, distinct from the
    // `value[0]` amount used for the actual heal.
    actor.values[1][CharacterValue::MagicShield as usize] = 1;
    let mut potion = lab5_item(7, vec![4]);

    let outcome = execute_item_driver(&mut actor, &mut potion, lab5_request(7, 1), 22, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5PotionDrunk {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(actor.hp, 100 * POWERSCALE);
    assert_eq!(actor.lifeshield, 30 * POWERSCALE);
}

#[test]
fn lab5_combopotion_skips_lifeshield_without_magicshield_gate() {
    let mut actor = character(1);
    actor.values[0][CharacterValue::MagicShield as usize] = 30;
    // `value[1]` (the gate) stays 0 even though `value[0]` is nonzero.
    let mut potion = lab5_item(7, vec![4]);

    execute_item_driver(&mut actor, &mut potion, lab5_request(7, 1), 22, false);

    assert_eq!(actor.lifeshield, 0);
}

#[test]
fn lab5_manapotion_only_restores_mana() {
    let mut actor = character(1);
    actor.hp = 5;
    actor.values[0][CharacterValue::Hp as usize] = 100;
    actor.values[0][CharacterValue::Mana as usize] = 50;
    let mut potion = lab5_item(7, vec![12]);

    let outcome = execute_item_driver(&mut actor, &mut potion, lab5_request(7, 1), 22, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5PotionDrunk {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(actor.mana, 50 * POWERSCALE);
    assert_eq!(actor.hp, 5, "manapotion must not touch hp");
}

#[test]
fn lab5_chestbox_blocks_on_occupied_cursor_or_already_open_sprite() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(99));
    let mut chest = lab5_item(7, vec![3, 1, 0, 0]);

    assert_eq!(
        execute_item_driver(&mut actor, &mut chest, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Noop
    );

    actor.cursor_item = None;
    chest.driver_data[3] = 1;
    assert_eq!(
        execute_item_driver(&mut actor, &mut chest, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn lab5_chestbox_reports_already_opened_from_context() {
    let mut actor = character(1);
    let mut chest = lab5_item(7, vec![3, 1, 0, 0]);
    let context = ItemDriverContext {
        lab5_chestbox_already_opened: true,
        ..ItemDriverContext::default()
    };

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut chest,
        lab5_request(7, 1),
        22,
        false,
        &context,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5ChestboxAlreadyOpened {
            character_id: CharacterId(1)
        }
    );
}

#[test]
fn lab5_chestbox_opens_and_marks_sprite_and_driver_data() {
    let mut actor = character(1);
    let mut chest = lab5_item(7, vec![3, 6, 0, 0]);
    chest.sprite = 500;

    let outcome = execute_item_driver(&mut actor, &mut chest, lab5_request(7, 1), 22, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5ChestboxOpen {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reward: 6,
        }
    );
    assert_eq!(chest.driver_data[3], 1);
    assert_eq!(chest.sprite, 501);
}

#[test]
fn lab5_chestbox_timer_closes_only_when_open() {
    let mut timer = character(0);
    let mut closed_chest = lab5_item(7, vec![3, 1, 0, 0]);
    closed_chest.sprite = 500;
    assert_eq!(
        execute_item_driver(&mut timer, &mut closed_chest, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Noop
    );
    assert_eq!(closed_chest.sprite, 500);

    let mut open_chest = lab5_item(7, vec![3, 1, 0, 1]);
    open_chest.sprite = 501;
    assert_eq!(
        execute_item_driver(&mut timer, &mut open_chest, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Lab5ChestboxClose { item_id: ItemId(7) }
    );
    assert_eq!(open_chest.driver_data[3], 0);
    assert_eq!(open_chest.sprite, 500);
}

#[test]
fn lab5_nameplate_starts_ritual_when_untouched_and_hurts_otherwise() {
    let mut actor = character(1);
    let mut plate = lab5_item(7, vec![5, 2]);

    // ritualstate defaults to 0 (no context override).
    assert_eq!(
        execute_item_driver(&mut actor, &mut plate, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5RitualStart {
            character_id: CharacterId(1),
            daemon: 2,
        }
    );

    let context = ItemDriverContext {
        lab5_ritual_state: Some(1),
        lab5_ritual_daemon: Some(1),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut plate,
            lab5_request(7, 1),
            22,
            false,
            &context,
        ),
        ItemDriverOutcome::Lab5RitualHurtAtItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            stored_daemon: 1,
        }
    );
}

#[test]
fn lab5_realnameplate_covers_nothing_progress_and_hurt() {
    let mut actor = character(1);
    let mut plate = lab5_item(7, vec![6, 2]);

    // ritualstate == 0: "Nothing happens.".
    assert_eq!(
        execute_item_driver(&mut actor, &mut plate, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5RitualNothing {
            character_id: CharacterId(1)
        }
    );

    // ritualstate == 1 and matching daemon: progresses to state 2.
    let matching = ItemDriverContext {
        lab5_ritual_state: Some(1),
        lab5_ritual_daemon: Some(2),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut plate,
            lab5_request(7, 1),
            22,
            false,
            &matching,
        ),
        ItemDriverOutcome::Lab5RitualProgress {
            character_id: CharacterId(1),
            daemon: 2,
            new_state: 2,
        }
    );

    // ritualstate == 1 but mismatched daemon: hurts, using the stored
    // (not the plate's) daemon.
    let mismatched = ItemDriverContext {
        lab5_ritual_state: Some(1),
        lab5_ritual_daemon: Some(3),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut plate,
            lab5_request(7, 1),
            22,
            false,
            &mismatched,
        ),
        ItemDriverOutcome::Lab5RitualHurtAtItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            stored_daemon: 3,
        }
    );
}

#[test]
fn lab5_entrance_untouched_is_silent_progresses_or_hurts() {
    let mut actor = character(1);
    let mut entrance = lab5_item(7, vec![7, 2]);

    assert_eq!(
        execute_item_driver(&mut actor, &mut entrance, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Noop
    );

    let matching = ItemDriverContext {
        lab5_ritual_state: Some(2),
        lab5_ritual_daemon: Some(2),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut entrance,
            lab5_request(7, 1),
            22,
            false,
            &matching,
        ),
        ItemDriverOutcome::Lab5RitualProgress {
            character_id: CharacterId(1),
            daemon: 2,
            new_state: 3,
        }
    );

    // Wrong entrance (`drdata[1]==2`) additionally forces the "strange
    // power" message in the resolver.
    let mut forced_entrance = lab5_item(7, vec![7, 2]);
    let mismatched = ItemDriverContext {
        lab5_ritual_state: Some(3),
        lab5_ritual_daemon: Some(1),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut forced_entrance,
            lab5_request(7, 1),
            22,
            false,
            &mismatched,
        ),
        ItemDriverOutcome::Lab5EntranceRitualHurt {
            character_id: CharacterId(1),
            entrance_index: 2,
            stored_daemon: 1,
            forced_message: true,
        }
    );

    let mut plain_entrance = lab5_item(7, vec![7, 1]);
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut plain_entrance,
            lab5_request(7, 1),
            22,
            false,
            &mismatched,
        ),
        ItemDriverOutcome::Lab5EntranceRitualHurt {
            character_id: CharacterId(1),
            entrance_index: 1,
            stored_daemon: 1,
            forced_message: false,
        }
    );
}

#[test]
fn lab5_backdoor_always_reports_the_teleport_attempt() {
    let mut actor = character(1);
    let mut door = lab5_item(7, vec![8]);

    assert_eq!(
        execute_item_driver(&mut actor, &mut door, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5Backdoor {
            character_id: CharacterId(1)
        }
    );
}

#[test]
fn lab5_fireface_first_call_arms_and_schedules_by_position() {
    let mut timer = character(0);
    let mut statue = lab5_item(7, vec![2, 0]);
    statue.x = 100;
    statue.y = 50;
    statue.sprite = 11135; // faces right (dx=1, dy=0)

    let outcome = execute_item_driver(&mut timer, &mut statue, lab5_request(7, 0), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FireballMachineProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 101,
            start_y: 50,
            target_x: 102,
            target_y: 50,
            power: 50,
            // (100 + 50) % 17 + 1 = 15, * TICKS_PER_SECOND.
            schedule_after_ticks: Some(15 * TICKS_PER_SECOND),
        }
    );
    assert_eq!(statue.driver_data[1], 1);
}

#[test]
fn lab5_fireface_subsequent_calls_reschedule_every_five_seconds() {
    let mut timer = character(0);
    let mut statue = lab5_item(7, vec![2, 1]);
    statue.x = 10;
    statue.y = 20;
    statue.sprite = 11136; // faces up (dx=0, dy=-1)

    let outcome = execute_item_driver(&mut timer, &mut statue, lab5_request(7, 0), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FireballMachineProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 10,
            start_y: 19,
            target_x: 10,
            target_y: 18,
            power: 50,
            schedule_after_ticks: Some(5 * TICKS_PER_SECOND),
        }
    );
    assert_eq!(statue.driver_data[1], 1);
}

#[test]
fn lab5_fireface_default_direction_faces_down_for_unknown_sprite() {
    let mut timer = character(0);
    let mut statue = lab5_item(7, vec![2, 1]);
    statue.x = 10;
    statue.y = 20;
    statue.sprite = 11138; // faces down (dx=0, dy=1), same as any unmatched sprite

    let outcome = execute_item_driver(&mut timer, &mut statue, lab5_request(7, 0), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FireballMachineProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 10,
            start_y: 21,
            target_x: 10,
            target_y: 22,
            power: 50,
            schedule_after_ticks: Some(5 * TICKS_PER_SECOND),
        }
    );
}

#[test]
fn lab5_lightface_first_call_arms_and_schedules_by_position() {
    let mut timer = character(0);
    let mut statue = lab5_item(7, vec![13, 0, 0]);
    statue.x = 10;
    statue.y = 5;
    statue.sprite = 11137; // faces left (dx=-1, dy=0)

    let outcome = execute_item_driver(&mut timer, &mut statue, lab5_request(7, 0), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::BallTrapProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 9,
            start_y: 5,
            target_x: 8,
            target_y: 5,
            power: 40,
            // (10 + 5) % 10 + 1 = 6, * TICKS_PER_SECOND.
            schedule_after_ticks: Some(6 * TICKS_PER_SECOND),
        }
    );
    assert_eq!(statue.driver_data[1], 1);
}

#[test]
fn lab5_lightface_cycles_four_quick_reschedules_then_one_long_one() {
    let mut timer = character(0);
    let mut statue = lab5_item(7, vec![13, 1, 0]);
    statue.x = 1;
    statue.y = 1;
    statue.sprite = 11138; // faces down (dx=0, dy=1)

    for expected_counter in 1..=4_u8 {
        let outcome = execute_item_driver(&mut timer, &mut statue, lab5_request(7, 0), 22, false);
        assert_eq!(
            outcome,
            ItemDriverOutcome::BallTrapProjectile {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                start_x: 1,
                start_y: 2,
                target_x: 1,
                target_y: 3,
                power: 40,
                schedule_after_ticks: Some(7 * TICKS_PER_SECOND / 4),
            }
        );
        assert_eq!(statue.driver_data[2], expected_counter);
    }

    // The 5th call (counter reached 4) resets the counter and reschedules
    // after the long 9-second interval instead.
    let outcome = execute_item_driver(&mut timer, &mut statue, lab5_request(7, 0), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::BallTrapProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 1,
            start_y: 2,
            target_x: 1,
            target_y: 3,
            power: 40,
            schedule_after_ticks: Some(9 * TICKS_PER_SECOND),
        }
    );
    assert_eq!(statue.driver_data[2], 0);
}

#[test]
fn lab5_gun_locked_then_fires_and_reloads() {
    let mut actor = character(1);
    let mut gun = lab5_item(7, vec![9, 0]);
    gun.x = 100;
    gun.y = 50;
    gun.sprite = 200;

    let outcome = execute_item_driver(&mut actor, &mut gun, lab5_request(7, 1), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FireballMachineProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            start_x: 102,
            start_y: 50,
            target_x: 160,
            target_y: 50,
            power: 100,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2 / 3),
        }
    );
    assert_eq!(gun.driver_data[1], 7);
    assert_eq!(gun.sprite, 207);

    // Locked while reloading.
    assert_eq!(
        execute_item_driver(&mut actor, &mut gun, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5GunLocked {
            character_id: CharacterId(1)
        }
    );
}

#[test]
fn lab5_gun_reload_timer_decrements_and_reschedules_until_empty() {
    let mut timer = character(0);
    let mut gun = lab5_item(7, vec![9, 2]);
    gun.sprite = 207;

    let outcome = execute_item_driver(&mut timer, &mut gun, lab5_request(7, 0), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5GunReloadTick {
            item_id: ItemId(7),
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2 / 3),
        }
    );
    assert_eq!(gun.driver_data[1], 1);
    assert_eq!(gun.sprite, 206);

    let outcome = execute_item_driver(&mut timer, &mut gun, lab5_request(7, 0), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5GunReloadTick {
            item_id: ItemId(7),
            schedule_after_ticks: None,
        }
    );
    assert_eq!(gun.driver_data[1], 0);
    assert_eq!(gun.sprite, 205);

    assert_eq!(
        execute_item_driver(&mut timer, &mut gun, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn lab5_pike_always_hurts_and_arms_once() {
    let mut actor = character(1);
    let mut pike = lab5_item(7, vec![10, 0]);
    pike.sprite = 300;

    let outcome = execute_item_driver(&mut actor, &mut pike, lab5_request(7, 1), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5PikeHurt {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            arming: true,
        }
    );
    assert_eq!(pike.driver_data[1], 1);
    assert_eq!(pike.sprite, 301);

    // Already armed: still hurts, but does not re-arm/re-schedule.
    let outcome = execute_item_driver(&mut actor, &mut pike, lab5_request(7, 1), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5PikeHurt {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            arming: false,
        }
    );
    assert_eq!(pike.sprite, 301);
}

#[test]
fn lab5_pike_timer_resets_only_when_armed() {
    let mut timer = character(0);
    let mut idle_pike = lab5_item(7, vec![10, 0]);
    idle_pike.sprite = 300;
    assert_eq!(
        execute_item_driver(&mut timer, &mut idle_pike, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Noop
    );

    let mut armed_pike = lab5_item(7, vec![10, 1]);
    armed_pike.sprite = 301;
    assert_eq!(
        execute_item_driver(&mut timer, &mut armed_pike, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Lab5PikeReset { item_id: ItemId(7) }
    );
    assert_eq!(armed_pike.driver_data[1], 0);
    assert_eq!(armed_pike.sprite, 300);
}

#[test]
fn lab5_no_potion_door_blocks_only_when_carrying_a_potion_from_the_west() {
    let mut actor = character(1);
    actor.x = 5;
    let mut door = lab5_item(7, vec![11]);
    door.x = 10;
    door.y = 20;

    let carrying_potion = ItemDriverContext {
        has_potion: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            lab5_request(7, 1),
            22,
            false,
            &carrying_potion,
        ),
        ItemDriverOutcome::Lab5NoPotionDoorBlocked {
            character_id: CharacterId(1)
        }
    );

    assert_eq!(
        execute_item_driver(&mut actor, &mut door, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5NoPotionDoorPass {
            character_id: CharacterId(1),
            target_x: 1,
            target_y: 13,
        }
    );

    actor.x = 15;
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            lab5_request(7, 1),
            22,
            false,
            &carrying_potion,
        ),
        ItemDriverOutcome::Lab5NoPotionDoorPass {
            character_id: CharacterId(1),
            target_x: 19,
            target_y: 27,
        }
    );
}
