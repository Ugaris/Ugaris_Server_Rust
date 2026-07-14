// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    CharacterDriverMessage, TwoSkellyDriverData, CDR_TWOSKELLY, NT_CHAR, NT_CREATE, NT_GIVE,
    NT_TEXT,
};
use crate::item_driver::{IID_AREA17_CROSS, IID_AREA17_GREENKEY, IID_AREA17_REDKEY};
use crate::world::npc::area17::two_skelly::{TwoSkellyOutcomeEvent, TwoSkellyPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn skelly_npc(id: u32) -> Character {
    let mut skelly = character(id);
    skelly.name = "Scarcewind".into();
    skelly.driver = CDR_TWOSKELLY;
    // `alive_tick` defaults to a value close to `BASELINE_TICK` (the tick
    // every non-self-destruct test below runs at) so those tests don't
    // trip the 30-second self-destruct timer just because they never
    // simulate the `NT_CREATE` message a real spawn always sends the same
    // tick. The dedicated self-destruct tests below override this.
    skelly.driver_state = Some(CharacterDriverState::TwoSkelly(TwoSkellyDriverData {
        alive_tick: BASELINE_TICK,
        ..Default::default()
    }));
    skelly
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(player_id: CharacterId, skelly_state: i32) -> HashMap<CharacterId, TwoSkellyPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, TwoSkellyPlayerFacts { skelly_state });
    map
}

fn skelly_state(world: &World, skelly_id: CharacterId) -> TwoSkellyDriverData {
    match world
        .characters
        .get(&skelly_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::TwoSkelly(data)) => data,
        _ => panic!("expected two skelly driver state"),
    }
}

#[test]
fn skelly_create_message_stamps_alive_tick() {
    let mut world = World::default();
    world.tick = Tick(BASELINE_TICK);
    assert!(world.spawn_character(skelly_npc(1), 10, 10));
    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        skelly.push_driver_message(NT_CREATE, 0, 0, 0);
    }

    world.process_two_skelly_actions(&HashMap::new(), 17);

    assert_eq!(
        skelly_state(&world, CharacterId(1)).alive_tick,
        BASELINE_TICK
    );
}

