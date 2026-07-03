//! Merchant stores and trading.
//!
//! Ports `src/module/merchants/store.c` (store creation, sales/buy prices,
//! buy/sell) and the trade-activation/greeting slices of
//! `src/module/merchants/merchant.c`. Database-backed store persistence and
//! day/night shop movement remain unported.

use super::*;
use crate::character_driver::{mem_add_driver, mem_check_driver, mem_erase_driver};

/// C `STORESIZE` from `src/module/merchants/store.h`.
pub const MERCHANT_STORE_SIZE: usize = INVENTORY_SIZE - 2;

const MERCHANT_GREET_DISTANCE: i32 = 10;
const FRED_GREET_DISTANCE: i32 = 25;
const MERCHANT_MEMORY_CLEAR_TICKS: u64 = TICKS_PER_SECOND * 60 * 60 * 12;
/// C `mem_add_driver(cn, co, 7)`/`mem_check_driver(cn, co, 7)` in
/// `merchant.c`'s greeting/trade-request handlers.
const MERCHANT_GREET_MEMORY_SLOT: usize = 7;

#[derive(Debug, Clone)]
pub struct StoreWare {
    pub item: Item,
    pub count: u32,
    pub always: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MerchantStore {
    pub wares: Vec<Option<StoreWare>>,
    pub gold: i64,
    pub price_multi: i32,
}

impl MerchantStore {
    fn new(price_multi: i32) -> Self {
        Self {
            wares: (0..MERCHANT_STORE_SIZE).map(|_| None).collect(),
            gold: 0,
            price_multi,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MerchantTradeResult {
    /// Trade succeeded for the given price in silver.
    Traded(u32),
    SoldOut,
    TooExpensive,
    CursorOccupied,
    NoCursorItem,
    Rejected,
}

/// C `salesprice`: what the player pays when buying ware `nr`.
pub fn merchant_sales_price(
    ware_value: u32,
    price_multi: i32,
    barter: i32,
    trader_profession: i32,
) -> u32 {
    let price = f64::from(ware_value);
    let divisor = f64::from(barter + 100 + trader_profession * 5);
    let scaled = price * f64::from(price_multi.max(0)) / divisor.max(1.0);
    (price * 1.25).max(scaled) as u32
}

/// C `buyprice`: what the player receives when selling an item.
pub fn merchant_buy_price(
    item_value: u32,
    is_money: bool,
    barter: i32,
    trader_profession: i32,
) -> u32 {
    if is_money {
        return item_value;
    }
    let price = f64::from(item_value);
    let modifier = f64::from(barter + 100 + trader_profession * 5);
    (price * 0.80).min(price * modifier / 400.0) as u32
}

/// C `store_items_equal`: wares stack only for identical item snapshots.
fn store_items_equal(a: &Item, b: &Item) -> bool {
    a.description == b.description
        && a.driver_data == b.driver_data
        && a.driver == b.driver
        && a.flags == b.flags
        && a.name == b.name
        && a.sprite == b.sprite
        && a.value == b.value
        && a.owner_id == b.owner_id
        && a.modifier_index == b.modifier_index
        && a.modifier_value == b.modifier_value
}

impl World {
    fn merchant_barter_and_trader(&self, character_id: CharacterId) -> (i32, i32) {
        self.characters
            .get(&character_id)
            .map(|character| {
                (
                    i32::from(character.values[0][CharacterValue::Barter as usize]),
                    character
                        .professions
                        .get(profession::TRADER)
                        .copied()
                        .map(i32::from)
                        .unwrap_or_default(),
                )
            })
            .unwrap_or((0, 0))
    }

    /// C `create_store`: turn the merchant's carried inventory (slots 30+
    /// beyond `ignore`) into permanent `always` store stock.
    pub fn ensure_merchant_store(&mut self, merchant_id: CharacterId) -> bool {
        if self.merchant_stores.contains_key(&merchant_id) {
            return true;
        }
        let Some(merchant) = self.characters.get(&merchant_id) else {
            return false;
        };
        let (ignore, price_multi) = match &merchant.driver_state {
            Some(CharacterDriverState::Merchant(data)) => (
                data.ignore.max(0) as usize,
                if data.pricemulti > 0 {
                    data.pricemulti
                } else {
                    400
                },
            ),
            _ => (0, 400),
        };

        let mut store = MerchantStore::new(price_multi);
        let mut taken: Vec<ItemId> = Vec::new();
        if let Some(merchant) = self.characters.get_mut(&merchant_id) {
            for slot in 0..MERCHANT_STORE_SIZE {
                let inventory_slot = INVENTORY_START_INVENTORY + ignore + slot;
                if inventory_slot >= merchant.inventory.len() {
                    break;
                }
                if let Some(item_id) = merchant.inventory[inventory_slot].take() {
                    taken.push(item_id);
                }
            }
        }
        let mut ware_slot = 0;
        for item_id in taken {
            if let Some(item) = self.items.remove(&item_id) {
                store.wares[ware_slot] = Some(StoreWare {
                    item,
                    count: 1,
                    always: true,
                });
                ware_slot += 1;
            }
        }
        self.merchant_stores.insert(merchant_id, store);
        if let Some(merchant) = self.characters.get_mut(&merchant_id) {
            if let Some(CharacterDriverState::Merchant(data)) = merchant.driver_state.as_mut() {
                data.store_created = true;
            }
        }
        true
    }

    /// C `add_item_to_store`.
    pub fn add_item_to_merchant_store(&mut self, merchant_id: CharacterId, item: Item) {
        let Some(store) = self.merchant_stores.get_mut(&merchant_id) else {
            return;
        };
        if let Some(ware) = store
            .wares
            .iter_mut()
            .flatten()
            .find(|ware| store_items_equal(&ware.item, &item))
        {
            ware.count += 1;
            return;
        }
        if let Some(slot) = store.wares.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(StoreWare {
                item,
                count: 1,
                always: false,
            });
            return;
        }
        // C: overwrite a random non-always ware.
        let candidates: Vec<usize> = store
            .wares
            .iter()
            .enumerate()
            .filter(|(_, ware)| ware.as_ref().is_some_and(|ware| !ware.always))
            .map(|(slot, _)| slot)
            .collect();
        if candidates.is_empty() {
            return;
        }
        let roll =
            legacy_random_below_from_seed(&mut self.legacy_random_seed, candidates.len() as u32)
                as usize;
        if let Some(store) = self.merchant_stores.get_mut(&merchant_id) {
            store.wares[candidates[roll]] = Some(StoreWare {
                item,
                count: 1,
                always: false,
            });
        }
    }

    /// C `sell(cn, co, nr)`: the player buys ware `slot` from the merchant.
    pub fn merchant_store_buy(
        &mut self,
        merchant_id: CharacterId,
        player_id: CharacterId,
        slot: usize,
    ) -> MerchantTradeResult {
        if slot >= MERCHANT_STORE_SIZE {
            return MerchantTradeResult::Rejected;
        }
        if self
            .characters
            .get(&player_id)
            .is_none_or(|player| player.cursor_item.is_some())
        {
            return MerchantTradeResult::CursorOccupied;
        }
        let (barter, trader) = self.merchant_barter_and_trader(player_id);
        let Some(store) = self.merchant_stores.get(&merchant_id) else {
            return MerchantTradeResult::Rejected;
        };
        let Some(ware) = store.wares.get(slot).and_then(Option::as_ref) else {
            return MerchantTradeResult::SoldOut;
        };
        if ware.count < 1 {
            return MerchantTradeResult::SoldOut;
        }
        let price = merchant_sales_price(ware.item.value, store.price_multi, barter, trader);
        if self
            .characters
            .get(&player_id)
            .is_none_or(|player| player.gold < price)
        {
            return MerchantTradeResult::TooExpensive;
        }

        let mut item = ware.item.clone();
        item.id = self.next_runtime_item_id();
        item.carried_by = Some(player_id);
        item.contained_in = None;
        item.x = 0;
        item.y = 0;
        let item_id = item.id;
        self.items.insert(item_id, item);

        if let Some(player) = self.characters.get_mut(&player_id) {
            player.gold -= price;
            player.cursor_item = Some(item_id);
            player.flags.insert(CharacterFlags::ITEMS);
        }
        if let Some(store) = self.merchant_stores.get_mut(&merchant_id) {
            store.gold += i64::from(price);
            if let Some(ware) = store.wares.get_mut(slot).and_then(Option::as_mut) {
                if !ware.always {
                    ware.count -= 1;
                    if ware.count == 0 {
                        store.wares[slot] = None;
                    }
                }
            }
        }
        MerchantTradeResult::Traded(price)
    }

    /// C `buy(cn, co)`: the player sells the cursor item to the merchant.
    pub fn merchant_store_sell(
        &mut self,
        merchant_id: CharacterId,
        player_id: CharacterId,
    ) -> MerchantTradeResult {
        let Some(cursor_id) = self
            .characters
            .get(&player_id)
            .and_then(|player| player.cursor_item)
        else {
            return MerchantTradeResult::NoCursorItem;
        };
        let Some(item) = self.items.get(&cursor_id).cloned() else {
            return MerchantTradeResult::NoCursorItem;
        };
        if !self.merchant_stores.contains_key(&merchant_id) {
            return MerchantTradeResult::Rejected;
        }
        let (barter, trader) = self.merchant_barter_and_trader(player_id);
        let restricted = item.flags.intersects(
            ItemFlags::QUEST
                | ItemFlags::NODEPOT
                | ItemFlags::BONDTAKE
                | ItemFlags::LABITEM
                | ItemFlags::MONEY,
        );
        let price = merchant_buy_price(
            item.value,
            item.flags.contains(ItemFlags::MONEY),
            barter,
            trader,
        );
        if !restricted {
            let mut ware_item = item.clone();
            ware_item.carried_by = None;
            self.add_item_to_merchant_store(merchant_id, ware_item);
        }
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.cursor_item = None;
            player.gold = player.gold.saturating_add(price);
            player.flags.insert(CharacterFlags::ITEMS);
        }
        self.destroy_item(cursor_id);
        if let Some(store) = self.merchant_stores.get_mut(&merchant_id) {
            store.gold -= i64::from(price);
        }
        MerchantTradeResult::Traded(price)
    }

    /// C `check_merchant` from `src/system/act.c`: drop the active merchant
    /// when the player is busy or can no longer see the merchant.
    pub fn check_merchant(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let Some(merchant_id) = character.merchant else {
            return;
        };
        let mut clear = false;
        if character.action != action::IDLE && character.action != action::BLESS_SELF {
            clear = true;
        } else {
            match self.characters.get(&merchant_id) {
                Some(merchant) => {
                    if !self.merchant_stores.contains_key(&merchant_id)
                        || !char_see_char(character, merchant, &self.map, self.date.daylight)
                    {
                        clear = true;
                    }
                }
                None => clear = true,
            }
        }
        if clear {
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.merchant = None;
            }
        }
    }

