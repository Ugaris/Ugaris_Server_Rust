//! `CDR_MILITARY_MASTER` (`military_master_driver`, `military.c:
//! 2108-2206`): greeting/mission-offer/accept/complete/reroll dialogue,
//! the NPC-scoped [`MilitaryMasterStorage`] quest counters, and the
//! clan-recommendation feed. Split out of the former single
//! `military.rs` for size - see `super` (`military/mod.rs`) for the
//! full porting history.

use super::*;

/// Outcome of [`crate::PlayerRuntime::greet_player`] (C `greet_player`,
/// `military.c:1764-1798`), mirroring every distinct `say()` branch (plus
/// the silent no-op ones).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreetPlayerOutcome {
    /// C: `ppd->master_state != 0` (after the stale-`10` reset) - already
    /// greeted this visit, no text.
    AlreadyGreeted,
    /// C: an advisor's specific-mission recommendation already rendered
    /// the greeting text this visit (`process_advisor_recommendation`,
    /// still unported) - no additional text here, just the `master_state
    /// = 2` stamp.
    AdvisorRecommendationAlreadyShown,
    /// C: `ppd->took_mission` nonzero -> "Ah, hello %s. Any luck with
    /// your mission? Or would you like to hear it again? Or have you
    /// failed to complete it?".
    HasActiveMission,
    /// C: `ppd->solved_yday == yday + 1` -> "I don't have another
    /// mission for you today, %s.".
    AlreadyCompletedToday,
    /// C: `get_army_rank_int(co)` nonzero -> "Hello, %s. I might have a
    /// mission for you. If you don't like the available missions, you
    /// can reroll for 200 gold.".
    HasRank,
    /// C: none of the above -> "Greetings, %s.".
    NewPlayer,
}

/// Outcome of [`World::complete_mission`] (C `complete_mission`,
/// `military.c:1362-1436`)'s successful branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompletedMission {
    pub difficulty: usize,
    pub exp_awarded: i32,
    pub military_pts_awarded: i32,
    /// Mercenary-only bonus gold (`ppd->mis[difficulty].exp / 5`), 0 for
    /// every other profession.
    pub gold_awarded: i32,
    /// `Some(new_rank)` if this completion crossed an Imperial Army rank
    /// threshold (C's `rank > get_army_rank_int(co)` guard).
    pub promoted_to: Option<i32>,
}

/// Outcome of [`World::complete_mission`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteMissionResult {
    /// C: `if (!ppd->solved_mission) return 0;` - nothing to complete, no
    /// mutation happened.
    NoActiveMission,
    Completed(CompletedMission),
}

impl World {
    /// C `complete_mission(cn, co, ppd, dat)` (`military.c:1362-1436`)'s
    /// full ppd + character mutation: awards the mission's exp via
    /// [`World::give_exp`] (`ppd->normal_exp` bookkeeping matches
    /// `Character.military_normal_exp`, same field [`World::give_military_
    /// pts`] uses), the mercenary bonus gold/points formula
    /// (`ch[co].prof[P_MERCENARY]`, `legacy::profession::MERCENARY`), the
    /// raw `military_pts` add (deliberately *not* routed through
    /// [`World::give_military_pts`] - unlike that function's `_no_npc`
    /// form, C's own `complete_mission` never applies
    /// `hardcore_military_exp_bonus` to `pts`, and the exp was already
    /// awarded above, so reusing it would double-grant exp and misapply
    /// the hardcore bonus), and the identical rank-promotion
    /// message/broadcast pattern. C's "Well done, %s. You've solved your
    /// mission!" and "You've been promoted to X. Congratulations, %s!"
    /// lines are the Master NPC's own `say(cn, ...)` (`military.c:
    /// 1394,1418`), ported as [`World::npc_quiet_say`] from `master_id`
    /// (matching every other line in this NPC's driver, all of which are
    /// uniformly ported as `npc_quiet_say` regardless of whether C used
    /// `say` or `quiet_say` at that call site); the (mercenary-only)
    /// gold-received text is a genuine private system message
    /// (`give_money`'s own `log_char`, `tool.c:1470-1471`), so it stays on
    /// [`World::queue_system_text_bytes`]. Also bumps `dat->storage_data.
    /// quests_solved/pts_given/exp_given[difficulty]` on the Military
    /// Master NPC identified by `master_id` (`military.c:1382,1407,1411`) -
    /// a no-op if `master_id` isn't a live `CDR_MILITARY_MASTER` NPC.
    /// Does *not* itself track the wealth-achievement ladder the real
    /// `give_money` also updates (`tool.c:1475-1477`) - that needs
    /// `add_gold_earned`'s DB-backed first-unlock announce, which lives in
    /// the server crate. The one real call site
    /// (`ugaris-server`'s `apply_military_master_nearby_player`) wires it
    /// itself via `award_swap_money_converted_achievement` on
    /// [`CompletedMission::gold_awarded`] after calling this function -
    /// see that function's doc comment.
    pub fn complete_mission(
        &mut self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        area_id: u32,
        master_id: CharacterId,
    ) -> CompleteMissionResult {
        if !player.military_solved_mission() {
            return CompleteMissionResult::NoActiveMission;
        }
        player.set_military_solved_mission(false);

        let took_yday = player.military_took_yday();
        player.set_military_solved_yday(took_yday);
        player.set_military_took_yday(0);

        let took_mission = player.military_took_mission();
        let difficulty = (took_mission - 1).clamp(0, 4) as usize;
        player.set_military_took_mission(0);

        let mission = player.military_mission(difficulty);

        self.give_exp(character_id, i64::from(mission.exp), area_id);

        let Some(character) = self.characters.get_mut(&character_id) else {
            return CompleteMissionResult::Completed(CompletedMission {
                difficulty,
                exp_awarded: mission.exp,
                ..Default::default()
            });
        };
        character.military_normal_exp = character.military_normal_exp.saturating_add(mission.exp);

        let mercenary_level = i32::from(character.professions[profession::MERCENARY]);
        let mut gold_awarded = 0;
        let pts = if mercenary_level > 0 {
            gold_awarded = mission.exp / 5;
            character.gold = character.gold.saturating_add(gold_awarded as u32);
            character.flags.insert(CharacterFlags::ITEMS);
            mission.pts + mission.pts / 2 + mission.pts * mercenary_level * 3 / 100 + 1
        } else {
            mission.pts + mission.pts / 2
        };

        let old_rank = army_rank_for_points(character.military_points);
        character.military_points = character.military_points.saturating_add(pts);
        character.flags.insert(CharacterFlags::UPDATE);
        let new_rank = army_rank_for_points(character.military_points);
        let name = character.name.clone();

        // C `dat->storage_data.quests_solved[difficulty]++;` /
        // `pts_given[difficulty] += ppd->mis[difficulty].pts;` /
        // `exp_given[difficulty] += ppd->mis[difficulty].exp;`
        // (`military.c:1382,1407,1411`) - a no-op if `master_id` isn't a
        // live `CDR_MILITARY_MASTER` NPC.
        if let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            let storage_id = data.storage_id;
            self.military_master_storage.add_completed_mission_stats(
                storage_id,
                difficulty,
                mission.exp,
                mission.pts,
            );
        }