#[test]
fn skelly_greets_new_player_opens_quest_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(skelly_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        skelly.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_skelly_actions(&facts(CharacterId(2), 0), 17);
    assert!(events.contains(&TwoSkellyOutcomeEvent::UpdateSkellyState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&TwoSkellyOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Former governor")));
    assert_eq!(
        skelly_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn skelly_state1_speaks_riddle_and_advances_to_state2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(skelly_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        skelly.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_skelly_actions(&facts(CharacterId(2), 1), 17);
    assert!(events.contains(&TwoSkellyOutcomeEvent::UpdateSkellyState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, TwoSkellyOutcomeEvent::QuestOpen { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Pass the green")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("speaks in a different voice now")));
}

#[test]
fn skelly_state2_and_3_stay_silent() {
    for state in [2, 3] {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(skelly_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
            skelly.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_two_skelly_actions(&facts(CharacterId(2), state), 17);
        assert!(events.is_empty());
        assert!(world.drain_pending_area_texts().is_empty());
    }
}

#[test]
fn skelly_repeat_command_resets_state_to_zero_when_at_or_below_two() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(skelly_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        skelly.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_two_skelly_actions(&facts(CharacterId(2), 2), 17);
    assert!(events.contains(&TwoSkellyOutcomeEvent::UpdateSkellyState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
}

#[test]
fn skelly_repeat_command_is_ignored_once_past_state_two() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(skelly_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        skelly.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_two_skelly_actions(&facts(CharacterId(2), 3), 17);
    assert!(!events
        .iter()
        .any(|event| matches!(event, TwoSkellyOutcomeEvent::UpdateSkellyState { .. })));
}

#[test]
fn skelly_greeting_qa_reply_substitutes_speaker_name() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(skelly_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        skelly.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("hello".to_string()),
        });
    }

    world.process_two_skelly_actions(&facts(CharacterId(2), 0), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn skelly_receiving_cross_at_or_below_state2_completes_quest_and_cleans_up_keys() {
    let mut world = World::default();
    let mut skelly = skelly_npc(1);
    skelly.cursor_item = Some(ItemId(50));
    world.add_character(skelly);

    let mut cross = item(50, ItemFlags::empty());
    cross.name = "Governor's Cross".into();
    cross.template_id = IID_AREA17_CROSS;
    cross.carried_by = Some(CharacterId(1));
    world.add_item(cross);

    let mut godmode = player(2, "Godmode");
    let greenkey_id = ItemId(51);
    let redkey_id = ItemId(52);
    godmode.inventory[0] = Some(greenkey_id);
    godmode.inventory[1] = Some(redkey_id);
    world.add_character(godmode);

    let mut greenkey = item(51, ItemFlags::empty());
    greenkey.template_id = IID_AREA17_GREENKEY;
    greenkey.carried_by = Some(CharacterId(2));
    world.add_item(greenkey);
    let mut redkey = item(52, ItemFlags::empty());
    redkey.template_id = IID_AREA17_REDKEY;
    redkey.carried_by = Some(CharacterId(2));
    world.add_item(redkey);

    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        skelly.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_two_skelly_actions(&facts(CharacterId(2), 2), 17);
    assert!(events.contains(&TwoSkellyOutcomeEvent::UpdateSkellyState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    assert!(events.contains(&TwoSkellyOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I thank thee, Godmode")));
    assert!(!world.items.contains_key(&ItemId(50)));
    assert!(!world.items.contains_key(&greenkey_id));
    assert!(!world.items.contains_key(&redkey_id));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}

#[test]
fn skelly_receiving_unrelated_item_hands_it_back() {
    let mut world = World::default();
    let mut skelly = skelly_npc(1);
    skelly.cursor_item = Some(ItemId(50));
    world.add_character(skelly);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        skelly.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_two_skelly_actions(&facts(CharacterId(2), 2), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn skelly_self_destructs_thirty_seconds_after_creation() {
    let mut world = World::default();
    assert!(world.spawn_character(skelly_npc(1), 10, 10));
    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        // C `create_drop_char` stamps `tmpx`/`tmpy` (ported as `rest_x`/
        // `rest_y`) to the actual spawn tile - reproduced here since the
        // `ugaris-server`-side `raise_skeleton_from_template` glue (which
        // does this for real spawns) isn't exercised by this `ugaris-core`
        // test.
        skelly.rest_x = skelly.x;
        skelly.rest_y = skelly.y;
        skelly.driver_state = Some(CharacterDriverState::TwoSkelly(TwoSkellyDriverData {
            last_talk_tick: 0,
            current_victim: None,
            alive_tick: 0,
        }));
    }
    world.tick = Tick(TICKS_PER_SECOND * 30 + 1);

    world.process_two_skelly_actions(&HashMap::new(), 17);

    assert!(!world.characters.contains_key(&CharacterId(1)));
}

#[test]
fn skelly_does_not_self_destruct_before_thirty_seconds() {
    let mut world = World::default();
    assert!(world.spawn_character(skelly_npc(1), 10, 10));
    if let Some(skelly) = world.characters.get_mut(&CharacterId(1)) {
        skelly.rest_x = skelly.x;
        skelly.rest_y = skelly.y;
        skelly.driver_state = Some(CharacterDriverState::TwoSkelly(TwoSkellyDriverData {
            last_talk_tick: 0,
            current_victim: None,
            alive_tick: 0,
        }));
    }
    world.tick = Tick(TICKS_PER_SECOND * 29);

    world.process_two_skelly_actions(&HashMap::new(), 17);

    assert!(world.characters.contains_key(&CharacterId(1)));
}
