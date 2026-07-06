use super::*;

#[derive(Debug, Clone)]
pub(crate) struct AccountDepotState {
    pub(crate) slots: Vec<Option<Item>>,
}

#[allow(dead_code)]
pub(crate) mod legacy_account_depot_codec {
    use super::*;

    pub(crate) const LEGACY_ACCOUNT_DEPOT_ITEM_SIZE: usize = 232;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_ITEM_PERSISTED_PREFIX: usize = 224;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_FLAGS_OFFSET: usize = 0;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_NAME_OFFSET: usize = 8;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET: usize = 48;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET: usize = 128;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET: usize = 132;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_MAX_LEVEL_OFFSET: usize = 133;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_NEEDS_CLASS_OFFSET: usize = 134;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_OWNER_OFFSET: usize = 136;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET: usize = 140;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_MOD_VALUE_OFFSET: usize = 150;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_CONTENT_OFFSET: usize = 168;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET: usize = 170;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET: usize = 172;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET: usize = 212;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_SERIAL_OFFSET: usize = 216;
    pub(crate) const LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET: usize = 220;

    pub(crate) fn write_fixed_c_string(dst: &mut [u8], value: &str) {
        dst.fill(0);
        let bytes = value.as_bytes();
        let len = bytes.len().min(dst.len().saturating_sub(1));
        dst[..len].copy_from_slice(&bytes[..len]);
    }

    pub(crate) fn read_fixed_c_string(src: &[u8]) -> String {
        let len = src.iter().position(|&byte| byte == 0).unwrap_or(src.len());
        String::from_utf8_lossy(&src[..len]).into_owned()
    }

    pub(crate) fn encode_legacy_account_depot_item(
        item: &Item,
    ) -> [u8; LEGACY_ACCOUNT_DEPOT_ITEM_SIZE] {
        let mut bytes = [0u8; LEGACY_ACCOUNT_DEPOT_ITEM_SIZE];
        bytes[LEGACY_ACCOUNT_DEPOT_FLAGS_OFFSET..LEGACY_ACCOUNT_DEPOT_FLAGS_OFFSET + 8]
            .copy_from_slice(&item.flags.bits().to_le_bytes());
        write_fixed_c_string(
            &mut bytes[LEGACY_ACCOUNT_DEPOT_NAME_OFFSET..LEGACY_ACCOUNT_DEPOT_NAME_OFFSET + 40],
            &item.name,
        );
        write_fixed_c_string(
            &mut bytes[LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET
                ..LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET + 80],
            &item.description,
        );
        bytes[LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET..LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET + 4]
            .copy_from_slice(&item.value.to_le_bytes());
        bytes[LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET] = item.min_level;
        bytes[LEGACY_ACCOUNT_DEPOT_MAX_LEVEL_OFFSET] = item.max_level;
        bytes[LEGACY_ACCOUNT_DEPOT_NEEDS_CLASS_OFFSET] = item.needs_class;
        bytes[LEGACY_ACCOUNT_DEPOT_OWNER_OFFSET..LEGACY_ACCOUNT_DEPOT_OWNER_OFFSET + 4]
            .copy_from_slice(&item.owner_id.to_le_bytes());
        for index in 0..ugaris_core::entity::MAX_MODIFIERS {
            let base = LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET + index * 2;
            bytes[base..base + 2].copy_from_slice(&item.modifier_index[index].to_le_bytes());
            let base = LEGACY_ACCOUNT_DEPOT_MOD_VALUE_OFFSET + index * 2;
            bytes[base..base + 2].copy_from_slice(&item.modifier_value[index].to_le_bytes());
        }
        bytes[LEGACY_ACCOUNT_DEPOT_CONTENT_OFFSET..LEGACY_ACCOUNT_DEPOT_CONTENT_OFFSET + 2]
            .copy_from_slice(&item.content_id.to_le_bytes());
        bytes[LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET..LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET + 2]
            .copy_from_slice(&item.driver.to_le_bytes());
        let drdata_len = item.driver_data.len().min(40);
        bytes[LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET..LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET + drdata_len]
            .copy_from_slice(&item.driver_data[..drdata_len]);
        bytes[LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET..LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET + 4]
            .copy_from_slice(&item.template_id.to_le_bytes());
        bytes[LEGACY_ACCOUNT_DEPOT_SERIAL_OFFSET..LEGACY_ACCOUNT_DEPOT_SERIAL_OFFSET + 4]
            .copy_from_slice(&item.serial.to_le_bytes());
        bytes[LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET..LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET + 4]
            .copy_from_slice(&item.sprite.to_le_bytes());
        bytes
    }

