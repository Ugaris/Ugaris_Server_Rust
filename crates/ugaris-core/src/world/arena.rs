//! `CDR_ARENAMASTER` arena tournament master NPC (`src/system/arena.c::
//! master_driver`).
//!
//! Ports the tournament pairing/fight state machine: `register`/`enter`/
//! `leave` text commands ([`World::arena_add_contender`]/
//! [`World::arena_handle_enter`]/[`World::arena_handle_leave`]), the
//! contender-pairing search ([`World::arena_find_contender`]), the
//! "both fighters stepped into the box" gate
//! ([`World::arena_check_inside`]), fight-completion detection
//! ([`World::arena_check_fight`]), the post-fight kick-out
//! ([`World::arena_empty_arena`]), and the server-wide top-100 ranking
//! table ([`World::arena_update_toplist`]/[`World::arena_toplist_entries`],
//! consumed by `IDR_TOPLIST`'s `arena_toplist_driver`/`arena_toplist_lines`
//! in `crates/ugaris-core/src/item_driver/arena.rs`).
//!
//! Like `world/bank.rs`/`world/clanmaster.rs`, actually mutating a
//! combatant's persistent `arena_ppd` (`PlayerRuntime`, owned by
//! `ugaris-server`) is outside `World`'s reach, so a completed fight is
//! queued as [`ArenaMasterEvent::FightScored`] for `ugaris-server`'s
//! `apply_arena_master_events` to resolve (via
//! [`crate::player::PlayerRuntime::apply_arena_win`]/`apply_arena_loss`)
//! and then feed the resulting scores back into
//! [`World::arena_update_toplist`]. Symmetrically, `add_contender` needs
//! the *registrant's* current arena score at registration time
//! (`ppd->score`, `arena.c:278`) - since that read also needs
//! `PlayerRuntime`, [`World::process_arena_master_actions`] takes an
//! `arena_score_of` callback (the same injection pattern already
//! established for RNG, e.g. `update_weather`'s `runtime_random_below`
//! closure parameter) rather than queuing an event for it, since the
//! pairing algorithm needs the score synchronously, not on a later tick.
//!
//! `ArenaContender::character_id` merges C's `ID`/`cn` pair into a single
//! `CharacterId` - see [`crate::character_driver::ArenaContender`]'s doc
//! comment for why that's a safe simplification here.
//!
//! Deliberately out of scope for this slice (documented in the "Arena
//! rankings" P3 task's REMAINING notes, not silently dropped):
//! - `CDR_ARENAFIGHTER` (`fighter_driver`, the autonomous tournament
//!   practice-bot) and `CDR_ARENAMANAGER` (`manager_driver`, the paid
//!   arena-rental system) - both separate NPC drivers with their own
//!   state machines, not ported this slice. The `NT_NPC`/`NTID_ARENA`
//!   notify messages `master_driver` sends to real players (as opposed to
//!   a future `fighter_driver` bot) are harmless no-ops in C too - only
//!   `fighter_driver` ever reads them (verified by grep: no other C file
//!   switches on `NTID_ARENA`), so this port's own unconsumed messages to
//!   human players match C exactly, not a gap.
//! - DB/storage-blob persistence for the ranking table (`struct toplist`)
//!   and this NPC's own tournament state (`storage_state`/
//!   `storage_version`/`storage_ID`/`lastsave`) - this codebase has no
//!   generic storage-blob primitive yet (same gap as
//!   `MilitaryMasterStorageRegistry`). The tournament tick therefore
//!   always runs as if C's `storage_state > 3` gate ("storage is ready")
//!   were already true - the eventual real behavior, just without the
//!   one-time load delay (same class of simplification as `/killclan`'s
//!   immediate-delete).
//! - The top-of-tick defensive `if (ch[cn].citem) { charlog("oops: ...");
//!   destroy_item(...); }` safety net (`arena.c:704-708`) is not ported:
//!   it is dead in normal play since the `NT_GIVE` handler below always
//!   clears `cursor_item` the same tick it is set.
use super::*;
use crate::character_driver::{
    analyse_text_qa, ArenaContender, TextAnalysisOutcome, ARENA_FIGHTER_MASTER_POS,
    ARENA_MAX_CONTENDER, ARENA_QA, CDR_ARENAFIGHTER, CDR_ARENAMASTER, FS_ENTER, FS_FIGHT,
    FS_LEISURE, FS_REGISTER, FS_START, FS_WAIT, FS_WAIT2, MS_FIGHT, MS_IN, MS_PAIR, NTID_ARENA,
};
use crate::drvlib::offset2dx;
use crate::player::{PlayerRuntime, ARENA_PPD_NEWCOMER_SCORE};

/// C `#define MAXCONTENDER 50`'s sibling ranking-table size, `struct
/// toplist { struct entry entry[100]; }` (`arena.c:232-234`).
pub const ARENA_TOPLIST_SIZE: usize = 100;

/// C `60*60*24*7` (`arena.c:415`): a toplist entry older than this
/// (seconds) is evicted the next time `update_toplist` scans past it.
const ARENA_TOPLIST_STALE_SECONDS: i64 = 60 * 60 * 24 * 7;

/// The arena box both fighters must step into (`arena.c:348-354,
/// 551-560`'s `x <= 233 || x >= 243 || y <= 132 || y >= 142` bounds,
/// i.e. the open interval `234..=242, 133..=141`).
const ARENA_BOX: (u16, u16, u16, u16) = (234, 242, 133, 141);

/// C `struct entry` (`arena.c:226-230`), one ranking-table slot. An empty
/// `name` is C's "unused slot" sentinel (`entry.name[0] == 0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaToplistRecord {
    pub name: String,
    pub score: i32,
    /// C `entry.updated` (`realtime` at last write), used only for the
    /// 7-day staleness eviction.
    pub updated: i64,
}

