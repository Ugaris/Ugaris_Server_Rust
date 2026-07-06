use super::*;

#[test]
fn queue_and_drain_round_trip() {
    let mut world = World::default();
    world.queue_querystats_lookup(CharacterId(1));

    let queued = world.drain_pending_querystats_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(1));

    // Draining empties the queue.
    assert!(world.drain_pending_querystats_lookups().is_empty());
}

#[test]
fn multiple_lookups_queue_independently() {
    let mut world = World::default();
    world.queue_querystats_lookup(CharacterId(1));
    world.queue_querystats_lookup(CharacterId(2));

    let queued = world.drain_pending_querystats_lookups();
    assert_eq!(queued.len(), 2);
    assert_eq!(queued[0].caller_id, CharacterId(1));
    assert_eq!(queued[1].caller_id, CharacterId(2));
}
