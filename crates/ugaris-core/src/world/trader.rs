//! `CDR_TRADER` player-to-player trade middleman NPC.
//!
//! Ports `src/module/base.c`'s `trader_driver`: the "trade with <name>"/
//! "stop trade"/"accept trade"/"show trade" text-command state machine,
//! `NT_GIVE` item collection (up to 10 items per side), the three-minute
//! timeout, the generic `analyse_text_driver` small talk (`TRADER_QA`),
//! the greeting (ported as the same periodic nearby-player scan
//! `world/merchant.rs`/`world/bank.rs` already established instead of
//! reacting to `NT_CHAR` notify-area broadcasts), the idle-murmur table,
//! and the 12h driver-memory clear timer.
//!
//! Two things are deferred to [`TraderEvent`]/[`World::drain_pending_trader_events`]
//! because they need the `legacy_item_look_text` item-look formatter,
//! which lives in the `ugaris-server` crate, not here (see that crate's
//! `world_events.rs::apply_trader_events`): the "show trade" item dump
//! and the `NT_GIVE` "`<name>` gave me:" cross-notification to the other
//! trading partner.
//!
//! Deviations from C (documented here, not silent):
//! - `dat->c1ID`/`c2ID` (`ch[co].ID`, a player's persistent ID) are
//!   represented as the raw runtime [`CharacterId`] instead - the same
//!   simplification already established for driver-memory membership and
//!   the merchant/bank greet-tracking ports (see `character_driver.rs`'s
//!   `DriverMemory` doc comment).
//! - `find_char_byname`'s C iteration order is the character-table slot
//!   order (`getfirst_char`/`getnext_char`); `World::characters` is a
//!   `HashMap` with no such order, so ties (two players with the same
//!   name would already be impossible; case-insensitive duplicates are
//!   not) are broken by sorting on `CharacterId` for determinism.
//! - `is_gk_room(c2)` (`return_items`'s gatekeeper-room guard) is not
//!   replicated - the gatekeeper NPC/lab room concept is not ported yet
//!   (see `PORTING_TODO.md`'s "Gatekeeper NPC" task).
//! - The successful "Deal." branch's `achievement_award(...,
//!   ACHIEVEMENT_TRUST_BUT_VERIFY, 1)` calls are queued as
//!   [`TraderEvent::DealCompleted`] and applied in `ugaris-server` (see
//!   that crate's `world_events.rs::apply_trader_events` and
//!   `achievement.rs::award_trader_deal_achievement`), since awarding
//!   needs `ServerRuntime`'s `PlayerRuntime` map.
//! - `give_char_item`'s `dlog(cn, in, "was given %s from NPC", ...)` audit
//!   log line is not replicated (no generic "was given from NPC" audit
//!   log path exists yet, matching precedent elsewhere in this codebase).
//! - COL_LIGHT_BLUE/COL_LIGHT_GREEN/COL_RESET color markers around
//!   "help"/"accept trade"/"stop trade"/"show trade" keywords and the
//!   "gave me:" notice are dropped (same simplification `world/bank.rs`
//!   already made: the legacy color marker is a raw non-UTF8 byte that
//!   cannot round-trip through a plain Rust `&str` literal) - wording
//!   stays byte-for-byte identical otherwise.
use super::*;
use crate::character_driver::{
    mem_add_driver, mem_check_driver, mem_erase_driver, TraderDriverData, TRADER_QA,
};
use crate::drvlib::offset2dx;
use crate::item_ops::count_free_inventory_slots;

const TRADER_GREET_DISTANCE: i32 = 10;
const TRADER_QA_DISTANCE: i32 = 12;
/// C `TICKS * 60 * 3` (`base.c:4365`): three-minute trade timeout.
const TRADER_TIMEOUT_TICKS: u64 = TICKS_PER_SECOND * 60 * 3;
const TRADER_MEMORY_CLEAR_TICKS: u64 = TICKS_PER_SECOND * 60 * 60 * 12;
/// C `TICKS * 60` in `trader_driver`'s idle-murmur throttle (`base.c:4555`).
const TRADER_TALK_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `mem_add_driver(cn, co, 7)`/`mem_check_driver(cn, co, 7)` in
/// `trader_driver`'s greeting handler (`base.c:4280,4290`).
const TRADER_GREET_MEMORY_SLOT: usize = 7;
/// C `dat->c1itm[10]`/`c2itm[10]`: max items per side of a trade.
const MAX_TRADER_ITEMS: usize = 10;

