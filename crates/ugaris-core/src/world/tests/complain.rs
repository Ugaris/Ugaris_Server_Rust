use super::*;

#[test]
fn valid_name_is_queued_without_an_immediate_reply() {
    let mut world = World::default();
    world.queue_complain_lookup(CharacterId(1), "Godmode");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_complain_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].requester_id, CharacterId(1));
    assert_eq!(queued[0].target_name, "Godmode");
}

#[test]
fn short_name_is_rejected_immediately() {
    // C's own `if (n < 3 || n > 40) ret = -n;` bound (`command.c:2325`),
    // tighter than (and checked before) `lookup_name`'s own `2..=38` gate.
    let mut world = World::default();
    world.queue_complain_lookup(CharacterId(1), "Ab");

    assert!(world.drain_pending_complain_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "Sorry, no player by the name 'Ab' found.");
}

#[test]
fn overlong_name_is_rejected_immediately() {
    let mut world = World::default();
    let long_name = "a".repeat(41);
    world.queue_complain_lookup(CharacterId(1), &long_name);

    assert!(world.drain_pending_complain_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(
        texts[0].message,
        format!("Sorry, no player by the name '{long_name}' found.")
    );
}

#[test]
fn boundary_lengths_three_and_forty_are_queued() {
    let mut world = World::default();
    world.queue_complain_lookup(CharacterId(1), &"a".repeat(3));
    world.queue_complain_lookup(CharacterId(1), &"a".repeat(40));

    assert!(world.drain_pending_system_texts().is_empty());
    assert_eq!(world.drain_pending_complain_lookups().len(), 2);
}
