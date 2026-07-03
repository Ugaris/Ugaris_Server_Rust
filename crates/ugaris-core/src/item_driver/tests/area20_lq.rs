use super::*;

#[test]
fn lq_ticker_timer_call_reschedules_every_second() {
    let mut actor = character(0);
    let mut ticker = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LQ_TICKER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LQ_TICKER,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut ticker,
            request,
            20,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LqTicker {
            item_id: ItemId(7),
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
}

#[test]
fn lq_ticker_character_call_is_handled_noop() {
    let mut actor = character(1);
    let mut ticker = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LQ_TICKER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LQ_TICKER,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut ticker,
            request,
            20,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Noop,
    );
}

#[test]
fn lq_ticker_is_area20_guarded_like_legacy_module() {
    let mut actor = character(0);
    let mut ticker = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LQ_TICKER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LQ_TICKER,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut ticker,
            request,
            1,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_LQ_TICKER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            required_area: 20,
        }
    );
}

#[test]
fn lq_entrance_is_area20_guarded_and_zero_character_noops() {
    let mut actor = character(1);
    let mut entrance = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LQ_ENTRANCE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LQ_ENTRANCE,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut entrance,
            request,
            1,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_LQ_ENTRANCE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            required_area: 20,
        }
    );

    let mut timer_actor = character(0);
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_LQ_ENTRANCE,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_actor,
            &mut entrance,
            timer_request,
            20,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Noop,
    );
}

#[test]
fn lq_entrance_blocks_closed_level_missing_target_and_penalty() {
    let mut actor = character(1);
    actor.level = 15;
    let mut entrance = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LQ_ENTRANCE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LQ_ENTRANCE,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut entrance,
            request,
            20,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LqEntranceClosed {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let level_block = ItemDriverContext {
        lq_open: true,
        lq_min_level: 20,
        lq_max_level: 30,
        lq_entrance: Some((100, 101)),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut entrance,
            request,
            20,
            false,
            &level_block,
        ),
        ItemDriverOutcome::LqEntranceLevelBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            min_level: 20,
            max_level: 30,
        }
    );

    let missing_target = ItemDriverContext {
        lq_open: true,
        lq_min_level: 10,
        lq_max_level: 30,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut entrance,
            request,
            20,
            false,
            &missing_target,
        ),
        ItemDriverOutcome::LqEntranceUndefined {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let penalized = ItemDriverContext {
        lq_open: true,
        lq_min_level: 10,
        lq_max_level: 30,
        lq_entrance: Some((100, 101)),
        lq_death_penalty_seconds: Some(125),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut entrance, request, 20, false, &penalized,),
        ItemDriverOutcome::LqEntrancePenalty {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            remaining_seconds: 125,
        }
    );
}

#[test]
fn lq_entrance_success_returns_quiet_same_area_teleport() {
    let mut actor = character(1);
    actor.level = 25;
    let mut entrance = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LQ_ENTRANCE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LQ_ENTRANCE,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut entrance,
            request,
            20,
            false,
            &ItemDriverContext {
                lq_open: true,
                lq_min_level: 20,
                lq_max_level: 30,
                lq_entrance: Some((123, 45)),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::Teleport {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 123,
            y: 45,
            area_id: 20,
            stop_driver: true,
            quiet: true,
        }
    );
}
