use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_MISSIONGIVE, NT_CHAR, NT_GIVE};
use crate::entity::CharacterValue;
use crate::player::{MissionPpd, SingleMission};
use crate::world::npc::area32::governor::{
    MissionGiveOutcomeEvent, MissionGivePlayerFacts, MissionGiverDriverData, MIS_REWARDS,
    SPECIAL_OFFER_SLOT, SPECIAL_OFFER_VIEW_EXTENSION_TICKS,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn mission_giver_npc(id: u32) -> Character {
    let mut giver = character(id);
    giver.name = "Mister Jones".into();
    giver.driver = CDR_MISSIONGIVE;
    giver.driver_state = Some(CharacterDriverState::MissionGiver(
        MissionGiverDriverData::default(),
    ));
    giver
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(player_id: CharacterId, ppd: MissionPpd) -> HashMap<CharacterId, MissionGivePlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, MissionGivePlayerFacts { ppd });
    map
}

fn find_update(events: &[MissionGiveOutcomeEvent], player_id: CharacterId) -> Option<MissionPpd> {
    events.iter().find_map(|event| match event {
        MissionGiveOutcomeEvent::UpdatePpd { player_id: id, ppd } if *id == player_id => Some(*ppd),
        _ => None,
    })
}

#[test]
fn state0_greets_and_advances_to_state1_when_no_job() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_mission_giver_actions(
        &facts(CharacterId(2), MissionPpd::default()),
        32,
        1000,
    );
    let ppd = find_update(&events, CharacterId(2)).expect("ppd update expected");
    assert_eq!(ppd.missiongive_state, 1);
    assert_eq!(ppd.lastseenmissiongiver, 1000);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Looking for a job")));
}

#[test]
fn state0_resets_after_30_seconds_of_absence() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let stale_ppd = MissionPpd {
        missiongive_state: 2,
        lastseenmissiongiver: 100,
        ..MissionPpd::default()
    };
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), stale_ppd), 32, 200);
    let ppd = find_update(&events, CharacterId(2)).expect("ppd update expected");
    // still-stale state (2 = waiting) would be a silent no-op, but the
    // 30s-absence reset (C `missions.c:1433-1435`) forces state back to 0
    // first, which greets again and advances to 1.
    assert_eq!(ppd.missiongive_state, 1);
}

#[test]
fn state1_offer_mission_rolls_three_distinct_jobs_and_advances_to_state2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let ppd = MissionPpd {
        missiongive_state: 1,
        lastseenmissiongiver: 1000,
        ..MissionPpd::default()
    };
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).expect("ppd update expected");
    assert_eq!(new_ppd.missiongive_state, 2);
    // C `offer_mission`'s dedup loop guarantees the three rolled mdidx are
    // pairwise distinct (`missions.c:664-665`).
    assert_ne!(new_ppd.sm[0].mdidx, new_ppd.sm[1].mdidx);
    assert_ne!(new_ppd.sm[1].mdidx, new_ppd.sm[2].mdidx);
    assert_ne!(new_ppd.sm[0].mdidx, new_ppd.sm[2].mdidx);
    for slot in new_ppd.sm {
        assert_eq!(slot.mission_type, 1);
        assert!((0..7).contains(&slot.mdidx));
        assert!(slot.difficulty >= new_ppd.dif_kill);
    }
    let texts = world.drain_pending_system_texts();
    assert_eq!(
        texts
            .iter()
            .filter(|text| text.character_id == CharacterId(2))
            .count(),
        3
    );
}

#[test]
fn text_job_alpha_shows_offered_job_details() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
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
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "job alpha");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(find_update(&events, CharacterId(2)).is_some());
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Stolen Documents")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("bold thief named 'Sacewan'")));
}

#[test]
fn text_increase_and_decrease_adjust_dif_kill() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "increase");
    }
    let events = world.process_mission_giver_actions(
        &facts(CharacterId(2), MissionPpd::default()),
        32,
        1000,
    );
    let ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(ppd.dif_kill, 10);

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .driver_messages
        .clear();
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "decrease");
    }
    let events = world.process_mission_giver_actions(
        &facts(CharacterId(2), MissionPpd::default()),
        32,
        1000,
    );
    let ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(ppd.dif_kill, 0);
}

