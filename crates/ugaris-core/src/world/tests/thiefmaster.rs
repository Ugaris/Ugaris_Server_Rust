use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    CharacterDriverMessage, FightDriverData, TwoThiefMasterDriverData, CDR_TWOTHIEFMASTER, NT_CHAR,
    NT_GIVE, NT_GOTHIT, NT_TEXT,
};
use crate::item_driver::{
    IID_AREA17_GOLDENLOCKPICK, IID_AREA17_LOCKPICK, IID_AREA17_MERCHANTNOTE1, IID_AREA17_SEWERKEY1,
    IID_AREA17_SEWERKEY2,
};
use crate::world::npc::area17::thiefmaster::{
    TwoThiefMasterOutcomeEvent, TwoThiefMasterPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn thiefmaster_npc(id: u32) -> Character {
    let mut thiefmaster = character(id);
    thiefmaster.name = "Guild Master".into();
    thiefmaster.driver = CDR_TWOTHIEFMASTER;
    thiefmaster.driver_state = Some(CharacterDriverState::TwoThiefMaster(
        TwoThiefMasterDriverData::default(),
    ));
    thiefmaster
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.level = 40;
    player
}

fn facts(
    player_id: CharacterId,
    thief_state: i32,
) -> HashMap<CharacterId, TwoThiefMasterPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        TwoThiefMasterPlayerFacts {
            thief_state,
            thief_last_seen: 0,
            thief_killed: [0; 6],
            quest25_count: 0,
            quest26_count: 0,
        },
    );
    map
}

fn facts_with(
    player_id: CharacterId,
    mutate: impl FnOnce(&mut TwoThiefMasterPlayerFacts),
) -> HashMap<CharacterId, TwoThiefMasterPlayerFacts> {
    let mut map = HashMap::new();
    let mut fact = TwoThiefMasterPlayerFacts {
        thief_state: 0,
        thief_last_seen: 0,
        thief_killed: [0; 6],
        quest25_count: 0,
        quest26_count: 0,
    };
    mutate(&mut fact);
    map.insert(player_id, fact);
    map
}

fn thiefmaster_state(world: &World, thiefmaster_id: CharacterId) -> TwoThiefMasterDriverData {
    match world
        .characters
        .get(&thiefmaster_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::TwoThiefMaster(data)) => data,
        _ => panic!("expected two thiefmaster driver state"),
    }
}

#[test]
fn thiefmaster_state4_greets_new_member_and_advances_to_five() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 4), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("new member")));
    assert_eq!(
        thiefmaster_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn thiefmaster_below_state_four_is_bumped_to_four_first() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // C `if (ppd->thief_state < 4) ppd->thief_state = 4;` runs
    // unconditionally, then the switch reaches `case 4`.
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 0), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 4,
        })
    );
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
}

#[test]
fn thiefmaster_state5_without_lockpick_offers_the_robber_mission() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 5), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 6,
        })
    );
    assert!(events.contains(&TwoThiefMasterOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 25,
    }));
    assert!(events.contains(&TwoThiefMasterOutcomeEvent::QuestClose {
        player_id: CharacterId(2),
        quest: 26,
    }));
    assert!(events.contains(&TwoThiefMasterOutcomeEvent::QuestClose {
        player_id: CharacterId(2),
        quest: 27,
    }));
    assert!(events.contains(&TwoThiefMasterOutcomeEvent::QuestClose {
        player_id: CharacterId(2),
        quest: 28,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("hasn't earned his lockpick")));
}

#[test]
fn thiefmaster_state5_with_lockpick_silently_skips_to_eleven() {
    let mut world = World::default();
    let mut thiefmaster = thiefmaster_npc(1);
    thiefmaster.cursor_item = None;
    world.spawn_character(thiefmaster, 10, 10);
    let mut godmode = player(2, "Godmode");
    let lockpick_id = ItemId(50);
    godmode.inventory[0] = Some(lockpick_id);
    world.spawn_character(godmode, 12, 10);
    let mut lockpick = item(50, ItemFlags::empty());
    lockpick.template_id = IID_AREA17_LOCKPICK;
    lockpick.carried_by = Some(CharacterId(2));
    world.add_item(lockpick);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 5), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 11,
        })
    );
    // Silent skip - no dialogue, no `current_victim`/`thief_last_seen`
    // bookkeeping (C's `didsay` never fires here).
    assert!(world.drain_pending_area_texts().is_empty());
    assert!(!events
        .iter()
        .any(|e| matches!(e, TwoThiefMasterOutcomeEvent::UpdateThiefLastSeen { .. })));
}

