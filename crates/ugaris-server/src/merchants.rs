//! Merchant store client views and trade commands.
//!
//! Ports the merchant slices of `src/system/player.c` (con_type 2 store
//! views with `SV_PRICE`/`SV_ITEMPRICE`) and `player_store` from
//! `src/module/merchants/store.c`.

use super::*;

/// C store view: container type 2 with the merchant name, ware sprites,
/// per-slot sale prices, and per-inventory-slot buy prices.
pub(crate) fn merchant_store_payload(
    world: &World,
    character_id: CharacterId,
) -> Option<bytes::BytesMut> {
    let character = world.characters.get(&character_id)?;
    let merchant_id = character.merchant?;
    let merchant = world.characters.get(&merchant_id)?;
    let store = world.merchant_stores.get(&merchant_id)?;

    let (barter, trader) = (
        i32::from(character.values[0][ugaris_core::entity::CharacterValue::Barter as usize]),
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
