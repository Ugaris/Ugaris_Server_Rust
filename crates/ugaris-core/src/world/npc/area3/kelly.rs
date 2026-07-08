//! Kelly NPC (`CDR_KELLY`), the Seyan'Du Sergeant who runs area 3's
//! longest quest chain: park shrines (`QLOG` 13-14), reaching Clara
//! (`QLOG` 15), swamp-beast-head bounties, and the Caligar-plaque hunt
//! (`QLOG` 54/60).
//!
//! Ports `src/area/3/area3.c::kelly_driver` (`:984-1379`) plus its shared
//! `analyse_text_driver`/`qa[]` table (`:106-204`, ported as [`AREA3_QA`]
//! in `world::npc::area3`, the same table `world::thomas`/`world::
//! sir_jones`/`world::astro2`/`world::seymour` share) and its own local
//! `collect_heads` helper (`:965-982`). Follows the same `World`/
//! `PlayerRuntime` split established by those siblings: the caller
//! supplies a per-player fact snapshot ([`KellyPlayerFacts`]) up front and
//! applies the returned [`KellyOutcomeEvent`]s afterwards, since
//! `area3_ppd.kelly_state`/`kelly_found1..3`/`kelly_found_cnt` and the
//! `QLOG` 13/14/15/54/60 quest-log entries live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! `kelly_driver`'s twenty-seven-state (`0`-`26`) dialogue chain: greeting
//! -> (gated on Seymour's chain reaching state 16) "Stone Creepers" quest
//! 13 -> "slay the huge creeper, bring its head" -> (external: player
//! hands over `IID_AREA2_CREEPERHEAD`, completes quest 13, sets state `6`,
//! awards 4 military points + 1 exp on first completion) -> "underground
//! park shrines" quest 14 -> "find them all" -> (external:
//! `world::npc::area2::parkshrine_driver`, ported as
//! [`crate::player::PlayerRuntime::memorize_park_shrine`], sets
//! `kelly_found1..3`; each newly-found shrine awards 2 military points +
//! `EXP_AREA3_SHRINE` exp immediately, and finding all three completes
//! quest 14) -> six lore lines -> (gated on level 22) "lost contact with
//! the swamp outpost" quest 15 -> (gated on Clara's chain reaching state
//! 5) "so Clara is well" (completes quest 15, awards 3 military points + 1
//! exp) -> "I will pay for swamp beast heads" -> (every subsequent visit:
//! `collect_heads` sells any carried `IID_AREA15_HEAD` items for silver)
//! -> (gated on level 56) "the Emperor's Plaque was stolen" quests 54/60
//! -> three lore lines -> "go to Gwendylon with this letter" (grants
//! `IID_CALIGARLETTER` if not already carried) -> (external: player hands
//! over `IID_CALIGARPLAQUE`, completes quest 60, awards 5000 silver) ->
//! done.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `case 1` (`ppd->seymour_state >= 16`), `case 13`
//!   (`ch[co].level >= 22`), `case 15` (`ppd->clara_state >= 5`), and
//!   `case 19` (`ch[co].level >= 56`) each conditionally fall through into
//!   the next case's body with no intervening `break` (`area3.c:1051-1057`,
//!   `:1149-1155`, `:1168-1174`, `:1196-1202`, all `// fall thru`/`// fall
//!   through intended`). Unlike `world::sir_jones`'s single unconditional
//!   `case 10` fallthrough, three of these four (`1`->`2`, `15`->`16`,
//!   `19`->`20`) land on a state (`2`/`16`/`20`) that always resolves
//!   further within the same call and is never independently reachable
//!   from a later tick (confirmed against every `kelly_state` write site,
//!   including the `NT_TEXT` repeat/restart reset buckets below), so
//!   those three are reproduced by inlining the target case's body
//!   directly inside the source case's conditional, matching `world::
//!   sir_jones`'s `case 10`->`11` precedent. `case 13`->`14` is different:
//!   `case 14`'s own body (`ppd->kelly_state == 14`) *is* independently
//!   reachable, via the `NT_TEXT` reset bucket `14..=15 -> 14` - so this
//!   one needed a shared `kelly_case14_body` helper called from both the
//!   `13` (level-gated) and `14` (direct) match arms.
//! - C's `case 9`'s two `if` blocks (per-shrine progress, then the
//!   found-all-three completion) both read/write `ppd->kelly_found_cnt`
//!   directly and unconditionally re-enter every tick while parked at
//!   state `9` (there's no early `break` between them, and no `didsay`
//!   gate on the surrounding `switch`). The per-shrine military-points/exp
//!   award (`give_military_pts`) touches only `Character` fields, so it's
//!   applied directly via [`World::give_military_pts_from_npc`] without
//!   needing an outcome event; only the `kelly_found_cnt` write itself
//!   (`PlayerRuntime`) needs [`KellyOutcomeEvent::UpdateFoundCnt`].
//! - No self-defense/regen/spell-self cascade exists in C's `kelly_
//!   driver` body at all (matching every other area-3 "pure talker" NPC's
//!   identical observation) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`area3.c:1378`) is
//!   not ported, matching the established sibling-driver precedent for
//!   stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::item_driver::{
    IID_AREA15_HEAD, IID_AREA2_CREEPERHEAD, IID_CALIGARLETTER, IID_CALIGARPLAQUE,
};
use crate::quest::quest_exp::EXP_AREA3_SHRINE;
use crate::world::*;

