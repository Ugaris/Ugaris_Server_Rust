use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssembleTemplate {
    SunAmulet12,
    SunAmulet13,
    SunAmulet23,
    SunAmulet123,
    WarrBluekey12,
    WarrBluekey13,
    WarrBluekey23,
    WarrBluekey123,
    WarrGreenkey12,
    WarrGreenkey13,
    WarrGreenkey23,
    WarrGreenkey123,
    WarrRedkey12,
    WarrRedkey13,
    WarrRedkey23,
    WarrRedkey123,
}

impl AssembleTemplate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SunAmulet12 => "sun_amulet12",
            Self::SunAmulet13 => "sun_amulet13",
            Self::SunAmulet23 => "sun_amulet23",
            Self::SunAmulet123 => "sun_amulet123",
            Self::WarrBluekey12 => "warr_bluekey12",
            Self::WarrBluekey13 => "warr_bluekey13",
            Self::WarrBluekey23 => "warr_bluekey23",
            Self::WarrBluekey123 => "warr_bluekey123",
            Self::WarrGreenkey12 => "warr_greenkey12",
            Self::WarrGreenkey13 => "warr_greenkey13",
            Self::WarrGreenkey23 => "warr_greenkey23",
            Self::WarrGreenkey123 => "warr_greenkey123",
            Self::WarrRedkey12 => "warr_redkey12",
            Self::WarrRedkey13 => "warr_redkey13",
            Self::WarrRedkey23 => "warr_redkey23",
            Self::WarrRedkey123 => "warr_redkey123",
        }
    }
}

pub(crate) fn shrike_amulet_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::ShrikeAmuletNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let own_bits = drdata(item, 0);
    let cursor_bits = context.cursor_drdata0.unwrap_or(0);
    if context.cursor_driver != Some(IDR_SHRIKEAMULET) || (own_bits & cursor_bits) != 0 {
        return ItemDriverOutcome::ShrikeAmuletDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::ShrikeAmuletAssemble {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        combined_bits: own_bits | cursor_bits,
    }
}