#[test]
fn text_reset_me_wipes_ppd_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    let ppd = MissionPpd {
        points: 500,
        dif_kill: 300,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "reset me");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(new_ppd, MissionPpd::default());
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn ibuy_gold1_deducts_points_and_gives_gold_directly() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let reward_index = MIS_REWARDS.iter().position(|r| r.code == "GOLD1").unwrap();
    let reward = &MIS_REWARDS[reward_index];
    assert_eq!(reward.value, 10);

    let ppd = MissionPpd {
        points: 20,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "ibuy GOLD1");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(new_ppd.points, 10);
    // GOLD/MEXP rewards are applied directly (no ZoneLoader needed), so no
    // `GiveItemReward` event should be queued for this code.
    assert!(!events
        .iter()
        .any(|event| matches!(event, MissionGiveOutcomeEvent::GiveItemReward { .. })));
    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.gold, 10 * 500);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Here you go")));
}

#[test]
fn ibuy_ring_reward_queues_a_give_item_event() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let reward_index = MIS_REWARDS.iter().position(|r| r.code == "LNROS").unwrap();
    let ppd = MissionPpd {
        points: 100,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "ibuy LNROS");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    // Points are *not* deducted from the `UpdatePpd` snapshot for the
    // generic item-template path - the server-side `apply_
    // mission_giver_events` only deducts them once `ZoneLoader` confirms
    // the item was actually created and handed over.
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(new_ppd.points, 100);
    assert!(events.iter().any(|event| matches!(
        event,
        MissionGiveOutcomeEvent::GiveItemReward { reward_index: idx, .. } if *idx == reward_index
    )));
}

#[test]
fn ibuy_reports_insufficient_points_without_any_event() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        points: 0,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "ibuy GOLD1");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(!events
        .iter()
        .any(|event| matches!(event, MissionGiveOutcomeEvent::GiveItemReward { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("costs 10 points, but you only have 0 points")));
}

#[test]
fn show_offer_lists_rewards_around_current_points() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.driver_state = Some(CharacterDriverState::MissionGiver(MissionGiverDriverData {
            spec_cost: 777,
            ..MissionGiverDriverData::default()
        }));
    }

    let ppd = MissionPpd {
        points: 300,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "offer");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(find_update(&events, CharacterId(2)).is_some());
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Code Cost Description")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("You have: 300 points.")));
    // C `case 11:`'s trailing "I also have a special offer..." teaser
    // (`missions.c:1588-1589`).
    let area_texts = world.drain_pending_area_texts();
    assert!(
        area_texts
            .iter()
            .any(|text| text.message.contains("special offer")
                && text.message.contains("777 points."))
    );
}

#[test]
fn special_offer_extends_visibility_and_queues_show_event() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    let mut giver = mission_giver_npc(1);
    giver.inventory[SPECIAL_OFFER_SLOT] = Some(ItemId(900));
    giver.driver_state = Some(CharacterDriverState::MissionGiver(MissionGiverDriverData {
        spec_cost: 500,
        next_spec: 100,
        ..MissionGiverDriverData::default()
    }));
    assert!(world.spawn_character(giver, 10, 10));
    let mut offer_item = item(900, ItemFlags::empty());
    offer_item.name = "Special Sword".into();
    offer_item.carried_by = Some(CharacterId(1));
    world.add_item(offer_item);
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "special offer");
    }
    let events = world.process_mission_giver_actions(
        &facts(CharacterId(2), MissionPpd::default()),
        32,
        1000,
    );
    assert!(events.iter().any(|event| matches!(
        event,
        MissionGiveOutcomeEvent::ShowSpecialOffer { player_id, npc_id }
            if *player_id == CharacterId(2) && *npc_id == CharacterId(1)
    )));
    // C `dat->next_spec = max(dat->next_spec, ticker + TICKS*60*5);`
    // (`missions.c:1634`).
    match world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .driver_state
        .as_ref()
    {
        Some(CharacterDriverState::MissionGiver(data)) => {
            assert_eq!(
                data.next_spec,
                BASELINE_TICK + SPECIAL_OFFER_VIEW_EXTENSION_TICKS
            );
        }
        _ => panic!("expected MissionGiver driver state"),
    }
    // The item still sits in the governor's own slot - only the visibility
    // window changed.
    let giver = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(giver.inventory[SPECIAL_OFFER_SLOT], Some(ItemId(900)));
}

