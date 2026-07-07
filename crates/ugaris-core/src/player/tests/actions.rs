use super::*;

#[test]
fn command_queue_keeps_legacy_capacity() {
    let mut player = PlayerRuntime::connected(1, 0);
    for n in 0..20 {
        player.push_queued_action(QueuedAction {
            action: PlayerActionCode::Move,
            arg1: n,
            arg2: 0,
        });
    }
    assert_eq!(player.queue.len(), COMMAND_QUEUE_SIZE);
    assert_eq!(player.queue.front().unwrap().arg1, 4);
}

#[test]
fn fight_driver_suppressions_maps_every_no_toggle_and_leaves_nomana_nolife_nocombo_unused() {
    // C `fight_driver_attack_visible`'s player-side branch passes
    // exactly 8 of `ppd`'s toggles (`nobless`/`noheal`/`noflash`/
    // `nofireball`/`noball`/`noshield`/`nowarcry`/`nofreeze`/`nopulse`)
    // plus `nomove` from its own caller - `nolife`/`nomana`/`nocombo`
    // are consumed elsewhere (`lostcon_driver`'s own potion-drinking
    // block, not `fight_driver_attack_enemy`) and have no
    // `FightDriverSuppressions` field at all.
    let mut player = PlayerRuntime::connected(1, 0);
    player.no_move = true;
    player.no_bless = true;
    player.no_heal = true;
    player.no_flash = true;
    player.no_fireball = true;
    player.no_ball = true;
    player.no_shield = true;
    player.no_warcry = true;
    player.no_freeze = true;
    player.no_pulse = true;
    // Deliberately not mapped:
    player.no_life = true;
    player.no_mana = true;
    player.no_combo = true;

    let suppressions = player.fight_driver_suppressions();
    assert!(suppressions.nomove);
    assert!(suppressions.nobless);
    assert!(suppressions.noheal);
    assert!(suppressions.noflash);
    assert!(suppressions.nofireball);
    assert!(suppressions.noball);
    assert!(suppressions.noshield);
    assert!(suppressions.nowarcry);
    assert!(suppressions.nofreeze);
    assert!(suppressions.nopulse);

    let default_player = PlayerRuntime::connected(2, 0);
    assert_eq!(
        default_player.fight_driver_suppressions(),
        crate::world::FightDriverSuppressions::default()
    );
}

#[test]
fn driver_stop_clears_action_queue_and_fightback_state() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.driver_move(10, 11);
    player.driver_selfspell(PlayerActionCode::Bless);
    player.next_fightback_character = Some(CharacterId(2));
    player.next_fightback_serial = 44;
    player.next_fightback_tick = 55;

    player.driver_stop(99, true);

    assert_eq!(player.action.action, PlayerActionCode::Idle);
    assert!(player.queue.is_empty());
    assert_eq!(player.next_fightback_character, None);
    assert_eq!(player.next_fightback_serial, 0);
    assert_eq!(player.next_fightback_tick, 0);
    assert_eq!(player.nofight_timer, 99);
}

#[test]
fn driver_setters_match_c_action_payloads() {
    let mut player = PlayerRuntime::connected(1, 0);

    player.driver_take(7, 1234);
    assert_eq!(player.action.action, PlayerActionCode::Take);
    assert_eq!((player.action.arg1, player.action.arg2), (7, 1234));

    player.driver_kill(CharacterId(9), 4321);
    assert_eq!(player.action.action, PlayerActionCode::Kill);
    assert_eq!((player.action.arg1, player.action.arg2), (9, 4321));

    player.driver_drop(12, 13);
    assert_eq!(player.action.action, PlayerActionCode::Drop);
    assert_eq!((player.action.arg1, player.action.arg2), (12, 13));
}

#[test]
fn got_hit_fightback_immediately_kills_when_idle_and_nearby() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert!(player.apply_got_hit_fightback(CharacterId(2), 77, 2, TICKS_PER_SECOND * 3 + 1,));

    assert_eq!(player.action.action, PlayerActionCode::Kill);
    assert_eq!((player.action.arg1, player.action.arg2), (2, 77));
    assert_eq!(player.next_fightback_character, None);
}

#[test]
fn got_hit_fightback_defers_while_busy_and_promotes_when_idle() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.driver_move(20, 21);
    let hit_tick = TICKS_PER_SECOND * 4;

    assert!(player.apply_got_hit_fightback(CharacterId(3), 88, 2, hit_tick));
    assert_eq!(player.action.action, PlayerActionCode::Move);
    assert_eq!(player.next_fightback_character, Some(CharacterId(3)));
    assert_eq!(player.next_fightback_serial, 88);
    assert_eq!(player.next_fightback_tick, hit_tick);

    player.driver_halt();
    player.next_fightback_character = Some(CharacterId(3));
    player.next_fightback_serial = 88;
    player.next_fightback_tick = hit_tick;

    assert!(player.apply_deferred_fightback(hit_tick + TICKS_PER_SECOND - 1));
    assert_eq!(player.action.action, PlayerActionCode::Kill);
    assert_eq!((player.action.arg1, player.action.arg2), (3, 88));
}

#[test]
fn deferred_fightback_expires_after_one_second() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.next_fightback_character = Some(CharacterId(2));
    player.next_fightback_serial = 77;
    player.next_fightback_tick = TICKS_PER_SECOND * 4;

    assert!(!player.apply_deferred_fightback(TICKS_PER_SECOND * 5));
    assert_eq!(player.action.action, PlayerActionCode::Idle);
}

#[test]
fn driver_spell_queue_overwrites_last_slot_when_full() {
    let mut player = PlayerRuntime::connected(1, 0);
    for n in 0..COMMAND_QUEUE_SIZE {
        player.driver_mapspell(PlayerActionCode::Fireball, n as i32, 0);
    }

    player.driver_selfspell(PlayerActionCode::Bless);

    assert_eq!(player.queue.len(), COMMAND_QUEUE_SIZE);
    assert_eq!(player.queue.front().unwrap().arg1, 0);
    assert_eq!(player.queue.back().unwrap().action, PlayerActionCode::Bless);
}