use super::AREA3_QA;

/// C `char_dist(cn, co) > 10` (`area3.c:1033`).
const KELLY_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`area3.c:232`, the shared
/// `analyse_text_driver` copy's own guard).
const KELLY_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`area3.c:1016`).
const KELLY_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`area3.c:1021`, `:1265`).
const KELLY_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`area3.c:1372`): idle "return to post" threshold.
const KELLY_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ch[co].level >= 22` (`area3.c:1150`): the swamp-outpost mission gate.
const KELLY_SWAMP_MIN_LEVEL: u32 = 22;
/// C `ch[co].level >= 56` (`area3.c:1197`): the Caligar-plaque mission
/// gate.
const KELLY_PLAQUE_MIN_LEVEL: u32 = 56;
/// C `questlog_open(co, 13)` (`area3.c:1062`).
const QLOG_KELLY_CREEPER: usize = 13;
/// C `questlog_open(co, 14)` (`area3.c:1089`).
const QLOG_KELLY_SHRINES: usize = 14;
/// C `questlog_open(co, 15)` (`area3.c:1164`).
const QLOG_KELLY_CLARA: usize = 15;
/// C `questlog_open(co, 54)` (`area3.c:1212`).
const QLOG_KELLY_LOOKING: usize = 54;
/// C `questlog_open(co, 60)` (`area3.c:1213`).
const QLOG_KELLY_PLAQUE: usize = 60;

/// Per-player facts [`World::process_kelly_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KellyPlayerFacts {
    /// `PlayerRuntime::area3_kelly_state()`.
    pub kelly_state: i32,
    /// `PlayerRuntime::area3_seymour_state()`, needed for the `case 1`
    /// gate (`ppd->seymour_state >= 16`).
    pub seymour_state: i32,
    /// `PlayerRuntime::quest_log.is_done(QLOG_KELLY_SHRINES)` (C
    /// `questlog_isdone(co, 14)`).
    pub quest14_done: bool,
    /// `PlayerRuntime::quest_log.is_done(QLOG_KELLY_CLARA)` (C
    /// `questlog_isdone(co, 15)`).
    pub quest15_done: bool,
    /// `PlayerRuntime::area3_clara_state()`, needed for the `case 15`
    /// gate (`ppd->clara_state >= 5`).
    pub clara_state: i32,
    /// `PlayerRuntime::area3_kelly_found1()`.
    pub found1: bool,
    /// `PlayerRuntime::area3_kelly_found2()`.
    pub found2: bool,
    /// `PlayerRuntime::area3_kelly_found3()`.
    pub found3: bool,
    /// `PlayerRuntime::area3_kelly_found_cnt()`.
    pub found_cnt: i32,
    /// `PlayerRuntime::quest_log.count(QLOG_KELLY_LOOKING)` (C
    /// `questlog_count(co, 54)`).
    pub quest54_count: u8,
    /// `PlayerRuntime::quest_log.count(QLOG_KELLY_PLAQUE)` (C
    /// `questlog_count(co, 60)`).
    pub quest60_count: u8,
}

