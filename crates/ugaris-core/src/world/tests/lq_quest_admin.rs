use super::*;

fn god(world: &mut World, id: u32, x: u16, y: u16) -> CharacterId {
    let character_id = CharacterId(id);
    let mut spawned = character(id);
    spawned.flags = CharacterFlags::USED | CharacterFlags::GOD;
    spawned.x = x;
    spawned.y = y;
    world.characters.insert(character_id, spawned);
    character_id
}

fn plain_player(world: &mut World, id: u32) -> CharacterId {
    let character_id = CharacterId(id);
    world.characters.insert(character_id, character(id));
    character_id
}

fn plain_texts(world: &mut World) -> Vec<String> {
    world
        .drain_pending_system_texts()
        .into_iter()
        .map(|event| event.message)
        .collect()
}

// ---- lq_admin_wants_questend / lq_admin_wants_xinfo gates ----

#[test]
fn questend_gate_rejects_outside_area_20_or_35() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(!world.lq_admin_wants_questend(caller, 1, "#questend"));
}

#[test]
fn questend_gate_rejects_unauthorized_caller() {
    let mut world = World::default();
    let caller = plain_player(&mut world, 1);
    assert!(!world.lq_admin_wants_questend(caller, 20, "#questend"));
}

#[test]
fn questend_gate_accepts_slash_and_hash_prefix() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(world.lq_admin_wants_questend(caller, 20, "#questend"));
    assert!(world.lq_admin_wants_questend(caller, 35, "/questend"));
}

#[test]
fn questend_gate_does_not_match_other_quest_commands() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(!world.lq_admin_wants_questend(caller, 20, "#questentrance"));
    assert!(!world.lq_admin_wants_questend(caller, 20, "#questlevel 1 10"));
}

#[test]
fn xinfo_gate_rejects_outside_area_20_or_35() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(!world.lq_admin_wants_xinfo(caller, 1, "#xinfo"));
}

#[test]
fn xinfo_gate_rejects_unauthorized_caller() {
    let mut world = World::default();
    let caller = plain_player(&mut world, 1);
    assert!(!world.lq_admin_wants_xinfo(caller, 20, "#xinfo"));
}

#[test]
fn xinfo_gate_accepts_two_char_prefix() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(world.lq_admin_wants_xinfo(caller, 20, "#xi"));
}

// ---- apply_lq_questend_reward ----

#[test]
fn questend_reward_is_a_noop_when_sum_is_zero() {
    let mut world = World::default();
    let target = plain_player(&mut world, 1);
    assert!(!world.apply_lq_questend_reward(target, 0));
    assert_eq!(world.characters[&target].exp, 0);
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn questend_reward_grants_level_scaled_exp_and_feedback() {
    let mut world = World::default();
    let target = plain_player(&mut world, 1);
    // level 1: level_value(1) = 2^4 - 1^4 = 15; base = 15 / (1/10+1) = 15.
    // sum capped at 100 -> val = 15 / 100.0 * 100 = 15.
    assert!(world.apply_lq_questend_reward(target, 100));
    assert_eq!(world.characters[&target].exp, 15);
    assert_eq!(
        plain_texts(&mut world),
        vec!["You have been rewarded for your participation in this quest.".to_string()]
    );
}

#[test]
fn questend_reward_caps_sum_at_100() {
    let mut world = World::default();
    let low = plain_player(&mut world, 1);
    let high = plain_player(&mut world, 2);
    assert!(world.apply_lq_questend_reward(low, 100));
    let low_exp = world.characters[&low].exp;
    plain_texts(&mut world);
    assert!(world.apply_lq_questend_reward(high, 400));
    let high_exp = world.characters[&high].exp;
    assert_eq!(
        low_exp, high_exp,
        "sum > 100 should be clamped like sum == 100"
    );
}

#[test]
fn questend_reward_rewards_even_a_small_sum() {
    let mut world = World::default();
    let target = plain_player(&mut world, 1);
    // sum = 1 still passes C's truthy `if (sum)` gate, even if the
    // resulting exp grant rounds down to 0.
    assert!(world.apply_lq_questend_reward(target, 1));
}

// ---- report_lq_xinfo ----

#[test]
fn xinfo_reports_only_set_marks_in_order() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    let mut marks = [false; MAXLQMARK];
    marks[2] = true;
    marks[7] = true;
    world.report_lq_xinfo(caller, &marks);
    assert_eq!(
        plain_texts(&mut world),
        vec!["I have mark 2".to_string(), "I have mark 7".to_string()]
    );
}

#[test]
fn xinfo_reports_nothing_when_no_marks_are_set() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    let marks = [false; MAXLQMARK];
    world.report_lq_xinfo(caller, &marks);
    assert!(plain_texts(&mut world).is_empty());
}

#[test]
fn xinfo_ignores_mark_index_zero() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    let mut marks = [false; MAXLQMARK];
    marks[0] = true;
    world.report_lq_xinfo(caller, &marks);
    assert!(plain_texts(&mut world).is_empty());
}
