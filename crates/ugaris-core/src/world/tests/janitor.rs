use super::*;
use crate::character_driver::{CharacterDriverState, JanitorDriverData, CDR_JANITOR};

fn janitor_npc(id: u32) -> Character {
    let mut janitor = character(id);
    janitor.name = "Grimble".into();
    janitor.driver = CDR_JANITOR;
    janitor.driver_state = Some(CharacterDriverState::Janitor(JanitorDriverData::default()));
    janitor
}

/// Spawns a janitor and pins `rest_x`/`rest_y` (C `ch.tmpx`/`tmpy`, the
/// home tile) to the spawn tile - the real zone loader does this
/// (`zone.rs:273-274`), but the bare `character()`/`spawn_character()`
/// test helpers do not.
fn spawn_janitor(world: &mut World, id: u32, x: usize, y: usize) {
    let mut janitor = janitor_npc(id);
    janitor.rest_x = x as u16;
    janitor.rest_y = y as u16;
    assert!(world.spawn_character(janitor, x, y));
}

fn toylight_item(id: u32, x: u16, y: u16, on: bool) -> Item {
    let mut light = item(id, ItemFlags::USE);
    light.driver = IDR_TOYLIGHT;
    light.x = x;
    light.y = y;
    light.driver_data = vec![u8::from(on)];
    light
}

fn take_item_at(id: u32, x: u16, y: u16) -> Item {
    let mut junk = item(id, ItemFlags::TAKE);
    junk.x = x;
    junk.y = y;
    junk
}

fn place_item(world: &mut World, it: Item) {
    let (x, y, id) = (usize::from(it.x), usize::from(it.y), it.id);
    world.map.tile_mut(x, y).unwrap().item = id.0;
    // C `char_see_item`'s `IF_TAKE` light gate needs some light on the
    // tile; real gameplay lights this via `add_character_light`/area
    // lighting, tests pin it directly (same pattern as
    // `world/tests/trader.rs`'s `char_see_char` light pin).
    world.map.tile_mut(x, y).unwrap().light = 255;
    world.items.insert(id, it);
}

#[test]
fn janitor_uses_adjacent_toylight_needing_state_flip() {
    let mut world = World::default();
    world.date.daylight = 20; // dark: ls == 1 (lights should be on)
    spawn_janitor(&mut world, 1, 10, 10);
    place_item(&mut world, toylight_item(900, 11, 10, false));

    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(janitor.action, action::USE);
    assert_eq!(janitor.act1, 900);
    assert_eq!(janitor.dir, Direction::Right as u8);
}

#[test]
fn janitor_walks_toward_distant_toylight_needing_flip() {
    let mut world = World::default();
    world.date.daylight = 20; // dark: ls == 1
    spawn_janitor(&mut world, 1, 10, 10);
    place_item(&mut world, toylight_item(900, 20, 10, false));

    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(janitor.action, action::WALK);
    assert_eq!(janitor.dir, Direction::Right as u8);
}

#[test]
fn janitor_leaves_toylight_alone_when_already_in_desired_state() {
    let mut world = World::default();
    world.date.daylight = 220; // bright: ls == 0 (lights should be off)
    spawn_janitor(&mut world, 1, 10, 10);
    // Already off, matches ls == 0: nothing to do.
    place_item(&mut world, toylight_item(900, 12, 10, false));
    place_item(&mut world, take_item_at(901, 11, 10));

    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(janitor.action, action::TAKE);
    assert_eq!(janitor.act1, 901);
}

#[test]
fn janitor_ignores_take_item_on_opposite_town_half() {
    let mut world = World::default();
    world.date.daylight = 220;
    // Home half is `y < 192` (spawn tile y=10); the junk sits on the
    // other side of the `y == 192` divide and must be ignored (C
    // `janitor_driver`'s `NT_ITEM` town-half filter, `base.c:5107-5111`).
    spawn_janitor(&mut world, 1, 10, 10);
    place_item(&mut world, take_item_at(901, 15, 200));

    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(janitor.action, action::IDLE);
}

#[test]
fn janitor_ignores_take_item_already_in_home_drop_zone() {
    let mut world = World::default();
    world.date.daylight = 220;
    spawn_janitor(&mut world, 1, 10, 10);
    // Sitting on one of the nine home-area tiles: never re-picked up.
    place_item(&mut world, take_item_at(901, 161, 180));

    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(janitor.action, action::IDLE);
}

#[test]
fn janitor_takes_nearest_of_two_visible_junk_items() {
    let mut world = World::default();
    world.date.daylight = 220;
    spawn_janitor(&mut world, 1, 10, 10);
    place_item(&mut world, take_item_at(901, 18, 10));
    place_item(&mut world, take_item_at(902, 12, 10));

    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(janitor.action, action::WALK);
    // Walking toward the closer item (902 at x=12), not the farther one.
    assert_eq!(janitor.dir, Direction::Right as u8);
}

