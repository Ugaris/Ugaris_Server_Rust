use super::*;

#[test]
fn clanspawn_exit_returns_rest_location() {
    let mut character = character(1);
    character.rest_area = 2;
    character.rest_x = 33;
    character.rest_y = 44;
    let mut exit = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CLANSPAWNEXIT);

    let outcome = execute_item_driver(
        &mut character,
        &mut exit,
        ItemDriverRequest::Driver {
            driver: IDR_CLANSPAWNEXIT,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        30,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::ClanSpawnExit {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            area_id: 2,
            x: 33,
            y: 44,
        }
    );
}

#[test]
fn clanspawn_exit_is_area30_guarded_like_legacy_libload() {
    let mut character = character(1);
    let mut exit = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CLANSPAWNEXIT);

    let outcome = execute_item_driver(
        &mut character,
        &mut exit,
        ItemDriverRequest::Driver {
            driver: IDR_CLANSPAWNEXIT,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_CLANSPAWNEXIT,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 30,
        }
    );
}

#[test]
fn clanspawn_timer_initializes_and_spawns_on_legacy_rounded_schedule() {
    let mut timer_character = character(0);
    let mut spawner = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CLANSPAWN);
    spawner.sprite = 1000;
    spawner.driver_data = vec![75, 4, 0];

    let init = execute_item_driver_with_context(
        &mut timer_character,
        &mut spawner,
        ItemDriverRequest::Driver {
            driver: IDR_CLANSPAWN,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        30,
        false,
        &ItemDriverContext {
            timer_call: true,
            current_tick: 100 * TICKS_PER_SECOND as u32,
            clanspawn_random_seconds: Some(123),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(spawner.max_level, 75);
    assert_eq!(drdata_u32(&spawner, 4), 3_600);
    assert_eq!(drdata(&spawner, 2), 0);
    assert_eq!(
        init,
        ItemDriverOutcome::ClanSpawnTimer {
            item_id: ItemId(8),
            spawned: false,
            jewel_count: 0,
            next_spawn_seconds: 3_600,
            schedule_after_ticks: CLANSPAWN_CHECK_INTERVAL_TICKS,
        }
    );

    let spawned = execute_item_driver_with_context(
        &mut timer_character,
        &mut spawner,
        ItemDriverRequest::Driver {
            driver: IDR_CLANSPAWN,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        30,
        false,
        &ItemDriverContext {
            timer_call: true,
            current_tick: 3_700 * TICKS_PER_SECOND as u32,
            clanspawn_random_seconds: Some(321),
            clanspawn_max_jewel_count: Some(2),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(spawner.sprite, 1001);
    assert_eq!(drdata(&spawner, 2), 1);
    assert_eq!(drdata_u32(&spawner, 4), 10_800);
    assert_eq!(
        spawned,
        ItemDriverOutcome::ClanSpawnTimer {
            item_id: ItemId(8),
            spawned: true,
            jewel_count: 1,
            next_spawn_seconds: 10_800,
            schedule_after_ticks: CLANSPAWN_CHECK_INTERVAL_TICKS,
        }
    );
}

#[test]
fn clanspawn_player_use_blocks_level_contested_and_reports_countdown() {
    let mut character = character(1);
    character.level = 80;
    let mut spawner = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CLANSPAWN);
    spawner.driver_data = vec![75, 48, 0];
    set_drdata_u32(&mut spawner, 4, 10_000);
    let request = ItemDriverRequest::Driver {
        driver: IDR_CLANSPAWN,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut spawner,
            request,
            30,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::ClanSpawnLevelTooHigh {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    character.level = 75;
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut spawner,
            request,
            30,
            false,
            &ItemDriverContext {
                clanspawn_contested: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::ClanSpawnContested {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut spawner,
            request,
            30,
            false,
            &ItemDriverContext {
                current_tick: 4_000 * TICKS_PER_SECOND as u32,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::ClanSpawnCountdown {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            remaining_minutes: 100,
            freq_hours: 48,
            god_added: false,
        }
    );
}

#[test]
fn clanspawn_god_force_adds_and_award_decrements_jewels() {
    let mut character = character(1);
    character.level = 20;
    character.flags.insert(CharacterFlags::GOD);
    let mut spawner = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CLANSPAWN);
    spawner.sprite = 500;
    spawner.max_level = 20;
    spawner.driver_data = vec![20, 0, 0];
    set_drdata_u32(&mut spawner, 4, 10_000);
    let request = ItemDriverRequest::Driver {
        driver: IDR_CLANSPAWN,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut spawner,
            request,
            30,
            false,
            &ItemDriverContext {
                current_tick: 4_000 * TICKS_PER_SECOND as u32,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::ClanSpawnCountdown {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            remaining_minutes: 100,
            freq_hours: CLANSPAWN_DEFAULT_FREQ_HOURS,
            god_added: true,
        }
    );
    assert_eq!(spawner.sprite, 501);
    assert_eq!(drdata(&spawner, 2), 1);

    let award = execute_item_driver_with_context(
        &mut character,
        &mut spawner,
        request,
        30,
        false,
        &ItemDriverContext::default(),
    );

    assert_eq!(spawner.sprite, 500);
    assert_eq!(drdata(&spawner, 2), 0);
    assert_eq!(
        award,
        ItemDriverOutcome::ClanSpawnAward {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            level: 20,
            remaining_jewels: 0,
        }
    );
}

#[test]
fn clanjewel_driver_initializes_creation_time_and_reschedules_timer() {
    let mut timer_character = character(0);
    let mut jewel = item(8, ItemFlags::USED, 0, IDR_CLANJEWEL);

    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut jewel,
        ItemDriverRequest::Driver {
            driver: IDR_CLANJEWEL,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        30,
        false,
        &ItemDriverContext {
            timer_call: true,
            current_tick: 123 * TICKS_PER_SECOND as u32,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(drdata_u32(&jewel, 0), 123);
    assert_eq!(
        outcome,
        ItemDriverOutcome::ClanJewelRescheduled {
            item_id: ItemId(8),
            schedule_after_ticks: CLANJEWEL_CHECK_INTERVAL_TICKS,
        }
    );
}

#[test]
fn clanjewel_driver_expires_after_one_hour_timer_lifetime() {
    let mut timer_character = character(0);
    let mut jewel = item(8, ItemFlags::USED, 0, IDR_CLANJEWEL);
    jewel.name = "Clan Jewel".into();
    jewel.carried_by = Some(CharacterId(42));
    set_drdata_u32(&mut jewel, 0, 100);

    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut jewel,
        ItemDriverRequest::Driver {
            driver: IDR_CLANJEWEL,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        30,
        false,
        &ItemDriverContext {
            timer_call: true,
            current_tick: (100 + CLANJEWEL_LIFETIME_SECONDS + 1) * TICKS_PER_SECOND as u32,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::ClanJewelExpired {
            item_id: ItemId(8),
            character_id: Some(CharacterId(42)),
            item_name: outcome_item_name("Clan Jewel"),
        }
    );
}

#[test]
fn clanjewel_driver_ignores_direct_character_use() {
    let mut character = character(1);
    let mut jewel = item(8, ItemFlags::USED, 0, IDR_CLANJEWEL);

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut jewel,
        ItemDriverRequest::Driver {
            driver: IDR_CLANJEWEL,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        30,
        false,
        &ItemDriverContext::default(),
    );

    assert_eq!(outcome, ItemDriverOutcome::Noop);
    assert_eq!(drdata_u32(&jewel, 0), 0);
}