/// C `static const char *trader_mutterings[]` (`base.c:4556-4569`).
const TRADER_MUTTERINGS: [&str; 12] = [
    "Trust is the currency of trade. Well, that and actual currency.",
    "Another day, another deal. Or not. Some days are slow.",
    "I've seen things traded that would make your head spin.",
    "No, I do NOT take a cut. ...officially.",
    "The art of the deal: make both sides think they won.",
    "Someone once tried to trade a rock for a sword. I admired the audacity.",
    "Read the fine print. Actually, there is no fine print. Just trust me.",
    "I wonder if I should start charging for my services...",
    "Fair deals only. Unfair deals require an appointment.",
    "Some people take forever to accept. I'm not getting any younger here.",
    "Trade with confidence! Or at least fake it convincingly.",
    "Best middleman in the land. Also the only middleman. Coincidence? I think not.",
];

/// A `trader_driver` outcome that needs `legacy_item_look_text`
/// (`ugaris-server` crate) to finish formatting. See the module doc
/// comment for why this split exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraderEvent {
    /// C `trader_driver`'s "show trade" text command (`base.c:443-465`):
    /// dumps both sides' held items to the requesting player via
    /// `log_char(co, LOG_SYSTEM, 0, "Trading:")`/`"For:"` + `look_item`
    /// per item.
    ShowTrade {
        viewer_id: CharacterId,
        c1_items: Vec<ItemId>,
        c2_items: Vec<ItemId>,
    },
    /// C `trader_driver`'s `NT_GIVE` success branches (`base.c:496-523`):
    /// notifies the *other* trading partner that an item was added to
    /// the deal.
    ItemAddedToTrade {
        notify_id: CharacterId,
        giver_name: String,
        item_id: ItemId,
    },
    /// C `trader_driver`'s "accept trade" success branch (`base.c:4416-
    /// 4428`): once both sides have accepted, `achievement_award(c1,
    /// ACHIEVEMENT_TRUST_BUT_VERIFY, 1)`/`achievement_award(c2_trader,
    /// ACHIEVEMENT_TRUST_BUT_VERIFY, 1)` fire for both traders. Deferred
    /// to `ugaris-server` because awarding needs `ServerRuntime`'s
    /// `PlayerRuntime` map, which `ugaris-core` doesn't have access to
    /// (same reason `world/death.rs`'s `KillAchievementAward` queue
    /// exists).
    DealCompleted {
        c1_id: CharacterId,
        c2_id: CharacterId,
    },
}

impl World {
    pub fn drain_pending_trader_events(&mut self) -> Vec<TraderEvent> {
        self.pending_trader_events.drain(..).collect()
    }