#[test]
fn thiefmaster_state9_nags_after_the_cooldown_and_stays_silent_before_it() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let facts_ready = facts_with(CharacterId(2), |f| {
        f.thief_state = 9;
        f.thief_last_seen = 0;
    });
    let events = world.process_two_thiefmaster_actions(&facts_ready, 121, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefLastSeen {
            player_id: CharacterId(2),
            realtime: 121,
        })
    );
    // This line uses `npc_say_bytes` (carries a `COL_STR_LIGHT_BLUE`
    // sentinel), so it lands in the byte-native text queue, not the
    // plain-`String` one - see `World::npc_say_bytes`'s doc comment.
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts
        .iter()
        .any(|t| String::from_utf8_lossy(&t.message).contains("art thou done")));

    // Still on cooldown: no message at all.
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let facts_cooldown = facts_with(CharacterId(2), |f| {
        f.thief_state = 9;
        f.thief_last_seen = 100;
    });
    let events = world.process_two_thiefmaster_actions(&facts_cooldown, 150, 17);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn thiefmaster_state10_with_no_kills_scolds_and_resets_to_five() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 10), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
    assert!(!events
        .iter()
        .any(|e| matches!(e, TwoThiefMasterOutcomeEvent::Quest25Reward { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("disappointed")));
}

#[test]
fn thiefmaster_state10_with_kills_grants_scaled_exp_and_advances_to_eleven() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    // score = killed[0]*1 = 5 -> tier "score < 10" -> val = 5000, no name
    // substitution.
    let facts_score = facts_with(CharacterId(2), |f| {
        f.thief_state = 10;
        f.thief_killed[0] = 5;
    });
    let events = world.process_two_thiefmaster_actions(&facts_score, 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 11,
        })
    );
    assert!(events.contains(&TwoThiefMasterOutcomeEvent::Quest25Reward {
        player_id: CharacterId(2),
    }));
    // level 40 -> level_value/5 comfortably exceeds 5000, so the exp
    // grant isn't capped and the character's exp increases by exactly
    // the tier value (quest25_count is 0, so `scale_exp` is a no-op).
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().exp, 5000);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("limited abilities allowed")));
}

#[test]
fn thiefmaster_state10_robber_baron_kill_always_hails_the_slayer() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let facts_baron = facts_with(CharacterId(2), |f| {
        f.thief_state = 10;
        f.thief_killed[5] = 1;
    });
    world.process_two_thiefmaster_actions(&facts_baron, 0, 17);
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().exp, 20000);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("robber slayer")));
}

#[test]
fn thiefmaster_state11_without_lockpick_resets_to_five() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 11), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("lost thine lockpick")));
}

#[test]
fn thiefmaster_state11_with_sewerkey1_silently_skips_to_fifteen() {
    let mut world = World::default();
    world.spawn_character(thiefmaster_npc(1), 10, 10);
    let mut godmode = player(2, "Godmode");
    godmode.inventory[0] = Some(ItemId(50));
    godmode.inventory[1] = Some(ItemId(51));
    world.spawn_character(godmode, 12, 10);
    let mut lockpick = item(50, ItemFlags::empty());
    lockpick.template_id = IID_AREA17_LOCKPICK;
    lockpick.carried_by = Some(CharacterId(2));
    world.add_item(lockpick);
    let mut sewerkey1 = item(51, ItemFlags::empty());
    sewerkey1.template_id = IID_AREA17_SEWERKEY1;
    sewerkey1.carried_by = Some(CharacterId(2));
    world.add_item(sewerkey1);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 11), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 15,
        })
    );
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn thiefmaster_state11_with_only_lockpick_offers_the_burndown_mission() {
    let mut world = World::default();
    let mut godmode = player(2, "Godmode");
    godmode.inventory[0] = Some(ItemId(50));
    world.spawn_character(thiefmaster_npc(1), 10, 10);
    world.spawn_character(godmode, 12, 10);
    let mut lockpick = item(50, ItemFlags::empty());
    lockpick.template_id = IID_AREA17_LOCKPICK;
    lockpick.carried_by = Some(CharacterId(2));
    world.add_item(lockpick);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 11), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 12,
        })
    );
    assert!(events.contains(&TwoThiefMasterOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 26,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("punish a merchant")));
}

#[test]
fn thiefmaster_state13_and_17_and_19_and_21_are_silent_waiting_states() {
    for state in [13, 17, 19, 21] {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
            thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
        }
        let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), state), 0, 17);
        assert!(events.is_empty(), "state {state} should be silent");
        assert!(world.drain_pending_area_texts().is_empty());
    }
}