#[test]
fn buy_special_offer_succeeds_and_transfers_the_item() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    let mut giver = mission_giver_npc(1);
    giver.inventory[SPECIAL_OFFER_SLOT] = Some(ItemId(900));
    giver.driver_state = Some(CharacterDriverState::MissionGiver(MissionGiverDriverData {
        spec_cost: 500,
        next_spec: 100,
        ..MissionGiverDriverData::default()
    }));
    assert!(world.spawn_character(giver, 10, 10));
    let mut offer_item = item(900, ItemFlags::empty());
    offer_item.name = "Special Sword".into();
    offer_item.carried_by = Some(CharacterId(1));
    world.add_item(offer_item);
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        points: 600,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "buy the special offer");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    // C `ppd->points -= dat->spec_cost;` (`missions.c:1647`).
    assert_eq!(new_ppd.points, 100);
    // C `ch[cn].item[30] = 0;` (`missions.c:1646`).
    let giver = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(giver.inventory[SPECIAL_OFFER_SLOT], None);
    // C `dat->next_spec = 0;` (`missions.c:1648`) - forces an immediate
    // reroll on the next driver call.
    match giver.driver_state.as_ref() {
        Some(CharacterDriverState::MissionGiver(data)) => assert_eq!(data.next_spec, 0),
        _ => panic!("expected MissionGiver driver state"),
    }
    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.cursor_item, Some(ItemId(900)));
    let item = world.items.get(&ItemId(900)).unwrap();
    assert_eq!(item.carried_by, Some(CharacterId(2)));
}

#[test]
fn buy_special_offer_refuses_when_too_poor() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    let mut giver = mission_giver_npc(1);
    giver.inventory[SPECIAL_OFFER_SLOT] = Some(ItemId(900));
    giver.driver_state = Some(CharacterDriverState::MissionGiver(MissionGiverDriverData {
        spec_cost: 500,
        next_spec: 100,
        ..MissionGiverDriverData::default()
    }));
    assert!(world.spawn_character(giver, 10, 10));
    let mut offer_item = item(900, ItemFlags::empty());
    offer_item.carried_by = Some(CharacterId(1));
    world.add_item(offer_item);
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        points: 100,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "buy the special offer");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(
        new_ppd.points, 100,
        "unaffordable buy leaves points untouched"
    );
    let giver = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(
        giver.inventory[SPECIAL_OFFER_SLOT],
        Some(ItemId(900)),
        "item stays with the governor"
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Sorry, you can't afford it.")));
}

#[test]
fn buy_special_offer_reports_no_space_when_inventory_is_full() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    let mut giver = mission_giver_npc(1);
    giver.inventory[SPECIAL_OFFER_SLOT] = Some(ItemId(900));
    giver.driver_state = Some(CharacterDriverState::MissionGiver(MissionGiverDriverData {
        spec_cost: 500,
        next_spec: 100,
        ..MissionGiverDriverData::default()
    }));
    assert!(world.spawn_character(giver, 10, 10));
    let mut offer_item = item(900, ItemFlags::empty());
    offer_item.carried_by = Some(CharacterId(1));
    world.add_item(offer_item);
    let mut buyer = player(2, "Godmode");
    buyer.cursor_item = Some(ItemId(901));
    for slot in buyer
        .inventory
        .iter_mut()
        .skip(crate::legacy::INVENTORY_START_INVENTORY)
    {
        *slot = Some(ItemId(999));
    }
    assert!(world.spawn_character(buyer, 12, 10));
    world.add_item(item(901, ItemFlags::empty()));

    let ppd = MissionPpd {
        points: 600,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "buy the special offer");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(new_ppd.points, 600, "failed buy leaves points untouched");
    let giver = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(giver.inventory[SPECIAL_OFFER_SLOT], Some(ItemId(900)));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("You don't have any space in your inventory, dude.")));
}

