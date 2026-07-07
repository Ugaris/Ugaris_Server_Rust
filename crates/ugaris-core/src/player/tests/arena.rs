use super::*;

#[test]
fn arena_score_seeds_newcomer_score_until_first_recorded_fight() {
    let player = PlayerRuntime::connected(1, 0);
    // C `!ppd->fights` re-seeds the score to -2000 (`arena.c:437-443`).
    assert_eq!(player.arena_score(), ARENA_PPD_NEWCOMER_SCORE);
    assert_eq!(player.arena_fights(), 0);
    assert_eq!(player.arena_wins(), 0);
    assert_eq!(player.arena_losses(), 0);
    assert_eq!(player.arena_lastfight(), 0);
}

#[test]
fn arena_fight_worth_matches_every_c_ladder_boundary() {
    // Representative values straddling each branch in `score_fight`
    // (`arena.c:451-524`), including the exact boundary values.
    let cases: &[(i32, i32)] = &[
        (10001, 0),
        (10000, 1),
        (8001, 1),
        (8000, 2),
        (1, 100),
        (0, 150),
        (-1, 150),
        (-100, 200),
        (-7999, 950),
        (-8000, 1000),
        (-8001, 1000),
        (i32::MIN, 1000),
    ];
    for (diff, expected) in cases {
        assert_eq!(
            PlayerRuntime::arena_fight_worth(*diff),
            *expected,
            "diff={diff}"
        );
    }
}

#[test]
fn record_arena_fight_result_seeds_both_newcomers_then_updates_fights_wins_losses() {
    let mut winner = PlayerRuntime::connected(1, 0);
    let mut loser = PlayerRuntime::connected(2, 0);

    PlayerRuntime::record_arena_fight_result(&mut winner, &mut loser, 1_000);

    // Both start at -2000 (first fight for each); diff = 0 => worth 150.
    assert_eq!(winner.arena_score(), ARENA_PPD_NEWCOMER_SCORE + 150);
    assert_eq!(loser.arena_score(), ARENA_PPD_NEWCOMER_SCORE - 150);
    assert_eq!(winner.arena_fights(), 1);
    assert_eq!(winner.arena_wins(), 1);
    assert_eq!(winner.arena_losses(), 0);
    assert_eq!(loser.arena_fights(), 1);
    assert_eq!(loser.arena_losses(), 1);
    assert_eq!(loser.arena_wins(), 0);
    assert_eq!(winner.arena_lastfight(), 1_000);
    assert_eq!(loser.arena_lastfight(), 1_000);
}

#[test]
fn record_arena_fight_result_accumulates_across_repeated_fights() {
    let mut winner = PlayerRuntime::connected(1, 0);
    let mut loser = PlayerRuntime::connected(2, 0);

    PlayerRuntime::record_arena_fight_result(&mut winner, &mut loser, 1_000);
    let score_after_first = winner.arena_score();
    PlayerRuntime::record_arena_fight_result(&mut winner, &mut loser, 2_000);

    assert_eq!(winner.arena_fights(), 2);
    assert_eq!(winner.arena_wins(), 2);
    assert_eq!(loser.arena_fights(), 2);
    assert_eq!(loser.arena_losses(), 2);
    // Second fight's diff is no longer 0 (winner pulled ahead), so it
    // must not reuse the newcomer seed again.
    assert!(winner.arena_score() > score_after_first);
    assert_eq!(winner.arena_lastfight(), 2_000);
    assert_eq!(loser.arena_lastfight(), 2_000);
}

#[test]
fn apply_arena_win_and_loss_match_record_arena_fight_result_when_called_separately() {
    // `apply_arena_master_events` cannot hold two simultaneous `&mut
    // PlayerRuntime` borrows from the same `ServerRuntime::players`
    // map (see `apply_arena_win`'s doc comment), so it calls the two
    // halves sequentially using the *other* side's pre-fight score
    // instead. This must produce exactly the same result as the
    // combined `record_arena_fight_result` call.
    let mut winner_a = PlayerRuntime::connected(1, 0);
    let mut loser_a = PlayerRuntime::connected(2, 0);
    PlayerRuntime::record_arena_fight_result(&mut winner_a, &mut loser_a, 42);

    let mut winner_b = PlayerRuntime::connected(1, 0);
    let mut loser_b = PlayerRuntime::connected(2, 0);
    let winner_score_before = winner_b.arena_score();
    let loser_score_before = loser_b.arena_score();
    let new_winner_score = winner_b.apply_arena_win(loser_score_before, 42);
    let new_loser_score = loser_b.apply_arena_loss(winner_score_before, 42);

    assert_eq!(winner_a.arena_score(), winner_b.arena_score());
    assert_eq!(loser_a.arena_score(), loser_b.arena_score());
    assert_eq!(new_winner_score, winner_b.arena_score());
    assert_eq!(new_loser_score, loser_b.arena_score());
    assert_eq!(winner_a.arena_fights(), winner_b.arena_fights());
    assert_eq!(winner_a.arena_wins(), winner_b.arena_wins());
    assert_eq!(loser_a.arena_losses(), loser_b.arena_losses());
}

#[test]
fn arena_ppd_blob_round_trips_through_encode_decode() {
    let mut winner = PlayerRuntime::connected(1, 0);
    let mut loser = PlayerRuntime::connected(2, 0);
    PlayerRuntime::record_arena_fight_result(&mut winner, &mut loser, 5_000);

    let encoded = winner.encode_legacy_ppd_blob(&[]);
    let mut round_tripped = PlayerRuntime::connected(1, 0);
    assert!(round_tripped.decode_legacy_ppd_blob(&encoded));
    assert_eq!(round_tripped.arena_ppd, winner.arena_ppd);
    assert_eq!(round_tripped.arena_score(), winner.arena_score());
    assert_eq!(round_tripped.arena_fights(), 1);
    assert_eq!(round_tripped.arena_wins(), 1);
    assert_eq!(round_tripped.arena_lastfight(), 5_000);
}

#[test]
fn clear_turn_seyan_ppd_clears_arena_ppd() {
    let mut winner = PlayerRuntime::connected(1, 0);
    let mut loser = PlayerRuntime::connected(2, 0);
    PlayerRuntime::record_arena_fight_result(&mut winner, &mut loser, 5_000);
    assert!(!winner.arena_ppd.is_empty());

    winner.clear_turn_seyan_ppd();
    assert!(winner.arena_ppd.is_empty());
    assert_eq!(winner.arena_score(), ARENA_PPD_NEWCOMER_SCORE);
}
