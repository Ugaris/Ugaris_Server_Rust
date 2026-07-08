use super::*;
use crate::character_driver::{CDR_FDEMON_BOSS, CDR_LOSTCON};

fn boss_npc(id: u32, x: u16, y: u16) -> Character {
    let mut boss = character(id);
    boss.driver = CDR_FDEMON_BOSS;
    boss.name = "Commander".into();
    boss.x = x;
    boss.y = y;
    // Deterministic vision in tests regardless of default (pitch-dark)
    // tile lighting - same convenience already established by
    // `fdemon.rs`'s own `fdemon_demon_npc` test helper.
    boss.flags |= CharacterFlags::INFRARED;
    boss
}

fn player_at(id: u32, x: u16, y: u16) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = "Hero".into();
    player.x = x;
    player.y = y;
    player
}

fn facts(boss_stage: i32) -> FdemonBossPlayerFacts {
    FdemonBossPlayerFacts {
        boss_stage,
        boss_counter: 0,
        boss_reported: 0,
    }
}

#[test]
fn sighted_players_excludes_self_lostcon_and_out_of_range() {
    let mut world = World::default();
    let boss = boss_npc(1, 100, 100);
    world.characters.insert(boss.id, boss);

    let mut near = player_at(2, 105, 100);
    world.characters.insert(near.id, near.clone());

    let mut far = player_at(3, 200, 100);
    world.characters.insert(far.id, far.clone());

    let mut lostcon = player_at(4, 101, 100);
    lostcon.driver = CDR_LOSTCON;
    world.characters.insert(lostcon.id, lostcon);

    let sighted = world.fdemon_boss_sighted_players(CharacterId(1));
    assert_eq!(sighted, vec![CharacterId(2)]);

    // Sanity: `near`/`far` positions actually differ enough to matter.
    near.x = 105;
    far.x = 200;
    let _ = (near, far);
}

#[test]
fn stage_zero_low_rank_stays_put_high_rank_advances() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let mut player = player_at(2, 0, 0);
    player.military_points = 0; // rank 0 < 2
    world.characters.insert(player.id, player);

    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(0), 8);
    assert_eq!(update.new_stage, None);
    assert!(update.timer_touched);
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("governer of Aston"));

    // Rank >= 2 (military_points high enough that cbrt >= 2, i.e. >= 8).
    world
        .characters
        .get_mut(&CharacterId(2))
        .unwrap()
        .military_points = 8;
    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(0), 8);
    assert_eq!(update.new_stage, Some(1));
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("Welcome, Hero"));
}

#[test]
fn stage_one_uses_color_marked_bytes_message() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(1), 8);
    assert_eq!(update.new_stage, Some(2));
    assert!(world.drain_pending_area_texts().is_empty());
    let bytes = world.drain_pending_area_text_bytes();
    assert_eq!(bytes.len(), 1);
    assert!(bytes[0].message.windows(4).any(|w| w == b"take"));
}

#[test]
fn stage_four_uses_rank_string_not_name() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(4), 8);
    assert_eq!(update.new_stage, Some(5));
    let texts = world.drain_pending_area_texts();
    assert!(texts[0].message.contains("Alright, nobody,"));
}

#[test]
fn waiting_stages_touch_nothing() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    for stage in [5, 8, 11, 14, 17, 20, 23, 26] {
        let update =
            world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(stage), 8);
        assert_eq!(
            update,
            FdemonBossStageUpdate {
                timer_touched: false,
                ..Default::default()
            }
        );
        assert!(world.drain_pending_area_texts().is_empty());
    }
}

#[test]
fn reward_stage_grants_exp_and_military_points_and_advances() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let mut player = player_at(2, 0, 0);
    player.level = 50;
    world.characters.insert(player.id, player);

    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(6), 8);
    assert_eq!(update.new_stage, Some(7));
    assert!(update.timer_touched);

    let after = &world.characters[&CharacterId(2)];
    assert!(after.exp > 0);
    assert_eq!(after.military_points, 2);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Well done, Hero")));
}

#[test]
fn reward_stage_announces_promotion_when_rank_increases() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let mut player = player_at(2, 0, 0);
    player.level = 60;
    // Just under rank 1 (cbrt(1) == 1 already, so start below any
    // threshold at 0 and grant enough points in one call to cross it).
    player.military_points = 0;
    world.characters.insert(player.id, player);

    world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(6), 8);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You've been promoted to")));
}