    /// Merchant NPC tick: create the store, greet nearby players, react to
    /// trade requests and given items. Ports the message loop core of C
    /// `merchant_driver`.
    pub fn process_merchant_actions(&mut self) {
        let merchant_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_MERCHANT
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for merchant_id in merchant_ids {
            self.ensure_merchant_store(merchant_id);
            self.process_merchant_messages(merchant_id);
            self.greet_nearby_players(merchant_id);
            self.clear_expired_merchant_memory(merchant_id);
        }
    }

    fn process_merchant_messages(&mut self, merchant_id: CharacterId) {
        let Some(merchant) = self.characters.get_mut(&merchant_id) else {
            return;
        };
        let merchant_name = merchant.name.clone();
        let messages = std::mem::take(&mut merchant.driver_messages);
        let mut destroy_cursor = false;
        let mut trade_requests: Vec<CharacterId> = Vec::new();
        let mut small_talk: Vec<(CharacterId, String)> = Vec::new();

        for message in messages {
            match message.message_type {
                NT_TEXT => {
                    // C: talk containing "<merchant name>" and "trade".
                    let speaker_id = CharacterId(message.dat3 as u32);
                    if speaker_id != merchant_id {
                        if let Some(text) = message.text.as_deref() {
                            let lower = text.to_ascii_lowercase();
                            if lower.contains(&merchant_name.to_ascii_lowercase())
                                && lower.contains("trade")
                            {
                                trade_requests.push(speaker_id);
                            }

                            // C `analyse_text_driver` shared guard clauses:
                            // ignore non-players, distance > 12, and
                            // characters we can't see.
                            if let Some(reply) = self.merchant_qa_reply(
                                merchant_id,
                                &merchant_name,
                                speaker_id,
                                text,
                            ) {
                                small_talk.push((speaker_id, reply));
                            }
                        }
                    }
                }
                NT_GIVE => {
                    destroy_cursor = true;
                }
                _ => {}
            }
        }

        if destroy_cursor {
            // C: received items vanish.
            let cursor = self
                .characters
                .get_mut(&merchant_id)
                .and_then(|merchant| merchant.cursor_item.take());
            if let Some(item_id) = cursor {
                self.destroy_item(item_id);
            }
        }
        for player_id in trade_requests {
            if let Some(player) = self.characters.get_mut(&player_id) {
                player.merchant = Some(merchant_id);
            }
        }
        for (_, reply) in small_talk {
            self.merchant_quiet_say(merchant_id, &merchant_name, &reply);
        }
    }

