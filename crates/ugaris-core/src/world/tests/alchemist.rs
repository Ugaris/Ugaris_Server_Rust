use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    CharacterDriverMessage, TwoAlchemistDriverData, CDR_TWOALCHEMIST, NT_CHAR, NT_GIVE, NT_TEXT,
};
use crate::item_driver::IID_AREA17_POISON;
use crate::world::npc::area17::alchemist::{TwoAlchemistOutcomeEvent, TwoAlchemistPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn alchemist_npc(id: u32) -> Character {
    let mut alchemist = character(id);
    alchemist.name = "Cervik".into();
    alchemist.driver = CDR_TWOALCHEMIST;
    alchemist.driver_state = Some(CharacterDriverState::TwoAlchemist(
        TwoAlchemistDriverData::default(),
    ));
    alchemist
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    alchemist_state: i32,
) -> HashMap<CharacterId, TwoAlchemistPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, TwoAlchemistPlayerFacts { alchemist_state });
    map
}

fn alchemist_state(world: &World, alchemist_id: CharacterId) -> TwoAlchemistDriverData {
    match world
        .characters
        .get(&alchemist_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::TwoAlchemist(data)) => data,
        _ => panic!("expected two alchemist driver state"),
    }
}

#[test]
fn alchemist_greets_new_player_opens_quest_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(alchemist_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(alchemist) = world.characters.get_mut(&CharacterId(1)) {
        alchemist.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_alchemist_actions(&facts(CharacterId(2), 0), 17);
    assert!(
        events.contains(&TwoAlchemistOutcomeEvent::UpdateAlchemistState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    assert!(events.contains(&TwoAlchemistOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("too much sulphur")
            || text.message.contains("Too much sulphur")));
    assert_eq!(
        alchemist_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn alchemist_state1_gives_favor_speech_and_advances_to_state2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(alchemist_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(alchemist) = world.characters.get_mut(&CharacterId(1)) {
        alchemist.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_alchemist_actions(&facts(CharacterId(2), 1), 17);
    assert!(
        events.contains(&TwoAlchemistOutcomeEvent::UpdateAlchemistState {
            player_id: CharacterId(2),
            new_state: 2,
        })
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, TwoAlchemistOutcomeEvent::QuestOpen { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("spider poison for my experiments")));
}

#[test]
fn alchemist_states_4_and_5_stay_silent() {
    for state in [4, 5] {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(alchemist_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(alchemist) = world.characters.get_mut(&CharacterId(1)) {
            alchemist.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_two_alchemist_actions(&facts(CharacterId(2), state), 17);
        assert!(events.is_empty());
        assert!(world.drain_pending_area_texts().is_empty());
    }
}

#[test]
fn alchemist_repeat_command_resets_state_to_zero_when_at_or_below_four() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(alchemist_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(alchemist) = world.characters.get_mut(&CharacterId(1)) {
        alchemist.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_two_alchemist_actions(&facts(CharacterId(2), 3), 17);
    assert!(
        events.contains(&TwoAlchemistOutcomeEvent::UpdateAlchemistState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn alchemist_repeat_command_is_ignored_once_turned_in() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(alchemist_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(alchemist) = world.characters.get_mut(&CharacterId(1)) {
        alchemist.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_two_alchemist_actions(&facts(CharacterId(2), 5), 17);
    assert!(!events
        .iter()
        .any(|event| matches!(event, TwoAlchemistOutcomeEvent::UpdateAlchemistState { .. })));
}

#[test]
fn alchemist_greeting_qa_reply_substitutes_speaker_name() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(alchemist_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(alchemist) = world.characters.get_mut(&CharacterId(1)) {
        alchemist.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("hello".to_string()),
        });
    }

    world.process_two_alchemist_actions(&facts(CharacterId(2), 0), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn alchemist_receiving_poison_at_or_below_state4_completes_quest() {
    let mut world = World::default();
    let mut alchemist = alchemist_npc(1);
    alchemist.cursor_item = Some(ItemId(50));
    world.add_character(alchemist);

    let mut poison = item(50, ItemFlags::empty());
    poison.name = "Spider Poison".into();
    poison.template_id = IID_AREA17_POISON;
    poison.carried_by = Some(CharacterId(1));
    world.add_item(poison);

    let mut godmode = player(2, "Godmode");
    let poison_in_inventory_id = ItemId(51);
    godmode.inventory[0] = Some(poison_in_inventory_id);
    world.add_character(godmode);

    let mut poison_copy = item(51, ItemFlags::empty());
    poison_copy.template_id = IID_AREA17_POISON;
    poison_copy.carried_by = Some(CharacterId(2));
    world.add_item(poison_copy);

    if let Some(alchemist) = world.characters.get_mut(&CharacterId(1)) {
        alchemist.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_two_alchemist_actions(&facts(CharacterId(2), 4), 17);
    assert!(
        events.contains(&TwoAlchemistOutcomeEvent::UpdateAlchemistState {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
    assert!(events.contains(&TwoAlchemistOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        alchemist_id: CharacterId(1),
    }));
    // The cursor poison item and the matching inventory poison item are
    // both destroyed (C's belt-and-suspenders `destroy_item_byID` plus
    // `destroy_item(ch[cn].citem)`).
    assert!(world.items.get(&ItemId(50)).is_none());
    assert!(world.items.get(&poison_in_inventory_id).is_none());
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
    // No `say` fires here - the reward/thank-you text is only decided
    // once the server-side glue knows the quest completion count.
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn alchemist_receiving_poison_while_already_turned_in_hands_it_back() {
    let mut world = World::default();
    let mut alchemist = alchemist_npc(1);
    alchemist.cursor_item = Some(ItemId(50));
    world.add_character(alchemist);

    let mut poison = item(50, ItemFlags::empty());
    poison.name = "Spider Poison".into();
    poison.template_id = IID_AREA17_POISON;
    poison.carried_by = Some(CharacterId(1));
    world.add_item(poison);
    world.add_character(player(2, "Godmode"));

    if let Some(alchemist) = world.characters.get_mut(&CharacterId(1)) {
        alchemist.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_two_alchemist_actions(&facts(CharacterId(2), 5), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn alchemist_receiving_unrelated_item_hands_it_back() {
    let mut world = World::default();
    let mut alchemist = alchemist_npc(1);
    alchemist.cursor_item = Some(ItemId(50));
    world.add_character(alchemist);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(alchemist) = world.characters.get_mut(&CharacterId(1)) {
        alchemist.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_two_alchemist_actions(&facts(CharacterId(2), 2), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
