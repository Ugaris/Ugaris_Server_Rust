use super::*;

pub(crate) fn staffer2_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    match drdata(item, 0) {
        1 => {
            if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
                return ItemDriverOutcome::Noop;
            }
            staffer_book_driver(character, item)
        }
        2 => staffer_mine_driver(character, item),
        3 => staffer_block_driver(character, item),
        4 | 5 => staffer_spec_door_driver(character, item),
        6 => {
            if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
                return ItemDriverOutcome::Noop;
            }
            let exp_added =
                (legacy_level_value(60) / 5).min(legacy_level_value(character.level) / 4);
            ItemDriverOutcome::StafferAnimationBook {
                item_id: item.id,
                character_id: character.id,
                exp_added,
            }
        }
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_STAFFER2,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

pub(crate) fn staffer_spec_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if item.x == 0 {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::StafferSpecDoorToggle {
        item_id: item.id,
        character_id: character.id,
        kind: drdata(item, 0),
    }
}

pub(crate) fn staffer_mine_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::StafferMineTimer { item_id: item.id };
    }
    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    if drdata(item, 3) < 9 {
        if character.endurance < POWERSCALE {
            return ItemDriverOutcome::StafferMineExhausted {
                item_id: item.id,
                character_id: character.id,
            };
        }
        let miner = character.professions.get(2).copied().unwrap_or_default();
        let cost = POWERSCALE / 4 - (i32::from(miner) * POWERSCALE / (4 * 25));
        character.endurance = character.endurance.saturating_sub(cost.max(0));
        set_drdata(item, 3, drdata(item, 3).saturating_add(1));
        set_drdata(item, 5, 0);
        item.sprite += 1;
    }

    ItemDriverOutcome::StafferMineDig {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn staffer_block_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::StafferBlockTimer { item_id: item.id };
    }
    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::StafferBlockMove {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn staffer_book_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    if drdata_u32(item, 4) != character.id.0 {
        set_drdata(item, 1, 0);
        set_drdata_u32(item, 4, character.id.0);
    }

    let page = drdata(item, 1);
    if page > 4 {
        return ItemDriverOutcome::Noop;
    }

    if page == 4 {
        set_drdata(item, 1, 0);
    } else {
        set_drdata(item, 1, page + 1);
    }

    ItemDriverOutcome::StafferBookText {
        item_id: item.id,
        character_id: character.id,
        page,
    }
}

pub fn staffer_book_text(page: u8) -> Option<&'static str> {
    match page {
        0 => Some("The training of these thieves into skilled mages has been succesful. They can now create Golems, and summon the old enemies of Aston, the Grolms. I will not teach them how to create and control Undead though, lest they use them against me... Also, to this end, I have enlisted the help of an assassin by the name of Brenneth. I hope he will not disappoint me..."),
        1 => Some("My golems have dug their way into the Brannington Crypt. I have taken their Holy Relic, and turned it into my weapon to make undead of the Brannington Ancestors. They shall serve as my army and take over Brannington town. All serve as zombies and skeletons, however, there is one spirit who managed to escape my grasp. I will have to find ways to control it... Also, Brenneth was attacked by a grolm and is suffering from loss of memory... He is in one of the thief mage houses right now... Fortunately, they don't know who he is..."),
        2 => Some("Brenneth got rescued by a group of traveling adventurers while the thief mage who had him captured was creating more golems... Luckily, Brenneth doesn't recall anything of what he is supposed to do, and it doesn't look like he'll get his memory back... ever..."),
        3 => Some("The spirit seems uncontrollable... I will have to become stronger to control it, which means I have to train... And that takes time, time which I'd rather not waste... I have also seen the face of a new enemy... This enemy has killed my thief mages, and surely must be coming for me next... He ruined my plans to open the crypt doors with the jewelry the thief mages had managed to steal... They should have been faster in returning it to me... fools..."),
        4 => Some("I can hear my enemy coming for me... I shall kill and make of my enemy a commander in my army of undead... Now, I will fight and show my power!"),
        _ => None,
    }
}

pub fn staffer_book_continue_text(page: u8) -> Option<&'static str> {
    match page {
        0..=3 => Some("USE again to continue."),
        4 => Some("USE to start over."),
        _ => None,
    }
}