    /// C `analyse_text_driver` from `src/module/merchants/merchant.c`,
    /// wired through the generic [`crate::character_driver::analyse_text_qa`]
    /// matcher. Returns the formatted reply text `quiet_say` would emit, or
    /// `None` if a guard clause rejected the message or no qa entry matched.
    fn merchant_qa_reply(
        &self,
        merchant_id: CharacterId,
        merchant_name: &str,
        speaker_id: CharacterId,
        text: &str,
    ) -> Option<String> {
        let merchant = self.characters.get(&merchant_id)?;
        let speaker = self.characters.get(&speaker_id)?;
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return None;
        }
        if char_dist(merchant, speaker) > 12 {
            return None;
        }
        if !char_see_char(merchant, speaker, &self.map, self.date.daylight) {
            return None;
        }
        match crate::character_driver::analyse_text_qa(
            text,
            merchant_name,
            &speaker.name,
            crate::character_driver::MERCHANT_QA,
        ) {
            crate::character_driver::TextAnalysisOutcome::Said(reply) => Some(reply),
            // C: `answer_code == 1` -> `quiet_say(cn, "I'm %s.", ch[cn].name)`.
            crate::character_driver::TextAnalysisOutcome::Matched(1) => {
                Some(format!("I'm {merchant_name}."))
            }
            _ => None,
        }
    }