    /// C `find_char_byname` (`base.c:4189-4201`): first `CF_PLAYER`
    /// character whose name case-insensitively matches. See the module
    /// doc comment for the iteration-order caveat.
    fn find_trader_char_by_name(&self, name: &str) -> Option<CharacterId> {
        let mut candidates: Vec<&Character> = self
            .characters
            .values()
            .filter(|character| {
                character.flags.contains(CharacterFlags::PLAYER)
                    && character.name.eq_ignore_ascii_case(name)
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);
        candidates.first().map(|character| character.id)
    }

    /// C `trader_driver`'s `NT_GIVE` early-exit paths (`base.c:4475-4478`,
    /// `4484-4487`): `if (ch[cn].citem && !give_char_item(co,
    /// ch[cn].citem)) destroy_item(...); ch[cn].citem = 0;` - try handing
    /// the trader's held cursor item back to `target_id`, destroying it
    /// on failure. Uses the shared `World::give_char_item` (the plain,
    /// non-"smart" give C itself calls here).
    fn trader_return_or_destroy_cursor_item(
        &mut self,
        trader_id: CharacterId,
        target_id: CharacterId,
    ) {
        let Some(item_id) = self
            .characters
            .get_mut(&trader_id)
            .and_then(|trader| trader.cursor_item.take())
        else {
            return;
        };
        if !self.give_char_item(target_id, item_id) {
            self.destroy_item(item_id);
        }
    }

    /// C `return_items(dat, switched)` (`base.c:4214-4246`): returns each
    /// side's collected items, clearing `IF_VOID`; `switched` swaps the
    /// recipients (used only by the successful "accept trade" deal).
    fn trader_return_items(&mut self, data: &mut TraderDriverData, switched: bool) {
        let c1_target = if switched { data.c2_id } else { data.c1_id };
        for item_id in data.c1_items.drain(..) {
            if let Some(item) = self.items.get_mut(&item_id) {
                item.flags.remove(ItemFlags::VOID);
            }
            let given = c1_target.is_some_and(|target_id| self.give_char_item(target_id, item_id));
            if !given {
                self.destroy_item(item_id);
            }
        }

        let c2_target = if switched { data.c1_id } else { data.c2_id };
        for item_id in data.c2_items.drain(..) {
            if let Some(item) = self.items.get_mut(&item_id) {
                item.flags.remove(ItemFlags::VOID);
            }
            let given = c2_target.is_some_and(|target_id| self.give_char_item(target_id, item_id));
            if !given {
                self.destroy_item(item_id);
            }
        }
    }

    /// C `analyse_text_driver` from `src/module/base.c`, wired through the
    /// generic [`crate::character_driver::analyse_text_qa`] matcher (same
    /// pattern as `world/bank.rs::bank_qa_reply`).
    fn trader_qa_reply(
        &self,
        trader_id: CharacterId,
        trader_name: &str,
        speaker: &Character,
        text: &str,
    ) -> Option<String> {
        let trader = self.characters.get(&trader_id)?;
        if char_dist(trader, speaker) > TRADER_QA_DISTANCE {
            return None;
        }
        if !char_see_char(trader, speaker, &self.map, self.date.daylight) {
            return None;
        }
        match crate::character_driver::analyse_text_qa(text, trader_name, &speaker.name, TRADER_QA)
        {
            crate::character_driver::TextAnalysisOutcome::Said(reply) => Some(reply),
            // C: `answer_code == 1` -> `quiet_say(cn, "I'm %s.", ch[cn].name)`.
            crate::character_driver::TextAnalysisOutcome::Matched(1) => {
                Some(format!("I'm {trader_name}."))
            }
            _ => None,
        }
    }

    /// C `trader_driver`'s `NT_TEXT` branch (`base.c:4293-4466`).
    fn trader_handle_text_message(
        &mut self,
        trader_id: CharacterId,
        trader_name: &str,
        data: &mut TraderDriverData,
        message: &CharacterDriverMessage,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        if speaker_id == trader_id {
            return;
        }
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(text) = message.text.as_deref() else {
            return;
        };
        let speaker_pos = (i32::from(speaker.x), i32::from(speaker.y));

        // Generic small talk (`analyse_text_driver`'s `qa[]` table). This
        // guard-clauses independently of the trade-command parsing below
        // (C never gates the latter on distance/visibility).
        if let Some(reply) = self.trader_qa_reply(trader_id, trader_name, &speaker, text) {
            self.npc_quiet_say(trader_id, &reply);
            *face_target = Some(speaker_pos);
        }

        // C `continue`s the outer message loop from several early-exit
        // branches below, skipping the remaining "stop trade"/"accept
        // trade"/"show trade" checks for *this* message; a labeled block
        // reproduces that with `break 'commands`.
        'commands: {
            if let Some(pos) = text.find("trade with") {
                if data.state != 0 {
                    self.npc_quiet_say(trader_id, "Sorry, I am busy.");
                    *face_target = Some(speaker_pos);
                    break 'commands;
                }
                let after = text[pos + "trade with".len()..].trim_start();
                let name: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_alphabetic())
                    .take(39)
                    .collect();
                let Some(c2_id) = self.find_trader_char_by_name(&name) else {
                    self.npc_quiet_say(
                        trader_id,
                        &format!("Sorry, {name} does not seem to be around."),
                    );
                    *face_target = Some(speaker_pos);
                    break 'commands;
                };
                let Some(c2) = self.characters.get(&c2_id).cloned() else {
                    break 'commands;
                };
                if count_free_inventory_slots(&speaker) < 10 {
                    self.npc_quiet_say(
                        trader_id,
                        &format!(
                            "Sorry, your inventory is too filled to trade, {}.",
                            speaker.name
                        ),
                    );
                    *face_target = Some(speaker_pos);
                    break 'commands;
                }
                if count_free_inventory_slots(&c2) < 10 {
                    self.npc_quiet_say(
                        trader_id,
                        &format!(
                            "Sorry, {}'s inventory is too filled to trade, {}.",
                            c2.name, speaker.name
                        ),
                    );
                    *face_target = Some(speaker_pos);
                    break 'commands;
                }
                data.state = 1;
                data.c1_id = Some(speaker_id);
                data.c2_id = Some(c2_id);
                data.timeout = self.tick.0 + TRADER_TIMEOUT_TICKS;
                self.npc_quiet_say(
                    trader_id,
                    &format!(
                        "I will handle a trade between {} and {}. You have three minutes to \
                         complete it. When you are satisfied with the deal, say accept trade. If \
                         you wish to stop the deal, say stop trade. You can check the deal with \
                         show trade.",
                        speaker.name, c2.name
                    ),
                );
                *face_target = Some(speaker_pos);
            }

            if text.contains("stop trade") {
                if data.state != 1 && data.state != 2 {
                    self.npc_quiet_say(trader_id, "Sorry, not possible right now.");
                    *face_target = Some(speaker_pos);
                    break 'commands;
                }
                if data.c1_id != Some(speaker_id) && data.c2_id != Some(speaker_id) {
                    self.npc_quiet_say(
                        trader_id,
                        "Sorry, I am not trading on your behalf at the moment.",
                    );
                    *face_target = Some(speaker_pos);
                    break 'commands;
                }
                self.trader_return_items(data, false);
                self.npc_quiet_say(trader_id, "The trade is cancelled.");
                *face_target = Some(speaker_pos);
                data.state = 0;
                data.c1_ok = false;
                data.c2_ok = false;
            }

            if text == "accept trade" {
                if data.state != 1 && data.state != 2 {
                    self.npc_quiet_say(trader_id, "Sorry, not possible right now.");
                    *face_target = Some(speaker_pos);
                    break 'commands;
                }
                if data.c1_id != Some(speaker_id) && data.c2_id != Some(speaker_id) {
                    self.npc_quiet_say(
                        trader_id,
                        "Sorry, I am not trading at your behalf at the moment.",
                    );
                    *face_target = Some(speaker_pos);
                    break 'commands;
                }
                if data.c1_id == Some(speaker_id) {
                    data.c1_ok = true;
                }
                if data.c2_id == Some(speaker_id) {
                    data.c2_ok = true;
                }
                if data.c1_ok && data.c2_ok {
                    self.trader_return_items(data, true);
                    self.npc_quiet_say(trader_id, "Deal.");
                    *face_target = Some(speaker_pos);
                    // C: "Award Trust But Verify achievement to both
                    // traders" (`base.c:4420-4428`).
                    if let (Some(c1_id), Some(c2_id)) = (data.c1_id, data.c2_id) {
                        self.pending_trader_events
                            .push(TraderEvent::DealCompleted { c1_id, c2_id });
                    }
                    data.state = 0;
                    data.c1_ok = false;
                    data.c2_ok = false;
                } else {
                    self.npc_quiet_say(
                        trader_id,
                        &format!("{} is satisfied with the deal.", speaker.name),
                    );
                    *face_target = Some(speaker_pos);
                    data.state = 2;
                }
            } else if text.contains("accept trade") {
                self.npc_say(
                    trader_id,
                    "You have to say \"accept trade\" by itself, not as part of a longer \
                     sentence to make it work. Like this:",
                );
                self.npc_say(trader_id, "accept trade");
                self.npc_say(trader_id, "No leading or trailing spaces, either.");
            }

            if text.contains("show trade") {
                if data.state != 1 && data.state != 2 {
                    self.npc_quiet_say(trader_id, "Sorry, not possible right now.");
                    *face_target = Some(speaker_pos);
                    break 'commands;
                }
                if data.c1_id != Some(speaker_id) && data.c2_id != Some(speaker_id) {
                    self.npc_quiet_say(
                        trader_id,
                        "Sorry, I am not trading at your behalf at the moment.",
                    );
                    *face_target = Some(speaker_pos);
                    break 'commands;
                }
                self.pending_trader_events.push(TraderEvent::ShowTrade {
                    viewer_id: speaker_id,
                    c1_items: data.c1_items.clone(),
                    c2_items: data.c2_items.clone(),
                });
            }
        }
    }

    /// C `trader_driver`'s `NT_GIVE` branch (`base.c:4469-4529`).
    fn trader_handle_give_message(
        &mut self,
        trader_id: CharacterId,
        data: &mut TraderDriverData,
        message: &CharacterDriverMessage,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let giver_pos = (i32::from(giver.x), i32::from(giver.y));

        if data.state != 1 {
            self.npc_quiet_say(trader_id, "Sorry, not possible right now.");
            *face_target = Some(giver_pos);
            self.trader_return_or_destroy_cursor_item(trader_id, giver_id);
            return;
        }

        let is_c1 = data.c1_id == Some(giver_id);
        let is_c2 = data.c2_id == Some(giver_id);

        if !is_c1 && !is_c2 {
            self.npc_quiet_say(
                trader_id,
                &format!(
                    "I am not trading at your behalf at the moment, {}.",
                    giver.name
                ),
            );
            *face_target = Some(giver_pos);
            self.trader_return_or_destroy_cursor_item(trader_id, giver_id);
            return;
        }

        let (side_items, other_side_id) = if is_c1 {
            (&mut data.c1_items, data.c2_id)
        } else {
            (&mut data.c2_items, data.c1_id)
        };

        if side_items.len() >= MAX_TRADER_ITEMS {
            self.npc_quiet_say(
                trader_id,
                &format!(
                    "I cannot trade more than ten items at once, {}.",
                    giver.name
                ),
            );
            *face_target = Some(giver_pos);
            self.trader_return_or_destroy_cursor_item(trader_id, giver_id);
            return;
        }

        let Some(item_id) = self
            .characters
            .get_mut(&trader_id)
            .and_then(|trader| trader.cursor_item.take())
        else {
            return;
        };
        if is_c1 {
            data.c1_items.push(item_id);
        } else {
            data.c2_items.push(item_id);
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.flags.insert(ItemFlags::VOID);
        }
        if let Some(notify_id) = other_side_id {
            self.pending_trader_events
                .push(TraderEvent::ItemAddedToTrade {
                    notify_id,
                    giver_name: giver.name.clone(),
                    item_id,
                });
        }
    }

    /// C `trader_driver`'s message loop (`base.c:4260-4533`).
    fn process_trader_messages(&mut self, trader_id: CharacterId) {
        let Some(trader_name) = self.characters.get(&trader_id).map(|t| t.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::Trader(mut data)) = self
            .characters
            .get(&trader_id)
            .and_then(|t| t.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&trader_id)
            .map(|trader| std::mem::take(&mut trader.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_TEXT => self.trader_handle_text_message(
                    trader_id,
                    &trader_name,
                    &mut data,
                    message,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.trader_handle_give_message(trader_id, &mut data, message, &mut face_target)
                }
                _ => {}
            }
        }

        if let Some(trader) = self.characters.get_mut(&trader_id) {
            trader.driver_state = Some(CharacterDriverState::Trader(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`base.c:4550-4552`).
        if let (Some(trader), Some((tx, ty))) =
            (self.characters.get(&trader_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(trader.x), i32::from(trader.y), tx, ty) {
                if let Some(trader_mut) = self.characters.get_mut(&trader_id) {
                    let _ = turn(trader_mut, direction as u8);
                }
            }
        }
    }

    /// C `trader_driver`'s `NT_CHAR` greeting branch (`base.c:4264-4291`),
    /// ported as a periodic nearby-player scan matching the
    /// simplification `world/bank.rs`/`world/merchant.rs` already
    /// established, since it turns to face the greeted player
    /// immediately (an observable part of C's behavior via `talkdir`),
    /// unlike the bank/merchant greetings which never turn.
    fn greet_nearby_traders(&mut self, trader_id: CharacterId) {
        let Some(trader) = self.characters.get(&trader_id).cloned() else {
            return;
        };

        let mut greetings: Vec<(CharacterId, String, i32, i32)> = Vec::new();
        for character in self.characters.values() {
            if character.id == trader_id
                || !character.flags.contains(CharacterFlags::PLAYER)
                || mem_check_driver(
                    &trader.driver_memory,
                    TRADER_GREET_MEMORY_SLOT,
                    character.id.0,
                )
            {
                continue;
            }
            if char_dist(&trader, character) > TRADER_GREET_DISTANCE {
                continue;
            }
            if !char_see_char(&trader, character, &self.map, self.date.daylight) {
                continue;
            }
            greetings.push((
                character.id,
                character.name.clone(),
                i32::from(character.x),
                i32::from(character.y),
            ));
        }

        for (player_id, name, x, y) in greetings {
            self.npc_quiet_say(
                trader_id,
                &format!(
                    "Hello {name}! I will work as middleman in any deal you might wish to make \
                     with another player. With my help, no one will cheat you. "
                ),
            );
            if let Some(trader_mut) = self.characters.get_mut(&trader_id) {
                mem_add_driver(
                    &mut trader_mut.driver_memory,
                    TRADER_GREET_MEMORY_SLOT,
                    player_id.0,
                );
            }
            if let Some(direction) = offset2dx(i32::from(trader.x), i32::from(trader.y), x, y) {
                if let Some(trader_mut) = self.characters.get_mut(&trader_id) {
                    let _ = turn(trader_mut, direction as u8);
                }
            }
        }
    }

    /// C `trader_driver`'s timeout-cancel block (`base.c:4537-4543`).
    fn trader_check_timeout(&mut self, trader_id: CharacterId) {
        let tick = self.tick.0;
        let Some(CharacterDriverState::Trader(mut data)) = self
            .characters
            .get(&trader_id)
            .and_then(|t| t.driver_state.clone())
        else {
            return;
        };
        if data.state > 0 && tick > data.timeout {
            self.npc_quiet_say(trader_id, "The trade is cancelled!");
            self.trader_return_items(&mut data, false);
            data.state = 0;
            data.c1_ok = false;
            data.c2_ok = false;
            if let Some(trader) = self.characters.get_mut(&trader_id) {
                trader.driver_state = Some(CharacterDriverState::Trader(data));
            }
        }
    }

    /// C `trader_driver`'s memory-clear block (`base.c:4545-4548`).
    fn clear_expired_trader_memory(&mut self, trader_id: CharacterId) {
        let tick = self.tick.0;
        if let Some(trader) = self.characters.get_mut(&trader_id) {
            let memory_clear_tick = match trader.driver_state.as_ref() {
                Some(CharacterDriverState::Trader(data)) => data.memory_clear_tick,
                _ => return,
            };
            if tick > memory_clear_tick {
                mem_erase_driver(&mut trader.driver_memory, TRADER_GREET_MEMORY_SLOT);
                if let Some(CharacterDriverState::Trader(data)) = trader.driver_state.as_mut() {
                    data.memory_clear_tick = tick + TRADER_MEMORY_CLEAR_TICKS;
                }
            }
        }
    }

    /// C `trader_driver`'s idle-murmur block (`base.c:4554-4572`).
    fn trader_idle_chatter(&mut self, trader_id: CharacterId) {
        let tick = self.tick.0;
        let Some(trader) = self.characters.get(&trader_id) else {
            return;
        };
        let last_talk = match trader.driver_state.as_ref() {
            Some(CharacterDriverState::Trader(data)) => data.last_talk,
            _ => return,
        };
        if tick <= last_talk + TRADER_TALK_INTERVAL_TICKS {
            return;
        }
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, 25) != 0 {
            return;
        }

        let index = legacy_random_below_from_seed(&mut self.legacy_random_seed, 12) as usize;
        self.npc_murmur(trader_id, TRADER_MUTTERINGS[index]);

        if let Some(CharacterDriverState::Trader(data)) = self
            .characters
            .get_mut(&trader_id)
            .and_then(|trader| trader.driver_state.as_mut())
        {
            data.last_talk = tick;
        }
    }

    /// Trader NPC tick: process messages, greet nearby players, check the
    /// trade timeout, clear expired driver memory, and roll idle
    /// mutterings. Ports the per-tick body of C `trader_driver`.
    pub fn process_trader_actions(&mut self) {
        let trader_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TRADER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for trader_id in trader_ids {
            self.process_trader_messages(trader_id);
            self.greet_nearby_traders(trader_id);
            self.trader_check_timeout(trader_id);
            self.clear_expired_trader_memory(trader_id);
            self.trader_idle_chatter(trader_id);
        }
    }
}