pub(crate) fn assemble_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::AssembleNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    if !is_assemblable_primary(item.template_id) {
        return ItemDriverOutcome::AssembleUnknownItem {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let Some(template) = assemble_template(item.template_id, context.cursor_template_id) else {
        return ItemDriverOutcome::AssembleDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::AssembleItem {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        template,
    }
}

pub(crate) fn is_assemblable_primary(primary_id: u32) -> bool {
    matches!(
        primary_id,
        IID_AREA2_SUN1
            | IID_AREA2_SUN2
            | IID_AREA2_SUN3
            | IID_AREA2_SUN12
            | IID_AREA2_SUN13
            | IID_AREA2_SUN23
            | IID_STAFF_BLUEKEY1
            | IID_STAFF_BLUEKEY2
            | IID_STAFF_BLUEKEY3
            | IID_STAFF_BLUEKEY12
            | IID_STAFF_BLUEKEY13
            | IID_STAFF_BLUEKEY23
            | IID_STAFF_GREENKEY1
            | IID_STAFF_GREENKEY2
            | IID_STAFF_GREENKEY3
            | IID_STAFF_GREENKEY12
            | IID_STAFF_GREENKEY13
            | IID_STAFF_GREENKEY23
            | IID_STAFF_REDKEY1
            | IID_STAFF_REDKEY2
            | IID_STAFF_REDKEY3
            | IID_STAFF_REDKEY12
            | IID_STAFF_REDKEY13
            | IID_STAFF_REDKEY23
    )
}

pub fn assemble_template(primary_id: u32, cursor_id: Option<u32>) -> Option<AssembleTemplate> {
    let cursor_id = cursor_id?;
    match primary_id {
        IID_AREA2_SUN1 => match cursor_id {
            IID_AREA2_SUN2 => Some(AssembleTemplate::SunAmulet12),
            IID_AREA2_SUN3 => Some(AssembleTemplate::SunAmulet13),
            IID_AREA2_SUN23 => Some(AssembleTemplate::SunAmulet123),
            _ => None,
        },
        IID_AREA2_SUN2 => match cursor_id {
            IID_AREA2_SUN1 => Some(AssembleTemplate::SunAmulet12),
            IID_AREA2_SUN3 => Some(AssembleTemplate::SunAmulet23),
            IID_AREA2_SUN13 => Some(AssembleTemplate::SunAmulet123),
            _ => None,
        },
        IID_AREA2_SUN3 => match cursor_id {
            IID_AREA2_SUN1 => Some(AssembleTemplate::SunAmulet13),
            IID_AREA2_SUN2 => Some(AssembleTemplate::SunAmulet23),
            IID_AREA2_SUN12 => Some(AssembleTemplate::SunAmulet123),
            _ => None,
        },
        IID_AREA2_SUN12 => (cursor_id == IID_AREA2_SUN3).then_some(AssembleTemplate::SunAmulet123),
        IID_AREA2_SUN13 => (cursor_id == IID_AREA2_SUN2).then_some(AssembleTemplate::SunAmulet123),
        IID_AREA2_SUN23 => (cursor_id == IID_AREA2_SUN1).then_some(AssembleTemplate::SunAmulet123),

        IID_STAFF_BLUEKEY1 => match cursor_id {
            IID_STAFF_BLUEKEY2 => Some(AssembleTemplate::WarrBluekey12),
            IID_STAFF_BLUEKEY3 => Some(AssembleTemplate::WarrBluekey13),
            IID_STAFF_BLUEKEY23 => Some(AssembleTemplate::WarrBluekey123),
            _ => None,
        },
        IID_STAFF_BLUEKEY2 => match cursor_id {
            IID_STAFF_BLUEKEY1 => Some(AssembleTemplate::WarrBluekey12),
            IID_STAFF_BLUEKEY3 => Some(AssembleTemplate::WarrBluekey23),
            IID_STAFF_BLUEKEY13 => Some(AssembleTemplate::WarrBluekey123),
            _ => None,
        },
        IID_STAFF_BLUEKEY3 => match cursor_id {
            IID_STAFF_BLUEKEY1 => Some(AssembleTemplate::WarrBluekey13),
            IID_STAFF_BLUEKEY2 => Some(AssembleTemplate::WarrBluekey23),
            IID_STAFF_BLUEKEY12 => Some(AssembleTemplate::WarrBluekey123),
            _ => None,
        },
        IID_STAFF_BLUEKEY12 => {
            (cursor_id == IID_STAFF_BLUEKEY3).then_some(AssembleTemplate::WarrBluekey123)
        }
        IID_STAFF_BLUEKEY13 => {
            (cursor_id == IID_STAFF_BLUEKEY2).then_some(AssembleTemplate::WarrBluekey123)
        }
        IID_STAFF_BLUEKEY23 => {
            (cursor_id == IID_STAFF_BLUEKEY1).then_some(AssembleTemplate::WarrBluekey123)
        }

        IID_STAFF_GREENKEY1 => match cursor_id {
            IID_STAFF_GREENKEY2 => Some(AssembleTemplate::WarrGreenkey12),
            IID_STAFF_GREENKEY3 => Some(AssembleTemplate::WarrGreenkey13),
            IID_STAFF_GREENKEY23 => Some(AssembleTemplate::WarrGreenkey123),
            _ => None,
        },
        IID_STAFF_GREENKEY2 => match cursor_id {
            IID_STAFF_GREENKEY1 => Some(AssembleTemplate::WarrGreenkey12),
            IID_STAFF_GREENKEY3 => Some(AssembleTemplate::WarrGreenkey23),
            IID_STAFF_GREENKEY13 => Some(AssembleTemplate::WarrGreenkey123),
            _ => None,
        },
        IID_STAFF_GREENKEY3 => match cursor_id {
            IID_STAFF_GREENKEY1 => Some(AssembleTemplate::WarrGreenkey13),
            IID_STAFF_GREENKEY2 => Some(AssembleTemplate::WarrGreenkey23),
            IID_STAFF_GREENKEY12 => Some(AssembleTemplate::WarrGreenkey123),
            _ => None,
        },
        IID_STAFF_GREENKEY12 => {
            (cursor_id == IID_STAFF_GREENKEY3).then_some(AssembleTemplate::WarrGreenkey123)
        }
        IID_STAFF_GREENKEY13 => {
            (cursor_id == IID_STAFF_GREENKEY2).then_some(AssembleTemplate::WarrGreenkey123)
        }
        IID_STAFF_GREENKEY23 => {
            (cursor_id == IID_STAFF_GREENKEY1).then_some(AssembleTemplate::WarrGreenkey123)
        }

        IID_STAFF_REDKEY1 => match cursor_id {
            IID_STAFF_REDKEY2 => Some(AssembleTemplate::WarrRedkey12),
            IID_STAFF_REDKEY3 => Some(AssembleTemplate::WarrRedkey13),
            IID_STAFF_REDKEY23 => Some(AssembleTemplate::WarrRedkey123),
            _ => None,
        },
        IID_STAFF_REDKEY2 => match cursor_id {
            IID_STAFF_REDKEY1 => Some(AssembleTemplate::WarrRedkey12),
            IID_STAFF_REDKEY3 => Some(AssembleTemplate::WarrRedkey23),
            IID_STAFF_REDKEY13 => Some(AssembleTemplate::WarrRedkey123),
            _ => None,
        },
        IID_STAFF_REDKEY3 => match cursor_id {
            IID_STAFF_REDKEY1 => Some(AssembleTemplate::WarrRedkey13),
            IID_STAFF_REDKEY2 => Some(AssembleTemplate::WarrRedkey23),
            IID_STAFF_REDKEY12 => Some(AssembleTemplate::WarrRedkey123),
            _ => None,
        },
        IID_STAFF_REDKEY12 => {
            (cursor_id == IID_STAFF_REDKEY3).then_some(AssembleTemplate::WarrRedkey123)
        }
        IID_STAFF_REDKEY13 => {
            (cursor_id == IID_STAFF_REDKEY2).then_some(AssembleTemplate::WarrRedkey123)
        }
        IID_STAFF_REDKEY23 => {
            (cursor_id == IID_STAFF_REDKEY1).then_some(AssembleTemplate::WarrRedkey123)
        }
        _ => None,
    }
}
