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
    world.queue_rmdeath_lookup(CharacterId(1), "A");

    assert!(world.drain_pending_rmdeath_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "No character by the name A.");
}

#[test]
fn valid_name_is_queued_without_an_immediate_reply() {
    let mut world = World::default();
    world.queue_rmdeath_lookup(CharacterId(1), "  Baddie  ");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_rmdeath_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(1));
    assert_eq!(queued[0].target_name, "Baddie");
}

#[test]
fn resolve_with_no_online_match_tells_caller_no_player() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), make_player(1, "Godmode"));

    world.resolve_rmdeath_lookup(CharacterId(1), "Ghost");

    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "No player by that name.");
}

#[test]
fn resolve_decrements_deaths_and_messages_caller() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), make_player(1, "Godmode"));
    let mut target = make_player(2, "Baddie");
    target.deaths = 5;
    world.characters.insert(CharacterId(2), target);

    world.resolve_rmdeath_lookup(CharacterId(1), "baddie");

    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.deaths, 4);

    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "Removing 1 death from Baddie.");
}

#[test]
fn resolve_saturates_at_zero_deaths() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), make_player(1, "Godmode"));
    let mut target = make_player(2, "Baddie");
    target.deaths = 0;
    world.characters.insert(CharacterId(2), target);

    world.resolve_rmdeath_lookup(CharacterId(1), "baddie");

    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.deaths, 0);
}
