use bytes::{BufMut, BytesMut};
use thiserror::Error;

pub const SV_SCROLL_UP: u8 = 1;
pub const SV_SCROLL_DOWN: u8 = 2;
pub const SV_SCROLL_LEFT: u8 = 3;
pub const SV_SCROLL_RIGHT: u8 = 4;
pub const SV_SCROLL_LEFTUP: u8 = 5;
pub const SV_SCROLL_RIGHTUP: u8 = 6;
pub const SV_SCROLL_LEFTDOWN: u8 = 7;
pub const SV_SCROLL_RIGHTDOWN: u8 = 8;
pub const SV_TEXT: u8 = 9;
pub const SV_SETVAL0: u8 = 10;
pub const SV_SETVAL1: u8 = 11;
pub const SV_SETHP: u8 = 12;
pub const SV_SETMANA: u8 = 13;
pub const SV_SETITEM: u8 = 14;
pub const SV_ORIGIN: u8 = 15;
pub const SV_TICKER: u8 = 16;
pub const SV_SETCITEM: u8 = 17;
pub const SV_ACT: u8 = 18;
pub const SV_EXIT: u8 = 19;
pub const SV_NAME: u8 = 20;
pub const SV_SERVER: u8 = 21;
pub const SV_CONTAINER: u8 = 22;
pub const SV_CONCNT: u8 = 23;
pub const SV_ENDURANCE: u8 = 24;
pub const SV_LIFESHIELD: u8 = 25;
pub const SV_EXP: u8 = 26;
pub const SV_EXP_USED: u8 = 27;
pub const SV_PRICE: u8 = 28;
pub const SV_CPRICE: u8 = 29;
pub const SV_GOLD: u8 = 30;
pub const SV_LOOKINV: u8 = 31;
pub const SV_ITEMPRICE: u8 = 32;
pub const SV_AREAINFO: u8 = 33;
pub const SV_CEFFECT: u8 = 34;
pub const SV_UEFFECT: u8 = 35;
pub const SV_REALTIME: u8 = 36;
pub const SV_SPEEDMODE: u8 = 37;
pub const SV_FIGHTMODE: u8 = 38;
pub const SV_CONTYPE: u8 = 39;
pub const SV_CONNAME: u8 = 40;
pub const SV_LS: u8 = 41;
pub const SV_CAT: u8 = 42;
pub const SV_LOGINDONE: u8 = 43;
pub const SV_SPECIAL: u8 = 44;
pub const SV_TELEPORT: u8 = 45;
pub const SV_SETRAGE: u8 = 46;
pub const SV_MIRROR: u8 = 47;
pub const SV_PROF: u8 = 48;
pub const SV_PING: u8 = 49;
pub const SV_UNIQUE: u8 = 50;
pub const SV_MIL_EXP: u8 = 51;
pub const SV_QUESTLOG: u8 = 52;
pub const SV_PROTOCOL: u8 = 53;
pub const SV_RESERVED1: u8 = 54;
pub const SV_RESERVED2: u8 = 55;
pub const SV_RESERVED3: u8 = 56;
pub const SV_RESERVED4: u8 = 57;
pub const SV_MOD1: u8 = 58;
pub const SV_MOD2: u8 = 59;
pub const SV_MOD3: u8 = 60;
pub const SV_MOD4: u8 = 61;
pub const SV_MOD5: u8 = 62;
pub const SV_RESERVED5: u8 = 63;

pub const SV_MAPTHIS: u8 = 0;
pub const SV_MAPNEXT: u8 = 16;
pub const SV_MAPOFF: u8 = 32;
pub const SV_MAPPOS: u8 = SV_MAPNEXT + SV_MAPOFF;
pub const SV_MAP01: u8 = 64;
pub const SV_MAP10: u8 = 128;
pub const SV_MAP11: u8 = SV_MAP01 + SV_MAP10;

pub const CMF_LIGHT: u16 = 1 + 2 + 4 + 8;
pub const CMF_VISIBLE: u16 = 16;
pub const CMF_TAKE: u16 = 32;
pub const CMF_USE: u16 = 64;
pub const CMF_INFRA: u16 = 128;
pub const CMF_UNDERWATER: u16 = 256;
pub const CMF_SINK_ANKLE: u16 = 512;
pub const CMF_SINK_KNEE: u16 = 1024;
pub const CMF_SINK_BELLY: u16 = 2048;
pub const CMF_SINK_CHEST: u16 = 4096;

pub const MAP_EFFECT_0: u8 = 1;
pub const MAP_EFFECT_1: u8 = 2;
pub const MAP_EFFECT_2: u8 = 4;
pub const MAP_EFFECT_3: u8 = 8;
pub const MAP_CHARACTER_SPRITE: u8 = 1;
pub const MAP_CHARACTER_ACTION: u8 = 2;
pub const MAP_CHARACTER_STATUS: u8 = 4;
pub const MAP_CHARACTER_CLEAR: u8 = 8;
pub const MAP_TILE_GSPRITE: u8 = 1;
pub const MAP_TILE_FSPRITE: u8 = 2;
pub const MAP_TILE_ISPRITE: u8 = 4;
pub const MAP_TILE_FLAGS: u8 = 8;

