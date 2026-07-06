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

#[test]
fn ac_status_verified_constant_matches_c_header() {
    assert_eq!(AC_STATUS_VERIFIED, 1);
}

#[test]
fn queue_and_drain_ac_reset_lookup_round_trips() {
    let mut world = World::default();
    world.queue_ac_reset_lookup(CharacterId(4), "Baddie".to_string(), 77);

    let queued = world.drain_pending_ac_reset_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(4));
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].session_id, 77);
    assert!(world.drain_pending_ac_reset_lookups().is_empty());
}

#[test]
fn queue_and_drain_ac_flag_lookup_round_trips() {
    let mut world = World::default();
    world.queue_ac_flag_lookup(CharacterId(5), "Baddie".to_string(), 88);

    let queued = world.drain_pending_ac_flag_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(5));
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].session_id, 88);
    assert!(world.drain_pending_ac_flag_lookups().is_empty());
}

#[test]
fn queue_and_drain_ac_unflag_lookup_round_trips() {
    let mut world = World::default();
    world.queue_ac_unflag_lookup(CharacterId(6), "Baddie".to_string(), 99);

    let queued = world.drain_pending_ac_unflag_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(6));
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].session_id, 99);
    assert!(world.drain_pending_ac_unflag_lookups().is_empty());
}

#[test]
fn queue_and_drain_ac_trust_lookup_round_trips() {
    let mut world = World::default();
    world.queue_ac_trust_lookup(CharacterId(8), "Goodie".to_string(), 101);

    let queued = world.drain_pending_ac_trust_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(8));
    assert_eq!(queued[0].target_name, "Goodie");
    assert_eq!(queued[0].session_id, 101);
    assert!(world.drain_pending_ac_trust_lookups().is_empty());
}

#[test]
fn queue_and_drain_ac_untrust_lookup_round_trips() {
    let mut world = World::default();
    world.queue_ac_untrust_lookup(CharacterId(10), "Goodie".to_string(), 102);

    let queued = world.drain_pending_ac_untrust_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(10));
    assert_eq!(queued[0].target_name, "Goodie");
    assert_eq!(queued[0].session_id, 102);
    assert!(world.drain_pending_ac_untrust_lookups().is_empty());
}

#[test]
fn queue_and_drain_ac_warn_lookup_round_trips() {
    let mut world = World::default();
    world.queue_ac_warn_lookup(
        CharacterId(11),
        CharacterId(12),
        "Baddie".to_string(),
        103,
        "Speedhacking".to_string(),
    );

    let queued = world.drain_pending_ac_warn_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(11));
    assert_eq!(queued[0].target_id, CharacterId(12));
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].session_id, 103);
    assert_eq!(queued[0].reason, "Speedhacking");
    assert!(world.drain_pending_ac_warn_lookups().is_empty());
}

#[test]
fn queue_and_drain_ac_sessions_lookup_round_trips() {
    let mut world = World::default();
    world.queue_ac_sessions_lookup(CharacterId(13), "Baddie".to_string(), 104);

    let queued = world.drain_pending_ac_sessions_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, CharacterId(13));
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].session_id, 104);
    assert!(world.drain_pending_ac_sessions_lookups().is_empty());
}