impl Default for ArenaToplistRecord {
    fn default() -> Self {
        Self {
            name: String::new(),
            score: 0,
            updated: 0,
        }
    }
}

/// A `master_driver` outcome that needs `ugaris-server`'s `PlayerRuntime`
/// map to finish - see the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaMasterEvent {
    /// C `check_fight`'s winning branch calling `score_fight(winner, loser,
    /// dat)` (`arena.c:571-580`).
    FightScored {
        winner_id: CharacterId,
        loser_id: CharacterId,
    },
}

fn arena_inside_box(x: u16, y: u16) -> bool {
    let (min_x, max_x, min_y, max_y) = ARENA_BOX;
    x > min_x && x < max_x && y > min_y && y < max_y
}

impl World {
    pub fn drain_pending_arena_master_events(&mut self) -> Vec<ArenaMasterEvent> {
        std::mem::take(&mut self.pending_arena_master_events)
    }

    /// C `update_toplist` (`arena.c:390-430`): dedups `cn_name`/`co_name`
    /// against the existing 100-slot table (clearing any *duplicate*
    /// occurrence past the first), evicts entries untouched for more than
    /// [`ARENA_TOPLIST_STALE_SECONDS`], inserts either name at the fixed
    /// slots `98`/`99` if not already present (matching C's unconditional
    /// overwrite - a real, if minor, C quirk: this can clobber whatever
    /// unrelated entry currently sits at index 98/99, which in steady
    /// state is one of the two lowest-ranked entries thanks to the sort
    /// below), then re-sorts descending by score with empty-name slots
    /// pushed to the bottom (`toplist_cmp`, `arena.c:375-388`).
    pub fn arena_update_toplist(
        &mut self,
        cn_name: &str,
        co_name: &str,
        cn_score: i32,
        co_score: i32,
        now_unix: i64,
    ) {
        if self.arena_toplist.len() < ARENA_TOPLIST_SIZE {
            self.arena_toplist
                .resize(ARENA_TOPLIST_SIZE, ArenaToplistRecord::default());
        }

        let mut found_cn = false;
        let mut found_co = false;
        for n in 0..ARENA_TOPLIST_SIZE {
            if self.arena_toplist[n].name == cn_name {
                if found_cn {
                    self.arena_toplist[n].name.clear();
                } else {
                    self.arena_toplist[n].score = cn_score;
                    self.arena_toplist[n].updated = now_unix;
                    found_cn = true;
                }
                continue;
            }
            if self.arena_toplist[n].name == co_name {
                if found_co {
                    self.arena_toplist[n].name.clear();
                } else {
                    self.arena_toplist[n].score = co_score;
                    self.arena_toplist[n].updated = now_unix;
                    found_co = true;
                }
                continue;
            }
            if !self.arena_toplist[n].name.is_empty()
                && now_unix - self.arena_toplist[n].updated > ARENA_TOPLIST_STALE_SECONDS
            {
                self.arena_toplist[n].name.clear();
            }
        }
        if !found_cn {
            self.arena_toplist[98] = ArenaToplistRecord {
                name: cn_name.to_string(),
                score: cn_score,
                updated: now_unix,
            };
        }
        if !found_co {
            self.arena_toplist[99] = ArenaToplistRecord {
                name: co_name.to_string(),
                score: co_score,
                updated: now_unix,
            };
        }
        self.arena_toplist
            .sort_by(|a, b| match (a.name.is_empty(), b.name.is_empty()) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                (false, false) => b.score.cmp(&a.score),
            });
    }

    /// [`crate::item_driver::ArenaToplistEntry`] view of
    /// [`World::arena_toplist`] for `arena_toplist_lines`.
    pub fn arena_toplist_entries(&self) -> Vec<crate::item_driver::ArenaToplistEntry> {
        self.arena_toplist
            .iter()
            .map(|record| crate::item_driver::ArenaToplistEntry {
                name: record.name.clone(),
                score: record.score,
            })
            .collect()
    }

    /// C `notify_char(co, NT_NPC, NTID_ARENA, dat2, dat3)`.
    fn arena_notify_char(&mut self, target_id: CharacterId, dat2: i32, dat3: i32) {
        if let Some(target) = self.characters.get_mut(&target_id) {
            target.push_driver_message(NT_NPC, NTID_ARENA, dat2, dat3);
        }
    }

    /// C `teleport_char_driver` (`src/system/drvlib.c:2651-2673`): a no-op
    /// when already within Manhattan distance `1` of the target, otherwise
    /// remove-and-redrop at the exact tile, falling back to the old
    /// position on failure (both halves already handled by
    /// [`World::teleport_character_exact`]).
    fn arena_teleport_char_driver(&mut self, character_id: CharacterId, x: u16, y: u16) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let dx = i32::from(character.x) - i32::from(x);
        let dy = i32::from(character.y) - i32::from(y);
        if dx.abs() + dy.abs() < 2 {
            return false;
        }
        self.teleport_character_exact(character_id, usize::from(x), usize::from(y))
    }

    /// C `add_contender` (`arena.c:257-287`), the `register` command's
    /// handler. `registrant_score` is the registrant's current arena
    /// rating (`ppd->score`, supplied by the caller's `arena_score_of`
    /// closure - see the module doc comment).
    fn arena_add_contender(
        &mut self,
        master_id: CharacterId,
        registrant_id: CharacterId,
        registrant_score: i32,
    ) {
        let Some(registrant_name) = self.characters.get(&registrant_id).map(|c| c.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::ArenaMaster(mut data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        if data
            .contenders
            .iter()
            .any(|c| c.character_id == registrant_id)
        {
            self.npc_say(
                master_id,
                &format!("You're already registered for this tournament, {registrant_name}."),
            );
            self.arena_notify_char(registrant_id, 3, 0);
            return;
        }
        if data.contenders.len() >= ARENA_MAX_CONTENDER {
            self.npc_say(
                master_id,
                &format!(
                    "I'm sorry, {registrant_name}, but there are no free slots at the moment. \
                     Please try again after the next fight."
                ),
            );
            return;
        }

        data.contenders.push(ArenaContender {
            character_id: registrant_id,
            score: registrant_score,
            reg_time: self.tick.0,
        });
        self.npc_say(
            master_id,
            &format!("Good luck, {registrant_name}. I will call you when your fight starts."),
        );
        self.arena_notify_char(registrant_id, 3, 0);

        if let Some(master) = self.characters.get_mut(&master_id) {
            master.driver_state = Some(CharacterDriverState::ArenaMaster(data));
        }
    }

    /// C `find_contender` (`arena.c:289-342`), the `MS_PAIR` tick: scans
    /// every contender pair for the minimum `abs(score1-score2)*100 -
    /// (waited1+waited2)` (favoring closely-matched scores and
    /// longer-waiting pairs), advances to `MS_IN` on a hit, and gives both
    /// fighters 30 (game) seconds to say "enter". Contenders whose
    /// character no longer exists are dropped first (merging C's
    /// `charID(cn) != ID` stale-slot invalidation and `!ch[cn].flags`
    /// vanished-character check into one existence test, per
    /// `ArenaContender`'s doc comment).
    fn arena_find_contender(&mut self, master_id: CharacterId) {
        let Some(CharacterDriverState::ArenaMaster(mut data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        data.contenders
            .retain(|c| self.characters.contains_key(&c.character_id));

        let tick = self.tick.0;
        let mut best_diff = i64::MAX;
        let mut pair: Option<(ArenaContender, ArenaContender)> = None;
        for i in 0..data.contenders.len() {
            for j in (i + 1)..data.contenders.len() {
                let a = data.contenders[i];
                let b = data.contenders[j];
                let mut diff = i64::from((a.score - b.score).abs()) * 100;
                diff -= (tick - a.reg_time) as i64 + (tick - b.reg_time) as i64;
                if diff < best_diff {
                    best_diff = diff;
                    pair = Some((a, b));
                }
            }
        }

        let Some((fight1, fight2)) = pair else {
            if let Some(master) = self.characters.get_mut(&master_id) {
                master.driver_state = Some(CharacterDriverState::ArenaMaster(data));
            }
            return;
        };

        data.state = MS_IN;
        data.fight1 = Some(fight1.character_id);
        data.fight2 = Some(fight2.character_id);
        data.timeout = tick + TICKS_PER_SECOND * 30;

        let name1 = self
            .characters
            .get(&fight1.character_id)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let name2 = self
            .characters
            .get(&fight2.character_id)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        self.npc_say(
            master_id,
            &format!(
                "Next fight is: {name1} versus {name2}. Both participants please step forward \
                 and say: 'enter'. You have 30 seconds to enter the arena, otherwise you lose by \
                 default."
            ),
        );
        self.arena_notify_char(fight1.character_id, 0, 0);
        self.arena_notify_char(fight2.character_id, 0, 0);

        if let Some(master) = self.characters.get_mut(&master_id) {
            master.driver_state = Some(CharacterDriverState::ArenaMaster(data));
        }
    }

    /// C `check_inside` (`arena.c:344-373`), the `MS_IN` tick: while still
    /// within the 30-second timeout, tells both fighters to attack each
    /// other (`NT_NPC` dat2=1) only once *both* have stepped into the
    /// arena box; either way (in time or not), always clears the pair's
    /// leftover contender slots and unconditionally advances to
    /// `MS_FIGHT` with a fresh 2-minute timer - a fighter who never
    /// entered is judged (and loses by default) on the very next
    /// `check_fight` tick, matching C exactly.
    fn arena_check_inside(&mut self, master_id: CharacterId) {
        let Some(CharacterDriverState::ArenaMaster(mut data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };
        let (Some(fight1), Some(fight2)) = (data.fight1, data.fight2) else {
            return;
        };

        if self.tick.0 < data.timeout {
            let inside = |id: CharacterId| {
                self.characters
                    .get(&id)
                    .is_some_and(|c| arena_inside_box(c.x, c.y))
            };
            if !inside(fight1) || !inside(fight2) {
                return;
            }
            self.arena_notify_char(fight1, 1, fight2.0 as i32);
            self.arena_notify_char(fight2, 1, fight1.0 as i32);
        }

        data.contenders
            .retain(|c| c.character_id != fight1 && c.character_id != fight2);

        self.npc_say(
            master_id,
            "Let the fight begin! You have two minutes to kill your opponent.",
        );
        data.state = MS_FIGHT;
        data.timeout = self.tick.0 + TICKS_PER_SECOND * 60 * 2;

        if let Some(master) = self.characters.get_mut(&master_id) {
            master.driver_state = Some(CharacterDriverState::ArenaMaster(data));
        }
    }

    /// C `check_fight` (`arena.c:548-598`), the `MS_FIGHT` tick: a fighter
    /// who has left the arena box (or vanished entirely - not a distinct
    /// C case, but the closest equivalent to a stale/missing slot) loses;
    /// a timeout with both still inside is a draw (`"Hu? No one won?"`).
    /// A real winner queues [`ArenaMasterEvent::FightScored`] only if
    /// *both* combatants' characters still exist (mirroring C's
    /// `charID(fight1_cn)==fight1_ID && charID(fight2_cn)==fight2_ID`
    /// scoring guard). Always ends by kicking everyone still in the box
    /// out ([`World::arena_empty_arena`]) and returning to `MS_PAIR`.
    fn arena_check_fight(&mut self, master_id: CharacterId) {
        let Some(CharacterDriverState::ArenaMaster(mut data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };
        let (Some(fight1), Some(fight2)) = (data.fight1, data.fight2) else {
            return;
        };

        let mut end = 0u8;
        let mut win1 = true;
        let mut win2 = true;
        match self.characters.get(&fight1) {
            Some(c) if arena_inside_box(c.x, c.y) => {}
            _ => {
                end += 1;
                win1 = false;
            }
        }
        match self.characters.get(&fight2) {
            Some(c) if arena_inside_box(c.x, c.y) => {}
            _ => {
                end += 1;
                win2 = false;
            }
        }
        if self.tick.0 > data.timeout {
            win1 = false;
            win2 = false;
            end = 1;
        }
        if end == 0 {
            return;
        }

        let name1 = self
            .characters
            .get(&fight1)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let name2 = self
            .characters
            .get(&fight2)
            .map(|c| c.name.clone())
            .unwrap_or_default();

        if win1 {
            self.npc_say(master_id, &format!("And the winner is {name1}."));
            if self.characters.contains_key(&fight1) && self.characters.contains_key(&fight2) {
                self.pending_arena_master_events
                    .push(ArenaMasterEvent::FightScored {
                        winner_id: fight1,
                        loser_id: fight2,
                    });
            }
        } else if win2 {
            self.npc_say(master_id, &format!("And the winner is {name2}."));
            if self.characters.contains_key(&fight1) && self.characters.contains_key(&fight2) {
                self.pending_arena_master_events
                    .push(ArenaMasterEvent::FightScored {
                        winner_id: fight2,
                        loser_id: fight1,
                    });
            }
        } else {
            self.npc_say(master_id, "Hu? No one won? Oh well...");
        }

        if self
            .characters
            .get(&fight1)
            .is_some_and(|c| !c.flags.is_empty())
        {
            self.arena_notify_char(fight1, 2, 0);
        }
        if self
            .characters
            .get(&fight2)
            .is_some_and(|c| !c.flags.is_empty())
        {
            self.arena_notify_char(fight2, 2, 0);
        }

        self.arena_empty_arena(master_id);
        data.state = MS_PAIR;

        if let Some(master) = self.characters.get_mut(&master_id) {
            master.driver_state = Some(CharacterDriverState::ArenaMaster(data));
        }
    }

    /// C `empty_arena` (`arena.c:536-546`): teleports everyone still
    /// standing in the arena box to the master NPC's own tile.
    fn arena_empty_arena(&mut self, master_id: CharacterId) {
        let Some(master) = self.characters.get(&master_id) else {
            return;
        };
        let (master_x, master_y) = (master.x, master.y);
        let (min_x, max_x, min_y, max_y) = ARENA_BOX;

        let mut occupants = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if let Some(tile) = self.map.tile(usize::from(x), usize::from(y)) {
                    if tile.character != 0 {
                        occupants.push(CharacterId(u32::from(tile.character)));
                    }
                }
            }
        }
        for occupant_id in occupants {
            self.arena_teleport_char_driver(occupant_id, master_x, master_y);
        }
    }

    /// C `master_driver`'s `NT_TEXT` branch's shared `analyse_text_driver`
    /// call (`arena.c:630`): non-player/playerlike speakers and speakers
    /// the master can't see never match any `ARENA_QA` entry, matching
    /// C's own guard clauses ahead of tokenization.
    fn arena_text_outcome(
        &self,
        master_id: CharacterId,
        speaker_id: CharacterId,
        text: &str,
    ) -> TextAnalysisOutcome {
        let Some(master) = self.characters.get(&master_id) else {
            return TextAnalysisOutcome::NoMatch;
        };
        let Some(speaker) = self.characters.get(&speaker_id) else {
            return TextAnalysisOutcome::NoMatch;
        };
        if !speaker
            .flags
            .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        {
            return TextAnalysisOutcome::NoMatch;
        }
        if !char_see_char(master, speaker, &self.map, self.date.daylight) {
            return TextAnalysisOutcome::NoMatch;
        }
        analyse_text_qa(text, &master.name, &speaker.name, ARENA_QA)
    }

    /// C `master_driver`'s `enter` handler (`arena.c:643-663`).
    fn arena_handle_enter(&mut self, master_id: CharacterId, speaker_id: CharacterId) {
        let Some(speaker_name) = self.characters.get(&speaker_id).map(|c| c.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::ArenaMaster(data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        if data.state != MS_IN {
            self.npc_say(
                master_id,
                &format!("No fight has been scheduled, {speaker_name}."),
            );
            self.arena_notify_char(speaker_id, 5, 0);
            return;
        }
        if data.fight1 == Some(speaker_id) {
            self.arena_teleport_char_driver(speaker_id, 235, 140);
            self.arena_notify_char(speaker_id, 4, 0);
            if let Some(speaker) = self.characters.get_mut(&speaker_id) {
                speaker.flags.remove(CharacterFlags::LAG);
            }
        } else if data.fight2 == Some(speaker_id) {
            self.arena_teleport_char_driver(speaker_id, 241, 134);
            self.arena_notify_char(speaker_id, 4, 0);
            if let Some(speaker) = self.characters.get_mut(&speaker_id) {
                speaker.flags.remove(CharacterFlags::LAG);
            }
        } else {
            self.npc_say(
                master_id,
                &format!("You are not invited to this fight, {speaker_name}."),
            );
            self.arena_notify_char(speaker_id, 5, 0);
        }
    }

    /// C `master_driver`'s `leave` handler (`arena.c:664-666`).
    fn arena_handle_leave(&mut self, speaker_id: CharacterId) {
        self.arena_teleport_char_driver(speaker_id, 238, 146);
    }

    /// C `master_driver`'s `NT_TEXT` branch (`arena.c:627-672`). Returns
    /// C's `didsay` (truthy for *any* `ARENA_QA` hit, including the dead
    /// `2`/`6` codes - see `ARENA_QA`'s doc comment).
    fn arena_handle_text_message(
        &mut self,
        master_id: CharacterId,
        speaker_id: CharacterId,
        text: &str,
        arena_score_of: &mut dyn FnMut(CharacterId) -> i32,
    ) -> bool {
        if speaker_id == master_id {
            return false;
        }
        let outcome = self.arena_text_outcome(master_id, speaker_id, text);
        let didsay = !matches!(outcome, TextAnalysisOutcome::NoMatch);
        match outcome {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(master_id, &reply);
            }
            TextAnalysisOutcome::Matched(3) => {
                let score = arena_score_of(speaker_id);
                self.arena_add_contender(master_id, speaker_id, score);
            }
            TextAnalysisOutcome::Matched(4) => self.arena_handle_enter(master_id, speaker_id),
            TextAnalysisOutcome::Matched(5) => self.arena_handle_leave(speaker_id),
            TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
        }
        didsay
    }

    /// C `master_driver`'s `NT_GIVE` branch (`arena.c:675-694`): every
    /// gift gets the "Thou hast better use for this" message exactly once
    /// per tick's message batch (`dat->amgivingback` is reset to `0` at
    /// the end of every tick, `arena.c:702`), then is destroyed - C's
    /// `give_driver(cn, co)` give-it-back-to-the-sender fallback isn't
    /// ported (no generic "give item back" driver helper exists yet, same
    /// simplification `world/bank.rs`/`world/merchant.rs`/`world/
    /// clanmaster.rs` already established).
    ///
    /// Reads/writes `amgivingback` via a direct `driver_state` field
    /// mutation (rather than threading a `&mut ArenaMasterDriverData`
    /// captured earlier in the tick) so this can't clobber a concurrent
    /// `driver_state` update from a *different* message in the same
    /// batch (e.g. a `register` earlier in the loop) with a stale
    /// snapshot - see [`World::process_arena_master_messages`]'s own doc
    /// comment for why that matters.
    fn arena_handle_give_message(&mut self, master_id: CharacterId) {
        let Some(item_id) = self
            .characters
            .get_mut(&master_id)
            .and_then(|master| master.cursor_item.take())
        else {
            return;
        };
        let amgivingback = match self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::ArenaMaster(data)) => data.amgivingback,
            _ => 0,
        };
        if amgivingback == 0 {
            self.npc_say(
                master_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
        }
        if let Some(CharacterDriverState::ArenaMaster(data)) = self
            .characters
            .get_mut(&master_id)
            .and_then(|c| c.driver_state.as_mut())
        {
            data.amgivingback += 1;
        }
        self.destroy_item(item_id);
    }

    /// C `master_driver`'s message loop (`arena.c:613-697`). Returns the
    /// last speaker that triggered a `didsay` hit this tick (C's
    /// `talkdir`/`offset2dx` target), if any.
    ///
    /// Deliberately does *not* hold a single `ArenaMasterDriverData`
    /// snapshot across the whole loop and write it back at the end: text/
    /// give handlers below (`arena_add_contender`/`arena_find_contender`-
    /// adjacent state changes don't happen here, but `arena_handle_enter`/
    /// `arena_handle_give_message` do read-modify-write `driver_state`
    /// mid-loop) each fetch and persist their own fresh copy, so a stale
    /// outer snapshot captured before the loop started must never be
    /// written back afterward - doing so would silently revert whatever a
    /// message earlier in the same batch just committed. `last_talk`/
    /// `amgivingback` are therefore applied via a direct field mutation
    /// on the *current* `driver_state` after the loop, not threaded
    /// through it.
    fn process_arena_master_messages(
        &mut self,
        master_id: CharacterId,
        arena_score_of: &mut dyn FnMut(CharacterId) -> i32,
    ) -> Option<CharacterId> {
        let messages = self
            .characters
            .get_mut(&master_id)
            .map(|master| std::mem::take(&mut master.driver_messages))
            .unwrap_or_default();

        let mut talk_target = None;
        let mut last_talk_tick = None;
        for message in &messages {
            match message.message_type {
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    let Some(text) = message.text.as_deref() else {
                        continue;
                    };
                    if self.arena_handle_text_message(master_id, speaker_id, text, arena_score_of) {
                        last_talk_tick = Some(self.tick.0);
                        talk_target = Some(speaker_id);
                    }
                }
                NT_GIVE => self.arena_handle_give_message(master_id),
                _ => {}
            }
        }

        if let Some(CharacterDriverState::ArenaMaster(data)) = self
            .characters
            .get_mut(&master_id)
            .and_then(|c| c.driver_state.as_mut())
        {
            if let Some(tick) = last_talk_tick {
                data.last_talk = tick;
            }
            data.amgivingback = 0;
        }
        talk_target
    }

    /// C `if (talkdir) turn(cn, talkdir);` (`arena.c:710-712`).
    fn arena_face_talk_target(&mut self, master_id: CharacterId, target_id: CharacterId) {
        let (Some(master), Some(target)) = (
            self.characters.get(&master_id).cloned(),
            self.characters.get(&target_id),
        ) else {
            return;
        };
        if let Some(direction) = offset2dx(
            i32::from(master.x),
            i32::from(master.y),
            i32::from(target.x),
            i32::from(target.y),
        ) {
            if let Some(master_mut) = self.characters.get_mut(&master_id) {
                let _ = turn(master_mut, direction as u8);
            }
        }
    }

    /// C `if (dat->last_talk + TICKS*10 < ticker) { if
    /// (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
    /// lastact)) return; }` (`arena.c:714-718`), ported via the same
    /// `setup_walk_toward`/`turn` fallback `world/bank.rs::
    /// process_bank_tick_action`/`world/clanmaster.rs::
    /// clanmaster_tick_action` already established (`DX_DOWN` = 3 =
    /// [`Direction::Down`], hardcoded in C rather than a zone-file arg).
    fn arena_master_return_to_post(&mut self, master_id: CharacterId, area_id: u16) {
        let Some(master) = self.characters.get(&master_id).cloned() else {
            return;
        };
        let Some(CharacterDriverState::ArenaMaster(data)) = master.driver_state.clone() else {
            return;
        };
        if self.tick.0 <= data.last_talk + TICKS_PER_SECOND * 10 {
            return;
        }
        if self.setup_walk_toward(
            master_id,
            usize::from(master.rest_x),
            usize::from(master.rest_y),
            0,
            area_id,
            false,
        ) {
            return;
        }
        if master.dir != Direction::Down as u8 {
            if let Some(master_mut) = self.characters.get_mut(&master_id) {
                let _ = turn(master_mut, Direction::Down as u8);
            }
        }
    }

    /// C `if (dat->storage_state > 3) { switch (dat->state) { ... } }`
    /// (`arena.c:720-732`) - the storage gate is always true here, see the
    /// module doc comment.
    fn arena_master_tournament_tick(&mut self, master_id: CharacterId) {
        let Some(CharacterDriverState::ArenaMaster(data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };
        match data.state {
            MS_PAIR => self.arena_find_contender(master_id),
            MS_IN => self.arena_check_inside(master_id),
            MS_FIGHT => self.arena_check_fight(master_id),
            _ => {}
        }
    }

    fn arena_master_tick(
        &mut self,
        master_id: CharacterId,
        area_id: u16,
        arena_score_of: &mut dyn FnMut(CharacterId) -> i32,
    ) {
        let talk_target = self.process_arena_master_messages(master_id, arena_score_of);
        if let Some(target_id) = talk_target {
            self.arena_face_talk_target(master_id, target_id);
        }
        self.arena_master_return_to_post(master_id, area_id);
        self.arena_master_tournament_tick(master_id);
    }

    /// Arena tournament master NPC tick: process messages (register/
    /// enter/leave/give), face whoever last spoke, walk/turn back to
    /// post, and advance the tournament state machine. Ports the per-tick
    /// body of C `master_driver`. `arena_score_of` resolves a
    /// registrant's current arena rating (`PlayerRuntime::arena_score`,
    /// which `World` cannot reach directly - see the module doc comment).
    pub fn process_arena_master_actions(
        &mut self,
        area_id: u16,
        mut arena_score_of: impl FnMut(CharacterId) -> i32,
    ) {
        let master_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ARENAMASTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for master_id in master_ids {
            self.arena_master_tick(master_id, area_id, &mut arena_score_of);
        }
    }

    /// This fighter bot's own local arena rating - see
    /// [`crate::character_driver::ArenaFighterDriverData`]'s doc comment
    /// for why it lives on `driver_state` instead of a real
    /// `PlayerRuntime::arena_score`. Reproduces C's `!ppd->fights`
    /// newcomer reseed (`arena.c:437-443`), matching
    /// `PlayerRuntime::arena_score` exactly.
    pub fn arena_fighter_score(&self, fighter_id: CharacterId) -> Option<i32> {
        match self.characters.get(&fighter_id)?.driver_state.as_ref()? {
            CharacterDriverState::ArenaFighter(data) => Some(if data.fights == 0 {
                ARENA_PPD_NEWCOMER_SCORE
            } else {
                data.score
            }),
            _ => None,
        }
    }

    /// Winner-side half of `score_fight` (`arena.c:432-534`) for a fighter
    /// bot combatant with no `PlayerRuntime` - the `World`-local
    /// counterpart to `PlayerRuntime::apply_arena_win`, called by
    /// `crates/ugaris-server/src/world_events.rs::apply_arena_master_events`
    /// whenever a `FightScored` participant isn't a real player. See that
    /// function's own doc comment for why this takes only the loser's
    /// pre-fight score rather than a second simultaneous mutable borrow.
    pub fn apply_arena_fighter_win(
        &mut self,
        fighter_id: CharacterId,
        loser_score_before: i32,
    ) -> Option<i32> {
        let Some(CharacterDriverState::ArenaFighter(mut data)) = self
            .characters
            .get(&fighter_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return None;
        };
        let winner_score_before = if data.fights == 0 {
            ARENA_PPD_NEWCOMER_SCORE
        } else {
            data.score
        };
        let worth = PlayerRuntime::arena_fight_worth(winner_score_before - loser_score_before);
        let new_score = winner_score_before + worth;
        data.score = new_score;
        data.fights += 1;
        data.wins += 1;
        if let Some(character) = self.characters.get_mut(&fighter_id) {
            character.driver_state = Some(CharacterDriverState::ArenaFighter(data));
        }
        Some(new_score)
    }

    /// Loser-side half - see [`World::apply_arena_fighter_win`]'s doc
    /// comment.
    pub fn apply_arena_fighter_loss(
        &mut self,
        fighter_id: CharacterId,
        winner_score_before: i32,
    ) -> Option<i32> {
        let Some(CharacterDriverState::ArenaFighter(mut data)) = self
            .characters
            .get(&fighter_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return None;
        };
        let loser_score_before = if data.fights == 0 {
            ARENA_PPD_NEWCOMER_SCORE
        } else {
            data.score
        };
        let worth = PlayerRuntime::arena_fight_worth(winner_score_before - loser_score_before);
        let new_score = loser_score_before - worth;
        data.score = new_score;
        data.fights += 1;
        data.losses += 1;
        if let Some(character) = self.characters.get_mut(&fighter_id) {
            character.driver_state = Some(CharacterDriverState::ArenaFighter(data));
        }
        Some(new_score)
    }

    /// Locates the arena tournament master this fighter bot should talk
    /// to. C's `say(cn, "register")`/`say(cn, "enter")` rely on the
    /// generic `log_area` broadcast finding whichever `master_driver` NPC
    /// is within earshot; since this port calls the master's own
    /// (private, same-module) handlers directly instead of faking a say
    /// (no generic "NPC speech also reaches other NPCs' `NT_TEXT` queues"
    /// plumbing exists yet - only player speech does, in
    /// `ugaris-server::commands_chat`), this picks the nearest
    /// `CDR_ARENAMASTER` NPC the fighter can currently see instead - by
    /// construction the fighter only calls this once it has already
    /// walked to [`ARENA_FIGHTER_MASTER_POS`], so there is normally only
    /// one candidate in range.
    fn arena_fighter_find_master(&self, fighter_id: CharacterId) -> Option<CharacterId> {
        let fighter = self.characters.get(&fighter_id)?;
        self.characters
            .values()
            .filter(|master| {
                master.driver == CDR_ARENAMASTER && master.flags.contains(CharacterFlags::USED)
            })
            .filter(|master| char_see_char(master, fighter, &self.map, self.date.daylight))
            .min_by_key(|master| {
                i32::from(master.x).abs_diff(i32::from(fighter.x))
                    + i32::from(master.y).abs_diff(i32::from(fighter.y))
            })
            .map(|master| master.id)
    }

    /// C `fighter_driver`'s message loop (`arena.c:850-878`), narrowed to
    /// the `NT_GIVE` (destroy any gift immediately, C's `NT_TEXT`/
    /// `NT_CHAR` branches are dead code - both only ever assign to a
    /// commented-out local `co`) and `NT_NPC`/`NTID_ARENA` branches (the
    /// same 6 signals `master_driver` sends, see `arena_notify_char`'s
    /// call sites): `0`=paired-wait-for-enter, `1`=attack-now (`dat3` =
    /// opponent), `2`=fight-over, `3`=registered, `4`=entered-ok,
    /// `5`=rejected/kicked-out. Each is only honored from the matching
    /// prior state, exactly like C's own state guards.
    fn process_arena_fighter_messages(&mut self, fighter_id: CharacterId) {
        let messages = self
            .characters
            .get_mut(&fighter_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();
        let Some(CharacterDriverState::ArenaFighter(mut data)) = self
            .characters
            .get(&fighter_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        // C `-TICKS*60*5` (`arena.c:955, 966, 972`): an absolute deeply-
        // negative `lastact`, guaranteeing the very next tick's `ticker -
        // lastact` reads as "long enough ago" without an artificial wait,
        // same trick as the `NT_CREATE` seed (`arena.c:854`).
        let far_past = -(TICKS_PER_SECOND as i64) * 60 * 5;

        for message in &messages {
            if message.message_type == NT_GIVE {
                if let Some(item_id) = self
                    .characters
                    .get_mut(&fighter_id)
                    .and_then(|character| character.cursor_item.take())
                {
                    self.destroy_item(item_id);
                }
                continue;
            }
            if message.message_type != NT_NPC || message.dat1 != NTID_ARENA {
                continue;
            }
            match message.dat2 {
                0 if data.state == FS_WAIT => {
                    data.state = FS_ENTER;
                    data.last_act = far_past;
                }
                1 if data.state == FS_WAIT2 => {
                    data.state = FS_FIGHT;
                    data.enemy = Some(CharacterId(message.dat3.max(0) as u32));
                    data.enemy_visible = false;
                }
                2 if data.state == FS_FIGHT || data.state == FS_WAIT2 => {
                    data.state = FS_LEISURE;
                    data.last_act = self.tick.0 as i64;
                }
                3 if data.state == FS_REGISTER => {
                    data.state = FS_WAIT;
                    data.last_act = far_past;
                }
                4 if data.state == FS_ENTER => {
                    data.state = FS_WAIT2;
                    data.last_act = far_past;
                }
                5 => {
                    data.state = FS_LEISURE;
                    data.last_act = self.tick.0 as i64;
                }
                _ => {}
            }
        }

        if let Some(character) = self.characters.get_mut(&fighter_id) {
            character.driver_state = Some(CharacterDriverState::ArenaFighter(data));
        }
    }

    /// C `fight_driver_update`'s narrowed single-enemy equivalent (see
    /// `world/gate_fight.rs`'s module doc comment for why this codebase
    /// never ported the generic 10-slot `struct fight_driver_data`):
    /// refreshes `enemy_visible`/last-known position for the one enemy
    /// `FS_FIGHT` was handed (`arena.c:872-875`'s `fight_driver_add_enemy`
    /// call), or gives up if the enemy's character has vanished entirely.
    fn arena_fighter_update_enemy_visibility(&mut self, fighter_id: CharacterId) {
        let Some(CharacterDriverState::ArenaFighter(mut data)) = self
            .characters
            .get(&fighter_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };
        let Some(enemy_id) = data.enemy else { return };

        match self
            .characters
            .get(&fighter_id)
            .cloned()
            .zip(self.characters.get(&enemy_id).cloned())
        {
            Some((fighter, enemy)) => {
                if char_see_char(&fighter, &enemy, &self.map, self.date.daylight) {
                    data.enemy_visible = true;
                    data.enemy_last_x = enemy.x;
                    data.enemy_last_y = enemy.y;
                } else {
                    data.enemy_visible = false;
                }
            }
            None => {
                data.enemy = None;
                data.enemy_visible = false;
            }
        }

        if let Some(character) = self.characters.get_mut(&fighter_id) {
            character.driver_state = Some(CharacterDriverState::ArenaFighter(data));
        }
    }

    /// C `if (dat->storage_state > 3) { switch (dat->state) { ... } }`
    /// (`arena.c:920-967`) - the storage gate is always true here, same
    /// simplification as `arena_master_tournament_tick`. Returns C's
    /// early `return` (`true` skips `spell_self_driver`/`do_idle` this
    /// tick, matching every branch that itself performed a move/attack).
    fn arena_fighter_state_action(&mut self, fighter_id: CharacterId, area_id: u16) -> bool {
        self.arena_fighter_update_enemy_visibility(fighter_id);

        let Some(CharacterDriverState::ArenaFighter(mut data)) = self
            .characters
            .get(&fighter_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return false;
        };
        let Some(fighter) = self.characters.get(&fighter_id).cloned() else {
            return false;
        };
        let tick = self.tick.0 as i64;

        let acted = match data.state {
            // C `case FS_LEISURE` (`arena.c:921-929`).
            FS_LEISURE => {
                let (rest_x, rest_y) = (fighter.rest_x, fighter.rest_y);
                let far = i32::from(fighter.x).abs_diff(i32::from(rest_x)) > 2
                    || i32::from(fighter.y).abs_diff(i32::from(rest_y)) > 2;
                if far
                    && self.setup_walk_toward(
                        fighter_id,
                        usize::from(rest_x),
                        usize::from(rest_y),
                        2,
                        area_id,
                        false,
                    )
                {
                    true
                } else if tick - data.last_act < TICKS_PER_SECOND as i64 * 60 * 3 {
                    false
                } else {
                    data.state = FS_START;
                    false
                }
            }
            // C `case FS_START` (`arena.c:930-936`).
            FS_START => {
                let (master_x, master_y) = ARENA_FIGHTER_MASTER_POS;
                let close = i32::from(fighter.x).abs_diff(i32::from(master_x)) < 5
                    && i32::from(fighter.y).abs_diff(i32::from(master_y)) < 5;
                if close {
                    data.state = FS_REGISTER;
                    false
                } else {
                    self.setup_walk_toward(
                        fighter_id,
                        usize::from(master_x),
                        usize::from(master_y),
                        4,
                        area_id,
                        false,
                    )
                }
            }
            // C `case FS_REGISTER` (`arena.c:937-943`): keeps re-saying
            // "register" every 30 seconds (C's own `dat->state++` advance
            // is commented out) until `master_driver`'s `NT_NPC` dat2=3
            // ack moves it to `FS_WAIT`.
            FS_REGISTER => {
                if tick - data.last_act < TICKS_PER_SECOND as i64 * 30 {
                    false
                } else {
                    self.npc_say(fighter_id, "register");
                    if let Some(master_id) = self.arena_fighter_find_master(fighter_id) {
                        let score = self
                            .arena_fighter_score(fighter_id)
                            .unwrap_or(ARENA_PPD_NEWCOMER_SCORE);
                        self.arena_add_contender(master_id, fighter_id, score);
                    }
                    data.last_act = tick;
                    false
                }
            }
            // C `case FS_WAIT`: `break;` (`arena.c:944-945`).
            FS_WAIT => false,
            // C `case FS_ENTER` (`arena.c:946-952`): same re-say pattern
            // as `FS_REGISTER`.
            FS_ENTER => {
                if tick - data.last_act < TICKS_PER_SECOND as i64 * 30 {
                    false
                } else {
                    self.npc_say(fighter_id, "enter");
                    if let Some(master_id) = self.arena_fighter_find_master(fighter_id) {
                        self.arena_handle_enter(master_id, fighter_id);
                    }
                    data.last_act = tick;
                    false
                }
            }
            // C `case FS_WAIT2`: `break;` (`arena.c:953-954`).
            FS_WAIT2 => false,
            // C `case FS_FIGHT` (`arena.c:955-961`):
            // `fight_driver_attack_visible`/`fight_driver_follow_invisible`
            // narrowed to the single tracked enemy (see
            // `arena_fighter_update_enemy_visibility`'s doc comment).
            FS_FIGHT => {
                if data.enemy_visible {
                    match data.enemy {
                        Some(enemy_id) => self.attack_driver_direct(fighter_id, enemy_id, area_id),
                        None => false,
                    }
                } else if data.enemy.is_some() {
                    let (last_x, last_y) = (data.enemy_last_x, data.enemy_last_y);
                    let arrived = fighter.x.abs_diff(last_x) < 2 && fighter.y.abs_diff(last_y) < 2;
                    if arrived {
                        data.enemy = None;
                        false
                    } else {
                        self.secure_move_driver(
                            fighter_id,
                            last_x,
                            last_y,
                            Direction::Down as u8,
                            0,
                            0,
                            area_id,
                        )
                    }
                } else {
                    false
                }
            }
            _ => false,
        };

        if let Some(character) = self.characters.get_mut(&fighter_id) {
            character.driver_state = Some(CharacterDriverState::ArenaFighter(data));
        }
        acted
    }

    fn arena_fighter_tick(&mut self, fighter_id: CharacterId, area_id: u16) {
        self.process_arena_fighter_messages(fighter_id);
        if self.arena_fighter_state_action(fighter_id, area_id) {
            return;
        }
        // C `if (spell_self_driver(cn)) return; ... do_idle(cn, TICKS);`
        // (`arena.c:963-969`, minus the no-op storage-management switch -
        // see the module doc comment).
        if self.spell_self_simple_baddy(fighter_id) {
            return;
        }
        self.idle_simple_baddy(fighter_id);
    }

    /// Arena tournament practice-bot NPC tick (`CDR_ARENAFIGHTER`, C
    /// `fighter_driver`): processes its own `driver_messages`, walks
    /// home/to the master, registers/enters/fights on its own, entirely
    /// self-contained (its own local win/loss ledger lives on
    /// `ArenaFighterDriverData`, not `PlayerRuntime` - see that struct's
    /// doc comment).
    pub fn process_arena_fighter_actions(&mut self, area_id: u16) {
        let fighter_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ARENAFIGHTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for fighter_id in fighter_ids {
            self.arena_fighter_tick(fighter_id, area_id);
        }
    }
}
