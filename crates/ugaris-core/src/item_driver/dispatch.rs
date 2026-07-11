use super::*;

pub fn legacy_item_driver_return_code(driver: Option<u16>, outcome: &ItemDriverOutcome) -> i32 {
    match outcome {
        ItemDriverOutcome::DoorToggle { .. }
        | ItemDriverOutcome::KeyedDoorToggle { .. }
        | ItemDriverOutcome::DoubleDoorToggle { .. }
        | ItemDriverOutcome::PickDoorToggle { .. }
        | ItemDriverOutcome::TrapdoorOpen { .. }
        | ItemDriverOutcome::TrapdoorBlocked { .. }
        | ItemDriverOutcome::TrapdoorClose { .. }
        | ItemDriverOutcome::TrapdoorBusy { .. }
        | ItemDriverOutcome::TrapdoorNeedsStick { .. }
        | ItemDriverOutcome::JunkpileSearch { .. }
        | ItemDriverOutcome::JunkpileCursorOccupied { .. }
        | ItemDriverOutcome::GasTrapPulse { .. }
        | ItemDriverOutcome::StafferSpecDoorToggle { .. }
        | ItemDriverOutcome::EdemonDoorToggle { .. } => 1,
        ItemDriverOutcome::StafferSpecDoorLocked { .. }
        | ItemDriverOutcome::EdemonDoorLocked { .. }
        | ItemDriverOutcome::EdemonDoorLifeless { .. }
        | ItemDriverOutcome::PickDoorLocked { .. }
        | ItemDriverOutcome::CaligarWeightDoorLocked { .. }
        | ItemDriverOutcome::CaligarSkellyDoorLocked { .. }
        | ItemDriverOutcome::CaligarSkellyDoorBusy { .. }
        | ItemDriverOutcome::PentBossDoorLocked { .. }
        | ItemDriverOutcome::PentBossDoorBusy { .. }
        | ItemDriverOutcome::WarpKeyDoorMissingKey { .. }
        | ItemDriverOutcome::WarpKeyDoorBug { .. }
        | ItemDriverOutcome::WarpTrialDoorWrongSide { .. }
        | ItemDriverOutcome::WarpTrialDoorBusy { .. }
        | ItemDriverOutcome::WarpTrialDoorBug { .. } => 2,
        ItemDriverOutcome::EdemonBlockMove { .. }
        | ItemDriverOutcome::EdemonBlockBlocked { .. }
        | ItemDriverOutcome::EdemonTubePulse { .. } => 1,
        ItemDriverOutcome::Noop
            if matches!(
                driver,
                Some(IDR_CLANVAULT)
                    | Some(IDR_PALACEBOMB)
                    | Some(IDR_PALACECAP)
                    | Some(IDR_LAB2_GRAVE)
                    | Some(IDR_STR_MINE)
                    | Some(IDR_STR_STORAGE)
                    | Some(IDR_STR_SPAWNER)
                    | Some(IDR_STR_DEPOT)
                    | Some(IDR_STR_TICKER)
                    | Some(IDR_NOSNOW)
                    | Some(IDR_WARPKEYDOOR)
            ) =>
        {
            1
        }
        ItemDriverOutcome::Noop
            if matches!(
                driver,
                Some(IDR_DOOR)
                    | Some(IDR_DOUBLE_DOOR)
                    | Some(IDR_STAFFER2)
                    | Some(IDR_CALIGAR)
                    | Some(IDR_WARPTRIALDOOR)
            ) =>
        {
            2
        }
        ItemDriverOutcome::IdentityTag { .. } => 1,
        ItemDriverOutcome::Noop | ItemDriverOutcome::Unsupported { .. } => 0,
        _ => 1,
    }
}

