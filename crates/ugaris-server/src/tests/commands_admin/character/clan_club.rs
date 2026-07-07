use super::*;

#[test]
pub(crate) fn god_visibility_toggle_commands_preserve_legacy_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let xray = apply_admin_character_command(&mut world, &mut runtime, character_id, "/xray", 1)
        .expect("god xray command should be recognized");
    assert_eq!(xray.messages, vec!["Turned x-ray mode on."]);
    assert!(xray.inventory_changed);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::XRAY));

    let spy = apply_admin_character_command(&mut world, &mut runtime, character_id, "/spy", 1)
        .expect("god spy command should be recognized");
    assert_eq!(
        spy.messages,
        vec!["Turned spy mode on. You will now see all tells, clan, alliance, club, area, and mirror chat."]
    );
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::SPY));

    let spy_off = apply_admin_character_command(&mut world, &mut runtime, character_id, "/spy", 1)
        .expect("god spy command should toggle off");
    assert_eq!(
        spy_off.messages,
        vec!["Turned spy mode off. You will no longer see all tells, clan, alliance, club, area, and mirror chat."]
    );
}

#[test]
pub(crate) fn god_dlight_and_showattack_commands_mutate_runtime_without_feedback() {
    let mut world = World::default();
    world.date = GameDate::calculate(START_TIME + HOUR_LEN * 12, 23, None);
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let dlight =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/dlight 123abc", 1)
            .expect("god dlight command should be recognized");
    assert!(dlight.messages.is_empty());
    assert_eq!(runtime.dlight_override, 123);
    assert_eq!(world.date.daylight, 123);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/dlight 0", 23)
        .expect("god dlight zero command should clear the override");
    assert_eq!(runtime.dlight_override, 0);
    assert_eq!(
        world.date.daylight,
        GameDate::calculate(START_TIME + HOUR_LEN * 12, 23, None).daylight
    );

    let showattack =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/showat", 1)
            .expect("god showattack command should allow C minlen 6 abbreviation");
    assert!(showattack.messages.is_empty());
    assert!(runtime.show_attack);
    assert!(world.show_attack_debug);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/showattack", 1)
        .expect("god showattack command should toggle back off");
    assert!(!runtime.show_attack);
    assert!(!world.show_attack_debug);
}

#[test]
pub(crate) fn god_joinclan_and_joinclub_commands_mutate_identity_without_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let joined_clan =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/joinclan 3abc", 1)
            .expect("god joinclan command should be recognized");
    assert!(joined_clan.messages.is_empty());
    assert!(joined_clan.name_changed);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.clan, 3);
    assert_eq!(character.clan_rank, 4);
    assert_eq!(character.clan_serial, 0);

    let joined_club =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/joinclub 5", 1)
            .expect("god joinclub command should be recognized");
    assert!(joined_club.messages.is_empty());
    assert!(joined_club.name_changed);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.clan, 1029);
    assert_eq!(character.clan_rank, 2);
    assert_eq!(character.clan_serial, 0);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/joinclan 32", 1)
        .expect("out-of-range joinclan is still handled like C");
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.clan, 1029);
    assert_eq!(character.clan_rank, 2);
}

#[test]
pub(crate) fn joinclan_and_joinclub_require_exact_god_commands() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/joinclan 1",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/joincla 1", 1,)
            .is_none()
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/joinclu 1", 1,)
            .is_none()
    );
}

#[test]
pub(crate) fn god_killclan_command_deletes_an_existing_clan_immediately() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Doomed", 0).unwrap();
    assert!(world.clan_registry.exists(nr));

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/killclan {nr}"),
        1,
    )
    .expect("god killclan command should be recognized");
    assert!(result.messages.is_empty(), "C emits no feedback either");
    assert!(!world.clan_registry.exists(nr));
}

#[test]
pub(crate) fn killclan_requires_god_and_ignores_out_of_range_numbers() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/killclan 1",
        1
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let nr = world.clan_registry.found_clan("Safe", 0).unwrap();
    apply_admin_character_command(&mut world, &mut runtime, character_id, "/killclan 0", 1)
        .expect("still recognized, just a no-op for out-of-range numbers");
    assert!(world.clan_registry.exists(nr));
}

#[test]
pub(crate) fn staff_renclan_command_renames_an_existing_clan_in_aston() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Old Name", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclan {nr} New Name"),
        3,
    )
    .expect("staff renclan command should be recognized");
    assert_eq!(
        result.messages,
        vec![format!("Clan {nr} name changed to \"New Name\".")]
    );
    assert_eq!(world.clan_registry.name(nr), Some("New Name"));
}

#[test]
pub(crate) fn renclan_is_rejected_outside_aston_and_for_unknown_clans() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Old Name", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclan {nr} New Name"),
        1,
    )
    .expect("renclan should still be recognized outside Aston");
    assert_eq!(
        result.messages,
        vec!["Sorry, this command only works in Aston."]
    );
    assert_eq!(world.clan_registry.name(nr), Some("Old Name"));

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/renclan 9 Ghost Clan",
        3,
    )
    .expect("renclan should still be recognized for unknown clans");
    assert_eq!(result.messages, vec!["No clan by that number (9)."]);
}

#[test]
pub(crate) fn renclan_requires_staff_or_god() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/renclan 1 Name",
        3,
    )
    .is_none());
}