#[test]
fn give_item_is_always_handed_back() {
    let mut world = World::default();
    let mut giver = mission_giver_npc(1);
    giver.cursor_item = Some(ItemId(50));
    world.add_character(giver);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_mission_giver_actions(&HashMap::new(), 32, 1000);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn text_accept_job_alpha_refuses_when_job_was_never_offered() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "accept job alpha");
    }
    let events = world.process_mission_giver_actions(
        &facts(CharacterId(2), MissionPpd::default()),
        32,
        1000,
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, MissionGiveOutcomeEvent::SpawnMissionFighters { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I haven't offered you that job yet.")));
}

#[test]
fn text_accept_job_alpha_starts_the_mission_and_teleports_the_player() {
    let mut world = World::default();
    // The governor NPC/player stand well outside every area/slice
    // combination the busy scan can pick (`x/y in 1..=246`, `MAX_MAP` is
    // 256) so the talking player isn't seen as an occupant of their own
    // new instance.
    world.map.tile_mut(252, 250).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 250, 250));
    assert!(world.spawn_character(player(2, "Godmode"), 252, 250));

    // Thief mission (mdidx=0, area=0): slice n=0 spans x=1..=41, y=1..=41.
    let mut entrance = item(90, ItemFlags::USED);
    entrance.template_id = crate::item_driver::IID_MISSIONENTRY;
    assert!(world.map.set_item_map(&mut entrance, 30, 30));
    world.add_item(entrance);

    let ppd = MissionPpd {
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
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "accept job alpha");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).expect("ppd update expected");
    assert_eq!(new_ppd.active, 1);
    assert_eq!(new_ppd.mcnt, 1);
    assert!(events
        .iter()
        .any(|event| matches!(event, MissionGiveOutcomeEvent::SpawnMissionFighters { .. })));

    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((player.x, player.y), (30, 30));

    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "#30Stolen Documents"));
}

// ---- `CTPOT` custom stat potion multi-turn flow (`missions.c:1202-1211,
// 1652-1739`) ----

#[test]
fn ibuy_ctpot_deducts_points_and_starts_the_skill_naming_flow() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let reward_index = MIS_REWARDS.iter().position(|r| r.code == "CTPOT").unwrap();
    assert_eq!(MIS_REWARDS[reward_index].value, 250);

    let ppd = MissionPpd {
        points: 300,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "ibuy CTPOT");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(new_ppd.points, 50);
    assert_eq!(new_ppd.statowed, 1);
    assert_eq!(new_ppd.statcnt, 0);
    assert_eq!(new_ppd.stat, [0, 0, 0]);
    // C `mission_give_reward`'s `CTPOT` branch `return`s before the
    // generic "Here you go, ..." trailer, and no `GiveItemReward` event
    // is queued (unlike every other reward code).
    assert!(!events
        .iter()
        .any(|event| matches!(event, MissionGiveOutcomeEvent::GiveItemReward { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(
        |text| text.message.contains("One custom stat potion coming up.")
            && text.message.contains("one skill")
            && text.message.contains("two skills")
            && text.message.contains("three skills")
    ));
}

#[test]
fn ctpot_without_enough_points_reports_insufficient_points() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        points: 100,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "ibuy CTPOT");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(new_ppd.statowed, 0, "too poor to buy - no potion owed");
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("costs 250 points, but you only have 100 points")));
}

#[test]
fn text_one_skill_without_owed_potion_reports_error() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "one skill");
    }
    let events = world.process_mission_giver_actions(
        &facts(CharacterId(2), MissionPpd::default()),
        32,
        1000,
    );
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(new_ppd.statcnt, 0);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("You did not buy a stat potion.")));
}

#[test]
fn text_two_skills_sets_statcnt_and_prompts_for_names() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        statowed: 1,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "two skills");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(new_ppd.statcnt, 2);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Alright, a two-stat potion it will be.")
        && text.message.contains("one skill per line")));
}

#[test]
fn text_one_skill_name_finalizes_immediately_and_queues_give_event() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        statowed: 1,
        statcnt: 1,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        // C `find_skill_text`'s leading skip chain expects the actual
        // upstream `"<Name> says: <text>"` NT_TEXT payload shape (same
        // precedent as `world::strategy_worker_trim_command_prefix`'s own
        // tests) - plain unprefixed text would have both words eaten by
        // the skip itself.
        giver.push_driver_text_message(CharacterId(2), "Godmode says: attack skill");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(events.iter().any(|event| matches!(
        event,
        MissionGiveOutcomeEvent::GiveCustomStatPotion {
            player_id,
            stat,
            statcnt: 1,
            ..
        } if *player_id == CharacterId(2) && stat[0] == CharacterValue::Attack as i32
    )));
}

