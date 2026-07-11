use super::*;
use crate::entity::CharacterValue as V;
use crate::item_driver::{
    IID_MISSIONCHEST, IID_MISSIONDOOR1, IID_MISSIONDOOR2, IID_MISSIONENTRY, IID_MISSIONFIGHTER,
};
use crate::player::{MissionPpd, SingleMission};
use crate::world::npc::area32::mission_start::{
    build_fighter_stat_values, mission_status_lines, special_item_tier_for_level,
    MissionStartError, MISSION_FIGHTER_DATA,
};

fn mission_marker(id: u32, fighter_kind: u8) -> Item {
    let mut marker = item(id, ItemFlags::USED);
    marker.template_id = IID_MISSIONFIGHTER;
    marker.driver_data = vec![fighter_kind];
    marker
}

#[test]
fn build_fighter_stat_values_only_touches_raisable_nonzero_markers() {
    // C `build_fighter`'s `V_HAND`/`V_ATTACK` branch: `max(1, diff)`.
    let markers = vec![0i16; crate::entity::CHARACTER_VALUE_COUNT];
    let mut markers = markers;
    markers[V::Hp as usize] = 10; // C: max(10, diff-15)
    markers[V::Hand as usize] = 1; // C: max(1, diff)
    markers[V::ArmorSkill as usize] = 1; // C: max(1, (diff/10)*10)
    markers[V::Mana as usize] = 0; // untouched: marker is zero

    let scaled = build_fighter_stat_values(&markers, 42);
    assert_eq!(scaled[V::Hp as usize], 27); // max(10, 42-15)
    assert_eq!(scaled[V::Hand as usize], 42); // max(1, 42)
    assert_eq!(scaled[V::ArmorSkill as usize], 40); // (42/10)*10
    assert_eq!(scaled[V::Mana as usize], 0);
}

#[test]
fn build_fighter_stat_values_caps_at_250() {
    let mut markers = vec![0i16; crate::entity::CHARACTER_VALUE_COUNT];
    markers[V::Hand as usize] = 1;
    let scaled = build_fighter_stat_values(&markers, 1000);
    assert_eq!(scaled[V::Hand as usize], 250);
}

#[test]
fn special_item_tier_for_level_matches_c_ladder_boundaries() {
    assert_eq!(special_item_tier_for_level(1), (3, 1));
    assert_eq!(special_item_tier_for_level(9), (3, 1));
    assert_eq!(special_item_tier_for_level(10), (4, 10));
    assert_eq!(special_item_tier_for_level(73), (18, 90));
    assert_eq!(special_item_tier_for_level(74), (20, 90));
    assert_eq!(special_item_tier_for_level(500), (20, 90));
}

#[test]
fn mission_status_lines_renders_title_and_padded_slots() {
    let ppd = MissionPpd {
        kill_easy: [0, 3],
        kill_boss: [0, 1],
        ..MissionPpd::default()
    };
    let lines = mission_status_lines(&ppd, "Stolen Documents", &MISSION_FIGHTER_DATA[0]);
    assert_eq!(lines[0], "#30Stolen Documents");
    assert_eq!(lines[1], "#40- Kill 3 Thief Apprentices");
    assert_eq!(lines[2], "#50- Kill 1 Sacewan");
    // Padded blank HUD slots up to line 8: title(3) + easy(4) + boss(5) +
    // blank 6/7/8 = 6 lines total.
    assert_eq!(lines.last().unwrap(), "#80");
    assert_eq!(lines.len(), 6);
}

#[test]
fn mission_status_lines_singular_kill_count_has_no_trailing_s() {
    let ppd = MissionPpd {
        kill_easy: [0, 1],
        ..MissionPpd::default()
    };
    let lines = mission_status_lines(&ppd, "Stolen Documents", &MISSION_FIGHTER_DATA[0]);
    assert_eq!(lines[1], "#40- Kill 1 Thief Apprentice");
}

#[test]
fn plan_start_mission_rejects_when_every_slice_is_busy() {
    let mut world = World::default();
    // Area 0 (thief) has 6 slices (n=0..=5); occupy each with a player.
    for n in 0..6u16 {
        let fx: u16 = 1;
        let fy: u16 = 1 + n * 41;
        assert!(world.spawn_character(
            {
                let mut player = character(100 + u32::from(n));
                player.flags.insert(CharacterFlags::PLAYER);
                player
            },
            usize::from(fx),
            usize::from(fy),
        ));
    }
    let mut ppd = MissionPpd {
        sm: [
            SingleMission {
                mission_type: 1,
                mdidx: 0,
                difficulty: 42,
            },
            SingleMission::default(),
            SingleMission::default(),
        ],
        ..MissionPpd::default()
    };
    let result = world.plan_start_mission(0, &mut ppd);
    assert_eq!(result.err(), Some(MissionStartError::AllSlicesBusy));
    // C never mutates `ppd` on the busy-refusal path.
    assert_eq!(ppd.active, 0);
    assert_eq!(ppd.mcnt, 0);
}