pub fn use_item(
    character: &mut Character,
    item: &Item,
    request: ItemUseRequest,
    account_depot_available: bool,
) -> Result<UseItemOutcome, UseItemError> {
    if character.id != request.character_id {
        return Err(UseItemError::IllegalCharacter);
    }
    if item.id != request.item_id {
        return Err(UseItemError::IllegalItem);
    }
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(UseItemError::Dead);
    }

    if item.driver == IDR_ACCOUNT_DEPOT {
        if !account_depot_available {
            return Err(UseItemError::AccountDepotUnavailable);
        }
        character.current_container = Some(item.id);
        return Ok(UseItemOutcome::OpenAccountDepot { item_id: item.id });
    }

    if item.content_id != 0 {
        if grave_access_denied(item, character.id) {
            return Err(UseItemError::AccessDenied);
        }
        character.current_container = Some(item.id);
        return Ok(UseItemOutcome::OpenContainer { item_id: item.id });
    }

    if item.flags.contains(ItemFlags::DEPOT) {
        character.current_container = Some(item.id);
        return Ok(UseItemOutcome::OpenDepot { item_id: item.id });
    }

    Ok(UseItemOutcome::Dispatch(ItemDriverRequest::Driver {
        driver: item.driver,
        item_id: item.id,
        character_id: character.id,
        spec: request.spec,
    }))
}

pub fn execute_item_driver(
    character: &mut Character,
    item: &mut Item,
    request: ItemDriverRequest,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    execute_item_driver_with_context(
        character,
        item,
        request,
        area_id,
        in_arena,
        &ItemDriverContext::default(),
    )
}

