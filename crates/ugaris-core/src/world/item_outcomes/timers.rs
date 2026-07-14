use super::*;

impl World {
    pub fn process_due_timers(&mut self, area_id: u16) -> Vec<ItemDriverOutcome> {
        let mut outcomes = Vec::new();
        for event in self.timers.tick(self.tick.0) {
            match event.name.as_str() {
                ITEM_DRIVER_TIMER => {
                    let [driver, item_id, character_id, timer_call, _] = event.payload.0;
                    if driver <= 0 || item_id <= 0 || character_id < 0 {
                        continue;
                    }
                    let request = ItemDriverRequest::Driver {
                        driver: driver as u16,
                        item_id: ItemId(item_id as u32),
                        character_id: CharacterId(character_id as u32),
                        spec: 0,
                    };
                    outcomes.push(self.execute_item_driver_timer_request(
                        request,
                        area_id,
                        &ItemDriverContext {
                            timer_call: timer_call != 0,
                            ..ItemDriverContext::default()
                        },
                    ));
                }
                REMOVE_SPELL_TIMER => {
                    let [character_id, item_id, slot, character_serial, item_serial] =
                        event.payload.0;
                    if character_id <= 0 || item_id <= 0 || slot < 0 {
                        continue;
                    }
                    self.remove_spell_from_timer(
                        CharacterId(character_id as u32),
                        ItemId(item_id as u32),
                        slot as usize,
                        character_serial as u32,
                        item_serial as u32,
                    );
                }
                POISON_CALLBACK_TIMER => {
                    let [character_id, item_id, slot, character_serial, item_serial] =
                        event.payload.0;
                    if character_id <= 0 || item_id <= 0 || slot < 0 {
                        continue;
                    }
                    self.poison_callback_from_timer(
                        CharacterId(character_id as u32),
                        ItemId(item_id as u32),
                        slot as usize,
                        character_serial as u32,
                        item_serial as u32,
                    );
                }
                NPC_RESPAWN_TIMER => {
                    let [slot, ..] = event.payload.0;
                    self.queue_npc_respawn_from_timer(slot);
                }
                EXPIRE_ITEM_TIMER => {
                    let [item_id, ..] = event.payload.0;
                    self.expire_item_from_timer(item_id);
                }
                _ => {}
            }
        }
        outcomes
    }

    pub fn use_item_request(
        &mut self,
        request: ItemUseRequest,
        account_depot_available: bool,
    ) -> Result<UseItemOutcome, UseItemError> {
        let Some(character) = self.characters.get_mut(&request.character_id) else {
            return Err(UseItemError::IllegalCharacter);
        };
        let Some(item) = self.items.get(&request.item_id) else {
            return Err(UseItemError::IllegalItem);
        };
        use_item(character, item, request, account_depot_available)
    }

    pub fn execute_item_driver_request(
        &mut self,
        request: ItemDriverRequest,
        area_id: u16,
    ) -> ItemDriverOutcome {
        self.execute_item_driver_request_with_context(
            request,
            area_id,
            &ItemDriverContext::default(),
        )
    }