/// A side effect [`World::process_kelly_actions`] could not apply directly
/// because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KellyOutcomeEvent {
    /// Write the new `area3_ppd.kelly_state` back.
    UpdateKellyState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, ...)`.
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `ppd->kelly_found_cnt = cnt;` (`area3.c:1116`), the tail of
    /// `case 9`'s per-shrine progress `if` block. The military-points/exp
    /// award itself is applied directly in `World` - see the module doc
    /// comment.
    UpdateFoundCnt {
        player_id: CharacterId,
        new_found_cnt: i32,
    },
    /// C `tmp = questlog_done(co, 13); ... if (tmp == 1) { give_military_
    /// pts(cn, co, 4, 1); }` (`area3.c:1328-1333`, `NT_GIVE`). `kelly_id`
    /// is needed for the conditional `give_military_pts_from_npc`
    /// promotion announcement.
    CreeperHeadQuestDone {
        player_id: CharacterId,
        kelly_id: CharacterId,
    },
    /// C `questlog_done(co, 14);` (`area3.c:1123`, `case 9`'s found-all-
    /// three tail) - return value unused, no additional point reward
    /// (quest 14's own table `exp` is `0`; the real reward already came
    /// from the per-shrine `give_military_pts` calls).
    ParkShrinesQuestDone { player_id: CharacterId },
    /// C `questlog_done(co, 15); give_military_pts(cn, co, 3, 1);`
    /// (`area3.c:1176-1177`, `case 16`) - unconditional, unlike the
    /// `NT_GIVE` completions above. `kelly_id` is needed for the
    /// `give_military_pts_from_npc` promotion announcement.
    ClaraReportDone {
        player_id: CharacterId,
        kelly_id: CharacterId,
    },
    /// C `give_money`'s `achievement_add_gold_earned` wealth-ladder half
    /// (`tool.c:1477-1479`); the gold-add/message half is already applied
    /// directly in `World` (`collect_heads`'s `give_money(co, sum, ...)`,
    /// `area3.c:978`, and the `NT_GIVE` plaque reward's `give_money(co,
    /// 5000*100, ...)`, `area3.c:1345`), matching every other `GoldEarned`
    /// event in this codebase (see `area1.rs`'s module doc comment).
    GoldEarned { player_id: CharacterId, amount: u32 },
    /// C `case 24`'s conditional letter grant (`area3.c:1239-1244`):
    /// `create_item("caligar_letter")` + `give_char_item`. The `!has_item`
    /// gate is already checked directly in `World`.
    GrantCaligarLetter { player_id: CharacterId },
    /// C `questlog_done(co, 60);` (`area3.c:1339`, `NT_GIVE`) - the exp/
    /// resend half; the `give_money` reward is applied directly in
    /// `World` (see [`KellyOutcomeEvent::GoldEarned`]).
    PlaqueQuestDone { player_id: CharacterId },
}

