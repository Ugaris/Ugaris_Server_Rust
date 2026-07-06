use super::*;

#[test]
fn lockname_queues_the_lowercased_name_alongside_the_original() {
    let mut world = World::default();
    world.queue_lockname_command(CharacterId(1), "BadName");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_lockname_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].requester_id, CharacterId(1));
    assert_eq!(queued[0].original_name, "BadName");
    assert_eq!(queued[0].lookup_name, "badname");
}

#[test]
fn unlockname_queues_the_lowercased_name_alongside_the_original() {
    let mut world = World::default();
    world.queue_unlockname_command(CharacterId(1), "BadName");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_unlockname_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].original_name, "BadName");
    assert_eq!(queued[0].lookup_name, "badname");
}

#[test]
fn length_is_checked_before_the_alpha_loop() {
    // A too-short name that also happens to contain a digit: C's own
    // `db_lockname` checks `strlen` first, so the length message wins
    // even though the alpha loop would also reject it.
    let mut world = World::default();
    world.queue_lockname_command(CharacterId(1), "a1");

    assert!(world.drain_pending_lockname_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Name too long or too short.");
}

#[test]
fn too_long_name_is_rejected() {
    let mut world = World::default();
    let long_name = "a".repeat(36);
    world.queue_lockname_command(CharacterId(1), &long_name);

    assert!(world.drain_pending_lockname_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Name too long or too short.");
}

#[test]
fn non_alpha_character_is_rejected_after_the_length_check_passes() {
    let mut world = World::default();
    world.queue_unlockname_command(CharacterId(1), "bad1name");

    assert!(world.drain_pending_unlockname_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Illegal name.");
}