    pub(crate) fn decode_legacy_account_depot_item(bytes: &[u8], slot: usize) -> Option<Item> {
        if bytes.len() < LEGACY_ACCOUNT_DEPOT_ITEM_PERSISTED_PREFIX {
            return None;
        }
        let read_u16 = |offset: usize| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let read_i16 = |offset: usize| i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let read_u32 = |offset: usize| {
            u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ])
        };
        let read_i32 = |offset: usize| {
            i32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ])
        };
        let flags = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let mut modifier_index = [0i16; ugaris_core::entity::MAX_MODIFIERS];
        let mut modifier_value = [0i16; ugaris_core::entity::MAX_MODIFIERS];
        for index in 0..ugaris_core::entity::MAX_MODIFIERS {
            modifier_index[index] = read_i16(LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET + index * 2);
            modifier_value[index] = read_i16(LEGACY_ACCOUNT_DEPOT_MOD_VALUE_OFFSET + index * 2);
        }
        Some(Item {
            id: ItemId((slot + 1) as u32),
            name: read_fixed_c_string(
                &bytes[LEGACY_ACCOUNT_DEPOT_NAME_OFFSET..LEGACY_ACCOUNT_DEPOT_NAME_OFFSET + 40],
            ),
            description: read_fixed_c_string(
                &bytes[LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET
                    ..LEGACY_ACCOUNT_DEPOT_DESCRIPTION_OFFSET + 80],
            ),
            flags: ItemFlags::from_bits_retain(flags),
            sprite: read_i32(LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET),
            value: read_u32(LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET),
            min_level: bytes[LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET],
            max_level: bytes[LEGACY_ACCOUNT_DEPOT_MAX_LEVEL_OFFSET],
            needs_class: bytes[LEGACY_ACCOUNT_DEPOT_NEEDS_CLASS_OFFSET],
            template_id: read_u32(LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET),
            owner_id: read_i32(LEGACY_ACCOUNT_DEPOT_OWNER_OFFSET),
            modifier_index,
            modifier_value,
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: read_u16(LEGACY_ACCOUNT_DEPOT_CONTENT_OFFSET),
            driver: read_u16(LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET),
            driver_data: bytes
                [LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET..LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET + 40]
                .to_vec(),
            serial: read_u32(LEGACY_ACCOUNT_DEPOT_SERIAL_OFFSET),
        })
    }

    pub(crate) fn encode_legacy_account_depot_blob(depot: &AccountDepotState) -> Vec<u8> {
        let mut bytes = Vec::new();
        for item in depot.slots.iter().flatten() {
            bytes.extend_from_slice(&encode_legacy_account_depot_item(item));
        }
        bytes
    }

    pub(crate) fn decode_legacy_account_depot_blob(bytes: &[u8]) -> AccountDepotState {
        let mut depot = AccountDepotState::default();
        for (slot, chunk) in bytes
            .chunks_exact(LEGACY_ACCOUNT_DEPOT_ITEM_SIZE)
            .take(depot.slots.len())
            .enumerate()
        {
            depot.slots[slot] = decode_legacy_account_depot_item(chunk, slot);
        }
        depot
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LegacySubscriberBlock<'a> {
    pub(crate) id: u32,
    pub(crate) data: &'a [u8],
}

pub(crate) fn parse_legacy_subscriber_blocks(
    bytes: &[u8],
) -> Option<Vec<LegacySubscriberBlock<'_>>> {
    let mut blocks = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        if bytes.len().saturating_sub(offset) < 8 {
            return None;
        }
        let id = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?);
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().ok()?) as usize;
        offset += 8;
        if bytes.len().saturating_sub(offset) < size {
            return None;
        }
        blocks.push(LegacySubscriberBlock {
            id,
            data: &bytes[offset..offset + size],
        });
        offset += size;
    }
    Some(blocks)
}

