use serde::{Deserialize, Serialize};

use crate::{ids::CharacterId, legacy::MAX_FIELD};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub effect_type: i32,
    pub serial: i32,
    pub start_tick: i32,
    pub stop_tick: i32,
    pub caster: Option<CharacterId>,
    pub caster_serial: i32,
    pub strength: i32,
    pub light: i32,
    pub fields: Vec<i32>,
    pub target_character: Option<CharacterId>,
    pub from_x: i32,
    pub from_y: i32,
    pub to_x: i32,
    pub to_y: i32,
    pub x: i32,
    pub y: i32,
    pub last_x: i32,
    pub last_y: i32,
    pub number_of_enemies: i32,
    pub base_sprite: i32,
}

impl Effect {
    pub fn new(effect_type: i32, serial: i32, start_tick: i32, stop_tick: i32) -> Self {
        Self {
            effect_type,
            serial,
            start_tick,
            stop_tick,
            caster: None,
            caster_serial: 0,
            strength: 0,
            light: 0,
            fields: Vec::with_capacity(MAX_FIELD),
            target_character: None,
            from_x: 0,
            from_y: 0,
            to_x: 0,
            to_y: 0,
            x: 0,
            y: 0,
            last_x: 0,
            last_y: 0,
            number_of_enemies: 0,
            base_sprite: 0,
        }
    }
}