#[test]
fn thiefmaster_state14_burndown_reward_grants_exp_and_a_sewer_key() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let facts_score = facts_with(CharacterId(2), |f| {
        f.thief_state = 14;
        f.thief_killed[0] = 20;
    });
    let events = world.process_two_thiefmaster_actions(&facts_score, 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 15,
        })
    );
    assert!(events.contains(&TwoThiefMasterOutcomeEvent::Quest26Reward {
        player_id: CharacterId(2),
    }));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().exp, 10000);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("nice fire")));
    assert!(texts
        .iter()
        .any(|t| t.message.contains("opens most of the doors")));
}

#[test]
fn thiefmaster_state18_with_palacekey3_silently_skips_to_twenty() {
    let mut world = World::default();
    let mut godmode = player(2, "Godmode");
    godmode.inventory[0] = Some(ItemId(50));
    godmode.inventory[1] = Some(ItemId(51));
    godmode.inventory[2] = Some(ItemId(52));
    godmode.inventory[3] = Some(ItemId(53));
    world.spawn_character(thiefmaster_npc(1), 10, 10);
    world.spawn_character(godmode, 12, 10);
    let mut lockpick = item(50, ItemFlags::empty());
    lockpick.template_id = IID_AREA17_LOCKPICK;
    lockpick.carried_by = Some(CharacterId(2));
    world.add_item(lockpick);
    let mut sewerkey1 = item(51, ItemFlags::empty());
    sewerkey1.template_id = IID_AREA17_SEWERKEY1;
    sewerkey1.carried_by = Some(CharacterId(2));
    world.add_item(sewerkey1);
    let mut sewerkey2 = item(52, ItemFlags::empty());
    sewerkey2.template_id = IID_AREA17_SEWERKEY2;
    sewerkey2.carried_by = Some(CharacterId(2));
    world.add_item(sewerkey2);
    let mut palacekey3 = item(53, ItemFlags::empty());
    palacekey3.template_id = crate::item_driver::IID_AREA17_PALACEKEY3;
    palacekey3.carried_by = Some(CharacterId(2));
    world.add_item(palacekey3);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 18), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 20,
        })
    );
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn thiefmaster_state20_all_items_present_finishes_the_chain() {
    let mut world = World::default();
    let mut godmode = player(2, "Godmode");
    godmode.inventory[0] = Some(ItemId(50));
    godmode.inventory[1] = Some(ItemId(51));
    godmode.inventory[2] = Some(ItemId(52));
    godmode.inventory[3] = Some(ItemId(53));
    world.spawn_character(thiefmaster_npc(1), 10, 10);
    world.spawn_character(godmode, 12, 10);
    let mut lockpick = item(50, ItemFlags::empty());
    lockpick.template_id = IID_AREA17_LOCKPICK;
    lockpick.carried_by = Some(CharacterId(2));
    world.add_item(lockpick);
    let mut sewerkey1 = item(51, ItemFlags::empty());
    sewerkey1.template_id = IID_AREA17_SEWERKEY1;
    sewerkey1.carried_by = Some(CharacterId(2));
    world.add_item(sewerkey1);
    let mut sewerkey2 = item(52, ItemFlags::empty());
    sewerkey2.template_id = IID_AREA17_SEWERKEY2;
    sewerkey2.carried_by = Some(CharacterId(2));
    world.add_item(sewerkey2);
    let mut palacekey3 = item(53, ItemFlags::empty());
    palacekey3.template_id = crate::item_driver::IID_AREA17_PALACEKEY3;
    palacekey3.carried_by = Some(CharacterId(2));
    world.add_item(palacekey3);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 20), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 21,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("enjoying thine stay")));
}

#[test]
fn thiefmaster_repeat_command_resets_state_by_range() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 12), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 11,
        })
    );
}

#[test]
fn thiefmaster_i_am_done_transitions_state_nine_to_ten() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("i am done".to_string()),
        });
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 9), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 10,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Now, let's see")));
}

#[test]
fn thiefmaster_i_am_done_outside_state_nine_replies_hu() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("i am done".to_string()),
        });
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 4), 0, 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Hu?")));
}

#[test]
fn thiefmaster_greeting_qa_reply_substitutes_speaker_name() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("hello".to_string()),
        });
    }
    world.process_two_thiefmaster_actions(&facts(CharacterId(2), 0), 0, 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Hello, Godmode!")));
}

