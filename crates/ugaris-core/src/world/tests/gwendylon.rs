use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    GwendylonDriverData, CDR_GWENDYLON, NTID_TUTORIAL, NT_CHAR, NT_GIVE,
};
use crate::entity::CharacterValue;
use crate::item_driver::{IID_AREA1_SKELKEY1, IID_AREA1_SKELSKULL, IID_CALIGARLETTER};
use crate::quest::QLOG_GWENDY_FIRST_SKULL;
use crate::spell::BLESS_COST;
use crate::world::gwendylon::{GwendylonOutcomeEvent, GwendylonPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn gwendylon_npc(id: u32) -> Character {
    let mut gwendylon = character(id);
    gwendylon.name = "Gwendylon".into();
    gwendylon.driver = CDR_GWENDYLON;
    gwendylon.driver_state = Some(CharacterDriverState::Gwendylon(
        GwendylonDriverData::default(),
    ));
    // Same rest-tile precedent as `world::terion`'s tests: keeps the
    // "return to post" idle branch a no-op.
    gwendylon.rest_x = 10;
    gwendylon.rest_y = 10;
    gwendylon
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    state: i32,
    seen_timer: i32,
) -> HashMap<CharacterId, GwendylonPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        GwendylonPlayerFacts {
            state,
            seen_timer,
            quest2_isdone: false,
            quest3_isdone: false,
            quest4_isdone: false,
            quest1_done_count: 0,
            quest2_done_count: 0,
            quest3_done_count: 0,
            quest4_done_count: 0,
        },
    );
    map
}

fn gwendylon_state(world: &World, gwendylon_id: CharacterId) -> GwendylonDriverData {
    match world
        .characters
        .get(&gwendylon_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Gwendylon(data)) => data,
        _ => panic!("expected gwendylon driver state"),
    }
}