        if gold_awarded > 0 {
            let gold_str = if gold_awarded < 100 {
                format!("{gold_awarded}s")
            } else {
                format!("{:.2}G", f64::from(gold_awarded) / 100.0)
            };
            let mut message = Vec::with_capacity(64);
            message.extend_from_slice(b"You received");
            message.extend_from_slice(crate::text::COL_YELLOW);
            message.push(b' ');
            message.extend_from_slice(gold_str.as_bytes());
            message.extend_from_slice(crate::text::COL_RESET);
            message.extend_from_slice(b". It has been placed in your gold pouch.");
            self.queue_system_text_bytes(character_id, message);
        }
        // C `complete_mission`'s "Well done"/promotion lines are the
        // Military Master NPC's own `say(cn, ...)` (`military.c:1394,1418`),
        // not a private system message to the player - matches every other
        // line in this driver, which is uniformly ported as `npc_quiet_say`
        // from `master_id` (see this module's server-crate call site,
        // `apply_military_master_nearby_player`).
        self.npc_quiet_say(
            master_id,
            &format!("Well done, {name}. You've solved your mission!"),
        );

        let promoted_to = if new_rank > old_rank {
            self.npc_quiet_say(
                master_id,
                &format!(
                    "You've been promoted to {}. Congratulations, {name}!",
                    army_rank_name(new_rank)
                ),
            );
            if new_rank > 9 {
                let mut broadcast = b"0000000000".to_vec();
                broadcast.extend_from_slice(crate::text::COL_CHAT_GRATS);
                broadcast.extend_from_slice(
                    format!("Grats: {name} is a {} now!", army_rank_name(new_rank)).as_bytes(),
                );
                self.queue_channel_broadcast(6, broadcast);
            }
            Some(new_rank)
        } else {
            None
        };

        CompleteMissionResult::Completed(CompletedMission {
            difficulty,
            exp_awarded: mission.exp,
            military_pts_awarded: pts,
            gold_awarded,
            promoted_to,
        })
    }
}

/// Outcome of [`World::mission_reroll`] (C `handle_mission_reroll`,
/// `military.c:1889-1936`), mirroring every distinct `say()` branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionRerollOutcome {
    /// C: `ppd->reroll_yday == yday + 1` -> "I've already offered you a
    /// different set of missions today, %s. Come back tomorrow if you
    /// want more options.".
    AlreadyRerolledToday,
    /// C: `ppd->took_mission` nonzero -> "You already accepted a mission,
    /// %s. You must either complete it or report your failure before
    /// requesting new missions.".
    HasActiveMission,
    /// C: `ch[co].gold < 20000` -> "Generating new mission plans costs
    /// 200 gold, %s, which you don't seem to have.".
    InsufficientGold,
    /// C: `ppd->master_state != 10` (not yet confirmed) -> "I can prepare
    /// a different set of missions for you, %s, but it will cost 200
    /// gold. Say reroll again to confirm.", stamps `master_state = 10`.
    ConfirmationRequested,
    /// Confirmed; 200 gold spent and a fresh 5-slot offer table
    /// generated (now in `ppd->mis[]`), matching C's "Very well, %s.
    /// Here are your new mission options:" plus its `offer_missions`
    /// call - callers should read the mission table back via
    /// [`crate::PlayerRuntime::military_mission`] to render it, same as
    /// every other offer-table consumer in this module.
    Rerolled,
}

impl World {
    /// C `handle_mission_reroll(cn, co, ppd)` (`military.c:1889-1936`):
    /// the paid mission-reroll confirmation flow. `yday` is C's global
    /// `yday` (`World.date.yday`); `rng_seed` is caller-supplied, same as
    /// [`crate::PlayerRuntime::apply_mission_offer`] (no Rust call site
    /// yet resolves either - see this module's doc comment). Reproduces
    /// C's own rank-cubed `military_pts` floor-up (`generate_mission_
    /// with_preference`'s "Adjust military exp for rank if the player
    /// gained a rank elsewhere" comment) here at the call site, exactly
    /// like that comment describes, since `military_pts` isn't otherwise
    /// kept in sync with `Character.military_points` between mission
    /// generations.
    pub fn mission_reroll(
        &mut self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        yday: i32,
        rng_seed: &mut u32,
    ) -> MissionRerollOutcome {
        if player.military_reroll_yday() == yday + 1 {
            return MissionRerollOutcome::AlreadyRerolledToday;
        }
        if player.military_took_mission() != 0 {
            return MissionRerollOutcome::HasActiveMission;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return MissionRerollOutcome::InsufficientGold;
        };
        if character.gold < 20_000 {
            return MissionRerollOutcome::InsufficientGold;
        }
        if player.master_state() != 10 {
            player.set_master_state(10);
            return MissionRerollOutcome::ConfirmationRequested;
        }

        let (level, rank) = {
            let character = self
                .characters
                .get_mut(&character_id)
                .expect("checked above");
            character.gold -= 20_000;
            character.flags.insert(CharacterFlags::ITEMS);
            (
                character.level as i32,
                army_rank_for_points(character.military_points),
            )
        };

        let rank_cubed = rank.saturating_mul(rank).saturating_mul(rank);
        if rank_cubed > player.military_pts() {
            player.set_military_pts(rank_cubed);
        }

        player.set_military_reroll_yday(yday + 1);
        player.set_mission_yday(0);

        let preferred_type = player.mission_type_preference();
        let military_pts = player.military_pts();
        player.apply_mission_offer(level, military_pts, preferred_type, yday, rng_seed);

        player.set_master_state(2);

        MissionRerollOutcome::Rerolled
    }
}

