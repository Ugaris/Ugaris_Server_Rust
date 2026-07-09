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

fn error_text(world: &mut World) -> String {
    let mut bytes = world.drain_pending_system_text_bytes();
    assert_eq!(bytes.len(), 1, "expected exactly one queued error message");
    let message = bytes.remove(0).message;
    // Strip the `COL_LIGHT_RED` prefix for readable assertions.
    String::from_utf8_lossy(&message[crate::text::COL_LIGHT_RED.len()..]).into_owned()
}

fn plain_texts(world: &mut World) -> Vec<String> {
    world
        .drain_pending_system_texts()
        .into_iter()
        .map(|event| event.message)
        .collect()
}

/// Discards both feedback queues (used after setup calls whose exact
/// wording isn't under test).
fn drain_all(world: &mut World) {
    world.drain_pending_system_texts();
    world.drain_pending_system_text_bytes();
}

#[test]
fn outside_area_20_or_35_is_not_recognized() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(!world.apply_lq_admin_command(caller, 1, "#npc foo 10 a 60"));
    assert!(world.drain_pending_system_texts().is_empty());
    assert!(world.drain_pending_system_text_bytes().is_empty());
}

#[test]
fn area_35_mirror_is_also_recognized() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(world.apply_lq_admin_command(caller, 35, "#npc foo 10 a 60"));
}

#[test]
fn plain_speech_without_prefix_is_not_recognized() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(!world.apply_lq_admin_command(caller, 20, "hello there"));
}

#[test]
fn unauthorized_caller_is_not_recognized_even_for_a_valid_command() {
    let mut world = World::default();
    let caller = plain_player(&mut world, 1);
    assert!(!world.apply_lq_admin_command(caller, 20, "#npc foo 10 a 60"));
    assert!(world.lq_npcs.is_empty());
}

#[test]
fn lqmaster_flag_alone_is_sufficient() {
    let mut world = World::default();
    let character_id = CharacterId(1);
    let mut spawned = character(1);
    spawned.flags = CharacterFlags::USED | CharacterFlags::LQMASTER;
    world.characters.insert(character_id, spawned);
    assert!(world.apply_lq_admin_command(character_id, 20, "#npc foo 10 a 60"));
}

#[test]
fn npc_creates_a_template_at_the_callers_position() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 12, 34);

    assert!(world.apply_lq_admin_command(caller, 20, "#npc guard_base 10 a 60 gate guardian"));

    assert_eq!(world.lq_npcs.len(), 1);
    let npc = &world.lq_npcs[0];
    assert_eq!(npc.slot, 1);
    assert_eq!(npc.basename, "guard_base");
    assert_eq!(npc.level, 10);
    assert_eq!(npc.mode, b'a');
    assert_eq!(npc.respawn_seconds, 60);
    assert_eq!(npc.x, 12);
    assert_eq!(npc.y, 34);
    assert_eq!(npc.nick, ["gate".to_string(), "guardian".to_string()]);
    assert_eq!(plain_texts(&mut world), vec!["Added NPC 1".to_string()]);
}

#[test]
fn npc_lowercases_mode() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    assert!(world.apply_lq_admin_command(caller, 20, "#npc base 1 N 0"));
    assert_eq!(world.lq_npcs[0].mode, b'n');
}

#[test]
fn npc_rejects_position_collision_with_an_existing_template() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 first second"));
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npc base2 1 a 0"));
    assert_eq!(
        error_text(&mut world),
        " 1 first second is already at this position"
    );
    assert_eq!(world.lq_npcs.len(), 1);
}

#[test]
fn npc_missing_args_reports_usage() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    assert!(world.apply_lq_admin_command(caller, 20, "#npc"));
    assert!(error_text(&mut world).starts_with("Missing base. Usage is: /npc"));
}

#[test]
fn short_prefix_below_minlen_is_not_recognized() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    // C `cmdcmp(ptr, "npcname", 5)` needs at least 5 chars.
    assert!(!world.apply_lq_admin_command(caller, 20, "#npcn Foo Bar"));
}

