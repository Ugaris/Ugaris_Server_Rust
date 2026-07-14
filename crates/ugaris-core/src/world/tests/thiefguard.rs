use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    CharacterDriverMessage, FightDriverData, SimpleBaddyEnemy, TwoThiefGuardDriverData,
    CDR_TWOTHIEFGUARD, NT_CHAR, NT_GIVE, NT_TEXT,
};
use crate::world::npc::area17::thiefguard::{TwoThiefGuardOutcomeEvent, TwoThiefGuardPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn thiefguard_npc(id: u32) -> Character {
    let mut thiefguard = character(id);
    thiefguard.name = "Guard".into();
    thiefguard.driver = CDR_TWOTHIEFGUARD;
    thiefguard.driver_state = Some(CharacterDriverState::TwoThiefGuard(
        TwoThiefGuardDriverData::default(),
    ));
    thiefguard
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    thief_state: i32,
) -> HashMap<CharacterId, TwoThiefGuardPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, TwoThiefGuardPlayerFacts { thief_state });
    map
}

fn thiefguard_state(world: &World, thiefguard_id: CharacterId) -> TwoThiefGuardDriverData {
    match world
        .characters
        .get(&thiefguard_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::TwoThiefGuard(data)) => data,
        _ => panic!("expected two thiefguard driver state"),
    }
}

#[test]
fn thiefguard_greets_new_player_and_advances_state_to_one() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    // y >= 27 so the hostility check never fires here.
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 0), 17);
    assert!(
        events.contains(&TwoThiefGuardOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("welcome to the thieves guild")));
    assert_eq!(
        thiefguard_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn thiefguard_state1_mentions_the_fee_and_advances_to_two() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 1), 17);
    assert!(
        events.contains(&TwoThiefGuardOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 2,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("100G")));
}

#[test]
fn thiefguard_state2_waits_silently_for_the_fee() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 2), 17);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn thiefguard_state3_points_to_the_guild_master_and_advances_to_four() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 3), 17);
    assert!(
        events.contains(&TwoThiefGuardOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 4,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("guild master")));
}

#[test]
fn thiefguard_state4_stays_silent_once_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 4), 17);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn thiefguard_state50_recognizes_the_guild_masters_killer_and_advances_to_51() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 50), 17);
    assert!(
        events.contains(&TwoThiefGuardOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 51,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("killed the old guild master")));
}

#[test]
fn thiefguard_state51_holds_no_grudges_and_resets_to_one() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 51), 17);
    assert!(
        events.contains(&TwoThiefGuardOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("won't hold any grudges")));
}

#[test]
fn thiefguard_adds_a_hostile_player_as_an_enemy_inside_the_sewers() {
    // C `if (ppd && ppd->thief_state < 3 && ch[co].y < 27 &&
    // char_see_char(cn, co)) fight_driver_add_enemy(cn, co, 1, 1);`
    // (`two.c:1572-1575`) - unconditional on the talk cooldown, so make
    // the guard "recently talked" to prove the enemy add still happens.
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.driver_state = Some(CharacterDriverState::TwoThiefGuard(
            TwoThiefGuardDriverData {
                last_talk_tick: BASELINE_TICK,
                current_victim: None,
            },
        ));
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_two_thiefguard_actions(&facts(CharacterId(2), 2), 17);

    let thiefguard = world.characters.get(&CharacterId(1)).unwrap();
    let enemies = &thiefguard.fight_driver.as_ref().unwrap().enemies;
    assert!(enemies
        .iter()
        .any(|enemy| enemy.target_id == CharacterId(2)));
    // Still on cooldown, so no dialogue happened despite the enemy add.
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn thiefguard_does_not_add_enemy_for_player_outside_the_sewer_boundary() {
    let mut world = World::default();
    world.map.tile_mut(12, 30).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 30));
    // y == 30, at or above the C `< 27` boundary.
    assert!(world.spawn_character(player(2, "Godmode"), 12, 30));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_two_thiefguard_actions(&facts(CharacterId(2), 0), 17);

    let thiefguard = world.characters.get(&CharacterId(1)).unwrap();
    assert!(thiefguard.fight_driver.is_none());
}

#[test]
fn thiefguard_does_not_add_enemy_once_thief_state_reaches_three() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_two_thiefguard_actions(&facts(CharacterId(2), 3), 17);

    let thiefguard = world.characters.get(&CharacterId(1)).unwrap();
    assert!(thiefguard.fight_driver.is_none());
}

#[test]
fn thiefguard_repeat_command_resets_state_to_zero_when_at_or_below_two() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 2), 17);
    assert!(
        events.contains(&TwoThiefGuardOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn thiefguard_repeat_command_is_inert_above_state_two() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 3), 17);
    assert!(events.is_empty());
    // The NPC still tracks the speaker as its current victim even though
    // the "repeat" command was inert (C's `if (didsay)` bookkeeping fires
    // regardless of which qa row matched).
    assert_eq!(
        thiefguard_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn thiefguard_pay_a_fee_succeeds_when_enough_gold_and_state_two() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 20_000;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay a fee".to_string()),
        });
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 2), 17);
    assert!(
        events.contains(&TwoThiefGuardOutcomeEvent::UpdateThiefState {
            player_id: CharacterId(2),
            new_state: 3,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("welcome thee")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 10_000);
}

#[test]
fn thiefguard_pay_a_fee_fails_when_not_enough_gold() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 100;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay a fee".to_string()),
        });
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 2), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("dost not have enough money")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 100);
}

#[test]
fn thiefguard_pay_a_fee_replies_hu_when_state_is_not_two() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("pay a fee".to_string()),
        });
    }

    let events = world.process_two_thiefguard_actions(&facts(CharacterId(2), 0), 17);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Hu?")));
}

#[test]
fn thiefguard_greeting_qa_reply_substitutes_speaker_name() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(thiefguard_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("hello".to_string()),
        });
    }

    world.process_two_thiefguard_actions(&facts(CharacterId(2), 0), 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn thiefguard_receiving_any_item_destroys_it_unconditionally() {
    let mut world = World::default();
    let mut thiefguard = thiefguard_npc(1);
    thiefguard.cursor_item = Some(ItemId(50));
    world.add_character(thiefguard);

    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(thiefguard) = world.characters.get_mut(&CharacterId(1)) {
        thiefguard.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_two_thiefguard_actions(&facts(CharacterId(2), 0), 17);

    assert!(!world.items.contains_key(&ItemId(50)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}

#[test]
fn thiefguard_never_follows_invisible_enemies_unlike_simplebaddy() {
    // C `thiefguard`'s tail calls `fight_driver_attack_visible(cn, 0)` but
    // never `fight_driver_follow_invisible` (`two.c:1719-1723`) - unlike
    // `simple_baddy_driver`/`lostcon_driver`/`guard_driver`. A lone
    // invisible enemy should therefore never make this NPC walk.
    let mut world = World::default();
    let mut thiefguard = thiefguard_npc(1);
    thiefguard.fight_driver = Some(FightDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 1,
            visible: false,
            last_x: 15,
            last_y: 10,
        }],
        ..FightDriverData::default()
    });
    world.spawn_character(thiefguard, 10, 10);
    let mut target = player(2, "Godmode");
    target.flags.insert(CharacterFlags::INVISIBLE);
    world.spawn_character(target, 15, 10);

    world.process_two_thiefguard_actions(&facts(CharacterId(2), 0), 17);

    let thiefguard = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(thiefguard.action, action::IDLE);
    assert_eq!((thiefguard.x, thiefguard.y), (10, 10));
}
