use super::*;

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

pub(super) fn tile_for_range_copy(mut tile: MapTile) -> MapTile {
    tile.item = 0;
    tile.character = 0;
    tile.flags.remove(
        MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::TSOUNDBLOCK | MapFlags::DOOR,
    );
    tile
}

pub(super) fn apply_range_copy(tile: &mut MapTile, copied_tile: MapTile) {
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

pub(super) fn item_template_from_record(record: ZoneRecord) -> Result<ItemTemplate, ZoneError> {
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

pub(super) fn character_template_from_record(
    record: ZoneRecord,
) -> Result<CharacterTemplate, ZoneError> {
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

pub(super) fn place_character(
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

pub(super) fn place_item(
    world: &mut World,
    mut item: Item,
    x: usize,
    y: usize,
) -> Result<(), ZoneError> {
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
