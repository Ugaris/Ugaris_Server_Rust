//! `CDR_BANK` banker NPC.
//!
//! Ports `src/module/bank.c`'s `bank_driver` (greeting, small talk,
//! deposit/withdraw/balance text commands, idle murmurs, and the
//! day/night shop-position/door movement) plus the `DRD_BANK_PPD`
//! persistent account-balance codec (`crate::player::PlayerRuntime`,
//! see `encode_legacy_bank_ppd`/`decode_legacy_bank_ppd`).
//!
//! Unlike `Character.gold` (`ch[cn].gold`, mutated directly here), the
//! bank's `ppd->imperial_gold` balance is *player*-persistent state that
//! `World` cannot see (see `crate::player::PlayerRuntime`, owned by the
//! `ugaris-server` session layer, not `World`). Deposit/withdraw/balance
//! text commands therefore validate and apply whatever they can from
//! `Character` state alone (deposit's "enough carried gold?" check,
//! withdraw's "amount <= 0" check) synchronously, and queue a
//! [`BankEvent`] via [`World::drain_pending_bank_events`] for the
//! remainder (crediting/debiting the persistent balance, and the
//! withdraw/balance reply text that depends on its current value) -
//! mirroring the existing `pending_*`/`drain_pending_*` convention used
//! throughout `World` (e.g. `pending_kill_exp`).
use crate::character_driver::{mem_add_driver, mem_check_driver, mem_erase_driver};
use crate::world::*;

const BANK_GREET_DISTANCE: i32 = 10;
const BANK_QA_DISTANCE: i32 = 12;
const BANK_MEMORY_CLEAR_TICKS: u64 = TICKS_PER_SECOND * 60 * 60 * 12;
/// C `TICKS * 60` in `bank_driver`'s idle-murmur throttle (`bank.c:459`).
pub(crate) const BANK_TALK_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `mem_add_driver(cn, co, 7)`/`mem_check_driver(cn, co, 7)` in
/// `bank_driver`'s greeting handler (`bank.c:332,341`).
const BANK_GREET_MEMORY_SLOT: usize = 7;

/// C `static const char *bank_mutterings[]` (`bank.c:460-477`).
const BANK_MUTTERINGS: [&str; 16] = [
    "I love the clicking of coins.",
    "Gold and Silver, Silver and Gold.",
    "Don't spend it all in one place.",
    "Keep your savings safe!",
    "Counting coins is my cardio.",
    "Another day, another deposit. Or withdrawal. Preferably deposit.",
    "Inflation these days... a gold piece isn't what it used to be.",
    "Please wipe your boots before entering the bank. This is a respectable establishment.",
    "The vault is reinforced with enchanted steel. Not that anyone has tried to rob us. Yet.",
    "I've seen adventurers deposit fortunes and withdraw them the next day. No discipline.",
    "Interest rates? We don't do interest rates. This isn't that kind of bank.",
    "Mud. On my floor. Again.",
    "Some clients deposit a single gold coin. I process it with the same dignity as a thousand.",
    "The Imperial Bank has stood for centuries. Through wars, plagues, and budget cuts.",
    "I once counted to a million. For fun. On a slow Tuesday.",
    "Please form an orderly queue. I know there is no queue. I'm speaking prophetically.",
];