    pub fn execute_item_driver_request_with_context(
        &mut self,
        request: ItemDriverRequest,
        area_id: u16,
        context: &ItemDriverContext,
    ) -> ItemDriverOutcome {
        let (driver, character_id, item_id) = match request {
            ItemDriverRequest::Driver {
                driver,
                character_id,
                item_id,
                ..
            } => (Some(driver), character_id, item_id),
            ItemDriverRequest::AccountDepot {
                character_id,
                item_id,
            } => (None, character_id, item_id),
        };
        let character_tile_flags = self
            .characters
            .get(&character_id)
            .and_then(|character| {
                self.map
                    .tile(usize::from(character.x), usize::from(character.y))
            })
            .map(|tile| tile.flags)
            .unwrap_or_else(MapFlags::empty);
        let in_arena = character_tile_flags.contains(MapFlags::ARENA);
        let cursor_context = self
            .characters
            .get(&character_id)
            .and_then(|character| character.cursor_item)
            .and_then(|cursor_item_id| self.items.get(&cursor_item_id))
            .map(|item| {
                let cursor_drdata1_u32 = item
                    .driver_data
                    .get(1..5)
                    .and_then(|bytes| bytes.try_into().ok())
                    .map(u32::from_le_bytes)
                    .unwrap_or(0);
                (
                    item.template_id,
                    item.driver,
                    item.sprite,
                    item.driver_data.first().copied().unwrap_or(0),
                    cursor_drdata1_u32,
                )
            });
        let fdemon_loader_power = (matches!(driver, Some(IDR_FDEMONLIGHT | IDR_FDEMONCANNON))
            && context.fdemon_loader_power.is_none())
        .then(|| fdemon_loader_power_for_light(&self.items, item_id))
        .flatten();
        let edemon_section_power = (matches!(
            driver,
            Some(IDR_EDEMONLIGHT | IDR_EDEMONDOOR | IDR_EDEMONTUBE)
        ) && context.edemon_section_power.is_none())
        .then(|| edemon_section_power_for_light(&self.items, item_id))
        .flatten();
        let edemon_tube_target = (driver == Some(IDR_EDEMONTUBE)
            && context.edemon_tube_target.is_none())
        .then(|| edemon_tube_target(&self.items, &self.map, item_id))
        .flatten();
        let edemon_gate_spawn = (driver == Some(IDR_EDEMONGATE)
            && context.edemon_gate_spawn.is_none())
        .then(|| self.edemon_gate_spawn_context(item_id))
        .flatten();
        let fdemon_gate_spawn = (driver == Some(IDR_FDEMONGATE)
            && context.fdemon_gate_spawn.is_none())
        .then(|| self.fdemon_gate_spawn_context(item_id))
        .flatten();
        let dungeon_door_context = (driver == Some(IDR_DUNGEONDOOR))
            .then(|| self.dungeon_door_context(character_id, item_id));
        let deathfibrin_master = (driver == Some(IDR_DEATHFIBRIN)
            && context.deathfibrin_master.is_none())
        .then(|| self.deathfibrin_scan(character_id))
        .flatten();
        let deathfibrin_tile_light = if driver == Some(IDR_DEATHFIBRIN) {
            self.deathfibrin_tile_light(character_id)
        } else {
            0
        };
        let clanspawn_contested = if driver == Some(IDR_CLANSPAWN) {
            self.clanspawn_is_contested(character_id, item_id)
        } else {
            false
        };
        let random_shrine_key_context =
            if driver == Some(IDR_RANDOMSHRINE) && !context.has_matching_random_shrine_key {
                self.has_matching_random_shrine_key(character_id, item_id)
            } else {
                false
            };
        let shrike_cube_push_target = (driver == Some(IDR_SHRIKE)
            && context.shrike_cube_push_target.is_none()
            && self
                .items
                .get(&item_id)
                .is_some_and(|item| item.driver_data.first().copied().unwrap_or(0) == 5))
        .then(|| self.shrike_cube_push_target(character_id, item_id))
        .flatten();
        let area11_palace_key_context =
            if driver == Some(IDR_PALACEDOOR) && !context.has_area11_palace_key {
                self.character_has_template_id(character_id, IID_AREA11_PALACEKEY)
            } else {
                false
            };
        let area16_robber_key_context =
            if driver == Some(IDR_FORESTCHEST) && !context.has_area16_robber_key {
                self.character_has_template_id(character_id, IID_AREA16_ROBBERKEY)
            } else {
                false
            };
        let area16_skelly_key_context =
            if driver == Some(IDR_FORESTCHEST) && !context.has_area16_skelly_key {
                self.character_has_template_id(character_id, IID_AREA16_SKELLYKEY)
            } else {
                false
            };
        let mine_gateway_key_context =
            if driver == Some(IDR_MINEGATEWAY) && !context.has_mine_gateway_key {
                self.character_has_template_id(character_id, IID_MINEGATEWAY)
            } else {
                false
            };
        let warp_trial_door_context = (driver == Some(IDR_WARPTRIALDOOR)
            && context.warp_trial_door.is_none())
        .then(|| self.warp_trial_door_context(item_id))
        .flatten();
        let area25_door_key = (driver == Some(IDR_WARPKEYDOOR)
            && context.area25_door_key.is_none())
        .then(|| self.character_inventory_item_by_template(character_id, IID_AREA25_DOORKEY))
        .flatten();
        let mine_door_target = (driver == Some(IDR_MINEDOOR) && context.mine_door_target.is_none())
            .then(|| self.mine_door_target(item_id))
            .flatten();
        let swamp_arm_triggered = (driver == Some(IDR_SWAMPARM)
            && context.swamp_arm_triggered.is_none())
        .then(|| self.swamp_arm_triggered(item_id))
        .flatten();
        let swamp_whisp_move_succeeds = (driver == Some(IDR_SWAMPWHISP)
            && context.swamp_whisp_move_succeeds.is_none())
        .then(|| self.swamp_whisp_move_succeeds(item_id))
        .flatten();
        let swamp_spawn_live = (driver == Some(IDR_SWAMPSPAWN)
            && context.swamp_spawn_live.is_none())
        .then(|| self.swamp_spawn_live(item_id))
        .flatten();
        let swamp_spawn_player_close = (driver == Some(IDR_SWAMPSPAWN)
            && context.swamp_spawn_player_close.is_none())
        .then(|| self.swamp_spawn_player_close(item_id, 4))
        .flatten();
        let swamp_spawn_ground_sprite = (driver == Some(IDR_SWAMPSPAWN)
            && context.swamp_spawn_ground_sprite.is_none())
        .then(|| self.swamp_spawn_ground_sprite(item_id))
        .flatten();
        let Some(character) = self.characters.get_mut(&character_id) else {
            return ItemDriverOutcome::Noop;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return ItemDriverOutcome::Noop;
        };
        let mut effective_context = context.clone();
        effective_context.current_tick = self.tick.0 as u32;
        if effective_context.pent_last_solve_tick.is_none() {
            effective_context.pent_last_solve_tick = self.pentagram_quest.last_solve_tick;
        }
        if effective_context.pent_demon_lord_access_seconds.is_none() {
            effective_context.pent_demon_lord_access_seconds =
                Some(self.settings.get_demon_lord_door_after_solve_access_time() as u32);
        }
        if effective_context.fdemon_loader_power.is_none() {
            effective_context.fdemon_loader_power = fdemon_loader_power;
        }
        if effective_context.edemon_section_power.is_none() {
            effective_context.edemon_section_power = edemon_section_power;
        }
        if effective_context.edemon_tube_target.is_none() {
            effective_context.edemon_tube_target = edemon_tube_target;
        }
        if effective_context.edemon_gate_spawn.is_none() {
            effective_context.edemon_gate_spawn = edemon_gate_spawn;
        }
        if effective_context.fdemon_gate_spawn.is_none() {
            effective_context.fdemon_gate_spawn = fdemon_gate_spawn;
        }
        if let Some((has_key1, has_key2, defender_count)) = dungeon_door_context {
            effective_context.has_dungeon_door_key1 |= has_key1;
            effective_context.has_dungeon_door_key2 |= has_key2;
            effective_context.dungeon_defender_count = effective_context
                .dungeon_defender_count
                .or(Some(defender_count));
        }
        if effective_context.deathfibrin_master.is_none() {
            effective_context.deathfibrin_master = deathfibrin_master;
        }
        if effective_context.deathfibrin_tile_light == 0 {
            effective_context.deathfibrin_tile_light = deathfibrin_tile_light;
        }
        effective_context.clanspawn_contested |= clanspawn_contested;
        effective_context.has_matching_random_shrine_key |= random_shrine_key_context;
        if effective_context.shrike_cube_push_target.is_none() {
            effective_context.shrike_cube_push_target = shrike_cube_push_target;
        }
        if driver == Some(IDR_SHRIKE) {
            effective_context.is_fullnight = self.date.moonlight != 0 && self.date.sunlight < 100;
        }
        effective_context.has_area11_palace_key |= area11_palace_key_context;
        effective_context.has_area16_robber_key |= area16_robber_key_context;
        effective_context.has_area16_skelly_key |= area16_skelly_key_context;
        effective_context.has_mine_gateway_key |= mine_gateway_key_context;
        if effective_context.warp_trial_door.is_none() {
            effective_context.warp_trial_door = warp_trial_door_context;
        }
        if effective_context.area25_door_key.is_none() {
            effective_context.area25_door_key = area25_door_key;
        }
        if effective_context.mine_door_target.is_none() {
            effective_context.mine_door_target = mine_door_target;
        }
        if effective_context.swamp_arm_triggered.is_none() {
            effective_context.swamp_arm_triggered = swamp_arm_triggered;
        }
        if effective_context.swamp_whisp_move_succeeds.is_none() {
            effective_context.swamp_whisp_move_succeeds = swamp_whisp_move_succeeds;
        }
        if effective_context.swamp_spawn_live.is_none() {
            effective_context.swamp_spawn_live = swamp_spawn_live;
        }
        if effective_context.swamp_spawn_player_close.is_none() {
            effective_context.swamp_spawn_player_close = swamp_spawn_player_close;
        }
        if effective_context.swamp_spawn_ground_sprite.is_none() {
            effective_context.swamp_spawn_ground_sprite = swamp_spawn_ground_sprite;
        }
        if let Some((
            cursor_template_id,
            cursor_driver,
            cursor_sprite,
            cursor_drdata0,
            cursor_drdata1_u32,
        )) = cursor_context
        {
            effective_context.cursor_template_id = effective_context
                .cursor_template_id
                .or(Some(cursor_template_id));
            effective_context.cursor_driver =
                effective_context.cursor_driver.or(Some(cursor_driver));
            effective_context.cursor_sprite =
                effective_context.cursor_sprite.or(Some(cursor_sprite));
            effective_context.cursor_drdata0 =
                effective_context.cursor_drdata0.or(Some(cursor_drdata0));
            effective_context.cursor_drdata1_u32 = effective_context
                .cursor_drdata1_u32
                .or(Some(cursor_drdata1_u32));
        }
        effective_context.character_underwater |=
            character_tile_flags.contains(MapFlags::UNDERWATER);
        effective_context.daylight = effective_context
            .daylight
            .max(self.date.daylight.clamp(0, 255) as u8);
        let before = item.clone();
        let outcome = execute_item_driver_with_context(
            character,
            item,
            request,
            area_id,
            in_arena,
            &effective_context,
        );
        if item_light_may_have_changed(&outcome) {
            self.refresh_item_light_after_mutation(&before, item_id);
        }
        self.apply_item_driver_outcome(outcome, area_id)
    }

