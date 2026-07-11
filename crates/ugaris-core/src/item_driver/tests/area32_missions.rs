use super::*;

// C `missionchest_driver`'s pure gate (`missions.c:1795-1797`, `if (!cn)
// return;`): a `cn==0` timer/ambient call never reaches the real logic -
// every other check needs the acting player's `governor: MissionPpd` and
// a `ZoneLoader`, both deferred to `ugaris-server::area32::
// apply_mission_chest_open`.
#[test]
fn missionchest_driver_ignores_character_zero() {
    let mut timer_character = character(0);
    let mut chest = item(200, ItemFlags::USED | ItemFlags::USE, 0, IDR_MISSIONCHEST);
    chest.driver_data = vec![0, 0, 0, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_MISSIONCHEST,
        item_id: ItemId(200),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut timer_character, &mut chest, request, 32, false),
        ItemDriverOutcome::Noop
    );
}

// A real player use dispatches straight to `MissionChestOpen` - every
// other decision (key check, `md->itemtemp`, cursor, `find_item`
// bookkeeping) is deferred server-side.
#[test]
fn missionchest_driver_dispatches_for_a_real_player() {
    let mut actor = character(7);
    let mut chest = item(200, ItemFlags::USED | ItemFlags::USE, 0, IDR_MISSIONCHEST);
    chest.driver_data = vec![0, 0, 0, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_MISSIONCHEST,
        item_id: ItemId(200),
        character_id: CharacterId(7),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut chest, request, 32, false),
        ItemDriverOutcome::MissionChestOpen {
            item_id: ItemId(200),
            character_id: CharacterId(7),
        }
    );
}
