use super::*;
use crate::{
    entity::{ItemFlags, SpeedMode},
    ids::ItemId,
};

mod dialogue;
mod framework;
mod registry;
mod simple_baddy;

fn gate_context(
    welcome_state: i32,
    needs_lab: bool,
    flags: CharacterFlags,
) -> GateWelcomeContext<'static> {
    GateWelcomeContext {
        player_name: "Hero",
        welcome_state,
        needs_lab,
        flags,
    }
}

fn test_character() -> Character {
    Character {
        merchant: None,
        template_key: String::new(),
        respawn_ticks: 0,
        id: crate::ids::CharacterId(1),
        serial: 1,
        name: "Rat".to_string(),
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
        driver_memory: DriverMemory::default(),
        class: 0,
        dungeonfighter: None,
        fight_driver: None,
        lq_usurp: None,
    }
}

fn clara_context(clara_state: i32, kelly_state: i32) -> ClaraDialogueContext<'static> {
    ClaraDialogueContext {
        player_name: "Hero",
        clara_name: "Clara",
        army_rank: "Private",
        kelly_state,
        clara_state,
        has_hardkill_item: false,
        hardkill_ritual_progress: 0,
        questlog_21_count: 0,
    }
}

fn test_item(id: ItemId, driver: u16, driver_data: &[u8]) -> Item {
    Item {
        id,
        name: String::new(),
        description: String::new(),
        flags: ItemFlags::USED,
        sprite: 0,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; crate::entity::MAX_MODIFIERS],
        modifier_value: [0; crate::entity::MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: None,
        contained_in: None,
        content_id: 0,
        driver,
        driver_data: driver_data.to_vec(),
        serial: 0,
    }
}