#[test]
fn janitor_absorbs_held_item_into_deep_inventory_bag_when_no_drop_spot_is_free() {
    let mut world = World::default();
    world.date.daylight = 220;
    let mut janitor = janitor_npc(1);
    janitor.cursor_item = Some(ItemId(900));
    janitor.rest_x = 161;
    janitor.rest_y = 179;
    // Placed at home (rest_x/rest_y == spawn tile), so the "walk home"
    // fallback never fires and the tick settles on idle, letting us
    // observe the bag-restore branch in isolation.
    assert!(world.spawn_character(janitor, 161, 179));
    world.items.insert(ItemId(900), take_item_at(900, 0, 0));

    // Block all nine home drop tiles so `janitor_drop_held_item` fails
    // every candidate and the popped bag item is restored.
    for (i, (x, y)) in [
        (161, 180),
        (161, 179),
        (161, 178),
        (162, 178),
        (162, 179),
        (162, 180),
        (162, 181),
        (162, 182),
        (162, 183),
    ]
    .into_iter()
    .enumerate()
    {
        let blocker_id = 1000 + i as u32;
        world.map.tile_mut(x, y).unwrap().item = blocker_id;
        world.items.insert(
            ItemId(blocker_id),
            take_item_at(blocker_id, x as u16, y as u16),
        );
    }

    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    // The held item was absorbed into bag slot 30 at the top of the
    // tick, then popped back out to try dropping it off, failed at
    // every blocked spot, and was restored to slot 30.
    assert_eq!(janitor.inventory[30], Some(ItemId(900)));
    assert!(janitor.cursor_item.is_none());
    assert_eq!(janitor.action, action::IDLE);
}

#[test]
fn janitor_drops_bagged_item_at_first_open_home_spot() {
    let mut world = World::default();
    world.date.daylight = 220;
    let mut janitor = janitor_npc(1);
    janitor.inventory[30] = Some(ItemId(900));
    janitor.rest_x = 161;
    janitor.rest_y = 179;
    // Adjacent (via `Direction::Down`) to the first drop candidate
    // `(161, 180)`.
    assert!(world.spawn_character(janitor, 161, 179));
    world.items.insert(ItemId(900), take_item_at(900, 0, 0));

    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(janitor.action, action::DROP);
    assert_eq!(janitor.act1, 900);
    assert_eq!(janitor.dir, Direction::Down as u8);
    assert!(janitor.inventory[30].is_none());
    assert_eq!(janitor.cursor_item, Some(ItemId(900)));
}

#[test]
fn janitor_drop_skips_occupied_first_spot_for_the_next_candidate() {
    let mut world = World::default();
    world.date.daylight = 220;
    let mut janitor = janitor_npc(1);
    janitor.inventory[30] = Some(ItemId(900));
    janitor.rest_x = 160;
    janitor.rest_y = 179;
    // Block the first candidate `(161, 180)` outright (C `drop_driver`'s
    // pre-pathfind `map[m].it` guard, `base.c:516-519`, checked before
    // adjacency/pathing) so `janitor_drop_held_item` moves on to the
    // second candidate `(161, 179)`, which the janitor stands directly
    // adjacent to (`Direction::Right`).
    world.map.tile_mut(161, 180).unwrap().item = 999;
    world.items.insert(ItemId(999), take_item_at(999, 161, 180));
    assert!(world.spawn_character(janitor, 160, 179));
    world.items.insert(ItemId(900), take_item_at(900, 0, 0));

    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(janitor.action, action::DROP);
    assert_eq!(janitor.act1, 900);
    assert_eq!(janitor.dir, Direction::Right as u8);
}

#[test]
fn janitor_murmurs_fixed_line_after_successful_light_toggle() {
    let mut world = World::default();
    world.date.daylight = 20; // dark: ls == 1
                              // seed=23: RANDOM(50) == 0 (murmur fires), RANDOM(18) == 15 ("Sometimes
                              // I think the dirt's the only thing that listens to me.").
    world.legacy_random_seed = 23;
    spawn_janitor(&mut world, 1, 10, 10);
    place_item(&mut world, toylight_item(900, 11, 10, false));

    world.process_janitor_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message
        == "Grimble murmurs: \"Sometimes I think the dirt's the only thing that listens to me.\""));
}

#[test]
fn janitor_murmurs_dynamic_light_counter_on_case_one() {
    let mut world = World::default();
    world.date.daylight = 20; // dark: ls == 1
                              // seed=61: RANDOM(50) == 0 (murmur fires), RANDOM(18) == 1 (dynamic
                              // "N lights I turned on" counter case, seeded to 25598).
    world.legacy_random_seed = 61;
    spawn_janitor(&mut world, 1, 10, 10);
    place_item(&mut world, toylight_item(900, 11, 10, false));

    world.process_janitor_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message
        == "Grimble murmurs: \"25598 lights I turned on in my life, 25598 lights I turned on in my life...\""));

    match world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .driver_state
        .as_ref()
    {
        Some(CharacterDriverState::Janitor(data)) => assert_eq!(data.cnt, 25599),
        _ => panic!("expected janitor driver state"),
    }
}

#[test]
fn process_janitor_actions_skips_dead_and_unused_characters() {
    let mut world = World::default();
    let mut janitor = janitor_npc(1);
    janitor.flags.remove(CharacterFlags::USED);
    assert!(world.spawn_character(janitor, 10, 10));

    // Should not panic and should not act on the unused character.
    world.process_janitor_actions(0);

    let janitor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(janitor.action, action::IDLE);
}
