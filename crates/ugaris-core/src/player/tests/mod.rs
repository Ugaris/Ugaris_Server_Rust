mod actions;
mod area1;
mod area3;
mod areas_misc;
mod arena;
mod blob;
mod chests;
mod keyring;
mod labs;
mod military;
mod misc;
mod pk;
mod questlog;
mod settings;
mod shrines;
mod staffer;
mod transport;
mod tunnel;
mod twocity;

use crate::{
    entity::{Character, CharacterFlags, ItemFlags, MAX_MODIFIERS},
    ids::ItemId,
};

use super::*;

fn sample_depot_item(id: u32, template_id: u32, flags: ItemFlags) -> Item {
    Item {
        id: ItemId(id),
        name: "Test Item".into(),
        description: "A test item".into(),
        flags,
        sprite: 4242,
        value: 100,
        min_level: 5,
        max_level: 50,
        needs_class: 3,
        template_id,
        owner_id: 12,
        modifier_index: [1, 2, 3, 4, 5],
        modifier_value: [10, 20, 30, 40, 50],
        x: 0,
        y: 0,
        carried_by: None,
        contained_in: None,
        content_id: 0,
        driver: 9,
        driver_data: (0..40).collect(),
        serial: 555,
    }
}

/// Puts `quest` into the `QF_DONE` state `reopen_quest_legacy`'s
/// generic preconditions require, without going through the full
/// `complete_legacy` exp-reward path.
fn mark_reopenable(player: &mut PlayerRuntime, quest: usize) {
    player.quest_log.mark_done(quest);
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
        speed_mode: crate::entity::SpeedMode::Normal,
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
    }
}
