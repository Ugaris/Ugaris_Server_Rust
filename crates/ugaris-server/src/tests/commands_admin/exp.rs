use super::*;

#[test]
pub(crate) fn random_shrine_edge_blocks_without_marking_for_no_saves_or_noexp() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut no_saves = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);

    let result = apply_random_shrine_edge(&mut player, &mut no_saves, 31, 20);

    assert_eq!(result, RandomShrineEdgeApplyResult::AlreadyOnEdge);
    assert!(!player.has_used_random_shrine(31));

    let mut noexp = login_character(CharacterId(8), &login_block("Lisa"), 14, 10, 10);
    noexp.saves = 1;
    noexp.flags.insert(CharacterFlags::NOEXP);

    let result = apply_random_shrine_edge(&mut player, &mut noexp, 32, 20);

    assert_eq!(result, RandomShrineEdgeApplyResult::NoExp);
    assert_eq!(noexp.saves, 1);
    assert!(!player.has_used_random_shrine(32));
}

#[test]
pub(crate) fn random_shrine_vitality_blocks_noexp_and_capped_without_marking() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut noexp = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    noexp
        .flags
        .insert(CharacterFlags::WARRIOR | CharacterFlags::NOEXP);

    let result = apply_random_shrine_vitality(&mut player, &mut noexp, 50);

    assert_eq!(result, RandomShrineVitalityApplyResult::NoExp);
    assert!(!player.has_used_random_shrine(50));

    let mut capped = login_character(CharacterId(8), &login_block("Lisa"), 14, 10, 10);
    capped.values[1][CharacterValue::Mana as usize] = 115;

    let result = apply_random_shrine_vitality(&mut player, &mut capped, 50);

    assert_eq!(result, RandomShrineVitalityApplyResult::Capped);
    assert!(!player.has_used_random_shrine(50));
}

#[test]
pub(crate) fn god_setexpmod_updates_runtime_with_legacy_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setexpmod 2.5xyz", 1)
            .expect("god setexpmod should be recognized");

    assert_eq!(world.settings.exp_modifier, 2.5);
    assert_eq!(
        result.messages,
        vec!["Global experience modifier changed from 1.00 to 2.50"]
    );

    let invalid =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setexpmod 0.09", 1)
            .expect("god setexpmod should handle invalid values");
    assert_eq!(world.settings.exp_modifier, 2.5);
    assert_eq!(
        invalid.messages,
        vec!["Invalid value. Please specify a number between 0.1 and 1000.0"]
    );
}

#[test]
pub(crate) fn setexpmod_is_god_only_and_full_command_only() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setexpmod 2",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setexpmo 2",
        1,
    )
    .is_none());
}

#[test]
pub(crate) fn god_exp_command_reports_and_grants_self_or_named_target() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    god.exp = 100;
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.exp = 200;
    world.add_character(god);
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    let report = apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp", 1)
        .expect("god exp should be recognized");
    assert_eq!(report.messages, vec!["Godmode has 100 exp."]);

    let self_grant = apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp 25", 1)
        .expect("god exp self grant should be recognized");
    assert_eq!(self_grant.messages, vec!["Gave Godmode 25 exp."]);
    assert!(self_grant.inventory_changed);
    assert_eq!(world.characters.get(&god_id).unwrap().exp, 125);
    assert!(world
        .characters
        .get(&god_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::UPDATE));

    let target_grant =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Target 50", 1)
            .expect("god exp target grant should be recognized");
    assert_eq!(target_grant.messages, vec!["Gave Target 50 exp."]);
    assert_eq!(world.characters.get(&target_id).unwrap().exp, 250);

    let target_report =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Target", 1)
            .expect("god exp target report should be recognized");
    assert_eq!(target_report.messages, vec!["Target has 250 exp."]);
}

