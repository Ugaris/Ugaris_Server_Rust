use super::*;

impl ZoneLoader {
    pub fn new() -> Self {
        Self {
            item_templates: HashMap::new(),
            character_templates: HashMap::new(),
            next_item_id: 1,
            next_character_id: 1,
            next_serial: 1,
        }
    }

    pub fn load_item_templates_str(&mut self, text: &str) -> Result<(), ZoneError> {
        for record in parse_zone_records(text)? {
            let template = item_template_from_record(record)?;
            self.item_templates.insert(template.key.clone(), template);
        }
        Ok(())
    }

    pub fn load_character_templates_str(&mut self, text: &str) -> Result<(), ZoneError> {
        for record in parse_zone_records(text)? {
            let template = character_template_from_record(record)?;
            self.character_templates
                .insert(template.key.clone(), template);
        }
        Ok(())
    }

    pub fn apply_map_str(&mut self, world: &mut World, text: &str) -> Result<(), ZoneError> {
        let directives = parse_map_directives(text)?;
        self.apply_map_directives(world, &directives)
    }

    pub fn instantiate_character_template(
        &mut self,
        key: &str,
        character_id: CharacterId,
    ) -> Result<(Character, Vec<Item>), ZoneError> {
        self.create_character_with_id(key, character_id)
    }

    pub fn instantiate_item_template(
        &mut self,
        key: &str,
        carried_by: Option<CharacterId>,
    ) -> Result<Item, ZoneError> {
        self.create_item(key, carried_by)
    }

    pub fn instantiate_item_template_by_id(
        &mut self,
        template_id: u32,
        carried_by: Option<CharacterId>,
    ) -> Option<Item> {
        let key = self
            .item_templates
            .values()
            .find(|template| template.template_id == template_id)?
            .key
            .clone();
        self.create_item(&key, carried_by).ok()
    }

    pub fn allocate_item_id(&mut self) -> ItemId {
        let id = ItemId(self.next_item_id);
        self.next_item_id = self.next_item_id.saturating_add(1).max(1);
        id
    }

    pub fn apply_map_directives(
        &mut self,
        world: &mut World,
        directives: &[MapDirective],
    ) -> Result<(), ZoneError> {
        let mut current_x = 0;
        let mut current_y = 0;
        let mut source_tile = MapTile::default();
        let mut from_x = 0;
        let mut from_y = 0;

        for directive in directives {
            match directive {
                MapDirective::Origin { .. } => {}
                MapDirective::Field { x, y } => {
                    current_x = *x;
                    current_y = *y;
                    let tile = world.map.tile_mut(current_x, current_y).ok_or(
                        ZoneError::MapOutOfBounds {
                            x: current_x,
                            y: current_y,
                        },
                    )?;
                    *tile = MapTile::default();
                    source_tile = *tile;
                }
                MapDirective::From { x, y } => {
                    from_x = *x;
                    from_y = *y;
                }
                MapDirective::To { x, y } => {
                    let copied_tile = tile_for_range_copy(source_tile);
                    for ty in from_y..=*y {
                        for tx in from_x..=*x {
                            let tile = world
                                .map
                                .tile_mut(tx, ty)
                                .ok_or(ZoneError::MapOutOfBounds { x: tx, y: ty })?;
                            apply_range_copy(tile, copied_tile);
                        }
                    }
                }
                MapDirective::GroundSprite(sprite) => {
                    let tile = world.map.tile_mut(current_x, current_y).ok_or(
                        ZoneError::MapOutOfBounds {
                            x: current_x,
                            y: current_y,
                        },
                    )?;
                    tile.ground_sprite = *sprite;
                    source_tile = *tile;
                }
                MapDirective::ForegroundSprite(sprite) => {
                    let tile = world.map.tile_mut(current_x, current_y).ok_or(
                        ZoneError::MapOutOfBounds {
                            x: current_x,
                            y: current_y,
                        },
                    )?;
                    tile.foreground_sprite = *sprite;
                    source_tile = *tile;
                }
                MapDirective::Character(key) => {
                    let (character, inventory_items) = match self.create_character(key) {
                        Ok(character) => character,
                        Err(ZoneError::UnknownCharacter(_)) => continue,
                        Err(err) => return Err(err),
                    };
                    let character_id = character.id;
                    place_character(world, character, inventory_items, current_x, current_y)?;
                    source_tile =
                        *world
                            .map
                            .tile(current_x, current_y)
                            .ok_or(ZoneError::MapOutOfBounds {
                                x: current_x,
                                y: current_y,
                            })?;
                    if let Some(character) = world.characters.get_mut(&character_id) {
                        character.x = current_x as u16;
                        character.y = current_y as u16;
                        // C `pop_create_char` stores the spawn tile in
                        // `ch.tmpx/tmpy`; the Rust port reuses the rest
                        // coordinates as the NPC home/respawn position.
                        character.rest_x = current_x as u16;
                        character.rest_y = current_y as u16;
                    }
                    // C `create.c:1121-1125`: spawn-mode loot table
                    // (`loot_table=` in the .chr template), rolled right
                    // after the NPC's own creation/placement.
                    let loot_table_id = self
                        .character_templates
                        .get(key)
                        .map(|template| template.loot_table.clone())
                        .unwrap_or_default();
                    if !loot_table_id.is_empty() {
                        world.loot_apply_to_npc(self, character_id, &loot_table_id);
                    }
                }
                MapDirective::Item(key) => {
                    let item = match self.create_item(key, None) {
                        Ok(item) => item,
                        Err(ZoneError::UnknownItem(_)) => continue,
                        Err(err) => return Err(err),
                    };
                    let item_id = item.id;
                    place_item(world, item, current_x, current_y)?;
                    source_tile =
                        *world
                            .map
                            .tile(current_x, current_y)
                            .ok_or(ZoneError::MapOutOfBounds {
                                x: current_x,
                                y: current_y,
                            })?;
                    if let Some(item) = world.items.get_mut(&item_id) {
                        item.x = current_x as u16;
                        item.y = current_y as u16;
                    }
                }
                MapDirective::Flag(flag) => {
                    let tile = world.map.tile_mut(current_x, current_y).ok_or(
                        ZoneError::MapOutOfBounds {
                            x: current_x,
                            y: current_y,
                        },
                    )?;
                    tile.flags.insert(*flag);
                    source_tile = *tile;
                }
            }
        }

        Ok(())
    }

    fn create_item(
        &mut self,
        key: &str,
        carried_by: Option<CharacterId>,
    ) -> Result<Item, ZoneError> {
        let template = self
            .item_templates
            .get(key)
            .ok_or_else(|| ZoneError::UnknownItem(key.to_string()))?
            .clone();
        let id = self.allocate_item_id();
        let serial = self.next_serial;
        self.next_serial += 1;

        Ok(Item {
            id,
            name: template.name.clone(),
            description: template.description.clone(),
            flags: template.flags | ItemFlags::USED,
            sprite: template.sprite,
            value: template.value,
            min_level: template.min_level,
            max_level: template.max_level,
            needs_class: template.needs_class,
            template_id: template.template_id,
            owner_id: 0,
            modifier_index: template.modifier_index,
            modifier_value: template.modifier_value,
            x: 0,
            y: 0,
            carried_by,
            contained_in: None,
            content_id: 0,
            driver: template.driver,
            driver_data: template.driver_data.clone(),
            serial,
        })
    }

    fn create_character(&mut self, key: &str) -> Result<(Character, Vec<Item>), ZoneError> {
        let id = CharacterId(self.next_character_id);
        self.next_character_id += 1;
        self.create_character_with_id(key, id)
    }

