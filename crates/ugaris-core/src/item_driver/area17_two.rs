use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickChestTemplate {
    PalaceNote1,
    PalaceNote2,
    PalaceNote3,
    MerchantNote1,
}

impl PickChestTemplate {
    pub fn from_kind(kind: u8) -> Option<Self> {
        match kind {
            0 => Some(Self::PalaceNote1),
            1 => Some(Self::PalaceNote2),
            2 => Some(Self::PalaceNote3),
            3 => Some(Self::MerchantNote1),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PalaceNote1 => "palace_note1",
            Self::PalaceNote2 => "palace_note2",
            Self::PalaceNote3 => "palace_note3",
            Self::MerchantNote1 => "merchant_note1",
        }
    }
}

pub(crate) const TWOCITY_COLORS: [&str; 7] =
    ["null", "Red", "Green", "Blue", "Yellow", "Black", "White"];

pub fn bookcase_text_line_bytes(
    kind: u8,
    random_index: u8,
    color: u8,
    solved_library: bool,
) -> Vec<u8> {
    let standard = "After reading the title you put the book back.";
    let color = TWOCITY_COLORS
        .get(usize::from(color))
        .copied()
        .unwrap_or(TWOCITY_COLORS[0]);
    let (name, text) = match kind {
        0 => {
            let idx = usize::from(random_index % BOOKCASE_RANDOM_TITLES.len() as u8);
            let text = if idx == 3 {
                "One recipe most mages will find useful uses Adygalah, Bhalkissa and Firuba, plus one berry and one or two mushrooms."
            } else {
                standard
            };
            (BOOKCASE_RANDOM_TITLES[idx].to_string(), text)
        }
        1 => {
            let text = if solved_library {
                standard
            } else {
                "You read the book and absorb the knowledge contained therein."
            };
            ("The Knowledge of Ages by Ishtar".to_string(), text)
        }
        2 => (format!("How to Raise {color} Orchids by Klark"), standard),
        3 => (
            format!("A {color} Day in the Life of a Warrior by C. O. Nan"),
            standard,
        ),
        4 => (
            format!("Dancing in Ten Easy Lessons by James {color}"),
            standard,
        ),
        5 => (
            format!("Help! I Have Been Visited by Little {color} Man! by Meier"),
            standard,
        ),
        6 => (
            format!("The Day the World turned {color} by Casaldra"),
            standard,
        ),
        _ => (
            "Lady Manners' Guide to Decent Behaviour".to_string(),
            standard,
        ),
    };

    let mut out = Vec::new();
    out.extend_from_slice(COL_LIGHT_GREEN);
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(b".");
    out.extend_from_slice(COL_RESET);
    out.extend_from_slice(b" ");
    out.extend_from_slice(text.as_bytes());
    out
}

pub fn bookcase_locked_text_lines() -> [&'static str; 2] {
    [
        "The bookcase is locked and you do not have the right key.",
        "There is a note attached to the lock: A statue stole the key and vanished with it in the northern part of the library.",
    ]
}

pub fn bookcase_library_exp(level: u32) -> u32 {
    legacy_level_value(level).saturating_div(5).min(80_000)
}

pub(crate) fn bookcase_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    let kind = drdata(item, 0);
    if kind == 1 && !context.has_area17_library_key {
        return ItemDriverOutcome::BookcaseLocked {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::BookcaseText {
        item_id: item.id,
        character_id: character.id,
        kind,
    }
}

pub(crate) fn pick_chest_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::PickChestCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if !context.has_area17_lockpick {
        return ItemDriverOutcome::PickChestLocked {
            item_id: item.id,
            character_id: character.id,
        };
    }
    let Some(template) = PickChestTemplate::from_kind(drdata(item, 0)) else {
        return ItemDriverOutcome::PickChestBug {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::PickChest {
        item_id: item.id,
        character_id: character.id,
        template,
    }
}

pub(crate) fn pick_door_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call {
        if drdata(item, 0) == 0 {
            return ItemDriverOutcome::Noop;
        }
        return ItemDriverOutcome::PickDoorToggle {
            item_id: item.id,
            character_id: character.id,
            picked_lock: false,
        };
    }
    if drdata(item, 0) != 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.flags.contains(CharacterFlags::PLAYER) && !context.has_area17_cursor_lockpick {
        return ItemDriverOutcome::PickDoorLocked {
            item_id: item.id,
            character_id: character.id,
        };
    }
    ItemDriverOutcome::PickDoorToggle {
        item_id: item.id,
        character_id: character.id,
        picked_lock: character.flags.contains(CharacterFlags::PLAYER),
    }
}

pub(crate) fn burndown_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let burn_state = drdata(item, 0);
    if context.timer_call || character.id.0 == 0 {
        if burn_state == 0 {
            return ItemDriverOutcome::Noop;
        }
        return ItemDriverOutcome::BurndownTimerTick { item_id: item.id };
    }

    if burn_state > 15 {
        return ItemDriverOutcome::BurndownTooHot {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if burn_state != 0 {
        return ItemDriverOutcome::BurndownAlreadyBurned {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if context.cursor_driver != Some(IDR_TORCH) || context.cursor_drdata0.unwrap_or_default() == 0 {
        return ItemDriverOutcome::BurndownTouch {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::BurndownIgnite {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn colortile_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::ColorTile {
        item_id: item.id,
        character_id: character.id,
        row: drdata(item, 0),
        color: drdata(item, 1),
    }
}

pub(crate) fn skelraise_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        return ItemDriverOutcome::SkelRaiseTimer { item_id: item.id };
    }

    if drdata(item, 2) != 0 {
        return ItemDriverOutcome::SkelRaiseTouch {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::SkelRaiseDust {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if context.cursor_template_id != Some(IID_AREA17_BLOODBOWL) {
        return ItemDriverOutcome::SkelRaiseDust {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let template = match drdata(item, 0) {
        0 => "raised_skeleton_green",
        1 => "raised_skeleton_red",
        2 => "raised_skeleton_green_key",
        3 => "raised_skeleton_red_key",
        4 => "raised_skeleton_nolight",
        5 => "quest_skeleton",
        _ => {
            return ItemDriverOutcome::SkelRaiseDust {
                item_id: item.id,
                character_id: character.id,
            }
        }
    };

    ItemDriverOutcome::SkelRaiseRaise {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        template,
    }
}
