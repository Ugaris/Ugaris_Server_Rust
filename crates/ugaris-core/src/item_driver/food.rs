use super::*;

pub(crate) fn food_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let kind = drdata(item, 0);
    if kind == 2 {
        return lollipop_driver(character, item);
    }
    if kind == 3 {
        return ItemDriverOutcome::ChristmasPopInspected {
            item_id: item.id,
            character_id: character.id,
        };
    }

    consume_item(character, item);
    ItemDriverOutcome::FoodEaten {
        item_id: item.id,
        character_id: character.id,
        kind,
    }
}

pub(crate) fn lollipop_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    let licks = drdata(item, 1);
    if licks == 8 {
        return ItemDriverOutcome::LollipopMemories {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let next_licks = licks.saturating_add(1);
    set_drdata(item, 1, next_licks);
    item.sprite += 1;

    // C `lollipop` (`base.c:3242-3261`) grants exp via `give_exp(cn, ...)`,
    // not a raw `ch[cn].exp +=`, so the hardcore/global exp multipliers and
    // `check_levelup` need to run too - that only happens with `&mut World`
    // access, so this outcome carries the base amount and
    // `World::apply_item_driver_outcome`'s `LollipopLicked` arm calls
    // `World::give_exp` with it instead of mutating `character.exp` here.
    let exp_added = lollipop_exp(character.level);

    if next_licks == 1 {
        item.description = "A sweet lollipop. Well, it's already used.".to_string();
    } else if next_licks == 8 {
        item.description = "A lollipop stick.".to_string();
    }

    ItemDriverOutcome::LollipopLicked {
        item_id: item.id,
        character_id: character.id,
        exp_added,
        lick_count: next_licks,
    }
}

pub(crate) fn lollipop_exp(level: u32) -> u32 {
    legacy_level_value(level).saturating_div(750).max(5)
}