#[test]
pub(crate) fn god_exp_command_uses_runtime_exp_modifiers_and_legacy_gates() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let blocked_id = CharacterId(9);
    let capped_id = CharacterId(10);

    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);

    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.exp = 100;
    target.flags.insert(CharacterFlags::HARDCORE);
    world.add_character(target);

    let mut blocked = login_character(blocked_id, &login_block("Blocked"), 1, 12, 10);
    blocked.exp = 100;
    blocked.flags.insert(CharacterFlags::NOEXP);
    world.add_character(blocked);

    let mut capped = login_character(capped_id, &login_block("Capped"), 1, 13, 10);
    capped.level = 10;
    capped.exp = level2exp(10);
    capped.flags.insert(CharacterFlags::NOLEVEL);
    world.add_character(capped);

    let mut runtime = ServerRuntime::default();
    world.settings.exp_modifier = 2.0;
    world.settings.hardcore_exp_bonus = 1.5;

    apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Target 10", 1)
        .expect("god exp target grant should be recognized");
    let target = world.characters.get(&target_id).unwrap();
    assert_eq!(target.exp, 130);
    // C `give_exp` -> `check_levelup`: 130 exp crosses level2exp(3) == 81,
    // so the target levels up from 1 to 3 in the same call. Hardcore
    // characters reset `saves` to 0 on every level (already 0 here, so this
    // just confirms it stays 0 rather than incrementing).
    assert_eq!(target.level, 3);
    assert_eq!(target.saves, 0);

    apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Blocked 10", 1)
        .expect("god exp noexp target should be recognized");
    assert_eq!(world.characters.get(&blocked_id).unwrap().exp, 100);

    apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Capped 100000", 1)
        .expect("god exp nolevel target should be recognized");
    let capped = world.characters.get(&capped_id).unwrap();
    assert_eq!(capped.exp, level2exp(11) - 1);
    // C `give_exp`: `check_levelup` only runs `if (!(ch[cn].flags &
    // CF_NOLEVEL))`, so a NOLEVEL character never levels up even though its
    // capped exp is one shy of level2exp(11).
    assert_eq!(capped.level, 10);
}

#[test]
pub(crate) fn exp_command_is_god_only_uses_legacy_prefix_and_not_found_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/exp 10", 1)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let missing =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/exp Missing 10", 1)
            .expect("god exp missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/ex 10", 1)
            .is_none()
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/expx 10", 1)
            .is_none()
    );
}

#[test]
pub(crate) fn god_milexp_command_reports_and_grants_military_points() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    god.exp = 100;
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.exp = 200;
    target.flags.insert(CharacterFlags::HARDCORE);
    world.add_character(god);
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    let report = apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp", 1)
        .expect("god milexp should be recognized");
    assert_eq!(report.messages, vec!["Godmode has 100 exp."]);

    let self_grant =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp 25", 1)
            .expect("god milexp self grant should be recognized");
    assert_eq!(self_grant.messages, vec!["Gave Godmode 25 military exp."]);
    assert!(self_grant.inventory_changed);
    let god = world.characters.get(&god_id).unwrap();
    assert_eq!(god.exp, 101);
    assert_eq!(god.military_normal_exp, 1);
    assert_eq!(god.military_points, 25);
    assert!(god.flags.contains(CharacterFlags::UPDATE));

    let target_grant =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp Target 50", 1)
            .expect("god milexp target grant should be recognized");
    assert_eq!(target_grant.messages, vec!["Gave Target 50 military exp."]);
    let target = world.characters.get(&target_id).unwrap();
    assert_eq!(target.exp, 201);
    assert_eq!(target.military_normal_exp, 1);
    assert_eq!(target.military_points, 55);

    let target_report =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp Target", 1)
            .expect("god milexp target report should be recognized");
    assert_eq!(target_report.messages, vec!["Target has 201 exp."]);
}