/// C `military.c:2108-2206`'s `military_master_driver`'s `NT_CHAR`
/// distance gate (`char_dist(cn, co) > 10`).
pub(crate) const MILITARY_MASTER_GREET_DISTANCE: i32 = 10;

/// C `analyse_text_driver`'s own distance gate (`char_dist(cn, co) >
/// 12`), shared by every qa-table NPC's text handling.
pub(crate) const MILITARY_MASTER_TEXT_DISTANCE: i32 = 12;

/// C `DX_DOWN` (`common/direction.h:20`): the Military Master's fixed
/// resting facing (C's own `secure_move_driver(cn, ch[cn].tmpx,
/// ch[cn].tmpy, DX_DOWN, ret, lastact)`, `military.c:2201`).
pub(crate) const MILITARY_MASTER_REST_DIRECTION: u8 = 3;

/// C `static char *diff_name[5]` (`military.c:339`).
pub(crate) const MISSION_DIFFICULTY_NAMES: [&str; 5] =
    ["easy", "normal", "hard", "impossible", "insane"];

/// C `diff_name[difficulty]`/`get_colored_difficulty_name`'s own clamp
/// (`military.c:1350-1361` - out-of-range falls back to index `0`).
pub fn mission_difficulty_name(difficulty: usize) -> &'static str {
    MISSION_DIFFICULTY_NAMES
        .get(difficulty)
        .copied()
        .unwrap_or("easy")
}

/// C `describe_mission` (`military.c:1194-1220`): the offer-time
/// description ("I have an easy mission for you, NAME. ..."). `None` for
/// an empty mission slot (`mission->type == 0`) or an unrecognized type,
/// matching C's own guard/`default: return 0`. C wraps the difficulty
/// name in `COL_LIGHT_BLUE`/`COL_RESET` via `get_colored_difficulty_name`
/// (`military.c:1172-1182`); restored here via `COL_STR_LIGHT_BLUE`/
/// `COL_STR_RESET` sentinels - callers must use `npc_quiet_say_bytes`,
/// not the lossy `npc_quiet_say`.
pub fn describe_mission_text(
    mission: &SingleMission,
    difficulty: usize,
    player_name: &str,
) -> Option<String> {
    if mission.is_empty() {
        return None;
    }
    let diff = format!(
        "{COL_STR_LIGHT_BLUE}{}{COL_STR_RESET}",
        mission_difficulty_name(difficulty)
    );
    match mission.mission_type {
        MISSION_TYPE_DEMON => Some(format!(
            "I have an {diff} mission for you, {player_name}. It is to slay {} level {} demons \
             in the Pentagram Quest.",
            mission.opt1, mission.opt2
        )),
        MISSION_TYPE_RATLING => Some(format!(
            "I have an {diff} mission for you, {player_name}. It is to slay {} level {} \
             ratlings in the Sewers.",
            mission.opt1, mission.opt2
        )),
        MISSION_TYPE_SILVER => Some(format!(
            "I have an {diff} mission for you, {player_name}. It is to find {} units of silver \
             in the Mine.",
            mission.opt1
        )),
        _ => None,
    }
}

/// C `display_mission` (`military.c:1261-1288`): the accept/hear-time
/// description ("Your mission is to..."). `None` for an unrecognized
/// type; callers should say the "that mission is not available" line on
/// `None`, matching C's own fallback (this should not happen in practice
/// for a mission slot that was already validated non-empty by the
/// caller).
pub fn display_mission_text(mission: &SingleMission) -> Option<String> {
    match mission.mission_type {
        MISSION_TYPE_DEMON => Some(format!(
            "Your mission is to slay {} level {} demons in the Pentagram Quest.",
            mission.opt1, mission.opt2
        )),
        MISSION_TYPE_RATLING => Some(format!(
            "Your mission is to slay {} level {} ratlings in the Sewers.",
            mission.opt1, mission.opt2
        )),
        MISSION_TYPE_SILVER => Some(format!(
            "Your mission is to find {} units of silver in the Mine.",
            mission.opt1
        )),
        _ => None,
    }
}

/// C `offer_missions` (`military.c:1231-1246`): describes every mission
/// slot the player can currently afford (`mis[d].pts <= 1 ||
/// mis[d].pts <= current_pts`), falling back to the "no suitable
/// missions" line if none qualified.
pub fn offer_missions_text(
    missions: &[SingleMission; 5],
    current_pts: i32,
    player_name: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    for (difficulty, mission) in missions.iter().enumerate() {
        if mission.pts > 1 && mission.pts > current_pts {
            continue;
        }
        if let Some(text) = describe_mission_text(mission, difficulty, player_name) {
            lines.push(text);
        }
    }
    if lines.is_empty() {
        lines.push(format!(
            "I'm sorry, {player_name}, but I don't have any suitable missions for you at the \
             moment."
        ));
    }
    lines
}

/// Outcome of [`World::handle_mission_request`] (C `handle_mission_request`,
/// `military.c:1842-1896`), mirroring every distinct `say()` branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionRequestOutcome {
    /// C: `ppd->took_mission` nonzero -> "You already have a mission.
    /// Would you like to hear it again?" (this particular line has no
    /// `%s` player-name substitution, unlike almost every other branch in
    /// this file - matches C exactly).
    AlreadyHasMission,
    /// C: `ppd->solved_yday == yday + 1` -> "I don't have another mission
    /// for you today, %s.".
    AlreadyCompletedToday,
    /// C: `!get_army_rank_int(co)` -> "But you don't even belong to the
    /// army, %s. Talk to Seymour about enrollment.".
    NotEnrolled,
    /// C: a fresh advisor-recommended mission was generated and
    /// highlighted this call (`mission_type_preference > 0` and the
    /// preferred difficulty's freshly generated mission type matches it)
    /// - carries the mission description line plus the "accept by
    ///   saying X" prompt line; C returns immediately here without the
    ///   general `offer_missions` listing.
    AdvisorRecommendation { description: String, prompt: String },
    /// Normal offer: every line [`offer_missions_text`] produced, plus
    /// the reroll-footer line.
    Offered(Vec<String>),
}