pub(crate) fn write_legacy_subscriber_block(bytes: &mut Vec<u8>, id: u32, data: &[u8]) {
    bytes.extend_from_slice(&id.to_le_bytes());
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data);
}

pub(crate) fn account_depot_has_items(depot: &AccountDepotState) -> bool {
    depot.slots.iter().any(Option::is_some)
}

pub(crate) fn decode_legacy_account_depot_subscriber_blob(
    bytes: &[u8],
) -> Option<AccountDepotState> {
    parse_legacy_subscriber_blocks(bytes)?
        .into_iter()
        .find(|block| block.id == DRD_ACCOUNT_WIDE_DEPOT)
        .map(|block| decode_legacy_account_depot_blob(block.data))
}

pub(crate) fn encode_legacy_account_depot_subscriber_blob(
    existing: &[u8],
    depot: Option<&AccountDepotState>,
) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(existing.len());
    let Some(blocks) = parse_legacy_subscriber_blocks(existing) else {
        return existing.to_vec();
    };
    let mut had_account_depot = false;
    for block in blocks {
        if block.id == DRD_ACCOUNT_WIDE_DEPOT {
            had_account_depot = true;
            if let Some(depot) = depot.filter(|depot| account_depot_has_items(depot)) {
                write_legacy_subscriber_block(
                    &mut encoded,
                    DRD_ACCOUNT_WIDE_DEPOT,
                    &encode_legacy_account_depot_blob(depot),
                );
            }
        } else {
            write_legacy_subscriber_block(&mut encoded, block.id, block.data);
        }
    }
    if !had_account_depot {
        if let Some(depot) = depot.filter(|depot| account_depot_has_items(depot)) {
            write_legacy_subscriber_block(
                &mut encoded,
                DRD_ACCOUNT_WIDE_DEPOT,
                &encode_legacy_account_depot_blob(depot),
            );
        }
    }
    encoded
}