#[test]
fn gwendylon_entry_greets_opens_quest_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gwendylon_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gwendylon) = world.characters.get_mut(&CharacterId(1)) {
        gwendylon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_gwendylon_actions(&facts(CharacterId(2), 0, 0), 0, 1);
    assert!(events.contains(&GwendylonOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: QLOG_GWENDY_FIRST_SKULL,
    }));
    assert!(events.contains(&GwendylonOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Welcome Godmode! I am Gwendylon the Mage.")));
    assert_eq!(
        gwendylon_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn gwendylon_first_skull_wait_reminder_notifies_tutorial() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gwendylon_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gwendylon) = world.characters.get_mut(&CharacterId(1)) {
        gwendylon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer far in the past: > 60s elapsed -> reminder + notify_area.
    let events = world.process_gwendylon_actions(&facts(CharacterId(2), 5, 0), 1000, 1);
    // State does not change (stays at FIRST_SKULL_WAIT).
    assert!(!events
        .iter()
        .any(|event| matches!(event, GwendylonOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains(
        "Be greeted, Godmode! Didst thou find anything magical in the skeleton's ruin?"
    )));

    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert!(player
        .driver_messages
        .iter()
        .any(|m| m.message_type == NT_NPC && m.dat1 == NTID_TUTORIAL));
}

#[test]
fn gwendylon_first_skull_done_skips_ahead_silently_when_quest2_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gwendylon_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gwendylon) = world.characters.get_mut(&CharacterId(1)) {
        gwendylon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let mut player_facts = facts(CharacterId(2), 6, 0);
    player_facts.get_mut(&CharacterId(2)).unwrap().quest2_isdone = true;

    let events = world.process_gwendylon_actions(&player_facts, 0, 1);
    assert!(events.contains(&GwendylonOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 10,
    }));
    // Silent jump: no dialogue, no QuestOpen, no facing update.
    assert!(world.drain_pending_area_texts().is_empty());
    assert!(!events
        .iter()
        .any(|event| matches!(event, GwendylonOutcomeEvent::QuestOpen { .. })));
    assert_eq!(gwendylon_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn gwendylon_text_repeat_resets_first_bucket() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gwendylon_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(gwendylon) = world.characters.get_mut(&CharacterId(1)) {
        gwendylon.driver_state = Some(CharacterDriverState::Gwendylon(GwendylonDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        gwendylon.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_gwendylon_actions(&facts(CharacterId(2), 3, 0), 0, 1);
    assert!(events.contains(&GwendylonOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert_eq!(gwendylon_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn gwendylon_give_skelskull_completes_quest_and_rewards_gold_first_time() {
    let mut world = World::default();
    let mut gwendylon = gwendylon_npc(1);
    gwendylon.cursor_item = Some(ItemId(50));
    world.add_character(gwendylon);
    let mut skull = item(50, ItemFlags::empty());
    skull.name = "Skeleton Skull".into();
    skull.template_id = IID_AREA1_SKELSKULL;
    skull.carried_by = Some(CharacterId(1));
    world.add_item(skull);

    let mut godmode = player(2, "Godmode");
    // A leftover copy of the same skull sitting in the player's own
    // inventory should also be swept by `destroy_items_by_template_id`.
    godmode.inventory[30] = Some(ItemId(51));
    world.add_character(godmode);
    let mut leftover = item(51, ItemFlags::empty());
    leftover.template_id = IID_AREA1_SKELKEY1;
    leftover.carried_by = Some(CharacterId(2));
    world.add_item(leftover);

    if let Some(gwendylon) = world.characters.get_mut(&CharacterId(1)) {
        gwendylon.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let mut player_facts = facts(CharacterId(2), 5, 0);
    player_facts
        .get_mut(&CharacterId(2))
        .unwrap()
        .quest1_done_count = 0;

    let events = world.process_gwendylon_actions(&player_facts, 0, 1);
    assert!(events.contains(&GwendylonOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: QLOG_GWENDY_FIRST_SKULL,
    }));
    assert!(events.contains(&GwendylonOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert!(events
        .iter()
        .any(|event| matches!(event, GwendylonOutcomeEvent::GoldEarned { amount: 125, .. })));

    // Held skull destroyed.
    assert!(world.items.get(&ItemId(50)).is_none());
    // Leftover key swept from the player's own inventory.
    assert!(world.items.get(&ItemId(51)).is_none());
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 125);
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn gwendylon_give_skelskull_no_gold_on_repeat_completion() {
    let mut world = World::default();
    let mut gwendylon = gwendylon_npc(1);
    gwendylon.cursor_item = Some(ItemId(50));
    world.add_character(gwendylon);
    let mut skull = item(50, ItemFlags::empty());
    skull.template_id = IID_AREA1_SKELSKULL;
    skull.carried_by = Some(CharacterId(1));
    world.add_item(skull);
    world.add_character(player(2, "Godmode"));

    if let Some(gwendylon) = world.characters.get_mut(&CharacterId(1)) {
        gwendylon.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let mut player_facts = facts(CharacterId(2), 5, 0);
    player_facts
        .get_mut(&CharacterId(2))
        .unwrap()
        .quest1_done_count = 1;

    let events = world.process_gwendylon_actions(&player_facts, 0, 1);
    assert!(events.contains(&GwendylonOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: QLOG_GWENDY_FIRST_SKULL,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, GwendylonOutcomeEvent::GoldEarned { .. })));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 0);
}

#[test]
fn gwendylon_give_caligarletter_hands_it_back_and_queues_cross_area_transfer() {
    let mut world = World::default();
    let mut gwendylon = gwendylon_npc(1);
    gwendylon.cursor_item = Some(ItemId(60));
    world.add_character(gwendylon);
    let mut letter = item(60, ItemFlags::empty());
    letter.name = "Caligar Letter".into();
    letter.template_id = IID_CALIGARLETTER;
    letter.carried_by = Some(CharacterId(1));
    world.add_item(letter);
    world.add_character(player(2, "Godmode"));

    if let Some(gwendylon) = world.characters.get_mut(&CharacterId(1)) {
        gwendylon.push_driver_message(NT_GIVE, 2, 60, 0);
    }

    world.process_gwendylon_actions(&HashMap::new(), 0, 1);

    // The letter is handed back to the player rather than destroyed.
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.inventory[30], Some(ItemId(60)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());

    let transfers = world.drain_pending_gwendylon_cross_area_transfers();
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0].player_id, CharacterId(2));
    assert_eq!(transfers[0].gwendylon_id, CharacterId(1));
}

#[test]
fn gwendylon_give_unknown_item_hands_it_back_to_giver() {
    let mut world = World::default();
    let mut gwendylon = gwendylon_npc(1);
    gwendylon.cursor_item = Some(ItemId(70));
    world.add_character(gwendylon);
    let mut trinket = item(70, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(gwendylon) = world.characters.get_mut(&CharacterId(1)) {
        gwendylon.push_driver_message(NT_GIVE, 2, 70, 0);
    }

    world.process_gwendylon_actions(&HashMap::new(), 0, 1);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.inventory[30], Some(ItemId(70)));
}

#[test]
fn gwendylon_done_bless_success_leaves_message_unconsumed_for_retry() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gwendylon_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags.insert(CharacterFlags::PLAYERLIKE);
    assert!(world.spawn_character(godmode, 12, 10));

    if let Some(gwendylon) = world.characters.get_mut(&CharacterId(1)) {
        gwendylon.mana = 10 * BLESS_COST;
        gwendylon.values[0][CharacterValue::Bless as usize] = 40;
        gwendylon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.tick = Tick(BASELINE_TICK);
    let events = world.process_gwendylon_actions(&facts(CharacterId(2), 19, 0), 1000, 1);

    // The `return` path only pushes `UpdateSeenTimer`, no `UpdateState`
    // (state stays at DONE_BLESS since there's nothing further to
    // advance to) and no `didsay`-gated facing/talk-throttle update.
    assert!(events.contains(&GwendylonOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1000,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, GwendylonOutcomeEvent::UpdateState { .. })));

    // The triggering `NT_CHAR` message was never removed - it's still
    // sitting in the queue for the next tick to reprocess, matching C's
    // `return` before `remove_message` runs.
    let gwendylon = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(gwendylon.driver_messages.len(), 1);
    assert_eq!(gwendylon.driver_messages[0].message_type, NT_CHAR);

    // The caster's own action was queued (the actual buff is applied once
    // the queued action's duration finishes, ticks later).
    assert_ne!(gwendylon.action, 0);
}