impl World {
    /// C `kelly_driver`'s per-tick body (`area3.c:984-1379`).
    pub fn process_kelly_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, KellyPlayerFacts>,
        area_id: u16,
    ) -> Vec<KellyOutcomeEvent> {
        let kelly_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_KELLY
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for kelly_id in kelly_ids {
            self.process_kelly_messages(kelly_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_kelly_messages(
        &mut self,
        kelly_id: CharacterId,
        player_facts: &HashMap<CharacterId, KellyPlayerFacts>,
        area_id: u16,
        events: &mut Vec<KellyOutcomeEvent>,
    ) {
        let Some(kelly_name) = self
            .characters
            .get(&kelly_id)
            .map(|kelly| kelly.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Kelly(mut data)) = self
            .characters
            .get(&kelly_id)
            .and_then(|kelly| kelly.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&kelly_id)
            .map(|kelly| std::mem::take(&mut kelly.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.kelly_handle_char_message(
                    kelly_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.kelly_handle_text_message(
                    kelly_id,
                    &kelly_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.kelly_handle_give_message(kelly_id, message, player_facts, events),
                _ => {}
            }
        }

        if let Some(kelly) = self.characters.get_mut(&kelly_id) {
            kelly.driver_state = Some(CharacterDriverState::Kelly(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`area3.c:1368-1370`).
        if let (Some(kelly), Some((tx, ty))) =
            (self.characters.get(&kelly_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(kelly.x), i32::from(kelly.y), tx, ty) {
                if let Some(kelly_mut) = self.characters.get_mut(&kelly_id) {
                    let _ = turn(kelly_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`area3.c:1372-1376`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other area-3 sibling driver uses.
        let last_talk = if let Some(kelly) = self.characters.get(&kelly_id) {
            match kelly.driver_state.as_ref() {
                Some(CharacterDriverState::Kelly(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + KELLY_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(kelly) = self.characters.get(&kelly_id) else {
                return;
            };
            let (post_x, post_y) = (kelly.rest_x, kelly.rest_y);
            self.secure_move_driver(
                kelly_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `kelly_driver`'s `NT_CHAR` branch (`area3.c:1000-1258`).
    fn kelly_handle_char_message(
        &mut self,
        kelly_id: CharacterId,
        data: &mut KellyDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, KellyPlayerFacts>,
        events: &mut Vec<KellyOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(kelly) = self.characters.get(&kelly_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`area3.c:1004-1007`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`area3.c:1010-1013`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5) continue;`
        // (`area3.c:1016-1019`).
        if tick < data.last_talk + KELLY_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co) continue;` (`area3.c:1021-1024`).
        if tick < data.last_talk + KELLY_TALK_VICTIM_TICKS
            && data
                .current_victim
                .is_some_and(|victim| victim != player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`area3.c:1026-1030`).
        if kelly_id == player_id || !char_see_char(&kelly, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`area3.c:1032-1036`).
        if char_dist(&kelly, &player) > KELLY_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id).copied() else {
            return;
        };

        let rank_name = army_rank_name(army_rank_for_points(player.military_points));
        let mut didsay = false;
        let mut new_state = facts.kelly_state;
        match facts.kelly_state {
            // C `case 0:` (`area3.c:1043-1050`).
            0 => {
                self.npc_quiet_say(
                    kelly_id,
                    &format!(
                        "Greetings, {}! I am {}, First Sergeant of the Seyan'Du, the late emperor's personal guard.",
                        player.name, kelly.name
                    ),
                );
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` falls through into `case 2:` when `ppd->
            // seymour_state >= 16` (`area3.c:1051-1065`) - `state 2` is
            // never independently persisted, see the module doc comment.
            1 => {
                if facts.seymour_state >= 16 {
                    self.npc_quiet_say(
                        kelly_id,
                        &format!(
                            "Listen, {rank_name}. There have been some attacks from beings we call Stone Creepers. They come from the depths below the city. One known entrance is in the park.",
                        ),
                    );
                    events.push(KellyOutcomeEvent::QuestOpen {
                        player_id,
                        quest: QLOG_KELLY_CREEPER,
                    });
                    new_state = 3;
                    didsay = true;
                }
            }
            // C `case 3:` (`area3.c:1066-1071`).
            3 => {
                self.npc_quiet_say(
                    kelly_id,
                    "During the last raid, one huge creeper was seen. It did as much damage as all the others together. Go there, and slay it. As proof of thy deed, bring me its head.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`area3.c:1072-1076`).
            4 => {
                self.npc_quiet_say(
                    kelly_id,
                    &format!("That will be all, {rank_name}. Dismissed!"),
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5: break;` (`area3.c:1077-1078`) - waiting for the
            // player to hand over `IID_AREA2_CREEPERHEAD`.
            5 => {}
            // C `case 6:` (`area3.c:1079-1092`).
            6 => {
                if facts.quest14_done {
                    new_state = 10;
                } else {
                    self.npc_quiet_say(
                        kelly_id,
                        &format!(
                            "I have another mission for thee, {}. Scouts have found an underground park, which can be entered through a hole in the northern part of the forest in the southern corner of Aston.",
                            player.name
                        ),
                    );
                    events.push(KellyOutcomeEvent::QuestOpen {
                        player_id,
                        quest: QLOG_KELLY_SHRINES,
                    });
                    new_state = 7;
                    didsay = true;
                }
            }
            // C `case 7:` (`area3.c:1093-1098`).
            7 => {
                self.npc_quiet_say(
                    kelly_id,
                    "They found a strange shrine there. I suspect there are several of them. Thou are to go there and find them all. Report back when thou hast found at least one.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8:` (`area3.c:1099-1103`).
            8 => {
                self.npc_quiet_say(
                    kelly_id,
                    &format!("Dismissed, {rank_name}. And good luck. Thou shalt need it."),
                );
                new_state = 9;
                didsay = true;
            }
            // C `case 9:` (`area3.c:1104-1125`).
            9 => {
                let cnt =
                    i32::from(facts.found1) + i32::from(facts.found2) + i32::from(facts.found3);
                let mut effective_found_cnt = facts.found_cnt;
                if cnt > facts.found_cnt {
                    if cnt != 1 {
                        self.npc_quiet_say(
                            kelly_id,
                            &format!(
                                "Well done. I see thou hast discovered {cnt} shrines, {}.",
                                player.name
                            ),
                        );
                    } else {
                        self.npc_quiet_say(
                            kelly_id,
                            &format!(
                                "Well done. I see thou hast discovered {cnt} shrine, {}.",
                                player.name
                            ),
                        );
                    }
                    let delta = cnt - facts.found_cnt;
                    self.give_military_pts_from_npc(
                        player_id,
                        kelly_id,
                        delta * 2,
                        (i64::from(delta) * EXP_AREA3_SHRINE) as i32,
                        u32::from(self.area_id),
                    );
                    events.push(KellyOutcomeEvent::UpdateFoundCnt {
                        player_id,
                        new_found_cnt: cnt,
                    });
                    effective_found_cnt = cnt;
                    didsay = true;
                }
                if effective_found_cnt == 3 {
                    self.npc_quiet_say(
                        kelly_id,
                        &format!("I guess there are just three of them. Good work, {rank_name}."),
                    );
                    events.push(KellyOutcomeEvent::ParkShrinesQuestDone { player_id });
                    new_state = 10;
                    didsay = true;
                }
            }
            // C `case 10:` (`area3.c:1126-1134`).
            10 => {
                self.npc_quiet_say(
                    kelly_id,
                    &format!(
                        "I tell thee, {}, all these findings are most unsettling. I think we have but scratched on the surface of this underground world, and I fear what we are yet to find.",
                        player.name
                    ),
                );
                new_state = 11;
                didsay = true;
            }
            // C `case 11:` (`area3.c:1135-1142`).
            11 => {
                self.npc_quiet_say(
                    kelly_id,
                    "The Imperial Army, the little that is left of it, is exploring the underworld, but our losses are heavy. All of a sudden, holes open everywhere, and monsters start pouring out. We needst find out where they come from, and why they attack us all of a sudden.",
                );
                new_state = 12;
                didsay = true;
            }
            // C `case 12:` (`area3.c:1143-1148`).
            12 => {
                self.npc_quiet_say(
                    kelly_id,
                    "The future seemed so bright when Seyan was still alive and Ishtar was here to teach us.",
                );
                new_state = 13;
                didsay = true;
            }
            // C `case 13:` falls through into `case 14:` when `ch[co].
            // level >= 22` (`area3.c:1149-1155`).
            13 => {
                if player.level >= KELLY_SWAMP_MIN_LEVEL {
                    let (state, said) =
                        self.kelly_case14_body(kelly_id, &player, &rank_name, facts, events);
                    new_state = state;
                    didsay = said;
                }
            }
            // C `case 14:` (`area3.c:1155-1167`) - also directly reachable
            // via the `NT_TEXT` reset bucket `14..=15 -> 14`, see the
            // module doc comment.
            14 => {
                let (state, said) =
                    self.kelly_case14_body(kelly_id, &player, &rank_name, facts, events);
                new_state = state;
                didsay = said;
            }
            // C `case 15:` falls through into `case 16:` when `ppd->
            // clara_state >= 5` (`area3.c:1168-1180`) - `state 16` is
            // never independently persisted, see the module doc comment.
            15 => {
                if facts.clara_state >= 5 {
                    self.npc_quiet_say(
                        kelly_id,
                        &format!(
                            "So Clara is well? 'Tis is good to hear, {}. I thank thee.",
                            player.name
                        ),
                    );
                    events.push(KellyOutcomeEvent::ClaraReportDone {
                        player_id,
                        kelly_id,
                    });
                    new_state = 17;
                    didsay = true;
                }
            }
            // C `case 17:` (`area3.c:1181-1187`).
            17 => {
                self.npc_quiet_say(
                    kelly_id,
                    "I think Clara is right that we should not send reinforcements yet. But these swamp beasts worry me. I will pay thee for each swamp beast head thou bringst me. The larger the head, the larger the bounty.",
                );
                new_state = 18;
                didsay = true;
            }
            // C `case 18:` (`area3.c:1188-1192`).
            18 => {
                self.npc_quiet_say(
                    kelly_id,
                    &format!(
                        "Remember to report to Clara again, {}. Dismissed.",
                        player.name
                    ),
                );
                new_state = 19;
                didsay = true;
            }
            // C `case 19:` (`area3.c:1193-1201`): `collect_heads` runs
            // unconditionally every visit, independent of the level-gated
            // fallthrough into `case 20:`.
            19 => {
                if let Some(amount) = self.collect_heads(kelly_id, player_id) {
                    events.push(KellyOutcomeEvent::GoldEarned { player_id, amount });
                    didsay = true;
                }
                if player.level >= KELLY_PLAQUE_MIN_LEVEL {
                    // C `case 20:` (`area3.c:1202-1216`) - `state 20` is
                    // never independently persisted, see the module doc
                    // comment.
                    if facts.quest60_count > 0 {
                        new_state = 26;
                    } else if facts.quest54_count > 0 {
                        new_state = 21;
                    } else {
                        self.npc_quiet_say(kelly_id, &format!("Hello again, {}.", player.name));
                        events.push(KellyOutcomeEvent::QuestOpen {
                            player_id,
                            quest: QLOG_KELLY_LOOKING,
                        });
                        events.push(KellyOutcomeEvent::QuestOpen {
                            player_id,
                            quest: QLOG_KELLY_PLAQUE,
                        });
                        new_state = 21;
                        didsay = true;
                    }
                }
            }
            // C `case 21:` (`area3.c:1217-1222`).
            21 => {
                self.npc_quiet_say(
                    kelly_id,
                    "I have another mission for you. An important plaque containing the signatures of every Emporer who has ruled over Aston has been stolen from Wesley's bank vault.",
                );
                new_state = 22;
                didsay = true;
            }
            // C `case 22:` (`area3.c:1223-1228`).
            22 => {
                self.npc_quiet_say(
                    kelly_id,
                    "He found a note that was signed 'Grendom Carmin', a member of the Carmin Clan, who were a group of destructive mages, banished from Aston by the Imperial Army.",
                );
                new_state = 23;
                didsay = true;
            }
            // C `case 23:` (`area3.c:1229-1234`).
            23 => {
                self.npc_quiet_say(
                    kelly_id,
                    "They settled in a forest to the south west of Aston and a wall was built so they could not come back.",
                );
                new_state = 24;
                didsay = true;
            }
            // C `case 24:` (`area3.c:1235-1247`).
            24 => {
                self.npc_quiet_say(
                    kelly_id,
                    "Go to Gwendylon with this letter. He will teleport you there. I have a contact named Glori collecting information on the area. See what she knows, and get that plaque back at all costs! Dismissed!",
                );
                if !self.character_has_item_template(player_id, IID_CALIGARLETTER) {
                    events.push(KellyOutcomeEvent::GrantCaligarLetter { player_id });
                }
                new_state = 25;
                didsay = true;
            }
            // C `case 25: break;` (`area3.c:1248-1249`) - waiting for the
            // player to hand over `IID_CALIGARPLAQUE`.
            25 => {}
            // C `case 26: break;` (`area3.c:1250-1251`) - all done.
            _ => {}
        }

        if new_state != facts.kelly_state {
            events.push(KellyOutcomeEvent::UpdateKellyState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`area3.c:1253-1257`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `case 14:`'s shared body (`area3.c:1155-1167`), reachable both
    /// via `case 13:`'s level-gated fallthrough and directly via the
    /// `NT_TEXT` reset bucket - see the module doc comment. Returns
    /// `(new_state, didsay)`.
    fn kelly_case14_body(
        &mut self,
        kelly_id: CharacterId,
        player: &Character,
        rank_name: &str,
        facts: KellyPlayerFacts,
        events: &mut Vec<KellyOutcomeEvent>,
    ) -> (i32, bool) {
        if facts.quest15_done {
            return (19, false);
        }
        self.npc_quiet_say(
            kelly_id,
            &format!(
                "We have lost contact with our outpost in the swamp north of Aston. I want thee to go there and deliver a full report when thou getst back. Dismissed, {rank_name}.",
            ),
        );
        events.push(KellyOutcomeEvent::QuestOpen {
            player_id: player.id,
            quest: QLOG_KELLY_CLARA,
        });
        (15, true)
    }

    /// C `kelly_driver`'s `NT_TEXT` branch (`area3.c:1261-1313`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::seymour`'s text handler).
    #[allow(clippy::too_many_arguments)]
    fn kelly_handle_text_message(
        &mut self,
        kelly_id: CharacterId,
        kelly_name: &str,
        data: &mut KellyDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, KellyPlayerFacts>,
        events: &mut Vec<KellyOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // dat->current_victim = 0;` (`area3.c:1265-1267`).
        let tick = self.tick.0;
        if tick > data.last_talk + KELLY_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co) {
        // remove_message; continue; }` (`area3.c:1269-1272`).
        if let Some(current_victim) = data.current_victim {
            if current_victim != speaker_id {
                return;
            }
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `analyse_text_driver`'s own guard clauses (`area3.c:223-238`):
        // ignore our own talk, non-players, distance > 12, not-visible.
        if kelly_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(kelly) = self.characters.get(&kelly_id).cloned() else {
            return;
        };
        if char_dist(&kelly, &speaker) > KELLY_QA_DISTANCE
            || !char_see_char(&kelly, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let kelly_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.kelly_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, kelly_name, &speaker.name, AREA3_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(kelly_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat/restart) (`area3.c:1275-1301`): six
            // mutually exclusive state buckets; `16`/`20`/`26` are
            // excluded (left untouched), matching C's missing `else`
            // branches exactly.
            TextAnalysisOutcome::Matched(2) => {
                data.last_talk = 0;
                let new_state = match kelly_state {
                    0..=5 => Some(0),
                    6..=9 => Some(6),
                    10..=13 => Some(10),
                    14..=15 => Some(14),
                    17..=19 => Some(17),
                    21..=25 => Some(21),
                    _ => None,
                };
                if let Some(new_state) = new_state {
                    events.push(KellyOutcomeEvent::UpdateKellyState {
                        player_id: speaker_id,
                        new_state,
                    });
                }
                didsay = true;
            }
            // C `case 7:` (`area3.c:1302-1307`): the "shortcut to caligar"
            // god-only fast-forward.
            TextAnalysisOutcome::Matched(7) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    events.push(KellyOutcomeEvent::UpdateKellyState {
                        player_id: speaker_id,
                        new_state: 19,
                    });
                }
                didsay = true;
            }
            // Every other matched code is unhandled by kelly's own C
            // `switch` but still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`area3.c:1309-1312`) - note this does *not* touch `dat->
        // last_talk` (except the explicit reset inside the `case 2`
        // branch above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `kelly_driver`'s `NT_GIVE` branch (`area3.c:1316-1360`).
    fn kelly_handle_give_message(
        &mut self,
        kelly_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, KellyPlayerFacts>,
        events: &mut Vec<KellyOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&kelly_id)
            .and_then(|kelly| kelly.cursor_item.take())
        else {
            return;
        };
        let Some(template_id) = self.items.get(&item_id).map(|item| item.template_id) else {
            self.destroy_item(item_id);
            return;
        };
        let Some(giver_name) = self.characters.get(&giver_id).map(|c| c.name.clone()) else {
            self.destroy_item(item_id);
            return;
        };
        let kelly_state = player_facts
            .get(&giver_id)
            .map(|facts| facts.kelly_state)
            .unwrap_or(-1);

        if template_id == IID_AREA2_CREEPERHEAD && kelly_state <= 5 {
            // C `if (it[in].ID == IID_AREA2_CREEPERHEAD && ppd->
            // kelly_state <= 5) { quiet_say(cn, "Ah. Well done, %s.");
            // ppd->kelly_state = 6; tmp = questlog_done(co, 13);
            // destroy_item_byID(co, IID_AREA2_CREEPERHEAD); if (tmp == 1) {
            // give_military_pts(cn, co, 4, 1); } destroy_item(ch[cn].
            // citem); ch[cn].citem = 0; }` (`area3.c:1323-1337`).
            self.npc_quiet_say(kelly_id, &format!("Ah. Well done, {giver_name}."));
            events.push(KellyOutcomeEvent::UpdateKellyState {
                player_id: giver_id,
                new_state: 6,
            });
            events.push(KellyOutcomeEvent::CreeperHeadQuestDone {
                player_id: giver_id,
                kelly_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_AREA2_CREEPERHEAD);
            self.destroy_item(item_id);
        } else if template_id == IID_CALIGARPLAQUE && kelly_state == 25 {
            // C `} else if (it[in].ID == IID_CALIGARPLAQUE && ppd->
            // kelly_state == 25) { questlog_done(co, 60); quiet_say(cn,
            // "Oh thank you so much, %s! ..."); give_money(co, 5000*100,
            // "Kelly quest reward"); ppd->kelly_state = 26; destroy_item
            // (ch[cn].citem); ch[cn].citem = 0; }` (`area3.c:1338-1351`).
            events.push(KellyOutcomeEvent::PlaqueQuestDone {
                player_id: giver_id,
            });
            self.npc_quiet_say(
                kelly_id,
                &format!(
                    "Oh thank you so much, {giver_name}! I don't think I can ever repay you for your effort. However, please accept this reward.",
                ),
            );
            events.push(KellyOutcomeEvent::UpdateKellyState {
                player_id: giver_id,
                new_state: 26,
            });
            // C `give_money(co, 5000*100, "Kelly quest reward");`
            // (`area3.c:1345`) - inline gold-add/message, matching every
            // other `GoldEarned` call site's precedent (see the module
            // doc comment).
            const PLAQUE_REWARD_SILVER: u32 = 5000 * 100;
            if let Some(giver) = self.characters.get_mut(&giver_id) {
                giver.gold = giver.gold.saturating_add(PLAQUE_REWARD_SILVER);
                giver.flags.insert(CharacterFlags::ITEMS);
            }
            self.queue_system_text_bytes(giver_id, give_money_message(PLAQUE_REWARD_SILVER));
            events.push(KellyOutcomeEvent::GoldEarned {
                player_id: giver_id,
                amount: PLAQUE_REWARD_SILVER,
            });
            self.destroy_item(item_id);
        } else {
            // C `else { say("Thou hast better use for this than I do.
            // Well, if there is a use for it at all."); if (!give_char_
            // item(co, ch[cn].citem)) destroy_item(ch[cn].citem); ch[cn].
            // citem = 0; }` (`area3.c:1352-1358`).
            self.npc_quiet_say(
                kelly_id,
                "Thou hast better use for this than I do. Well, if there is a use for it at all.",
            );
            if !self.give_char_item(giver_id, item_id) {
                self.destroy_item(item_id);
            }
        }
    }

    /// C `collect_heads` (`area3.c:965-982`): sells every `IID_AREA15_
    /// HEAD` in `player_id`'s main inventory (C's `n = 30; n <
    /// INVENTORYSIZE`) for `125 + drdata[0] * 75` silver each, destroying
    /// the items and crediting the gold directly (both touch only `World`
    /// items/`Character`, not `PlayerRuntime` - see `give_money_message`'s
    /// existing inline-gold-add precedent, e.g. `world::gwendylon`).
    /// Returns `Some(silver)` when at least one head was sold, matching
    /// C's `return 1`/`return 0`; the achievement wealth-ladder half is
    /// left to the caller via [`KellyOutcomeEvent::GoldEarned`].
    fn collect_heads(&mut self, kelly_id: CharacterId, player_id: CharacterId) -> Option<u32> {
        let Some(player) = self.characters.get(&player_id) else {
            return None;
        };
        let head_item_ids: Vec<ItemId> = player
            .inventory
            .iter()
            .skip(INVENTORY_START_INVENTORY)
            .filter_map(|slot| *slot)
            .filter(|item_id| {
                self.items
                    .get(item_id)
                    .is_some_and(|item| item.template_id == IID_AREA15_HEAD)
            })
            .collect();
        if head_item_ids.is_empty() {
            return None;
        }

        let mut count = 0i32;
        let mut sum = 0i64;
        for item_id in head_item_ids {
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            let size = item
                .driver_data
                .get(0..4)
                .map(|bytes| i32::from_le_bytes(bytes.try_into().unwrap()))
                .unwrap_or(0);
            sum += 125 + i64::from(size) * 75;
            count += 1;
            self.destroy_item(item_id);
        }

        self.npc_quiet_say(
            kelly_id,
            &format!("Ah. {count} heads. Here is thy payment."),
        );
        let amount = sum.max(0) as u32;
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.gold = player.gold.saturating_add(amount);
            player.flags.insert(CharacterFlags::ITEMS);
        }
        self.queue_system_text_bytes(player_id, give_money_message(amount));
        Some(amount)
    }

    /// C `has_item(cn, ID)` (`src/system/drvlib.c:2411-2424`): scans every
    /// inventory slot (`0..INVENTORYSIZE`, matching C's full equipment/
    /// spell/main-inventory range) plus the cursor item.
    fn character_has_item_template(&self, character_id: CharacterId, template_id: u32) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        character
            .inventory
            .iter()
            .filter_map(|slot| *slot)
            .chain(character.cursor_item)
            .any(|item_id| {
                self.items
                    .get(&item_id)
                    .is_some_and(|item| item.template_id == template_id)
            })
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_KELLY;
use crate::entity::Character;
use crate::ids::ItemId;
use crate::legacy::INVENTORY_START_INVENTORY;

/// C `struct kelly_driver_data` (`src/area/3/area3.c:960-963`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KellyDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