impl World {
    /// C `handle_mission_request(cn, co, ppd)` (`military.c:1842-1896`):
    /// the "mission" keyword handler. Generates a fresh offer table via
    /// [`crate::PlayerRuntime::apply_mission_offer`] if none was
    /// generated today, reproducing the same rank-cubed `military_pts`
    /// floor-up [`World::mission_reroll`] already applies at its own call
    /// site (`generate_mission_with_preference`'s "Adjust military exp
    /// for rank" comment - the floor lives in the C *caller*, not the
    /// pure generator, so every caller must repeat it).
    pub fn handle_mission_request(
        &mut self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        yday: i32,
        rng_seed: &mut u32,
        player_name: &str,
    ) -> MissionRequestOutcome {
        if player.military_took_mission() != 0 {
            return MissionRequestOutcome::AlreadyHasMission;
        }
        if player.military_solved_yday() == yday + 1 {
            return MissionRequestOutcome::AlreadyCompletedToday;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return MissionRequestOutcome::NotEnrolled;
        };
        if army_rank_for_points(character.military_points) <= 0 {
            return MissionRequestOutcome::NotEnrolled;
        }

        if player.mission_yday() != yday + 1 {
            let rank = army_rank_for_points(character.military_points);
            let rank_cubed = rank.saturating_mul(rank).saturating_mul(rank);
            if rank_cubed > player.military_pts() {
                player.set_military_pts(rank_cubed);
            }
            let level = (character.level as i32).max(7);
            let preferred_type = player.mission_type_preference();
            let military_pts = player.military_pts();
            player.apply_mission_offer(level, military_pts, preferred_type, yday, rng_seed);

            if preferred_type > 0 {
                let diff_pref = player.mission_difficulty_preference();
                if (0..5).contains(&diff_pref) {
                    let mission = player.military_mission(diff_pref as usize);
                    if mission.mission_type == preferred_type {
                        let description =
                            describe_mission_text(&mission, diff_pref as usize, player_name)
                                .unwrap_or_default();
                        let prompt = format!(
                            "This mission was specifically requested by your advisor. You may \
                             accept it by saying {COL_STR_LIGHT_BLUE}{}{COL_STR_RESET}.",
                            mission_difficulty_name(diff_pref as usize)
                        );
                        return MissionRequestOutcome::AdvisorRecommendation {
                            description,
                            prompt,
                        };
                    }
                }
            }
        }

        let missions: [SingleMission; 5] = std::array::from_fn(|i| player.military_mission(i));
        let mut lines = offer_missions_text(&missions, player.military_current_pts(), player_name);
        lines.push(format!(
            "If you don't like these missions, you can request a new set by saying \
             {COL_STR_LIGHT_BLUE}reroll{COL_STR_RESET} for 200 gold. This can only be done once \
             per day."
        ));
        MissionRequestOutcome::Offered(lines)
    }
}

/// C `struct military_master_storage` (`military.c:346-352`): the
/// NPC-scoped counters `military_master_data` persists through the
/// generic `storage` table (`create_storage`/`read_storage`/
/// `update_storage`, `src/system/database/database_storage.c`) rather
/// than through per-character PPD or the world save. This is the first
/// consumer of that still-unported "storage-blob" concept - see
/// [`MilitaryMasterStorageRegistry`]'s doc comment for the scoped
/// in-memory-only approach this takes instead (no DB persistence yet).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MilitaryMasterStorage {
    /// `clan_pts[MAXCLAN]`: military points banked on behalf of each
    /// clan, fed by [`World::update_clan_points`] and spent by
    /// [`World::process_clan_recommendation`]. Index `0` ("no clan") is
    /// never read/written by either function (`get_char_clan` never
    /// returns `0`), matching C leaving `clan_pts[0]` permanently unused.
    clan_pts: [i32; crate::clan::MAX_CLAN],
    /// `quests_given[5]`: incremented once per difficulty every time a
    /// mission is offered (`military.c:1348`), fed by
    /// [`World::record_mission_offered`].
    quests_given: [i32; 5],
    /// `quests_solved[5]` (`military.c:1382`), fed by
    /// [`World::complete_mission`].
    quests_solved: [i32; 5],
    /// `exp_given[5]` (`military.c:1411`), fed by
    /// [`World::complete_mission`].
    exp_given: [i32; 5],
    /// `pts_given[5]` (`military.c:1407`), fed by
    /// [`World::complete_mission`].
    pts_given: [i32; 5],
}

impl Default for MilitaryMasterStorage {
    fn default() -> Self {
        Self {
            clan_pts: [0; crate::clan::MAX_CLAN],
            quests_given: [0; 5],
            quests_solved: [0; 5],
            exp_given: [0; 5],
            pts_given: [0; 5],
        }
    }
}

impl MilitaryMasterStorage {
    /// `dat->storage_data.clan_pts[clan_nr]`. Out-of-range clan numbers
    /// read as `0`, matching a fresh `struct military_master_storage`'s
    /// zero-initialized array (C itself never range-checks `clan_nr`
    /// beyond the caller already having a valid `get_char_clan` result).
    pub fn clan_pts(&self, clan_nr: u16) -> i32 {
        self.clan_pts.get(clan_nr as usize).copied().unwrap_or(0)
    }

    fn add_clan_pts(&mut self, clan_nr: u16, delta: i32) {
        if let Some(slot) = self.clan_pts.get_mut(clan_nr as usize) {
            *slot += delta;
        }
    }

    /// `dat->storage_data.quests_given[difficulty]`.
    pub fn quests_given(&self, difficulty: usize) -> i32 {
        self.quests_given.get(difficulty).copied().unwrap_or(0)
    }