#[test]
fn thiefmaster_quest27_give_note_at_state17_completes_and_hands_over_sewerkey2() {
    let mut world = World::default();
    let mut thiefmaster = thiefmaster_npc(1);
    thiefmaster.cursor_item = Some(ItemId(50));
    world.add_character(thiefmaster);
    let mut note = item(50, ItemFlags::empty());
    note.template_id = IID_AREA17_MERCHANTNOTE1;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_GIVE, 2, 50, 0);
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 17), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 18,
        })
    );
    assert!(events.contains(&TwoThiefMasterOutcomeEvent::Quest27Done {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("agreement I wanted")));
    assert!(!world.items.contains_key(&ItemId(50)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}

#[test]
fn thiefmaster_quest28_give_golden_lockpick_at_state19_completes() {
    let mut world = World::default();
    let mut thiefmaster = thiefmaster_npc(1);
    thiefmaster.cursor_item = Some(ItemId(50));
    world.add_character(thiefmaster);
    let mut pick = item(50, ItemFlags::empty());
    pick.template_id = IID_AREA17_GOLDENLOCKPICK;
    pick.carried_by = Some(CharacterId(1));
    world.add_item(pick);
    world.add_character(player(2, "Godmode"));

    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_GIVE, 2, 50, 0);
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 19), 0, 17);
    assert!(
        events.contains(&TwoThiefMasterOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 20,
        })
    );
    assert!(events.contains(&TwoThiefMasterOutcomeEvent::Quest28Done {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("I thank thee")));
}

#[test]
fn thiefmaster_give_unrelated_item_is_destroyed_unconditionally() {
    let mut world = World::default();
    let mut thiefmaster = thiefmaster_npc(1);
    thiefmaster.cursor_item = Some(ItemId(50));
    world.add_character(thiefmaster);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_GIVE, 2, 50, 0);
    }
    let events = world.process_two_thiefmaster_actions(&facts(CharacterId(2), 0), 0, 17);
    assert!(events.is_empty());
    assert!(!world.items.contains_key(&ItemId(50)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}

#[test]
fn thiefmaster_adds_a_different_group_attacker_as_an_enemy() {
    let mut world = World::default();
    let mut thiefmaster = thiefmaster_npc(1);
    thiefmaster.group = 0;
    world.add_character(thiefmaster);
    let mut attacker = player(2, "Attacker");
    attacker.group = 1;
    world.add_character(attacker);

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }
    world.process_two_thiefmaster_actions(&facts(CharacterId(2), 0), 0, 17);

    let thiefmaster = world.characters.get(&CharacterId(1)).unwrap();
    let fight_driver = thiefmaster.fight_driver.as_ref().unwrap();
    assert!(fight_driver
        .enemies
        .iter()
        .any(|enemy| enemy.target_id == CharacterId(2)));
    assert_ne!(fight_driver.last_hit, 0);
}

#[test]
fn thiefmaster_does_not_add_a_same_group_attacker_as_an_enemy() {
    let mut world = World::default();
    world.add_character(thiefmaster_npc(1));
    world.add_character(player(2, "Ally"));

    if let Some(thiefmaster) = world.characters.get_mut(&CharacterId(1)) {
        thiefmaster.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }
    world.process_two_thiefmaster_actions(&facts(CharacterId(2), 0), 0, 17);

    let thiefmaster = world.characters.get(&CharacterId(1)).unwrap();
    assert!(thiefmaster
        .fight_driver
        .as_ref()
        .map(|data| data.enemies.is_empty())
        .unwrap_or(true));
}

#[test]
fn thiefmaster_never_follows_invisible_enemies_unlike_simplebaddy() {
    // C `thiefmaster`'s tail calls `fight_driver_attack_visible(cn, 0)`
    // but never `fight_driver_follow_invisible` (`two.c:2169-2172`) -
    // unlike `simple_baddy_driver`/`lostcon_driver`/`guard_driver`. A
    // lone invisible enemy should therefore never make this NPC walk.
    let mut world = World::default();
    let mut thiefmaster = thiefmaster_npc(1);
    thiefmaster.fight_driver = Some(FightDriverData {
        enemies: vec![crate::character_driver::SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 1,
            visible: false,
            last_x: 15,
            last_y: 10,
        }],
        ..FightDriverData::default()
    });
    world.spawn_character(thiefmaster, 10, 10);
    let mut target = player(2, "Godmode");
    target.flags.insert(CharacterFlags::INVISIBLE);
    world.spawn_character(target, 15, 10);

    world.process_two_thiefmaster_actions(&facts(CharacterId(2), 0), 0, 17);

    let thiefmaster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(thiefmaster.action, action::IDLE);
    assert_eq!((thiefmaster.x, thiefmaster.y), (10, 10));
}