impl Default for AccountDepotState {
    fn default() -> Self {
        Self {
            slots: vec![None; ugaris_core::entity::INVENTORY_SIZE],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AccountDepotCommandResult {
    Ignored,
    Changed,
    Look(String),
    Blocked(String),
}

pub(crate) fn account_depot_payload(depot: &AccountDepotState) -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    builder
        .container_type(1)
        .container_name("Your Account Depot")
        .container_count(depot.slots.len().min(u8::MAX as usize) as u8);
    for (slot, item) in depot.slots.iter().enumerate().take(u8::MAX as usize + 1) {
        builder.container_item(
            slot as u8,
            item.as_ref()
                .map(|item| item.sprite.max(0) as u32)
                .unwrap_or(0),
        );
    }
    builder.into_payload()
}

pub(crate) fn apply_account_depot_command(
    world: &mut World,
    depot: &mut AccountDepotState,
    character_id: CharacterId,
    action: &ClientAction,
) -> AccountDepotCommandResult {
    if !check_current_container(world, character_id) {
        return AccountDepotCommandResult::Ignored;
    }
    let Some(character) = world.characters.get_mut(&character_id) else {
        return AccountDepotCommandResult::Ignored;
    };
    let Some(container_id) = character.current_container else {
        return AccountDepotCommandResult::Ignored;
    };
    if world
        .items
        .get(&container_id)
        .is_none_or(|item| item.driver != IDR_ACCOUNT_DEPOT)
    {
        return AccountDepotCommandResult::Ignored;
    }

    match *action {
        ClientAction::Container { slot, fast } => {
            let slot = usize::from(slot);
            if slot >= depot.slots.len() {
                return AccountDepotCommandResult::Ignored;
            }
            if fast && character.cursor_item.is_some() {
                return account_depot_store_cursor(world, depot, character_id);
            }
            account_depot_swap_slot(world, depot, character_id, slot)
        }
        ClientAction::LookContainer { slot } => {
            let Some(character) = world.characters.get(&character_id) else {
                return AccountDepotCommandResult::Ignored;
            };
            depot
                .slots
                .get(usize::from(slot))
                .and_then(Option::as_ref)
                .map(|item| legacy_item_look_text(item, character))
                .filter(|text| !text.is_empty())
                .map(AccountDepotCommandResult::Look)
                .unwrap_or(AccountDepotCommandResult::Ignored)
        }
        _ => AccountDepotCommandResult::Ignored,
    }
}

pub(crate) fn account_depot_swap_slot(
    world: &mut World,
    depot: &mut AccountDepotState,
    character_id: CharacterId,
    slot: usize,
) -> AccountDepotCommandResult {
    let cursor_id = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item);
    if let Some(cursor_id) = cursor_id {
        if world
            .items
            .get(&cursor_id)
            .is_some_and(|item| item.flags.intersects(ItemFlags::QUEST | ItemFlags::NODEPOT))
        {
            return AccountDepotCommandResult::Blocked(
                "You cannot store this item in the depot.".to_string(),
            );
        }
    }

    let withdrawn = depot.slots[slot].take();
    let stored = cursor_id.and_then(|item_id| world.items.remove(&item_id));

    if let Some(character) = world.characters.get_mut(&character_id) {
        character.cursor_item = None;
    } else {
        return AccountDepotCommandResult::Ignored;
    }
    if let Some(mut item) = stored {
        item.carried_by = None;
        item.contained_in = world
            .characters
            .get(&character_id)
            .and_then(|character| character.current_container);
        item.x = 0;
        item.y = 0;
        depot.slots[slot] = Some(item);
    }
    if let Some(mut item) = withdrawn {
        item.id = next_runtime_item_id(world);
        item.carried_by = Some(character_id);
        item.contained_in = None;
        item.x = 0;
        item.y = 0;
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.cursor_item = Some(item.id);
        }
        world.items.insert(item.id, item);
    }
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.flags.insert(CharacterFlags::ITEMS);
    }
    AccountDepotCommandResult::Changed
}

pub(crate) fn account_depot_store_cursor(
    world: &mut World,
    depot: &mut AccountDepotState,
    character_id: CharacterId,
) -> AccountDepotCommandResult {
    let Some(empty_slot) = depot.slots.iter().position(Option::is_none) else {
        return AccountDepotCommandResult::Ignored;
    };
    account_depot_swap_slot(world, depot, character_id, empty_slot)
}

pub(crate) fn account_depot_sort(depot: &mut AccountDepotState) {
    depot.slots.sort_by(|left, right| match (left, right) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(left), Some(right)) => right
            .sprite
            .cmp(&left.sprite)
            .then_with(|| right.value.cmp(&left.value))
            .then_with(|| {
                left.name[..left.name.len().min(35)].cmp(&right.name[..right.name.len().min(35)])
            }),
    });
}

pub(crate) fn account_depot_sort_if_open(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
) -> bool {
    if !check_current_container(world, character_id) {
        return false;
    }
    let Some(container_id) = world
        .characters
        .get(&character_id)
        .and_then(|character| character.current_container)
    else {
        return false;
    };
    if world
        .items
        .get(&container_id)
        .is_none_or(|item| item.driver != IDR_ACCOUNT_DEPOT)
    {
        return false;
    }
    let Some(depot) = runtime.account_depots.get_mut(&character_id) else {
        return false;
    };
    account_depot_sort(depot);
    true
}

// C `struct depot_ppd { struct item itm[MAXDEPOT]; }`
// (`src/system/depot.h:19-23`, `DRD_DEPOT_PPD`): the character's own
// 80-slot legacy storage depot, opened via any item with the `IF_DEPOT`
// flag (`src/system/act.c:1755`/`player.c:3282`'s `it[in].flags &
// IF_DEPOT` checks). Unlike `AccountDepotState` above (a distinct, newer,
// account-wide system stored outside `PlayerRuntime` in a subscriber
// blob), the backing `Vec<Option<Item>>` lives directly on
// `PlayerRuntime::depot` (`ugaris-core`, a real per-character PPD) since
// this system has no account-wide scope to justify a separate
// `ServerRuntime`-owned map.

