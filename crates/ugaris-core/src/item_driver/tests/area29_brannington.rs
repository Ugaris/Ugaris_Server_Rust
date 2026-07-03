use super::*;

#[test]
fn staffer2_animation_book_reports_legacy_exp_for_runtime_ppd_gate() {
    let mut reader = character(1);
    reader.flags.insert(CharacterFlags::PLAYER);
    reader.level = 60;
    let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
    set_drdata(&mut book, 0, 6);
    let request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_STAFFER2, 122);
    assert_eq!(legacy_level_value(60), 885_841);
    assert_eq!(
        execute_item_driver(&mut reader, &mut book, request, 29, false),
        ItemDriverOutcome::StafferAnimationBook {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            exp_added: 177_168,
        }
    );
    assert_eq!(reader.exp, 0);
}

#[test]
fn staffer2_animation_book_requires_area29_and_player_character() {
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
    set_drdata(&mut book, 0, 6);
    let request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut player, &mut book, request, 1, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 29,
        }
    );

    let mut npc = character(2);
    let npc_request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(2),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver(&mut npc, &mut book, npc_request, 29, false),
        ItemDriverOutcome::Noop
    );
    assert_eq!(npc.exp, 0);
}

#[test]
fn staffer2_book_cycles_pages_and_resets_per_reader() {
    let mut reader = character(1);
    reader.flags.insert(CharacterFlags::PLAYER);
    let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
    set_drdata(&mut book, 0, 1);
    let request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut reader, &mut book, request, 29, false),
        ItemDriverOutcome::StafferBookText {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            page: 0,
        }
    );
    assert_eq!(drdata(&book, 1), 1);
    assert_eq!(drdata_u32(&book, 4), 1);

    for expected_page in 1..=4 {
        assert_eq!(
            execute_item_driver(&mut reader, &mut book, request, 29, false),
            ItemDriverOutcome::StafferBookText {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                page: expected_page,
            }
        );
    }
    assert_eq!(drdata(&book, 1), 0);

    let mut other = character(2);
    other.flags.insert(CharacterFlags::PLAYER);
    let other_request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(2),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver(&mut other, &mut book, other_request, 29, false),
        ItemDriverOutcome::StafferBookText {
            item_id: ItemId(8),
            character_id: CharacterId(2),
            page: 0,
        }
    );
    assert_eq!(drdata_u32(&book, 4), 2);
}

#[test]
fn staffer_book_text_preserves_legacy_prompts() {
    assert_eq!(
        staffer_book_text(0).unwrap(),
        "The training of these thieves into skilled mages has been succesful. They can now create Golems, and summon the old enemies of Aston, the Grolms. I will not teach them how to create and control Undead though, lest they use them against me... Also, to this end, I have enlisted the help of an assassin by the name of Brenneth. I hope he will not disappoint me..."
    );
    assert_eq!(
        staffer_book_continue_text(3),
        Some("USE again to continue.")
    );
    assert_eq!(staffer_book_continue_text(4), Some("USE to start over."));
    assert_eq!(staffer_book_text(5), None);
}

#[test]
fn staffer2_mine_dig_ports_endurance_and_stage_progression() {
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.endurance = POWERSCALE * 2;
    player.professions[2] = 25;
    let mut mine = item(
        8,
        ItemFlags::USED | ItemFlags::USE | ItemFlags::SIGHTBLOCK,
        0,
        IDR_STAFFER2,
    );
    mine.sprite = 15070;
    set_drdata(&mut mine, 0, 2);
    let request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut player, &mut mine, request, 29, false),
        ItemDriverOutcome::StafferMineDig {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(player.endurance, POWERSCALE * 2);
    assert_eq!(drdata(&mine, 3), 1);
    assert_eq!(mine.sprite, 15071);
}

#[test]
fn staffer2_mine_dig_blocks_exhausted_players() {
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.endurance = POWERSCALE - 1;
    let mut mine = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
    set_drdata(&mut mine, 0, 2);
    let request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut player, &mut mine, request, 29, false),
        ItemDriverOutcome::StafferMineExhausted {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(drdata(&mine, 3), 0);
}

#[test]
fn staffer2_block_dispatches_player_and_timer_paths() {
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    let mut block = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
    set_drdata(&mut block, 0, 3);
    let request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut player, &mut block, request, 29, false),
        ItemDriverOutcome::StafferBlockMove {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    let mut timer = character(0);
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver(&mut timer, &mut block, timer_request, 29, false),
        ItemDriverOutcome::StafferBlockTimer { item_id: ItemId(8) }
    );
}

#[test]
fn staffer2_special_door_subtypes_dispatch_to_typed_outcome() {
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER2,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    for subtype in 4..=5 {
        let mut item = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
        set_drdata(&mut item, 0, subtype);
        item.x = 10;
        item.y = 10;
        assert_eq!(
            execute_item_driver(&mut character, &mut item, request, 29, false),
            ItemDriverOutcome::StafferSpecDoorToggle {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                kind: subtype,
            }
        );
    }
}
