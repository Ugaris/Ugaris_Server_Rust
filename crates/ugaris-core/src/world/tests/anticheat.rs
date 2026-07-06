use super::*;

#[test]
fn ac_status_string_matches_c_table() {
    assert_eq!(ac_status_string(0), "unverified");
    assert_eq!(ac_status_string(1), "verified");
    assert_eq!(ac_status_string(2), "suspicious");
    assert_eq!(ac_status_string(3), "flagged");
    assert_eq!(ac_status_string(99), "unknown");
}

#[test]
fn queue_and_drain_ac_status_lookup_round_trips() {
    let mut world = World::default();
    world.queue_ac_status_lookup(CharacterId(1), "Baddie".to_string(), 42);

    let queued = world.drain_pending_ac_status_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(1));
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].session_id, 42);
    assert!(world.drain_pending_ac_status_lookups().is_empty());
}

#[test]
fn queue_and_drain_ac_list_lookup_round_trips() {
    let mut world = World::default();
    let targets = vec![
        AcOnlineTarget {
            name: "Alice".to_string(),
            session_id: 1,
        },
        AcOnlineTarget {
            name: "Bob".to_string(),
            session_id: 2,
        },
    ];
    world.queue_ac_list_lookup(CharacterId(1), targets.clone());

    let queued = world.drain_pending_ac_list_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(1));
    assert_eq!(queued[0].targets, targets);
    assert!(world.drain_pending_ac_list_lookups().is_empty());
}

#[test]
fn queue_and_drain_ac_stats_lookup_round_trips() {
    let mut world = World::default();
    let targets = vec![AcOnlineTarget {
        name: "Alice".to_string(),
        session_id: 1,
    }];
    world.queue_ac_stats_lookup(CharacterId(7), targets.clone());

    let queued = world.drain_pending_ac_stats_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(7));
    assert_eq!(queued[0].targets, targets);
    assert!(world.drain_pending_ac_stats_lookups().is_empty());
}

#[test]
fn queue_and_drain_ac_suspicious_lookup_round_trips() {
    let mut world = World::default();
    let targets = vec![AcOnlineTarget {
        name: "Alice".to_string(),
        session_id: 1,
    }];
    world.queue_ac_suspicious_lookup(CharacterId(9), targets.clone());

    let queued = world.drain_pending_ac_suspicious_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(9));
    assert_eq!(queued[0].targets, targets);
    assert!(world.drain_pending_ac_suspicious_lookups().is_empty());
}

#[test]
fn ac_status_suspicious_constant_matches_c_header() {
    assert_eq!(AC_STATUS_SUSPICIOUS, 2);
}

#[test]
fn queue_and_drain_ac_cleanup_lookup_round_trips() {
    let mut world = World::default();
    world.queue_ac_cleanup_lookup(CharacterId(3), 30);

    let queued = world.drain_pending_ac_cleanup_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(3));
    assert_eq!(queued[0].days, 30);
    assert!(world.drain_pending_ac_cleanup_lookups().is_empty());
}