    /// `dat->storage_data.quests_solved[difficulty]`.
    pub fn quests_solved(&self, difficulty: usize) -> i32 {
        self.quests_solved.get(difficulty).copied().unwrap_or(0)
    }

    /// `dat->storage_data.exp_given[difficulty]`.
    pub fn exp_given(&self, difficulty: usize) -> i32 {
        self.exp_given.get(difficulty).copied().unwrap_or(0)
    }

    /// `dat->storage_data.pts_given[difficulty]`.
    pub fn pts_given(&self, difficulty: usize) -> i32 {
        self.pts_given.get(difficulty).copied().unwrap_or(0)
    }

    /// `dat->storage_data.quests_given[difficulty]++` (`military.c:1348`).
    fn add_quests_given(&mut self, difficulty: usize) {
        if let Some(slot) = self.quests_given.get_mut(difficulty) {
            *slot += 1;
        }
    }

    /// `dat->storage_data.quests_solved[difficulty]++` (`military.c:1382`).
    fn add_quests_solved(&mut self, difficulty: usize) {
        if let Some(slot) = self.quests_solved.get_mut(difficulty) {
            *slot += 1;
        }
    }

    /// `dat->storage_data.exp_given[difficulty] += ppd->mis[difficulty].
    /// exp;` (`military.c:1411`).
    fn add_exp_given(&mut self, difficulty: usize, delta: i32) {
        if let Some(slot) = self.exp_given.get_mut(difficulty) {
            *slot += delta;
        }
    }

    /// `dat->storage_data.pts_given[difficulty] += ppd->mis[difficulty].
    /// pts;` (`military.c:1407`) - note this is the mission's raw point
    /// *cost*, not the (larger, formula-adjusted) amount actually
    /// credited to the player's `military_pts` (`CompletedMission::
    /// military_pts_awarded`).
    fn add_pts_given(&mut self, difficulty: usize, delta: i32) {
        if let Some(slot) = self.pts_given.get_mut(difficulty) {
            *slot += delta;
        }
    }
}

/// A registry of [`MilitaryMasterStorage`] blobs keyed by `storage_id`
/// (the zone-file `storage=N;` arg every Military Master NPC is
/// configured with, `military.c:1634-1644` - see
/// [`crate::character_driver::MilitaryMasterDriverData`]), mirroring
/// [`crate::clan::ClanRegistry`]'s "one typed struct per consumer,
/// `Serialize`/`Deserialize` end to end" shape rather than C's own
/// generic byte-blob `storage` table
/// (`src/system/database/database_storage.c`'s `create_storage`/
/// `read_storage`/`update_storage`, id/version/blob with optimistic
/// concurrency) - the scoped recommendation researched in the "Military
/// ranks" task's own iteration-114 progress-log note: "a small
/// typed-struct-per-consumer table/repository in `ugaris-db` ... keyed
/// per storage id since these aren't singletons, not a generic
/// byte-blob framework".
///
/// Unlike [`crate::clan::ClanRegistry`], this registry is **not yet
/// wired to any DB repository** - it lives only in memory for the
/// lifetime of the process, resetting to empty (all-zero counters) on
/// every restart. This is a smaller regression than it sounds: C's own
/// per-clan `clan_pts` bonus feed only ever grows by `get_clan_bonus(n,
/// 1) * 20` every 60 seconds and is spent in 12000-point chunks, so a
/// restart merely delays the next recommendation rather than losing
/// meaningful player-facing state permanently. Closing this gap (a
/// `military_master_storage(storage_id integer primary key, storage_json
///      jsonb not null, updated_at)` table following `clan.rs`'s
/// `PgClanRegistryRepository` pattern, loaded at boot and periodically
/// saved when [`Self::dirty`]) is left for a future slice - see the
/// "Military ranks" task in `PORTING_TODO.md`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MilitaryMasterStorageRegistry {
    entries: std::collections::BTreeMap<i32, MilitaryMasterStorage>,
    #[serde(skip)]
    dirty: bool,
}

impl MilitaryMasterStorageRegistry {
    /// Read-only lookup; a `storage_id` with no entry yet reads as a
    /// fresh all-zero [`MilitaryMasterStorage`] (matching C's own
    /// zero-initialized `struct military_master_storage` before the
    /// first `create_storage` round trip completes) without allocating
    /// one.
    pub fn clan_pts(&self, storage_id: i32, clan_nr: u16) -> i32 {
        self.entries
            .get(&storage_id)
            .map(|storage| storage.clan_pts(clan_nr))
            .unwrap_or(0)
    }

    /// C `dat->storage_data.clan_pts[n] += bonus;`
    /// ([`World::update_clan_points`]) / `dat->storage_data.clan_pts[clan_
    /// nr] -= 12000;` ([`World::process_clan_recommendation`]). Creates
    /// the entry on first use, matching C's `create_storage` lazily
    /// bringing a fresh zeroed blob into existence.
    fn add_clan_pts(&mut self, storage_id: i32, clan_nr: u16, delta: i32) {
        self.entries
            .entry(storage_id)
            .or_default()
            .add_clan_pts(clan_nr, delta);
        self.dirty = true;
    }

    /// C `dat->storage_data.quests_given[difficulty]++` (`accept_mission`,
    /// `military.c:1348`). Creates the entry on first use, matching
    /// [`Self::add_clan_pts`]'s lazy-`create_storage` semantics.
    fn add_quests_given(&mut self, storage_id: i32, difficulty: usize) {
        self.entries
            .entry(storage_id)
            .or_default()
            .add_quests_given(difficulty);
        self.dirty = true;
    }

    /// C `dat->storage_data.quests_solved[difficulty]++` / `pts_given`/
    /// `exp_given[difficulty] += ...` (`complete_mission`,
    /// `military.c:1382,1407,1411`), all three bumped together since C's
    /// own `complete_mission` always updates them in the same call.
    fn add_completed_mission_stats(
        &mut self,
        storage_id: i32,
        difficulty: usize,
        exp: i32,
        pts: i32,
    ) {
        let entry = self.entries.entry(storage_id).or_default();
        entry.add_quests_solved(difficulty);
        entry.add_exp_given(difficulty, exp);
        entry.add_pts_given(difficulty, pts);
        self.dirty = true;
    }