#[test]
fn stage_28_falls_through_to_29_logic_and_resets_counter() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(28), 8);
    assert_eq!(update.new_stage, Some(30));
    assert_eq!(update.new_counter, Some(0));
    let texts = world.drain_pending_area_texts();
    assert!(texts[0].message.contains("scout the whole underground"));
}

#[test]
fn stage_29_produces_same_message_without_resetting_counter() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(29), 8);
    assert_eq!(update.new_stage, Some(30));
    assert_eq!(update.new_counter, None);
}

#[test]
fn stage_30_no_new_stations_is_a_silent_noop() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut f = facts(30);
    f.boss_counter = 0b101;
    f.boss_reported = 0b101; // everything already reported -> cnt == 0
    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), f, 8);
    assert_eq!(
        update,
        FdemonBossStageUpdate {
            timer_touched: false,
            ..Default::default()
        }
    );
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn stage_30_reports_new_stations_and_grants_reward() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let mut player = player_at(2, 0, 0);
    player.level = 40;
    world.characters.insert(player.id, player);

    let mut f = facts(30);
    f.boss_counter = 0b111; // 3 stations found
    f.boss_reported = 0b001; // only 1 already reported -> cnt = 2
    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), f, 8);
    assert_eq!(update.new_stage, None); // cnt2 == 3 < 26
    assert_eq!(update.new_reported, Some(0b111));
    assert!(update.timer_touched);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("found 2 new Defense Stations")));
    assert!(texts.iter().any(|t| t.message.contains("found 3 stations")));
}

#[test]
fn stage_30_advances_to_31_once_26_stations_known() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut f = facts(30);
    f.boss_counter = (1 << 26) - 1; // 26 stations found
    f.boss_reported = (1 << 25) - 1; // 25 already reported -> cnt = 1
    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), f, 8);
    assert_eq!(update.new_stage, Some(31));
}

#[test]
fn stage_32_advances_and_stage_33_is_a_dead_end() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(32), 8);
    assert_eq!(update.new_stage, Some(33));

    let update = world.fdemon_boss_greet_player(CharacterId(1), CharacterId(2), facts(33), 8);
    assert_eq!(
        update,
        FdemonBossStageUpdate {
            timer_touched: false,
            ..Default::default()
        }
    );
}

#[test]
fn repeat_reset_ladder_matches_c_case_table() {
    for stage in 0..=5 {
        assert_eq!(fdemon_boss_repeat_reset(stage), Some((0, 0)));
    }
    for (pair, target) in [
        ((7, 8), 7),
        ((10, 11), 10),
        ((13, 14), 13),
        ((16, 17), 16),
        ((19, 20), 19),
        ((22, 23), 22),
        ((25, 26), 25),
        ((29, 30), 29),
        ((31, 32), 31),
    ] {
        assert_eq!(fdemon_boss_repeat_reset(pair.0), Some((target, 0)));
        assert_eq!(fdemon_boss_repeat_reset(pair.1), Some((target, 0)));
    }
    for noop in [6, 9, 12, 15, 18, 21, 24, 27, 28, 33, 100] {
        assert_eq!(fdemon_boss_repeat_reset(noop), None);
    }
}

#[test]
fn text_messages_detect_repeat_within_talk_range() {
    let mut world = World::default();
    let boss = boss_npc(1, 100, 100);
    world.characters.insert(boss.id, boss);
    let near = player_at(2, 105, 100); // dist 5 <= 12
    world.characters.insert(near.id, near);
    let far = player_at(3, 120, 100); // dist 20 > 12
    world.characters.insert(far.id, far);

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "repeat");
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(3), "repeat");

    let repeats = world.fdemon_boss_process_text_messages(CharacterId(1));
    assert_eq!(repeats, vec![CharacterId(2)]);
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .driver_messages
        .is_empty());
}

#[test]
fn text_messages_reply_to_ordinary_small_talk() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "hello");

    let repeats = world.fdemon_boss_process_text_messages(CharacterId(1));
    assert!(repeats.is_empty());
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("Hello, Hero!"));
}

#[test]
fn text_messages_answer_name_question_with_own_name() {
    let mut world = World::default();
    let boss = boss_npc(1, 0, 0);
    world.characters.insert(boss.id, boss);
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_text_message(CharacterId(2), "who are you");

    let repeats = world.fdemon_boss_process_text_messages(CharacterId(1));
    assert!(repeats.is_empty());
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("I'm Commander."));
}