    fn create_character_with_id(
        &mut self,
        key: &str,
        id: CharacterId,
    ) -> Result<(Character, Vec<Item>), ZoneError> {
        let template = self
            .character_templates
            .get(key)
            .ok_or_else(|| ZoneError::UnknownCharacter(key.to_string()))?
            .clone();

        let mut values = Character::empty_values();
        values[1].clone_from(&template.base_values);
        values[0].clone_from(&template.base_values);

        let mut inventory = Character::empty_inventory();
        let mut inventory_items = Vec::new();
        for (slot, item_key) in template.inventory.iter().enumerate() {
            let Some(item_key) = item_key else {
                continue;
            };
            let item = self.create_item(item_key, Some(id))?;
            inventory[slot] = Some(item.id);
            inventory_items.push(item);
        }

        let mut character = Character {
            id,
            serial: id.0,
            name: template.name,
            description: template.description,
            flags: template.flags | CharacterFlags::USED,
            sprite: template.sprite,
            c1: 0,
            c2: 0,
            c3: 0,
            driver: template.driver,
            group: template.group as u16,
            clan: 0,
            clan_rank: 0,
            clan_serial: 0,
            staff_code: String::new(),
            speed_mode: Default::default(),
            x: 0,
            y: 0,
            rest_area: 0,
            rest_x: 0,
            rest_y: 0,
            tox: 0,
            toy: 0,
            dir: LEGACY_DIR_RIGHTDOWN,
            action: 0,
            duration: 0,
            step: 0,
            act1: 0,
            act2: 0,
            hp: i32::from(values[0][0]) * POWERSCALE,
            mana: i32::from(values[0][2]) * POWERSCALE,
            endurance: i32::from(values[0][1]) * POWERSCALE,
            lifeshield: 0,
            level: template.level_override.unwrap_or(0),
            exp: 0,
            exp_used: 0,
            military_points: 0,
            military_normal_exp: 0,
            gold: template.gold,
            karma: 0,
            creation_time: 0,
            saves: 0,
            got_saved: 0,
            deaths: 0,
            regen_ticker: 0,
            last_regen: 0,
            cursor_item: None,
            current_container: None,
            values,
            professions: template.professions,
            inventory,
            driver_state: None,
            driver_messages: Vec::new(),
            // C `ch.tmp`/`ch.respawn`: remember the source template and its
            // respawn delay so `respawn_callback` can recreate the NPC.
            template_key: key.to_string(),
            respawn_ticks: template
                .respawn_seconds
                .map(|seconds| seconds.saturating_mul(crate::tick::TICKS_PER_SECOND as u32))
                .unwrap_or(crate::game_settings::GameSettings::default().npc_respawn_timer as u32),
            merchant: None,
            driver_memory: crate::character_driver::DriverMemory::default(),
            class: template.class,
            dungeonfighter: None,
            fight_driver: None,
            lq_usurp: None,
        };

        if template.driver == CDR_SIMPLEBADDY {
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == crate::character_driver::CDR_MERCHANT {
            character.driver_state = Some(crate::character_driver::CharacterDriverState::Merchant(
                crate::character_driver::parse_merchant_driver_args(&template.args),
            ));
        }
        if template.driver == crate::character_driver::CDR_ACLERK {
            character.driver_state = Some(crate::character_driver::CharacterDriverState::Aclerk(
                crate::character_driver::parse_aclerk_driver_args(&template.args),
            ));
        }
        if template.driver == crate::character_driver::CDR_BANK {
            character.driver_state = Some(crate::character_driver::CharacterDriverState::Bank(
                crate::character_driver::parse_bank_driver_args(&template.args),
            ));
        }
        if template.driver == crate::character_driver::CDR_PROFESSOR {
            character.driver_state =
                Some(crate::character_driver::CharacterDriverState::Professor(
                    crate::character_driver::parse_professor_driver_args(&template.args),
                ));
        }
        if template.driver == CDR_TRADER {
            // C never parses zone-file args into `struct trader_data`
            // (`set_data` zero-initializes it) - no args to read here.
            character.driver_state =
                Some(CharacterDriverState::Trader(TraderDriverData::default()));
        }
        if template.driver == CDR_CLANMASTER {
            character.driver_state = Some(CharacterDriverState::Clanmaster(
                parse_clanmaster_driver_args(&template.args),
            ));
        }
        if template.driver == CDR_CLANCLERK {
            character.driver_state = Some(CharacterDriverState::Clanclerk(
                parse_clanclerk_driver_args(&template.args),
            ));
        }
        if template.driver == CDR_CLUBMASTER {
            character.driver_state = Some(CharacterDriverState::Clubmaster(
                parse_clubmaster_driver_args(&template.args),
            ));
        }
        if template.driver == CDR_ARENAMASTER {
            // C `master_driver` never parses zone-file args into `struct
            // master_data` (`set_data` zero-initializes it) - no args to
            // read here.
            character.driver_state = Some(CharacterDriverState::ArenaMaster(
                ArenaMasterDriverData::default(),
            ));
        }
        if template.driver == CDR_ARENAFIGHTER {
            // C `fighter_driver`'s `NT_CREATE` handler (`arena.c:850-855`):
            // parses `storage=N;` (unused here - no storage-blob primitive,
            // see `ArenaFighterDriverData`'s doc comment), then hardcodes
            // `restx`/`resty` to the arena's own rest tile regardless of
            // this NPC's actual zone-file spawn position, and seeds
            // `lastact` deeply in the past so the very first tick already
            // reads as "long enough ago" to advance past `FS_LEISURE`
            // without an initial multi-minute delay.
            character.rest_x = ARENA_FIGHTER_REST_POS.0;
            character.rest_y = ARENA_FIGHTER_REST_POS.1;
            character.driver_state =
                Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
                    last_act: -(crate::tick::TICKS_PER_SECOND as i64) * 60 * 6,
                    ..Default::default()
                }));
        }
        if template.driver == CDR_ARENAMANAGER {
            character.driver_state = Some(CharacterDriverState::ArenaManager(
                parse_arena_manager_driver_args(&template.args),
            ));
        }
        if template.driver == CDR_DUNGEONMASTER {
            // C never parses zone-file args into `struct master_data`
            // (`set_data` zero-initializes it) - no args to read here.
            character.driver_state = Some(CharacterDriverState::Dungeonmaster(
                DungeonmasterDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_DUNGEONFIGHTER {
            // C's `set_data(cn, DRD_DUNGEONFIGHTER, ...)` zero-initializes
            // `struct dungeonfighter_data` on first tick too - no zone-file
            // args to read here; that data now lives on
            // `Character::dungeonfighter` instead of `driver_state` (see
            // its doc comment) so the field below can hold the *other*
            // independent data blob C keeps for this same character: its
            // own `dungeonfighter`'s tail `char_driver(CDR_SIMPLEBADDY,
            // CDT_DRIVER, cn, ret, lastact)` call (`dungeon.c:2161`) reuses
            // the SimpleBaddy driver's full idle-wander/auto-attack AI on
            // this NPC, and that driver's own `NT_CREATE` handler parses
            // `ch[cn].arg` as `struct simplebaddy_data` fields
            // (`simple_baddy.c:174-189`) - the "warrior"/"mage"/"seyan"
            // dungeon-guard templates do carry an `arg="aggressive=1;...
            // "` string (`zones/13/dungeon.chr`) for exactly this purpose,
            // even though `dungeonfighter` itself never reads `ch[cn].arg`.
            character.dungeonfighter =
                Some(crate::character_driver::DungeonfighterDriverData::default());
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == crate::character_driver::CDR_PENTER {
            // C `pents.c::demon_character_driver` (`pents.c:1594-1603`):
            // every pentagram-quest guardian demon's own tail call is
            // `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, character_id, ret,
            // lastact)`, reusing the SimpleBaddy driver's full idle-wander/
            // auto-attack AI wholesale - same precedent as
            // `CDR_DUNGEONFIGHTER` above. The `penterN` templates
            // (`zones/4/pents.chr`) carry the same
            // `arg="aggressive=1;helper=0;scavenger=...;"` shape
            // SimpleBaddy's own `NT_CREATE` handler parses.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == crate::character_driver::CDR_SWAMPMONSTER {
            // C `ch_driver`'s `CDR_SWAMPMONSTER` dispatch (`swamp.c:807-
            // 809`): an unconditional every-tick tail call to
            // `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret,
            // lastact)`, reusing the SimpleBaddy driver's full idle-
            // wander/auto-attack AI wholesale - same precedent as
            // `CDR_PENTER`/`CDR_DUNGEONFIGHTER` above. The `swamp25n`/
            // `swamp27n`/`swamp29n`/`swamp31n` templates
            // (`zones/15/swamp.chr`) carry the same
            // `arg="aggressive=1;helper=0;scavenger=...;"` shape
            // SimpleBaddy's own `NT_CREATE` handler parses.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_SWAMPCLARA {
            // C never parses zone-file args into `struct
            // clara_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::Clara(ClaraDriverData::default()));
        }
        if template.driver == CDR_FORESTMONSTER {
            // C `ch_driver`'s `CDR_FORESTMONSTER` dispatch (`forest.c:909-
            // 911`): an unconditional every-tick tail call to
            // `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret,
            // lastact)`, reusing the SimpleBaddy driver's full idle-
            // wander/auto-attack AI wholesale - same precedent as
            // `CDR_PENTER`/`CDR_SWAMPMONSTER` above. The `wolf33`/
            // `bear35`/`skeleton38`/`skeleton38_key` templates
            // (`zones/16/forest.chr`) carry the same
            // `arg="aggressive=1;helper=0;scavenger=...;"` shape
            // SimpleBaddy's own `NT_CREATE` handler parses.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == crate::character_driver::CDR_TWOROBBER {
            // C `ch_driver`'s `CDR_TWOROBBER` dispatch (`two.c:3163-3165`):
            // an unconditional every-tick tail call to
            // `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret,
            // lastact)`, reusing the SimpleBaddy driver's full idle-
            // wander/auto-attack AI wholesale - same precedent as
            // `CDR_PENTER`/`CDR_SWAMPMONSTER`/`CDR_FORESTMONSTER` above.
            // The `robber1`-`robber4`/`robber_guard`/`robber_baron`
            // templates (`zones/17/two.chr`) carry the same
            // `arg="aggressive=1;helper=0;scavenger=...;"` shape
            // SimpleBaddy's own `NT_CREATE` handler parses.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_TEUFELDEMON {
            // C `ch_driver`'s `CDR_TEUFELDEMON` dispatch
            // (`teufel.c:373-394`): an `NT_CHAR` self-defense hook
            // (ported as `World::apply_teufeldemon_sighting_messages`,
            // see `world::npc::area34::teufeldemon`'s module doc comment)
            // followed by an unconditional every-tick tail call to
            // `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret,
            // lastact)`, reusing the SimpleBaddy driver's full idle-
            // wander/auto-attack AI wholesale - same precedent as
            // `CDR_PENTER`/`CDR_SWAMPMONSTER`/`CDR_FORESTMONSTER`/
            // `CDR_TWOROBBER` above. The `teufer1`-`teufer3` templates
            // (`zones/34/teufel.chr`) carry the same
            // `arg="aggressive=0;helper=0;scavenger=0;startdist=20;
            // chardist=0;stopdist=40;"` shape SimpleBaddy's own
            // `NT_CREATE` handler parses.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_TEUFELRAT {
            // C `ch_driver`'s `CDR_TEUFELRAT` dispatch (`teufel.c:1610-
            // 1626`): `teufelrat_driver`'s own `NT_CHAR` case body is
            // empty (commented out in C - `// co = msg->dat1;`), so this
            // is effectively a pure unconditional every-tick tail call to
            // `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret,
            // lastact)`, reusing the SimpleBaddy driver's full idle-
            // wander/auto-attack AI wholesale - same precedent as
            // `CDR_PENTER`/`CDR_SWAMPMONSTER`/`CDR_FORESTMONSTER`/
            // `CDR_TWOROBBER`/`CDR_TEUFELDEMON` above (`teufelrat_dead`'s
            // own kill-scoring, ported separately as `PlayerRuntime::
            // add_teufel_rat_kill`/`world_events::death_hooks::
            // apply_teufel_rat_death_from_hurt_event`, is the only other
            // C-visible behavior this driver has). The `rat70`-`rat94b`
            // templates (`zones/34/teufel.chr`) carry the same
            // `arg="aggressive=1;helper=1;scavenger=10;startdist=15;
            // chardist=0;stopdist=40;"` shape SimpleBaddy's own
            // `NT_CREATE` handler parses.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_ARKHATAPRISON {
            // C `ch_driver`'s `CDR_ARKHATAPRISON` dispatch (`arkhata.c:
            // 4616-4618`): `prisoner_driver`'s entire body is an
            // unconditional tail call to `char_driver(CDR_SIMPLEBADDY,
            // CDT_DRIVER, cn, ret, lastact)`, reusing the SimpleBaddy
            // driver's full idle-wander/auto-attack AI wholesale - same
            // precedent as `CDR_TEUFELRAT`/`CDR_CALIGARSKELLY` above
            // (`prisoner_dead`'s own "I know the secret, it's right
            // here!" line, ported separately as `ugaris-server::
            // world_events::death_hooks::
            // apply_arkhata_prisoner_death_from_hurt_event`, is the only
            // other C-visible behavior this driver has). The
            // `Fortress_Enemies_and_Guard.chr` template carries no
            // `arg=` line either.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == crate::character_driver::CDR_FORTRESSGUARD {
            // C `ch_driver`'s `CDR_FORTRESSGUARD` dispatch (`arkhata.c:
            // 4592-4594`) calls `fortressguard_driver`, its own
            // `simple_baddy_driver`-equivalent reimplementation over the
            // same `DRD_SIMPLEBADDYDRIVER` storage slot (see
            // `CDR_FORTRESSGUARD`'s own doc comment in
            // `character_driver.rs` for the two real behavioral deltas -
            // the `IID_ARKHATA_LETTER5` entrance-pass aggro exemption and
            // the always-false `drinkInvPots` no-op difference). Spawn
            // hookup follows the exact `CDR_ARKHATAPRISON`/`CDR_BOOKEATER`/
            // `CDR_ARKHATASKELLY` precedent above.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == crate::character_driver::CDR_BOOKEATER {
            // C `ch_driver`'s `CDR_BOOKEATER` dispatch (`arkhata.c:2083-
            // 2085`): `bookeater_driver`'s entire body is an
            // unconditional tail call to `char_driver(CDR_SIMPLEBADDY,
            // CDT_DRIVER, cn, ret, lastact)`, reusing the SimpleBaddy
            // driver's full idle-wander/auto-attack AI wholesale - same
            // precedent as `CDR_ARKHATAPRISON` above (`bookeater_dead`'s
            // own quest-70 completion, ported separately as `ugaris-
            // server::world_events::death_hooks::
            // apply_arkhata_bookeater_death_from_hurt_event`, is the
            // only other C-visible behavior this driver has). Unlike
            // `CDR_ARKHATAPRISON`, "The Book Eater"'s own
            // `Knogers_Creeper.chr` template *does* carry an `arg=
            // "aggressive=1;helper=1;scavenger=0;startdist=20;
            // chardist=0;stopdist=30;"` line - this spawn-time hookup
            // was missing until now (a real pre-existing gap: without
            // it `character.driver_state` stayed `None`, so every
            // `matches!(character.driver_state,
            // Some(CharacterDriverState::SimpleBaddy(_)))` gate in
            // `npc_fight.rs`/`npc_idle.rs` silently skipped every Book
            // Eater character - it could never fight back or idle-wander
            // even though the driver-id gates already listed it).
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == crate::character_driver::CDR_ARKHATASKELLY {
            // C `ch_driver`'s `CDR_ARKHATASKELLY` dispatch (`arkhata.c:
            // 4620-4622`): `arkhataskelly_driver`'s only other behavior
            // is a purely internal idle-tick position-hash bookkeeping
            // array (`skelly_ID`/`skelly_cn`, `:1587-1608`) used solely
            // to count still-alive arkhataskellies inside
            // `arkhataskelly_dead` - not ported since it has no
            // externally-visible effect (`World::
            // apply_arkhataskelly_death_from_hurt_event` counts living
            // `CDR_ARKHATASKELLY` characters directly from `self.
            // characters` instead, which is behaviorally equivalent);
            // the driver body itself is an unconditional tail call to
            // `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret,
            // lastact)`, reusing the SimpleBaddy driver's full idle-
            // wander/auto-attack AI wholesale - same precedent as
            // `CDR_ARKHATAPRISON`/`CDR_BOOKEATER` above. The
            // `Skeleton_for_final_area` template
            // (`zones/37/Vamp_Skele_Zombie.chr`) carries the same
            // `arg="aggressive=1;helper=1;scavenger=0;startdist=40;
            // chardist=0;stopdist=60;"` shape SimpleBaddy's own
            // `NT_CREATE` handler parses.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_NOP {
            // C `nop_driver`'s `dir=` zone-file arg is parsed at
            // `set_data`-creation time in C (the driver-data struct is
            // per-character transient state, not re-parsed every tick) -
            // ported here at spawn time instead of through the per-tick
            // message loop, same precedent as `CDR_TWOGUARD` above. See
            // `world::npc::area37::nop`'s module doc comment.
            character.driver_state = Some(CharacterDriverState::Nop(parse_nop_driver_args(
                &template.args,
            )));
        }
        if template.driver == CDR_RAMMY {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::Rammy(RammyDriverData::default()));
        }
        if template.driver == CDR_JAZ {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY` above.
            character.driver_state = Some(CharacterDriverState::Jaz(JazDriverData::default()));
        }
        if template.driver == crate::character_driver::CDR_FIONA {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ` above.
            character.driver_state = Some(CharacterDriverState::Fiona(
                crate::world::npc::area37::fiona::FionaDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_BRIDGEGUARD {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ` above.
            character.driver_state = Some(CharacterDriverState::BridgeGuard(
                crate::world::npc::area37::bridgeguard::BridgeGuardDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_RAMIN {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ` above.
            character.driver_state = Some(CharacterDriverState::Ramin(
                crate::world::npc::area37::ramin::RaminDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_ARKHATAMONK {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`
            // above.
            character.driver_state = Some(CharacterDriverState::Arkhatamonk(
                crate::world::npc::area37::arkhatamonk::ArkhatamonkDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_CAPTAIN {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`
            // above.
            character.driver_state = Some(CharacterDriverState::Captain(
                crate::world::npc::area37::captain::CaptainDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_JUDGE {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`
            // above.
            character.driver_state = Some(CharacterDriverState::Judge(
                crate::world::npc::area37::judge::JudgeDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_JADA {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`
            // above.
            character.driver_state = Some(CharacterDriverState::Jada(
                crate::world::npc::area37::jada::JadaDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_POTMAKER {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`/
            // `CDR_JADA` above.
            character.driver_state = Some(CharacterDriverState::Potmaker(
                crate::world::npc::area37::potmaker::PotmakerDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_HUNTER {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`/
            // `CDR_JADA`/`CDR_POTMAKER` above.
            character.driver_state = Some(CharacterDriverState::Hunter(
                crate::world::npc::area37::hunter::HunterDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_THAIPAN {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`/
            // `CDR_JADA`/`CDR_POTMAKER`/`CDR_HUNTER` above.
            character.driver_state = Some(CharacterDriverState::Thaipan(
                crate::world::npc::area37::thaipan::ThaipanDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_TRAINER {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`/
            // `CDR_JADA`/`CDR_POTMAKER`/`CDR_HUNTER`/`CDR_THAIPAN` above.
            character.driver_state = Some(CharacterDriverState::Trainer(
                crate::world::npc::area37::trainer::TrainerDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_KIDNAPPEE {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`/
            // `CDR_JADA`/`CDR_POTMAKER`/`CDR_HUNTER`/`CDR_THAIPAN` above.
            character.driver_state = Some(CharacterDriverState::Kidnappee(
                crate::world::npc::area37::kidnappee::KidnappeeDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_ARKHATACLERK {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`/
            // `CDR_JADA`/`CDR_POTMAKER`/`CDR_HUNTER`/`CDR_THAIPAN` above.
            character.driver_state = Some(CharacterDriverState::Clerk(
                crate::world::npc::area37::clerk::ClerkDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_KRENACH {
            // C never parses zone-file args into `struct
            // std_npc_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_RAMMY`/`CDR_JAZ`/`CDR_RAMIN`/
            // `CDR_JADA`/`CDR_POTMAKER`/`CDR_HUNTER`/`CDR_THAIPAN` above.
            character.driver_state = Some(CharacterDriverState::Krenach(
                crate::world::npc::area37::krenach::KrenachDriverData::default(),
            ));
        }
        if template.driver == CDR_CALIGARGUARD2 {
            // C `ch_driver`'s `CDR_CALIGARGUARD2` dispatch
            // (`caligar.c:395-442`): `guard2_driver`'s own `NT_CHAR` loop
            // (ported as `World::process_caligar_guard2_actions`, see
            // `world::npc::area36::caligar_guard2`'s module doc comment) is
            // followed by an unconditional every-tick tail call to
            // `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret,
            // lastact)`, reusing the SimpleBaddy driver's full idle-
            // wander/auto-attack AI wholesale - same precedent as
            // `CDR_PENTER`/`CDR_TEUFELRAT` above. The `Caligar_City.chr`
            // template carries no `arg=` line, same "nothing to parse"
            // precedent as `CDR_CALIGARSKELLY` below.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_CALIGARSKELLY {
            // C `ch_driver`'s `CDR_CALIGARSKELLY` dispatch
            // (`caligar.c:444-446`): `skelly_driver`'s entire body is an
            // unconditional tail call to `char_driver(CDR_SIMPLEBADDY,
            // CDT_DRIVER, cn, ret, lastact)`, reusing the SimpleBaddy
            // driver's full idle-wander/auto-attack AI wholesale - same
            // precedent as `CDR_PENTER`/`CDR_TEUFELRAT` above
            // (`skelly_dead_driver`'s own rune-door-unlock reward, ported
            // separately as `PlayerRuntime::mark_caligar_skelly_death`/
            // `world_events::death_hooks::
            // apply_caligar_skelly_death_from_hurt_event`, is the only
            // other C-visible behavior this driver has). The
            // `Caligar_Palace.chr` templates carry no `arg=` line either.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_FORESTIMP {
            // C never parses zone-file args into `struct
            // imp_driver_data` (`set_data` zero-initializes it) - no
            // args to read here (the zone file's own `arg="aggressive=
            // ...";` line is dead weight for this driver - `imp_driver`
            // never calls `fight_driver_set_dist`/reads `ch[cn].arg` at
            // all), same as `CDR_SWAMPCLARA` above.
            character.driver_state = Some(CharacterDriverState::ForestImp(
                ForestImpDriverData::default(),
            ));
        }
        if template.driver == CDR_FORESTWILLIAM {
            // C never parses zone-file args into `struct
            // william_driver_data` (`set_data` zero-initializes it) - no
            // `arg=` line exists for this template at all
            // (`zones/16/forest.chr`), same as `CDR_SWAMPCLARA` above.
            character.driver_state = Some(CharacterDriverState::ForestWilliam(
                ForestWilliamDriverData::default(),
            ));
        }
        if template.driver == CDR_FORESTHERMIT {
            // C never parses zone-file args into `struct
            // hermit_driver_data` (`set_data` zero-initializes it) - no
            // `arg=` line exists for this template at all
            // (`zones/16/forest.chr`), same as `CDR_SWAMPCLARA` above.
            character.driver_state = Some(CharacterDriverState::ForestHermit(
                ForestHermitDriverData::default(),
            ));
        }
        if template.driver == CDR_TWOBARKEEPER {
            // C never parses zone-file args into `struct barkeeper_data`
            // (`set_data` zero-initializes it) - no `arg=` line exists for
            // this template (`zones/17/two.chr`'s `barkeeper`), same as
            // `CDR_SWAMPCLARA` above.
            character.driver_state = Some(CharacterDriverState::TwoBarkeeper(
                TwoBarkeeperDriverData::default(),
            ));
        }
        if template.driver == CDR_TWOSERVANT {
            // C `servant`'s `NT_CREATE` handler (`two.c:995-999`):
            // `servant_parse(cn, dat)` parsed here at spawn time instead
            // of through the per-tick message loop, same precedent as
            // `CDR_TWOGUARD` above - see `world::npc::area17::servant`'s
            // module doc comment. Every `palace_maid*`/similar template
            // (`zones/17/two.chr`) carries a single `arg="nr=N;"` line.
            character.driver_state = Some(CharacterDriverState::TwoServant(
                crate::world::npc::area17::servant::parse_two_servant_driver_args(&template.args),
            ));
        }
        if template.driver == CDR_TWOSANWYN {
            // C never parses zone-file args into `struct
            // sanwyn_driver_data` (`set_data` zero-initializes it) - no
            // `arg=` line exists for this template (`zones/17/two.chr`'s
            // `sanwyn`), same as `CDR_SWAMPCLARA` above.
            character.driver_state = Some(CharacterDriverState::TwoSanwyn(
                TwoSanwynDriverData::default(),
            ));
        }
        if template.driver == CDR_TWOSKELLY {
            // C never parses zone-file args into `struct
            // skelly_driver_data` (`set_data` zero-initializes it) - no
            // `arg=` line exists for this template (`zones/17/two.chr`'s
            // `quest_skeleton`), same as `CDR_SWAMPCLARA` above. This
            // template is never statically spawned by a zone's initial
            // load - it's raised at runtime by `IDR_SKELRAISE` via
            // `ugaris-server::area_apply::raise_skeleton_from_template`,
            // which reuses this same instantiation path.
            character.driver_state = Some(CharacterDriverState::TwoSkelly(
                TwoSkellyDriverData::default(),
            ));
        }
        if template.driver == CDR_TWOTHIEFGUARD {
            // C never parses zone-file args into `struct
            // thiefguard_data` (`set_data` zero-initializes it) - no
            // `arg=` line exists for this template (`zones/17/two.chr`'s
            // `thief_guard`), same as `CDR_SWAMPCLARA` above.
            character.driver_state = Some(CharacterDriverState::TwoThiefGuard(
                TwoThiefGuardDriverData::default(),
            ));
        }
        if template.driver == CDR_TWOTHIEFMASTER {
            // C never parses zone-file args into `struct
            // thiefmaster_data` (`set_data` zero-initializes it) - no
            // `arg=` line exists for this template (`zones/17/two.chr`'s
            // `thiefmaster`), same as `CDR_SWAMPCLARA` above.
            character.driver_state = Some(CharacterDriverState::TwoThiefMaster(
                TwoThiefMasterDriverData::default(),
            ));
        }
        if template.driver == CDR_TWOALCHEMIST {
            // C never parses zone-file args into `struct
            // alchemist_driver_data` (`set_data` zero-initializes it) -
            // no `arg=` line exists for this template (`zones/17/
            // two.chr`'s `cervik`), same as `CDR_SWAMPCLARA` above.
            character.driver_state = Some(CharacterDriverState::TwoAlchemist(
                TwoAlchemistDriverData::default(),
            ));
        }
        if template.driver == CDR_TWOGUARD {
            // C `guard_driver`'s `NT_CREATE` handler (`two.c:381-386`):
            // `guard_parse(cn, dat)` parsed here at spawn time instead of
            // through the per-tick message loop, same precedent as
            // `CDR_PALACEGUARD` above - see `world::npc::area17::guard`'s
            // module doc comment. Some `two_guard*` templates
            // (`zones/17/two.chr`) carry repeated `arg="patx=N;paty=N;"`
            // lines (patrol waypoints); most carry none at all (a
            // stationary post).
            character.driver_state = Some(CharacterDriverState::TwoGuard(
                crate::world::npc::area17::guard::parse_two_guard_driver_args(&template.args),
            ));
        }
        if template.driver == crate::character_driver::CDR_FDEMON_DEMON {
            // C `fdemon_demon`'s own very first check (`fdemon.c:2746-2749`)
            // is `if (ch[cn].sprite==190) { char_driver(CDR_SIMPLEBADDY,
            // CDT_DRIVER, cn, ret, lastact); return; }` - an unconditional
            // every-tick tail call with no other logic, so the
            // `sprite==190` "Fire Golem" boss template (`fdemon_big1`,
            // `ugaris_data/zones/8/fire.chr`) is 100% indistinguishable
            // from a plain `CDR_SIMPLEBADDY` character. Assign it
            // `CDR_SIMPLEBADDY` directly (with its real zone-file
            // `arg="...";`, which C's own SimpleBaddy tail call parses on
            // this same first tick) instead of `CDR_FDEMON_DEMON` here -
            // see `world::npc::area8::fdemon_demon`'s module doc comment.
            //
            // The other, non-190-sprite "Fire Demon" trash-mob templates
            // (`fdemon1s..fdemon10s`) genuinely run `CDR_FDEMON_DEMON`'s
            // own extra hunt/gohome logic on top of a reused SimpleBaddy
            // driver state - C's own `NT_CREATE` handler for those never
            // parses zone-file args (`ch[cn].arg=NULL`), hardcoding
            // `fight_driver_set_dist(cn, 0, 30, 0)` instead, and its
            // per-tick message handling always calls
            // `standard_message_driver(cn, msg, 1, 1)` (hardcoded
            // `aggressive=1, helper=1`, not args-driven either).
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            if template.sprite == 190 {
                character.driver = CDR_SIMPLEBADDY;
                apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
            } else {
                apply_simple_baddy_create_message(&mut character, None, 0);
                if let Some(fight_driver) = character.fight_driver.as_mut() {
                    fight_driver.start_dist = 0;
                    fight_driver.char_dist = 30;
                    fight_driver.stop_dist = 0;
                }
                if let Some(CharacterDriverState::SimpleBaddy(data)) =
                    character.driver_state.as_mut()
                {
                    data.aggressive = 1;
                    data.helper = 1;
                }
            }
        }
        if template.driver == crate::character_driver::CDR_MILITARY_MASTER {
            character.driver_state = Some(CharacterDriverState::MilitaryMaster(
                crate::character_driver::parse_military_master_driver_args(&template.args),
            ));
        }
        if template.driver == crate::character_driver::CDR_MILITARY_ADVISOR {
            character.driver_state = Some(CharacterDriverState::MilitaryAdvisor(
                crate::character_driver::parse_military_advisor_driver_args(&template.args),
            ));
        }
        if template.driver == CDR_JANITOR {
            // C never parses zone-file args into `struct janitor_data`
            // (`set_data` zero-initializes it) - no args to read here.
            character.driver_state =
                Some(CharacterDriverState::Janitor(JanitorDriverData::default()));
        }
        if template.driver == CDR_GATE_WELCOME {
            // C never parses zone-file args into `struct
            // gate_welcome_driver_data` (`set_data` zero-initializes it) -
            // no args to read here.
            character.driver_state = Some(CharacterDriverState::GateWelcome(
                GateWelcomeDriverData::default(),
            ));
        }
        if template.driver == CDR_GATE_FIGHT {
            // C `create_char` generically does `notify_char(n, NT_CREATE,
            // ticker, 0, 0)` (`create.c:1128`); `gate_fight_driver` reads
            // this on its own next tick to seed `dat->creation_time`
            // (`gatekeeper.c:653-656`). No zone-file args to parse - C's
            // `struct gate_fight_driver_data` is zero-initialized by
            // `set_data`, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::GateFight(
                GateFightDriverData::default(),
            ));
            character.push_driver_message(NT_CREATE, 0, 0, 0);
        }
        if template.driver == CDR_GOLEMKEYHOLDER {
            // C `keyhold_fight_driver` reuses `struct gate_fight_driver_
            // data` and is zero-initialized by `set_data` the same way
            // `CDR_GATE_FIGHT` is above (`mine.c:1211-1220`); this NPC is
            // never statically placed by any zone file - it is only ever
            // created at runtime by `keyholder_door`
            // (`ugaris-server::mine::spawn_keyholder_golem`), which also
            // sets `victim`/`rest_x`/`rest_y` after this returns (see
            // `world::npc::area12::golemkeyholder`'s module doc comment).
            character.driver_state = Some(CharacterDriverState::GolemKeyhold(
                GolemKeyholdDriverData::default(),
            ));
            character.push_driver_message(NT_CREATE, 0, 0, 0);
        }
        if template.driver == CDR_SUPERIOR {
            // C `superior_driver`'s `NT_CREATE` handler (`area2.c:99-100`):
            // `dat->nr = atoi(ch[cn].arg); dat->mode = M_FIGHT;` - parsed
            // here at spawn time instead (see `world::superior`'s module
            // doc comment).
            let nr = template.args.trim().parse::<i32>().unwrap_or(0);
            character.driver_state = Some(CharacterDriverState::Superior(SuperiorDriverData {
                nr,
                mode: crate::world::npc::area2::superior::SUPERIOR_MODE_FIGHT,
                ..Default::default()
            }));
        }
        if template.driver == CDR_LAB2UNDEAD {
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_lab2_undead_create_message(&mut character, Some(&template.args));
            self.add_lab2_undead_regenerate_spell(&mut character, &mut inventory_items);
        }
        if template.driver == CDR_LABGNOMEDRIVER {
            // Data-only half of C's `NT_CREATE` handler; the map-dependent
            // remainder (idle-facing direction, torch scan) runs on this
            // character's first live tick instead - see
            // `world::npc::area22::lab1_gnome::apply_labgnome_create_message`'s
            // own doc comment.
            apply_labgnome_create_message(&mut character, Some(&template.args));
        }
        if template.driver == CDR_LAB2HERALD {
            // C never parses zone-file args into `struct
            // lab2_herald_driver_data` (`set_data` zero-initializes it) -
            // no args to read here, same as `CDR_LABGNOMEDRIVER`'s own
            // torch-gnome data (which does parse args) shows is the norm
            // for this dispatch site - see
            // `world::npc::area22::lab2_herald::apply_lab2_herald_create_
            // message`'s own doc comment.
            apply_lab2_herald_create_message(&mut character);
        }
        if template.driver == crate::character_driver::CDR_LAB3PASSGUARD {
            // C never parses zone-file args into `struct
            // lab3_passguard_driver_data` (`set_data` zero-initializes
            // it) - no args to read here, same as `CDR_LAB2HERALD` above.
            // The `NT_CREATE` message must be pushed explicitly (unlike
            // `CDR_LAB2HERALD`'s C driver, this one reacts to it - see
            // `world::npc::area22::lab3_passguard`'s module doc comment).
            character.driver_state = Some(CharacterDriverState::Lab3Passguard(
                crate::world::npc::area22::lab3_passguard::Lab3PassguardDriverData::default(),
            ));
            character.push_driver_message(NT_CREATE, 0, 0, 0);
        }
        if template.driver == crate::character_driver::CDR_LAB3PRISONER {
            // C never parses zone-file args into `struct
            // lab3_prisoner_driver_data` (`set_data` zero-initializes
            // it), and the C driver has no `NT_CREATE` handler either -
            // no `NT_CREATE` push needed here.
            character.driver_state = Some(CharacterDriverState::Lab3Prisoner(
                crate::world::npc::area22::lab3_prisoner::Lab3PrisonerDriverData::default(),
            ));
        }
        if template.driver == CDR_LAB4SEYAN {
            // C never parses zone-file args into `struct lab4_seyan_data`
            // (`zones/22/lab4.chr`'s `lab4_seyan` template has no `arg=`),
            // and the C driver has no `NT_CREATE` handler either - no
            // `NT_CREATE` push needed here, same as `CDR_LAB3PRISONER`
            // above.
            apply_lab4_seyan_create_message(&mut character);
        }
        if template.driver == CDR_LAB4GNALB {
            // Data-only half of C's `NT_CREATE` handler (parses `type=`);
            // the map-dependent remainder (nearest-path-node lookup for
            // `type=1`) runs on this character's first live tick instead
            // - see `world::npc::area22::lab4_gnalb::
            // apply_lab4_gnalb_create_message`'s own doc comment.
            apply_lab4_gnalb_create_message(&mut character, Some(&template.args));
        }
        if template.driver == CDR_LAB5SEYAN {
            // C never parses zone-file args for Laros (`zones/22/lab5.chr`'s
            // `lab5_seyan` template has no `arg=`), and the C driver has no
            // `NT_CREATE` handler either - no `NT_CREATE` push needed here,
            // same as `CDR_LAB4SEYAN` above.
            apply_lab5_seyan_create_message(&mut character);
        }
        if template.driver == CDR_LAB5DAEMON {
            // Data-only half of C's `NT_CREATE` handler (parses `type=`);
            // the ticker-dependent remainder (`attackstart`) runs on this
            // character's first live tick instead - see
            // `world::npc::area22::lab5_daemon::
            // apply_lab5_daemon_create_message`'s own doc comment.
            apply_lab5_daemon_create_message(&mut character, Some(&template.args));
        }
        if template.driver == CDR_LAB5MAGE {
            // C `create_char` generically fires `NT_CREATE`, and
            // `lab5_mage_driver` reads it (captures its own spawn tile
            // into `namecoordx[0]`/`namecoordy[0]`) - see
            // `world::npc::area22::lab5_mage::
            // apply_lab5_mage_create_message`'s own doc comment.
            apply_lab5_mage_create_message(&mut character);
        }
        if template.driver == crate::character_driver::CDR_PALACEGUARD {
            // C `palace_guard`'s `NT_CREATE` handler (`palace.c:152-163`):
            // parsed here at spawn time instead of through the per-tick
            // message loop, same precedent as `CDR_LAB2UNDEAD` above - see
            // `world::npc::area11::palace_guard`'s module doc comment.
            crate::world::apply_palace_guard_create_message(&mut character, Some(&template.args));
        }
        if template.driver == CDR_CAMHERMIT {
            // C never parses zone-file args into `struct
            // camhermit_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::Camhermit(
                CamhermitDriverData::default(),
            ));
        }
        if template.driver == CDR_YOAKIN {
            // C never parses zone-file args into `struct
            // yoakin_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Yoakin(YoakinDriverData::default()));
        }
        if template.driver == CDR_THOMAS {
            // C never parses zone-file args into `struct
            // thomas_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Thomas(ThomasDriverData::default()));
        }
        if template.driver == CDR_ASTRO2 {
            // C never parses zone-file args into `struct
            // astro2_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Astro2(Astro2DriverData::default()));
        }
        if template.driver == CDR_SIRJONES {
            // C never parses zone-file args into `struct
            // sir_jones_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::SirJones(SirJonesDriverData::default()));
        }
        if template.driver == CDR_SEYMOUR {
            // C never parses zone-file args into `struct
            // seymour_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Seymour(SeymourDriverData::default()));
        }
        if template.driver == CDR_KELLY {
            // C never parses zone-file args into `struct
            // kelly_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::Kelly(KellyDriverData::default()));
        }
        if template.driver == CDR_SMUGGLECOM {
            // C never parses zone-file args into `struct
            // smugglecom_data` (`set_data` zero-initializes it) - no args
            // to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::SmuggleCom(
                SmuggleComDriverData::default(),
            ));
        }
        if template.driver == CDR_ROUVEN {
            // C never parses zone-file args into `struct rouven_data`
            // (`set_data` zero-initializes it) - no args to read here,
            // same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Rouven(RouvenDriverData::default()));
        }
        if template.driver == CDR_ARISTOCRAT {
            // C never parses zone-file args into `struct aristocrat_data`
            // (`set_data` zero-initializes it) - no args to read here,
            // same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::Aristocrat(
                AristocratDriverData::default(),
            ));
        }
        if template.driver == CDR_YOATIN {
            // C never parses zone-file args into `struct yoatin_data`
            // (`set_data` zero-initializes it) - no args to read here,
            // same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Yoatin(YoatinDriverData::default()));
        }
        if template.driver == CDR_SPIRITBRAN {
            // C never parses zone-file args into `struct
            // spirit_brannington_data` (`set_data` zero-initializes it) -
            // no args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::SpiritBran(
                SpiritBranDriverData::default(),
            ));
        }
        if template.driver == CDR_GUARDBRAN {
            // C never parses zone-file args into `struct
            // guard_brannington_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::GuardBran(
                GuardBranDriverData::default(),
            ));
        }
        if template.driver == CDR_BRENNETHBRAN {
            // C never parses zone-file args into `struct
            // brenneth_brannington_data` (`set_data` zero-initializes it) -
            // no args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::BrennethBran(
                BrennethBranDriverData::default(),
            ));
        }
        if template.driver == CDR_BROKLIN {
            // C never parses zone-file args into `struct broklin_data`
            // (`set_data` zero-initializes it) - no args to read here, same
            // as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Broklin(BroklinDriverData::default()));
        }
        if template.driver == CDR_COUNTBRAN {
            // C never parses zone-file args into `struct
            // count_brannington_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::CountBran(
                CountBranDriverData::default(),
            ));
        }
        if template.driver == CDR_COUNTESSABRAN {
            // C never parses zone-file args into `struct
            // countessa_brannington_data` (`set_data` zero-initializes it) -
            // no args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::CountessaBran(
                CountessaBranDriverData::default(),
            ));
        }
        if template.driver == CDR_DAUGHTERBRAN {
            // C never parses zone-file args into `struct
            // daughter_brannington_data` (`set_data` zero-initializes it) -
            // no args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::DaughterBran(
                DaughterBranDriverData::default(),
            ));
        }
        if template.driver == CDR_FORESTBRAN {
            // C never parses zone-file args into `struct
            // forest_brannington_data` (`set_data` zero-initializes it) -
            // no args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::ForestBran(
                ForestBranDriverData::default(),
            ));
        }
        if template.driver == CDR_GRINNICH {
            // C never parses zone-file args into `struct grinnich_data`
            // (`set_data` zero-initializes it) - no args to read here, same
            // as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Grinnich(GrinnichDriverData::default()));
        }
        if template.driver == CDR_SHANRA {
            // C never parses zone-file args into `struct shanra_data`
            // (`set_data` zero-initializes it) - no args to read here, same
            // as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Shanra(ShanraDriverData::default()));
        }
        if template.driver == CDR_DWARFCHIEF {
            // C never parses zone-file args into `struct dwarfchief_data`
            // (`set_data` zero-initializes it) - no args to read here, same
            // as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::DwarfChief(
                DwarfChiefDriverData::default(),
            ));
        }
        if template.driver == CDR_LOSTDWARF {
            // C `lostdwarf_driver`'s `NT_CREATE` handler (`warrmines.c:932-
            // 936`): `dat->nr = atoi(ch[cn].arg);` - parsed here at spawn
            // time instead (see `world::npc::area31::lostdwarf`'s module
            // doc comment, same precedent as `CDR_SUPERIOR` above).
            let nr = template.args.trim().parse::<i32>().unwrap_or(0);
            character.driver_state = Some(CharacterDriverState::LostDwarf(LostDwarfDriverData {
                nr,
                ..Default::default()
            }));
        }
        if template.driver == CDR_DWARFSHAMAN {
            // C never parses zone-file args into `struct dwarfshaman_data`
            // (`set_data` zero-initializes it) - no args to read here, same
            // as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::DwarfShaman(
                DwarfShamanDriverData::default(),
            ));
        }
        if template.driver == CDR_DWARFSMITH {
            // C never parses zone-file args into `struct dwarfsmith_data`
            // (`set_data` zero-initializes it) - no args to read here, same
            // as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::DwarfSmith(
                DwarfSmithDriverData::default(),
            ));
        }
        if template.driver == CDR_MISSIONGIVE {
            // C never parses zone-file args into `struct mission_giver_
            // data` (`set_data` zero-initializes it) - no args to read
            // here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::MissionGiver(
                MissionGiverDriverData::default(),
            ));
        }
        if template.driver == CDR_TUNNELER_GORWIN {
            // C never parses zone-file args into `struct
            // gorwin_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above
            // (`ugaris_data/zones/33/tunnel.chr`'s `gorwin` template has
            // no `arg=` line at all).
            character.driver_state =
                Some(CharacterDriverState::Gorwin(GorwinDriverData::default()));
        }
        if template.driver == CDR_TEUFELQUEST {
            // C never parses zone-file args into `struct gamble_data`
            // for `teufelquest_driver` (`set_data` zero-initializes it) -
            // `ugaris_data/zones/34/teufel.chr`'s `arg="3"` on these
            // templates is only ever read by the sibling
            // `teufelgambler_driver`'s `NT_CREATE` handler, not this one.
            character.driver_state = Some(CharacterDriverState::TeufelQuest(
                TeufelQuestDriverData::default(),
            ));
        }
        if template.driver == CDR_TEUFELGAMBLER {
            // C `teufelgambler_driver`'s `NT_CREATE` handler
            // (`teufel.c:1248-1251`): `dat->nr = atoi(ch[cn].arg);` -
            // parsed here at spawn time instead, same precedent as
            // `CDR_LOSTDWARF` above (see `world::npc::area34::
            // teufelgambler`'s module doc comment). The `gambler`/
            // `gambler2`/`gambler3` templates (`zones/34/teufel.chr:750-
            // 946`) carry `arg="1"`/`"2"`/`"3"`.
            let nr = template.args.trim().parse::<i32>().unwrap_or(0);
            character.driver_state = Some(CharacterDriverState::TeufelGambler(
                TeufelGambleDriverData {
                    nr,
                    ..Default::default()
                },
            ));
        }
        if template.driver == CDR_WHITEROBBERBOSS {
            // C `ch_driver`'s `CDR_WHITEROBBERBOSS` dispatch
            // (`brannington_forest.c:684-686`): an unconditional every-tick
            // tail call to `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn,
            // ret, lastact)`, reusing the SimpleBaddy driver's full idle-
            // wander/auto-attack AI wholesale - same precedent as
            // `CDR_PENTER`/`CDR_SWAMPMONSTER`/`CDR_FORESTMONSTER`/
            // `CDR_TWOROBBER` above. The `Aston_Robber_Boss` template
            // (`zones/28/WS_Robbers.chr`) carries the same
            // `arg="aggressive=1;helper=1;scavenger=...;"` shape
            // SimpleBaddy's own `NT_CREATE` handler parses.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_CENTINEL {
            // C `ch_driver`'s `CDR_CENTINEL` dispatch (`brannington.c:2802-
            // 2804`): an unconditional every-tick tail call to
            // `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn, ret, lastact)`,
            // reusing the SimpleBaddy driver's full idle-wander/auto-attack
            // AI wholesale - same precedent as `CDR_WHITEROBBERBOSS` above.
            // The `centinel_count` template (`zones/29/wrtower.chr`) carries
            // the same `arg="aggressive=1;helper=1;scavenger=...;"` shape
            // SimpleBaddy's own `NT_CREATE` handler parses.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_SHR_WEREWOLF {
            // C `shr_werewolf_driver`'s night-time tail call to
            // `char_driver(CDR_SIMPLEBADDY, ...)` (`shrike.c:382`) reuses
            // the exact same `set_data(cn, DRD_SIMPLEBADDYDRIVER, ...)`
            // memory slot `simple_baddy_driver_parse` fills from
            // `ch[cn].arg` on `NT_CREATE` - parsed here at spawn time
            // instead, same precedent as `CDR_WHITEROBBERBOSS`/
            // `CDR_CENTINEL` above (`ugaris_data/zones/38/shrike.chr`'s
            // `werewolf` template carries the same
            // `arg="aggressive=1;helper=0;scavenger=20;..."` shape).
            // `World::process_shr_werewolf_actions`
            // (`world::npc::area38::werewolf`) decides, every tick,
            // whether to actually run this SimpleBaddy state or the
            // day-time invisible-walk-home behavior instead.
            character.push_driver_message(NT_CREATE, 0, 0, 0);
            apply_simple_baddy_create_message(&mut character, Some(&template.args), 0);
        }
        if template.driver == CDR_CARLOS {
            // C never parses zone-file args into `struct
            // carlos_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Carlos(CarlosDriverData::default()));
        }
        if template.driver == CDR_KASSIM {
            // C never parses zone-file args into `struct
            // kassim_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Kassim(KassimDriverData::default()));
        }
        if template.driver == CDR_SUPERMAX {
            // C never parses zone-file args into `struct
            // supermax_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Supermax(SupermaxDriverData::default()));
        }
        if template.driver == CDR_TERION {
            // C never parses zone-file args into `struct
            // terion_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Terion(TerionDriverData::default()));
        }
        if template.driver == CDR_GWENDYLON {
            // C never parses zone-file args into `struct
            // gwendylon_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::Gwendylon(
                GwendylonDriverData::default(),
            ));
        }
        if template.driver == CDR_GREETER {
            // C never parses zone-file args into `struct
            // greeter_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Greeter(GreeterDriverData::default()));
        }
        if template.driver == CDR_JESSICA {
            // C never parses zone-file args into `struct
            // jessica_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Jessica(JessicaDriverData::default()));
        }
        if template.driver == CDR_JIU {
            // C never parses zone-file args into `struct
            // jiu_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::Jiu(JiuDriverData::default()));
        }
        if template.driver == CDR_BRITHILDIE {
            // C never parses zone-file args into `struct
            // brithildie_driver_data` (`set_data` zero-initializes it) -
            // no args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::Brithildie(
                BrithildieDriverData::default(),
            ));
        }
        if template.driver == CDR_NOOK {
            // C never parses zone-file args into `struct
            // nook_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state = Some(CharacterDriverState::Nook(NookDriverData::default()));
        }
        if template.driver == CDR_RESKIN {
            // C never parses zone-file args into `struct
            // reskin_driver_data` (`set_data` zero-initializes it) - no
            // args to read here, same as `CDR_GATE_WELCOME` above.
            character.driver_state =
                Some(CharacterDriverState::Reskin(ReskinDriverData::default()));
        }
        if template.driver == CDR_FOREST_RANGER {
            // C never parses zone-file args into `struct
            // forest_ranger_driver_data` (`set_data` zero-initializes it) -
            // no args to read here, same as `CDR_GATE_WELCOME` above. The
            // `forest_ranger` template's own `arg="dir=3;"` is not read by
            // any C code (confirmed: no `"dir="` parser exists anywhere in
            // the C server source) - dead zone-file data, not a missed
            // port.
            character.driver_state = Some(CharacterDriverState::ForestRanger(
                ForestRangerDriverData::default(),
            ));
        }
        if template.driver == crate::character_driver::CDR_NOMAD {
            // C `nomad`'s `NT_CREATE` handler (`nomad.c:944-949`):
            // `nomad_parse(cn, dat)` parsed here at spawn time instead of
            // through the per-tick message loop, same precedent as
            // `CDR_TWOSERVANT`/`CDR_TWOGUARD` above - see `world::npc::
            // area19::nomad`'s module doc comment. Every `nomad1`-`nomad3`/
            // `nomad6`/`monk1`/`monk2` template (`zones/19/nomad.chr`)
            // carries an `arg="nr=N;diceskill=N;minbet=N;maxbet=N;
            // maxloss=N;"` line.
            character.driver_state = Some(CharacterDriverState::Nomad(
                crate::world::npc::area19::parse_nomad_driver_args(&template.args),
            ));
        }
        if template.driver == crate::character_driver::CDR_MADHERMIT {
            // C `madhermit_driver`'s `NT_CREATE` handler (`nomad.c:1189-
            // 1191`): `fight_driver_set_dist(cn, 30, 0, 60)`, a fixed,
            // template-independent seed - see `world::npc::area19::
            // madhermit`'s module doc comment for why this is seeded here
            // instead of round-tripping an `NT_CREATE` message.
            character.driver_state = Some(CharacterDriverState::Madhermit(
                crate::world::npc::area19::MadhermitDriverData,
            ));
            character.fight_driver = Some(FightDriverData {
                start_dist: 30,
                char_dist: 0,
                stop_dist: 60,
                ..Default::default()
            });
        }

        Ok((character, inventory_items))
    }

    fn add_lab2_undead_regenerate_spell(
        &mut self,
        character: &mut Character,
        inventory_items: &mut Vec<Item>,
    ) {
        let Some(CharacterDriverState::Lab2Undead(data)) = character.driver_state.as_ref() else {
            return;
        };
        if data.undead == 0 || data.regenerate_item_id.is_some() {
            return;
        }
        if character.inventory.iter().flatten().any(|item_id| {
            inventory_items
                .iter()
                .any(|item| item.id == *item_id && item.driver == IDR_LAB2_REGENERATE)
        }) {
            return;
        }

        let mut free_slot = None;
        for slot in INVENTORY_START_SPELLS..INVENTORY_START_INVENTORY {
            if character.inventory[slot].is_none() {
                free_slot = Some(slot);
            }
        }
        let Some(slot) = free_slot else {
            return;
        };
        let Ok(mut spell) = self.create_item("lab2_regenerate_spell", Some(character.id)) else {
            return;
        };
        if spell.driver_data.len() < 12 {
            spell.driver_data.resize(12, 0);
        }
        spell.driver_data[4..8].copy_from_slice(&character.id.0.to_le_bytes());
        let spell_id = spell.id;
        character.inventory[slot] = Some(spell_id);
        character
            .flags
            .insert(CharacterFlags::NODEATH | CharacterFlags::ITEMS);
        if let Some(CharacterDriverState::Lab2Undead(data)) = character.driver_state.as_mut() {
            data.regenerate_item_id = Some(spell_id);
        }
        inventory_items.push(spell);
    }
}
