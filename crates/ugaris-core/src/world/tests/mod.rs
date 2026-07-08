use super::*;

mod aclerk;
mod actions;
mod admin_flag;
mod allow;
mod anticheat;
mod area_mech;
mod arena;
mod assembly;
mod astro1;
mod astro2;
mod asturin;
mod balltrap;
mod bank;
mod brithildie;
mod camhermit;
mod carlos;
mod character_values;
mod clanclerk;
mod clanmaster;
mod clubmaster;
mod combat;
mod complain;
mod consistency;
mod date;
mod death;
mod doors;
mod dungeon_fighter;
mod dungeon_master;
mod effect_tick;
mod effects;
mod exp;
mod exterminate;
mod forest_ranger;
mod gate_fight;
mod gatekeeper;
mod greeter;
mod guiwynn;
mod gwendylon;
mod hurt;
mod item_outcomes;
mod items;
mod jail;
mod james;
mod janitor;
mod jessica;
mod jiu;
mod kelly;
mod lab2_undead;
mod lastseen;
mod light;
mod lockname;
mod logain;
mod look;
mod loot;
mod lostcon;
mod lq;
mod lydia;
mod macro_npc;
mod merchant;
mod military;
mod moonie;
mod nook;
mod npc_fight;
mod npc_idle;
mod npc_messages;
mod player_driver;
mod punish;
mod querystats;
mod regen;
mod rename;
mod reskin;
mod rmdeath;
mod robber;
mod sanoa;
mod seymour;
mod sir_jones;
mod skills;
mod spawn;
mod special_item;
mod speed;
mod spells;
mod steal;
mod superior;
mod teleport;
mod terion;
mod text;
mod thomas;
mod trader;
mod turn_seyan;
mod tutorial;
mod values;
mod vampire;
mod vampire2;
mod weather;
mod world_misc;
mod yoakin;

use crate::{
    character_driver::{
        CharacterDriverState, Lab2UndeadDriverData, SimpleBaddyDriverData, SimpleBaddyEnemy,
        FDEMON_MSG_WAYPOINT, NTID_FDEMON, NTID_GLADIATOR, NTID_LAB2_DEAMONCHECK,
        NTID_LABGNOMETORCH, NT_CHAR, NT_DEAD, NT_DIDHIT, NT_GIVE, NT_GOTHIT, NT_NPC, NT_SEEHIT,
    },
    clan::ClanRelation,
    direction::Direction,
    entity::{CharacterFlags, CharacterValue, ItemFlags, SpeedMode, MAX_MODIFIERS, POWERSCALE},
    item_driver::{
        UseItemOutcome, IDR_ANTIENCHANTITEM, IDR_BALLTRAP, IDR_BONEBRIDGE, IDR_BRANNINGTONFOREST,
        IDR_CALIGAR, IDR_CALIGARFLAME, IDR_CHESTSPAWN, IDR_DOOR, IDR_EDEMONBALL, IDR_EDEMONLIGHT,
        IDR_ENCHANTITEM, IDR_FDEMONBLOOD, IDR_FDEMONLAVA, IDR_FIREBALL, IDR_FLAMETHROW, IDR_FLASK,
        IDR_LAB2_REGENERATE, IDR_LAB2_STEPACTION, IDR_LAB2_WATER, IDR_LAB3_PLANT, IDR_LABTORCH,
        IDR_LIZARDFLOWER, IDR_NIGHTLIGHT, IDR_ONOFFLIGHT, IDR_OXYPOTION, IDR_PALACEBOMB,
        IDR_PALACECAP, IDR_PALACEGATE, IDR_PALACEKEY, IDR_PENT, IDR_POTION, IDR_SKELRAISE,
        IDR_SPECIAL_POTION, IDR_SPIKETRAP, IDR_STAFFER2, IDR_STEPTRAP, IDR_SWAMPARM,
        IDR_SWAMPSPAWN, IDR_SWAMPWHISP, IDR_TORCH, IDR_USETRAP, IID_AREA18_BONE,
    },
    legacy::{action, profession, worn_slot},
    map::{MapFlags, MapGrid},
    player::{PlayerActionCode, PlayerRuntime, QueuedAction},
    spell::{
        IDR_INFRARED, IDR_NONOMAGIC, IDR_OXYGEN, IDR_POISON0, IDR_POISON1, IDR_POISON2, IDR_UWTALK,
    },
    tick::TICKS_PER_SECOND,
};

fn mine_door_neighbor_points(x: usize, y: usize) -> [(usize, usize); 8] {
    [
        (x + 1, y),
        (x - 1, y),
        (x, y + 1),
        (x, y - 1),
        (x + 1, y + 1),
        (x + 1, y - 1),
        (x - 1, y + 1),
        (x - 1, y - 1),
    ]
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
    }
}

fn item(id: u32, flags: ItemFlags) -> Item {
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
        content_id: 0,
        driver: 0,
        driver_data: Vec::new(),
        serial: 0,
    }
}
