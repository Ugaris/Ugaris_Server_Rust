use super::*;

mod alchemy;
mod area11_palace;
mod area12_mine;
mod area13_dungeon;
mod area14_random;
mod area15_swamp;
mod area16_forest;
mod area17_two;
mod area18_bones;
mod area19_nomad;
mod area2;
mod area20_lq;
mod area22_lab;
mod area25_warped;
mod area26_staffer;
mod area28_forest;
mod area29_brannington;
mod area30_clan;
mod area31_warrmines;
mod area34_teufel;
mod area36_caligar;
mod area37_arkhata;
mod area4_pents;
mod area6_edemon;
mod area8_fdemon;
mod arena;
mod assemble;
mod base;
mod books;
mod chests;
mod doors;
mod food;
mod ice;
mod lights;
mod orbs;
mod potions;
mod saltmine;
mod scrolls;
mod sewers;
mod shrines;
mod teleports;
mod traps;
mod xmas;

use crate::{
    entity::{Character, Item, ItemFlags, SpeedMode, MAX_MODIFIERS},
    ids::{CharacterId, ItemId},
};

fn request(character_id: u32, item_id: u32, spec: i32) -> ItemUseRequest {
    ItemUseRequest {
        character_id: CharacterId(character_id),
        item_id: ItemId(item_id),
        spec,
    }
}

fn character(id: u32) -> Character {
    Character {
        merchant: None,
        template_key: String::new(),
        respawn_ticks: 0,
        id: CharacterId(id),
        serial: id,
        name: "Character".into(),
        description: String::new(),
        flags: CharacterFlags::USED,
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
        level: 1,
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

fn item(id: u32, flags: ItemFlags, content_id: u16, driver: u16) -> Item {
    Item {
        id: ItemId(id),
        name: "Item".into(),
        description: String::new(),
        flags,
        sprite: 0,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; MAX_MODIFIERS],
        modifier_value: [0; MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: None,
        contained_in: None,
        content_id,
        driver,
        driver_data: Vec::new(),
        serial: 0,
    }
}
