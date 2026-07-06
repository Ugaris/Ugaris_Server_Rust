use super::*;

#[test]
fn valid_names_are_queued_with_the_to_name_case_normalized() {
    let mut world = World::default();
    world.queue_rename_command(CharacterId(1), "Baddie", "eviltwin");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_rename_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].requester_id, CharacterId(1));
    assert_eq!(queued[0].from_name, "Baddie");
    assert_eq!(queued[0].to_name, "Eviltwin");
}

#[test]
fn illegal_to_name_character_is_rejected_before_the_length_check() {
    let mut world = World::default();
    // Long enough to pass the length bound, but a digit mid-name should
    // still trip the alpha check first (C's per-character loop bails on
    // the first non-alphabetic byte, before ever reaching its own length
    // check below the loop).
    world.queue_rename_command(CharacterId(1), "Baddie", "evil2twin");

    assert!(world.drain_pending_rename_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "Illegal name.");
}

#[test]
fn to_name_too_short_is_rejected() {
    let mut world = World::default();
    world.queue_rename_command(CharacterId(1), "Baddie", "ab");

    assert!(world.drain_pending_rename_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Name too long or too short.");
}

#[test]
fn to_name_too_long_is_rejected() {
    let mut world = World::default();
    let long_name = "a".repeat(36);
    world.queue_rename_command(CharacterId(1), "Baddie", &long_name);

    assert!(world.drain_pending_rename_lookups().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Name too long or too short.");
}

#[test]
fn from_name_is_passed_through_unvalidated() {
    let mut world = World::default();
    world.queue_rename_command(CharacterId(1), "", "Newname");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_rename_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].from_name, "");
    assert_eq!(queued[0].to_name, "Newname");
}
