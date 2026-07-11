use super::*;
use crate::character_driver::CDR_STRATEGY_BOSS;
use crate::player::StrategyPpd;

fn boss_npc(id: u32, x: u16, y: u16) -> Character {
    let mut boss = character(id);
    boss.driver = CDR_STRATEGY_BOSS;
    boss.name = "Cinciac".into();
    boss.x = x;
    boss.y = y;
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

#[test]
fn sighted_players_excludes_out_of_range() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), boss_npc(1, 100, 100));

    let near = player_at(2, 105, 100);
    world.characters.insert(near.id, near);

    let far = player_at(3, 200, 100);
    world.characters.insert(far.id, far);

    let sighted = world.strategy_boss_sighted_players(CharacterId(1));
    assert_eq!(sighted, vec![CharacterId(2)]);
}

#[test]
fn stage_zero_low_rank_stays_at_one_high_rank_jumps_to_two() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let mut player = player_at(2, 0, 0);
    player.military_points = 0; // rank 0 < 8
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    world.strategy_boss_greet_player(CharacterId(1), CharacterId(2), &mut ppd, 1000);
    assert_eq!(ppd.boss_stage, 1);
    assert_eq!(ppd.boss_timer, 1000);
    assert_eq!(ppd.max_level, 60);
    assert_eq!(ppd.max_worker, 4);
    assert_eq!(ppd.trainspeed, 1);
    assert_eq!(ppd.eguardlvl, 50);
    assert_eq!(ppd.init_done, 1);
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("governer of Aston"));

    // Rank >= 8 (cbrt(military_points) >= 8, i.e. military_points >= 512).
    world
        .characters
        .get_mut(&CharacterId(2))
        .unwrap()
        .military_points = 512;
    let mut ppd = StrategyPpd::default();
    world.strategy_boss_greet_player(CharacterId(1), CharacterId(2), &mut ppd, 1000);
    assert_eq!(ppd.boss_stage, 2);
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("Welcome, Hero"));
    assert!(texts[0].message.contains("I am Cinciac"));
}

#[test]
fn stage_one_low_rank_does_not_touch_timer() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let mut player = player_at(2, 0, 0);
    player.military_points = 0;
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 1;
    ppd.init_done = 1;
    ppd.boss_timer = 42;
    world.strategy_boss_greet_player(CharacterId(1), CharacterId(2), &mut ppd, 1000);
    assert_eq!(ppd.boss_stage, 1);
    assert_eq!(ppd.boss_timer, 42);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn stage_one_high_rank_falls_through_straight_to_stage_three() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let mut player = player_at(2, 0, 0);
    player.military_points = 512;
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 1;
    ppd.init_done = 1;
    world.strategy_boss_greet_player(CharacterId(1), CharacterId(2), &mut ppd, 1000);
    assert_eq!(ppd.boss_stage, 3);
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("We've discovered these caves"));
}

#[test]
fn stage_five_uses_rank_string_not_player_name() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 5;
    ppd.init_done = 1;
    world.strategy_boss_greet_player(CharacterId(1), CharacterId(2), &mut ppd, 1000);
    assert_eq!(ppd.boss_stage, 6);
    let texts = world.drain_pending_area_texts();
    assert!(texts[0].message.contains("Your mission, nobody,"));
}

#[test]
fn stage_ten_only_advances_when_boss_exp_is_positive() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 10;
    ppd.init_done = 1;
    ppd.boss_exp = 0;
    world.strategy_boss_greet_player(CharacterId(1), CharacterId(2), &mut ppd, 1000);
    assert_eq!(ppd.boss_stage, 10);
    assert_eq!(ppd.boss_timer, 1000); // timer still touched even with no message
    assert!(world.drain_pending_area_texts().is_empty());

    ppd.boss_exp = 50;
    world.strategy_boss_greet_player(CharacterId(1), CharacterId(2), &mut ppd, 2000);
    assert_eq!(ppd.boss_stage, 11);
    assert_eq!(ppd.boss_msg_exp, 50);
    let texts = world.drain_pending_area_texts();
    assert!(texts[0].message.contains("military rank"));
    assert!(texts[0].message.contains("levels and experience"));
}

#[test]
fn stage_eleven_reverts_to_ten_when_more_exp_arrived() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 11;
    ppd.init_done = 1;
    ppd.boss_exp = 100;
    ppd.boss_msg_exp = 50;
    world.strategy_boss_greet_player(CharacterId(1), CharacterId(2), &mut ppd, 1000);
    assert_eq!(ppd.boss_stage, 10);
}