#[test]
pub(crate) fn milexp_routes_its_fixed_one_exp_through_give_exp_and_honors_military_bonus() {
    // C `cmd_milexp` -> `give_military_pts_no_npc(co, val, 1)`
    // (`command.c:3048`, `tool.c:3281-3299`): the exp side is always a
    // fixed `1` through `give_exp` (so `exp_modifier`/`hardcore_exp_bonus`
    // apply), while `military_points` uses the typed amount multiplied by
    // the separately-tunable `hardcore_military_exp_bonus`.
    let mut world = World::default();
    world.settings.exp_modifier = 3.0;
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.exp = 0;
    target.flags.insert(CharacterFlags::HARDCORE);
    world.add_character(god);
    world.add_character(target);
    let mut runtime = ServerRuntime::default();
    world.settings.hardcore_military_exp_bonus = 2.0;

    apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp Target 50", 1)
        .expect("god milexp target grant should be recognized");

    let target = world.characters.get(&target_id).unwrap();
    // give_exp(co, 1) with exp_modifier 3.0 (no hardcore_exp_bonus set,
    // defaults to 1.0) -> +3, not the raw +1 a bare mutation would give.
    assert_eq!(target.exp, 3);
    assert_eq!(target.military_normal_exp, 1);
    // 50 * hardcore_military_exp_bonus(2.0) = 100, not the old hardcoded
    // 1.10 multiplier.
    assert_eq!(target.military_points, 100);
}

#[test]
pub(crate) fn milexp_command_is_god_only_full_command_and_not_found_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/milexp 10", 1)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let missing = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/milexp Missing 10",
        1,
    )
    .expect("god milexp missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/milex 10", 1)
            .is_none()
    );
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/milexpx 10",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn noexp_and_nolevel_toggle_legacy_flags_and_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    let noexp_on =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 1)
            .expect("noexp should be recognized");
    assert_eq!(noexp_on.messages, vec!["Turned NoExp mode on."]);
    assert!(noexp_on.inventory_changed);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));

    let noexp_off =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 1)
            .expect("noexp should toggle off");
    assert_eq!(noexp_off.messages, vec!["Turned NoExp mode off."]);
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));

    let nolevel_on =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/nolevel", 1)
            .expect("nolevel should be recognized");
    assert_eq!(
        nolevel_on.messages,
        vec!["NoLevel mode enabled. You will not level up until you disable this mode."]
    );
    assert!(nolevel_on.inventory_changed);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOLEVEL));

    let nolevel_off =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/nolevel", 1)
            .expect("nolevel should toggle off");
    assert_eq!(
        nolevel_off.messages,
        vec!["NoLevel mode disabled. You will now gain levels normally."]
    );
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOLEVEL));

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noex", 1,)
            .is_none()
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noleve", 1,)
            .is_none()
    );
}

#[test]
pub(crate) fn noexp_and_nolevel_cannot_be_enabled_in_gatekeeper_room() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 178, 196);
    character.x = 178;
    character.y = 196;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let noexp = apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 3)
        .expect("noexp should be recognized");
    assert_eq!(
        noexp.messages,
        vec!["Cannot turn NoExp mode on while in Gatekeeper room."]
    );
    assert!(!noexp.inventory_changed);
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));

    let nolevel =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/nolevel", 3)
            .expect("nolevel should be recognized");
    assert_eq!(
        nolevel.messages,
        vec!["Cannot turn NoLevel mode on while in Gatekeeper room."]
    );
    assert!(!nolevel.inventory_changed);
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOLEVEL));

    let character = world.characters.get_mut(&character_id).unwrap();
    character
        .flags
        .insert(CharacterFlags::NOEXP | CharacterFlags::NOLEVEL);

    let noexp_off =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 3)
            .expect("enabled noexp can be disabled in gatekeeper room");
    assert_eq!(noexp_off.messages, vec!["Turned NoExp mode off."]);
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));

    let nolevel_off =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/nolevel", 3)
            .expect("enabled nolevel can be disabled in gatekeeper room");
    assert_eq!(
        nolevel_off.messages,
        vec!["NoLevel mode disabled. You will now gain levels normally."]
    );
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOLEVEL));
}

#[test]
pub(crate) fn noexp_gatekeeper_room_guard_is_area_specific() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 178, 196);
    character.x = 178;
    character.y = 196;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 1)
        .expect("noexp outside area 3 should be recognized");
    assert_eq!(result.messages, vec!["Turned NoExp mode on."]);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));
}

#[test]
pub(crate) fn macro_ppd_default_matches_fresh_reset_expectations() {
    let ppd = MacroPpd::default();
    assert_eq!(ppd.karma, 0);
    assert_eq!(ppd.suspicion, 0);
    assert_eq!(ppd.history_count, 0);
    assert!(!ppd.force_summon);
    assert!(!ppd.in_challenge_room);
}
