use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfiniteChestTemplate {
    Rune1,
    Rune2,
    Rune3,
    Rune4,
    Rune5,
    Rune6,
    Rune7,
    Rune8,
    Rune9,
}

impl InfiniteChestTemplate {
    pub fn from_kind(kind: u8) -> Option<Self> {
        match kind {
            1 => Some(Self::Rune1),
            2 => Some(Self::Rune2),
            3 => Some(Self::Rune3),
            4 => Some(Self::Rune4),
            5 => Some(Self::Rune5),
            6 => Some(Self::Rune6),
            7 => Some(Self::Rune7),
            8 => Some(Self::Rune8),
            9 => Some(Self::Rune9),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rune1 => "rune1",
            Self::Rune2 => "rune2",
            Self::Rune3 => "rune3",
            Self::Rune4 => "rune4",
            Self::Rune5 => "rune5",
            Self::Rune6 => "rune6",
            Self::Rune7 => "rune7",
            Self::Rune8 => "rune8",
            Self::Rune9 => "rune9",
        }
    }
}

pub(crate) fn chestspawn_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        if drdata(item, 1) != 0 {
            return ItemDriverOutcome::Noop;
        }

        return match drdata(item, 0) {
            0 => ItemDriverOutcome::ChestSpawn {
                item_id: item.id,
                character_id: character.id,
                template: "normal_vampire",
                x: item.x,
                y: item.y,
                schedule_after_ticks: TICKS_PER_SECOND * 10,
            },
            _ => ItemDriverOutcome::Noop,
        };
    }

    if drdata(item, 1) == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::ChestSpawnCheck {
        item_id: item.id,
        character_id: character.id,
        spawned_character_id: CharacterId(u32::from(drdata_u16(item, 2))),
        schedule_after_ticks: TICKS_PER_SECOND * 10,
    }
}

pub(crate) fn chest_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::ChestTreasure {
        item_id: item.id,
        character_id: character.id,
        treasure_index: drdata(item, 0),
    }
}

pub(crate) fn randchest_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    ItemDriverOutcome::RandomChest {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn infinite_chest_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::InfiniteChestCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let required_key_id = drdata_u32(item, 1);
    let key_name = if required_key_id == 0 {
        None
    } else {
        match context
            .door_key
            .as_ref()
            .filter(|key| key.key_id == required_key_id)
        {
            Some(key) => Some(outcome_item_name(&key.name)),
            None => {
                return ItemDriverOutcome::InfiniteChestKeyRequired {
                    item_id: item.id,
                    character_id: character.id,
                };
            }
        }
    };

    let Some(template) = InfiniteChestTemplate::from_kind(drdata(item, 0)) else {
        return ItemDriverOutcome::InfiniteChestUnknown {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::InfiniteChest {
        item_id: item.id,
        character_id: character.id,
        template,
        key_name,
    }
}

pub(crate) fn keyring_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    match character.cursor_item {
        Some(key_item_id) => ItemDriverOutcome::KeyringAddCursorItem {
            item_id: item.id,
            character_id: character.id,
            key_item_id,
        },
        None => ItemDriverOutcome::KeyringShow {
            item_id: item.id,
            character_id: character.id,
        },
    }
}