#[test]
fn ambiguous_five_char_prefix_resolves_to_first_dispatch_table_entry() {
    // "npcre" is a valid minlen-5 prefix of npcreply, npcrewarditem, and
    // npcrespawn; C's if-chain checks npcreply first.
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    assert!(world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 nick"));
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcre nick 1 hi hello"));
    assert_eq!(world.lq_npcs[0].trigger[0], "hi");
    assert_eq!(world.lq_npcs[0].reply[0], "hello");
}

#[test]
fn npcname_updates_matching_nick_and_reports_not_found() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcname gate \"Gate Guard\""));
    assert_eq!(world.lq_npcs[0].name, "Gate Guard");
    assert_eq!(
        plain_texts(&mut world),
        vec!["Set name of 1 NPCs".to_string()]
    );

    assert!(world.apply_lq_admin_command(caller, 20, "#npcname missing Nope"));
    assert_eq!(error_text(&mut world), "NPC not found.");
}

#[test]
fn npcgold_rejects_more_than_2000() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcgold gate 2001"));
    assert_eq!(error_text(&mut world), "Too much gold.");
    assert_eq!(world.lq_npcs[0].carry_gold, 0);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcgold gate 1500"));
    assert_eq!(world.lq_npcs[0].carry_gold, 1500);
}

#[test]
fn npcsprite_blocks_islena_sprites() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcsprite gate 313"));
    assert_eq!(
        plain_texts(&mut world),
        vec!["Sorry, Islena is not available for Life Quests.".to_string()]
    );
    assert_eq!(world.lq_npcs[0].sprite, 0);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcsprite gate 42"));
    assert_eq!(world.lq_npcs[0].sprite, 42);
}

#[test]
fn npcreply_rejects_out_of_bounds_index_with_the_c_typo() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcreply gate 6 hi hello"));
    assert_eq!(
        plain_texts(&mut world),
        vec!["Nr 6 it out of bounds.".to_string()]
    );
}

#[test]
fn npcmodlevel_clamps_and_reports_via_all_keyword() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 5 a 0 gate");
    drain_all(&mut world);
    let caller2 = god(&mut world, 2, 2, 2);
    world.apply_lq_admin_command(caller2, 20, "#npc base2 195 a 0 gate2");
    drain_all(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcmodlevel all 10"));
    let messages = plain_texts(&mut world);
    assert!(messages
        .iter()
        .any(|m| m.contains("set to level 200 to avoid too high levels.")));
    assert!(messages.iter().any(|m| m == "Changed level of 2 NPCs"));
    assert_eq!(world.lq_npcs[1].level, 200);
    assert_eq!(world.lq_npcs[0].level, 15);
}

#[test]
fn npcmodlevel_clamps_negative_to_one() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 5 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcmodlevel gate -10"));
    let messages = plain_texts(&mut world);
    assert!(messages
        .iter()
        .any(|m| m.contains("set to level 1 to avoid negative level.")));
    assert_eq!(world.lq_npcs[0].level, 1);
}

#[test]
fn npcrespawn_supports_all_keyword() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 5 a 0 gate");
    drain_all(&mut world);
    let caller2 = god(&mut world, 2, 2, 2);
    world.apply_lq_admin_command(caller2, 20, "#npc base2 5 a 0 gate2");
    drain_all(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcrespawn all 120"));
    assert_eq!(world.lq_npcs[0].respawn_seconds, 120);
    assert_eq!(world.lq_npcs[1].respawn_seconds, 120);
    assert_eq!(
        plain_texts(&mut world),
        vec!["Changed respawn time of 2 NPCs to 120".to_string()]
    );
}

#[test]
fn npcpos_defaults_to_callers_position_and_rejects_out_of_bounds() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 7, 9);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcpos gate"));
    assert_eq!(world.lq_npcs[0].x, 7);
    assert_eq!(world.lq_npcs[0].y, 9);
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcpos gate 0 300"));
    assert_eq!(
        plain_texts(&mut world),
        vec!["Position 0,300 is out of bounds.".to_string()]
    );
}

#[test]
fn npcpos_rejects_ambiguous_nick_matches() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 dup samedup");
    drain_all(&mut world);
    let caller2 = god(&mut world, 2, 2, 2);
    world.apply_lq_admin_command(caller2, 20, "#npc base2 1 a 0 other dup");
    drain_all(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcpos dup 20 20"));
    assert_eq!(
        error_text(&mut world),
        "Cannot set the same position for multiple NPCs."
    );
}

#[test]
fn npcpos_rejects_position_already_used_by_another_npc() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 one");
    plain_texts(&mut world);
    let caller2 = god(&mut world, 2, 6, 6);
    world.apply_lq_admin_command(caller2, 20, "#npc base2 1 a 0 two");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcpos two 5 5"));
    assert_eq!(
        error_text(&mut world),
        " 1 one  is already at this position"
    );
}

