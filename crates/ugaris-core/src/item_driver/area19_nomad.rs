use super::*;

pub(crate) fn nomad_dice_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::NomadDice {
        item_id: item.id,
        character_id: character.id,
        luck: drdata(item, 0),
    }
}

pub fn legacy_lucky_die_from_rolls(sides: u8, luck: u8, rolls: impl IntoIterator<Item = u8>) -> u8 {
    let needed = usize::from(luck) + 1;
    rolls
        .into_iter()
        .take(needed)
        .map(|roll| roll.clamp(1, sides.max(1)))
        .max()
        .unwrap_or(1)
}

pub fn legacy_nomad_dice_total<const ROLLS_PER_DIE: usize>(
    luck: u8,
    rolls: [[u8; ROLLS_PER_DIE]; 3],
) -> u8 {
    rolls
        .into_iter()
        .map(|die_rolls| legacy_lucky_die_from_rolls(6, luck, die_rolls))
        .sum()
}

pub(crate) fn nomad_stack_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::NomadStack {
        item_id: item.id,
        character_id: character.id,
    }
}
