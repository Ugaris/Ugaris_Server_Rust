use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    CharacterDriverMessage, TwoServantDriverData, CDR_TWOSERVANT, NT_CHAR, NT_GIVE, NT_GOTHIT,
    NT_TEXT,
};
use crate::world::npc::area17::{TwoServantOutcomeEvent, TwoServantPlayerFacts, CS_ENEMY};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn servant_npc(id: u32, nr: i32) -> Character {
    let mut servant = character(id);
    servant.name = "Maid".into();
    servant.driver = CDR_TWOSERVANT;
    servant.driver_state = Some(CharacterDriverState::TwoServant(TwoServantDriverData {
        nr,
        ..Default::default()
    }));
    servant
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    citizen_status: i32,
) -> HashMap<CharacterId, TwoServantPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, TwoServantPlayerFacts { citizen_status });
    map
}

fn servant_state(world: &World, servant_id: CharacterId) -> TwoServantDriverData {
    match world
        .characters
        .get(&servant_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::TwoServant(data)) => data,
        _ => panic!("expected two servant driver state"),
    }
}

#[test]
fn servant_greets_new_player_with_not_supposed_to_be_here() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 0), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_text_bytes();
    // Both the "not supposed to be here" hint and the never-reachable
    // "My greetings..." second line embed `chat`/`bribe`/`threaten` in
    // `COL_LIGHT_BLUE` markers - only one text is queued (see the module
    // doc comment's double-switch quirk).
    assert_eq!(texts.len(), 1);
    assert!(String::from_utf8_lossy(&texts[0].message).contains("Thou art not supposed to be here"));
    assert_eq!(
        servant_state(&world, CharacterId(1)).current_state,
        1,
        "the second (dead) switch never advances state past 1"
    );
    assert_eq!(
        servant_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn servant_nr4_governor_double_calls_guard_on_first_greeting() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 4), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    let mut guard = character(3);
    guard.group = 0;
    guard.level = 50;
    assert!(world.spawn_character(guard, 10, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("GUARDS!")));
    // `call_guard` pushes an `NT_NPC` message to the nearest higher-level
    // same-group character (here, the `guard` stand-in).
    assert!(world
        .characters
        .get(&CharacterId(3))
        .unwrap()
        .driver_messages
        .iter()
        .any(|message| message.message_type == crate::character_driver::NT_NPC));
}

#[test]
fn servant_repeat_command_resets_state_to_zero() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 0), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert_eq!(servant_state(&world, CharacterId(1)).current_state, 0);
    assert_eq!(
        servant_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn servant_greeting_qa_reply_substitutes_speaker_name() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 0), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("hello".to_string()),
        });
    }

    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn servant_chat_nr0_scullery_girl_reply() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 0), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("chat".to_string()),
        });
    }

    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("scrubbing pots and pans")));
    assert_eq!(
        servant_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn servant_chat_nr4_governor_double_says_nothing_but_still_marks_victim() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 4), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("chat".to_string()),
        });
    }

    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert!(world.drain_pending_area_texts().is_empty());
    // C's `didsay` stays truthy (the outer `analyse_text_driver` return
    // code) even though the inner `nr`-switch said nothing.
    assert_eq!(
        servant_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn servant_bribe_nr2_mistress_male_speaker_gets_kiss_offer() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 2), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::MALE;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("bribe".to_string()),
        });
    }

    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts.iter().any(|text| {
        let text = String::from_utf8_lossy(&text.message);
        text.contains("most handsome") && text.contains("a kiss")
    }));
}

#[test]
fn servant_bribe_nr2_mistress_female_speaker_declines() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 2), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::FEMALE;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("bribe".to_string()),
        });
    }

    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("most common, wench")));
}

#[test]
fn servant_threaten_nr2_mistress_male_speaker_calls_guard() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 2), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::MALE;
    assert!(world.spawn_character(godmode, 11, 10));
    let mut guard = character(3);
    guard.group = 0;
    guard.level = 50;
    assert!(world.spawn_character(guard, 10, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("threaten".to_string()),
        });
    }

    let events = world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert!(events.is_empty());
    assert!(world
        .characters
        .get(&CharacterId(3))
        .unwrap()
        .driver_messages
        .iter()
        .any(|message| message.message_type == crate::character_driver::NT_NPC));
}

#[test]
fn servant_threaten_nr2_mistress_female_speaker_gets_palace_key1_event() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 2), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::FEMALE;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("threaten".to_string()),
        });
    }

    let events = world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert!(events.contains(&TwoServantOutcomeEvent::GivePalaceKey1 {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Here's the key")));
}

#[test]
fn servant_pay_bribe_nr0_succeeds_and_uses_sirname_not_player_name() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 0), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::MALE;
    godmode.gold = 5000;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay bribe".to_string()),
        });
    }

    let events = world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("noble Sir")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 3000);
}

#[test]
fn servant_pay_bribe_nr0_fails_without_enough_gold() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 0), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 100;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay bribe".to_string()),
        });
    }

    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("First thou offerest me money")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 100);
}

#[test]
fn servant_pay_bribe_nr1_succeeds_and_awards_palace_key2_event() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 10000;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay bribe".to_string()),
        });
    }

    let events = world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert!(events.contains(&TwoServantOutcomeEvent::GivePalaceKey2 {
        player_id: CharacterId(2),
    }));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 5000);
}

#[test]
fn servant_pay_bribe_nr1_fails_without_enough_gold_no_key_event() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(servant_npc(1, 1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 100;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay bribe".to_string()),
        });
    }

    let events = world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("No money no key")));
}

#[test]
fn servant_gothit_alert_calls_guard_and_respects_cooldown() {
    let mut world = World::default();
    assert!(world.spawn_character(servant_npc(1, 0), 10, 10));
    let mut attacker = player(2, "Attacker");
    attacker.group = 1;
    assert!(world.spawn_character(attacker, 11, 10));
    let mut guard = character(3);
    guard.group = 0;
    guard.level = 50;
    assert!(world.spawn_character(guard, 10, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }
    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Guards! HELP!")));
    assert_eq!(
        servant_state(&world, CharacterId(1)).lastalert,
        BASELINE_TICK
    );

    // Within cooldown: no repeat alert.
    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }
    world.tick = Tick(BASELINE_TICK + 1);
    world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn servant_receiving_any_item_destroys_it() {
    let mut world = World::default();
    let mut servant = servant_npc(1, 0);
    servant.cursor_item = Some(ItemId(50));
    world.add_character(servant);

    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(servant) = world.characters.get_mut(&CharacterId(1)) {
        servant.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_two_servant_actions(&facts(CharacterId(2), CS_ENEMY), 17);
    assert!(events.is_empty());
    assert!(world.items.get(&ItemId(50)).is_none());
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}