pub fn execute_item_driver_with_context(
    character: &mut Character,
    item: &mut Item,
    request: ItemDriverRequest,
    area_id: u16,
    in_arena: bool,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    match request {
        ItemDriverRequest::Driver {
            driver,
            item_id,
            character_id,
            spec,
        } => {
            if character.id != character_id || item.id != item_id {
                return ItemDriverOutcome::Noop;
            }
            if let Some(required_area) = legacy_libload_required_area(driver) {
                if area_id != required_area {
                    return ItemDriverOutcome::LibloadAreaBlocked {
                        driver,
                        item_id,
                        character_id,
                        required_area,
                    };
                }
            }
            if driver >= 1000 {
                return ItemDriverOutcome::IdentityTag {
                    driver,
                    item_id,
                    character_id,
                };
            }

            match driver {
                0 => ItemDriverOutcome::LookItem {
                    item_id,
                    character_id,
                },
                IDR_POTION => potion_driver(character, item, area_id, in_arena),
                IDR_DOOR => door_driver(character, item, context),
                IDR_BALLTRAP => balltrap_driver(character, item),
                IDR_BONEBRIDGE => bonebridge_driver(character, item, context),
                IDR_BONELADDER => boneladder_driver(character, item),
                IDR_BONEHOLDER => boneholder_driver(character, item, context),
                IDR_BONEWALL => bonewall_driver(character, item, context),
                IDR_BONEHINT => bonehint_driver(character, item, context),
                IDR_FIREBALL => fireball_machine_driver(character, item, context),
                IDR_EDEMONBALL => edemonball_driver(character, item, context),
                IDR_EDEMONSWITCH => edemon_switch_driver(character, item, context),
                IDR_EDEMONGATE => edemon_gate_driver(character, item, context),
                IDR_EDEMONLOADER => edemon_loader_driver(character, item, context),
                IDR_EDEMONLIGHT => edemon_light_driver(character, item, context),
                IDR_EDEMONDOOR => edemon_door_driver(character, item, context),
                IDR_EDEMONBLOCK => edemon_block_driver(character, item, context),
                IDR_EDEMONTUBE => edemon_tube_driver(character, item, context),
                IDR_FDEMONLIGHT => fdemon_light_driver(character, item, context),
                IDR_FDEMONLOADER => fdemon_loader_driver(character, item, context),
                IDR_FDEMONCANNON => fdemon_cannon_driver(character, item, context),
                IDR_FDEMONGATE => fdemon_gate_driver(character, item, context),
                IDR_FDEMONWAYPOINT => fdemon_waypoint_driver(character, item, context),
                IDR_FDEMONFARM => fdemon_farm_driver(character, item, context),
                IDR_FDEMONBLOOD => fdemon_blood_driver(character, item, context),
                IDR_FDEMONLAVA => fdemon_lava_driver(character, item, context),
                IDR_ITEMSPAWN => itemspawn_driver(character, item, area_id),
                IDR_WARMFIRE => warmfire_driver(character, item, area_id, context),
                IDR_BACKTOFIRE => backtofire_driver(character, item, area_id),
                IDR_MELTINGKEY => meltingkey_driver(character, item, area_id),
                IDR_PALACEBOMB => palace_bomb_driver(character, item),
                IDR_PALACECAP => palace_cap_driver(character, item, context),
                IDR_FLAMETHROW => flamethrow_driver(character, item, context),
                IDR_USETRAP => usetrap_driver(character, item),
                IDR_STEPTRAP => steptrap_driver(character, item, context),
                IDR_PALACEGATE => palace_gate_driver(character, item, context),
                IDR_SPIKETRAP => spiketrap_driver(character, item, context),
                IDR_EXTINGUISH => extinguish_driver(character, item),
                IDR_CHEST => chest_driver(character, item),
                IDR_RANDCHEST => randchest_driver(character, item),
                IDR_TRAPDOOR => trapdoor_driver(character, item, context),
                IDR_JUNKPILE => junkpile_driver(character, item, context),
                IDR_GASTRAP => gastrap_driver(character, item, context),
                IDR_SWAMPARM => swamparm_driver(character, item, context),
                IDR_SWAMPWHISP => swampwhisp_driver(character, item, context),
                IDR_SWAMPSPAWN => swampspawn_driver(character, item, context),
                IDR_PALACEDOOR => palace_door_driver(character, item, context),
                IDR_ISLENADOOR => islena_door_driver(character, item, context),
                IDR_FORESTSPADE => forest_spade_driver(character, item, area_id),
                IDR_FORESTCHEST => forest_chest_driver(character, item, context),
                IDR_PICKDOOR => pick_door_driver(character, item, context),
                IDR_PICKCHEST => pick_chest_driver(character, item, context),
                IDR_MINEDOOR => mine_door_driver(character, item, context, area_id),
                IDR_MINEKEYDOOR => mine_key_door_driver(character, item, context),
                IDR_PENT => pentagram_driver(character, item, context),
                IDR_PENTBOSSDOOR => pent_boss_door_driver(character, item, context),
                IDR_BURNDOWN => burndown_driver(character, item, context),
                IDR_COLORTILE => colortile_driver(character, item),
                IDR_SKELRAISE => skelraise_driver(character, item, context),
                IDR_RANDOMSHRINE => randomshrine_driver(character, item, context),
                IDR_SHRINE => zombie_shrine_driver(character, item, context),
                IDR_RATCHEST => ratchest_driver(character, item),
                IDR_CHESTSPAWN => chestspawn_driver(character, item),
                IDR_PARKSHRINE => parkshrine_driver(character, item),
                IDR_BOOK => book_driver(character, item),
                IDR_BOOKCASE => bookcase_driver(character, item, context),
                IDR_DEMONSHRINE => demonshrine_driver(character, item, area_id),
                IDR_PALACEKEY => palace_key_driver(character, item, context),
                IDR_MINEWALL => minewall_driver(character, item),
                IDR_INFINITE_CHEST => infinite_chest_driver(character, item, context),
                IDR_RECALL => recall_driver(character, item, area_id, in_arena),
                IDR_TRANSPORT => transport_driver(character, item, spec),
                IDR_STATSCROLL => stat_scroll_driver(character, item),
                IDR_CLANSPAWN => clanspawn_driver(character, item, context),
                IDR_CLANVAULT => ItemDriverOutcome::Noop,
                IDR_CLANJEWEL => clanjewel_driver(character, item, context),
                IDR_CLANSPAWNEXIT => clanspawn_exit_driver(character, item),
                IDR_ASSEMBLE => assemble_driver(character, item, context),
                IDR_CITY_RECALL => city_recall_driver(character, item, area_id, in_arena),
                IDR_DUNGEONTELE => dungeon_teleport_driver(character, item),
                IDR_DUNGEONFAKE => dungeon_fake_driver(character, item),
                IDR_DUNGEONDOOR => dungeon_door_driver(character, item, context),
                IDR_DUNGEONKEY => dungeon_key_driver(character, item),
                IDR_FLASK => flask_driver(character, item, context, area_id, in_arena),
                IDR_DOUBLE_DOOR => double_door_driver(character, item),
                IDR_TELE_DOOR => teleport_door_driver(character, item),
                IDR_TELEPORT => teleport_driver(character, item),
                IDR_ONOFFLIGHT => onofflight_driver(character, item, context),
                IDR_NIGHTLIGHT => nightlight_driver(character, item, context),
                IDR_TORCH => torch_driver(character, item, context),
                IDR_FOOD => food_driver(character, item),
                IDR_TOPLIST => toplist_driver(character, item),
                IDR_ENHANCE => nomad_stack_driver(character, item),
                IDR_ENCHANTITEM => enchant_driver(character, item),
                IDR_ANTIENCHANTITEM => anti_enchant_driver(character, item, false),
                IDR_SPECIALANTIENCHANTITEM => anti_enchant_driver(character, item, true),
                IDR_ORBSPAWN => orbspawn_driver(character, item, false),
                IDR_ANTIORBSPAWN => orbspawn_driver(character, item, true),
                IDR_SPECIAL_POTION => {
                    special_potion_driver(character, item, area_id, in_arena, context.current_tick)
                }
                IDR_SPECIAL_SHRINE => special_shrine_driver(character, item),
                IDR_STAFFER => staffer_driver(character, item, context),
                IDR_NOMADDICE => nomad_dice_driver(character, item),
                IDR_NOMADSTACK => nomad_stack_driver(character, item),
                IDR_DEMONCHIP => nomad_stack_driver(character, item),
                IDR_STAFFER2 => staffer2_driver(character, item),
                IDR_SHRIKEAMULET => shrike_amulet_driver(character, item, context),
                IDR_MINEGATEWAYKEY => mine_gateway_key_driver(character, item, context),
                IDR_MINEGATEWAY => mine_gateway_driver(character, item, context),
                IDR_WARPTELEPORT => warpteleport_driver(character, item, context),
                IDR_WARPTRIALDOOR => warptrialdoor_driver(character, item, context),
                IDR_WARPBONUS => warpbonus_driver(character, item, context, area_id),
                IDR_WARPKEYSPAWN => warpkeyspawn_driver(character, item),
                IDR_WARPKEYDOOR => warpkeydoor_driver(character, item, context),
                IDR_TOYLIGHT => toylight_driver(character, item, context),
                IDR_BRANNINGTONFOREST => brannington_forest_driver(character, item),
                IDR_DECAYITEM => decaying_item_driver(character, item, context),
                IDR_OXYPOTION => oxy_potion_driver(character, item, area_id),
                IDR_FLOWER => alchemy_flower_driver(character, item, area_id),
                IDR_PICKBERRY => pick_berry_driver(character, item, area_id),
                IDR_LIZARDFLOWER => lizard_flower_driver(character, item, context, area_id),
                IDR_LAB3_PLANT => lab3_plant_driver(character, item, context),
                IDR_LAB3_SPECIAL => lab3_special_driver(character, item, context),
                IDR_LAB2_WATER => lab2_water_driver(character, item),
                IDR_LAB2_STEPACTION => lab2_stepaction_driver(character, item),
                IDR_LAB2_REGENERATE => lab2_regenerate_driver(character, item, context),
                IDR_LAB2_GRAVE => lab2_grave_driver(character, item, context),
                IDR_LABTORCH => labtorch_driver(character, item),
                IDR_DEATHFIBRIN => deathfibrin_driver(character, item, context),
                IDR_LABEXIT => labexit_driver(character, item, context),
                IDR_LABENTRANCE => labentrance_driver(character, item, context),
                IDR_LAB4_ITEM => lab4_item_driver(character, item),
                IDR_LAB5_ITEM => lab5_item_driver(character, item, context),
                IDR_LQ_TICKER => lq_ticker_driver(character, item),
                IDR_LQ_ENTRANCE => lq_entrance_driver(character, item, context),
                IDR_STR_TICKER => str_ticker_driver(character, item),
                IDR_STR_MINE => str_mine_driver(character, item),
                IDR_STR_STORAGE => str_storage_driver(character, item, context),
                IDR_STR_DEPOT => str_depot_driver(character, item),
                IDR_STR_SPAWNER => str_spawner_driver(character, item),
                IDR_NOSNOW => ItemDriverOutcome::Noop,
                IDR_SALTMINE_ITEM => saltmine_item_driver(character, item),
                IDR_BEYONDPOTION => beyond_potion_driver(character, item, area_id, in_arena),
                IDR_XMASTREE => xmastree_driver(character, item),
                IDR_XMASMAKER => xmasmaker_driver(character, item),
                IDR_CALIGAR => caligar_driver(character, item, context),
                IDR_ARKHATA => arkhata_driver(character, item, context),
                IDR_TEUFELDOOR => teufel_door_driver(character, item),
                IDR_TEUFELARENA => teufel_arena_driver(character, item, context),
                IDR_TEUFELRATNEST => teufel_ratnest_driver(character, item, context),
                IDR_TEUFELARENAEXIT => teufel_arena_exit_driver(character, item),
                IDR_CALIGARFLAME => flamethrow_driver(character, item, context),
                IDR_FREAKDOOR => freakdoor_driver(character, item),
                IDR_KEY_RING => keyring_driver(character, item),
                _ => ItemDriverOutcome::Unsupported {
                    driver,
                    item_id,
                    character_id,
                },
            }
        }
        ItemDriverRequest::AccountDepot {
            item_id,
            character_id,
        } => ItemDriverOutcome::AccountDepotOpened {
            item_id,
            character_id,
        },
    }
}