#[test]
fn plan_start_mission_spawns_easy_fighter_and_wires_entry_door_and_chest() {
    let mut world = World::default();
    // Thief mission (mdidx=0, area=0): slice n=0 spans x=1..=41, y=1..=41.
    let mut easy = mission_marker(1, 1);
    assert!(world.map.set_item_map(&mut easy, 5, 5));
    world.add_item(easy);

    let mut entrance = item(2, ItemFlags::USED);
    entrance.template_id = IID_MISSIONENTRY;
    assert!(world.map.set_item_map(&mut entrance, 6, 6));
    world.add_item(entrance);

    let mut door1 = item(3, ItemFlags::DOOR);
    door1.template_id = IID_MISSIONDOOR1;
    door1.driver_data = vec![0u8; 40];
    assert!(world.map.set_item_map(&mut door1, 7, 7));
    world.add_item(door1);

    let mut door2 = item(4, ItemFlags::DOOR);
    door2.template_id = IID_MISSIONDOOR2;
    door2.driver_data = vec![0u8; 40];
    assert!(world.map.set_item_map(&mut door2, 8, 8));
    world.add_item(door2);

    let mut chest = item(5, ItemFlags::USED);
    chest.template_id = IID_MISSIONCHEST;
    chest.driver_data = vec![0u8; 14];
    assert!(world.map.set_item_map(&mut chest, 9, 9));
    world.add_item(chest);

    let mut ppd = MissionPpd {
        sm: [
            SingleMission {
                mission_type: 1,
                mdidx: 0,
                difficulty: 42,
            },
            SingleMission::default(),
            SingleMission::default(),
        ],
        ..MissionPpd::default()
    };
    // C: `itemname` must be `Some` (the thief mission has one) for the
    // chest's key to be wired at all.
    assert!(MISSION_FIGHTER_DATA[0].itemname.is_some());

    let plan = world
        .plan_start_mission(0, &mut ppd)
        .expect("slice 0 should be free");

    assert_eq!(plan.entry, (6, 6));
    assert_eq!(plan.fighters.len(), 1);
    let fighter = &plan.fighters[0];
    assert_eq!((fighter.x, fighter.y), (5, 5));
    assert_eq!(fighter.diff, 42 / 3);
    assert_eq!(fighter.name, "Thief Apprentice");
    assert_eq!(fighter.temp, "mis_warrior");
    assert_eq!(fighter.fighter_kind, 1);
    assert_eq!(fighter.key_id, 0);
    assert!(!fighter.has_special_item);

    assert_eq!(ppd.active, 1);
    assert_eq!(ppd.solved, 0);
    assert_eq!(ppd.md_idx, 0);
    assert_eq!(ppd.mcnt, 1);
    assert_eq!(ppd.kill_easy, [0, 1]);
    assert_eq!(ppd.kill_normal, [0, 0]);
    assert_eq!(ppd.find_item, [0, 1]); // chest counted since itemname is Some

    let key_id = crate::item_driver::make_item_id(crate::item_driver::DEV_ID_MISSION, 3);
    let door1 = &world.items[&ItemId(3)];
    assert_eq!(
        u32::from(door1.driver_data[1])
            | (u32::from(door1.driver_data[2]) << 8)
            | (u32::from(door1.driver_data[3]) << 16)
            | (u32::from(door1.driver_data[4]) << 24),
        key_id
    );
    let door2 = &world.items[&ItemId(4)];
    assert_eq!(
        u32::from(door2.driver_data[1])
            | (u32::from(door2.driver_data[2]) << 8)
            | (u32::from(door2.driver_data[3]) << 16)
            | (u32::from(door2.driver_data[4]) << 24),
        key_id + 1
    );
    let chest = &world.items[&ItemId(5)];
    assert_eq!(
        u32::from(chest.driver_data[1])
            | (u32::from(chest.driver_data[2]) << 8)
            | (u32::from(chest.driver_data[3]) << 16)
            | (u32::from(chest.driver_data[4]) << 24),
        key_id + 2
    );
    // C `it[in].sprite = 0` on the entry marker.
    assert_eq!(world.items[&ItemId(2)].sprite, 0);
}

#[test]
fn plan_start_mission_removes_junk_items_and_non_player_characters_in_the_slice() {
    let mut world = World::default();
    assert!(world.spawn_character(character(50), 5, 5));

    let mut junk = item(6, ItemFlags::TAKE);
    assert!(world.map.set_item_map(&mut junk, 6, 5));
    world.add_item(junk);

    let mut ppd = MissionPpd {
        sm: [
            SingleMission {
                mission_type: 1,
                mdidx: 0,
                difficulty: 42,
            },
            SingleMission::default(),
            SingleMission::default(),
        ],
        ..MissionPpd::default()
    };
    let plan = world
        .plan_start_mission(0, &mut ppd)
        .expect("slice 0 should be free");
    assert!(plan.fighters.is_empty());
    assert!(!world.characters.contains_key(&CharacterId(50)));
    assert!(!world.items.contains_key(&ItemId(6)));
}
