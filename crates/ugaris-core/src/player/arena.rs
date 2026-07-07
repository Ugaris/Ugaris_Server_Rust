use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_arena_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_ARENA_PPD_SIZE];
        let copy_len = self.arena_ppd.len().min(LEGACY_ARENA_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.arena_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_arena_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_ARENA_PPD_SIZE {
            return false;
        }
        self.arena_ppd = bytes[..LEGACY_ARENA_PPD_SIZE].to_vec();
        true
    }

    pub(crate) fn read_arena_i32(&self, offset: usize) -> i32 {
        if self.arena_ppd.len() < LEGACY_ARENA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arena_ppd, offset)
    }

    pub(crate) fn write_arena_i32(&mut self, offset: usize, value: i32) {
        if self.arena_ppd.len() < LEGACY_ARENA_PPD_SIZE {
            self.arena_ppd.resize(LEGACY_ARENA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arena_ppd, offset, value);
    }

    /// C `struct arena_ppd::score` (`arena.c:205`): the ELO-like arena
    /// tournament rating. A character with no recorded fights yet reads
    /// as the C `!ppd->fights` newcomer seed (`-2000`, `arena.c:437-443`)
    /// rather than the raw zeroed byte value, matching `score_fight` and
    /// `toplist_driver` both re-seeding on read whenever `fights == 0`.
    pub fn arena_score(&self) -> i32 {
        if self.arena_fights() == 0 {
            return ARENA_PPD_NEWCOMER_SCORE;
        }
        self.read_arena_i32(ARENA_PPD_SCORE_OFFSET)
    }

    pub fn arena_fights(&self) -> i32 {
        self.read_arena_i32(ARENA_PPD_FIGHTS_OFFSET)
    }

    pub fn arena_wins(&self) -> i32 {
        self.read_arena_i32(ARENA_PPD_WINS_OFFSET)
    }

    pub fn arena_losses(&self) -> i32 {
        self.read_arena_i32(ARENA_PPD_LOSSES_OFFSET)
    }

    pub fn arena_lastfight(&self) -> i32 {
        self.read_arena_i32(ARENA_PPD_LASTFIGHT_OFFSET)
    }

    /// C `score_fight`'s `diff -> worth` lookup ladder (`arena.c:451-524`),
    /// ported as a free function so it can be unit tested against every
    /// branch boundary without a `Player` instance. `diff` is the winner's
    /// score minus the loser's score *before* either is adjusted. Bigger
    /// favorite-beats-underdog `diff` yields a smaller `worth` (0 pts above
    /// 10000), bigger underdog-upset (negative `diff`) yields a bigger
    /// `worth` (capped at 1000 below -8000).
    pub fn arena_fight_worth(diff: i32) -> i32 {
        if diff > 10000 {
            0
        } else if diff > 8000 {
            1
        } else if diff > 6000 {
            2
        } else if diff > 5000 {
            3
        } else if diff > 4000 {
            4
        } else if diff > 3000 {
            5
        } else if diff > 2500 {
            6
        } else if diff > 2000 {
            7
        } else if diff > 1500 {
            8
        } else if diff > 1250 {
            9
        } else if diff > 1000 {
            10
        } else if diff > 800 {
            20
        } else if diff > 600 {
            30
        } else if diff > 500 {
            40
        } else if diff > 400 {
            50
        } else if diff > 300 {
            60
        } else if diff > 200 {
            70
        } else if diff > 100 {
            85
        } else if diff > 0 {
            100
        } else if diff > -100 {
            150
        } else if diff > -200 {
            200
        } else if diff > -300 {
            250
        } else if diff > -400 {
            300
        } else if diff > -500 {
            350
        } else if diff > -600 {
            400
        } else if diff > -800 {
            450
        } else if diff > -1000 {
            500
        } else if diff > -1250 {
            550
        } else if diff > -1500 {
            600
        } else if diff > -2000 {
            650
        } else if diff > -2500 {
            700
        } else if diff > -3000 {
            750
        } else if diff > -4000 {
            800
        } else if diff > -5000 {
            850
        } else if diff > -6000 {
            900
        } else if diff > -8000 {
            950
        } else {
            1000
        }
    }

    /// C `score_fight` (`arena.c:432-534`), minus the server-wide
    /// `update_toplist` call (a separate, not-yet-ported ranking-table
    /// persistence concern - see the "Arena rankings" `PORTING_TODO.md`
    /// entry). Records a single arena-tournament fight result on both
    /// combatants' `arena_ppd`: seeds either side's score to `-2000` on
    /// their very first fight, increments `fights`/`wins`/`losses`,
    /// applies the `arena_fight_worth` ladder to `diff = winner.score -
    /// loser.score` (before adjustment), and stamps `lastfight` on both.
    pub fn record_arena_fight_result(
        winner: &mut PlayerRuntime,
        loser: &mut PlayerRuntime,
        now: i32,
    ) {
        let winner_score = winner.arena_score();
        let loser_score = loser.arena_score();
        winner.apply_arena_win(loser_score, now);
        loser.apply_arena_loss(winner_score, now);
    }

    /// Winner-side half of `score_fight` (`arena.c:432-534`): reads this
    /// player's own pre-fight score internally (so callers only need the
    /// *opponent's* pre-fight score, avoiding the need for two
    /// simultaneous `&mut PlayerRuntime` borrows when the two combatants
    /// live in the same collection - see `crates/ugaris-server/src/
    /// world_events.rs::apply_arena_master_events`, the only real caller
    /// outside `record_arena_fight_result`'s own unit tests), applies the
    /// `arena_fight_worth` ladder, and stamps `lastfight`. Returns the
    /// resulting new score for [`crate::world::World::arena_update_toplist`].
    pub fn apply_arena_win(&mut self, loser_score_before: i32, now: i32) -> i32 {
        let winner_score_before = self.arena_score();
        let worth = Self::arena_fight_worth(winner_score_before - loser_score_before);
        let new_score = winner_score_before + worth;
        self.write_arena_i32(ARENA_PPD_SCORE_OFFSET, new_score);
        self.write_arena_i32(ARENA_PPD_FIGHTS_OFFSET, self.arena_fights() + 1);
        self.write_arena_i32(ARENA_PPD_WINS_OFFSET, self.arena_wins() + 1);
        self.write_arena_i32(ARENA_PPD_LASTFIGHT_OFFSET, now);
        new_score
    }

    /// Loser-side half of `score_fight` (`arena.c:432-534`) - see
    /// [`Self::apply_arena_win`]'s doc comment for why this takes the
    /// *winner's* pre-fight score rather than a second `&mut
    /// PlayerRuntime`.
    pub fn apply_arena_loss(&mut self, winner_score_before: i32, now: i32) -> i32 {
        let loser_score_before = self.arena_score();
        let worth = Self::arena_fight_worth(winner_score_before - loser_score_before);
        let new_score = loser_score_before - worth;
        self.write_arena_i32(ARENA_PPD_SCORE_OFFSET, new_score);
        self.write_arena_i32(ARENA_PPD_FIGHTS_OFFSET, self.arena_fights() + 1);
        self.write_arena_i32(ARENA_PPD_LOSSES_OFFSET, self.arena_losses() + 1);
        self.write_arena_i32(ARENA_PPD_LASTFIGHT_OFFSET, now);
        new_score
    }
}
