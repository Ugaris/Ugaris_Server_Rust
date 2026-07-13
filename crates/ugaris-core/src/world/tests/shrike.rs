use super::*;
use crate::direction::Direction;
use crate::item_driver::{
    ShrikeAmbientKind, ShrikeAmuletPiece, IDR_FORESTSPADE, IDR_SHRIKE, IDR_SHRIKEAMULET,
    IID_SHRIKE_TALISMAN,
};

fn shrike_item(id: u32, sub_driver: u8, x: u16, y: u16) -> Item {
    let mut it = item(id, ItemFlags::USED | ItemFlags::USE);
    it.driver = IDR_SHRIKE;
    it.driver_data = vec![sub_driver];
    it.x = x;
    it.y = y;
    it
}

fn shrike_request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_SHRIKE,
        item_id: ItemId(item_id),
        character_id: CharacterId(character_id),
        spec: 0,
    }
}

// C `tree_driver`'s `!cn` branch (`shrike.c:88-104`), driven end-to-end
// through `World::process_due_timers` like every other ambient-item
// timer in this codebase.
#[test]
fn ambient_refresh_swaps_sprite_and_reschedules_via_the_timer_pipeline() {
    let mut world = World::default();
    world.date.moonlight = 1;
    world.date.sunlight = 0;
    let tree = shrike_item(7, 1, 10, 10);
    world.add_item(tree);

    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
    world.advance();
    let outcomes = world.process_due_timers(38);
    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::ShrikeAmbientRefresh {
            item_id: ItemId(7),
            x: 10,
            y: 10,
            kind: ShrikeAmbientKind::Tree,
            night: true,
            schedule_after_ticks: TICKS_PER_SECOND * 60,
        }]
    );
    let tree = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(tree.sprite, 51631);
    assert_eq!(
        tree.description,
        "A silver chain is hanging from one twig of this three."
    );

    // The unconditional reschedule fires again `TICKS_PER_SECOND * 60`
    // ticks later.
    for _ in 0..(TICKS_PER_SECOND * 60) {
        world.advance();
    }
    let outcomes = world.process_due_timers(38);
    assert_eq!(outcomes.len(), 1);
}

// C `pool_driver`'s success branch (`shrike.c:276-280`).
#[test]
fn pool_talisman_creation_rewrites_template_id_and_description() {
    let mut world = World::default();
    world.date.moonlight = 1;
    world.date.sunlight = 0;
    let mut player = character(1);
    player.cursor_item = Some(ItemId(9));
    world.add_character(player);
    let pool = shrike_item(1, 4, 10, 10);
    world.add_item(pool);
    let mut amulet = item(9, ItemFlags::USED | ItemFlags::USE);
    amulet.driver = IDR_SHRIKEAMULET;
    amulet.driver_data = vec![7];
    amulet.carried_by = Some(CharacterId(1));
    world.add_item(amulet);

    let outcome = world.execute_item_driver_request(shrike_request(1, 1), 38);
    assert_eq!(
        outcome,
        ItemDriverOutcome::ShrikePoolTalismanCreated {
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
        }
    );
    let amulet = world.items.get(&ItemId(9)).unwrap();
    assert_eq!(amulet.template_id, IID_SHRIKE_TALISMAN);
    assert_eq!(amulet.description, "The Talisman of the Moon.");
}

// C `door_driver`'s success branch (`shrike.c:243-247`): `change_area(cn,
// 38, 8, 92)`, ported as a same-area teleport.
#[test]
fn door_enter_teleports_character_to_the_moon_door_target() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 65;
    player.cursor_item = Some(ItemId(9));
    player.x = 20;
    player.y = 20;
    world.add_character(player);
    let door = shrike_item(1, 3, 20, 20);
    world.add_item(door);
    let mut talisman = item(9, ItemFlags::USED | ItemFlags::USE);
    talisman.template_id = IID_SHRIKE_TALISMAN;
    talisman.carried_by = Some(CharacterId(1));
    world.add_item(talisman);

    let outcome = world.execute_item_driver_request(shrike_request(1, 1), 38);
    assert_eq!(
        outcome,
        ItemDriverOutcome::ShrikeDoorEnter {
            character_id: CharacterId(1),
        }
    );
    let player = &world.characters[&CharacterId(1)];
    assert_eq!((player.x, player.y), (8, 92));
}

// C `cube_driver`'s player-push branch (`shrike.c:283-310`): the cube
// slides one tile toward the direction the character is facing, and its
// last-touch tick is recorded for the auto-reset timer.
#[test]
fn cube_push_relocates_the_item_and_records_the_touch_tick() {
    let mut world = World {
        tick: Tick(500),
        ..World::default()
    };
    let mut player = character(1);
    player.x = 10;
    player.y = 10;
    player.dir = Direction::Right as u8;
    world.add_character(player);

    world.map.set_flags(11, 10, MapFlags::empty());
    if let Some(tile) = world.map.tile_mut(11, 10) {
        tile.ground_sprite = 59755;
    }
    let cube = shrike_item(1, 5, 10, 10);
    world.add_item(cube);
    world.map.tile_mut(10, 10).unwrap().item = 1;

    let outcome = world.execute_item_driver_request(shrike_request(1, 1), 38);
    assert_eq!(
        outcome,
        ItemDriverOutcome::ShrikeCubePush {
            item_id: ItemId(1),
            character_id: CharacterId(1),
            from_x: 10,
            from_y: 10,
            to_x: 11,
            to_y: 10,
        }
    );
    let cube = world.items.get(&ItemId(1)).unwrap();
    assert_eq!((cube.x, cube.y), (11, 10));
    assert!(world
        .map
        .tile(11, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
    assert_eq!(world.map.tile(11, 10).unwrap().item, 1);
    assert!(!world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
    assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
    assert_eq!(crate::item_driver::drdata_u32(cube, 4), 500);
}

// C `cube_driver`'s player-push blocked branch (`shrike.c:262-268`): the
// target tile's ground sprite isn't in the walkable-floor range.
#[test]
fn cube_push_is_blocked_when_target_ground_sprite_is_wrong() {
    let mut world = World::default();
    let mut player = character(1);
    player.x = 10;
    player.y = 10;
    player.dir = Direction::Right as u8;
    world.add_character(player);
    let cube = shrike_item(1, 5, 10, 10);
    world.add_item(cube);

    let outcome = world.execute_item_driver_request(shrike_request(1, 1), 38);
    assert_eq!(
        outcome,
        ItemDriverOutcome::ShrikeCubeBlocked {
            character_id: CharacterId(1),
        }
    );
}

// C `rock_driver`'s wrong-tool branch (`shrike.c:194-198`) driven
// end-to-end, confirming the generic cursor-context (`cursor_driver`)
// World already computes for every item driver covers `IDR_SHRIKE` too.
#[test]
fn rock_dig_requires_a_forestspade_end_to_end() {
    let mut world = World::default();
    world.date.moonlight = 1;
    world.date.sunlight = 0;
    let mut player = character(1);
    player.cursor_item = Some(ItemId(9));
    world.add_character(player);
    let rock = shrike_item(1, 2, 10, 10);
    world.add_item(rock);
    let mut spade = item(9, ItemFlags::USED | ItemFlags::USE);
    spade.driver = IDR_FORESTSPADE;
    spade.carried_by = Some(CharacterId(1));
    world.add_item(spade);

    let outcome = world.execute_item_driver_request(shrike_request(1, 1), 38);
    assert_eq!(
        outcome,
        ItemDriverOutcome::ShrikeRockDigSuccess {
            item_id: ItemId(1),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            piece: ShrikeAmuletPiece::Charm,
        }
    );
}
