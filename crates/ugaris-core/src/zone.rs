use std::collections::HashMap;

use thiserror::Error;

use crate::{
    character_driver::{
        apply_lab2_undead_create_message, apply_simple_baddy_create_message,
        parse_arena_manager_driver_args, parse_clanclerk_driver_args, parse_clanmaster_driver_args,
        parse_clubmaster_driver_args, ArenaFighterDriverData, ArenaMasterDriverData,
        Astro2DriverData, BrithildieDriverData, CamhermitDriverData, CarlosDriverData,
        CharacterDriverState, ClaraDriverData, DungeonmasterDriverData, ForestHermitDriverData,
        ForestImpDriverData, ForestRangerDriverData, ForestWilliamDriverData, GateFightDriverData,
        GateWelcomeDriverData, GolemKeyholdDriverData, GreeterDriverData, GwendylonDriverData,
        JanitorDriverData, JessicaDriverData, JiuDriverData, KassimDriverData, KellyDriverData,
        NookDriverData, ReskinDriverData, SeymourDriverData, SirJonesDriverData,
        SuperiorDriverData, SupermaxDriverData, TerionDriverData, ThomasDriverData,
        TraderDriverData, TwoAlchemistDriverData, TwoSanwynDriverData, TwoSkellyDriverData,
        YoakinDriverData, ARENA_FIGHTER_REST_POS, CDR_ARENAFIGHTER, CDR_ARENAMANAGER,
        CDR_ARENAMASTER, CDR_ASTRO2, CDR_BRITHILDIE, CDR_CAMHERMIT, CDR_CARLOS, CDR_CLANCLERK,
        CDR_CLANMASTER, CDR_CLUBMASTER, CDR_DUNGEONMASTER, CDR_FORESTHERMIT, CDR_FORESTIMP,
        CDR_FORESTMONSTER, CDR_FORESTWILLIAM, CDR_FOREST_RANGER, CDR_GATE_FIGHT, CDR_GATE_WELCOME,
        CDR_GOLEMKEYHOLDER, CDR_GREETER, CDR_GWENDYLON, CDR_JANITOR, CDR_JESSICA, CDR_JIU,
        CDR_KASSIM, CDR_KELLY, CDR_LAB2UNDEAD, CDR_NOOK, CDR_RESKIN, CDR_SEYMOUR, CDR_SIMPLEBADDY,
        CDR_SIRJONES, CDR_SUPERIOR, CDR_SUPERMAX, CDR_SWAMPCLARA, CDR_TERION, CDR_THOMAS,
        CDR_TRADER, CDR_TWOALCHEMIST, CDR_TWOSANWYN, CDR_TWOSKELLY, CDR_YOAKIN, NT_CREATE,
    },
    entity::{
        Character, CharacterFlags, Item, ItemFlags, CHARACTER_VALUE_COUNT, INVENTORY_SIZE,
        MAX_MODIFIERS, POWERSCALE, PROFESSION_COUNT,
    },
    ids::{CharacterId, ItemId},
    item_driver::IDR_LAB2_REGENERATE,
    legacy::{INVENTORY_START_INVENTORY, INVENTORY_START_SPELLS},
    map::{MapFlags, MapTile},
    world::World,
};

macro_rules! match_ascii_name {
    ($name:expr, $($literal:literal => $value:expr,)+) => {{
        let name = $name;
        let mut result = None;
        $(
            if name.eq_ignore_ascii_case($literal) {
                result = Some($value);
            }
        )+
        result
    }};
}

