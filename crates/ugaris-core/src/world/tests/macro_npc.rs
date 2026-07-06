use super::*;
use crate::character_driver::CDR_MACRO;
use crate::world::macro_npc::MACRO_MUTTERINGS_FOR_TESTS;

fn macro_daemon(id: u32) -> Character {
    let mut daemon = character(id);
    daemon.name = "Macro Daemon".into();
    daemon.driver = CDR_MACRO;
    daemon
}

fn player_at(id: u32, level: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.level = level;
    player
}

#[test]
fn appearance_reskins_between_macro_daemon_and_saint_nick() {
    let mut world = World::default();
    let daemon = macro_daemon(1);
    assert!(world.spawn_character(daemon, 10, 10));

    world.macro_update_appearance(CharacterId(1), false);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.name, "Macro Daemon");
    assert_eq!(character.sprite, 161);

    world.macro_update_appearance(CharacterId(1), true);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.name, "Saint Nick");
    assert_eq!(character.sprite, 13);
}

#[test]
fn macro_daemon_ids_only_returns_live_used_macro_drivers() {
    let mut world = World::default();
    let alive = macro_daemon(1);
    assert!(world.spawn_character(alive, 10, 10));

    let mut dead = macro_daemon(2);
    dead.flags.insert(CharacterFlags::DEAD);
    assert!(world.spawn_character(dead, 11, 10));

    let mut unused = macro_daemon(3);
    unused.flags.remove(CharacterFlags::USED);
    world.characters.insert(CharacterId(3), unused);

    assert_eq!(world.macro_daemon_ids(), vec![CharacterId(1)]);
}

#[test]
fn search_candidates_excludes_low_level_invisible_staff_and_god() {
    let mut world = World::default();
    let eligible = player_at(10, 20);
    assert!(world.spawn_character(eligible, 5, 5));
    let low_level = player_at(11, 5);
    assert!(world.spawn_character(low_level, 5, 6));
    let mut invisible = player_at(12, 20);
    invisible.flags.insert(CharacterFlags::INVISIBLE);
    assert!(world.spawn_character(invisible, 5, 7));
    let mut staff = player_at(13, 20);
    staff.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(staff, 5, 8));
    let mut god = player_at(14, 20);
    god.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(god, 5, 9));

    assert_eq!(world.macro_search_candidates(1, 0), vec![CharacterId(10)]);
}

#[test]
fn search_candidates_respects_area_exclusion_and_from_cursor() {
    let mut world = World::default();
    // Area 22's hardcoded rectangle exclusion (matches
    // `macro_is_area_excluded`'s own test).
    let excluded = player_at(20, 20);
    assert!(world.spawn_character(excluded, 80, 25));
    let included = player_at(21, 20);
    assert!(world.spawn_character(included, 10, 10));
    let before_cursor = player_at(5, 20);
    assert!(world.spawn_character(before_cursor, 10, 11));

    let candidates = world.macro_search_candidates(22, 10);
    assert_eq!(candidates, vec![CharacterId(21)]);
}

#[test]
fn give_message_destroys_any_cursor_item() {
    let mut world = World::default();
    let daemon = macro_daemon(1);
    assert!(world.spawn_character(daemon, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .cursor_item = Some(ItemId(900));

    world.macro_handle_give_message(CharacterId(1));

    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(!world.items.contains_key(&ItemId(900)));
}

#[test]
fn idle_mutter_only_fires_on_the_one_in_two_hundred_roll() {
    let mut world = World::default();
    let daemon = macro_daemon(1);
    assert!(world.spawn_character(daemon, 10, 10));

    world.legacy_random_seed = 1;
    let mut fired = 0;
    for _ in 0..2000 {
        world.pending_area_texts.clear();
        world.macro_idle_mutter(CharacterId(1));
        if !world.pending_area_texts.is_empty() {
            fired += 1;
            let message = &world.pending_area_texts[0].message;
            assert!(MACRO_MUTTERINGS_FOR_TESTS
                .iter()
                .any(|line| message.contains(line)));
        }
    }
    assert!(fired > 0, "expected at least one murmur across 2000 rolls");
}
