//! Merchant store client views and trade commands.
//!
//! Ports the merchant slices of `src/system/player.c` (con_type 2 store
//! views with `SV_PRICE`/`SV_ITEMPRICE`) and `player_store` from
//! `src/module/merchants/store.c`.

use super::*;

/// C store view: container type 2 with the merchant name, ware sprites,
/// per-slot sale prices, and per-inventory-slot buy prices.
pub(crate) fn merchant_store_payload(
    world: &mut World,
    character_id: CharacterId,
) -> Option<bytes::BytesMut> {
    let clan_bonus = world.clan_trade_bonus(character_id);
    let character = world.characters.get(&character_id)?;
    let merchant_id = character.merchant?;
    let merchant = world.characters.get(&merchant_id)?;
    let store = world.merchant_stores.get(&merchant_id)?;

    let (barter, trader) = (
        i32::from(character.values[0][ugaris_core::entity::CharacterValue::Barter as usize])
            + clan_bonus,
        character
            .professions
            .get(ugaris_core::legacy::profession::TRADER)
            .copied()
            .map(i32::from)
            .unwrap_or_default(),
    );

    let mut builder = PacketBuilder::new();
    builder
        .container_type(2)
        .container_name(&merchant.name)
        .container_count(MERCHANT_STORE_SIZE.min(u8::MAX as usize) as u8);
    for slot in 0..MERCHANT_STORE_SIZE.min(u8::MAX as usize + 1) {
        let (sprite, price) = store
            .wares
            .get(slot)
            .and_then(Option::as_ref)
            .filter(|ware| ware.count > 0)
            .map(|ware| {
                (
                    ware.item.sprite.max(0) as u32,
                    merchant_sales_price(ware.item.value, store.price_multi, barter, trader),
                )
            })
            .unwrap_or((0, 0));
        builder.container_item(slot as u8, sprite);
        builder.container_price(slot as u8, price);
    }
    for slot in 0..character.inventory.len().min(u8::MAX as usize + 1) {
        let price = character.inventory[slot]
            .and_then(|item_id| world.items.get(&item_id))
            .map(|item| {
                merchant_buy_price(
                    item.value,
                    item.flags.contains(ItemFlags::MONEY),
                    barter,
                    trader,
                )
            })
            .unwrap_or(0);
        builder.item_price(slot as u8, price);
    }
    if let Some(cursor_id) = character.cursor_item {
        if let Some(item) = world.items.get(&cursor_id) {
            builder.cursor_price(merchant_buy_price(
                item.value,
                item.flags.contains(ItemFlags::MONEY),
                barter,
                trader,
            ));
        }
    }
    Some(builder.into_payload())
}

/// Empty container view sent when a merchant/store view closes.
pub(crate) fn container_close_payload() -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    builder.container_type(0);
    builder.into_payload()
}

/// C `save_merchant_inventory`: snapshot the current in-memory store for
/// persistence, keyed like C by the merchant's name and spawn position
/// (`ch[cn].name`/`tmpx`/`tmpy`).
pub(crate) fn merchant_store_snapshot(
    world: &World,
    merchant_id: CharacterId,
) -> Option<MerchantStoreSnapshot> {
    let merchant = world.characters.get(&merchant_id)?;
    let store = world.merchant_stores.get(&merchant_id)?;
    Some(MerchantStoreSnapshot {
        merchant_name: merchant.name.clone(),
        x: i32::from(merchant.x),
        y: i32::from(merchant.y),
        gold: store.gold,
        price_multi: store.price_multi,
        wares: store
            .wares
            .iter()
            .map(|ware| {
                ware.as_ref().map(|ware| MerchantWareSnapshot {
                    item: ware.item.clone(),
                    count: ware.count,
                    always: ware.always,
                })
            })
            .collect(),
    })
}

/// C `load_merchant_inventory`: overwrite the freshly created in-memory
/// store's gold/pricemulti and every saved ware slot with the persisted
/// snapshot.
pub(crate) fn apply_merchant_store_snapshot(
    world: &mut World,
    merchant_id: CharacterId,
    snapshot: MerchantStoreSnapshot,
) {
    let Some(store) = world.merchant_stores.get_mut(&merchant_id) else {
        return;
    };
    store.gold = snapshot.gold;
    store.price_multi = snapshot.price_multi;
    for (slot, ware) in snapshot.wares.into_iter().enumerate() {
        if slot >= store.wares.len() {
            break;
        }
        store.wares[slot] = ware.map(|ware| StoreWare {
            item: ware.item,
            count: ware.count,
            always: ware.always,
        });
    }
}

/// C `queue_merchant_gold_update`/`queue_merchant_item_*`: after a trade
/// mutates a store's gold or wares, persist the full snapshot (Rust has no
/// task queue, so this just re-saves inline like C's own
/// `add_item_to_merchant`/`remove_item_from_merchant`/`update_merchant_item`
/// helpers do). A no-op when no `--database-url` was configured.
pub(crate) async fn save_merchant_store_if_configured(
    world: &World,
    repository: &Option<ugaris_db::PgMerchantRepository>,
    merchant_id: CharacterId,
) {
    let Some(repository) = repository else {
        return;
    };
    let Some(snapshot) = merchant_store_snapshot(world, merchant_id) else {
        return;
    };
    let name = snapshot.merchant_name.clone();
    let (x, y) = (snapshot.x, snapshot.y);
    match repository.save_store(&snapshot).await {
        Ok(()) => {}
        Err(err) => {
            tracing::warn!(merchant = %name, x, y, error = %err, "failed to save merchant store after trade");
        }
    }
}

pub(crate) struct MerchantCommandResult {
    pub messages: Vec<String>,
    pub changed: bool,
}

