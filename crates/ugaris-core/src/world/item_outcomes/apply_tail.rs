use super::*;

impl World {
    /// Mechanical continuation of [`World::apply_item_driver_outcome`]'s
    /// single legacy outcome match, split at an arm boundary to satisfy the
    /// per-file size cap; the arms below are verbatim and behavior is
    /// identical.
    pub(super) fn apply_item_driver_outcome_tail(
        &mut self,
        outcome: ItemDriverOutcome,
        current_area_id: u16,
    ) -> ItemDriverOutcome {
        match outcome {
            ItemDriverOutcome::CaligarGunProjectile {
                item_id,
                direction,
                schedule_after_ticks,
                ..
            } => {
                if self.create_caligar_gun_effects(item_id, direction) {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::SpikeTrapTriggered {
                item_id,
                character_id,
                damage,
                reset_after_ticks,
            } => {
                self.apply_legacy_hurt(character_id, None, damage, 1, 75, 75);
                self.schedule_item_driver_timer(item_id, CharacterId(0), reset_after_ticks);
                outcome
            }
            ItemDriverOutcome::SpikeTrapReset { .. } => outcome,
            ItemDriverOutcome::PalaceBombExplode {
                item_id,
                owner_id,
                x,
                y,
                ..
            } => {
                self.apply_palace_bomb_explosion(item_id, owner_id, x, y);
                outcome
            }
            ItemDriverOutcome::PalaceCapTimer {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.apply_palace_cap_timer(item_id, schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::Extinguish {
                item_id,
                character_id,
                ..
            } => {
                let extinguished = self.remove_character_burn_effect(character_id);
                ItemDriverOutcome::Extinguish {
                    item_id,
                    character_id,
                    extinguished,
                }
            }
            ItemDriverOutcome::FlameThrowerPulse {
                item_id,
                direction,
                schedule_after_ticks,
                ..
            } => {
                self.mark_flamethrower_targets_for_burn(item_id, direction);
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::FlameThrowerExtinguished {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::LightChanged {
                item_id,
                character_id,
                schedule_after_ticks,
            } => {
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, character_id, after_ticks);
                }
                let labtorch_extinguished = self.items.get(&item_id).and_then(|item| {
                    (item.driver == IDR_LABTORCH
                        && character_id.0 != 0
                        && item.driver_data.first().copied() == Some(0))
                    .then_some((item.x, item.y))
                });
                if let Some((x, y)) = labtorch_extinguished {
                    self.notify_area(
                        x,
                        y,
                        NT_NPC,
                        NTID_LABGNOMETORCH,
                        item_id.0 as i32,
                        character_id.0 as i32,
                    );
                }
                outcome
            }
            ItemDriverOutcome::BurndownIgnite {
                item_id,
                character_id,
            } => {
                if self.ignite_burndown_barrel(item_id) {
                    self.notify_twocity_pick_from_character(character_id);
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BurndownTimerTick { item_id } => {
                if self.tick_burndown_barrel(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BurndownTouch { .. }
            | ItemDriverOutcome::BurndownTooHot { .. }
            | ItemDriverOutcome::BurndownAlreadyBurned { .. } => outcome,
            ItemDriverOutcome::FdemonLoaderChanged {
                item_id,
                character_id,
                consumed_cursor_item_id,
                station_id: _,
                ground_overlay_sprite,
                sound_type,
                schedule_after_ticks,
            } => {
                if let Some(cursor_item_id) = consumed_cursor_item_id {
                    self.destroy_item(cursor_item_id);
                }
                let item_pos = self
                    .items
                    .get(&item_id)
                    .map(|item| (usize::from(item.x), usize::from(item.y)));
                if let Some((x, y)) = item_pos {
                    if let Some(tile) = self.map.tile_mut(x, y) {
                        let new_ground_sprite =
                            (tile.ground_sprite & 0xffff) | (ground_overlay_sprite << 16);
                        if tile.ground_sprite != new_ground_sprite {
                            tile.ground_sprite = new_ground_sprite;
                            self.mark_dirty_sector(x, y);
                        }
                    }
                    if let Some(sound_type) = sound_type {
                        self.queue_sound_area(x, y, sound_type);
                    }
                }
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, character_id, after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::FdemonLoaderBlocked { .. } => outcome,
            ItemDriverOutcome::EdemonLoaderChanged {
                item_id,
                character_id,
                consumed_cursor_item_id,
                ground_overlay_sprite,
                sound_type,
                schedule_after_ticks,
            } => {
                if let Some(cursor_item_id) = consumed_cursor_item_id {
                    self.destroy_item(cursor_item_id);
                }
                let item_pos = self
                    .items
                    .get(&item_id)
                    .map(|item| (usize::from(item.x), usize::from(item.y)));
                if let Some((x, y)) = item_pos {
                    if let Some(tile) = self.map.tile_mut(x, y) {
                        let new_ground_sprite =
                            (tile.ground_sprite & 0xffff) | (ground_overlay_sprite << 16);
                        if tile.ground_sprite != new_ground_sprite {
                            tile.ground_sprite = new_ground_sprite;
                            self.mark_dirty_sector(x, y);
                        }
                    }
                    if let Some(sound_type) = sound_type {
                        self.queue_sound_area(x, y, sound_type);
                    }
                }
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, character_id, after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::EdemonLoaderBlocked { .. } => outcome,
            ItemDriverOutcome::FdemonFarmChanged {
                item_id,
                foreground_sprite,
                schedule_after_ticks,
                ..
            } => {
                self.apply_fdemon_farm_foreground(item_id, foreground_sprite);
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::FdemonFarmHarvest {
                item_id,
                foreground_sprite,
                ..
            } => {
                self.apply_fdemon_farm_foreground(item_id, foreground_sprite);
                outcome
            }
            ItemDriverOutcome::FdemonFarmCursorOccupied { .. }
            | ItemDriverOutcome::FdemonFarmNotReady { .. }
            | ItemDriverOutcome::FdemonFarmBug { .. } => outcome,
            ItemDriverOutcome::FdemonBloodDestroyedFlask { flask_item_id, .. } => {
                self.destroy_item(flask_item_id);
                outcome
            }
            ItemDriverOutcome::FdemonBloodFilled {
                item_id,
                container_item_id,
                amount,
                ..
            } => {
                if let Some(container) = self.items.get_mut(&container_item_id) {
                    container.driver_data.resize(1, 0);
                    container.driver_data[0] = amount;
                    container.sprite += 1;
                    container.description =
                        format!("A container holding {} parts golem blood.", amount);
                }
                self.destroy_item(item_id);
                outcome
            }
            ItemDriverOutcome::FdemonBloodBlocked { .. } => outcome,
            ItemDriverOutcome::FdemonLavaActivated {
                item_id,
                container_item_id,
                amount,
                schedule_after_ticks,
                ..
            } => {
                if let Some(container) = self.items.get_mut(&container_item_id) {
                    container.driver_data.resize(1, 0);
                    container.driver_data[0] = amount;
                    container.sprite -= 1;
                    container.description =
                        format!("A container holding {} parts golem blood.", amount);
                }
                self.apply_fdemon_lava_tile(item_id, 120);
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::FdemonLavaPulse {
                item_id,
                stage,
                damage,
                armor_percent,
                schedule_after_ticks,
                ..
            } => {
                if let Some(target_id) = self.apply_fdemon_lava_tile(item_id, stage) {
                    if damage > 0 {
                        self.apply_legacy_hurt(target_id, None, damage, 1, 0, armor_percent);
                    }
                }
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::FdemonLavaBlocked { .. } => outcome,
            ItemDriverOutcome::FdemonWaypoint {
                item_id,
                spotted_enemy,
                target_character_id,
                target_serial,
                schedule_after_ticks,
                ..
            } => {
                self.apply_fdemon_waypoint(
                    item_id,
                    spotted_enemy,
                    target_character_id,
                    target_serial,
                );
                if let Some(item) = self.items.get(&item_id) {
                    self.notify_area(
                        item.x,
                        item.y,
                        NT_NPC,
                        NTID_FDEMON,
                        FDEMON_MSG_WAYPOINT,
                        item_id.0 as i32,
                    );
                }
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::EdemonSwitchStuck { .. } => outcome,
            ItemDriverOutcome::OnOffLightChanged {
                item_id,
                character_id,
                now_on,
                ..
            } => {
                let mut remaining_off = None;
                let mut gates_opened = false;
                if now_on {
                    self.area3_palace_lamps.switched_on_count += 1;
                    let remaining = self.area3_palace_lamps.switched_off_count
                        - self.area3_palace_lamps.switched_on_count;
                    remaining_off = Some(remaining);
                    if remaining == 0 {
                        gates_opened = true;
                        self.area3_palace_lamps.keep_open_until_tick =
                            self.tick.0 + (TICKS_PER_SECOND * 60 * 3);
                        self.schedule_registered_area3_lamp_extinguish();
                    }
                } else {
                    self.area3_palace_lamps.switched_off_count += 1;
                }
                ItemDriverOutcome::OnOffLightChanged {
                    item_id,
                    character_id,
                    now_on,
                    remaining_off,
                    gates_opened,
                }
            }
            ItemDriverOutcome::PalaceGateTick { item_id, .. } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
                let Some((opened, closed, blocked)) = self.tick_area3_palace_gate(item_id) else {
                    return ItemDriverOutcome::Noop;
                };
                ItemDriverOutcome::PalaceGateTick {
                    item_id,
                    opened,
                    closed,
                    blocked,
                }
            }
            ItemDriverOutcome::PalaceDoorTick {
                item_id,
                schedule_after_ticks,
                set_tmoveblock,
                ..
            } => {
                if let Some(blocked) = set_tmoveblock {
                    if let Some(item) = self.items.get(&item_id) {
                        if let Some(tile) =
                            self.map.tile_mut(usize::from(item.x), usize::from(item.y))
                        {
                            if blocked {
                                tile.flags.insert(MapFlags::TMOVEBLOCK);
                            } else {
                                tile.flags.remove(MapFlags::TMOVEBLOCK);
                            }
                        }
                    }
                }
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::PalaceDoorKeyRequired { .. } => outcome,
            ItemDriverOutcome::TunnelDoorAreaCheck {
                item_id,
                x,
                y,
                opened,
                schedule_after_ticks,
            } => {
                // C `mean_door`'s unconditional `call_item(...)`
                // reschedule (`tunnel.c:739`) runs before the
                // `check_area_clear` check, regardless of its result.
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                if opened {
                    // C `open_door` (`tunnel.c:764-772`).
                    if let Some(tile) = self.map.tile_mut(usize::from(x), usize::from(y)) {
                        tile.foreground_sprite = 0;
                        tile.flags
                            .remove(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
                    }
                    self.mark_dirty_sector(usize::from(x), usize::from(y));
                    self.pending_area_texts.push(WorldAreaText {
                        x,
                        y,
                        max_distance: 10,
                        message: "The door opens mysteriously.".to_string(),
                    });
                }
                outcome
            }
            ItemDriverOutcome::TunnelDoorFlavor { .. } => outcome,
            ItemDriverOutcome::TorchExtinguishedUnderwater {
                item_id,
                character_id,
                schedule_after_ticks,
            } => {
                self.schedule_item_driver_timer(item_id, character_id, schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::TorchExpired { item_id, .. } => {
                if self.destroy_item(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::ClanJewelRescheduled {
                item_id,
                schedule_after_ticks,
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::ClanJewelExpired { item_id, .. } => {
                if self.destroy_item(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::ClanSpawnTimer {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::ArkhataStopwatch {
                item_id,
                character_id: _,
                schedule_after_ticks,
            } => {
                self.schedule_item_driver_timer(
                    item_id,
                    CharacterId(0),
                    u64::from(schedule_after_ticks),
                );
                outcome
            }
            ItemDriverOutcome::DecayItemToggled {
                item_id,
                character_id,
                schedule_after_ticks,
                ..
            } => {
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, character_id, after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::DecayItemExpired { item_id, .. } => {
                if self.destroy_item(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::LabExitAnimating {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::LabExitExpired { item_id } => {
                if self.destroy_item(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::LabExitUse { .. }
            | ItemDriverOutcome::LabExitWrongOwner { .. }
            | ItemDriverOutcome::LabEntranceSolvedAll { .. }
            | ItemDriverOutcome::LabEntranceTooLow { .. }
            | ItemDriverOutcome::DeathfibrinShrineGive { .. }
            | ItemDriverOutcome::DeathfibrinShrineOccupied { .. }
            | ItemDriverOutcome::DeathfibrinNeedsCarry { .. }
            | ItemDriverOutcome::DeathfibrinNoMaster { .. } => outcome,
            ItemDriverOutcome::DeathfibrinStrike {
                character_id,
                master_id,
                ..
            } => {
                // C `lab1.c:527-540`: pulse-back show effect at the
                // master, then the actual strike - both unconditional
                // regardless of whether this hit vanishes the staff.
                let start_tick = self.tick.0 as u32;
                self.create_show_effect(
                    EF_PULSEBACK,
                    master_id,
                    start_tick,
                    start_tick + 7,
                    20,
                    42,
                );
                if let Some(master) = self.characters.get_mut(&master_id) {
                    master.flags.remove(CharacterFlags::IMMORTAL);
                }
                self.apply_legacy_hurt(master_id, Some(character_id), 10 * POWERSCALE, 1, 0, 0);
                if let Some(master) = self.characters.get_mut(&master_id) {
                    master.flags.insert(CharacterFlags::IMMORTAL);
                }
                self.npc_say(master_id, "Oh no! Deathfibrin hurts.");
                outcome
            }
            ItemDriverOutcome::StafferMineDig {
                item_id,
                character_id: _,
            } => {
                if self.apply_staffer_mine_dig(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StafferMineTimer { item_id } => {
                if self.apply_staffer_mine_timer(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::MineWallDig { item_id, .. } => {
                if self.apply_staffer_mine_dig(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::MineWallCollapse { item_id, .. } => {
                if self.apply_staffer_mine_timer(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StafferBlockMove {
                item_id,
                character_id,
            } => {
                if self.apply_staffer_block_move(item_id, character_id) {
                    outcome
                } else {
                    ItemDriverOutcome::StafferBlockBlocked {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::StafferBlockTimer { item_id } => {
                if self.apply_staffer_block_timer(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::CaligarWeightMove {
                item_id,
                character_id,
            } => {
                if self.apply_caligar_weight_move(item_id, character_id) {
                    outcome
                } else {
                    ItemDriverOutcome::CaligarWeightBlocked {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::CaligarWeightTimer { item_id } => {
                if self.apply_caligar_weight_timer(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::CaligarWeightDoor {
                item_id,
                character_id,
            } => match self.apply_caligar_weight_door(item_id, character_id) {
                CaligarWeightDoorResult::Moved => outcome,
                CaligarWeightDoorResult::Locked => ItemDriverOutcome::CaligarWeightDoorLocked {
                    item_id,
                    character_id,
                },
                CaligarWeightDoorResult::Busy => ItemDriverOutcome::CaligarWeightDoorBusy {
                    item_id,
                    character_id,
                },
                CaligarWeightDoorResult::Noop => ItemDriverOutcome::Noop,
            },
            ItemDriverOutcome::CaligarSkellyDoor { .. } => outcome,
            ItemDriverOutcome::SkelRaiseTimer { item_id } => {
                if self.apply_skelraise_timer(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StafferMineExhausted { .. }
            | ItemDriverOutcome::StafferBlockBlocked { .. }
            | ItemDriverOutcome::CaligarWeightBlocked { .. }
            | ItemDriverOutcome::CaligarWeightDoorLocked { .. }
            | ItemDriverOutcome::CaligarWeightDoorBusy { .. }
            | ItemDriverOutcome::CaligarSkellyDoorLocked { .. }
            | ItemDriverOutcome::CaligarSkellyDoorBusy { .. } => outcome,
            ItemDriverOutcome::BeyondPotion {
                item_id,
                character_id,
                duration_minutes,
                modifier_index,
                modifier_value,
                beyond_max_mod,
            } => {
                if self.install_beyond_potion_spell(
                    character_id,
                    item_id,
                    duration_minutes,
                    modifier_index,
                    modifier_value,
                    beyond_max_mod,
                    true,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::BlockedByRequirements {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::AlchemyFlaskPotion {
                item_id,
                character_id,
                duration_minutes,
                modifier_index,
                modifier_value,
            } => {
                if self.install_beyond_potion_spell(
                    character_id,
                    item_id,
                    duration_minutes,
                    modifier_index,
                    modifier_value,
                    false,
                    false,
                ) {
                    if let Some(item) = self.items.get_mut(&item_id) {
                        reset_flask_empty_state(item);
                    }
                    outcome
                } else {
                    ItemDriverOutcome::BlockedByRequirements {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::SpecialPotionAntidote {
                item_id,
                character_id,
                kind,
                ..
            } => {
                let poison_removed = if kind <= 3 {
                    self.remove_poison(character_id, u16::from(kind))
                } else {
                    self.remove_all_poison(character_id)
                };
                self.destroy_item(item_id);
                ItemDriverOutcome::SpecialPotionAntidote {
                    item_id,
                    character_id,
                    kind,
                    poison_removed,
                }
            }
            ItemDriverOutcome::SpecialPotionInfravision {
                item_id,
                character_id,
                ..
            } => {
                let installed = self.install_infravision_spell(character_id);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::SpecialPotionInfravision {
                    item_id,
                    character_id,
                    installed,
                }
            }
            ItemDriverOutcome::OxygenPotion {
                item_id,
                character_id,
                ..
            } => {
                let installed = self.install_oxygen_spell(character_id);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::OxygenPotion {
                    item_id,
                    character_id,
                    installed,
                }
            }
            ItemDriverOutcome::BranningtonUnderwaterBerry {
                item_id,
                character_id,
                duration_ticks,
                ..
            } => {
                let installed = self.install_oxygen_spell_for_ticks(character_id, duration_ticks);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::BranningtonUnderwaterBerry {
                    item_id,
                    character_id,
                    duration_ticks,
                    installed,
                }
            }
            ItemDriverOutcome::Lab3YellowBerry {
                item_id,
                character_id,
                duration_ticks,
                ..
            } => {
                self.remove_driver_spells(character_id, IDR_OXYGEN);
                let installed = self.install_oxygen_spell_for_ticks(character_id, duration_ticks);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::Lab3YellowBerry {
                    item_id,
                    character_id,
                    duration_ticks,
                    installed,
                }
            }
            ItemDriverOutcome::Lab3WhiteBerry {
                item_id,
                character_id,
                light_power,
                ..
            } => {
                let (installed, started_emit) =
                    self.apply_lab3_whiteberry(character_id, light_power);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::Lab3WhiteBerry {
                    item_id,
                    character_id,
                    light_power,
                    started_emit,
                    installed,
                }
            }
            ItemDriverOutcome::Lab3WhiteBerryLightTick { item_id, .. } => {
                let destroyed = self.decay_lab3_whiteberry_light(item_id);
                ItemDriverOutcome::Lab3WhiteBerryLightTick { item_id, destroyed }
            }
            ItemDriverOutcome::Lab3BrownBerry {
                item_id,
                character_id,
                duration_ticks,
                ..
            } => {
                let installed = self.install_underwater_talk_spell(character_id, duration_ticks);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::Lab3BrownBerry {
                    item_id,
                    character_id,
                    duration_ticks,
                    installed,
                }
            }
            ItemDriverOutcome::Lab3TeleportDoor {
                item_id,
                character_id,
                dx,
                dy,
                password_protected,
                ..
            } => self.apply_lab3_teleport_door(item_id, character_id, dx, dy, password_protected),
            ItemDriverOutcome::SpecialPotionSecurity { .. }
            | ItemDriverOutcome::SpecialPotionProfessionReset { .. }
            | ItemDriverOutcome::SpecialPotionBug { .. } => outcome,
            ItemDriverOutcome::SpecialShrine { .. } => outcome,
            ItemDriverOutcome::TorchExtractOrb { .. } => outcome,
            ItemDriverOutcome::NomadStack { .. }
            | ItemDriverOutcome::TransportOpen { .. }
            | ItemDriverOutcome::TransportTravel { .. }
            | ItemDriverOutcome::TransportInvalid { .. }
            | ItemDriverOutcome::ArenaToplist { .. } => outcome,
            ItemDriverOutcome::EnchantCursorItem {
                item_id,
                character_id,
                cursor_item_id,
                modifier,
                amount,
            } => {
                if self.apply_enchant_cursor_item(
                    item_id,
                    character_id,
                    cursor_item_id,
                    modifier,
                    amount,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::BlockedByRequirements {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::AntiEnchantCursorItem {
                item_id,
                character_id,
                cursor_item_id,
                modifier,
                amount,
                extract_orb: _,
            } => {
                if self.apply_anti_enchant_cursor_item(
                    item_id,
                    character_id,
                    cursor_item_id,
                    modifier,
                    amount,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::BlockedByRequirements {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::ShrikeAmuletAssemble {
                item_id,
                character_id,
                cursor_item_id,
                combined_bits,
            } => {
                if self.apply_shrike_amulet_assemble(
                    item_id,
                    character_id,
                    cursor_item_id,
                    combined_bits,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::ShrikeAmuletDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::MineGatewayKeyAssemble {
                item_id,
                character_id,
                cursor_item_id,
                combined_bits,
            } => {
                if self.apply_mine_gateway_key_assemble(
                    item_id,
                    character_id,
                    cursor_item_id,
                    combined_bits,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::MineGatewayKeyDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::ArkhataKeyAssemble {
                item_id,
                character_id,
                cursor_item_id,
                result_template_id,
                result_sprite,
                final_key,
            } => {
                if self.apply_arkhata_key_assemble(
                    item_id,
                    character_id,
                    cursor_item_id,
                    result_template_id,
                    result_sprite,
                    final_key,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::ArkhataKeyDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::CaligarKeyAssemble {
                item_id,
                character_id,
                cursor_item_id,
                result_sprite,
                final_key,
            } => {
                if self.apply_caligar_key_assemble(
                    item_id,
                    character_id,
                    cursor_item_id,
                    result_sprite,
                    final_key,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::CaligarKeyDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::PalaceKeyCombine {
                item_id,
                character_id,
                cursor_item_id,
                result_sprite,
                final_key,
            } => {
                if self.apply_palace_key_combine(
                    item_id,
                    character_id,
                    cursor_item_id,
                    result_sprite,
                    final_key,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::PalaceKeyDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::LizardFlowerMixed {
                item_id,
                character_id,
                cursor_item_id,
                combined_bits,
                ..
            } => {
                if self.apply_lizard_flower_mixed(
                    item_id,
                    character_id,
                    cursor_item_id,
                    combined_bits,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::LizardFlowerDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::StatScrollUsed { character_id, .. } => {
                // See the `ItemDriverOutcome::StatScrollUsed` doc comment
                // for why a single batched `check_levelup` +
                // `update_character` pass here matches C's per-charge
                // `check_levelup(cn)`/`update_char(cn)` calls in
                // `raise_value_exp`.
                self.check_levelup(character_id);
                self.update_character(character_id);
                outcome
            }
            ItemDriverOutcome::LollipopLicked {
                character_id,
                exp_added,
                ..
            } => {
                // C `lollipop` (`base.c:3250`) calls `give_exp(cn, ...)`,
                // not a raw `ch[cn].exp +=`; see the doc comment on the
                // `lollipop_driver` call site (`item_driver/food.rs`) for
                // why the grant happens here instead of in the driver.
                self.give_exp(
                    character_id,
                    i64::from(exp_added),
                    u32::from(current_area_id),
                );
                outcome
            }
            ItemDriverOutcome::ShrikeAmbientRefresh {
                item_id,
                x,
                y,
                kind,
                night,
                schedule_after_ticks,
            } => {
                self.apply_shrike_ambient_refresh(item_id, x, y, kind, night, schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::ShrikeDoorEnter { character_id } => {
                self.apply_shrike_door_enter(character_id);
                outcome
            }
            ItemDriverOutcome::ShrikePoolTalismanCreated { cursor_item_id, .. } => {
                self.apply_shrike_pool_talisman(cursor_item_id);
                outcome
            }
            ItemDriverOutcome::ShrikeCubePush {
                item_id,
                from_x,
                from_y,
                to_x,
                to_y,
                ..
            } => {
                self.apply_shrike_cube_push(item_id, from_x, from_y, to_x, to_y);
                outcome
            }
            ItemDriverOutcome::ShrikeCubeAmbientTick {
                item_id,
                set_origin,
                reset_to,
                schedule_after_ticks,
            } => {
                self.apply_shrike_cube_ambient_tick(
                    item_id,
                    set_origin,
                    reset_to,
                    schedule_after_ticks,
                );
                outcome
            }
            _ => outcome,
        }
    }
}