#[test]
fn throttle_suppresses_repeated_calls_within_five_seconds() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_timer = 100;
    ppd.init_done = 1;
    world.strategy_boss_greet_player(
        CharacterId(1),
        CharacterId(2),
        &mut ppd,
        100 + STRATEGY_BOSS_TIMER_THROTTLE_TICKS - 1,
    );
    assert_eq!(ppd.boss_stage, 0);
    assert!(world.drain_pending_area_texts().is_empty());

    world.strategy_boss_greet_player(
        CharacterId(1),
        CharacterId(2),
        &mut ppd,
        100 + STRATEGY_BOSS_TIMER_THROTTLE_TICKS + 1,
    );
    assert_eq!(ppd.boss_stage, 1);
}

#[test]
fn init_done_bzeroes_the_whole_ppd_on_first_encounter() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.exp = 999; // stray state from e.g. `#reset` before ever meeting Cinciac
    ppd.mis_cnt = 3;
    world.strategy_boss_greet_player(CharacterId(1), CharacterId(2), &mut ppd, 1000);
    assert_eq!(ppd.exp, 0);
    assert_eq!(ppd.mis_cnt, 0);
    assert_eq!(ppd.max_level, 60);
    assert_eq!(ppd.init_done, 1);
}

fn push_text_message(world: &mut World, boss_id: CharacterId, speaker_id: CharacterId, text: &str) {
    world
        .characters
        .get_mut(&boss_id)
        .unwrap()
        .push_driver_text_message(speaker_id, text);
}

#[test]
fn text_messages_classify_independently_matching_commands() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    push_text_message(
        &mut world,
        CharacterId(1),
        CharacterId(2),
        "please repeat that",
    );
    push_text_message(
        &mut world,
        CharacterId(1),
        CharacterId(2),
        "I choose military rank",
    );
    push_text_message(
        &mut world,
        CharacterId(1),
        CharacterId(2),
        "levels and experience please",
    );

    let outcome = world.strategy_boss_process_text_messages(CharacterId(1));
    assert_eq!(
        outcome,
        vec![
            (CharacterId(2), StrategyBossTextCommand::Repeat),
            (CharacterId(2), StrategyBossTextCommand::MilitaryRank),
            (CharacterId(2), StrategyBossTextCommand::LevelsAndExperience),
        ]
    );
}

#[test]
fn text_messages_ignore_out_of_range_speakers_and_self_loopback() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let far_player = player_at(2, 200, 200);
    world.characters.insert(far_player.id, far_player);

    push_text_message(&mut world, CharacterId(1), CharacterId(2), "repeat");
    push_text_message(&mut world, CharacterId(1), CharacterId(1), "repeat");

    let outcome = world.strategy_boss_process_text_messages(CharacterId(1));
    assert!(outcome.is_empty());
}

#[test]
fn repeat_resets_stage_and_timer_for_any_stage_zero_through_eleven() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 7;
    ppd.boss_timer = 500;
    world.strategy_boss_apply_text_command(
        CharacterId(1),
        CharacterId(2),
        &mut ppd,
        StrategyBossTextCommand::Repeat,
        23,
    );
    assert_eq!(ppd.boss_stage, 0);
    assert_eq!(ppd.boss_timer, 0);
}

#[test]
fn military_rank_choice_awards_points_and_resets_exp() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 11;
    ppd.boss_exp = 100;
    ppd.boss_msg_exp = 100;
    world.strategy_boss_apply_text_command(
        CharacterId(1),
        CharacterId(2),
        &mut ppd,
        StrategyBossTextCommand::MilitaryRank,
        23,
    );
    assert_eq!(ppd.boss_exp, 0);
    assert_eq!(ppd.boss_msg_exp, 0);
    assert_eq!(ppd.boss_stage, 10);
    assert_eq!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .military_points,
        100
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("So be it.")));
}

#[test]
fn military_rank_choice_is_a_no_op_without_boss_exp() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let player = player_at(2, 0, 0);
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 11;
    ppd.boss_exp = 0;
    world.strategy_boss_apply_text_command(
        CharacterId(1),
        CharacterId(2),
        &mut ppd,
        StrategyBossTextCommand::MilitaryRank,
        23,
    );
    assert_eq!(ppd.boss_stage, 11);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn levels_and_experience_choice_scales_by_rank_and_awards_exp() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), boss_npc(1, 0, 0));
    let mut player = player_at(2, 0, 0);
    player.military_points = 0; // rank 0
    let starting_exp = player.exp;
    world.characters.insert(player.id, player);

    let mut ppd = StrategyPpd::default();
    ppd.boss_stage = 11;
    ppd.boss_exp = 100;
    world.strategy_boss_apply_text_command(
        CharacterId(1),
        CharacterId(2),
        &mut ppd,
        StrategyBossTextCommand::LevelsAndExperience,
        23,
    );
    assert_eq!(ppd.boss_exp, 0);
    assert_eq!(ppd.boss_stage, 10);
    // pts = 100 / 5 + 1 = 21
    assert_eq!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .military_points,
        21
    );
    assert!(world.characters.get(&CharacterId(2)).unwrap().exp > starting_exp);
}
