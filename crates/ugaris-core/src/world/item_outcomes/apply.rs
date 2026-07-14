use super::*;

impl World {
    pub(crate) fn apply_item_driver_outcome(
        &mut self,
        outcome: ItemDriverOutcome,
        current_area_id: u16,
    ) -> ItemDriverOutcome {
        match outcome {
            ItemDriverOutcome::LqTicker {
                item_id,
                schedule_after_ticks,
            } => {
                self.discover_lq_doors_once();
                self.queue_due_lq_npc_respawns();
                // C `lq_ticker`'s own self-reschedule (`lq.c:462`,
                // `call_item(it[in].driver, in, 0, ticker + TICKS)`). This
                // used to (incorrectly) live in `ugaris-server`'s player-
                // `item_use` completion dispatcher
                // (`tick_item_use_clan_lq_arena.rs`), a call path a
                // `character_id == 0` timer-fired outcome never actually
                // flows through - see that file's own doc comment for why
                // that arm is dead code. This is the real dispatch point
                // (`process_due_timers` -> `execute_item_driver_timer_
                // request` -> here) both `LqTicker`/`StrTicker` outcomes
                // actually reach, so the reschedule belongs here instead.
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::StrTicker {
                item_id,
                schedule_after_ticks,
            } => {
                self.str_ticker();
                // See the `LqTicker` arm above for why the reschedule
                // lives here now instead of `tick_item_use_clan_lq_arena.rs`.
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::StrSpawnerAmbientTick { item_id } => {
                self.str_spawner_ambient_tick(item_id);
                outcome
            }
            ItemDriverOutcome::StrStorageInteract {
                item_id,
                conversion:
                    StrStorageConversion::Converted {
                        cursor_item_id,
                        added,
                    },
                ..
            } => {
                if let Some(item) = self.items.get_mut(&item_id) {
                    let new_total = str_item_gold(item) + added;
                    set_str_item_gold(item, new_total);
                }
                self.destroy_item(cursor_item_id);
                outcome
            }
            ItemDriverOutcome::StrMineWorkerDig {
                item_id,
                character_id,
                mined,
            } => {
                if let Some(item) = self.items.get_mut(&item_id) {
                    let new_gold = str_item_gold(item).saturating_sub(mined);
                    set_str_item_gold(item, new_gold);
                }
                if let Some(CharacterDriverState::StrategyWorker(data)) = self
                    .characters
                    .get_mut(&character_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    data.platin += mined as i32;
                }
                outcome
            }
            ItemDriverOutcome::StrBuildingWorkerTransfer {
                item_id,
                character_id,
                deposited,
                withdrawn,
            } => {
                if deposited > 0 {
                    if let Some(item) = self.items.get_mut(&item_id) {
                        let new_gold = str_item_gold(item) + deposited;
                        set_str_item_gold(item, new_gold);
                    }
                    if let Some(CharacterDriverState::StrategyWorker(data)) = self
                        .characters
                        .get_mut(&character_id)
                        .and_then(|character| character.driver_state.as_mut())
                    {
                        data.platin = 0;
                    }
                } else if withdrawn > 0 {
                    if let Some(item) = self.items.get_mut(&item_id) {
                        let new_gold = str_item_gold(item).saturating_sub(withdrawn);
                        set_str_item_gold(item, new_gold);
                    }
                    if let Some(CharacterDriverState::StrategyWorker(data)) = self
                        .characters
                        .get_mut(&character_id)
                        .and_then(|character| character.driver_state.as_mut())
                    {
                        data.platin += withdrawn as i32;
                    }
                }
                outcome
            }
            ItemDriverOutcome::StrDepotWorkerTakeover {
                item_id,
                character_id,
                owner,
            } => {
                // C `*(unsigned int *)(it[in].drdata + 0) = ch[cn].group;
                // sprintf(it[in].name, "%.20s's Depot (%d)", dat->name,
                // it[in].drdata[8]); say(cn, "Taking over depot.");`
                // (`strategy.c:1225-1229`).
                let owner_name = match self
                    .characters
                    .get(&character_id)
                    .and_then(|character| character.driver_state.as_ref())
                {
                    Some(CharacterDriverState::StrategyWorker(data)) => data.owner_name.clone(),
                    _ => String::new(),
                };
                if let Some(item) = self.items.get_mut(&item_id) {
                    set_str_item_owner(item, owner);
                    let slot = item.driver_data.get(8).copied().unwrap_or(0);
                    let truncated: String = owner_name.chars().take(20).collect();
                    item.name = format!("{truncated}'s Depot ({slot})");
                }
                self.npc_say(character_id, "Taking over depot.");
                outcome
            }
            ItemDriverOutcome::Teleport {
                item_id,
                character_id,
                x,
                y,
                area_id,
                ..
            } => {
                if area_id != 0 && area_id != current_area_id {
                    return outcome;
                }
                let is_warp_teleport = self
                    .items
                    .get(&item_id)
                    .is_some_and(|item| item.driver == IDR_WARPTELEPORT);
                let teleported = if is_warp_teleport {
                    self.teleport_character_exact(character_id, usize::from(x), usize::from(y))
                } else {
                    self.teleport_character(character_id, x, y, true)
                };
                if teleported {
                    outcome
                } else if is_warp_teleport {
                    ItemDriverOutcome::WarpTeleportBusy {
                        item_id,
                        character_id,
                    }
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::WarpTeleportSpheres {
                character_id,
                cursor_item_id,
                x,
                y,
                ..
            } => {
                if self.teleport_character(character_id, x, y, true) {
                    self.destroy_item(cursor_item_id);
                    let inventory_spheres = self
                        .characters
                        .get(&character_id)
                        .map(|character| {
                            character
                                .inventory
                                .iter()
                                .flatten()
                                .copied()
                                .filter(|item_id| {
                                    self.items
                                        .get(item_id)
                                        .is_some_and(|item| item.template_id == IID_AREA25_TELEKEY)
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    for item_id in inventory_spheres {
                        self.destroy_item(item_id);
                    }
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::WarpTeleportMissingSphere { .. }
            | ItemDriverOutcome::WarpTeleportBug { .. } => outcome,
            ItemDriverOutcome::WarpKeyDoor {
                character_id,
                key_item_id,
                x,
                y,
                ..
            } => {
                if self.teleport_character(character_id, x, y, true) {
                    self.destroy_item(key_item_id);
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        character.dir = match Direction::try_from(character.dir).ok() {
                            Some(Direction::Right) => Direction::Left as u8,
                            Some(Direction::Left) => Direction::Right as u8,
                            Some(Direction::Up) => Direction::Down as u8,
                            Some(Direction::Down) => Direction::Up as u8,
                            _ => character.dir,
                        };
                    }
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::WarpKeyDoorMissingKey { .. }
            | ItemDriverOutcome::WarpKeyDoorBug { .. } => outcome,
            ItemDriverOutcome::WarpTrialDoor {
                character_id,
                player_x,
                player_y,
                ..
            } => {
                if self.teleport_character(character_id, player_x, player_y, true) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::WarpTrialDoorWrongSide { .. }
            | ItemDriverOutcome::WarpTrialDoorBusy { .. }
            | ItemDriverOutcome::WarpTrialDoorBug { .. } => outcome,
            ItemDriverOutcome::WarpKeySpawn { .. }
            | ItemDriverOutcome::WarpKeySpawnCursorOccupied { .. } => outcome,
            ItemDriverOutcome::TeleportDoor {
                character_id, x, y, ..
            } => {
                if self.teleport_character(character_id, x, y, false) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::MineGateway {
                character_id,
                x,
                y,
                area_id,
                ..
            } => {
                if area_id != current_area_id {
                    return outcome;
                }
                if self.teleport_character(character_id, x, y, true) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::MineDoorTeleport {
                character_id,
                target_x,
                target_y,
                fallback_x,
                fallback_y,
                ..
            } => {
                if self.teleport_character(character_id, target_x, target_y, false)
                    || self.teleport_character(character_id, fallback_x, fallback_y, false)
                {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::MineDoorTimer { item_id } => {
                self.apply_mine_door_timer(item_id);
                outcome
            }
            ItemDriverOutcome::MineKeyDoor {
                item_id,
                character_id,
                cursor_item_id,
                golem_nr,
            } => match self.first_free_mine_keyholder_room() {
                Some((target_x, target_y))
                    if self.teleport_character(character_id, target_x, target_y, false) =>
                {
                    if self.character_holds_cursor_item(character_id, cursor_item_id) {
                        self.destroy_item(cursor_item_id);
                    }
                    ItemDriverOutcome::MineKeyDoorOpened {
                        item_id,
                        character_id,
                        golem_nr,
                        room_x: target_x,
                        room_y: target_y,
                    }
                }
                _ => ItemDriverOutcome::MineKeyDoorBusy {
                    item_id,
                    character_id,
                },
            },
            ItemDriverOutcome::BackToFire {
                item_id,
                character_id,
                x,
                y,
            } => {
                if self.teleport_character(character_id, x, y, false) {
                    self.destroy_item(item_id);
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::WarmFire {
                character_id,
                removed_curse,
                ..
            } => {
                if removed_curse {
                    self.remove_driver_spells(character_id, IDR_CURSE);
                    self.remove_show_effect_type(character_id, EF_CURSE);
                }
                outcome
            }
            ItemDriverOutcome::Lab2RegenerateTick {
                item_id,
                target_id,
                start_tick,
                regen_percent,
                schedule_after_ticks,
            } => {
                let carried_by_target = self
                    .items
                    .get(&item_id)
                    .is_some_and(|item| item.carried_by == Some(target_id));
                if carried_by_target {
                    let current_tick = self.tick.0 as u32;
                    if let Some(target) = self.characters.get_mut(&target_id) {
                        if current_tick >= start_tick {
                            let max_hp = character_value(target, CharacterValue::Hp) * POWERSCALE;
                            let diff = max_hp - target.hp;
                            let add = if diff > 0 {
                                i32::from(regen_percent) * diff / 256
                            } else {
                                1
                            };
                            target.hp = max_hp.min(target.hp + add);
                            target.flags.insert(CharacterFlags::NODEATH);
                        } else {
                            target.flags.remove(CharacterFlags::NODEATH);
                        }
                    }
                }
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::Lab2GraveClose { item_id } => {
                self.close_lab2_grave(item_id);
                outcome
            }
            ItemDriverOutcome::Lab2GraveCheckOpen {
                item_id,
                undead_id,
                undead_serial,
                schedule_after_ticks,
            } => {
                let undead_still_open = self
                    .characters
                    .get(&undead_id)
                    .is_some_and(|character| character.serial == undead_serial);
                if undead_still_open {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                } else {
                    self.close_lab2_grave(item_id);
                }
                outcome
            }
            ItemDriverOutcome::Lab2StepActionDaemonCheck {
                item_id,
                character_id,
            } => {
                if let Some(item) = self.items.get(&item_id) {
                    self.notify_area(
                        item.x,
                        item.y,
                        NT_NPC,
                        NTID_LAB2_DEAMONCHECK,
                        character_id.0 as i32,
                        0,
                    );
                }
                outcome
            }
            ItemDriverOutcome::MeltingKeyTick {
                item_id,
                melted,
                schedule_after_ticks,
                ..
            } => {
                if melted {
                    self.destroy_item(item_id);
                } else if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::PentBossDoor {
                item_id,
                character_id,
                x,
                y,
            } => {
                if self.teleport_character(character_id, x, y, false) {
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        character.dir = match character.dir {
                            1 => 5,
                            5 => 1,
                            7 => 3,
                            3 => 7,
                            other => other,
                        };
                    }
                    outcome
                } else {
                    ItemDriverOutcome::PentBossDoorBusy {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::PentagramActivate {
                item_id,
                character_id,
                ..
            } => {
                self.apply_pentagram_activate(item_id, character_id);
                outcome
            }
            ItemDriverOutcome::PentagramTimer {
                item_id,
                status,
                area_status,
                ..
            } => {
                self.apply_pentagram_timer(item_id, i32::from(status), i32::from(area_status));
                outcome
            }
            ItemDriverOutcome::DungeonDoorSolved {
                character_id,
                clan_number,
                catacomb,
                first_solve,
                ..
            } => {
                if first_solve {
                    self.resolve_dungeon_door_first_solve(character_id, clan_number, catacomb);
                }
                if [(245, 250), (240, 250), (235, 250), (230, 250)]
                    .into_iter()
                    .any(|(x, y)| self.teleport_character(character_id, x, y, false))
                {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::ClanSpawnExit {
                item_id,
                character_id,
                area_id,
                x,
                y,
            } => {
                if area_id != current_area_id {
                    return outcome;
                }
                if self.teleport_character(character_id, x, y, false) {
                    outcome
                } else {
                    ItemDriverOutcome::ClanSpawnExitBusy {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::StafferAnimationBook { character_id, .. } => {
                if self.teleport_character(character_id, 25, 114, true) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::ForestSpadeCollapse {
                character_id, x, y, ..
            } => {
                if self.teleport_character(character_id, x, y, false) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::Recall {
                item_id,
                character_id,
                x,
                y,
                area_id,
            } => {
                if area_id != current_area_id {
                    return outcome;
                }
                if !self.teleport_character(character_id, x, y, false) {
                    return ItemDriverOutcome::Noop;
                }
                if let (Some(character), Some(item)) = (
                    self.characters.get_mut(&character_id),
                    self.items.get_mut(&item_id),
                ) {
                    consume_item(character, item);
                }
                outcome
            }
            ItemDriverOutcome::CityRecall {
                item_id,
                character_id,
                x,
                y,
                area_id,
            } => {
                self.consume_city_recall_scroll(character_id, item_id);
                if area_id != current_area_id {
                    return outcome;
                }
                if self.teleport_character(character_id, x, y, false) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TeufelArena {
                item_id,
                character_id,
                x,
                y,
            } => {
                if let Some(blocked) = self.teufel_arena_equipment_block(item_id, character_id) {
                    blocked
                } else if self.teleport_character(character_id, x, y, true) {
                    self.clear_character_spell_slots_and_effects(character_id);
                    self.pending_system_texts.push(WorldSystemText {
                        character_id,
                        message: "All your spells have been removed.".to_string(),
                    });
                    outcome
                } else {
                    ItemDriverOutcome::TeufelArenaBusy {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::TeufelArenaExit {
                character_id, x, y, ..
            } => {
                if self.teleport_character(character_id, x, y, true) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TeufelDoor {
                item_id,
                character_id,
                x,
                y,
            } => {
                if self.teleport_character(character_id, x, y, true) {
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        character.dir = match character.dir {
                            1 => 5,
                            5 => 1,
                            7 => 3,
                            3 => 7,
                            other => other,
                        };
                    }
                    outcome
                } else {
                    ItemDriverOutcome::TeufelDoorBusy {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::DoorToggle {
                item_id,
                character_id,
            } => {
                if self.toggle_door(item_id, character_id) == DoorToggleResult::Toggled {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::KeyedDoorToggle {
                item_id,
                character_id,
                ..
            } => {
                if self.toggle_door(item_id, character_id) == DoorToggleResult::Toggled {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::DoubleDoorToggle {
                item_id,
                character_id,
            } => {
                if self.toggle_double_door(item_id, character_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::PickDoorToggle {
                item_id,
                character_id,
                picked_lock,
            } => {
                if self.toggle_pick_door(item_id, character_id) == DoorToggleResult::Toggled {
                    if picked_lock {
                        self.notify_twocity_pick_from_character(character_id);
                    }
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StafferSpecDoorToggle {
                item_id,
                character_id,
                kind,
            } => match self.toggle_staffer_spec_door(item_id, character_id, kind) {
                StafferSpecDoorResult::Toggled => outcome,
                StafferSpecDoorResult::Locked => ItemDriverOutcome::StafferSpecDoorLocked {
                    item_id,
                    character_id,
                },
                StafferSpecDoorResult::Blocked | StafferSpecDoorResult::Failed => {
                    ItemDriverOutcome::Noop
                }
            },
            ItemDriverOutcome::EdemonDoorToggle {
                item_id,
                character_id,
                ..
            } => {
                if self.toggle_door(item_id, character_id) == DoorToggleResult::Toggled {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::EdemonBlockMove {
                item_id,
                target_x,
                target_y,
                schedule_after_ticks,
                ..
            } => {
                let moved = self.move_edemon_block(item_id, target_x, target_y);
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                if moved {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::EdemonBlockBlocked { .. } => outcome,
            ItemDriverOutcome::EdemonTubePulse {
                item_id,
                x,
                y,
                schedule_after_ticks,
                ..
            } => {
                self.pulse_edemon_tube(item_id, x, y);
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::EdemonGateSpawn {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::ChestSpawn { .. } => outcome,
            ItemDriverOutcome::SwampSpawn {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::SwampSpawnPulse {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                if let Some(item) = self.items.get(&item_id) {
                    self.mark_dirty_sector(usize::from(item.x), usize::from(item.y));
                }
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::ChestSpawnCheck {
                item_id,
                spawned_character_id,
                schedule_after_ticks,
                ..
            } => {
                if self.chestspawn_spawn_alive(spawned_character_id) {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                } else {
                    self.reset_chestspawn_item(item_id);
                }
                outcome
            }
            ItemDriverOutcome::FdemonGateSpawn {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::FdemonCannonPulse {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.pulse_fdemon_cannon(item_id);
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::FdemonCannonLifeless { .. } => outcome,
            ItemDriverOutcome::FreakDoorUse {
                item_id,
                character_id,
                link_group,
                one_way,
                recursion_guard,
                cached_partner_id,
                no_target,
            } => {
                if recursion_guard {
                    return ItemDriverOutcome::Noop;
                }
                if self.use_freak_door(
                    item_id,
                    character_id,
                    link_group,
                    one_way,
                    cached_partner_id,
                    no_target,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TriggerMapItem {
                x,
                y,
                target_character_id,
                delay_ticks,
                ..
            } => {
                if self.schedule_map_item_driver_timer(
                    usize::from(x),
                    usize::from(y),
                    target_character_id,
                    delay_ticks,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StepTrapDiscoverTarget { item_id } => {
                if self.discover_steptrap_target(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TrapdoorOpen {
                item_id,
                character_id,
                target_x,
                target_y,
                schedule_after_ticks,
            } => {
                if self.open_trapdoor(
                    item_id,
                    character_id,
                    target_x,
                    target_y,
                    schedule_after_ticks,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TrapdoorBlocked {
                item_id,
                cursor_item_id,
                ..
            } => {
                if self.block_trapdoor(item_id, cursor_item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TrapdoorClose { item_id } => {
                if self.close_trapdoor(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TrapdoorBusy { .. }
            | ItemDriverOutcome::TrapdoorNeedsStick { .. } => outcome,
            ItemDriverOutcome::GasTrapPulse {
                item_id,
                character_id,
                power,
                schedule_initial_trigger,
                schedule_animation,
            } => {
                if schedule_initial_trigger {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), 1);
                }
                if schedule_animation {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), 3);
                }
                if let Some(animation) = self
                    .items
                    .get(&item_id)
                    .and_then(|item| item.driver_data.get(1).copied())
                {
                    self.apply_gastrap_foreground(item_id, animation);
                }
                if character_id.0 != 0
                    && self
                        .characters
                        .get(&character_id)
                        .is_some_and(|character| character.flags.contains(CharacterFlags::PLAYER))
                {
                    self.apply_legacy_hurt(
                        character_id,
                        None,
                        i32::from(power) * POWERSCALE,
                        1,
                        50,
                        33,
                    );
                }
                outcome
            }
            ItemDriverOutcome::SwampArmPulse {
                item_id,
                damage_now,
                schedule_after_ticks,
                ..
            } => {
                if damage_now {
                    for target_id in self.swamp_arm_damage_targets(item_id) {
                        self.apply_legacy_hurt(target_id, None, 10 * POWERSCALE, 1, 50, 90);
                    }
                }
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::SwampWhispPulse {
                item_id,
                moved_from,
                moved_to,
                schedule_after_ticks,
                ..
            } => {
                if let (Some(from), Some(to)) = (moved_from, moved_to) {
                    self.move_item_map_slot(item_id, from, to);
                } else if let Some(item) = self.items.get(&item_id) {
                    self.mark_dirty_sector(usize::from(item.x), usize::from(item.y));
                }
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::BoneBridgePlace {
                item_id,
                character_id,
                cursor_item_id,
            } => {
                if self.place_bone_bridge(item_id, character_id, cursor_item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BoneBridgeTimerTick { item_id } => {
                if self.tick_bone_bridge(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BoneBridgeAddBone {
                item_id,
                character_id,
                cursor_item_id,
            } => {
                if self.character_holds_cursor_item(character_id, cursor_item_id) {
                    self.add_bone_to_bridge(item_id, character_id, cursor_item_id);
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BoneBridgeRemoveBone {
                item_id,
                character_id,
            } => {
                self.remove_bone_from_bridge(item_id, character_id);
                outcome
            }
            ItemDriverOutcome::BoneWallTick { item_id, .. } => {
                if self.tick_bone_wall(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BoneHolderInsertRune {
                item_id,
                character_id,
                cursor_item_id,
                schedule_after_ticks,
                ..
            } => {
                if self.character_holds_cursor_item(character_id, cursor_item_id) {
                    self.destroy_item(cursor_item_id);
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        character.cursor_item = None;
                        character.flags.insert(CharacterFlags::ITEMS);
                    }
                    self.update_bone_holder_sprite(item_id);
                    self.schedule_item_driver_timer(
                        item_id,
                        CharacterId(0),
                        u64::from(schedule_after_ticks),
                    );
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BoneHolderRemoveRune { item_id, .. } => {
                self.update_bone_holder_sprite(item_id);
                outcome
            }
            ItemDriverOutcome::BoneHolderExpired { item_id } => {
                self.update_bone_holder_sprite(item_id);
                outcome
            }
            ItemDriverOutcome::BoneHolderActivate {
                item_id,
                character_id,
                last_holder,
            } => {
                let (nr, cleared) = self.scan_and_clear_bone_holder_runes(item_id, character_id);
                ItemDriverOutcome::BoneHolderActivateResolved {
                    item_id,
                    character_id,
                    last_holder,
                    nr,
                    cleared,
                }
            }
            ItemDriverOutcome::BallTrapProjectile {
                item_id,
                start_x,
                start_y,
                target_x,
                target_y,
                power,
                schedule_after_ticks,
                ..
            } => {
                self.create_ball_trap_effect(start_x, start_y, target_x, target_y, power);
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::FireballMachineProjectile {
                item_id,
                start_x,
                start_y,
                target_x,
                target_y,
                power,
                schedule_after_ticks,
                ..
            } => {
                let effect_id = self
                    .create_fireball_machine_effect(start_x, start_y, target_x, target_y, power);
                if let Some(item) = self.items.get(&item_id) {
                    self.notify_area(item.x, item.y, NT_SPELL, 0, V_FIREBALL, effect_id as i32);
                }
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::EdemonBallProjectile {
                item_id,
                start_x: _,
                start_y: _,
                target_x: _,
                target_y: _,
                strength,
                base_sprite,
                ..
            } => {
                let applied = if let Some(targeted) =
                    self.find_edemonball_target_shot(item_id, strength, base_sprite)
                {
                    if let Some(item) = self.items.get_mut(&item_id) {
                        if item.driver_data.len() > 3 {
                            item.driver_data[3] = item.driver_data[3].saturating_sub(1);
                        }
                    }
                    targeted
                } else {
                    outcome
                };
                let ItemDriverOutcome::EdemonBallProjectile {
                    start_x,
                    start_y,
                    target_x,
                    target_y,
                    strength,
                    base_sprite,
                    schedule_after_ticks,
                    ..
                } = applied
                else {
                    return ItemDriverOutcome::Noop;
                };
                self.create_edemonball_effect(
                    start_x,
                    start_y,
                    target_x,
                    target_y,
                    strength,
                    base_sprite,
                );
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                applied
            }
            ItemDriverOutcome::EdemonBallInactive {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                if let Some(item) = self.items.get(&item_id) {
                    self.mark_dirty_sector(usize::from(item.x), usize::from(item.y));
                }
                outcome
            }
            other => self.apply_item_driver_outcome_tail(other, current_area_id),
        }
    }
}
