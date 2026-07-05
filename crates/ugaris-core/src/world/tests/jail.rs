use super::*;

fn make_player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.name = name.to_string();
    player.flags |= CharacterFlags::PLAYER;
    player
}

#[test]
fn invalid_name_is_rejected_immediately() {
    let mut world = World::default();
    world.queue_jail_lookup(CharacterId(1), "A", JailAction::Jail);

    assert!(world.drain_pending_jail_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "No character by the name A.");
}

#[test]
fn valid_name_is_queued_without_an_immediate_reply() {
    let mut world = World::default();
    world.queue_jail_lookup(CharacterId(1), "  Baddie  ", JailAction::Unjail);

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_jail_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(1));
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].action, JailAction::Unjail);
}

#[test]
fn resolve_with_no_online_match_tells_caller_no_player() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), make_player(1, "Godmode"));

    world.resolve_jail_lookup(CharacterId(1), "Ghost", JailAction::Jail);

    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "No player by that name.");
}

#[test]
fn jail_sets_respawn_point_and_flag_and_teleports_locally() {
    let mut world = World::default();
    world.area_id = 3;
    world.settings.jail_x = 186;
    world.settings.jail_y = 234;
    world.settings.jail_area = 3;

    world
        .characters
        .insert(CharacterId(1), make_player(1, "Godmode"));
    let mut target = make_player(2, "Baddie");
    target.x = 50;
    target.y = 50;
    world.map.set_char(&mut target, 50, 50);
    world.characters.insert(CharacterId(2), target);

    world.resolve_jail_lookup(CharacterId(1), "baddie", JailAction::Jail);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.rest_x, 186);
    assert_eq!(target.rest_y, 234);
    assert_eq!(target.rest_area, 3);
    assert!(target.flags.contains(CharacterFlags::RESPAWN));
    assert_eq!(target.x, 186);
    assert_eq!(target.y, 234);

    let mut texts = world.drain_pending_system_texts();
    texts.sort_by_key(|text| text.character_id.0);
    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "You have jailed Baddie.");
    assert_eq!(texts[1].character_id, CharacterId(2));
    assert_eq!(texts[1].message, "You have been jailed by Godmode.");
}

#[test]
fn unjail_sets_aston_respawn_without_respawn_flag() {
    let mut world = World::default();
    world.area_id = 3;
    world.settings.aston_x = 133;
    world.settings.aston_y = 203;
    world.settings.aston_area = 3;

    world
        .characters
        .insert(CharacterId(1), make_player(1, "Godmode"));
    let mut target = make_player(2, "Baddie");
    target.x = 50;
    target.y = 50;
    world.map.set_char(&mut target, 50, 50);
    world.characters.insert(CharacterId(2), target);

    world.resolve_jail_lookup(CharacterId(1), "Baddie", JailAction::Unjail);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.rest_x, 133);
    assert_eq!(target.rest_y, 203);
    assert_eq!(target.rest_area, 3);
    assert!(!target.flags.contains(CharacterFlags::RESPAWN));

    let mut texts = world.drain_pending_system_texts();
    texts.sort_by_key(|text| text.character_id.0);
    assert_eq!(texts[0].message, "You have unjailed Baddie.");
    assert_eq!(texts[1].message, "You have been unjailed by Godmode.");
}

#[test]
fn cross_area_target_gets_the_shared_server_down_message() {
    let mut world = World::default();
    world.area_id = 1; // current server is NOT the jail area
    world.settings.jail_x = 186;
    world.settings.jail_y = 234;
    world.settings.jail_area = 3;

    world
        .characters
        .insert(CharacterId(1), make_player(1, "Godmode"));
    let mut target = make_player(2, "Baddie");
    target.x = 50;
    target.y = 50;
    world.map.set_char(&mut target, 50, 50);
    world.characters.insert(CharacterId(2), target);

    world.resolve_jail_lookup(CharacterId(1), "Baddie", JailAction::Jail);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    // Respawn point is still set unconditionally, matching C.
    assert_eq!(target.rest_x, 186);
    assert_eq!(target.rest_y, 234);
    assert_eq!(target.rest_area, 3);
    // Position itself is unchanged (no local teleport happened).
    assert_eq!(target.x, 50);
    assert_eq!(target.y, 50);

    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.character_id == CharacterId(1)
        && text.message == "Nothing happens - target area server is down."));
}
