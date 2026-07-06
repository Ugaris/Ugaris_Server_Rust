use super::*;
use ugaris_core::character_driver::{MacroDriverState, CDR_MACRO};

const AREA_ID: u16 = 1;

fn macro_npc(id: u32, x: usize, y: usize) -> Character {
    let mut daemon = login_character(CharacterId(id), &login_block("Macro Daemon"), AREA_ID, x, y);
    daemon.flags = CharacterFlags::USED | CharacterFlags::ALIVE;
    daemon.driver = CDR_MACRO;
    daemon
}

fn eligible_player(id: u32, x: usize, y: usize) -> Character {
    let mut player = login_character(CharacterId(id), &login_block("Victim"), AREA_ID, x, y);
    player.level = 20;
    player
}

fn connect_player(runtime: &mut ServerRuntime, session_id: u64, character_id: CharacterId) {
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(session_id, commands, 0);
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.character_id = Some(character_id);
    }
}

fn macro_state(
    world: &World,
    macro_id: CharacterId,
) -> ugaris_core::character_driver::MacroDriverData {
    match world
        .characters
        .get(&macro_id)
        .and_then(|character| character.driver_state.clone())
    {
        Some(CharacterDriverState::Macro(dat)) => dat,
        other => panic!("expected Macro driver state, got {other:?}"),
    }
}

#[test]
fn force_summon_finds_and_teleports_to_the_summoned_player() {
    let macro_id = CharacterId(1);
    let victim_id = CharacterId(2);
    let mut world = World::default();
    assert!(world.spawn_character(macro_npc(1, 10, 10), 10, 10));
    // Deliberately far away and otherwise ineligible (level below 9) - the
    // forced-summon path must find them anyway, skipping the normal
    // candidate filter entirely.
    let mut victim = eligible_player(2, 40, 40);
    victim.level = 1;
    assert!(world.spawn_character(victim, 40, 40));

    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, victim_id);
    runtime
        .player_for_character_mut(victim_id)
        .unwrap()
        .macro_ppd
        .force_summon = true;
    let mut loader = ZoneLoader::new();

    let applied = apply_macro_events(&mut world, &mut runtime, &mut loader, AREA_ID, false, 1_000);
    assert!(applied > 0);

    assert!(
        !runtime
            .player_for_character(victim_id)
            .unwrap()
            .macro_ppd
            .force_summon
    );
    let dat = macro_state(&world, macro_id);
    assert_eq!(dat.victim, Some(victim_id));
    assert_eq!(dat.state, MacroDriverState::Challenging);
    // The daemon actually teleported next to the victim.
    let daemon = &world.characters[&macro_id];
    let victim = &world.characters[&victim_id];
    assert!(daemon.x.abs_diff(victim.x) <= 1 && daemon.y.abs_diff(victim.y) <= 1);
}

#[test]
fn eligible_active_player_is_challenged_in_the_same_tick() {
    let macro_id = CharacterId(1);
    let victim_id = CharacterId(2);
    let mut world = World::default();
    assert!(world.spawn_character(macro_npc(1, 10, 10), 10, 10));
    assert!(world.spawn_character(eligible_player(2, 12, 12), 12, 12));

    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, victim_id);
    // Recently active, matching `macro_is_player_active`'s gate.
    runtime
        .player_for_character_mut(victim_id)
        .unwrap()
        .macro_ppd
        .last_combat = 1_000;
    let mut loader = ZoneLoader::new();

    let applied = apply_macro_events(&mut world, &mut runtime, &mut loader, AREA_ID, false, 1_000);
    assert!(applied > 0);

    let dat = macro_state(&world, macro_id);
    assert_eq!(dat.victim, Some(victim_id));
    assert_eq!(dat.state, MacroDriverState::Challenging);
    assert!(dat.challenge.is_some());
}

#[test]
fn afk_player_is_skipped_and_nextcheck_pushed_back() {
    let macro_id = CharacterId(1);
    let victim_id = CharacterId(2);
    let mut world = World::default();
    assert!(world.spawn_character(macro_npc(1, 10, 10), 10, 10));
    assert!(world.spawn_character(eligible_player(2, 12, 12), 12, 12));

    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, victim_id);
    // No recent activity at all (`MacroPpd::default()`), so
    // `macro_is_player_active` reports `false` for any realistic `now`.
    let mut loader = ZoneLoader::new();

    apply_macro_events(
        &mut world,
        &mut runtime,
        &mut loader,
        AREA_ID,
        false,
        1_700_000_000,
    );

    let dat = macro_state(&world, macro_id);
    assert_eq!(dat.victim, None);
    assert_eq!(dat.state, MacroDriverState::Idle);
    assert_eq!(
        runtime
            .player_for_character(victim_id)
            .unwrap()
            .macro_ppd
            .nextcheck,
        1_700_000_000 + 60 * 30
    );
}

