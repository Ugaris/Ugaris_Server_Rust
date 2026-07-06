use super::*;

#[test]
fn exterminate_queues_the_exact_parsed_name() {
    let mut world = World::default();
    world.queue_exterminate_command(CharacterId(1), "Baddie");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_exterminate_requests();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(1));
    assert_eq!(queued[0].target_name, "Baddie");
}

#[test]
fn unlike_lockname_no_length_or_charset_validation_runs_before_queuing() {
    // C `cmd_exterminate` has no synchronous validation beyond its own
    // `isalpha` parse loop (already applied by the caller before this
    // method ever runs) - even a name too short/long for `/lockname`
    // queues unconditionally here, letting the DB round trip itself
    // decide "not found".
    let mut world = World::default();
    world.queue_exterminate_command(CharacterId(1), "ab");
    world.queue_exterminate_command(CharacterId(2), &"a".repeat(90));

    assert!(world.drain_pending_system_texts().is_empty());
    assert_eq!(world.drain_pending_exterminate_requests().len(), 2);
}

#[test]
fn an_empty_name_still_queues_matching_cs_zero_iteration_parse_loop() {
    let mut world = World::default();
    world.queue_exterminate_command(CharacterId(1), "");

    assert!(world.drain_pending_system_texts().is_empty());
    let queued = world.drain_pending_exterminate_requests();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].target_name, "");
}

#[test]
fn multiple_requests_drain_in_fifo_order() {
    let mut world = World::default();
    world.queue_exterminate_command(CharacterId(1), "First");
    world.queue_exterminate_command(CharacterId(2), "Second");

    let queued = world.drain_pending_exterminate_requests();
    assert_eq!(queued.len(), 2);
    assert_eq!(queued[0].target_name, "First");
    assert_eq!(queued[1].target_name, "Second");
    assert!(world.drain_pending_exterminate_requests().is_empty());
}
