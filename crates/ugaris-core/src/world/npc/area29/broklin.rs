//! Broklin NPC (`CDR_BROKLIN`), Brannington's Chief Miner, who runs "The
//! Missing Pickaxe"/"The Head Robber" quest chain (quests 45/46) and, once
//! both are complete, offers a permanent gold<->silver trade service.
//!
//! Ports `src/area/29/brannington.c::broklin_driver` (`:2118-2395`) plus
//! `broklin_trade_gold`/`broklin_trade_silver` (`:2029-2116`) and the
//! shared `analyse_text_driver`/`qa[]` table (`:86-206`, ported as
//! [`super::AREA29_QA`] in `world::npc::area29`, the same table every other
//! `brannington.c` NPC driver shares - `AREA29_QA`'s own doc comment already
//! called out its two extra area-29-only entries, `4`/`5` ("thousand gold"/
//! "five thousand silver"), as feeding this driver specifically). Follows
//! the same `World`/`PlayerRuntime` split established by `world::npc::
//! area29::brennethbran`/`countbran`: the caller supplies a per-player fact
//! snapshot ([`BroklinPlayerFacts`]) up front and applies the returned
//! [`BroklinOutcomeEvent`]s afterwards, since `staffer_ppd.broklin_state`
//! and the `QLOG` 45/46 quest-log entries live on `crate::player::
//! PlayerRuntime`, not `World`. Quest 46's own completion is a *different*
//! hook entirely - `robberboss_dead` (`world_events::death_hooks::
//! apply_robberboss_death_from_hurt_event`) already fast-forwards
//! `broklin_state` from `5..=10` to `11` and marks quest 46 done on the
//! White Robber Boss's death - this driver only opens quest 46 and reacts
//! to that state jump.
//!
//! Unlike every other `brannington.c` NPC ported so far, `has_item`/
//! inventory-scan checks (the sewer-key giveaway, the gold<->silver
//! conversion scan) touch only live `World`-owned data (`Character::
//! inventory`, `Item::driver_data`), so those checks and their
//! decrement/destroy side effects run directly inside `World` with no
//! outcome event at all - only the *reward* item (the new key, or the
//! swapped currency stack) needs an outcome event, since creating a brand
//! new item instance requires the zone loader's item-template table, which
//! `World` cannot see (same reason `world::npc::area28::aristocrat`'s gold
//! reward needs one).
//!
//! `broklin_driver`'s nineteen-state (`0`-`18`) dialogue chain: greeting
//! (opens quest 45) -> "robbers stole my pickaxe" -> "you look capable" ->
//! "a shopkeeper is involved" -> (waiting: state `4`) -> (`NT_GIVE`: hand in
//! `IID_STAFF_PICKAXE`, quest 45 done, destroy `IID_STAFF_ROBBERKEYAREA1`
//! too, first completion only grants 2,000g, state jumps to `5`) -> if
//! quest 46 is already done, fast-forward straight to `11`; else "impose
//! more" (opens quest 46) -> "kill the head robber" -> "find his
//! whereabouts" -> (grant `WS_Robber_Key_Area2`/`IID_STAFF_SEWERKEY` unless
//! already carried) -> "begin your search" -> (waiting for `robberboss_
//! dead`'s external state jump to `11`: state `10`) -> "trade rates" ->
//! "1,000 gold for 5,000 silver" -> "4,000 silver for 1,000 gold" -> "which
//! wouldst thou like to trade?" -> (waiting: state `15`) -> "hail again" ->
//! "which wouldst thou like to trade?" (repeat) -> done (state `18`).
//!
//! Deviations/gaps (documented, not silent):
//! - Like `world::npc::area29::brennethbran`/`spiritbran`'s own `NT_TEXT`
//!   branch, this driver's own C body has no `dat->current_victim`
//!   staleness-reset preamble and no victim-mismatch early-out at all -
//!   reproduced verbatim: replies to *any* nearby player's matched small
//!   talk, not just its tracked victim.
//! - C `case 2:` (`:2302-2315`) resets to whichever of the three dialogue
//!   spans the player is currently mid-way through (three range checks:
//!   `0..=4` -> `0`, `5..=10` -> `5`, `11..=19` -> `16`), ported as
//!   [`BroklinOutcomeEvent::ResetToMiniQuestStart`] with the resolved
//!   target state computed in `World` itself, same shape as `brennethbran`'s
//!   own three-way reset.
//! - C `case 3:` (`:2316-2321`) speaks a visible `say(cn, "reset done")`
//!   line (not `quiet_say`) before wiping the state to `0` - only if the
//!   speaker is `CF_GOD`, matching `world::npc::area29::brennethbran`'s own
//!   `case 3` precedent exactly.
//! - `broklin_trade_gold`/`broklin_trade_silver`'s own "here you go"/"you
//!   need to have..." lines use C `say` (not `quiet_say`), reproduced with
//!   [`crate::world::World::npc_say`].
//! - No self-defense/regen/spell-self cascade exists in C's `broklin_
//!   driver` body at all (matching every other `brannington.c` "pure
//!   talker" NPC) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:2394`) is not
//!   ported, matching the established `world::npc::area29::brennethbran`/
//!   `spiritbran` precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:2167`).
const BROKLIN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const BROKLIN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:2150`).
const BROKLIN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:2155`).
const BROKLIN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:2388`): idle "return to post" threshold.
const BROKLIN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `broklin_trade_gold`'s `1000` gold-unit price (`brannington.c:2046`).
const BROKLIN_TRADE_GOLD_COST: u32 = 1000;
/// C `broklin_trade_silver`'s `5000` silver-unit price (`brannington.c:2092`).
const BROKLIN_TRADE_SILVER_COST: u32 = 5000;
/// C `it[in].drdata[0] == 2` gold marker (`brannington.c:2047`, matching
/// `gold_*.itm`'s `arg="02..."` template byte).
const ENHANCE_KIND_GOLD: u8 = 2;
/// C `it[in].drdata[0] == 1` silver marker (`brannington.c:2091`, matching
/// `silver_*.itm`'s `arg="01..."` template byte).
const ENHANCE_KIND_SILVER: u8 = 1;
/// C `it[in].value = (*(unsigned int*)(it[in].drdata+1)) * 25` for a gold
/// stack (`brannington.c:2056`, matching `gold_1000`'s `25000 / 1000`).
const ENHANCE_GOLD_VALUE_PER_UNIT: u32 = 25;
/// C `it[in].value = (*(unsigned int*)(it[in].drdata+1)) * 10` for a silver
/// stack (`brannington.c:2100`, matching `silver_4000`'s `40000 / 4000`).
const ENHANCE_SILVER_VALUE_PER_UNIT: u32 = 10;

/// Per-player facts [`World::process_broklin_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BroklinPlayerFacts {
    /// `PlayerRuntime::staffer_broklin_state()`.
    pub broklin_state: i32,
    /// `PlayerRuntime::quest_log.is_done(46)` (C `questlog_isdone(co, 46)`,
    /// `brannington.c:2205`): `case 5`'s fast-forward guard.
    pub quest46_is_done: bool,
}