/// C `player_depot(int cn, int nr, int flag, int fast)` (`depot.c:89-120`)
/// dispatch entry, called for `CL_CONTAINER`/`CL_CONTAINER_FAST`/
/// `CL_LOOK_CONTAINER` whenever the open container item has the
/// `IF_DEPOT` flag (`player.c:1090/1121/1154/3282-3283`).
pub(crate) fn apply_personal_depot_command(
    world: &mut World,
    depot: &mut Vec<Option<Item>>,
    character_id: CharacterId,
    action: &ClientAction,
) -> AccountDepotCommandResult {
    if !check_current_container(world, character_id) {
        return AccountDepotCommandResult::Ignored;
    }
    let Some(character) = world.characters.get(&character_id) else {
        return AccountDepotCommandResult::Ignored;
    };
    let Some(container_id) = character.current_container else {
        return AccountDepotCommandResult::Ignored;
    };
    if world
        .items
        .get(&container_id)
        .is_none_or(|item| !item.flags.contains(ItemFlags::DEPOT))
    {
        return AccountDepotCommandResult::Ignored;
    }

    match *action {
        ClientAction::Container { slot, fast } => {
            let slot = usize::from(slot);
            if slot >= depot.len() {
                return AccountDepotCommandResult::Ignored;
            }
            let citem_present = world
                .characters
                .get(&character_id)
                .is_some_and(|character| character.cursor_item.is_some());
            // C: `if (fast && ch[cn].citem) { for (nr=0; nr<MAXDEPOT;
            // nr++) if (!ppd->itm[nr].flags) break; if (nr==MAXDEPOT)
            // return; swap_depot(cn, nr); }` - the client-supplied slot is
            // ignored entirely; the first empty depot slot is used
            // instead, and a full depot is a silent no-op.
            if fast && citem_present {
                let Some(empty_slot) = depot.iter().position(Option::is_none) else {
                    return AccountDepotCommandResult::Ignored;
                };
                personal_depot_swap_slot(world, depot, character_id, empty_slot)
            } else {
                // C: `else { swap_depot(cn, nr); if (fast)
                // store_citem(cn); }` - `store_citem` runs unconditionally
                // whenever `fast` is set, even if the swap itself was
                // blocked (e.g. `IF_NODEPOT`).
                let result = personal_depot_swap_slot(world, depot, character_id, slot);
                if fast {
                    store_cursor_item_into_inventory(world, character_id);
                }
                result
            }
        }
        ClientAction::LookContainer { slot } => {
            let Some(character) = world.characters.get(&character_id) else {
                return AccountDepotCommandResult::Ignored;
            };
            depot
                .get(usize::from(slot))
                .and_then(Option::as_ref)
                .map(|item| legacy_item_look_text(item, character))
                .filter(|text| !text.is_empty())
                .map(AccountDepotCommandResult::Look)
                .unwrap_or(AccountDepotCommandResult::Ignored)
        }
        _ => AccountDepotCommandResult::Ignored,
    }
}

/// C `swap_depot(int cn, int nr)` (`depot.c:30-85`): swaps the held
/// cursor item with depot slot `nr`. Unlike `account_depot_swap_slot`,
/// only `IF_NODEPOT` blocks the store (`depot.c:51-54`) - `IF_QUEST`
/// items are NOT blocked here (a real, intentional divergence from the
/// account-wide depot; this is exactly why `turn_seyan`'s
/// `PlayerRuntime::clear_turn_seyan_ppd` has to sweep quest items back
/// out of this depot, `tool.c:4379-4388`) - and a blocked store is a
/// silent no-op (C never `log_char`s here, unlike account depot's own
/// "You cannot store this item in the depot." message).
pub(crate) fn personal_depot_swap_slot(
    world: &mut World,
    depot: &mut [Option<Item>],
    character_id: CharacterId,
    slot: usize,
) -> AccountDepotCommandResult {
    if slot >= depot.len() {
        return AccountDepotCommandResult::Ignored;
    }
    let cursor_id = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item);
    if cursor_id.is_some_and(|item_id| {
        world
            .items
            .get(&item_id)
            .is_some_and(|item| item.flags.contains(ItemFlags::NODEPOT))
    }) {
        return AccountDepotCommandResult::Ignored;
    }

    let withdrawn = depot[slot].take();
    let stored = cursor_id.and_then(|item_id| world.items.remove(&item_id));

    if let Some(character) = world.characters.get_mut(&character_id) {
        character.cursor_item = None;
    } else {
        return AccountDepotCommandResult::Ignored;
    }
    if let Some(mut item) = stored {
        item.carried_by = None;
        item.contained_in = None;
        item.x = 0;
        item.y = 0;
        depot[slot] = Some(item);
    }
    if let Some(mut item) = withdrawn {
        item.id = next_runtime_item_id(world);
        item.carried_by = Some(character_id);
        item.contained_in = None;
        item.x = 0;
        item.y = 0;
        if let Some(character) = world.characters.get_mut(&character_id) {
            character.cursor_item = Some(item.id);
        }
        world.items.insert(item.id, item);
    }
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.flags.insert(CharacterFlags::ITEMS);
    }
    AccountDepotCommandResult::Changed
}