const LEGACY_DRIVER_DATA_SIZE: usize = 40;
const LEGACY_DIR_RIGHTDOWN: u8 = 2;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ZoneError {
    #[error("line {line}: {message}")]
    Syntax { line: usize, message: String },
    #[error("unknown item template `{0}`")]
    UnknownItem(String),
    #[error("unknown character template `{0}`")]
    UnknownCharacter(String),
    #[error("map coordinate ({x},{y}) is outside the current map")]
    MapOutOfBounds { x: usize, y: usize },
    #[error("character inventory is full")]
    InventoryFull,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneRecord {
    pub key: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTemplate {
    pub key: String,
    pub name: String,
    pub description: String,
    pub flags: ItemFlags,
    pub sprite: i32,
    pub value: u32,
    pub min_level: u8,
    pub max_level: u8,
    pub needs_class: u8,
    pub template_id: u32,
    pub modifier_index: [i16; MAX_MODIFIERS],
    pub modifier_value: [i16; MAX_MODIFIERS],
    pub driver: u16,
    pub driver_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterTemplate {
    pub key: String,
    pub name: String,
    pub description: String,
    pub flags: CharacterFlags,
    pub sprite: i32,
    pub sound: i32,
    pub gold: u32,
    pub driver: u16,
    pub group: i32,
    pub class: i32,
    pub respawn_seconds: Option<u32>,
    pub base_values: Vec<i16>,
    pub professions: Vec<i16>,
    pub inventory: Vec<Option<String>>,
    pub args: String,
    pub level_override: Option<u32>,
    pub loot_table: String,
    pub loot_table_death: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapDirective {
    Origin { x: usize, y: usize },
    Field { x: usize, y: usize },
    From { x: usize, y: usize },
    To { x: usize, y: usize },
    GroundSprite(u32),
    ForegroundSprite(u32),
    Character(String),
    Item(String),
    Flag(MapFlags),
}

#[derive(Debug, Default)]
pub struct ZoneLoader {
    pub item_templates: HashMap<String, ItemTemplate>,
    pub character_templates: HashMap<String, CharacterTemplate>,
    next_item_id: u32,
    next_character_id: u32,
    next_serial: u32,
}

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
        if template.driver == CDR_TWOALCHEMIST {
            // C never parses zone-file args into `struct
            // alchemist_driver_data` (`set_data` zero-initializes it) -
            // no `arg=` line exists for this template (`zones/17/
            // two.chr`'s `cervik`), same as `CDR_SWAMPCLARA` above.
            character.driver_state = Some(CharacterDriverState::TwoAlchemist(
                TwoAlchemistDriverData::default(),
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

fn tile_for_range_copy(mut tile: MapTile) -> MapTile {
    tile.item = 0;
    tile.character = 0;
    tile.flags.remove(
        MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::TSOUNDBLOCK | MapFlags::DOOR,
    );
    tile
}

fn apply_range_copy(tile: &mut MapTile, copied_tile: MapTile) {
    let item = tile.item;
    let character = tile.character;
    let dynamic_flags = tile.flags
        & (MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::TSOUNDBLOCK | MapFlags::DOOR);
    *tile = copied_tile;
    tile.item = item;
    tile.character = character;
    tile.flags.insert(dynamic_flags);
}

pub fn parse_zone_records(text: &str) -> Result<Vec<ZoneRecord>, ZoneError> {
    let mut parser = TokenParser::new(text);
    let mut records = Vec::new();
    let mut current: Option<ZoneRecord> = None;

    while let Some(token) = parser.next_token()? {
        match token {
            Token::Comment => {}
            Token::Start(key) => {
                if current.is_some() {
                    return Err(parser.error("need semicolon before next record"));
                }
                current = Some(ZoneRecord {
                    key,
                    fields: Vec::new(),
                });
            }
            Token::End => {
                let Some(record) = current.take() else {
                    return Err(parser.error("need record name before semicolon"));
                };
                records.push(record);
            }
            Token::Field(name, value) => {
                let Some(record) = current.as_mut() else {
                    return Err(parser.error("need record name before values"));
                };
                record.fields.push((name, value));
            }
        }
    }

    if current.is_some() {
        return Err(parser.error("premature end of input"));
    }

    Ok(records)
}

pub fn parse_map_directives(text: &str) -> Result<Vec<MapDirective>, ZoneError> {
    let mut parser = TokenParser::new(text);
    let mut directives = Vec::new();
    let mut origin_x = 0;
    let mut origin_y = 0;

    while let Some(token) = parser.next_token()? {
        let Token::Field(name, value) = token else {
            if matches!(token, Token::Comment) {
                continue;
            }
            return Err(parser.error("map files contain only name=value directives"));
        };

        if name.eq_ignore_ascii_case("origin") {
            let (x, y) = parse_pair(&value, &parser)?;
            origin_x = x;
            origin_y = y;
            directives.push(MapDirective::Origin { x, y });
        } else if name.eq_ignore_ascii_case("field") {
            let (x, y) = parse_pair(&value, &parser)?;
            directives.push(MapDirective::Field {
                x: x + origin_x,
                y: y + origin_y,
            });
        } else if name.eq_ignore_ascii_case("from") {
            let (x, y) = parse_pair(&value, &parser)?;
            directives.push(MapDirective::From {
                x: x + origin_x,
                y: y + origin_y,
            });
        } else if name.eq_ignore_ascii_case("to") {
            let (x, y) = parse_pair(&value, &parser)?;
            directives.push(MapDirective::To {
                x: x + origin_x,
                y: y + origin_y,
            });
        } else if name.eq_ignore_ascii_case("gsprite") {
            directives.push(MapDirective::GroundSprite(parse_sprite_u32(
                &value, &parser,
            )?));
        } else if name.eq_ignore_ascii_case("fsprite") {
            directives.push(MapDirective::ForegroundSprite(parse_sprite_u32(
                &value, &parser,
            )?));
        } else if name.eq_ignore_ascii_case("ch") {
            directives.push(MapDirective::Character(value));
        } else if name.eq_ignore_ascii_case("it") {
            directives.push(MapDirective::Item(value));
        } else if name.eq_ignore_ascii_case("flag") {
            directives
                .push(MapDirective::Flag(map_flag_by_name(&value).ok_or_else(
                    || parser.error(format!("unknown map flag `{value}`")),
                )?));
        } else {
            return Err(parser.error(format!("unknown map directive `{name}`")));
        }
    }

    Ok(directives)
}

fn item_template_from_record(record: ZoneRecord) -> Result<ItemTemplate, ZoneError> {
    let mut template = ItemTemplate {
        key: record.key.clone(),
        name: String::new(),
        description: String::new(),
        flags: ItemFlags::USED,
        sprite: 0,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        modifier_index: [0; MAX_MODIFIERS],
        modifier_value: [0; MAX_MODIFIERS],
        driver: 0,
        driver_data: Vec::new(),
    };
    let mut modifier_slot = 0;

    for (name, value) in record.fields {
        if name.eq_ignore_ascii_case("name") {
            template.name = value;
        } else if name.eq_ignore_ascii_case("description") {
            template.description = value;
        } else if name.eq_ignore_ascii_case("value") {
            template.value = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("sprite") {
            template.sprite = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("driver") {
            template.driver = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("min_level") {
            template.min_level = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("max_level") {
            template.max_level = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("needs_class") {
            template.needs_class = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("ID") {
            template.template_id = u32::from_str_radix(&value, 16).unwrap_or(0);
        } else if name.eq_ignore_ascii_case("arg") {
            template.driver_data = parse_hex_bytes(&value);
        } else if name.eq_ignore_ascii_case("mod_index") {
            template.modifier_index[modifier_slot] =
                value_index_by_name(&value).ok_or_else(|| ZoneError::Syntax {
                    line: 0,
                    message: format!("unknown character value `{value}`"),
                })? as i16;
        } else if name.eq_ignore_ascii_case("req_index") {
            template.modifier_index[modifier_slot] =
                -(value_index_by_name(&value).ok_or_else(|| ZoneError::Syntax {
                    line: 0,
                    message: format!("unknown character value `{value}`"),
                })? as i16);
        } else if name.eq_ignore_ascii_case("mod_value") || name.eq_ignore_ascii_case("req_value") {
            if modifier_slot >= MAX_MODIFIERS {
                return Err(ZoneError::Syntax {
                    line: 0,
                    message: "too many item modifiers".to_string(),
                });
            }
            template.modifier_value[modifier_slot] = value.parse().unwrap_or(0);
            modifier_slot += 1;
        } else if name.eq_ignore_ascii_case("flag") {
            let flag = item_flag_by_name(&value).ok_or_else(|| ZoneError::Syntax {
                line: 0,
                message: format!("unknown item flag `{value}`"),
            })?;
            template.flags.insert(flag);
        } else {
            return Err(ZoneError::Syntax {
                line: 0,
                message: format!("unknown item field `{name}`"),
            });
        }
    }

    if template.name.is_empty() {
        template.name = record.key;
    }
    template.driver_data.resize(LEGACY_DRIVER_DATA_SIZE, 0);
    Ok(template)
}

fn character_template_from_record(record: ZoneRecord) -> Result<CharacterTemplate, ZoneError> {
    let mut template = CharacterTemplate {
        key: record.key.clone(),
        name: String::new(),
        description: String::new(),
        flags: CharacterFlags::USED,
        sprite: 0,
        sound: 0,
        gold: 0,
        driver: 0,
        group: 0,
        class: 0,
        respawn_seconds: None,
        base_values: vec![0; CHARACTER_VALUE_COUNT],
        professions: vec![0; PROFESSION_COUNT],
        inventory: vec![None; INVENTORY_SIZE],
        args: String::new(),
        level_override: None,
        loot_table: String::new(),
        loot_table_death: String::new(),
    };
    let mut carry_slot = INVENTORY_START_INVENTORY;
    let mut spell_slot = INVENTORY_START_SPELLS;

    for (name, value) in record.fields {
        if name.eq_ignore_ascii_case("name") {
            template.name = value;
        } else if name.eq_ignore_ascii_case("description") {
            template.description = value;
        } else if name.eq_ignore_ascii_case("sprite") {
            template.sprite = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("sound") {
            template.sound = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("gold") {
            template.gold = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("driver") {
            template.driver = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("group") {
            template.group = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("class") {
            template.class = value.parse().unwrap_or(0);
        } else if name.eq_ignore_ascii_case("respawn") {
            template.respawn_seconds = Some(value.parse().unwrap_or(0));
        } else if name.eq_ignore_ascii_case("arg") {
            template.args.push_str(&value);
        } else if name.eq_ignore_ascii_case("item") {
            if carry_slot >= INVENTORY_SIZE {
                return Err(ZoneError::InventoryFull);
            }
            template.inventory[carry_slot] = Some(value);
            carry_slot += 1;
        } else if name.eq_ignore_ascii_case("spell") {
            if spell_slot >= INVENTORY_START_INVENTORY {
                return Err(ZoneError::InventoryFull);
            }
            template.inventory[spell_slot] = Some(value);
            spell_slot += 1;
        } else if name.eq_ignore_ascii_case("flag") {
            let flag = character_flag_by_name(&value).ok_or_else(|| ZoneError::Syntax {
                line: 0,
                message: format!("unknown character flag `{value}`"),
            })?;
            template.flags.insert(flag);
        } else if name.eq_ignore_ascii_case("LEVEL_OVERRIDE") {
            template.level_override = Some(value.parse().unwrap_or(0));
        } else if name.eq_ignore_ascii_case("loot_table") {
            template.loot_table = value;
        } else if name.eq_ignore_ascii_case("loot_table_death") {
            template.loot_table_death = value;
        } else if name.eq_ignore_ascii_case("rprob")
            || name.eq_ignore_ascii_case("ritem")
            || name.eq_ignore_ascii_case("special_prob")
            || name.eq_ignore_ascii_case("special_strength")
            || name.eq_ignore_ascii_case("special_base")
            || name.eq_ignore_ascii_case("gold_prob")
            || name.eq_ignore_ascii_case("gold_base")
            || name.eq_ignore_ascii_case("gold_random")
        {
            // Parsed legacy random/drop metadata is intentionally not materialized yet.
        } else if let Some(slot) = worn_slot_by_name(&name) {
            template.inventory[slot] = Some(value);
        } else if let Some(index) = value_index_by_name(&name) {
            template.base_values[index] = value.parse().unwrap_or(0);
        } else if let Some(index) = profession_index_by_name(&name) {
            template.professions[index] = value.parse().unwrap_or(0);
        } else {
            return Err(ZoneError::Syntax {
                line: 0,
                message: format!("unknown character field `{name}`"),
            });
        }
    }

    if template.name.is_empty() {
        template.name = record.key;
    }
    Ok(template)
}

fn place_character(
    world: &mut World,
    mut character: Character,
    inventory_items: Vec<Item>,
    x: usize,
    y: usize,
) -> Result<(), ZoneError> {
    let tile = world
        .map
        .tile_mut(x, y)
        .ok_or(ZoneError::MapOutOfBounds { x, y })?;
    character.x = x as u16;
    character.y = y as u16;
    tile.character = character.id.0 as u16;
    tile.flags.insert(MapFlags::TMOVEBLOCK);

    world.characters.insert(character.id, character);
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    Ok(())
}

fn place_item(world: &mut World, mut item: Item, x: usize, y: usize) -> Result<(), ZoneError> {
    let tile = world
        .map
        .tile_mut(x, y)
        .ok_or(ZoneError::MapOutOfBounds { x, y })?;
    item.x = x as u16;
    item.y = y as u16;
    tile.item = item.id.0;
    if item.flags.contains(ItemFlags::MOVEBLOCK) {
        tile.flags.insert(MapFlags::TMOVEBLOCK);
    }
    if item.flags.contains(ItemFlags::SIGHTBLOCK) {
        tile.flags.insert(MapFlags::TSIGHTBLOCK);
    }
    if item.flags.contains(ItemFlags::DOOR) {
        tile.flags.insert(MapFlags::DOOR);
    }
    if item.flags.contains(ItemFlags::SOUNDBLOCK) {
        tile.flags.insert(MapFlags::TSOUNDBLOCK);
    }
    world.items.insert(item.id, item);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Comment,
    Start(String),
    End,
    Field(String, String),
}

struct TokenParser<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    _text: &'a str,
}

impl<'a> TokenParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            chars: text.chars().collect(),
            pos: 0,
            line: 1,
            _text: text,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, ZoneError> {
        self.skip_whitespace();
        let Some(ch) = self.peek() else {
            return Ok(None);
        };

        if ch == ';' {
            self.bump();
            return Ok(Some(Token::End));
        }
        if ch == '#' {
            self.skip_line_comment();
            return Ok(Some(Token::Comment));
        }
        if ch == '/' && self.peek_next() == Some('/') {
            self.skip_line_comment();
            return Ok(Some(Token::Comment));
        }

        let name = self.read_word();
        if name.is_empty() {
            return Err(self.error(format!("unexpected character `{ch}`")));
        }
        self.skip_whitespace();

        match self.peek() {
            Some(':') => {
                self.bump();
                Ok(Some(Token::Start(name)))
            }
            Some('=') => {
                self.bump();
                self.skip_whitespace();
                let value = if self.peek() == Some('"') {
                    self.bump();
                    let mut value = String::new();
                    while let Some(ch) = self.peek() {
                        if ch == '"' {
                            self.bump();
                            return Ok(Some(Token::Field(name, value)));
                        }
                        value.push(ch);
                        self.bump();
                    }
                    return Err(self.error("unterminated quoted value"));
                } else {
                    self.read_word()
                };
                Ok(Some(Token::Field(name, value)))
            }
            _ => Err(self.error(format!("expected `:` or `=` after `{name}`"))),
        }
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    fn read_word(&mut self) -> String {
        let mut word = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                word.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        word
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            self.bump();
            if ch == '\n' || ch == '\r' {
                break;
            }
        }
    }

    fn bump(&mut self) {
        if self.peek() == Some('\n') {
            self.line += 1;
        }
        self.pos += 1;
    }

    fn error(&self, message: impl Into<String>) -> ZoneError {
        ZoneError::Syntax {
            line: self.line,
            message: message.into(),
        }
    }
}

fn parse_pair(value: &str, parser: &TokenParser<'_>) -> Result<(usize, usize), ZoneError> {
    let Some((left, right)) = value.split_once(',') else {
        return Err(parser.error("expected two comma-separated values"));
    };
    let x = left
        .trim()
        .parse()
        .map_err(|_| parser.error(format!("invalid coordinate `{left}`")))?;
    let y = right
        .trim()
        .parse()
        .map_err(|_| parser.error(format!("invalid coordinate `{right}`")))?;
    Ok((x, y))
}

fn parse_sprite_u32(value: &str, parser: &TokenParser<'_>) -> Result<u32, ZoneError> {
    if let Ok(value) = value.parse::<u32>() {
        return Ok(value);
    }
    let value = value
        .parse::<i32>()
        .map_err(|_| parser.error(format!("invalid integer `{value}`")))?;
    Ok(value as u32)
}

fn parse_hex_bytes(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .take(LEGACY_DRIVER_DATA_SIZE)
        .map(|chunk| {
            let high = (chunk[0] as char).to_digit(16).unwrap_or(0);
            let low = (chunk[1] as char).to_digit(16).unwrap_or(0);
            ((high << 4) | low) as u8
        })
        .collect()
}

fn value_index_by_name(name: &str) -> Option<usize> {
    const NAMES: [&str; CHARACTER_VALUE_COUNT] = [
        "V_HP",
        "V_ENDURANCE",
        "V_MANA",
        "V_WIS",
        "V_INT",
        "V_AGI",
        "V_STR",
        "V_ARMOR",
        "V_WEAPON",
        "V_LIGHT",
        "V_SPEED",
        "V_PULSE",
        "V_DAGGER",
        "V_HAND",
        "V_STAFF",
        "V_SWORD",
        "V_TWOHAND",
        "V_ARMORSKILL",
        "V_ATTACK",
        "V_PARRY",
        "V_WARCRY",
        "V_TACTICS",
        "V_SURROUND",
        "V_BODYCONTROL",
        "V_SPEEDSKILL",
        "V_BARTER",
        "V_PERCEPT",
        "V_STEALTH",
        "V_BLESS",
        "V_HEAL",
        "V_FREEZE",
        "V_MAGICSHIELD",
        "V_FLASH",
        "V_FIREBALL",
        "V_EMPTY",
        "V_REGENERATE",
        "V_MEDITATE",
        "V_IMMUNITY",
        "V_DEMON",
        "V_DURATION",
        "V_RAGE",
        "V_COLD",
        "V_PROFESSION",
    ];
    if name.eq_ignore_ascii_case("V_ARCANE") {
        return Some(34);
    }
    NAMES
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn profession_index_by_name(name: &str) -> Option<usize> {
    const NAMES: [&str; PROFESSION_COUNT] = [
        "P_ATHLETE",
        "P_ALCHEMIST",
        "P_MINER",
        "P_ASSASSIN",
        "P_THIEF",
        "P_LIGHT",
        "P_DARK",
        "P_TRADER",
        "P_MERCENARY",
        "P_CLAN",
        "P_HERBALIST",
        "P_DEMON",
        "P_NULL",
        "P_NULL",
        "P_NULL",
        "P_NULL",
        "P_NULL",
        "P_NULL",
        "P_NULL",
        "P_NULL",
    ];
    NAMES
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn worn_slot_by_name(name: &str) -> Option<usize> {
    const NAMES: [&str; 12] = [
        "WN_NECK", "WN_HEAD", "WN_CLOAK", "WN_ARMS", "WN_BODY", "WN_BELT", "WN_RHAND", "WN_LEGS",
        "WN_LHAND", "WN_RRING", "WN_FEET", "WN_LRING",
    ];
    NAMES
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn item_flag_by_name(name: &str) -> Option<ItemFlags> {
    Some(match_ascii_name!(name,
        "IF_USED" => ItemFlags::USED,
        "IF_MOVEBLOCK" => ItemFlags::MOVEBLOCK,
        "IF_SIGHTBLOCK" => ItemFlags::SIGHTBLOCK,
        "IF_TAKE" => ItemFlags::TAKE,
        "IF_USE" => ItemFlags::USE,
        "IF_WNHEAD" => ItemFlags::WNHEAD,
        "IF_WNNECK" => ItemFlags::WNNECK,
        "IF_WNBODY" => ItemFlags::WNBODY,
        "IF_WNARMS" => ItemFlags::WNARMS,
        "IF_WNBELT" => ItemFlags::WNBELT,
        "IF_WNLEGS" => ItemFlags::WNLEGS,
        "IF_WNFEET" => ItemFlags::WNFEET,
        "IF_WNLHAND" => ItemFlags::WNLHAND,
        "IF_WNRHAND" => ItemFlags::WNRHAND,
        "IF_WNCLOAK" => ItemFlags::WNCLOAK,
        "IF_WNLRING" => ItemFlags::WNLRING,
        "IF_WNRRING" => ItemFlags::WNRRING,
        "IF_WNTWOHANDED" => ItemFlags::WNTWOHANDED,
        "IF_AXE" => ItemFlags::AXE,
        "IF_DAGGER" => ItemFlags::DAGGER,
        "IF_HAND" => ItemFlags::HAND,
        "IF_SHANON" => ItemFlags::SHIELD,
        "IF_SHIELD" => ItemFlags::SHIELD,
        "IF_STAFF" => ItemFlags::STAFF,
        "IF_SWORD" => ItemFlags::SWORD,
        "IF_TWOHAND" => ItemFlags::TWOHAND,
        "IF_DOOR" => ItemFlags::DOOR,
        "IF_QUEST" => ItemFlags::QUEST,
        "IF_SOUNDBLOCK" => ItemFlags::SOUNDBLOCK,
        "IF_STEPACTION" => ItemFlags::STEPACTION,
        "IF_MONEY" => ItemFlags::MONEY,
        "IF_NODECAY" => ItemFlags::NODECAY,
        "IF_FRONTWALL" => ItemFlags::FRONTWALL,
        "IF_DEPOT" => ItemFlags::DEPOT,
        "IF_NODEPOT" => ItemFlags::NODEPOT,
        "IF_NODROP" => ItemFlags::NODROP,
        "IF_NOJUNK" => ItemFlags::NOJUNK,
        "IF_PLAYERBODY" => ItemFlags::PLAYERBODY,
        "IF_BONDTAKE" => ItemFlags::BONDTAKE,
        "IF_BONDWEAR" => ItemFlags::BONDWEAR,
        "IF_LABITEM" => ItemFlags::LABITEM,
        "IF_VOID" => ItemFlags::VOID,
        "IF_NOENHANCE" => ItemFlags::NOENHANCE,
        "IF_BEYONDBOUNDS" => ItemFlags::BEYONDBOUNDS,
        "IF_BEYONDMAXMOD" => ItemFlags::BEYONDMAXMOD,
        "IF_ENGRAVED" => ItemFlags::ENGRAVED,
        "IF_GIVEN_ITEM" => ItemFlags::GIVEN_ITEM,
        "IF_FORCEUPDATE" => ItemFlags::FORCEUPDATE,
    )?)
}

fn character_flag_by_name(name: &str) -> Option<CharacterFlags> {
    Some(match_ascii_name!(name,
        "CF_USED" => CharacterFlags::USED,
        "CF_IMMORTAL" => CharacterFlags::IMMORTAL,
        "CF_GOD" => CharacterFlags::GOD,
        "CF_PLAYER" => CharacterFlags::PLAYER,
        "CF_STAFF" => CharacterFlags::STAFF,
        "CF_INVISIBLE" => CharacterFlags::INVISIBLE,
        "CF_SHUTUP" => CharacterFlags::SHUTUP,
        "CF_KICKED" => CharacterFlags::KICKED,
        "CF_UPDATE" => CharacterFlags::UPDATE,
        "CF_DEAD" => CharacterFlags::DEAD,
        "CF_ITEMS" => CharacterFlags::ITEMS,
        "CF_RESPAWN" => CharacterFlags::RESPAWN,
        "CF_MALE" => CharacterFlags::MALE,
        "CF_FEMALE" => CharacterFlags::FEMALE,
        "CF_WARRIOR" => CharacterFlags::WARRIOR,
        "CF_MAGE" => CharacterFlags::MAGE,
        "CF_ARCH" => CharacterFlags::ARCH,
        "CF_NOATTACK" => CharacterFlags::NOATTACK,
        "CF_HASNAME" => CharacterFlags::HASNAME,
        "CF_QUESTITEM" => CharacterFlags::QUESTITEM,
        "CF_INFRARED" => CharacterFlags::INFRARED,
        "CF_PK" => CharacterFlags::PK,
        "CF_ITEMDEATH" => CharacterFlags::ITEMDEATH,
        "CF_NODEATH" => CharacterFlags::NODEATH,
        "CF_NOBODY" => CharacterFlags::NOBODY,
        "CF_EDEMON" => CharacterFlags::EDEMON,
        "CF_FDEMON" => CharacterFlags::FDEMON,
        "CF_IDEMON" => CharacterFlags::IDEMON,
        "CF_NOGIVE" => CharacterFlags::NOGIVE,
        "CF_PLAYERLIKE" => CharacterFlags::PLAYERLIKE,
        "CF_PAID" => CharacterFlags::PAID,
        "CF_PROF" => CharacterFlags::PROF,
        "CF_ALIVE" => CharacterFlags::ALIVE,
        "CF_DEMON" => CharacterFlags::DEMON,
        "CF_UNDEAD" => CharacterFlags::UNDEAD,
        "CF_HARDKILL" => CharacterFlags::HARDKILL,
        "CF_NOBLESS" => CharacterFlags::NOBLESS,
        "CF_AREACHANGE" => CharacterFlags::AREACHANGE,
        "CF_LAG" => CharacterFlags::LAG,
        "CF_THIEFMODE" => CharacterFlags::THIEFMODE,
        "CF_NOTELL" => CharacterFlags::NOTELL,
        "CF_INFRAVISION" => CharacterFlags::INFRAVISION,
        "CF_NOMAGIC" => CharacterFlags::NOMAGIC,
        "CF_NONOMAGIC" => CharacterFlags::NONOMAGIC,
        "CF_OXYGEN" => CharacterFlags::OXYGEN,
        "CF_NOPLRATT" => CharacterFlags::NOPLRATT,
        "CF_ALLOWSWAP" => CharacterFlags::ALLOWSWAP,
        "CF_LQMASTER" => CharacterFlags::LQMASTER,
        "CF_HARDCORE" => CharacterFlags::HARDCORE,
        "CF_NONOTIFY" => CharacterFlags::NONOTIFY,
        "CF_SMALLUPDATE" => CharacterFlags::SMALLUPDATE,
        "CF_NOWHO" => CharacterFlags::NOWHO,
        "CF_WON" => CharacterFlags::WON,
        "CF_NOEXP" => CharacterFlags::NOEXP,
        "CF_DEVELOPER" => CharacterFlags::DEVELOPER,
        "CF_EVENTMASTER" => CharacterFlags::EVENTMASTER,
        "CF_XRAY" => CharacterFlags::XRAY,
        "CF_NOLEVEL" => CharacterFlags::NOLEVEL,
        "CF_SPY" => CharacterFlags::SPY,
    )?)
}

fn map_flag_by_name(name: &str) -> Option<MapFlags> {
    Some(match_ascii_name!(name,
        "MF_MOVEBLOCK" => MapFlags::MOVEBLOCK,
        "MF_SIGHTBLOCK" => MapFlags::SIGHTBLOCK,
        "MF_TMOVEBLOCK" => MapFlags::TMOVEBLOCK,
        "MF_TSIGHTBLOCK" => MapFlags::TSIGHTBLOCK,
        "MF_INDOORS" => MapFlags::INDOORS,
        "MF_RESTAREA" => MapFlags::RESTAREA,
        "MF_DOOR" => MapFlags::DOOR,
        "MF_SOUNDBLOCK" => MapFlags::SOUNDBLOCK,
        "MF_TSOUNDBLOCK" => MapFlags::TSOUNDBLOCK,
        "MF_SHOUTBLOCK" => MapFlags::SHOUTBLOCK,
        "MF_CLAN" => MapFlags::CLAN,
        "MF_ARENA" => MapFlags::ARENA,
        "MF_PEACE" => MapFlags::PEACE,
        "MF_NEUTRAL" => MapFlags::NEUTRAL,
        "MF_FIRETHRU" => MapFlags::FIRETHRU,
        "MF_SLOWDEATH" => MapFlags::SLOWDEATH,
        "MF_NOLIGHT" => MapFlags::NOLIGHT,
        "MF_NOMAGIC" => MapFlags::NOMAGIC,
        "MF_UNDERWATER" => MapFlags::UNDERWATER,
        "MF_NOREGEN" => MapFlags::NOREGEN,
        "MF_SINK_ANKLE" => MapFlags::SINK_ANKLE,
        "MF_SINK_KNEE" => MapFlags::SINK_KNEE,
        "MF_SINK_BELLY" => MapFlags::SINK_BELLY,
        "MF_SINK_CHEST" => MapFlags::SINK_CHEST,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapGrid;

    #[test]
    fn parses_legacy_record_syntax() {
        let records = parse_zone_records(
            r#"
            # comments are ignored
            // C++ style comments are also ignored
            Torch:
              name="Training Torch"
              flag=IF_TAKE
              flag=IF_MOVEBLOCK
              mod_index=V_LIGHT
              mod_value=5
              ID=1A
            ;
            "#,
        )
        .unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].key, "Torch");
        assert!(records[0]
            .fields
            .contains(&("flag".to_string(), "IF_MOVEBLOCK".to_string())));
    }

    #[test]
    fn parses_map_directives_with_origin_offsets() {
        let directives = parse_map_directives(
            r#"
            origin="10,20"
            field="1,2"
            gsprite=100
            from="3,4"
            to="4,4"
            flag=MF_INDOORS
            "#,
        )
        .unwrap();

        assert!(directives.contains(&MapDirective::Field { x: 11, y: 22 }));
        assert!(directives.contains(&MapDirective::From { x: 13, y: 24 }));
        assert!(directives.contains(&MapDirective::To { x: 14, y: 24 }));
        assert!(directives.contains(&MapDirective::Flag(MapFlags::INDOORS)));
    }

    #[test]
    fn parses_negative_legacy_sprite_values_as_u32_bits() {
        let directives = parse_map_directives(
            r#"
            field="1,1"
            gsprite=-420589820
            fsprite=-1
            "#,
        )
        .unwrap();

        assert!(directives.contains(&MapDirective::GroundSprite((-420589820_i32) as u32)));
        assert!(directives.contains(&MapDirective::ForegroundSprite(u32::MAX)));
    }

    #[test]
    fn map_application_keeps_terrain_when_item_template_is_missing() {
        let mut loader = ZoneLoader::new();
        let mut world = World::default();

        loader
            .apply_map_str(
                &mut world,
                r#"
                field="5,6"
                gsprite=123
                it=missing_item_template
                "#,
            )
            .unwrap();

        let tile = world.map.tile(5, 6).unwrap();
        assert_eq!(tile.ground_sprite, 123);
        assert_eq!(tile.item, 0);
    }

    #[test]
    fn range_copy_does_not_duplicate_dynamic_item_or_character_ids() {
        let items = r#"
            Door:
              name="Door"
              sprite=42
              flag=IF_MOVEBLOCK
              flag=IF_SIGHTBLOCK
              flag=IF_DOOR
            ;
        "#;
        let chars = r#"
            Guard:
              name="Guard"
              V_HP=10
            ;
        "#;
        let map = r#"
            field="5,5"
            gsprite=100
            it=Door
            ch=Guard
            from="5,5"
            to="6,5"
        "#;

        let mut loader = ZoneLoader::new();
        loader.load_item_templates_str(items).unwrap();
        loader.load_character_templates_str(chars).unwrap();
        let mut world = World::default();
        loader.apply_map_str(&mut world, map).unwrap();

        let original = world.map.tile(5, 5).unwrap();
        assert_eq!(original.item, 1);
        assert_eq!(original.character, 1);

        let copied = world.map.tile(6, 5).unwrap();
        assert_eq!(copied.ground_sprite, 100);
        assert_eq!(copied.item, 0);
        assert_eq!(copied.character, 0);
        assert!(!copied
            .flags
            .intersects(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR));
    }

    #[test]
    fn applies_tiny_zone_to_world() {
        let items = r#"
            Torch:
              name="Training Torch"
              sprite=42
              flag=IF_MOVEBLOCK
              flag=IF_SIGHTBLOCK
              mod_index=V_LIGHT
              mod_value=7
            ;
        "#;
        let chars = r#"
            Guard:
              name="Practice Guard"
              flag=CF_RESPAWN
              flag=CF_NOBODY
              driver=7
              arg="aggressive=1; startdist=8; drinkinvpots=1;"
              V_HP=10
              P_ATHLETE=3
              WN_RHAND=Torch
              item=Torch
              spell=Torch
            ;
        "#;
        let map = r#"
            origin="10,20"
            field="1,2"
            gsprite=100
            fsprite=101
            flag=MF_INDOORS
            it=Torch
            ch=Guard
        "#;

        let mut loader = ZoneLoader::new();
        loader.load_item_templates_str(items).unwrap();
        loader.load_character_templates_str(chars).unwrap();

        let mut world = World::default();
        world.map = MapGrid::new(32, 32);
        loader.apply_map_str(&mut world, map).unwrap();

        let tile = world.map.tile(11, 22).unwrap();
        assert_eq!(tile.ground_sprite, 100);
        assert_eq!(tile.foreground_sprite, 101);
        assert_eq!(tile.item, 1);
        assert_eq!(tile.character, 1);
        assert!(tile.flags.contains(MapFlags::INDOORS));
        assert!(tile.flags.contains(MapFlags::TMOVEBLOCK));
        assert!(tile.flags.contains(MapFlags::TSIGHTBLOCK));

        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.name, "Practice Guard");
        assert_eq!(character.x, 11);
        assert_eq!(character.y, 22);
        assert_eq!(character.values[1][0], 10);
        assert_eq!(character.professions[0], 3);
        assert_eq!(character.driver, CDR_SIMPLEBADDY);
        assert_eq!(character.inventory[6], Some(ItemId(2)));
        assert_eq!(character.inventory[12], Some(ItemId(3)));
        assert_eq!(character.inventory[30], Some(ItemId(4)));
        assert!(!character.flags.contains(CharacterFlags::NOBODY));
        assert!(character.flags.contains(CharacterFlags::ITEMDEATH));
        assert!(character.driver_messages.is_empty());
        let Some(crate::character_driver::CharacterDriverState::SimpleBaddy(data)) =
            &character.driver_state
        else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.aggressive, 1);
        assert_eq!(data.startdist, 8);
        assert_eq!(data.drink_inventory_potions, 1);

        assert_eq!(world.items.get(&ItemId(1)).unwrap().x, 11);
        assert_eq!(
            world.items.get(&ItemId(2)).unwrap().carried_by,
            Some(CharacterId(1))
        );
    }

    #[test]
    fn lab2_undead_template_installs_regenerate_spell_for_undead() {
        let items = r#"
            lab2_regenerate_spell:
              name="lab2_regenerate_spell"
              driver=194
              arg="180800000000000000000000"
            ;
        "#;
        let chars = r#"
            Undead:
              name="Lab Undead"
              driver=198
              arg="undead=1; patrol=1;"
              V_HP=10
            ;
        "#;

        let mut loader = ZoneLoader::new();
        loader.load_item_templates_str(items).unwrap();
        loader.load_character_templates_str(chars).unwrap();

        let (character, inventory_items) = loader
            .instantiate_character_template("Undead", CharacterId(7))
            .unwrap();

        assert!(character.flags.contains(CharacterFlags::NODEATH));
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        let Some(CharacterDriverState::Lab2Undead(data)) = &character.driver_state else {
            panic!("lab2 undead state missing");
        };
        let regen_item_id = data.regenerate_item_id.expect("regenerate item id");
        assert_eq!(character.inventory[29], Some(regen_item_id));

        let regen = inventory_items
            .iter()
            .find(|item| item.id == regen_item_id)
            .expect("regenerate item");
        assert_eq!(regen.driver, IDR_LAB2_REGENERATE);
        assert_eq!(regen.carried_by, Some(CharacterId(7)));
        assert_eq!(&regen.driver_data[4..8], &7_u32.to_le_bytes());
        assert_eq!(data.undead, 1);
        assert_eq!(data.patstep, 4);
    }

    #[test]
    fn dungeonfighter_template_installs_simple_baddy_state_from_arg_and_own_data_field() {
        // Mirrors `zones/13/dungeon.chr`'s real "warrior"/"mage"/"seyan"
        // entries: `driver=52` (`CDR_DUNGEONFIGHTER`) with a SimpleBaddy-
        // style `arg=` string that `dungeonfighter` itself never reads but
        // its own tail `char_driver(CDR_SIMPLEBADDY, ...)` call does.
        let chars = r#"
            warrior:
              name="Warrior"
              driver=52
              arg="aggressive=1;helper=0;scavenger=0;startdist=40;chardist=0;stopdist=80;"
              V_HP=10
            ;
        "#;

        let mut loader = ZoneLoader::new();
        loader.load_character_templates_str(chars).unwrap();

        let (character, _inventory_items) = loader
            .instantiate_character_template("warrior", CharacterId(9))
            .unwrap();

        assert_eq!(
            character.driver,
            crate::character_driver::CDR_DUNGEONFIGHTER
        );
        assert!(character.driver_messages.is_empty());
        assert!(character.dungeonfighter.is_some());
        let Some(CharacterDriverState::SimpleBaddy(data)) = &character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.aggressive, 1);
        assert_eq!(data.startdist, 40);
        assert_eq!(data.stopdist, 80);
    }

    #[test]
    fn swampmonster_template_installs_simple_baddy_state_from_arg() {
        // Mirrors `zones/15/swamp.chr`'s real `swamp25n`/`swamp27n`/
        // `swamp29n`/`swamp31n` entries: `driver=56` (`CDR_SWAMPMONSTER`)
        // with a SimpleBaddy-style `arg=` string that `swamp_monster`'s
        // own tail `char_driver(CDR_SIMPLEBADDY, ...)` call reads.
        let chars = r#"
            swamp25n:
              name="Swamp Beastling"
              driver=56
              arg="aggressive=1;helper=0;scavenger=0;startdist=40;chardist=0;stopdist=60;"
              V_HP=12
            ;
        "#;

        let mut loader = ZoneLoader::new();
        loader.load_character_templates_str(chars).unwrap();

        let (character, _inventory_items) = loader
            .instantiate_character_template("swamp25n", CharacterId(9))
            .unwrap();

        assert_eq!(character.driver, crate::character_driver::CDR_SWAMPMONSTER);
        assert!(character.driver_messages.is_empty());
        let Some(CharacterDriverState::SimpleBaddy(data)) = &character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.aggressive, 1);
        assert_eq!(data.startdist, 40);
        assert_eq!(data.stopdist, 60);
    }

    #[test]
    fn clara_template_installs_default_clara_driver_state() {
        // C never parses zone-file args into `struct clara_driver_data`
        // (`set_data` zero-initializes it) - no args to read for
        // `CDR_SWAMPCLARA` (`driver=54`).
        let chars = r#"
            clara:
              name="Clara"
              driver=54
              V_HP=100
            ;
        "#;

        let mut loader = ZoneLoader::new();
        loader.load_character_templates_str(chars).unwrap();

        let (character, _inventory_items) = loader
            .instantiate_character_template("clara", CharacterId(9))
            .unwrap();

        assert_eq!(character.driver, crate::character_driver::CDR_SWAMPCLARA);
        assert_eq!(
            character.driver_state,
            Some(CharacterDriverState::Clara(ClaraDriverData::default()))
        );
    }

    #[test]
    fn fdemon_big1_sprite_190_spawns_as_plain_simplebaddy() {
        // Mirrors `zones/8/fire.chr`'s real `fdemon_big1` entry: `driver=46`
        // (`CDR_FDEMON_DEMON`) but `sprite=190` - C's own `fdemon_demon`
        // unconditionally tail-calls `char_driver(CDR_SIMPLEBADDY, ...)`
        // for this sprite every tick, so this port assigns
        // `CDR_SIMPLEBADDY` directly at spawn (see the `CDR_FDEMON_DEMON`
        // branch above).
        let chars = r#"
            fdemon_big1:
              name="Fire Golem"
              driver=46
              sprite=190
              arg="aggressive=1;helper=0;scavenger=0;startdist=20;chardist=0;stopdist=40;"
              V_HP=35
            ;
        "#;

        let mut loader = ZoneLoader::new();
        loader.load_character_templates_str(chars).unwrap();

        let (character, _inventory_items) = loader
            .instantiate_character_template("fdemon_big1", CharacterId(10))
            .unwrap();

        assert_eq!(character.driver, CDR_SIMPLEBADDY);
        assert!(character.driver_messages.is_empty());
        let Some(CharacterDriverState::SimpleBaddy(data)) = &character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.aggressive, 1);
        assert_eq!(data.startdist, 20);
        assert_eq!(data.stopdist, 40);
    }

    #[test]
    fn fdemon_trash_mob_installs_fixed_distances_ignoring_zone_file_args() {
        // Mirrors `zones/8/fire.chr`'s real `fdemon1s` entry: `driver=46`,
        // `sprite=157` (not 190), with its `arg=` commented out in the real
        // data since C's own `fdemon_demon` `NT_CREATE` handler never
        // parses `ch[cn].arg` and hardcodes `fight_driver_set_dist(cn, 0,
        // 30, 0)` plus `standard_message_driver(cn, msg, 1, 1)` instead.
        let chars = r#"
            fdemon1s:
              name="Fire Demon"
              driver=46
              sprite=157
              arg="aggressive=0;helper=0;scavenger=20;startdist=99;chardist=99;stopdist=99;"
              V_HP=35
            ;
        "#;

        let mut loader = ZoneLoader::new();
        loader.load_character_templates_str(chars).unwrap();

        let (character, _inventory_items) = loader
            .instantiate_character_template("fdemon1s", CharacterId(11))
            .unwrap();

        assert_eq!(character.driver, crate::character_driver::CDR_FDEMON_DEMON);
        assert!(character.driver_messages.is_empty());
        let fight_driver = character.fight_driver.expect("fight driver data");
        assert_eq!(fight_driver.start_dist, 0);
        assert_eq!(fight_driver.char_dist, 30);
        assert_eq!(fight_driver.stop_dist, 0);
        let Some(CharacterDriverState::SimpleBaddy(data)) = &character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.aggressive, 1);
        assert_eq!(data.helper, 1);
        // The zone-file `arg=` string above must be ignored entirely.
        assert_ne!(data.scavenger, 20);
    }

    #[test]
    fn zone_population_rolls_spawn_mode_loot_table_into_new_npcs_own_inventory() {
        // C `create.c:1121-1125`: `if (ch_temp[ctmp].loot_table[0])
        // loot_apply_to_npc(n, ch_temp[ctmp].loot_table);`, run for every
        // NPC `pop_create_char` places while loading a zone's map.
        let items = r#"
            bronzechip:
              name="Bronze Chip"
              flag=IF_TAKE
            ;
        "#;
        let chars = r#"
            Guard:
              name="Practice Guard"
              loot_table="guard_spawn_loot"
              V_HP=10
            ;
        "#;
        let map = r#"
            field="5,5"
            ch=Guard
        "#;

        let mut loader = ZoneLoader::new();
        loader.load_item_templates_str(items).unwrap();
        loader.load_character_templates_str(chars).unwrap();
        let mut world = World::default();
        world.loot_registry.load_str(
            r#"{
                "id": "guard_spawn_loot",
                "rolls": 1,
                "entries": [{"weight": 1, "item": "bronzechip"}]
            }"#,
        );
        world.legacy_random_seed = 0;
        loader.apply_map_str(&mut world, map).unwrap();

        let npc = world
            .characters
            .get(&CharacterId(1))
            .expect("guard spawned");
        assert_eq!(npc.name, "Practice Guard");
        // Slots 0-29 (worn/spells) stay empty; the rolled item lands at the
        // first free carried slot (30).
        assert!(npc.inventory[..30].iter().all(Option::is_none));
        let carried_id = npc.inventory[30].expect("loot item placed at slot 30");
        let carried_item = world.items.get(&carried_id).expect("item exists");
        assert_eq!(carried_item.name, "Bronze Chip");
        assert_eq!(carried_item.carried_by, Some(CharacterId(1)));
    }
}