/// C `price` feedback formatting: below one gold shows silver only.
fn legacy_price_text(prefix: &str, price: u32) -> String {
    if price < 100 {
        format!("{prefix} for {price}S")
    } else {
        format!("{prefix} for {}G {}S", price / 100, price % 100)
    }
}

/// C `player_store` (`cl_container` merchant branch): with a cursor item the
/// player sells, otherwise buys ware `slot`. `fast` stores the bought item
/// directly into the inventory like C `store_citem`.
pub(crate) fn apply_merchant_container_command(
    world: &mut World,
    character_id: CharacterId,
    merchant_id: CharacterId,
    action: &ClientAction,
) -> MerchantCommandResult {
    let mut result = MerchantCommandResult {
        messages: Vec::new(),
        changed: false,
    };
    match action {
        ClientAction::Container { slot, fast } => {
            let has_cursor = world
                .characters
                .get(&character_id)
                .is_some_and(|character| character.cursor_item.is_some());
            if has_cursor {
                match world.merchant_store_sell(merchant_id, character_id) {
                    MerchantTradeResult::Traded(price) => {
                        result.messages.push(legacy_price_text("Sold", price));
                        result.changed = true;
                    }
                    _ => {}
                }
            } else {
                match world.merchant_store_buy(merchant_id, character_id, usize::from(*slot)) {
                    MerchantTradeResult::Traded(price) => {
                        result.messages.push(legacy_price_text("Bought", price));
                        result.changed = true;
                        if *fast {
                            store_cursor_in_inventory(world, character_id);
                        }
                    }
                    MerchantTradeResult::TooExpensive => {
                        result
                            .messages
                            .push("Sorry, that's too expensive for you.".to_string());
                    }
                    _ => {}
                }
            }
        }
        ClientAction::LookContainer { slot } => {
            let ware_text = world
                .merchant_stores
                .get(&merchant_id)
                .and_then(|store| store.wares.get(usize::from(*slot)))
                .and_then(Option::as_ref)
                .filter(|ware| ware.count > 0)
                .map(|ware| (ware.item.clone(), ware.count));
            if let Some((item, _)) = ware_text {
                if let Some(character) = world.characters.get(&character_id) {
                    result
                        .messages
                        .push(legacy_item_look_text(&item, character));
                }
            }
        }
        _ => {}
    }
    result
}

pub(crate) struct FastSellResult {
    pub messages: Vec<String>,
    /// Set whenever the cursor/slot swap happened, even if the merchant
    /// sale itself didn't go through (matches C: `swap()` already moved the
    /// item before the quest/no-merchant checks run).
    pub inventory_changed: bool,
    /// Set only when the item was actually sold, so the merchant store
    /// view (prices, wares) is refreshed too.
    pub sold: bool,
}

/// C `cl_fastsell` (`src/system/player.c:877`): quick-sell an inventory slot
/// straight to the active merchant. Mirrors the C flow: `swap(cn, pos)`
/// picks the slot item up onto the cursor (swapping back whatever was
/// already held), `check_merchant` re-validates the open store, then a
/// quest-item guard blocks the shortcut before reusing `player_store`'s sell
/// path (`merchant_store_sell`).
///
/// REMAINING (scoped out for this task): C also falls through to
/// `check_container_item` + `player_depot`/`account_depot_store`/
/// `container` when no merchant is open (`ch[cn].con_in` branch). The
/// per-character legacy depot (`DRD_DEPOT_PPD`) isn't ported yet, so this
/// only implements the merchant branch; fast-selling into an open item
/// container or account depot from an inventory slot is not wired.
pub(crate) fn apply_fast_sell(
    world: &mut World,
    character_id: CharacterId,
    slot: usize,
) -> FastSellResult {
    let mut result = FastSellResult {
        messages: Vec::new(),
        inventory_changed: false,
        sold: false,
    };
    if !can_use_inventory_slot(slot) {
        return result;
    }

    // C: `if (!swap(cn, pos)) return;` then `if (!(in = ch[cn].citem)) return;`
    if inventory_swap_slot(world, character_id, slot) == InventoryCommandResult::Ignored {
        return result;
    }
    result.inventory_changed = true;
    let Some(cursor_id) = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item)
    else {
        return result;
    };

    // C: `if (ch[cn].merchant) check_merchant(cn);`
    world.check_merchant(character_id);
    let Some(merchant_id) = world
        .characters
        .get(&character_id)
        .and_then(|character| character.merchant)
    else {
        return result;
    };

    if world
        .items
        .get(&cursor_id)
        .is_some_and(|item| item.flags.contains(ItemFlags::QUEST))
    {
        result.messages.push(
            "You cannot quick-sell quest items (hold down SHIFT and LEFT-CLICK on the merchant's windows to go ahead)."
                .to_string(),
        );
        return result;
    }

    if let MerchantTradeResult::Traded(price) = world.merchant_store_sell(merchant_id, character_id)
    {
        result.messages.push(legacy_price_text("Sold", price));
        result.sold = true;
    }
    result
}

/// C `store_citem`: move the just-bought cursor item into the first free
/// carried inventory slot.
fn store_cursor_in_inventory(world: &mut World, character_id: CharacterId) {
    let Some(character) = world.characters.get_mut(&character_id) else {
        return;
    };
    let Some(cursor_id) = character.cursor_item else {
        return;
    };
    let free_slot = (ugaris_core::legacy::INVENTORY_START_INVENTORY..character.inventory.len())
        .find(|slot| character.inventory[*slot].is_none());
    if let Some(slot) = free_slot {
        character.inventory[slot] = Some(cursor_id);
        character.cursor_item = None;
        character
            .flags
            .insert(ugaris_core::entity::CharacterFlags::ITEMS);
    }
}