#[test]
pub(crate) fn god_killclub_command_bankrupts_an_existing_club_without_deleting_it() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.club_registry.create_club("Doomed", 0).unwrap();
    assert!(world.club_registry.exists(nr));

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/killclub {nr}"),
        1,
    )
    .expect("god killclub command should be recognized");
    assert!(result.messages.is_empty(), "C emits no feedback either");
    // C's `kill_club` doesn't clear the name - it zeroes `money`/sets
    // `paid = 1` so the next weekly `tick_billing` deletes it for
    // nonpayment, matching `killclan`'s own delayed-deletion trick.
    assert!(world.club_registry.exists(nr));
    assert_eq!(
        world.club_registry.tick_billing(3, 1),
        Some(ugaris_core::club::ClubBillingEvent::Deleted {
            club: nr,
            name: "Doomed".to_string(),
        })
    );
    assert!(!world.club_registry.exists(nr));
}

#[test]
pub(crate) fn killclub_requires_god_and_ignores_numbers_at_or_past_the_buggy_maxclan_cap() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/killclub 1",
        1
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    // C bug: bounds-checked against MAXCLAN (32), not MAXCLUB (16384), so
    // a club number of exactly 32 (a legal club number) is silently
    // ignored by this command.
    let nr = world.club_registry.create_club("Safe", 0).unwrap();
    apply_admin_character_command(&mut world, &mut runtime, character_id, "/killclub 32", 1)
        .expect("still recognized, just a no-op past the buggy cap");
    assert!(world.club_registry.exists(nr));
}

#[test]
pub(crate) fn god_setclanjewels_changes_jewels_and_reports_a_default_log_entry() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Jewelers", 0).unwrap();
    world.clan_registry.add_jewel(nr).unwrap();
    world.clan_registry.add_jewel(nr).unwrap();
    assert_eq!(world.clan_registry.jewel_count(nr), 2);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/setclanjewels {nr} 100"),
        1,
    )
    .expect("god setclanjewels command should be recognized");
    assert_eq!(
        result.messages,
        vec![format!("Clan {nr} (Jewelers) jewels changed from 2 to 100")]
    );
    assert_eq!(world.clan_registry.jewel_count(nr), 100);
    // `do_log` defaults to 1 (C `int do_log = 1;`), so a clan-log entry
    // must be queued for the call site to write.
    assert_eq!(
        result.clan_log_entry,
        Some((
            nr,
            world.clan_registry.serial(nr),
            1,
            "God Tester changed clan jewels from 2 to 100".to_string()
        ))
    );
}

#[test]
pub(crate) fn setclanjewels_do_log_zero_suppresses_the_clan_log_entry() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Jewelers", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/setclanjewels {nr} 50 0"),
        1,
    )
    .expect("god setclanjewels command should be recognized");
    assert_eq!(
        result.messages,
        vec![format!("Clan {nr} (Jewelers) jewels changed from 0 to 50")]
    );
    assert_eq!(world.clan_registry.jewel_count(nr), 50);
    assert_eq!(result.clan_log_entry, None);
}

#[test]
pub(crate) fn setclanjewels_requires_god_and_rejects_bad_args() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    // Non-god: not recognized at all.
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setclanjewels 1 100",
        1
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    let nr = world.clan_registry.found_clan("Jewelers", 0).unwrap();

    // Negative jewel count is rejected, matching C's `jewels >= 0` guard.
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/setclanjewels {nr} -5"),
        1,
    )
    .expect("still recognized, just reports the invalid-args message");
    assert_eq!(
        result.messages,
        vec!["Invalid clan number or jewel count".to_string()]
    );
    assert_eq!(world.clan_registry.jewel_count(nr), 0);

    // Out-of-range clan number.
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setclanjewels 0 100",
        1,
    )
    .expect("still recognized, just reports the invalid-args message");
    assert_eq!(
        result.messages,
        vec!["Invalid clan number or jewel count".to_string()]
    );

    // In-range but nonexistent clan number: also reports the same
    // invalid-args message (see `ClanRegistry::set_jewels`'s doc comment
    // for why this diverges from C's silent nameless-slot write).
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setclanjewels 31 100",
        1,
    )
    .expect("still recognized, just reports the invalid-args message");
    assert_eq!(
        result.messages,
        vec!["Invalid clan number or jewel count".to_string()]
    );
}

#[test]
pub(crate) fn staff_renclub_command_renames_an_existing_club_in_aston() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.club_registry.create_club("Old Name", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclub {nr} New Name"),
        3,
    )
    .expect("staff renclub command should be recognized");
    assert_eq!(
        result.messages,
        vec![format!("Club {nr} name changed to \"New Name\".")]
    );
    assert_eq!(world.club_registry.name(nr), Some("New Name"));
}

#[test]
pub(crate) fn renclub_is_rejected_outside_aston_and_for_illegal_names() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.club_registry.create_club("Old Name", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclub {nr} New Name"),
        1,
    )
    .expect("renclub should still be recognized outside Aston");
    assert_eq!(
        result.messages,
        vec!["Sorry, this command only works nearby a clubmaster."]
    );
    assert_eq!(world.club_registry.name(nr), Some("Old Name"));

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclub {nr} Illegal123"),
        3,
    )
    .expect("renclub should still be recognized for illegal names");
    assert_eq!(
        result.messages,
        vec!["That didn't work. The name is either taken or illegal."]
    );
    assert_eq!(world.club_registry.name(nr), Some("Old Name"));
}

#[test]
pub(crate) fn renclub_requires_staff_or_god() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/renclub 1 Name",
        3,
    )
    .is_none());
}
