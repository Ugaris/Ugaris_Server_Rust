use super::*;
use crate::character_driver::{
    CharacterDriverMessage, MoonieDriverData, CDR_MOONIE, NT_GOTHIT, NT_ITEM,
};
use crate::item_driver::IID_AREA2_SMALLSPIDER;

fn moonie_npc(id: u32) -> Character {
    let mut moonie = character(id);
    moonie.name = "Moony".into();
    moonie.driver = CDR_MOONIE;
    moonie.driver_state = Some(CharacterDriverState::Moonie(MoonieDriverData::default()));
    moonie
}

fn moonie_state(world: &World, id: CharacterId) -> MoonieDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Moonie(data)) => data,
        _ => panic!("expected moonie driver state"),
    }
}

#[test]
fn moonie_wants_a_visible_small_spider() {
    // C `area2.c:479-490`.
    let mut world = World::default();
    let moonie = moonie_npc(1);
    assert!(world.spawn_character(moonie, 10, 10));
    let mut spider = item(9, ItemFlags::USED);
    spider.template_id = IID_AREA2_SMALLSPIDER;
    assert!(world.map.set_item_map(&mut spider, 10, 11));
    world.items.insert(ItemId(9), spider);
    if let Some(moonie) = world.characters.get_mut(&CharacterId(1)) {
        moonie.driver_messages.push(CharacterDriverMessage {
            message_type: NT_ITEM,
            dat1: 9,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }

    world.process_moonie_actions(1);

    assert_eq!(
        moonie_state(&world, CharacterId(1)).want_it,
        Some(ItemId(9))
    );
}

#[test]
fn moonie_does_not_re_want_a_spider_while_already_busy() {
    let mut world = World::default();
    let mut moonie = moonie_npc(1);
    moonie.driver_state = Some(CharacterDriverState::Moonie(MoonieDriverData {
        want_it: Some(ItemId(5)),
        ..Default::default()
    }));
    assert!(world.spawn_character(moonie, 10, 10));
    let mut spider = item(9, ItemFlags::USED);
    spider.template_id = IID_AREA2_SMALLSPIDER;
    assert!(world.map.set_item_map(&mut spider, 10, 11));
    world.items.insert(ItemId(9), spider);
    if let Some(moonie) = world.characters.get_mut(&CharacterId(1)) {
        moonie.driver_messages.push(CharacterDriverMessage {
            message_type: NT_ITEM,
            dat1: 9,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }

    world.process_moonie_actions(1);

    // `want_it` (item 5) is never overwritten with item 9 while already
    // busy wanting something; it's separately cleared to `None` by the
    // "still visible?" check since item 5 no longer exists at all.
    assert_eq!(moonie_state(&world, CharacterId(1)).want_it, None);
}

#[test]
fn moonie_eats_the_spider_on_its_cursor_and_starts_munching() {
    // C `area2.c:507-515`.
    let mut world = World::default();
    let mut moonie = moonie_npc(1);
    let mut spider = item(9, ItemFlags::USED);
    spider.template_id = IID_AREA2_SMALLSPIDER;
    spider.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(9), spider);
    moonie.cursor_item = Some(ItemId(9));
    world.tick.0 = 500;
    assert!(world.spawn_character(moonie, 10, 10));

    world.process_moonie_actions(1);

    assert!(!world.items.contains_key(&ItemId(9)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
    let state = moonie_state(&world, CharacterId(1));
    assert_eq!(state.yummy, 500 + 60 * TICKS_PER_SECOND);
    let moonie = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(moonie.action, action::IDLE);
}

#[test]
fn moonie_idles_while_yummy_timer_is_still_running() {
    let mut world = World::default();
    let mut moonie = moonie_npc(1);
    moonie.driver_state = Some(CharacterDriverState::Moonie(MoonieDriverData {
        yummy: 10_000,
        lastmunch: 100,
        ..Default::default()
    }));
    // Under the 10-second `TICKS_PER_SECOND * 10` munch-message throttle
    // (`area2.c:518`): `lastmunch` must stay unchanged.
    world.tick.0 = 200;
    assert!(world.spawn_character(moonie, 10, 10));

    world.process_moonie_actions(1);

    let state = moonie_state(&world, CharacterId(1));
    assert_eq!(state.yummy, 10_000);
    assert_eq!(state.lastmunch, 100);
    let moonie = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(moonie.action, action::IDLE);
}

#[test]
fn moonie_repeats_munch_message_after_ten_seconds() {
    let mut world = World::default();
    let mut moonie = moonie_npc(1);
    moonie.driver_state = Some(CharacterDriverState::Moonie(MoonieDriverData {
        yummy: 100_000,
        lastmunch: 100,
        ..Default::default()
    }));
    world.tick.0 = 100 + TICKS_PER_SECOND * 10 + 1;
    assert!(world.spawn_character(moonie, 10, 10));

    world.process_moonie_actions(1);

    let state = moonie_state(&world, CharacterId(1));
    assert_eq!(state.lastmunch, world.tick.0);
}

#[test]
fn moonie_stops_eating_and_forgets_the_spider_when_hit() {
    // C `area2.c:492-498`.
    let mut world = World::default();
    let mut moonie = moonie_npc(1);
    moonie.driver_state = Some(CharacterDriverState::Moonie(MoonieDriverData {
        yummy: 10_000,
        want_it: Some(ItemId(3)),
        ..Default::default()
    }));
    assert!(world.spawn_character(moonie, 10, 10));
    let mut attacker = character(2);
    attacker.group = 1;
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(moonie) = world.characters.get_mut(&CharacterId(1)) {
        moonie.driver_messages.push(CharacterDriverMessage {
            message_type: NT_GOTHIT,
            dat1: 2,
            dat2: 5,
            dat3: 0,
            text: None,
        });
    }

    world.process_moonie_actions(1);

    let state = moonie_state(&world, CharacterId(1));
    assert_eq!(state.yummy, 0);
    assert_eq!(state.want_it, None);
}