pub(crate) fn legacy_libload_required_area(driver: u16) -> Option<u16> {
    match driver {
        IDR_BONEBRIDGE | IDR_BONELADDER | IDR_BONEHOLDER | IDR_BONEWALL | IDR_BONEHINT => Some(18),
        IDR_NOMADDICE => Some(19),
        IDR_CLANSPAWN | IDR_CLANVAULT | IDR_CLANSPAWNEXIT => Some(30),
        IDR_EDEMONGATE | IDR_EDEMONDOOR | IDR_EDEMONBLOCK | IDR_EDEMONTUBE => Some(6),
        IDR_ISLENADOOR => Some(11),
        IDR_PENT | IDR_PENTBOSSDOOR => Some(4),
        IDR_PICKDOOR | IDR_PICKCHEST | IDR_BURNDOWN | IDR_COLORTILE | IDR_SKELRAISE => Some(17),
        IDR_MINEWALL | IDR_MINEDOOR | IDR_MINEKEYDOOR | IDR_MINEGATEWAY => Some(12),
        IDR_RANDOMSHRINE | IDR_TRAPDOOR | IDR_JUNKPILE | IDR_GASTRAP => Some(14),
        IDR_SWAMPARM | IDR_SWAMPWHISP | IDR_SWAMPSPAWN => Some(15),
        IDR_FORESTCHEST => Some(16),
        IDR_LQ_TICKER | IDR_LQ_ENTRANCE => Some(20),
        IDR_WARPTELEPORT | IDR_WARPTRIALDOOR | IDR_WARPBONUS | IDR_WARPKEYSPAWN
        | IDR_WARPKEYDOOR => Some(25),
        IDR_BRANNINGTONFOREST => Some(28),
        IDR_STAFFER => Some(26),
        IDR_LAB2_WATER | IDR_LAB2_STEPACTION | IDR_LAB2_REGENERATE | IDR_LAB2_GRAVE
        | IDR_LABTORCH | IDR_DEATHFIBRIN | IDR_LAB4_ITEM | IDR_LAB5_ITEM => Some(22),
        IDR_STAFFER2 => Some(29),
        IDR_OXYPOTION | IDR_LIZARDFLOWER => Some(31),
        IDR_TEUFELDOOR | IDR_TEUFELARENA | IDR_TEUFELRATNEST | IDR_TEUFELARENAEXIT => Some(34),
        IDR_CALIGAR => Some(36),
        IDR_ARKHATA => Some(37),
        IDR_DUNGEONTELE | IDR_DUNGEONFAKE | IDR_DUNGEONDOOR | IDR_DUNGEONKEY => Some(13),
        IDR_PALACEBOMB | IDR_PALACECAP => Some(11),
        IDR_FDEMONLIGHT | IDR_FDEMONLOADER | IDR_FDEMONCANNON | IDR_FDEMONGATE
        | IDR_FDEMONWAYPOINT | IDR_FDEMONFARM | IDR_FDEMONBLOOD | IDR_FDEMONLAVA => Some(8),
        _ => None,
    }
}