/// A `bank_driver` deposit/withdraw/balance text command that needs the
/// player's persistent `DRD_BANK_PPD` balance (owned by `PlayerRuntime`,
/// outside `World`'s visibility) to finish applying. See the module doc
/// comment for the split between what `World` resolves synchronously and
/// what it defers here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankEvent {
    /// C `bank_driver`'s deposit branch already validated the amount
    /// against `ch[co].gold` and subtracted it; the caller must add
    /// `amount` to the player's persistent balance
    /// (`ppd->imperial_gold += val`).
    Deposit { player_id: CharacterId, amount: u32 },
    /// C `bank_driver`'s withdraw branch (`bank.c:366-378`): the caller
    /// must compare `amount` against the player's persistent balance,
    /// and on success subtract it there, credit `Character.gold`
    /// (`give_money_silent`), and reply via [`World::npc_quiet_say`];
    /// on failure reply "Thou dost not have that much gold in thine
    /// account." (`World` cannot make this decision - it does not see
    /// the persistent balance).
    Withdraw {
        bank_id: CharacterId,
        player_id: CharacterId,
        amount: u32,
    },
    /// C `bank_driver`'s balance branch (`bank.c:379-387`): the caller
    /// must format and send the balance reply from the player's
    /// persistent balance.
    Balance {
        bank_id: CharacterId,
        player_id: CharacterId,
    },
}