    fn merchant_quiet_say(&mut self, merchant_id: CharacterId, merchant_name: &str, text: &str) {
        let Some(merchant) = self.characters.get(&merchant_id) else {
            return;
        };
        // C `quiet_say`: `log_area(..., LOG_TALK, cn, quietsay_dist, ...)`.
        let max_distance = self.settings.quietsay_dist.max(0) as u16;
        let say = crate::log_text::say_message(merchant_name, text);
        self.pending_area_texts.push(WorldAreaText {
            x: merchant.x,
            y: merchant.y,
            max_distance,
            message: String::from_utf8_lossy(&say).into_owned(),
        });
    }

    fn greet_nearby_players(&mut self, merchant_id: CharacterId) {
        let Some(merchant) = self.characters.get(&merchant_id).cloned() else {
            return;
        };
        let greet_distance = if merchant.name == "Fred" {
            FRED_GREET_DISTANCE
        } else {
            MERCHANT_GREET_DISTANCE
        };

        let mut greetings: Vec<(CharacterId, String)> = Vec::new();
        for character in self.characters.values() {
            if character.id == merchant_id
                || !character.flags.contains(CharacterFlags::PLAYER)
                || mem_check_driver(
                    &merchant.driver_memory,
                    MERCHANT_GREET_MEMORY_SLOT,
                    character.id.0,
                )
            {
                continue;
            }
            if char_dist(&merchant, character) > greet_distance {
                continue;
            }
            if !char_see_char(&merchant, character, &self.map, self.date.daylight) {
                continue;
            }
            greetings.push((
                character.id,
                format!(
                    "Hello {}! If you'd like to trade, say: '{}, trade'!",
                    character.name, merchant.name
                ),
            ));
        }

        for (player_id, greeting) in &greetings {
            let say = crate::log_text::say_message(&merchant.name, greeting);
            self.pending_area_texts.push(WorldAreaText {
                x: merchant.x,
                y: merchant.y,
                max_distance: SAY_DIST as u16,
                message: String::from_utf8_lossy(&say).into_owned(),
            });
            if let Some(merchant) = self.characters.get_mut(&merchant_id) {
                mem_add_driver(
                    &mut merchant.driver_memory,
                    MERCHANT_GREET_MEMORY_SLOT,
                    player_id.0,
                );
            }
        }
    }

    fn clear_expired_merchant_memory(&mut self, merchant_id: CharacterId) {
        let tick = self.tick.0;
        if let Some(merchant) = self.characters.get_mut(&merchant_id) {
            let memory_clear_tick = match merchant.driver_state.as_ref() {
                Some(CharacterDriverState::Merchant(data)) => data.memory_clear_tick,
                _ => return,
            };
            if tick > memory_clear_tick {
                mem_erase_driver(&mut merchant.driver_memory, MERCHANT_GREET_MEMORY_SLOT);
                if let Some(CharacterDriverState::Merchant(data)) = merchant.driver_state.as_mut() {
                    data.memory_clear_tick = tick + MERCHANT_MEMORY_CLEAR_TICKS;
                }
            }
        }
    }
}