#[test]
fn correct_answer_grants_a_reward_and_returns_to_idle() {
    let macro_id = CharacterId(1);
    let victim_id = CharacterId(2);
    let mut world = World::default();
    assert!(world.spawn_character(macro_npc(1, 10, 10), 10, 10));
    assert!(world.spawn_character(eligible_player(2, 12, 12), 12, 12));

    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, victim_id);
    runtime
        .player_for_character_mut(victim_id)
        .unwrap()
        .macro_ppd
        .last_combat = 1_000;
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"healing_potion1: name="Healing Potion" flag=IF_TAKE driver=64 ;
combo_potion1: name="Combo Potion" flag=IF_TAKE driver=64 ;
lollipop: name="Lollipop" flag=IF_TAKE driver=64 ;"#,
        )
        .unwrap();

    apply_macro_events(&mut world, &mut runtime, &mut loader, AREA_ID, false, 1_000);
    let dat = macro_state(&world, macro_id);
    assert_eq!(dat.state, MacroDriverState::Challenging);
    let challenge = dat.challenge.clone().expect("challenge asked");
    let answer = match challenge.challenge_type {
        ugaris_core::macro_daemon::MACRO_CHALLENGE_MATH => {
            (challenge.val1 + challenge.val2).to_string()
        }
        _ => challenge.expected_answer.clone(),
    };

    let before_karma = runtime
        .player_for_character(victim_id)
        .unwrap()
        .macro_ppd
        .karma;

    if let Some(daemon) = world.characters.get_mut(&macro_id) {
        daemon.push_driver_text_message(victim_id, answer);
    }
    apply_macro_events(&mut world, &mut runtime, &mut loader, AREA_ID, false, 1_100);

    let dat = macro_state(&world, macro_id);
    // Correct answer always resets to Idle (a new victim may immediately
    // be found in the same tick, which is fine - only the *previous*
    // victim's resolution is asserted here).
    assert_ne!(dat.state, MacroDriverState::Challenging);
    let player = runtime.player_for_character(victim_id).unwrap();
    assert!(player.macro_ppd.karma > before_karma);
    assert_eq!(player.macro_ppd.total_passed, 1);
}

#[test]
fn timeout_kicks_the_player_after_three_failures() {
    let macro_id = CharacterId(1);
    let victim_id = CharacterId(2);
    let mut world = World::default();
    assert!(world.spawn_character(macro_npc(1, 10, 10), 10, 10));
    assert!(world.spawn_character(eligible_player(2, 12, 12), 12, 12));

    let mut runtime = ServerRuntime::default();
    connect_player(&mut runtime, 1, victim_id);
    {
        let player = runtime.player_for_character_mut(victim_id).unwrap();
        player.macro_ppd.last_combat = 1_000;
        player.macro_ppd.challenge_failures = 2;
    }
    let mut loader = ZoneLoader::new();

    apply_macro_events(&mut world, &mut runtime, &mut loader, AREA_ID, false, 1_000);
    assert_eq!(
        macro_state(&world, macro_id).state,
        MacroDriverState::Challenging
    );

    // Advance past `MACRO_CHALLENGE_TIME` (180s) with no answer.
    world.tick.0 += TICKS_PER_SECOND * 200;
    apply_macro_events(&mut world, &mut runtime, &mut loader, AREA_ID, false, 1_300);

    let player = runtime.player_for_character(victim_id).unwrap();
    assert_eq!(player.macro_ppd.challenge_failures, 0);
    let victim = &world.characters[&victim_id];
    assert!(victim.flags.contains(CharacterFlags::KICKED));
    assert_eq!(macro_state(&world, macro_id).state, MacroDriverState::Idle);
}

#[test]
fn appearance_reskins_to_saint_nick_when_xmas_is_active() {
    let macro_id = CharacterId(1);
    let mut world = World::default();
    assert!(world.spawn_character(macro_npc(1, 10, 10), 10, 10));
    let mut runtime = ServerRuntime::default();
    let mut loader = ZoneLoader::new();

    apply_macro_events(&mut world, &mut runtime, &mut loader, AREA_ID, true, 1_000);

    let daemon = &world.characters[&macro_id];
    assert_eq!(daemon.name, "Saint Nick");
    assert_eq!(daemon.sprite, 13);
}