    pub(crate) fn execute_item_driver_timer_request(
        &mut self,
        request: ItemDriverRequest,
        area_id: u16,
        context: &ItemDriverContext,
    ) -> ItemDriverOutcome {
        let ItemDriverRequest::Driver {
            driver,
            item_id,
            character_id,
            spec,
        } = request
        else {
            return self.execute_item_driver_request_with_context(request, area_id, context);
        };

        if character_id.0 != 0 {
            return self.execute_item_driver_request_with_context(request, area_id, context);
        }

        let mut effective_context = context.clone();
        // C timer-driven drivers read the global `dlight`/`hour`/moon-phase
        // vars directly (e.g. `nightlight_driver`'s `dlight > 80` check in
        // `src/module/base.c:1819`); mirror that here so every driver timer
        // sees the live game clock instead of an all-zero default context.
        effective_context.daylight = self.date.daylight.clamp(0, 255) as u8;
        effective_context.hour = self.date.hour as u8;
        effective_context.fullmoon = self.date.fullmoon;
        effective_context.newmoon = self.date.newmoon;
        effective_context.solstice = self.date.solstice;
        effective_context.equinox = self.date.equinox;
        effective_context.is_fullnight = self.date.moonlight != 0 && self.date.sunlight < 100;
        if driver == IDR_SHRIKE && effective_context.shrike_cube_origin_clear.is_none() {
            effective_context.shrike_cube_origin_clear = self.shrike_cube_origin_clear(item_id);
        }
        if driver == IDR_EDEMONBALL && effective_context.edemon_fire_enabled.is_none() {
            effective_context.edemon_fire_enabled = Some(edemon_fire_enabled(&self.items));
        }
        if matches!(
            driver,
            IDR_EDEMONBALL | IDR_EDEMONLIGHT | IDR_EDEMONDOOR | IDR_EDEMONTUBE
        ) && effective_context.edemon_section_power.is_none()
        {
            effective_context.edemon_section_power =
                edemon_section_power_for_light(&self.items, item_id);
        }
        if driver == IDR_EDEMONTUBE && effective_context.edemon_tube_target.is_none() {
            effective_context.edemon_tube_target =
                edemon_tube_target(&self.items, &self.map, item_id);
        }
        if driver == IDR_EDEMONGATE && effective_context.edemon_gate_spawn.is_none() {
            effective_context.edemon_gate_spawn = self.edemon_gate_spawn_context(item_id);
        }
        if driver == IDR_FDEMONGATE && effective_context.fdemon_gate_spawn.is_none() {
            effective_context.fdemon_gate_spawn = self.fdemon_gate_spawn_context(item_id);
        }
        if driver == IDR_SWAMPWHISP && effective_context.swamp_whisp_move_succeeds.is_none() {
            effective_context.swamp_whisp_move_succeeds = self.swamp_whisp_move_succeeds(item_id);
        }
        if driver == IDR_SWAMPSPAWN {
            if effective_context.swamp_spawn_live.is_none() {
                effective_context.swamp_spawn_live = self.swamp_spawn_live(item_id);
            }
            if effective_context.swamp_spawn_player_close.is_none() {
                effective_context.swamp_spawn_player_close =
                    self.swamp_spawn_player_close(item_id, 4);
            }
            if effective_context.swamp_spawn_ground_sprite.is_none() {
                effective_context.swamp_spawn_ground_sprite =
                    self.swamp_spawn_ground_sprite(item_id);
            }
        }
        if matches!(driver, IDR_FDEMONLIGHT | IDR_FDEMONCANNON)
            && effective_context.fdemon_loader_power.is_none()
        {
            effective_context.fdemon_loader_power =
                fdemon_loader_power_for_light(&self.items, item_id);
        }
        if driver == IDR_TUNNELDOOR2 && effective_context.tunnel_door_area_clear.is_none() {
            if let Some((x, y)) = self.items.get(&item_id).map(|item| (item.x, item.y)) {
                effective_context.tunnel_door_area_clear =
                    Some(self.tunnel_mean_door_area_clear(x, y));
            }
        }

        let Some(item) = self.items.get_mut(&item_id) else {
            return ItemDriverOutcome::Noop;
        };
        effective_context.current_tick = self.tick.0 as u32;
        effective_context.daylight = effective_context
            .daylight
            .max(self.date.daylight.clamp(0, 255) as u8);
        let mut timer_character = timer_callback_character();
        let before = item.clone();
        let outcome = execute_item_driver_with_context(
            &mut timer_character,
            item,
            ItemDriverRequest::Driver {
                driver,
                item_id,
                character_id,
                spec,
            },
            area_id,
            false,
            &effective_context,
        );
        if item_light_may_have_changed(&outcome) {
            self.refresh_item_light_after_mutation(&before, item_id);
        }
        self.apply_item_driver_outcome(outcome, area_id)
    }

    pub fn schedule_item_driver_timer(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        after_ticks: u64,
    ) -> bool {
        self.schedule_item_driver_timer_with_context(item_id, character_id, after_ticks, true)
    }

    pub(crate) fn schedule_item_driver_timer_with_context(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        after_ticks: u64,
        timer_call: bool,
    ) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.driver == 0 {
            return false;
        }
        self.timers.set_timer(
            self.tick.0.saturating_add(after_ticks),
            ITEM_DRIVER_TIMER,
            TimerPayload([
                i32::from(item.driver),
                item_id.0 as i32,
                character_id.0 as i32,
                if timer_call { 1 } else { 0 },
                0,
            ]),
        )
    }

    pub(crate) fn schedule_map_item_driver_timer(
        &mut self,
        x: usize,
        y: usize,
        character_id: CharacterId,
        after_ticks: u64,
    ) -> bool {
        let Some(target_item_id) = self
            .map
            .tile(x, y)
            .and_then(|tile| (tile.item != 0).then_some(ItemId(tile.item)))
        else {
            return false;
        };
        self.schedule_item_driver_timer_with_context(
            target_item_id,
            character_id,
            after_ticks,
            character_id.0 == 0,
        )
    }
}