/// C `atoi`: skip leading whitespace, an optional sign, then digits;
/// stops at the first non-digit and returns `0` if none were found.
fn legacy_atoi(text: &str) -> i32 {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let negative = match bytes.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if start == i {
        return 0;
    }
    let value: i64 = text[start..i].parse().unwrap_or(0);
    let value = if negative { -value } else { value };
    value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// C `opening_time(from, to)` (`bank.c:276-290`).
pub(crate) fn bank_opening_time(from: i32, to: i32, hour: i64) -> bool {
    if from > to {
        hour >= i64::from(from) || hour <= i64::from(to)
    } else {
        hour >= i64::from(from) && hour <= i64::from(to)
    }
}

impl World {
    pub fn drain_pending_bank_events(&mut self) -> Vec<BankEvent> {
        self.pending_bank_events.drain(..).collect()
    }

    /// C `is_closed(x, y)` (`drvlib.c:2543-2557`): `true` only for a door
    /// item (`IDR_DOOR`) whose `drdata[0]` (open flag) is unset.
    fn bank_door_is_closed(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        let Some(tile) = self.map.tile(x as usize, y as usize) else {
            return false;
        };
        if tile.item == 0 {
            return false;
        }
        let Some(item) = self.items.get(&ItemId(tile.item)) else {
            return false;
        };
        item.driver == IDR_DOOR && !door_open_state(item)
    }

    /// C `is_room_empty(xs, ys, xe, ye)` (`drvlib.c:2560-2578`): `true`
    /// when no `CF_PLAYER` character sits inside the bounding box. C
    /// walks a sector index in steps of 8; this is a plain linear scan
    /// (no sector index exists in this codebase), same observable result.
    fn bank_room_is_empty(&self, xs: i32, ys: i32, xe: i32, ye: i32) -> bool {
        !self.characters.values().any(|character| {
            character.flags.contains(CharacterFlags::PLAYER)
                && i32::from(character.x) >= xs
                && i32::from(character.x) <= xe
                && i32::from(character.y) >= ys
                && i32::from(character.y) <= ye
        })
    }

    /// C `use_item_at(cn, x, y, spec)` (`drvlib.c:2581-2601`): first tries
    /// `use_driver` directly (here: toggle a door item at the tile
    /// in-place, matching `use_driver`'s door dispatch outcome for the
    /// unkeyed case - bank doors are not expected to require a key in
    /// existing zone data, so the full keyed-door gate in
    /// `item_driver::door_driver` is not replicated here), then falls
    /// back to pathing adjacent (`mindist` 1) and walking/using.
    fn bank_use_item_at(&mut self, bank_id: CharacterId, x: i32, y: i32, area_id: u16) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        let (ux, uy) = (x as usize, y as usize);
        let item_id = self.map.tile(ux, uy).map(|tile| tile.item).unwrap_or(0);
        if item_id != 0
            && self
                .items
                .get(&ItemId(item_id))
                .is_some_and(|item| item.driver == IDR_DOOR)
            && matches!(
                self.toggle_door(ItemId(item_id), bank_id),
                DoorToggleResult::Toggled
            )
        {
            return true;
        }
        self.setup_walk_toward(bank_id, ux, uy, 1, area_id, false)
    }

    /// C `bank_driver`'s message loop (`bank.c:311-408`): the `NT_TEXT`
    /// (small talk + deposit/withdraw/balance) and `NT_GIVE` (received
    /// item) branches. `NT_CHAR` is drained without action - greeting is
    /// handled by [`Self::greet_nearby_bank_customers`], matching the
    /// simplification `world/merchant.rs` already established (a
    /// periodic nearby-player scan instead of reacting to `NT_CHAR`
    /// notify-area broadcasts).
    fn process_bank_messages(&mut self, bank_id: CharacterId) {
        let Some(bank) = self.characters.get_mut(&bank_id) else {
            return;
        };
        let bank_name = bank.name.clone();
        let messages = std::mem::take(&mut bank.driver_messages);

        let mut destroy_cursor = false;
        let mut replies: Vec<String> = Vec::new();
        let mut deposit_charges: Vec<(CharacterId, u32)> = Vec::new();
        let mut events: Vec<BankEvent> = Vec::new();

        for message in messages {
            match message.message_type {
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3 as u32);
                    if speaker_id == bank_id {
                        continue;
                    }
                    let Some(text) = message.text.as_deref() else {
                        continue;
                    };

                    if let Some(reply) = self.bank_qa_reply(bank_id, &bank_name, speaker_id, text) {
                        replies.push(reply);
                    }

                    // C `strcasestr(msg->dat2, "deposit"/"withdraw"/
                    // "balance")`: a raw substring search over the whole
                    // message (not the tokenized qa wordlist), so e.g.
                    // "explain deposit" matches *both* the qa table above
                    // *and* this deposit branch - producing two replies,
                    // matching C's literal (if odd) behavior.
                    let lower = text.to_ascii_lowercase();
                    if let Some(pos) = lower.find("deposit") {
                        let amount =
                            legacy_atoi(&lower[pos + "deposit".len()..]).saturating_mul(100);
                        if amount == 0 {
                            replies.push("Thou must name an amount.".to_string());
                        } else {
                            let carried_gold =
                                self.characters.get(&speaker_id).map(|player| player.gold);
                            match carried_gold {
                                Some(gold) if amount > 0 && amount as u32 <= gold => {
                                    deposit_charges.push((speaker_id, amount as u32));
                                    replies.push(format!(
                                        "Thou hast deposited {} gold coins.",
                                        amount / 100
                                    ));
                                    events.push(BankEvent::Deposit {
                                        player_id: speaker_id,
                                        amount: amount as u32,
                                    });
                                }
                                _ => {
                                    replies.push("Thou dost not have that much gold.".to_string());
                                }
                            }
                        }
                    } else if let Some(pos) = lower.find("withdraw") {
                        let amount =
                            legacy_atoi(&lower[pos + "withdraw".len()..]).saturating_mul(100);
                        if amount == 0 {
                            replies.push("Thou must name an amount.".to_string());
                        } else if amount < 0 {
                            replies.push(
                                "Thou dost not have that much gold in thine account.".to_string(),
                            );
                        } else {
                            events.push(BankEvent::Withdraw {
                                bank_id,
                                player_id: speaker_id,
                                amount: amount as u32,
                            });
                        }
                    } else if lower.contains("balance") {
                        events.push(BankEvent::Balance {
                            bank_id,
                            player_id: speaker_id,
                        });
                    }
                }
                NT_GIVE => {
                    // C `bank_driver` first tries `give_driver(cn, co)` to
                    // hand the item back to the sender, falling back to
                    // `destroy_item` only if that fails. `world/merchant.rs`
                    // already simplified this same C pattern (no generic
                    // "give item back" driver helper exists yet) to an
                    // unconditional destroy; this port keeps that
                    // established precedent for consistency.
                    destroy_cursor = true;
                }
                _ => {}
            }
        }

        for (player_id, amount) in deposit_charges {
            if let Some(player) = self.characters.get_mut(&player_id) {
                player.gold = player.gold.saturating_sub(amount);
                player.flags.insert(CharacterFlags::ITEMS);
            }
        }

        if destroy_cursor {
            let cursor = self
                .characters
                .get_mut(&bank_id)
                .and_then(|bank| bank.cursor_item.take());
            if let Some(item_id) = cursor {
                self.destroy_item(item_id);
            }
        }

        for reply in replies {
            self.npc_quiet_say(bank_id, &reply);
        }

        self.pending_bank_events.extend(events);
    }

    /// C `analyse_text_driver` from `src/module/bank.c`, wired through the
    /// generic [`crate::character_driver::analyse_text_qa`] matcher (same
    /// pattern as `world/merchant.rs::merchant_qa_reply`).
    fn bank_qa_reply(
        &self,
        bank_id: CharacterId,
        bank_name: &str,
        speaker_id: CharacterId,
        text: &str,
    ) -> Option<String> {
        let bank = self.characters.get(&bank_id)?;
        let speaker = self.characters.get(&speaker_id)?;
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return None;
        }
        if char_dist(bank, speaker) > BANK_QA_DISTANCE {
            return None;
        }
        if !char_see_char(bank, speaker, &self.map, self.date.daylight) {
            return None;
        }
        match crate::character_driver::analyse_text_qa(text, bank_name, &speaker.name, BANK_QA) {
            crate::character_driver::TextAnalysisOutcome::Said(reply) => Some(reply),
            // C: `answer_code == 1` -> `quiet_say(cn, "I'm %s.", ch[cn].name)`.
            crate::character_driver::TextAnalysisOutcome::Matched(1) => {
                Some(format!("I'm {bank_name}."))
            }
            _ => None,
        }
    }

    /// C `bank_driver`'s `NT_CHAR` greeting branch (`bank.c:316-341`),
    /// ported as a periodic nearby-player scan (see
    /// [`Self::process_bank_messages`]'s doc comment for why).
    fn greet_nearby_bank_customers(&mut self, bank_id: CharacterId) {
        let Some(bank) = self.characters.get(&bank_id).cloned() else {
            return;
        };

        let mut greetings: Vec<(CharacterId, String)> = Vec::new();
        for character in self.characters.values() {
            if character.id == bank_id
                || !character.flags.contains(CharacterFlags::PLAYER)
                || mem_check_driver(&bank.driver_memory, BANK_GREET_MEMORY_SLOT, character.id.0)
            {
                continue;
            }
            if char_dist(&bank, character) > BANK_GREET_DISTANCE {
                continue;
            }
            if !char_see_char(&bank, character, &self.map, self.date.daylight) {
                continue;
            }
            greetings.push((
                character.id,
                format!(
                    "Hello {}! Would you like to open an account with the Imperial Bank?",
                    character.name
                ),
            ));
        }

        for (player_id, greeting) in &greetings {
            self.npc_quiet_say(bank_id, greeting);
            if let Some(bank) = self.characters.get_mut(&bank_id) {
                mem_add_driver(&mut bank.driver_memory, BANK_GREET_MEMORY_SLOT, player_id.0);
            }
        }
    }

    /// C `bank_driver`'s idle-murmur block (`bank.c:459-480`): once per
    /// minute, roll `RANDOM(25)` and on a 1-in-25 hit pick a random line
    /// via `RANDOM(16)`.
    fn bank_idle_chatter(&mut self, bank_id: CharacterId) {
        let tick = self.tick.0;
        let Some(bank) = self.characters.get(&bank_id) else {
            return;
        };
        let last_talk = match bank.driver_state.as_ref() {
            Some(CharacterDriverState::Bank(data)) => data.last_talk,
            _ => return,
        };
        if tick <= last_talk + BANK_TALK_INTERVAL_TICKS {
            return;
        }
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, 25) != 0 {
            // C: `dat->last_talk` is only updated on the roll's 1-in-25
            // hit branch (the surrounding `if` guards the whole block).
            return;
        }

        let index = legacy_random_below_from_seed(&mut self.legacy_random_seed, 16) as usize;
        self.npc_murmur(bank_id, BANK_MUTTERINGS[index]);

        if let Some(CharacterDriverState::Bank(data)) = self
            .characters
            .get_mut(&bank_id)
            .and_then(|bank| bank.driver_state.as_mut())
        {
            data.last_talk = tick;
        }
    }

    /// C `bank_driver`'s memory-clear block (`bank.c:482-485`).
    fn clear_expired_bank_memory(&mut self, bank_id: CharacterId) {
        let tick = self.tick.0;
        if let Some(bank) = self.characters.get_mut(&bank_id) {
            let memory_clear_tick = match bank.driver_state.as_ref() {
                Some(CharacterDriverState::Bank(data)) => data.memory_clear_tick,
                _ => return,
            };
            if tick > memory_clear_tick {
                mem_erase_driver(&mut bank.driver_memory, BANK_GREET_MEMORY_SLOT);
                if let Some(CharacterDriverState::Bank(data)) = bank.driver_state.as_mut() {
                    data.memory_clear_tick = tick + BANK_MEMORY_CLEAR_TICKS;
                }
            }
        }
    }

    /// C `bank_driver`'s movement/door section (`bank.c:413-457`): moves
    /// between day/night shop positions (or back to the spawn tile,
    /// `ch[cn].tmpx/tmpy` -> `Character::rest_x/rest_y`, when no day/night
    /// positions are configured), opening/closing the shop door on
    /// schedule. Like C, idle chatter and the memory-clear timer only run
    /// when no movement/door/closing-announcement action fired this tick
    /// (every C branch above them ends in `return` on success).
    fn process_bank_tick_action(&mut self, bank_id: CharacterId, area_id: u16) {
        let Some(bank) = self.characters.get(&bank_id).cloned() else {
            return;
        };
        let Some(CharacterDriverState::Bank(data)) = bank.driver_state.clone() else {
            return;
        };

        let acted = if data.dayx != 0 {
            if !bank_opening_time(data.open, data.close, self.date.hour) {
                self.bank_closed_tick_action(bank_id, &bank, &data, area_id)
            } else {
                self.bank_open_tick_action(bank_id, &bank, &data, area_id)
            }
        } else if self.setup_walk_toward(
            bank_id,
            usize::from(bank.rest_x),
            usize::from(bank.rest_y),
            0,
            area_id,
            false,
        ) {
            true
        } else {
            if bank.dir != data.dir as u8 {
                if let Some(bank_mut) = self.characters.get_mut(&bank_id) {
                    let _ = turn(bank_mut, data.dir as u8);
                }
            }
            false
        };

        if acted {
            return;
        }
        self.bank_idle_chatter(bank_id);
        self.clear_expired_bank_memory(bank_id);
    }

    fn bank_closed_tick_action(
        &mut self,
        bank_id: CharacterId,
        bank: &Character,
        data: &BankDriverData,
        area_id: u16,
    ) -> bool {
        if data.doorx != 0 && !self.bank_door_is_closed(data.doorx, data.doory) {
            if !self.bank_room_is_empty(data.storefx, data.storefy, data.storetx, data.storety) {
                self.npc_quiet_say(bank_id, "We're closing, please leave now!");
            } else {
                self.bank_use_item_at(bank_id, data.doorx, data.doory, area_id);
            }
            return true;
        }
        if self.setup_walk_toward(
            bank_id,
            data.nightx as usize,
            data.nighty as usize,
            0,
            area_id,
            false,
        ) {
            return true;
        }
        if bank.dir != data.nightdir as u8 {
            if let Some(bank_mut) = self.characters.get_mut(&bank_id) {
                let _ = turn(bank_mut, data.nightdir as u8);
            }
        }
        false
    }

    fn bank_open_tick_action(
        &mut self,
        bank_id: CharacterId,
        bank: &Character,
        data: &BankDriverData,
        area_id: u16,
    ) -> bool {
        if data.doorx != 0 && self.bank_door_is_closed(data.doorx, data.doory) {
            self.bank_use_item_at(bank_id, data.doorx, data.doory, area_id);
            return true;
        }
        if self.setup_walk_toward(
            bank_id,
            data.dayx as usize,
            data.dayy as usize,
            0,
            area_id,
            false,
        ) {
            return true;
        }
        if bank.dir != data.daydir as u8 {
            if let Some(bank_mut) = self.characters.get_mut(&bank_id) {
                let _ = turn(bank_mut, data.daydir as u8);
            }
        }
        false
    }

    /// Bank NPC tick: process messages, greet nearby players, and run the
    /// movement/door/chatter/memory-clear block. Ports the per-tick body
    /// of C `bank_driver`.
    pub fn process_bank_actions(&mut self, area_id: u16) {
        let bank_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_BANK
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for bank_id in bank_ids {
            self.process_bank_messages(bank_id);
            self.greet_nearby_bank_customers(bank_id);
            self.process_bank_tick_action(bank_id, area_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct bank_driver_data` from `src/module/bank.c`, plus the driver
/// memory used for greeting throttling (shared 8-slot `DriverMemory`, same
/// as `MerchantDriverData`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BankDriverData {
    pub dir: i32,
    pub dayx: i32,
    pub dayy: i32,
    pub daydir: i32,
    pub nightx: i32,
    pub nighty: i32,
    pub nightdir: i32,
    pub storefx: i32,
    pub storefy: i32,
    pub storetx: i32,
    pub storety: i32,
    pub doorx: i32,
    pub doory: i32,
    pub open: i32,
    pub close: i32,
    #[serde(default)]
    pub last_talk: u64,
    #[serde(default)]
    pub memory_clear_tick: u64,
}

/// C `struct qa qa[]` from `src/module/bank.c`. Note `"help"`'s answer is a
/// verbatim copy-paste of `merchant.c`'s line (`"Sorry, I'm just a
/// merchant, %s!"`) even though this NPC is a banker - preserved as-is per
/// the porting rule to copy quirks, not "fix" them. The `"account"`/
/// `"explain deposit"`/`"explain withdraw"`/`"explain balance"` answers
/// wrap the referenced keywords in `COL_LIGHT_BLUE`/`COL_RESET` in C; the
/// shared [`analyse_text_qa`] pipeline works on plain `&str` (the legacy
/// color marker is a raw non-UTF8 byte, see `crate::text::COL_LIGHT_BLUE`,
/// and cannot be represented in a Rust string literal), so only the color
/// styling is dropped here - the wording is byte-for-byte identical.
pub const BANK_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm fine!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hello"],
        answer: Some("Hello, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hi"],
        answer: Some("Hi, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["greetings"],
        answer: Some("Greetings, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hail"],
        answer: Some("And hail to you, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["help"],
        answer: Some("Sorry, I'm just a merchant, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what", "is", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["what", "is", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["who", "are", "you"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["account"],
        answer: Some(
            "If you want to open an account, you must first deposit (explain deposit) some \
             money in it. After that, you can inquire for your balance (explain balance) or \
             withdraw (explain withdraw) money.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["explain", "deposit"],
        answer: Some("To deposit 38 gold coins for example, just say: 'deposit 38'."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["explain", "withdraw"],
        answer: Some("To withdraw 38 gold coins for example, just say: 'withdraw 38'."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["explain", "balance"],
        answer: Some("To inquire about the balance of your account, just say: 'balance'"),
        answer_code: 0,
    },
];
