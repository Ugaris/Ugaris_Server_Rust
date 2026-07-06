use super::*;

#[test]
fn allow_queues_a_valid_shape_target_name() {
    let mut world = World::default();
    world.queue_allow_command(CharacterId(1), "Bob");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_allow_requests();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(1));
    assert_eq!(queued[0].target_name, "Bob");
}

#[test]
fn allow_rejects_an_invalid_shape_name_immediately() {
    // C `allow_body`'s `coID == -1` branch shares "No player by that
    // name." with a DB-confirmed miss - both collapse to the same
    // immediate reply here, matching `/showvalues`/`/values`.
    let mut world = World::default();
    world.queue_allow_command(CharacterId(1), "");

    assert!(world.drain_pending_allow_requests().is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(texts[0].message, "No player by that name.");
}

#[test]
fn allow_trims_leading_whitespace_before_validating() {
    let mut world = World::default();
    world.queue_allow_command(CharacterId(1), "   Bob");

    let queued = world.drain_pending_allow_requests();
    assert_eq!(queued[0].target_name, "Bob");
}

#[test]
fn grant_grave_access_to_only_updates_graves_the_caller_owns() {
    // C `allow_body_db` (`death.c:1045-1067`): iterates every container,
    // skipping any not owned by the caller - a grave the caller merely
    // *killed* (owner is the victim, not the killer) is untouched, only
    // graves where the caller is the recorded `owner` (their own past
    // deaths) get the access grant.
    let mut world = World::default();
    let mut own_grave = item(10, ItemFlags::USED | ItemFlags::USE);
    own_grave.content_id = 1;
    crate::item_driver::set_grave_acl(&mut own_grave, CharacterId(1), None);
    world.items.insert(ItemId(10), own_grave);

    let mut killed_by_caller = item(11, ItemFlags::USED | ItemFlags::USE);
    killed_by_caller.content_id = 1;
    crate::item_driver::set_grave_acl(&mut killed_by_caller, CharacterId(5), Some(CharacterId(1)));
    world.items.insert(ItemId(11), killed_by_caller);

    let mut unrelated = item(12, ItemFlags::USED | ItemFlags::USE);
    unrelated.content_id = 1;
    crate::item_driver::set_grave_acl(&mut unrelated, CharacterId(9), Some(CharacterId(8)));
    world.items.insert(ItemId(12), unrelated);

    let count = world.grant_grave_access_to(CharacterId(1), CharacterId(2));

    assert_eq!(count, 1, "only the caller's own grave is granted");
    let own_grave = world.items.get(&ItemId(10)).unwrap();
    assert!(!crate::item_driver::grave_access_denied(
        own_grave,
        CharacterId(2)
    ));
    let killed_by_caller = world.items.get(&ItemId(11)).unwrap();
    assert!(
        crate::item_driver::grave_access_denied(killed_by_caller, CharacterId(2)),
        "the caller's kill is not their own grave, so /allow does not touch it"
    );
    let unrelated = world.items.get(&ItemId(12)).unwrap();
    assert!(crate::item_driver::grave_access_denied(
        unrelated,
        CharacterId(2)
    ));
}

#[test]
fn grant_grave_access_to_overwrites_a_previous_grant() {
    let mut world = World::default();
    let mut grave = item(10, ItemFlags::USED | ItemFlags::USE);
    grave.content_id = 1;
    crate::item_driver::set_grave_acl(&mut grave, CharacterId(1), None);
    world.items.insert(ItemId(10), grave);

    world.grant_grave_access_to(CharacterId(1), CharacterId(2));
    world.grant_grave_access_to(CharacterId(1), CharacterId(3));

    let grave = world.items.get(&ItemId(10)).unwrap();
    assert!(
        crate::item_driver::grave_access_denied(grave, CharacterId(2)),
        "granting a new character overwrites the previous grant"
    );
    assert!(!crate::item_driver::grave_access_denied(
        grave,
        CharacterId(3)
    ));
}