#[test]
fn npcdelete_removes_template_and_live_instance() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    let live_id = CharacterId(50);
    world.characters.insert(live_id, character(50));
    assert!(world.apply_lq_npc_spawn_result(1, live_id, 50));

    assert!(world.apply_lq_admin_command(caller, 20, "#npcdelete gate"));
    assert!(world.lq_npcs.is_empty());
    assert!(!world.characters.contains_key(&live_id));
    assert_eq!(plain_texts(&mut world), vec!["Deleted 1 NPCs.".to_string()]);
}

#[test]
fn npclist_filters_by_nick_and_reports_summary() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);
    world.apply_lq_admin_command(caller, 20, "#npc base2 2 a 0 other");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npclist gate"));
    let messages = plain_texts(&mut world);
    assert_eq!(messages.len(), 2);
    assert!(messages[0].contains("base base,"));
    assert!(messages[1].starts_with("1 of "));
}

#[test]
fn npcshow_lists_populated_fields_only() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 3 a 0 gate");
    plain_texts(&mut world);
    world.apply_lq_admin_command(caller, 20, "#npcname gate \"Gate Guard\"");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcshow gate"));
    let messages = plain_texts(&mut world);
    assert!(messages.contains(&"Base: base".to_string()));
    assert!(messages.contains(&"Name: Gate Guard".to_string()));
    assert!(messages.contains(&"Showed 1 NPCs".to_string()));
    // Description/greeting were never set, so those lines are absent.
    assert!(!messages.iter().any(|m| m.starts_with("Desc:")));
    assert!(!messages.iter().any(|m| m.starts_with("Greeting:")));
}

#[test]
fn npcitem_and_npcrewarditem_store_the_parsed_spec() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(
        caller,
        20,
        "#npcitem gate sword 7 Excalibur \"A fine blade\""
    ));
    assert_eq!(world.lq_npcs[0].carry_item.base, "sword");
    assert_eq!(world.lq_npcs[0].carry_item.key_id, 7);
    assert_eq!(world.lq_npcs[0].carry_item.name, "Excalibur");
    assert_eq!(world.lq_npcs[0].carry_item.description, "A fine blade");
    assert_eq!(
        plain_texts(&mut world),
        vec!["Set item of 1 NPCs".to_string()]
    );

    assert!(world.apply_lq_admin_command(caller, 20, "#npcrewarditem gate shield 9"));
    assert_eq!(world.lq_npcs[0].reward_item.base, "shield");
    assert_eq!(world.lq_npcs[0].reward_item.key_id, 9);
    // C's own copy-paste bug: the reward-item success message also says
    // "Set item", not "Set reward item".
    assert_eq!(
        plain_texts(&mut world),
        vec!["Set item of 1 NPCs".to_string()]
    );
}

#[test]
fn npcwantitem_npckillmark_npchurtmark_store_marks() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcwantitem gate 12345"));
    assert_eq!(world.lq_npcs[0].want_key_id, 12345);
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npckillmark gate 3"));
    assert_eq!(world.lq_npcs[0].kill_mark_id, 3);
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npchurtmark gate 5"));
    assert_eq!(world.lq_npcs[0].hurt_mark_id, 5);
    assert_eq!(
        plain_texts(&mut world),
        vec!["Set hurtmark of 1 NPCs".to_string()]
    );
}

#[test]
fn mark_out_of_bounds_uses_plain_text_not_color() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npchurtmark gate 20"));
    assert_eq!(
        plain_texts(&mut world),
        vec!["Mark is out of bounds (1-9)".to_string()]
    );
}

#[test]
fn slash_prefix_works_the_same_as_hash_prefix() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    assert!(world.apply_lq_admin_command(caller, 20, "/npc base 1 a 0"));
    assert_eq!(world.lq_npcs.len(), 1);
}

#[test]
fn numeric_slot_lookup_targets_a_single_npc() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 1, 1);
    world.apply_lq_admin_command(caller, 20, "#npc base 1 a 0 gate");
    plain_texts(&mut world);

    assert!(world.apply_lq_admin_command(caller, 20, "#npcname 1 Renamed"));
    assert_eq!(world.lq_npcs[0].name, "Renamed");
}
