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
        let deathfibrin_tile_light = (driver == Some(IDR_DEATHFIBRIN))
            .then(|| self.deathfibrin_tile_light(character_id))
            .unwrap_or(0);
        let clanspawn_contested = (driver == Some(IDR_CLANSPAWN))
            .then(|| self.clanspawn_is_contested(character_id, item_id))
            .unwrap_or(false);
        let random_shrine_key_context = (driver == Some(IDR_RANDOMSHRINE)
            && !context.has_matching_random_shrine_key)
            .then(|| self.has_matching_random_shrine_key(character_id, item_id))
            .unwrap_or(false);
        let shrike_cube_push_target = (driver == Some(IDR_SHRIKE)
            && context.shrike_cube_push_target.is_none()
            && self
                .items
                .get(&item_id)
                .is_some_and(|item| item.driver_data.first().copied().unwrap_or(0) == 5))
        .then(|| self.shrike_cube_push_target(character_id, item_id))
        .flatten();
        let area11_palace_key_context = (driver == Some(IDR_PALACEDOOR)
            && !context.has_area11_palace_key)
            .then(|| self.character_has_template_id(character_id, IID_AREA11_PALACEKEY))
            .unwrap_or(false);
        let area16_robber_key_context = (driver == Some(IDR_FORESTCHEST)
            && !context.has_area16_robber_key)
            .then(|| self.character_has_template_id(character_id, IID_AREA16_ROBBERKEY))
            .unwrap_or(false);
        let area16_skelly_key_context = (driver == Some(IDR_FORESTCHEST)
            && !context.has_area16_skelly_key)
            .then(|| self.character_has_template_id(character_id, IID_AREA16_SKELLYKEY))
            .unwrap_or(false);
        let mine_gateway_key_context = (driver == Some(IDR_MINEGATEWAY)
            && !context.has_mine_gateway_key)
            .then(|| self.character_has_template_id(character_id, IID_MINEGATEWAY))
            .unwrap_or(false);
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
                schedule_after_ticks: _,
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
                self.schedule_item_driver_timer(
                    item_id,
                    CharacterId(0),
                    u64::from(schedule_after_ticks),
                );
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
                self.schedule_item_driver_timer(
                    item_id,
                    CharacterId(0),
                    u64::from(schedule_after_ticks),
                );
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
                            self.tick.0 + (TICKS_PER_SECOND as u64 * 60 * 3);
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
                self.schedule_item_driver_timer(
                    item_id,
                    CharacterId(0),
                    u64::from(schedule_after_ticks),
                );
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
            .and_then(|tile| (tile.item != 0).then_some(ItemId(u32::from(tile.item))))
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

pub(crate) fn timer_callback_character() -> Character {
    Character {
        id: CharacterId(0),
        serial: 0,
        name: String::new(),
        description: String::new(),
        template_key: String::new(),
        respawn_ticks: 0,
        merchant: None,
        flags: CharacterFlags::empty(),
        sprite: 0,
        c1: 0,
        c2: 0,
        c3: 0,
        driver: 0,
        group: 0,
        clan: 0,
        clan_rank: 0,
        clan_serial: 0,
        staff_code: String::new(),
        speed_mode: SpeedMode::Normal,
        x: 0,
        y: 0,
        rest_area: 0,
        rest_x: 0,
        rest_y: 0,
        tox: 0,
        toy: 0,
        dir: 0,
        action: 0,
        duration: 0,
        step: 0,
        act1: 0,
        act2: 0,
        hp: 0,
        mana: 0,
        endurance: 0,
        lifeshield: 0,
        level: 0,
        exp: 0,
        exp_used: 0,
        military_points: 0,
        military_normal_exp: 0,
        gold: 0,
        karma: 0,
        creation_time: 0,
        saves: 0,
        got_saved: 0,
        deaths: 0,
        regen_ticker: 0,
        last_regen: 0,
        cursor_item: None,
        current_container: None,
        values: Character::empty_values(),
        professions: Character::empty_professions(),
        inventory: Character::empty_inventory(),
        driver_state: None,
        driver_messages: Vec::new(),
        driver_memory: crate::character_driver::DriverMemory::default(),
        class: 0,
        dungeonfighter: None,
        fight_driver: None,
        lq_usurp: None,
    }
}
