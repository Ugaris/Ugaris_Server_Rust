use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_LAB3PRISONER, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_LAB3_PRISONKEY;
use crate::world::npc::area22::lab3_prisoner::{
    Lab3PrisonerDriverData, Lab3PrisonerOutcomeEvent, Lab3PrisonerPlayerFacts,
};

fn prisoner_npc(id: u32) -> Character {
    let mut prisoner = character(id);
    prisoner.name = "Prisoner".into();
    prisoner.driver = CDR_LAB3PRISONER;
    prisoner.x = 10;
    prisoner.y = 10;
    prisoner.rest_x = 10;
    prisoner.rest_y = 10;
    prisoner.driver_state = Some(CharacterDriverState::Lab3Prisoner(
        Lab3PrisonerDriverData::default(),
    ));
    prisoner
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.x = 10;
    player.y = 11;
    player
}

fn facts(
    player_id: CharacterId,
    prisoner_talkstep: u8,
) -> HashMap<CharacterId, Lab3PrisonerPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, Lab3PrisonerPlayerFacts { prisoner_talkstep });
    map
}

fn prisoner_state(world: &World, prisoner_id: CharacterId) -> Lab3PrisonerDriverData {
    match world
        .characters
        .get(&prisoner_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lab3Prisoner(data)) => data,
        _ => panic!("expected lab3 prisoner driver state"),
    }
}

#[test]
fn greeting_step0_says_blub_and_advances() {
    let mut world = World::default();
    world.add_character(prisoner_npc(1));
    world.add_character(player(2, "Traveler"));
    if let Some(prisoner) = world.characters.get_mut(&CharacterId(1)) {
        prisoner.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab3_prisoner_actions(&facts(CharacterId(2), 0), 22);

    assert!(
        events.contains(&Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
            player_id: CharacterId(2),
            value: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Blub.")));
    assert!(texts.iter().any(|t| t
        .message
        .contains("The Prisoner looks glad to see thee, Traveler.")));
}

#[test]
fn greeting_step3_advances_to_255() {
    let mut world = World::default();
    world.add_character(prisoner_npc(1));
    world.add_character(player(2, "Traveler"));
    if let Some(prisoner) = world.characters.get_mut(&CharacterId(1)) {
        prisoner.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab3_prisoner_actions(&facts(CharacterId(2), 3), 22);

    assert!(
        events.contains(&Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
            player_id: CharacterId(2),
            value: 255,
        })
    );
}

#[test]
fn giving_prison_key_unlocks_note_step() {
    let mut world = World::default();
    world.add_character(prisoner_npc(1));
    let mut giver = player(2, "Traveler");
    let mut key = item(50, ItemFlags::empty());
    key.template_id = IID_LAB3_PRISONKEY;
    key.carried_by = Some(CharacterId(1));
    world.add_item(key);
    giver.x = 10;
    giver.y = 11;
    world.add_character(giver);
    if let Some(prisoner) = world.characters.get_mut(&CharacterId(1)) {
        prisoner.cursor_item = Some(ItemId(50));
        prisoner.push_driver_message(NT_GIVE, 2, 0, 0);
    }

    let events = world.process_lab3_prisoner_actions(&facts(CharacterId(2), 0), 22);

    assert!(
        events.contains(&Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
            player_id: CharacterId(2),
            value: 20,
        })
    );
    // The key is always destroyed, matching C's unconditional
    // `destroy_item(ch[cn].citem)`.
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn giving_non_key_item_is_destroyed_without_unlocking() {
    let mut world = World::default();
    world.add_character(prisoner_npc(1));
    let giver = player(2, "Traveler");
    let mut junk = item(50, ItemFlags::empty());
    junk.template_id = 0xDEAD;
    junk.carried_by = Some(CharacterId(1));
    world.add_item(junk);
    world.add_character(giver);
    if let Some(prisoner) = world.characters.get_mut(&CharacterId(1)) {
        prisoner.cursor_item = Some(ItemId(50));
        prisoner.push_driver_message(NT_GIVE, 2, 0, 0);
    }

    let events = world.process_lab3_prisoner_actions(&facts(CharacterId(2), 0), 22);

    assert!(events.is_empty());
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn step20_requests_note_creation_and_sets_give_target() {
    let mut world = World::default();
    world.add_character(prisoner_npc(1));
    world.add_character(player(2, "Traveler"));
    if let Some(prisoner) = world.characters.get_mut(&CharacterId(1)) {
        prisoner.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab3_prisoner_actions(&facts(CharacterId(2), 20), 22);

    assert!(
        events.contains(&Lab3PrisonerOutcomeEvent::CreateNoteOnCursor {
            npc_id: CharacterId(1)
        })
    );
    let state = prisoner_state(&world, CharacterId(1));
    assert_eq!(state.give_target, Some(CharacterId(2)));
}

#[test]
fn step21_says_blub_and_finishes() {
    let mut world = World::default();
    world.add_character(prisoner_npc(1));
    world.add_character(player(2, "Traveler"));
    if let Some(prisoner) = world.characters.get_mut(&CharacterId(1)) {
        prisoner.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab3_prisoner_actions(&facts(CharacterId(2), 21), 22);

    assert!(
        events.contains(&Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
            player_id: CharacterId(2),
            value: 255,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Blub.")));
}

#[test]
fn give_target_hands_over_note_when_adjacent() {
    let mut world = World::default();
    let mut prisoner = prisoner_npc(1);
    prisoner.driver_state = Some(CharacterDriverState::Lab3Prisoner(Lab3PrisonerDriverData {
        give_target: Some(CharacterId(2)),
        give_serial: 2,
        ..Default::default()
    }));
    prisoner.cursor_item = Some(ItemId(50));
    world.add_character(prisoner);
    let mut note = item(50, ItemFlags::empty());
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Traveler"));

    let events = world.process_lab3_prisoner_actions(&facts(CharacterId(2), 20), 22);

    assert!(
        events.contains(&Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
            player_id: CharacterId(2),
            value: 21,
        })
    );
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(50))
    );
    assert!(prisoner_state(&world, CharacterId(1)).give_target.is_none());
}

#[test]
fn give_target_cancelled_when_player_leaves_home_range() {
    let mut world = World::default();
    let mut prisoner = prisoner_npc(1);
    prisoner.driver_state = Some(CharacterDriverState::Lab3Prisoner(Lab3PrisonerDriverData {
        give_target: Some(CharacterId(2)),
        give_serial: 2,
        ..Default::default()
    }));
    prisoner.cursor_item = Some(ItemId(50));
    world.add_character(prisoner);
    let mut note = item(50, ItemFlags::empty());
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    let mut far_player = player(2, "Traveler");
    far_player.x = 40;
    far_player.y = 40;
    world.add_character(far_player);

    let _ = world.process_lab3_prisoner_actions(&facts(CharacterId(2), 20), 22);

    assert!(prisoner_state(&world, CharacterId(1)).give_target.is_none());
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn blub_text_resets_talkstep() {
    let mut world = World::default();
    world.tick = Tick(1000);
    world.add_character(prisoner_npc(1));
    world.add_character(player(2, "Traveler"));
    if let Some(prisoner) = world.characters.get_mut(&CharacterId(1)) {
        prisoner.push_driver_text_message(CharacterId(2), "blub!");
    }

    let events = world.process_lab3_prisoner_actions(&facts(CharacterId(2), 255), 22);

    assert!(
        events.contains(&Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
            player_id: CharacterId(2),
            value: 0,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Blub!")));
}