pub const MAP_ITEM_PLAYER_BODY_FLAG: u32 = 0x8000_0000;
pub const LOOKINV_WORN_SLOTS: usize = 12;
pub const SERVER_PROTOCOL_VERSION: u8 = 3;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PacketBuildError {
    #[error("legacy map field mask uses reserved high bits: {0:#04x}")]
    InvalidMapFieldMask(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapLayer {
    Effects,
    Character,
    Tile,
}

impl MapLayer {
    fn bits(self) -> u8 {
        match self {
            Self::Effects => SV_MAP01,
            Self::Character => SV_MAP10,
            Self::Tile => SV_MAP11,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapPosition {
    This,
    Next,
    Offset(u8),
    Absolute(u16),
}

#[derive(Debug, Default)]
pub struct PacketBuilder {
    payload: BytesMut,
}

impl PacketBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_payload(self) -> BytesMut {
        self.payload
    }

    pub fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.payload.extend_from_slice(bytes);
        self
    }

    pub fn realtime(&mut self, seconds: u32) -> &mut Self {
        self.raw(&realtime(seconds))
    }

    pub fn login_done(&mut self) -> &mut Self {
        self.payload.put_u8(SV_LOGINDONE);
        self
    }

    pub fn ticker(&mut self, ticker: u32) -> &mut Self {
        self.payload.put_u8(SV_TICKER);
        self.payload.put_u32_le(ticker);
        self
    }

    pub fn mirror(&mut self, mirror: u32) -> &mut Self {
        self.payload.put_u8(SV_MIRROR);
        self.payload.put_u32_le(mirror);
        self
    }

    pub fn protocol(&mut self, version: u8) -> &mut Self {
        self.raw(&protocol(version))
    }

    pub fn special(&mut self, special_type: u32, opt1: u32, opt2: u32) -> &mut Self {
        self.raw(&special(special_type, opt1, opt2))
    }

    pub fn exit(&mut self, reason: &str) -> &mut Self {
        let bytes = reason.as_bytes();
        let len = bytes.len().min(200);
        self.payload.put_u8(SV_EXIT);
        self.payload.put_u8(len as u8);
        self.payload.extend_from_slice(&bytes[..len]);
        self
    }

    pub fn scroll(&mut self, direction: u8) -> &mut Self {
        self.payload.put_u8(direction);
        self
    }

    pub fn origin(&mut self, x: u16, y: u16) -> &mut Self {
        self.raw(&origin(x, y))
    }

    pub fn map_delta(
        &mut self,
        layer: MapLayer,
        position: MapPosition,
        field_mask: u8,
        body: &[u8],
    ) -> Result<&mut Self, PacketBuildError> {
        let packet = map_delta(layer, position, field_mask, body)?;
        self.raw(&packet);
        Ok(self)
    }

    pub fn map_tile_basic(
        &mut self,
        position: MapPosition,
        ground_sprite: u32,
        foreground_sprite: u32,
        item_sprite: u32,
        flags: u16,
    ) -> Result<&mut Self, PacketBuildError> {
        let packet = map_tile_basic(
            position,
            ground_sprite,
            foreground_sprite,
            item_sprite,
            flags,
        )?;
        self.raw(&packet);
        Ok(self)
    }

    pub fn map_character_basic(
        &mut self,
        position: MapPosition,
        sprite: u32,
        character: u16,
        action: CharacterMapAction,
        status: CharacterMapStatus,
    ) -> Result<&mut Self, PacketBuildError> {
        let packet = map_character_basic(position, sprite, character, action, status)?;
        self.raw(&packet);
        Ok(self)
    }

    pub fn character_name(
        &mut self,
        character: u16,
        level: u8,
        colors: [u16; 3],
        clan: u8,
        pk_relation: u8,
        name: &str,
    ) -> &mut Self {
        self.raw(&character_name(
            character,
            level,
            colors,
            clan,
            pk_relation,
            name,
        ))
    }

    pub fn action(&mut self, action_id: u16, x: u16, y: u16) -> &mut Self {
        self.raw(&action(action_id, x, y))
    }

    pub fn set_value0(&mut self, value: u8, amount: i16) -> &mut Self {
        self.raw(&set_value0(value, amount))
    }

    pub fn set_value1(&mut self, value: u8, amount: i16) -> &mut Self {
        self.raw(&set_value1(value, amount))
    }

    pub fn set_hp(&mut self, percent: u16) -> &mut Self {
        self.payload.put_u8(SV_SETHP);
        self.payload.put_u16_le(percent);
        self
    }

    pub fn set_mana(&mut self, percent: u16) -> &mut Self {
        self.payload.put_u8(SV_SETMANA);
        self.payload.put_u16_le(percent);
        self
    }

    pub fn set_endurance(&mut self, percent: u16) -> &mut Self {
        self.payload.put_u8(SV_ENDURANCE);
        self.payload.put_u16_le(percent);
        self
    }

    pub fn set_lifeshield(&mut self, percent: u16) -> &mut Self {
        self.payload.put_u8(SV_LIFESHIELD);
        self.payload.put_u16_le(percent);
        self
    }

    pub fn set_rage(&mut self, percent: u16) -> &mut Self {
        self.payload.put_u8(SV_SETRAGE);
        self.payload.put_u16_le(percent);
        self
    }

    pub fn exp(&mut self, exp: u32) -> &mut Self {
        self.payload.put_u8(SV_EXP);
        self.payload.put_u32_le(exp);
        self
    }

    pub fn exp_used(&mut self, exp_used: u32) -> &mut Self {
        self.payload.put_u8(SV_EXP_USED);
        self.payload.put_u32_le(exp_used);
        self
    }

    pub fn military_exp(&mut self, exp: u32) -> &mut Self {
        self.payload.put_u8(SV_MIL_EXP);
        self.payload.put_u32_le(exp);
        self
    }

    pub fn speed_mode(&mut self, mode: u8) -> &mut Self {
        self.payload.put_u8(SV_SPEEDMODE);
        self.payload.put_u8(mode);
        self
    }

    pub fn fight_mode(&mut self, mode: u8) -> &mut Self {
        self.payload.put_u8(SV_FIGHTMODE);
        self.payload.put_u8(mode);
        self
    }

    pub fn set_item(&mut self, slot: u8, sprite: u32, flags: u32) -> &mut Self {
        self.raw(&set_item(slot, sprite, flags))
    }

    pub fn set_cursor_item(&mut self, sprite: u32, flags: u32) -> &mut Self {
        self.raw(&set_cursor_item(sprite, flags))
    }

    pub fn container_item(&mut self, slot: u8, sprite: u32) -> &mut Self {
        self.raw(&container_item(slot, sprite))
    }

    pub fn container_count(&mut self, count: u8) -> &mut Self {
        self.payload.put_u8(SV_CONCNT);
        self.payload.put_u8(count);
        self
    }

    pub fn container_type(&mut self, container_type: u8) -> &mut Self {
        self.payload.put_u8(SV_CONTYPE);
        self.payload.put_u8(container_type);
        self
    }

    pub fn container_name(&mut self, name: &str) -> &mut Self {
        self.raw(&container_name(name))
    }

    pub fn item_price(&mut self, slot: u8, price: u32) -> &mut Self {
        self.raw(&item_price(slot, price))
    }

    pub fn container_price(&mut self, slot: u8, price: u32) -> &mut Self {
        self.raw(&container_price(slot, price))
    }

    pub fn cursor_price(&mut self, price: u32) -> &mut Self {
        self.payload.put_u8(SV_CPRICE);
        self.payload.put_u32_le(price);
        self
    }

    pub fn gold(&mut self, gold: u32) -> &mut Self {
        self.payload.put_u8(SV_GOLD);
        self.payload.put_u32_le(gold);
        self
    }

    pub fn look_inventory(
        &mut self,
        sprite: u32,
        colors: [u32; 3],
        worn_sprites: [u32; LOOKINV_WORN_SLOTS],
    ) -> &mut Self {
        self.raw(&look_inventory(sprite, colors, worn_sprites))
    }

    pub fn system_text(&mut self, message: &str) -> &mut Self {
        self.raw(&system_text(message))
    }

    pub fn system_text_bytes(&mut self, message: &[u8]) -> &mut Self {
        self.raw(&system_text_bytes(message))
    }
}

pub fn realtime(seconds: u32) -> [u8; 5] {
    let mut out = [0; 5];
    out[0] = SV_REALTIME;
    out[1..].copy_from_slice(&seconds.to_le_bytes());
    out
}

pub fn protocol(version: u8) -> [u8; 2] {
    [SV_PROTOCOL, version.min(SERVER_PROTOCOL_VERSION)]
}

pub fn origin(x: u16, y: u16) -> [u8; 5] {
    let mut out = [0; 5];
    out[0] = SV_ORIGIN;
    out[1..3].copy_from_slice(&x.to_le_bytes());
    out[3..5].copy_from_slice(&y.to_le_bytes());
    out
}

pub fn map_delta(
    layer: MapLayer,
    position: MapPosition,
    field_mask: u8,
    body: &[u8],
) -> Result<BytesMut, PacketBuildError> {
    if field_mask & !0x0f != 0 {
        return Err(PacketBuildError::InvalidMapFieldMask(field_mask));
    }

    let mut out = BytesMut::with_capacity(body.len() + 3);
    let header = layer.bits() | field_mask;
    match position {
        MapPosition::This => out.put_u8(header | SV_MAPTHIS),
        MapPosition::Next => out.put_u8(header | SV_MAPNEXT),
        MapPosition::Offset(offset) => {
            out.put_u8(header | SV_MAPOFF);
            out.put_u8(offset);
        }
        MapPosition::Absolute(pos) => {
            out.put_u8(header | SV_MAPPOS);
            out.put_u16_le(pos);
        }
    }
    out.extend_from_slice(body);
    Ok(out)
}

pub fn map_tile_basic(
    position: MapPosition,
    ground_sprite: u32,
    foreground_sprite: u32,
    item_sprite: u32,
    flags: u16,
) -> Result<BytesMut, PacketBuildError> {
    let mut body = BytesMut::with_capacity(14);
    body.put_u32_le(ground_sprite);
    body.put_u32_le(foreground_sprite);
    body.put_u32_le(item_sprite);
    if flags & 0xff != 0 {
        body.put_u16_le(flags);
    } else {
        body.put_u8(flags as u8);
    }
    map_delta(
        MapLayer::Tile,
        position,
        MAP_TILE_GSPRITE | MAP_TILE_FSPRITE | MAP_TILE_ISPRITE | MAP_TILE_FLAGS,
        &body,
    )
}

pub fn map_effects_basic(
    position: MapPosition,
    effects: [u16; 4],
) -> Result<BytesMut, PacketBuildError> {
    let mut body = BytesMut::with_capacity(16);
    for effect in effects {
        body.put_u32_le(u32::from(effect));
    }
    map_delta(
        MapLayer::Effects,
        position,
        MAP_EFFECT_0 | MAP_EFFECT_1 | MAP_EFFECT_2 | MAP_EFFECT_3,
        &body,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterMapAction {
    pub action: u8,
    pub duration: u8,
    pub step: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterMapStatus {
    pub dir: u8,
    pub health: u8,
    pub mana: u8,
    pub shield: u8,
}

pub fn map_character_basic(
    position: MapPosition,
    sprite: u32,
    character: u16,
    action: CharacterMapAction,
    status: CharacterMapStatus,
) -> Result<BytesMut, PacketBuildError> {
    let mut body = BytesMut::with_capacity(13);
    body.put_u32_le(sprite);
    body.put_u16_le(character);
    body.put_u8(action.action);
    body.put_u8(action.duration);
    body.put_u8(action.step);
    body.put_u8(status.dir);
    body.put_u8(status.health);
    body.put_u8(status.mana);
    body.put_u8(status.shield);
    map_delta(
        MapLayer::Character,
        position,
        MAP_CHARACTER_SPRITE | MAP_CHARACTER_ACTION | MAP_CHARACTER_STATUS,
        &body,
    )
}

pub fn character_name(
    character: u16,
    level: u8,
    colors: [u16; 3],
    clan: u8,
    pk_relation: u8,
    name: &str,
) -> BytesMut {
    let bytes = name.as_bytes();
    let len = bytes.len().min(u8::MAX as usize);
    let mut out = BytesMut::with_capacity(len + 13);
    out.put_u8(SV_NAME);
    out.put_u16_le(character);
    out.put_u8(level);
    for color in colors {
        out.put_u16_le(color);
    }
    out.put_u8(clan);
    out.put_u8(pk_relation);
    out.put_u8(len as u8);
    out.extend_from_slice(&bytes[..len]);
    out
}

pub fn client_effect(slot: u8, body: &[u8]) -> BytesMut {
    let mut out = BytesMut::with_capacity(body.len() + 2);
    out.put_u8(SV_CEFFECT);
    out.put_u8(slot);
    out.extend_from_slice(body);
    out
}

pub fn used_effects(mask: u64) -> [u8; 9] {
    let mut out = [0; 9];
    out[0] = SV_UEFFECT;
    out[1..].copy_from_slice(&mask.to_le_bytes());
    out
}

pub fn ceffect_ball(
    effect_id: i32,
    start: i32,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> BytesMut {
    let mut out = BytesMut::with_capacity(24);
    out.put_i32_le(effect_id);
    out.put_i32_le(2);
    out.put_i32_le(start);
    out.put_i32_le(from_x);
    out.put_i32_le(from_y);
    out.put_i32_le(to_x);
    out.put_i32_le(to_y);
    out
}

pub fn ceffect_strike(effect_id: i32, character: i32, x: i32, y: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(20);
    out.put_i32_le(effect_id);
    out.put_i32_le(3);
    out.put_i32_le(character);
    out.put_i32_le(x);
    out.put_i32_le(y);
    out
}

pub fn ceffect_burn(effect_id: i32, character: i32, stop: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(16);
    out.put_i32_le(effect_id);
    out.put_i32_le(12);
    out.put_i32_le(character);
    out.put_i32_le(stop);
    out
}

pub fn ceffect_shield(effect_id: i32, character: i32, start: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(16);
    out.put_i32_le(effect_id);
    out.put_i32_le(1);
    out.put_i32_le(character);
    out.put_i32_le(start);
    out
}

pub fn ceffect_flash(effect_id: i32, character: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(12);
    out.put_i32_le(effect_id);
    out.put_i32_le(5);
    out.put_i32_le(character);
    out
}

pub fn ceffect_warcry(effect_id: i32, character: i32, stop: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(16);
    out.put_i32_le(effect_id);
    out.put_i32_le(8);
    out.put_i32_le(character);
    out.put_i32_le(stop);
    out
}

pub fn ceffect_bless(
    effect_id: i32,
    character: i32,
    start: i32,
    stop: i32,
    strength: i32,
) -> BytesMut {
    let mut out = BytesMut::with_capacity(24);
    out.put_i32_le(effect_id);
    out.put_i32_le(9);
    out.put_i32_le(character);
    out.put_i32_le(start);
    out.put_i32_le(stop);
    out.put_i32_le(strength);
    out
}

pub fn ceffect_heal(effect_id: i32, character: i32, start: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(16);
    out.put_i32_le(effect_id);
    out.put_i32_le(10);
    out.put_i32_le(character);
    out.put_i32_le(start);
    out
}

pub fn ceffect_freeze(effect_id: i32, character: i32, start: i32, stop: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(20);
    out.put_i32_le(effect_id);
    out.put_i32_le(11);
    out.put_i32_le(character);
    out.put_i32_le(start);
    out.put_i32_le(stop);
    out
}

pub fn ceffect_potion(
    effect_id: i32,
    character: i32,
    start: i32,
    stop: i32,
    strength: i32,
) -> BytesMut {
    let mut out = BytesMut::with_capacity(24);
    out.put_i32_le(effect_id);
    out.put_i32_le(14);
    out.put_i32_le(character);
    out.put_i32_le(start);
    out.put_i32_le(stop);
    out.put_i32_le(strength);
    out
}

pub fn ceffect_pulse(effect_id: i32, start: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(12);
    out.put_i32_le(effect_id);
    out.put_i32_le(21);
    out.put_i32_le(start);
    out
}

pub fn ceffect_pulseback(effect_id: i32, character: i32, x: i32, y: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(20);
    out.put_i32_le(effect_id);
    out.put_i32_le(22);
    out.put_i32_le(character);
    out.put_i32_le(x);
    out.put_i32_le(y);
    out
}

pub fn ceffect_firering(effect_id: i32, character: i32, start: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(16);
    out.put_i32_le(effect_id);
    out.put_i32_le(23);
    out.put_i32_le(character);
    out.put_i32_le(start);
    out
}

pub fn ceffect_fireball(
    effect_id: i32,
    start: i32,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> BytesMut {
    let mut out = BytesMut::with_capacity(24);
    out.put_i32_le(effect_id);
    out.put_i32_le(4);
    out.put_i32_le(start);
    out.put_i32_le(from_x);
    out.put_i32_le(from_y);
    out.put_i32_le(to_x);
    out.put_i32_le(to_y);
    out
}

pub fn ceffect_explode(effect_id: i32, start: i32, base_sprite: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(16);
    out.put_i32_le(effect_id);
    out.put_i32_le(7);
    out.put_i32_le(start);
    out.put_i32_le(base_sprite);
    out
}

pub fn ceffect_mist(effect_id: i32, start: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(12);
    out.put_i32_le(effect_id);
    out.put_i32_le(13);
    out.put_i32_le(start);
    out
}

pub fn ceffect_earthrain(effect_id: i32, strength: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(12);
    out.put_i32_le(effect_id);
    out.put_i32_le(15);
    out.put_i32_le(strength);
    out
}

pub fn ceffect_earthmud(effect_id: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(8);
    out.put_i32_le(effect_id);
    out.put_i32_le(16);
    out
}

pub fn ceffect_bubble(effect_id: i32, y_offset: i32) -> BytesMut {
    let mut out = BytesMut::with_capacity(12);
    out.put_i32_le(effect_id);
    out.put_i32_le(24);
    out.put_i32_le(y_offset);
    out
}

pub fn action(action: u16, x: u16, y: u16) -> [u8; 7] {
    let mut out = [0; 7];
    out[0] = SV_ACT;
    out[1..3].copy_from_slice(&action.to_le_bytes());
    out[3..5].copy_from_slice(&x.to_le_bytes());
    out[5..7].copy_from_slice(&y.to_le_bytes());
    out
}

pub fn set_value0(value: u8, amount: i16) -> [u8; 4] {
    set_value(SV_SETVAL0, value, amount)
}

pub fn set_value1(value: u8, amount: i16) -> [u8; 4] {
    set_value(SV_SETVAL1, value, amount)
}

fn set_value(command: u8, value: u8, amount: i16) -> [u8; 4] {
    let mut out = [0; 4];
    out[0] = command;
    out[1] = value;
    out[2..4].copy_from_slice(&amount.to_le_bytes());
    out
}

pub fn set_item(slot: u8, sprite: u32, flags: u32) -> [u8; 10] {
    let mut out = [0; 10];
    out[0] = SV_SETITEM;
    out[1] = slot;
    out[2..6].copy_from_slice(&sprite.to_le_bytes());
    out[6..10].copy_from_slice(&flags.to_le_bytes());
    out
}

pub fn set_cursor_item(sprite: u32, flags: u32) -> [u8; 9] {
    let mut out = [0; 9];
    out[0] = SV_SETCITEM;
    out[1..5].copy_from_slice(&sprite.to_le_bytes());
    out[5..9].copy_from_slice(&flags.to_le_bytes());
    out
}

pub fn container_item(slot: u8, sprite: u32) -> [u8; 6] {
    let mut out = [0; 6];
    out[0] = SV_CONTAINER;
    out[1] = slot;
    out[2..6].copy_from_slice(&sprite.to_le_bytes());
    out
}

pub fn container_name(name: &str) -> BytesMut {
    let bytes = name.as_bytes();
    let len = bytes.len().min(u8::MAX as usize);
    let mut out = BytesMut::with_capacity(len + 2);
    out.put_u8(SV_CONNAME);
    out.put_u8(len as u8);
    out.extend_from_slice(&bytes[..len]);
    out
}

pub fn item_price(slot: u8, price: u32) -> [u8; 6] {
    slot_u32_packet(SV_ITEMPRICE, slot, price)
}

pub fn container_price(slot: u8, price: u32) -> [u8; 6] {
    slot_u32_packet(SV_PRICE, slot, price)
}

fn slot_u32_packet(command: u8, slot: u8, value: u32) -> [u8; 6] {
    let mut out = [0; 6];
    out[0] = command;
    out[1] = slot;
    out[2..6].copy_from_slice(&value.to_le_bytes());
    out
}

pub fn look_inventory(
    sprite: u32,
    colors: [u32; 3],
    worn_sprites: [u32; LOOKINV_WORN_SLOTS],
) -> BytesMut {
    let mut out = BytesMut::with_capacity(17 + LOOKINV_WORN_SLOTS * 4);
    out.put_u8(SV_LOOKINV);
    out.put_u32_le(sprite);
    for color in colors {
        out.put_u32_le(color);
    }
    for worn_sprite in worn_sprites {
        out.put_u32_le(worn_sprite);
    }
    out
}

pub fn system_text(message: &str) -> BytesMut {
    system_text_bytes(message.as_bytes())
}

pub fn system_text_bytes(bytes: &[u8]) -> BytesMut {
    let len = bytes.len().min(u16::MAX as usize);
    let mut out = BytesMut::with_capacity(len + 3);
    out.put_u8(SV_TEXT);
    out.put_u16_le(len as u16);
    out.extend_from_slice(&bytes[..len]);
    out
}

pub fn special(special_type: u32, opt1: u32, opt2: u32) -> [u8; 13] {
    let mut out = [0; 13];
    out[0] = SV_SPECIAL;
    out[1..5].copy_from_slice(&special_type.to_le_bytes());
    out[5..9].copy_from_slice(&opt1.to_le_bytes());
    out[9..13].copy_from_slice(&opt2.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_builder_uses_little_endian_legacy_ints() {
        let mut builder = PacketBuilder::new();
        builder.ticker(0x11223344);
        let payload = builder.into_payload();
        assert_eq!(&payload[..], &[SV_TICKER, 0x44, 0x33, 0x22, 0x11]);
    }

    #[test]
    fn exit_reason_is_capped_to_legacy_limit() {
        let reason = "x".repeat(250);
        let mut builder = PacketBuilder::new();
        builder.exit(&reason);
        let payload = builder.into_payload();
        assert_eq!(payload[0], SV_EXIT);
        assert_eq!(payload[1], 200);
        assert_eq!(payload.len(), 202);
    }

    #[test]
    fn system_text_bytes_preserves_legacy_color_marker() {
        let packet = system_text_bytes(&[0xb0, b'c', b'3', b'H', b'i']);
        assert_eq!(&packet[..], &[SV_TEXT, 5, 0, 0xb0, b'c', b'3', b'H', b'i']);
    }

    #[test]
    fn special_packet_matches_player_special_layout() {
        assert_eq!(
            special(0x11223344, 0xffff_ffff, 0x55667788),
            [SV_SPECIAL, 0x44, 0x33, 0x22, 0x11, 0xff, 0xff, 0xff, 0xff, 0x88, 0x77, 0x66, 0x55,]
        );
    }

    #[test]
    fn map_anchored_effect_packets_match_legacy_layouts() {
        assert_eq!(
            &ceffect_explode(1, 2, 50050)[..],
            &[1, 0, 0, 0, 7, 0, 0, 0, 2, 0, 0, 0, 0x82, 0xc3, 0, 0]
        );
        assert_eq!(
            &ceffect_mist(3, 4)[..],
            &[3, 0, 0, 0, 13, 0, 0, 0, 4, 0, 0, 0]
        );
        assert_eq!(
            &ceffect_earthrain(5, 6)[..],
            &[5, 0, 0, 0, 15, 0, 0, 0, 6, 0, 0, 0]
        );
        assert_eq!(&ceffect_earthmud(7)[..], &[7, 0, 0, 0, 16, 0, 0, 0]);
        assert_eq!(
            &ceffect_bubble(8, 45)[..],
            &[8, 0, 0, 0, 24, 0, 0, 0, 45, 0, 0, 0]
        );
    }

    #[test]
    fn origin_uses_legacy_little_endian_coordinates() {
        assert_eq!(origin(0x1234, 0x5678), [SV_ORIGIN, 0x34, 0x12, 0x78, 0x56]);
    }

    #[test]
    fn stat_value_packets_use_signed_little_endian_amounts() {
        assert_eq!(set_value0(7, -2), [SV_SETVAL0, 7, 0xfe, 0xff]);
        assert_eq!(set_value1(8, 0x1234), [SV_SETVAL1, 8, 0x34, 0x12]);
    }

    #[test]
    fn item_packets_match_player_stats_layout() {
        assert_eq!(
            set_item(3, 0x11223344, 0x55667788),
            [SV_SETITEM, 3, 0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55]
        );
        assert_eq!(
            set_cursor_item(0x01020304, 0xa0b0c0d0),
            [SV_SETCITEM, 4, 3, 2, 1, 0xd0, 0xc0, 0xb0, 0xa0]
        );
    }

    #[test]
    fn effect_packets_match_legacy_ceffect_layouts() {
        assert_eq!(
            &used_effects(0x0102_0304_0506_0708)[..],
            &[SV_UEFFECT, 8, 7, 6, 5, 4, 3, 2, 1]
        );

        let ball = ceffect_ball(0x11, 0x22, 0x33, 0x44, 0x55, 0x66);
        assert_eq!(
            &client_effect(7, &ball)[..],
            &[
                SV_CEFFECT, 7, 0x11, 0, 0, 0, 2, 0, 0, 0, 0x22, 0, 0, 0, 0x33, 0, 0, 0, 0x44, 0, 0,
                0, 0x55, 0, 0, 0, 0x66, 0, 0, 0,
            ]
        );

        assert_eq!(
            &ceffect_strike(0x11, 0x22, 0x33, 0x44)[..],
            &[0x11, 0, 0, 0, 3, 0, 0, 0, 0x22, 0, 0, 0, 0x33, 0, 0, 0, 0x44, 0, 0, 0]
        );
        assert_eq!(
            &ceffect_burn(0x11, 0x22, 0x33)[..],
            &[0x11, 0, 0, 0, 12, 0, 0, 0, 0x22, 0, 0, 0, 0x33, 0, 0, 0]
        );
        assert_eq!(
            &ceffect_shield(0x11, 0x22, 0x33)[..],
            &[0x11, 0, 0, 0, 1, 0, 0, 0, 0x22, 0, 0, 0, 0x33, 0, 0, 0]
        );
        assert_eq!(
            &ceffect_flash(0x11, 0x22)[..],
            &[0x11, 0, 0, 0, 5, 0, 0, 0, 0x22, 0, 0, 0]
        );
        assert_eq!(
            &ceffect_bless(0x11, 0x22, 0x33, 0x44, 0x55)[..],
            &[
                0x11, 0, 0, 0, 9, 0, 0, 0, 0x22, 0, 0, 0, 0x33, 0, 0, 0, 0x44, 0, 0, 0, 0x55, 0, 0,
                0,
            ]
        );
        assert_eq!(&ceffect_heal(1, 2, 3)[4..8], &[10, 0, 0, 0]);
        assert_eq!(&ceffect_freeze(1, 2, 3, 4)[4..8], &[11, 0, 0, 0]);
        assert_eq!(&ceffect_potion(1, 2, 3, 4, 5)[4..8], &[14, 0, 0, 0]);
        assert_eq!(
            &ceffect_pulse(1, 2)[..],
            &[1, 0, 0, 0, 21, 0, 0, 0, 2, 0, 0, 0]
        );
        assert_eq!(
            &ceffect_pulseback(1, 2, 3, 4)[..],
            &[1, 0, 0, 0, 22, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]
        );
        assert_eq!(&ceffect_firering(1, 2, 3)[4..8], &[23, 0, 0, 0]);
        assert_eq!(&ceffect_fireball(1, 2, 3, 4, 5, 6)[4..8], &[4, 0, 0, 0]);
    }

    #[test]
    fn container_packets_match_legacy_layout() {
        assert_eq!(
            container_item(2, 0x11223344),
            [SV_CONTAINER, 2, 0x44, 0x33, 0x22, 0x11]
        );
        assert_eq!(container_price(9, 0x01020304), [SV_PRICE, 9, 4, 3, 2, 1]);
        assert_eq!(item_price(9, 0x01020304), [SV_ITEMPRICE, 9, 4, 3, 2, 1]);
        assert_eq!(
            &container_name("Depot")[..],
            &[SV_CONNAME, 5, b'D', b'e', b'p', b'o', b't']
        );
    }

    #[test]
    fn look_inventory_has_legacy_fixed_worn_slot_layout() {
        let packet = look_inventory(0x01020304, [0x11, 0x22, 0x33], [7; LOOKINV_WORN_SLOTS]);
        assert_eq!(packet.len(), 17 + LOOKINV_WORN_SLOTS * 4);
        assert_eq!(
            &packet[..17],
            &[SV_LOOKINV, 4, 3, 2, 1, 0x11, 0, 0, 0, 0x22, 0, 0, 0, 0x33, 0, 0, 0]
        );
        assert_eq!(&packet[17..21], &[7, 0, 0, 0]);
    }

    #[test]
    fn map_delta_encodes_layer_position_and_field_mask() {
        let packet = map_delta(
            MapLayer::Character,
            MapPosition::Offset(4),
            MAP_CHARACTER_SPRITE | MAP_CHARACTER_STATUS,
            &[1, 2, 3],
        )
        .unwrap();
        assert_eq!(&packet[..], &[SV_MAP10 | SV_MAPOFF | 5, 4, 1, 2, 3]);

        let packet = map_delta(
            MapLayer::Tile,
            MapPosition::Absolute(0x1234),
            MAP_TILE_FLAGS,
            &[0xaa],
        )
        .unwrap();
        assert_eq!(
            &packet[..],
            &[SV_MAP11 | SV_MAPPOS | MAP_TILE_FLAGS, 0x34, 0x12, 0xaa]
        );
    }

    #[test]
    fn map_delta_rejects_reserved_field_mask_bits() {
        assert_eq!(
            map_delta(MapLayer::Effects, MapPosition::This, 0x10, &[]),
            Err(PacketBuildError::InvalidMapFieldMask(0x10))
        );
    }

    #[test]
    fn map_tile_basic_matches_legacy_tile_layer_body() {
        let packet =
            map_tile_basic(MapPosition::Absolute(0x1234), 1, 2, 3, CMF_VISIBLE | 1).unwrap();

        assert_eq!(
            &packet[..],
            &[
                SV_MAP11
                    | SV_MAPPOS
                    | MAP_TILE_GSPRITE
                    | MAP_TILE_FSPRITE
                    | MAP_TILE_ISPRITE
                    | MAP_TILE_FLAGS,
                0x34,
                0x12,
                1,
                0,
                0,
                0,
                2,
                0,
                0,
                0,
                3,
                0,
                0,
                0,
                17,
                0,
            ]
        );
    }

    #[test]
    fn map_effects_basic_matches_legacy_effect_layer_body() {
        let packet = map_effects_basic(MapPosition::Absolute(0x1234), [1, 2, 0x1234, 0]).unwrap();

        assert_eq!(
            &packet[..],
            &[
                SV_MAP01 | SV_MAPPOS | MAP_EFFECT_0 | MAP_EFFECT_1 | MAP_EFFECT_2 | MAP_EFFECT_3,
                0x34,
                0x12,
                1,
                0,
                0,
                0,
                2,
                0,
                0,
                0,
                0x34,
                0x12,
                0,
                0,
                0,
                0,
                0,
                0,
            ]
        );
    }

    #[test]
    fn map_character_basic_matches_legacy_character_layer_body() {
        let packet = map_character_basic(
            MapPosition::Absolute(0x1234),
            0x01020304,
            0x1122,
            CharacterMapAction {
                action: 5,
                duration: 6,
                step: 7,
            },
            CharacterMapStatus {
                dir: 8,
                health: 90,
                mana: 80,
                shield: 70,
            },
        )
        .unwrap();

        assert_eq!(
            &packet[..],
            &[
                SV_MAP10
                    | SV_MAPPOS
                    | MAP_CHARACTER_SPRITE
                    | MAP_CHARACTER_ACTION
                    | MAP_CHARACTER_STATUS,
                0x34,
                0x12,
                4,
                3,
                2,
                1,
                0x22,
                0x11,
                5,
                6,
                7,
                8,
                90,
                80,
                70,
            ]
        );
    }

    #[test]
    fn character_name_matches_legacy_identity_layout() {
        let packet = character_name(0x1234, 55, [0x0102, 0x0304, 0x0506], 7, 8, "Guard");
        assert_eq!(
            &packet[..],
            &[
                SV_NAME, 0x34, 0x12, 55, 0x02, 0x01, 0x04, 0x03, 0x06, 0x05, 7, 8, 5, b'G', b'u',
                b'a', b'r', b'd'
            ]
        );
    }

    #[test]
    fn action_matches_legacy_local_action_layout() {
        assert_eq!(
            action(0x1234, 0x5678, 0x9abc),
            [SV_ACT, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a]
        );
    }
}
