use super::*;

#[test]
fn valid_names_are_queued_without_an_immediate_reply() {
    let mut world = World::default();
    world.queue_lastseen_lookup(CharacterId(1), "Godmode");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_lastseen_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].requester_id, CharacterId(1));
    assert_eq!(queued[0].target_name, "Godmode");
}

#[test]
fn empty_name_is_rejected_immediately() {
    let mut world = World::default();
    world.queue_lastseen_lookup(CharacterId(1), "");

    assert!(world.drain_pending_lastseen_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "No character by the name .");
}

#[test]
fn single_character_name_is_rejected_immediately() {
    // C `lookup_name`'s `strlen(name) < 2` gate (`lookup.c:57-59`).
    let mut world = World::default();
    world.queue_lastseen_lookup(CharacterId(1), "A");

    assert!(world.drain_pending_lastseen_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts[0].message, "No character by the name A.");
}

#[test]
fn overlong_name_is_rejected_immediately() {
    // C `lookup_name`'s `strlen(name) > 38` gate (`lookup.c:54-56`).
    let mut world = World::default();
    let long_name = "a".repeat(39);
    world.queue_lastseen_lookup(CharacterId(1), &long_name);

    assert!(world.drain_pending_lastseen_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(
        texts[0].message,
        format!("No character by the name {long_name}.")
    );
}

#[test]
fn name_with_non_alphabetic_byte_is_rejected_immediately() {
    // C `lookup_name`'s `!isalpha(*ptr)` gate (`lookup.c:49-52`) - catches
    // spaces, digits, and punctuation alike.
    let mut world = World::default();
    world.queue_lastseen_lookup(CharacterId(1), "John Doe");

    assert!(world.drain_pending_lastseen_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts[0].message, "No character by the name John Doe.");
}

#[test]
fn trailing_whitespace_is_rejected_just_like_c() {
    // A genuine C quirk (documented, not "fixed"): `lookup_name` only
    // trims leading whitespace at the call site (`command.c:9033-9035`);
    // trailing whitespace left in `ptr` fails the `isalpha` scan just like
    // any other non-alphabetic byte.
    let mut world = World::default();
    world.queue_lastseen_lookup(CharacterId(1), "Godmode ");

    assert!(world.drain_pending_lastseen_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts[0].message, "No character by the name Godmode .");
}