#[test]
fn text_two_skill_names_finalize_on_the_second_and_prompt_after_the_first() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        statowed: 1,
        statcnt: 2,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "Godmode says: attack skill");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(!events
        .iter()
        .any(|event| matches!(event, MissionGiveOutcomeEvent::GiveCustomStatPotion { .. })));
    let after_first = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(after_first.stat, [CharacterValue::Attack as i32, 0, 0]);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Very well, the first skill will be Attack.")));

    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .driver_messages
        .clear();
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "Godmode says: parry skill");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), after_first), 32, 1000);
    assert!(events.iter().any(|event| matches!(
        event,
        MissionGiveOutcomeEvent::GiveCustomStatPotion {
            stat,
            statcnt: 2,
            ..
        } if stat[0] == CharacterValue::Attack as i32 && stat[1] == CharacterValue::Parry as i32
    )));
}

#[test]
fn unrecognized_skill_text_is_ignored_while_a_potion_is_owed() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        statowed: 1,
        statcnt: 1,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "Godmode says: banana skill");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(!events
        .iter()
        .any(|event| matches!(event, MissionGiveOutcomeEvent::GiveCustomStatPotion { .. })));
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(
        new_ppd.stat,
        [0, 0, 0],
        "unrecognized skill leaves stat[] untouched"
    );
}

#[test]
fn skill_text_requires_the_word_skill() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        statowed: 1,
        statcnt: 1,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        // "attack" alone (no " skill" substring) must not match - C
        // `find_skill_text` bails out before the prefix table entirely.
        giver.push_driver_text_message(CharacterId(2), "Godmode says: attack");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(!events
        .iter()
        .any(|event| matches!(event, MissionGiveOutcomeEvent::GiveCustomStatPotion { .. })));
}

#[test]
fn skill_text_speed_maps_to_base_speed_not_speed_skill() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        statowed: 1,
        statcnt: 1,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        // C `find_skill_text` (`missions.c:387-389`) maps "speed" to
        // `V_SPEED`, not `V_SPEEDSKILL` - a real, deliberate-looking quirk.
        giver.push_driver_text_message(CharacterId(2), "Godmode says: speed skill");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(events.iter().any(|event| matches!(
        event,
        MissionGiveOutcomeEvent::GiveCustomStatPotion { stat, .. }
            if stat[0] == CharacterValue::Speed as i32
    )));
}

#[test]
fn skill_text_lightning_prefix_quirk_matches_truncated_word() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let ppd = MissionPpd {
        statowed: 1,
        statcnt: 1,
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        // C's `strncasecmp(text, "lightning", 8)` only checks 8 of 9
        // characters (`missions.c:420`), so "lightninx skill" (not a real
        // word) still matches, mapping to `V_FLASH`.
        giver.push_driver_text_message(CharacterId(2), "Godmode says: lightninx skill");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(events.iter().any(|event| matches!(
        event,
        MissionGiveOutcomeEvent::GiveCustomStatPotion { stat, .. }
            if stat[0] == CharacterValue::Flash as i32
    )));
}

#[test]
fn skill_text_third_skill_lands_in_the_third_slot() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(mission_giver_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    let mut ppd = MissionPpd {
        statowed: 1,
        statcnt: 3,
        stat: [
            CharacterValue::Attack as i32,
            CharacterValue::Parry as i32,
            0,
        ],
        ..MissionPpd::default()
    };
    if let Some(giver) = world.characters.get_mut(&CharacterId(1)) {
        giver.push_driver_text_message(CharacterId(2), "Godmode says: warcry skill");
    }
    let events = world.process_mission_giver_actions(&facts(CharacterId(2), ppd), 32, 1000);
    assert!(events.iter().any(|event| matches!(
        event,
        MissionGiveOutcomeEvent::GiveCustomStatPotion {
            stat,
            statcnt: 3,
            ..
        } if *stat == [
            CharacterValue::Attack as i32,
            CharacterValue::Parry as i32,
            CharacterValue::Warcry as i32,
        ]
    )));
    // Also sanity-check the `UpdatePpd` snapshot carries the final slot
    // before the server-side finalize event is applied.
    ppd.stat[2] = CharacterValue::Warcry as i32;
    let new_ppd = find_update(&events, CharacterId(2)).unwrap();
    assert_eq!(new_ppd.stat, ppd.stat);
}