    /// Read-only per-difficulty quest-stat lookup - `(given, solved,
    /// exp_given, pts_given)`, wired from [`World::record_mission_
    /// offered`] (quests_given) and [`World::complete_mission`] (the
    /// other three) since iteration 116.
    pub fn quest_stats(&self, storage_id: i32, difficulty: usize) -> (i32, i32, i32, i32) {
        match self.entries.get(&storage_id) {
            Some(storage) => (
                storage.quests_given(difficulty),
                storage.quests_solved(difficulty),
                storage.exp_given(difficulty),
                storage.pts_given(difficulty),
            ),
            None => (0, 0, 0, 0),
        }
    }

    /// Whether any entry has changed since the last [`Self::clear_dirty`],
    /// consulted by `ugaris-server`'s periodic save tick (see
    /// `crates/ugaris-db/src/military.rs`).
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// All `(storage_id, storage)` rows currently held, for the DB
    /// repository's per-row upsert on save (`crates/ugaris-db/src/
    /// military.rs`) - mirrors the table design this type's own doc
    /// comment describes (one row per `storage_id`, unlike
    /// [`crate::clan::ClanRegistry`]'s single-row-blob approach).
    pub fn iter(&self) -> impl Iterator<Item = (i32, &MilitaryMasterStorage)> {
        self.entries.iter().map(|(id, storage)| (*id, storage))
    }

    /// Rebuilds a registry from persisted `(storage_id, storage)` rows
    /// (the DB repository's load path) without marking it dirty - a
    /// freshly loaded registry has nothing new to save back until it is
    /// mutated again.
    pub fn from_rows(rows: impl IntoIterator<Item = (i32, MilitaryMasterStorage)>) -> Self {
        Self {
            entries: rows.into_iter().collect(),
            dirty: false,
        }
    }
}

impl World {
    /// C `process_clan_recommendation(cn, co, ppd, dat)`
    /// (`military.c:1654-1674`): grants `+5 military_current_pts` (C's
    /// `ppd->current_pts += 5`) and deducts 12000 clan points once a
    /// clan-member player is greeted, gated on the clan having banked
    /// more than 12000 points (fed by [`World::update_clan_points`]) and
    /// not being the same player already recommended (`dat->last_recom`)
    /// this NPC's lifetime. Called right before
    /// [`World::process_advisor_recommendation`] in C's own `NT_CHAR`
    /// handler (`military.c:2150-2153`), matching this function's own
    /// call site in `ugaris-server`'s `apply_military_master_nearby_
    /// player`.
    ///
    /// Reads through [`crate::clan::ClanRegistry::get_char_clan`], so a
    /// stale clan reference on the player is cleared as a side effect,
    /// exactly like every other `get_char_clan` call site. Non-clan-
    /// member players are a silent no-op, matching C's `!(clan_nr =
    /// get_char_clan(co))` early return.
    pub fn process_clan_recommendation(
        &mut self,
        master_id: CharacterId,
        player_id: CharacterId,
        player: &mut PlayerRuntime,
        player_name: &str,
    ) -> Option<String> {
        let clan_nr = {
            let character = self.characters.get_mut(&player_id)?;
            self.clan_registry.get_char_clan(character)?
        };

        let storage_id = match self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::MilitaryMaster(data)) => data.storage_id,
            _ => return None,
        };

        if self.military_master_storage.clan_pts(storage_id, clan_nr) <= 12000 {
            return None;
        }

        let already_recommended = match self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::MilitaryMaster(data)) => data.last_recom == player_id.0,
            _ => true,
        };
        if already_recommended {
            return None;
        }

        player.set_military_current_pts(player.military_current_pts() + 5);
        self.military_master_storage
            .add_clan_pts(storage_id, clan_nr, -12000);
        if let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get_mut(&master_id)
            .and_then(|c| c.driver_state.as_mut())
        {
            data.last_recom = player_id.0;
        }

        Some(format!(
            "Be greeted, {player_name}. You've been recommended by your clan!"
        ))
    }

    /// C `update_clan_points(dat)` (`military.c:1815-1832`): every 60
    /// seconds, feeds every clan's `get_clan_bonus(n, 1) * 20` ("Military
    /// Advisor" bonus level, [`crate::clan::CLAN_BONUS_MILITARY_ADVISOR`])
    /// into that clan's banked `clan_pts`. Called once per Military
    /// Master NPC per tick (`ugaris-server`'s
    /// `process_military_master_actions` call site), independent of any
    /// player message.
    ///
    /// C stamps `dat->last_clan_update = realtime` on the NPC's own
    /// `NT_CREATE` (`military.c:2126`), which Rust has no equivalent
    /// hook for at zone-parse time (see
    /// [`crate::character_driver::MilitaryMasterDriverData`]'s doc
    /// comment) - so a `last_clan_update == 0` reads as "just created"
    /// here and is lazily stamped to `now` without granting a bonus yet,
    /// reproducing the same "no bonus for the first 60 seconds after
    /// spawn" behavior.
    pub fn update_clan_points(&mut self, master_id: CharacterId, now: i64) {
        let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        else {
            return;
        };
        let storage_id = data.storage_id;
        let last_clan_update = data.last_clan_update;

        if last_clan_update == 0 {
            if let Some(CharacterDriverState::MilitaryMaster(data)) = self
                .characters
                .get_mut(&master_id)
                .and_then(|c| c.driver_state.as_mut())
            {
                data.last_clan_update = now;
            }
            return;
        }
        if now - last_clan_update <= 60 {
            return;
        }

        for clan_nr in 1..crate::clan::MAX_CLAN as u16 {
            let bonus = self
                .clan_registry
                .bonus_level(clan_nr, crate::clan::CLAN_BONUS_MILITARY_ADVISOR)
                * 20;
            if bonus > 0 {
                self.military_master_storage
                    .add_clan_pts(storage_id, clan_nr, bonus);
            }
        }

        if let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get_mut(&master_id)
            .and_then(|c| c.driver_state.as_mut())
        {
            data.last_clan_update += 60;
        }
    }

    /// C `dat->storage_data.quests_given[difficulty]++;`
    /// (`accept_mission`, `military.c:1348`) - the NPC-scoped mission-
    /// offer statistic `PlayerRuntime::accept_mission` itself
    /// deliberately skips (see that function's doc comment; it has no
    /// `master_id`/`World` access). Call once per successful
    /// [`AcceptMissionOutcome::Accepted`], mirroring C calling this
    /// unconditionally at the end of `accept_mission` (which itself only
    /// runs after every rejection branch has already returned). A no-op
    /// if `master_id` isn't a live `CDR_MILITARY_MASTER` NPC.
    pub fn record_mission_offered(&mut self, master_id: CharacterId, difficulty: usize) {
        if let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            let storage_id = data.storage_id;
            self.military_master_storage
                .add_quests_given(storage_id, difficulty);
        }
    }
}

