use super::*;

fn make_target(id: u32, name: &str, exp: u32, karma: i32, paid: bool) -> Character {
    let mut target = character(id);
    target.name = name.to_string();
    target.flags |= CharacterFlags::PLAYER;
    target.exp = exp;
    target.karma = karma;
    if paid {
        target.flags |= CharacterFlags::PAID;
    }
    target
}

#[test]
fn punishment_note_round_trips_through_encode_decode() {
    let note = PunishmentNote {
        level: 3,
        exp: 400,
        karma: 4,
        reason: "being mean".to_string(),
    };
    let encoded = encode_punishment_note(&note);
    assert_eq!(encoded.len(), 92);
    let decoded = decode_punishment_note(&encoded).unwrap();
    assert_eq!(decoded, note);
}

#[test]
fn punishment_note_reason_is_truncated_and_nul_terminated_like_c_strncpy() {
    let long_reason = "a".repeat(200);
    let note = PunishmentNote {
        level: 1,
        exp: 1,
        karma: 1,
        reason: long_reason,
    };
    let encoded = encode_punishment_note(&note);
    let decoded = decode_punishment_note(&encoded).unwrap();
    assert_eq!(decoded.reason.len(), 79);
}

#[test]
fn decode_punishment_note_rejects_short_buffers() {
    assert_eq!(decode_punishment_note(&[0u8; 10]), None);
}

#[test]
fn apply_punishment_level_zero_only_warns_no_loss() {
    let mut target = make_target(1, "Baddie", 10_000, 0, true);
    let outcome = apply_punishment(&mut target, 0);
    assert_eq!(outcome.exp_loss, 0);
    assert_eq!(outcome.karma_loss, 0);
    assert_eq!(target.exp, 10_000);
    assert_eq!(target.karma, 0);
    assert!(!outcome.lock);
    assert!(!outcome.kick);
}

#[test]
// The `(x + 3) / 4`-style entries intentionally mirror C `punish()`'s
// switch formulas verbatim.
#[allow(clippy::manual_div_ceil)]
fn apply_punishment_matches_c_per_level_exp_and_karma_table() {
    // C `death_loss(10_000) == 400`; each level's (exp, karma) matches
    // `punish()`'s switch (`punish.c:56-89`).
    let cases: [(u8, u32, i32); 7] = [
        (0, 0, 0),
        (1, (400 + 3) / 4, 1),
        (2, (400 + 1) / 2, 2),
        (3, 400, 4),
        (4, 800, 6),
        (5, 1_600, 8),
        (6, 0, 12),
    ];
    for (level, expected_exp_loss, expected_karma_loss) in cases {
        let mut target = make_target(1, "Baddie", 10_000, 0, true);
        let outcome = apply_punishment(&mut target, level);
        assert_eq!(outcome.exp_loss, expected_exp_loss, "level {level}");
        assert_eq!(outcome.karma_loss, expected_karma_loss, "level {level}");
        assert_eq!(target.exp, 10_000 - expected_exp_loss, "level {level}");
        assert_eq!(target.karma, -expected_karma_loss, "level {level}");
    }
}

#[test]
fn apply_punishment_level_six_locks_regardless_of_paid_status() {
    let mut target = make_target(1, "Baddie", 10_000, 0, true);
    let outcome = apply_punishment(&mut target, 6);
    assert_eq!(target.karma, -12);
    assert!(outcome.lock);
    // Paid players never trigger `pkick`, matching C's `!(co->flags &
    // CF_PAID)` guard.
    assert!(!outcome.kick);
}

#[test]
fn apply_punishment_unpaid_kicks_at_karma_minus_five() {
    let mut target = make_target(1, "Baddie", 10_000, -1, false);
    let outcome = apply_punishment(&mut target, 4); // -6 karma -> -7
    assert_eq!(target.karma, -7);
    assert!(outcome.kick);
    assert!(!outcome.lock);
}

#[test]
fn apply_punishment_paid_never_kicks_even_at_low_karma() {
    let mut target = make_target(1, "Baddie", 10_000, -20, true);
    let outcome = apply_punishment(&mut target, 0);
    assert!(!outcome.kick);
    // Karma was already below -12 before this call - lock still reports
    // true since it only checks the resulting karma value.
    assert!(outcome.lock);
}

#[test]
fn apply_unpunishment_refunds_exp_and_karma_from_the_note() {
    let mut target = make_target(1, "Baddie", 9_600, -4, true);
    let note = PunishmentNote {
        level: 3,
        exp: 400,
        karma: 4,
        reason: "test".to_string(),
    };
    apply_unpunishment(&mut target, &note);
    assert_eq!(target.exp, 10_000);
    assert_eq!(target.karma, 0);
}

#[test]
fn queue_punish_command_rejects_invalid_name_shape() {
    let mut world = World::default();
    let messages = world.queue_punish_command(CharacterId(1), "a", 1, "being mean", false);
    assert_eq!(
        messages,
        vec!["Sorry, no player by the name a.".to_string()]
    );
    assert!(world.drain_pending_punish_requests().is_empty());
}

#[test]
fn queue_punish_command_rejects_short_reason() {
    let mut world = World::default();
    let messages = world.queue_punish_command(CharacterId(1), "Target", 1, "bad", false);
    assert_eq!(
        messages,
        vec!["Sorry, the reason bad is too short.".to_string()]
    );
}

#[test]
fn queue_punish_command_rejects_overflowed_reason() {
    let mut world = World::default();
    let messages = world.queue_punish_command(CharacterId(1), "Target", 1, "long enough", true);
    assert_eq!(messages, vec!["Sorry, the reason is too long.".to_string()]);
}

#[test]
fn queue_punish_command_rejects_out_of_bounds_level() {
    let mut world = World::default();
    let messages =
        world.queue_punish_command(CharacterId(1), "Target", 7, "being quite mean", false);
    assert_eq!(
        messages,
        vec!["Sorry, the level is out of bounds (0-6).".to_string()]
    );
}

#[test]
fn queue_punish_command_queues_a_valid_request() {
    let mut world = World::default();
    let messages =
        world.queue_punish_command(CharacterId(1), "Target", 3, "being quite mean", false);
    assert!(messages.is_empty());
    let requests = world.drain_pending_punish_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].target_name, "Target");
    assert_eq!(requests[0].level, 3);
    assert_eq!(requests[0].reason, "being quite mean");
}

#[test]
fn queue_unpunish_command_rejects_invalid_name_shape_immediately() {
    let mut world = World::default();
    let messages = world.queue_unpunish_command(CharacterId(1), "a", 42);
    assert_eq!(
        messages,
        vec!["Sorry, no player by the name a.".to_string()]
    );
    assert!(world.drain_pending_unpunish_requests().is_empty());
}

#[test]
fn queue_unpunish_command_queues_a_valid_request_with_no_immediate_message() {
    let mut world = World::default();
    let messages = world.queue_unpunish_command(CharacterId(1), "Target", 42);
    assert!(messages.is_empty());
    let requests = world.drain_pending_unpunish_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].target_name, "Target");
    assert_eq!(requests[0].note_id, 42);
}

#[test]
fn find_punish_target_online_matches_any_loaded_character_case_insensitively() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(2), make_target(2, "Baddie", 10_000, 0, true));
    assert_eq!(
        world.find_punish_target_online("baddie"),
        Some(CharacterId(2))
    );
    assert_eq!(world.find_punish_target_online("nobody"), None);
}