/// Which currency stack `broklin_trade_gold`/`broklin_trade_silver` handed
/// out (C `create_item("silver_4000")`/`create_item("gold_1000")`,
/// `brannington.c:2061`/`2105`) - resolved to a template name by
/// `ugaris-server` (same split as `world::npc::area28::yoatin`'s vault-key
/// reward).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BroklinTradeReward {
    /// C `create_item("silver_4000")` (`brannington.c:2061`): the payout
    /// for trading in 1,000 gold units.
    Silver4000,
    /// C `create_item("gold_1000")` (`brannington.c:2105`): the payout for
    /// trading in 5,000 silver units.
    Gold1000,
}

/// A side effect [`World::process_broklin_actions`] could not apply
/// directly because it touches `PlayerRuntime`, or because it needs the
/// zone loader's item-template table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BroklinOutcomeEvent {
    /// Write the new `staffer_ppd.broklin_state` back.
    UpdateBroklinState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 45)` (`brannington.c:2180`).
    QuestOpen45 { player_id: CharacterId },
    /// C `questlog_open(co, 46)` (`brannington.c:2210`).
    QuestOpen46 { player_id: CharacterId },
    /// C `tmp = questlog_done(co, 45); ... if (tmp == 1 && (in =
    /// create_item("gold_2000"))) { give_char_item(co, in); }`
    /// (`brannington.c:2346-2361`) - `ugaris-server` decides the "Thank
    /// you! Take these 2,000 gu..." vs plain "Thank you!" reply, since only
    /// it knows `completion.times_done` after calling `complete_legacy`;
    /// `broklin_id` lets it speak through the right NPC.
    QuestDonePickaxe {
        player_id: CharacterId,
        broklin_id: CharacterId,
    },
    /// C `case 2:` (`brannington.c:2302-2315`): reset back to the start of
    /// whichever of the three dialogue spans the player is currently
    /// mid-way through. `new_state` is already resolved to the target
    /// state.
    ResetToMiniQuestStart {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `case 3:` (`brannington.c:2316-2321`): the god-only "reset me"
    /// full state wipe.
    ResetBroklin { player_id: CharacterId },
    /// C `case 8:`'s `!has_item(co, IID_STAFF_SEWERKEY)` branch
    /// (`brannington.c:2225-2232`): `World` has already confirmed the
    /// player doesn't carry the key; `ugaris-server` instantiates `WS_
    /// Robber_Key_Area2` and gives it.
    GrantSewerKey { player_id: CharacterId },
    /// C `broklin_trade_gold`/`broklin_trade_silver`'s `create_item(...)`
    /// call (`brannington.c:2061`/`2105`): `World` has already decremented
    /// (or destroyed) the matching currency stack; `ugaris-server`
    /// instantiates the reward and gives it.
    GrantTradeReward {
        player_id: CharacterId,
        reward: BroklinTradeReward,
    },
}

impl World {
    /// C `broklin_driver`'s per-tick body (`brannington.c:2118-2395`).
    pub fn process_broklin_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, BroklinPlayerFacts>,
        area_id: u16,
    ) -> Vec<BroklinOutcomeEvent> {
        let broklin_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_BROKLIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for broklin_id in broklin_ids {
            self.process_broklin_messages(broklin_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_broklin_messages(
        &mut self,
        broklin_id: CharacterId,
        player_facts: &HashMap<CharacterId, BroklinPlayerFacts>,
        area_id: u16,
        events: &mut Vec<BroklinOutcomeEvent>,
    ) {
        let Some(broklin_name) = self
            .characters
            .get(&broklin_id)
            .map(|broklin| broklin.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Broklin(mut data)) = self
            .characters
            .get(&broklin_id)
            .and_then(|broklin| broklin.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&broklin_id)
            .map(|broklin| std::mem::take(&mut broklin.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.broklin_handle_char_message(
                    broklin_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.broklin_handle_text_message(
                    broklin_id,
                    &broklin_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.broklin_handle_give_message(
                    broklin_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                _ => {}
            }
        }

        if let Some(broklin) = self.characters.get_mut(&broklin_id) {
            broklin.driver_state = Some(CharacterDriverState::Broklin(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:2384-2386`).
        if let (Some(broklin), Some((tx, ty))) =
            (self.characters.get(&broklin_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(broklin.x), i32::from(broklin.y), tx, ty) {
                if let Some(broklin_mut) = self.characters.get_mut(&broklin_id) {
                    let _ = turn(broklin_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`brannington.c:2388-2392`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::brennethbran` already uses.
        let last_talk = if let Some(broklin) = self.characters.get(&broklin_id) {
            match broklin.driver_state.as_ref() {
                Some(CharacterDriverState::Broklin(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + BROKLIN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(broklin) = self.characters.get(&broklin_id) else {
                return;
            };
            let (post_x, post_y) = (broklin.rest_x, broklin.rest_y);
            self.secure_move_driver(
                broklin_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `broklin_driver`'s `NT_CHAR` branch (`brannington.c:2133-2293`).
    #[allow(clippy::too_many_arguments)]
    fn broklin_handle_char_message(
        &mut self,
        broklin_id: CharacterId,
        data: &mut BroklinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, BroklinPlayerFacts>,
        events: &mut Vec<BroklinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(broklin) = self.characters.get(&broklin_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:2137-2141`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:2143-2147`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:2149-2153`).
        if tick < data.last_talk + BROKLIN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:2155-2158`).
        if tick < data.last_talk + BROKLIN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:2160-2164`).
        if broklin_id == player_id
            || !char_see_char(&broklin, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:2166-
        // 2170`).
        if char_dist(&broklin, &player) > BROKLIN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.broklin_state;
        match facts.broklin_state {
            // C `case 0:` (`brannington.c:2177-2183`).
            0 => {
                self.npc_quiet_say(
                    broklin_id,
                    "Greetings stranger! I am Broklin, the Chief Miner. I supply the town with all its plating materials.",
                );
                events.push(BroklinOutcomeEvent::QuestOpen45 { player_id });
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`brannington.c:2184-2189`).
            1 => {
                self.npc_quiet_say(
                    broklin_id,
                    "Well, I did before those damnable robbers broke into my house and stole my favorite pickaxe.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`brannington.c:2190-2195`).
            2 => {
                self.npc_quiet_say(
                    broklin_id,
                    "I wonder... you look like a capable adventurer - mayhaps you could take a trip to visit these robbers?",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`brannington.c:2196-2200`).
            3 => {
                self.npc_quiet_say(
                    broklin_id,
                    "I suspect that one of the towns shopkeepers is involved with the robbers.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4: break;` (`brannington.c:2201-2202`): waiting for
            // the pickaxe.
            4 => {}
            // C `case 5:` (`brannington.c:2204-2213`).
            5 => {
                if facts.quest46_is_done {
                    new_state = 11;
                } else {
                    self.npc_quiet_say(
                        broklin_id,
                        "Could I impose on your services some more? I would reward thee further.",
                    );
                    events.push(BroklinOutcomeEvent::QuestOpen46 { player_id });
                    new_state = 6;
                    didsay = true;
                }
            }
            // C `case 6:` (`brannington.c:2214-2218`).
            6 => {
                self.npc_quiet_say(
                    broklin_id,
                    "The robbers infest this city...perhaps you could kill the head robber.",
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7:` (`brannington.c:2219-2223`).
            7 => {
                self.npc_quiet_say(
                    broklin_id,
                    "I hear his whereabouts is a great secret, but maybe you could find him.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8:` (`brannington.c:2224-2238`).
            8 => {
                if self.character_has_item_template(player_id, IID_STAFF_SEWERKEY) {
                    self.npc_quiet_say(broklin_id, "You already have the key for the sewers...");
                } else {
                    self.npc_quiet_say(
                        broklin_id,
                        "This key lets you enter the sewers under the town.",
                    );
                    events.push(BroklinOutcomeEvent::GrantSewerKey { player_id });
                }
                new_state = 9;
                didsay = true;
            }
            // C `case 9:` (`brannington.c:2239-2243`).
            9 => {
                self.npc_quiet_say(broklin_id, "You can begin your search there!");
                new_state = 10;
                didsay = true;
            }
            // C `case 10: break;` (`brannington.c:2244-2245`): waiting for
            // `robberboss_dead`'s external state jump to `11`.
            10 => {}
            // C `case 11:` (`brannington.c:2247-2252`).
            11 => {
                self.npc_quiet_say(
                    broklin_id,
                    "If you ever have need of silver or gold for plating, I can offer reasonable trade rates.",
                );
                new_state = 12;
                didsay = true;
            }
            // C `case 12:` (`brannington.c:2253-2257`).
            12 => {
                self.npc_quiet_say(
                    broklin_id,
                    "I can offer 1,000 gold units for 5,000 silver units.",
                );
                new_state = 13;
                didsay = true;
            }
            // C `case 13:` (`brannington.c:2258-2262`).
            13 => {
                self.npc_quiet_say(
                    broklin_id,
                    "Or, I can offer 4,000 silver units for 1,000 gold units.",
                );
                new_state = 14;
                didsay = true;
            }
            // C `case 14:` (`brannington.c:2263-2269`).
            14 => {
                self.npc_quiet_say(
                    broklin_id,
                    "Which wouldst thou like to trade? thousand gold for 4000 silver or five thousand silver for 1000 gold?",
                );
                new_state = 15;
                didsay = true;
            }
            // C `case 15: break;` (`brannington.c:2270-2271`): waiting for
            // repeat.
            15 => {}
            // C `case 16:` (`brannington.c:2272-2276`).
            16 => {
                self.npc_quiet_say(
                    broklin_id,
                    &format!("Hail {}! Nice to see you again.", player.name),
                );
                new_state = 17;
                didsay = true;
            }
            // C `case 17:` (`brannington.c:2277-2283`).
            17 => {
                self.npc_quiet_say(
                    broklin_id,
                    "Which wouldst thou like to trade? thousand gold for 4000 silver or five thousand silver for 1000 gold?",
                );
                new_state = 18;
                didsay = true;
            }
            // C `case 18: break;` (`brannington.c:2284-2285`): all done.
            18 => {}
            _ => {}
        }

        if new_state != facts.broklin_state {
            events.push(BroklinOutcomeEvent::UpdateBroklinState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:2287-2291`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `broklin_driver`'s `NT_TEXT` branch (`brannington.c:2296-2334`),
    /// wired through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::npc::area29::brennethbran`'s text handler). This branch has
    /// no victim-staleness-reset preamble and no victim-mismatch early-out
    /// (see the module doc comment).
    #[allow(clippy::too_many_arguments)]
    fn broklin_handle_text_message(
        &mut self,
        broklin_id: CharacterId,
        broklin_name: &str,
        data: &mut BroklinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, BroklinPlayerFacts>,
        events: &mut Vec<BroklinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:2299`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if broklin_id == speaker_id {
            return;
        }
        let Some(broklin) = self.characters.get(&broklin_id).cloned() else {
            return;
        };
        if char_dist(&broklin, &speaker) > BROKLIN_QA_DISTANCE
            || !char_see_char(&broklin, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let broklin_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.broklin_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, broklin_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(broklin_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:2302-2315`): reset back to the
            // start of whichever of the three dialogue spans is in
            // progress.
            TextAnalysisOutcome::Matched(2) => {
                let new_state = if (0..=4).contains(&broklin_state) {
                    Some(0)
                } else if (5..=10).contains(&broklin_state) {
                    Some(5)
                } else if (11..=19).contains(&broklin_state) {
                    Some(16)
                } else {
                    None
                };
                if let Some(new_state) = new_state {
                    data.last_talk = 0;
                    events.push(BroklinOutcomeEvent::ResetToMiniQuestStart {
                        player_id: speaker_id,
                        new_state,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`brannington.c:2316-2321`): the god-only "reset
            // me" wipe, which speaks a visible `say(cn, "reset done")` line
            // first.
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(broklin_id, "reset done");
                    events.push(BroklinOutcomeEvent::ResetBroklin {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // C `case 4:` (`brannington.c:2322-2324`): "thousand gold" ->
            // trade 1,000 gold units for a `silver_4000` stack.
            TextAnalysisOutcome::Matched(4) => {
                self.broklin_trade(
                    broklin_id,
                    speaker_id,
                    broklin_state,
                    ENHANCE_KIND_GOLD,
                    BROKLIN_TRADE_GOLD_COST,
                    ENHANCE_GOLD_VALUE_PER_UNIT,
                    BroklinTradeReward::Silver4000,
                    events,
                );
                didsay = true;
            }
            // C `case 5:` (`brannington.c:2325-2327`): "five thousand
            // silver" -> trade 5,000 silver units for a `gold_1000` stack.
            TextAnalysisOutcome::Matched(5) => {
                self.broklin_trade(
                    broklin_id,
                    speaker_id,
                    broklin_state,
                    ENHANCE_KIND_SILVER,
                    BROKLIN_TRADE_SILVER_COST,
                    ENHANCE_SILVER_VALUE_PER_UNIT,
                    BroklinTradeReward::Gold1000,
                    events,
                );
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington.c:2329-2332`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `broklin_trade_gold`/`broklin_trade_silver` (`brannington.c:2029-
    /// 2116`) - the two functions are byte-for-byte identical shapes save
    /// for which `IDR_ENHANCE` kind byte they look for, how much they
    /// consume, the per-unit `value` recompute, and which reward template
    /// they hand out, so this one generic body parametrizes over all four.
    #[allow(clippy::too_many_arguments)]
    fn broklin_trade(
        &mut self,
        broklin_id: CharacterId,
        player_id: CharacterId,
        broklin_state: i32,
        kind: u8,
        cost: u32,
        value_per_unit: u32,
        reward: BroklinTradeReward,
        events: &mut Vec<BroklinOutcomeEvent>,
    ) {
        // C `if (ppd->broklin_state < 11) return;` (`brannington.c:2037`/
        // `2081`).
        if broklin_state < 11 {
            return;
        }
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };
        // C `if (ch[co].citem) { say(cn, "Please free your hand (mouse
        // cursor) first."); return; }` (`brannington.c:2041-2044`/`2085-
        // 2088`).
        if player.cursor_item.is_some() {
            self.npc_say(broklin_id, "Please free your hand (mouse cursor) first.");
            return;
        }

        // C `for (n = 30; n < INVENTORYSIZE; n++) { if ((in =
        // ch[co].item[n]) && it[in].driver == IDR_ENHANCE && it[in].
        // drdata[0] == kind) { if (amount < cost) continue; ... } }`
        // (`brannington.c:2046-2069`/`2090-2113`).
        let stack_item_id = player.inventory[30..].iter().find_map(|slot| {
            let item_id = (*slot)?;
            let item = self.items.get(&item_id)?;
            if item.driver == IDR_ENHANCE
                && item.driver_data.first().copied() == Some(kind)
                && enhance_amount(item) >= cost
            {
                Some(item_id)
            } else {
                None
            }
        });

        let Some(item_id) = stack_item_id else {
            let kind_name = if kind == ENHANCE_KIND_GOLD {
                "gold"
            } else {
                "silver"
            };
            self.npc_say(
                broklin_id,
                &format!(
                    "You need to have {cost} {kind_name} units in a single spot in your inventory, {}.",
                    player.name
                ),
            );
            return;
        };

        let remaining = self
            .items
            .get(&item_id)
            .map(|item| enhance_amount(item) - cost)
            .unwrap_or(0);
        if remaining > 0 {
            if let Some(item) = self.items.get_mut(&item_id) {
                set_enhance_amount(item, remaining);
                item.description = format!("{remaining} units of {}.", item.name);
                item.value = remaining * value_per_unit;
            }
        } else {
            self.destroy_item(item_id);
        }

        self.npc_say(broklin_id, &format!("Here you go, {}.", player.name));
        events.push(BroklinOutcomeEvent::GrantTradeReward { player_id, reward });
    }

    /// C `broklin_driver`'s `NT_GIVE` branch (`brannington.c:2337-2376`).
    #[allow(clippy::too_many_arguments)]
    fn broklin_handle_give_message(
        &mut self,
        broklin_id: CharacterId,
        data: &mut BroklinDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, BroklinPlayerFacts>,
        events: &mut Vec<BroklinOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&broklin_id)
            .and_then(|broklin| broklin.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let facts = player_facts.get(&giver_id).copied();
        let tick = self.tick.0;

        // C `if (it[in].ID == IID_STAFF_PICKAXE && ppd &&
        // ppd->broklin_state <= 4)` (`brannington.c:2344`).
        if item.template_id == IID_STAFF_PICKAXE
            && facts.is_some_and(|facts| facts.broklin_state <= 4)
        {
            events.push(BroklinOutcomeEvent::QuestDonePickaxe {
                player_id: giver_id,
                broklin_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_STAFF_PICKAXE);
            self.destroy_items_by_template_id(giver_id, IID_STAFF_ROBBERKEYAREA1);
            events.push(BroklinOutcomeEvent::UpdateBroklinState {
                player_id: giver_id,
                new_state: 5,
            });
            data.last_talk = tick;
            *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            data.current_victim = Some(giver_id);
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`brannington.c:2362-2367`): hand the
        // item back to the giver.
        self.npc_quiet_say(
            broklin_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

/// C `*(unsigned int *)(it[in].drdata + 1)` (`brannington.c:2048` et al.):
/// the little-endian unit count stored right after the kind byte.
fn enhance_amount(item: &Item) -> u32 {
    item.driver_data
        .get(1..5)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0)
}

/// Writes back the unit count `enhance_amount` reads, growing `driver_data`
/// if the template somehow shipped a short buffer.
fn set_enhance_amount(item: &mut Item, amount: u32) {
    if item.driver_data.len() < 5 {
        item.driver_data.resize(5, 0);
    }
    item.driver_data[1..5].copy_from_slice(&amount.to_le_bytes());
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_BROKLIN, CDR_LOSTCON};
use crate::item_driver::{
    IDR_ENHANCE, IID_STAFF_PICKAXE, IID_STAFF_ROBBERKEYAREA1, IID_STAFF_SEWERKEY,
};

/// C `struct broklin_data` (`src/area/29/brannington.c:2024-2027`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BroklinDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