/// A `military_master_driver` outcome that needs `PlayerRuntime`'s
/// `military_ppd` (owned by `ugaris-server`'s session layer, outside
/// `World`'s visibility) to finish applying - see this module's sixth-
/// slice doc comment for why nearly every branch ends up here, unlike
/// `world/bank.rs`'s narrower `BankEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MilitaryMasterEvent {
    /// C `military_master_driver`'s `NT_CHAR` branch (`military.c:
    /// 2153-2177`, minus the still-unported `process_clan_recommendation`/
    /// `process_advisor_recommendation` calls - see this module's doc
    /// comment): greet, the `master_state == 1` rank-follow-up check,
    /// and `complete_mission`.
    NearbyPlayer {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 2 ("repeat"): `ppd->master_state = 0;`, no text.
    Repeat {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 10 ("mission"): [`World::handle_mission_request`].
    MissionRequest {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa codes 11-15 ("easy".."insane"): [`crate::PlayerRuntime::
    /// accept_mission`]. `difficulty` is `0..=4`.
    AcceptMission {
        master_id: CharacterId,
        player_id: CharacterId,
        difficulty: usize,
    },
    /// qa code 16 ("failed"): abandon the active mission.
    Failed {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 17 ("hear"): repeat the active mission's description.
    Hear {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa codes 22/"decline"/"new missions": [`World::mission_reroll`].
    Reroll {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 18 ("info", admin-only, `military.c:2037-2059`): dumps the
    /// speaker's own `military_pts`/`normal_exp` plus the master NPC's
    /// storage-scoped clan points and per-difficulty quest statistics via
    /// consecutive `say()` lines. Only queued when the speaker has
    /// `CF_GOD` (`ugaris_core::character::CharacterFlags::GOD`), matching
    /// C's own guard - a non-admin typing "info" gets silent no-op
    /// handling here exactly like C's `break;` with no output.
    Info {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 19 ("reset", admin-only, `military.c:2068-2075`):
    /// `ppd->solved_yday = ppd->mission_yday = 0`, no text. Admin-gated
    /// the same way as [`Self::Info`].
    Reset { player_id: CharacterId },
    /// qa code 20 ("raise", admin-only, `military.c:2076-2082`):
    /// `ppd->military_pts += 1000`, no text. Admin-gated the same way as
    /// [`Self::Info`].
    Raise { player_id: CharacterId },
    /// qa code 21 ("promote", admin-only, `military.c:2083-2089`):
    /// `give_military_pts(cn, co, 100, 1)`. Admin-gated the same way as
    /// [`Self::Info`].
    Promote {
        master_id: CharacterId,
        player_id: CharacterId,
    },
}

impl World {
    pub fn drain_pending_military_master_events(&mut self) -> Vec<MilitaryMasterEvent> {
        self.pending_military_master_events.drain(..).collect()
    }

    /// C `military_master_driver`'s `NT_TEXT`/`NT_GIVE` message loop
    /// (`military.c:2178-2198`). `NT_CHAR` is handled separately by
    /// [`Self::greet_nearby_military_master_players`] (see this module's
    /// doc comment).
    pub(crate) fn process_military_master_messages(&mut self, master_id: CharacterId) {
        let Some(master) = self.characters.get(&master_id).cloned() else {
            return;
        };
        let messages = {
            let Some(master_mut) = self.characters.get_mut(&master_id) else {
                return;
            };
            std::mem::take(&mut master_mut.driver_messages)
        };

        let mut destroy_cursor = false;
        let mut replies: Vec<String> = Vec::new();
        let mut events: Vec<MilitaryMasterEvent> = Vec::new();

        for message in messages {
            match message.message_type {
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3 as u32);
                    if speaker_id == master_id {
                        continue;
                    }
                    let Some(text) = message.text.as_deref() else {
                        continue;
                    };
                    let Some(speaker) = self.characters.get(&speaker_id) else {
                        continue;
                    };
                    if !speaker.flags.contains(CharacterFlags::PLAYER) {
                        continue;
                    }
                    if char_dist(&master, speaker) > MILITARY_MASTER_TEXT_DISTANCE {
                        continue;
                    }
                    if !char_see_char(&master, speaker, &self.map, self.date.daylight) {
                        continue;
                    }
                    let speaker_name = speaker.name.clone();

                    match analyse_text_qa(text, &master.name, &speaker_name, MILITARY_QA) {
                        TextAnalysisOutcome::Said(reply) => replies.push(reply),
                        // C: `answer_code == 1` -> `quiet_say(cn, "I'm
                        // %s.", ch[cn].name)`.
                        TextAnalysisOutcome::Matched(1) => {
                            replies.push(format!("I'm {}.", master.name));
                        }
                        TextAnalysisOutcome::Matched(2) => {
                            events.push(MilitaryMasterEvent::Repeat {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(10) => {
                            events.push(MilitaryMasterEvent::MissionRequest {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(code @ 11..=15) => {
                            events.push(MilitaryMasterEvent::AcceptMission {
                                master_id,
                                player_id: speaker_id,
                                difficulty: (code - 11) as usize,
                            });
                        }
                        TextAnalysisOutcome::Matched(16) => {
                            events.push(MilitaryMasterEvent::Failed {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(17) => {
                            events.push(MilitaryMasterEvent::Hear {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(22) => {
                            events.push(MilitaryMasterEvent::Reroll {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        // C: `if (!(ch[co].flags & CF_GOD)) { break; }` -
                        // every admin-only code 18-21 guards identically,
                        // silently dropping the message with no output
                        // for a non-admin speaker (matching C's `break`
                        // out of the `switch`, still `return 1` overall).
                        TextAnalysisOutcome::Matched(code @ 18..=21)
                            if speaker.flags.contains(CharacterFlags::GOD) =>
                        {
                            match code {
                                18 => events.push(MilitaryMasterEvent::Info {
                                    master_id,
                                    player_id: speaker_id,
                                }),
                                19 => events.push(MilitaryMasterEvent::Reset {
                                    player_id: speaker_id,
                                }),
                                20 => events.push(MilitaryMasterEvent::Raise {
                                    player_id: speaker_id,
                                }),
                                21 => events.push(MilitaryMasterEvent::Promote {
                                    master_id,
                                    player_id: speaker_id,
                                }),
                                _ => unreachable!(),
                            }
                        }
                        // Advisor-only codes (3-9, 30-44) and any
                        // unmatched text: no handling, matches C's own
                        // `default: return 0`.
                        TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
                    }
                }
                NT_GIVE => {
                    destroy_cursor = true;
                    replies.push("That's junk.".to_string());
                }
                _ => {}
            }
        }

        if destroy_cursor {
            let cursor = self
                .characters
                .get_mut(&master_id)
                .and_then(|master| master.cursor_item.take());
            if let Some(item_id) = cursor {
                self.destroy_item(item_id);
            }
        }

        for reply in replies {
            self.npc_quiet_say(master_id, &reply);
        }

        self.pending_military_master_events.extend(events);
    }

    /// C `military_master_driver`'s `NT_CHAR` greeting branch
    /// (`military.c:2153-2177`), ported as a periodic nearby-player scan
    /// (see this module's doc comment for why).
    pub(crate) fn greet_nearby_military_master_players(&mut self, master_id: CharacterId) {
        let Some(master) = self.characters.get(&master_id).cloned() else {
            return;
        };

        let mut nearby: Vec<CharacterId> = Vec::new();
        for character in self.characters.values() {
            if character.id == master_id || !character.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            if char_dist(&master, character) > MILITARY_MASTER_GREET_DISTANCE {
                continue;
            }
            if !char_see_char(&master, character, &self.map, self.date.daylight) {
                continue;
            }
            nearby.push(character.id);
        }

        self.pending_military_master_events
            .extend(
                nearby
                    .into_iter()
                    .map(|player_id| MilitaryMasterEvent::NearbyPlayer {
                        master_id,
                        player_id,
                    }),
            );
    }

    /// C `military_master_driver`'s movement section (`military.c:
    /// 2200-2204`): stationary NPC returning to its `rest_x`/`rest_y`
    /// spawn tile, facing `DX_DOWN`. Unlike `world/bank.rs`'s day/night
    /// shop positions, C's own `struct military_master_data` has no
    /// movement fields at all, so this is always the "no configured
    /// position" fallback.
    pub(crate) fn process_military_master_tick_action(
        &mut self,
        master_id: CharacterId,
        area_id: u16,
    ) {
        let Some(master) = self.characters.get(&master_id).cloned() else {
            return;
        };
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
        if master.dir != MILITARY_MASTER_REST_DIRECTION {
            if let Some(master_mut) = self.characters.get_mut(&master_id) {
                let _ = turn(master_mut, MILITARY_MASTER_REST_DIRECTION);
            }
        }
    }

    /// Military Master NPC tick: process messages, greet/complete-
    /// mission scan, and the movement fallback. Ports the per-tick body
    /// of C `military_master_driver` (minus the deferred clan/advisor
    /// recommendation and storage-blob persistence - see this module's
    /// doc comment).
    pub fn process_military_master_actions(&mut self, area_id: u16, now: i64) {
        let master_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_MILITARY_MASTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for master_id in master_ids {
            self.process_military_master_messages(master_id);
            self.greet_nearby_military_master_players(master_id);
            // C `update_clan_points(dat)` (`military.c:2195`): once per
            // NPC per tick, independent of any player message.
            self.update_clan_points(master_id, now);
            self.process_military_master_tick_action(master_id, area_id);
        }
    }
}

/// C `struct military_master_data`'s zone-file-parsed half
/// (`src/module/military.c:355-364`), plus the two `dat`-scoped runtime
/// fields C persists as part of the NPC's own memory image rather than
/// through the `storage_data` subsystem: `last_clan_update` (the
/// `update_clan_points` 60-second throttle timestamp, `military.c:357`)
/// and `last_recom` (the character ID of the last player granted a clan
/// recommendation, deduplicating repeat recommendations,
/// `military.c:359`). Both default to `0` here (not zone-parsed); C
/// instead stamps `last_clan_update = realtime` on `NT_CREATE`
/// (`military.c:2126`) - Rust has no equivalent creation-time hook here,
/// so [`crate::world::World::update_clan_points`] lazily treats a `0`
/// timestamp as "just created" and stamps it to the current tick's time
/// without granting a bonus yet, reproducing the same "no bonus for the
/// first 60 seconds after spawn" behavior without needing a real-time
/// value at zone-parse time.
///
/// The actual persisted `military_master_storage` counters (clan
/// points/quests given/solved/exp/pts per difficulty,
/// `struct military_master_storage`, `military.c:346-352`) live in
/// [`crate::world::MilitaryMasterStorageRegistry`], keyed by
/// `storage_id`, not on this struct - see that type's doc comment for
/// the storage-blob architectural gap this still doesn't close (no DB
/// persistence yet).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MilitaryMasterDriverData {
    pub storage_id: i32,
    #[serde(default)]
    pub last_clan_update: i64,
    #[serde(default)]
    pub last_recom: u32,
}

/// C `military_master_parse` (`military.c:1634-1644`): the only zone-file
/// arg this driver reads is `storage=N;`.
pub fn parse_military_master_driver_args(args: &str) -> MilitaryMasterDriverData {
    let mut data = MilitaryMasterDriverData::default();
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        if name == "storage" {
            data.storage_id = value.parse::<i32>().unwrap_or(0);
        }
        rest = next;
    }
    data
}