/// C `store_citem(int cn)`: auto-places the character's held cursor item
/// into the first free ordinary inventory slot, matching the same
/// `INVENTORY_START_INVENTORY..`-onward scan `apply_item_container_swap`
/// already uses for generic containers' own fast-withdraw path. A no-op
/// (not an error) when the cursor is empty or every inventory slot is
/// full, matching C silently leaving the item on the cursor.
pub(crate) fn store_cursor_item_into_inventory(world: &mut World, character_id: CharacterId) {
    let Some(character) = world.characters.get_mut(&character_id) else {
        return;
    };
    let Some(item_id) = character.cursor_item else {
        return;
    };
    if let Some(slot) = character
        .inventory
        .iter_mut()
        .skip(INVENTORY_START_INVENTORY)
        .find(|slot| slot.is_none())
    {
        *slot = Some(item_id);
        character.cursor_item = None;
        character.flags.insert(CharacterFlags::ITEMS);
    }
}

/// C `player_act`'s depot-view branch (`player.c:3282-3313`): container
/// name "Your Depot", `MAXDEPOT` (80) slots.
pub(crate) fn personal_depot_payload(depot: &[Option<Item>]) -> bytes::BytesMut {
    let mut builder = PacketBuilder::new();
    builder
        .container_type(1)
        .container_name("Your Depot")
        .container_count(depot.len().min(u8::MAX as usize) as u8);
    for (slot, item) in depot.iter().enumerate().take(u8::MAX as usize + 1) {
        builder.container_item(
            slot as u8,
            item.as_ref()
                .map(|item| item.sprite.max(0) as u32)
                .unwrap_or(0),
        );
    }
    builder.into_payload()
}

/// C `depot_cmp`/`depot_sort` (`depot.c:122-159`): sort by sprite
/// descending, then value descending, then name (first 35 bytes)
/// ascending, empty slots sorted last - identical ordering to
/// `account_depot_sort`'s `AccountDepotState::slots`, just over a plain
/// `Vec<Option<Item>>` instead.
pub(crate) fn personal_depot_sort(depot: &mut [Option<Item>]) {
    depot.sort_by(|left, right| match (left, right) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(left), Some(right)) => right
            .sprite
            .cmp(&left.sprite)
            .then_with(|| right.value.cmp(&left.value))
            .then_with(|| {
                left.name[..left.name.len().min(35)].cmp(&right.name[..right.name.len().min(35)])
            }),
    });
}

/// C `/depotsort` (`command.c:9350-9357`, dispatching to `depot_sort`,
/// no permission gate, no `IF_DEPOT` open-container requirement in C
/// either - `depot_sort` just calls `set_data(cn, DRD_DEPOT_PPD, ...)`
/// directly, so it works even when the depot isn't currently open).
/// Unlike `account_depot_sort_if_open`, this always sorts (there's no
/// "open" precondition to check in C), so it never fails.
pub(crate) fn personal_depot_sort_command(runtime: &mut ServerRuntime, character_id: CharacterId) {
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    personal_depot_sort(&mut player.depot);
}
