use super::*;

fn world_with_character(id: u32) -> World {
    let mut world = World::default();
    world.characters.insert(CharacterId(id), character(id));
    world
}

#[test]
fn queue_look_command_empty_argument_reports_expected_name() {
    let mut world = world_with_character(1);
    world.queue_look_command(CharacterId(1), "");
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Expected a character name.");
    assert!(world.drain_pending_look_requests().is_empty());
}

#[test]
fn queue_look_command_invalid_shape_reports_not_found_immediately() {
    let mut world = world_with_character(1);
    world.queue_look_command(CharacterId(1), "a b");
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "No character by the name a b.");
    assert!(world.drain_pending_look_requests().is_empty());
}

#[test]
fn queue_look_command_valid_shape_queues_a_request() {
    let mut world = world_with_character(1);
    world.queue_look_command(CharacterId(1), "Someone");
    assert!(world.drain_pending_system_texts().is_empty());
    let requests = world.drain_pending_look_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].requester_id, CharacterId(1));
    assert_eq!(requests[0].target_name, "Someone");
    // Draining again returns nothing further.
    assert!(world.drain_pending_look_requests().is_empty());
}

#[test]
fn queue_klog_command_always_queues_with_no_validation() {
    let mut world = world_with_character(1);
    world.queue_klog_command(CharacterId(1));
    assert!(world.drain_pending_system_texts().is_empty());
    let requests = world.drain_pending_klog_requests();
    assert_eq!(requests, vec![CharacterId(1)]);
}
