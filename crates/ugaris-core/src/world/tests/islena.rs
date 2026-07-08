use super::*;
use crate::character_driver::{
    CharacterDriverMessage, IslenaDriverData, CDR_PALACEISLENA, NT_SPELL, NT_TEXT,
};
use crate::world::{IslenaOutcomeEvent, IslenaPlayerFacts};
use std::collections::HashMap;

fn islena_npc(id: u32) -> Character {
    let mut islena = character(id);
    islena.name = "Islena".into();
    islena.driver = CDR_PALACEISLENA;
    islena.values[0][CharacterValue::Hp as usize] = 100;
    islena.values[0][CharacterValue::Mana as usize] = 50;
    islena.values[0][CharacterValue::Endurance as usize] = 40;
    islena.values[0][CharacterValue::MagicShield as usize] = 20;
    islena
}

fn player_char(id: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = "Hero".into();
    player
}

fn islena_state(world: &World, id: CharacterId) -> IslenaDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Islena(data)) => data,
        _ => panic!("expected islena driver state"),
    }
}

#[test]
fn first_sighting_says_first_greeting_line_and_advances_state() {
    // C `palace_islena`'s `NT_CHAR` branch, `state == 0` (`palace.c:618-
    // 623`).
    let mut world = World::default();
    world.tick.0 = TICKS_PER_SECOND * 1000; // past the 5s talk cooldown from a fresh `last_talk = 0`.
    let islena = islena_npc(1);
    assert!(world.spawn_character(islena, 10, 10));
    let player = player_char(2);
    assert!(world.spawn_character(player, 11, 10));
    if let Some(islena) = world.characters.get_mut(&CharacterId(1)) {
        islena.driver_messages.push(CharacterDriverMessage {
            message_type: NT_CHAR,
            dat1: 2,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }
    let mut facts = HashMap::new();
    facts.insert(CharacterId(2), IslenaPlayerFacts { islena_state: 0 });

    let events = world.process_islena_actions(&facts, 1);

    assert_eq!(
        events,
        vec![IslenaOutcomeEvent::UpdateState {
            player_id: CharacterId(2),
            new_state: 1,
        }]
    );
}

#[test]
fn talk_throttle_suppresses_a_second_line_within_five_seconds() {
    // C `if (ticker < dat->last_talk + TICKS * 5) { ...; continue; }`
    // (`palace.c:612-615`).
    let mut world = World::default();
    let mut islena = islena_npc(1);
    islena.driver_state = Some(CharacterDriverState::Islena(IslenaDriverData {
        last_talk: 100,
        ..Default::default()
    }));
    world.tick.0 = 100 + TICKS_PER_SECOND * 4; // under the 5s cooldown.
    assert!(world.spawn_character(islena, 10, 10));
    let player = player_char(2);
    assert!(world.spawn_character(player, 11, 10));
    if let Some(islena) = world.characters.get_mut(&CharacterId(1)) {
        islena.driver_messages.push(CharacterDriverMessage {
            message_type: NT_CHAR,
            dat1: 2,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }
    let mut facts = HashMap::new();
    facts.insert(CharacterId(2), IslenaPlayerFacts { islena_state: 0 });

    let events = world.process_islena_actions(&facts, 1);

    assert!(events.is_empty());
}

#[test]
fn already_hostile_player_becomes_the_tracked_victim_on_sighting() {
    // C `if (ppd->islena_state >= 10) { fight_driver_add_enemy(cn, co, 1,
    // 1); ...; continue; }` (`palace.c:606-610`).
    let mut world = World::default();
    let islena = islena_npc(1);
    assert!(world.spawn_character(islena, 10, 10));
    let player = player_char(2);
    assert!(world.spawn_character(player, 11, 10));
    if let Some(islena) = world.characters.get_mut(&CharacterId(1)) {
        islena.driver_messages.push(CharacterDriverMessage {
            message_type: NT_CHAR,
            dat1: 2,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }
    let mut facts = HashMap::new();
    facts.insert(CharacterId(2), IslenaPlayerFacts { islena_state: 10 });

    world.process_islena_actions(&facts, 1);

    assert_eq!(
        islena_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
}

#[test]
fn being_hit_pins_islena_state_to_ten_and_tracks_the_attacker_as_victim() {
    // C `palace.c:659-667`.
    let mut world = World::default();
    let islena = islena_npc(1);
    assert!(world.spawn_character(islena, 10, 10));
    let mut attacker = player_char(2);
    attacker.group = 1; // C `if (ch[cn].group == ch[co].group) break;` - see module doc comment.
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(islena) = world.characters.get_mut(&CharacterId(1)) {
        islena.driver_messages.push(CharacterDriverMessage {
            message_type: NT_GOTHIT,
            dat1: 2,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }
    let mut facts = HashMap::new();
    facts.insert(CharacterId(2), IslenaPlayerFacts { islena_state: 0 });

    let events = world.process_islena_actions(&facts, 1);

    assert_eq!(
        events,
        vec![IslenaOutcomeEvent::UpdateState {
            player_id: CharacterId(2),
            new_state: 10,
        }]
    );
    assert_eq!(
        islena_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
}

#[test]
fn freeze_spell_hit_pins_islena_state_to_ten_the_same_as_gothit() {
    // C `((msg->type == NT_GOTHIT) || (msg->type == NT_SPELL && msg->dat2
    // == V_FREEZE)) && (co = msg->dat1)` (`palace.c:659`).
    let mut world = World::default();
    let islena = islena_npc(1);
    assert!(world.spawn_character(islena, 10, 10));
    let attacker = player_char(2);
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(islena) = world.characters.get_mut(&CharacterId(1)) {
        islena.driver_messages.push(CharacterDriverMessage {
            message_type: NT_SPELL,
            dat1: 2,
            dat2: CharacterValue::Freeze as i32,
            dat3: 0,
            text: None,
        });
    }
    let mut facts = HashMap::new();
    facts.insert(CharacterId(2), IslenaPlayerFacts { islena_state: 0 });

    let events = world.process_islena_actions(&facts, 1);

    assert_eq!(
        events,
        vec![IslenaOutcomeEvent::UpdateState {
            player_id: CharacterId(2),
            new_state: 10,
        }]
    );
}

#[test]
fn other_spell_types_do_not_pin_islena_state() {
    // Same guard: `msg->dat2 == V_FREEZE` only.
    let mut world = World::default();
    let islena = islena_npc(1);
    assert!(world.spawn_character(islena, 10, 10));
    let attacker = player_char(2);
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(islena) = world.characters.get_mut(&CharacterId(1)) {
        islena.driver_messages.push(CharacterDriverMessage {
            message_type: NT_SPELL,
            dat1: 2,
            dat2: CharacterValue::Fireball as i32,
            dat3: 0,
            text: None,
        });
    }
    let mut facts = HashMap::new();
    facts.insert(CharacterId(2), IslenaPlayerFacts { islena_state: 0 });

    let events = world.process_islena_actions(&facts, 1);

    assert!(events.is_empty());
}

#[test]
fn two_different_attackers_within_the_window_trigger_a_full_heal() {
    // C `palace.c:668-680`: "Power of Two".
    let mut world = World::default();
    let mut islena = islena_npc(1);
    islena.hp = 1;
    islena.mana = 1;
    islena.endurance = 1;
    islena.lifeshield = 1;
    islena.driver_state = Some(CharacterDriverState::Islena(IslenaDriverData {
        last_hurt_time: 0,
        last_hurt_by: Some(CharacterId(2)),
        ..Default::default()
    }));
    world.tick.0 = TICKS_PER_SECOND * 5; // well under the 30s window.
    assert!(world.spawn_character(islena, 10, 10));
    let attacker = player_char(3); // a *different* attacker than last_hurt_by.
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(islena) = world.characters.get_mut(&CharacterId(1)) {
        islena.driver_messages.push(CharacterDriverMessage {
            message_type: NT_GOTHIT,
            dat1: 3,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }
    let mut facts = HashMap::new();
    facts.insert(CharacterId(3), IslenaPlayerFacts { islena_state: 0 });

    world.process_islena_actions(&facts, 1);

    let islena = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(islena.hp, 100 * POWERSCALE);
    assert_eq!(islena.mana, 50 * POWERSCALE);
    assert_eq!(islena.endurance, 40 * POWERSCALE);
    assert_eq!(islena.lifeshield, 20 * POWERSCALE);
}

#[test]
fn same_attacker_hitting_again_does_not_trigger_the_full_heal() {
    let mut world = World::default();
    let mut islena = islena_npc(1);
    islena.hp = 1;
    islena.driver_state = Some(CharacterDriverState::Islena(IslenaDriverData {
        last_hurt_time: 0,
        last_hurt_by: Some(CharacterId(2)),
        ..Default::default()
    }));
    world.tick.0 = TICKS_PER_SECOND * 5;
    assert!(world.spawn_character(islena, 10, 10));
    let attacker = player_char(2); // the *same* attacker as last_hurt_by.
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(islena) = world.characters.get_mut(&CharacterId(1)) {
        islena.driver_messages.push(CharacterDriverMessage {
            message_type: NT_GOTHIT,
            dat1: 2,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }
    let mut facts = HashMap::new();
    facts.insert(CharacterId(2), IslenaPlayerFacts { islena_state: 0 });

    world.process_islena_actions(&facts, 1);

    // C's own unconditional `if (washit) { ...heal...; }` fallback
    // (`palace.c:712-721`) still fully heals her by the end of the tick -
    // this is not the distinguishing behavior. What must NOT happen is
    // the in-loop "Power of Two" text specifically (`palace.c:673`),
    // since this was the *same* attacker as `last_hurt_by`.
    let texts = world.drain_pending_area_texts();
    assert!(!texts
        .iter()
        .any(|text| text.message.contains("Power of Two")));
}

#[test]
fn tabunga_dumps_stats_when_a_god_says_the_keyword_nearby() {
    let mut world = World::default();
    let islena = islena_npc(1);
    assert!(world.spawn_character(islena, 10, 10));
    let mut god = character(2);
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 10, 11));
    if let Some(islena) = world.characters.get_mut(&CharacterId(1)) {
        islena.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("tabunga".to_string()),
        });
    }
    let facts = HashMap::new();

    world.process_islena_actions(&facts, 1);

    let texts = world.drain_pending_area_texts();
    assert!(!texts.is_empty());
}

#[test]
fn first_kill_marks_won_says_grats_and_queues_the_ladykiller_award() {
    // C `islena_dead`'s "first kill" branch (`palace.c:751-766`).
    let mut world = World::default();
    let islena = islena_npc(1);
    assert!(world.spawn_character(islena, 10, 10));
    let killer = player_char(2);
    assert!(world.spawn_character(killer, 11, 10));

    world.apply_islena_death(CharacterId(1), CharacterId(2));

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(killer.flags.contains(CharacterFlags::WON));
    assert_eq!(
        world.drain_pending_islena_ladykiller_awards(),
        vec![CharacterId(2)]
    );
    let broadcasts = world.drain_pending_channel_broadcasts();
    assert_eq!(broadcasts.len(), 1);
    assert!(String::from_utf8_lossy(&broadcasts[0].message_bytes).contains("Grats: Hero is a"));
}

#[test]
fn repeat_kill_hurts_the_killer_instead_of_re_awarding() {
    // C `islena_dead`'s "already `CF_WON`" branch (`palace.c:748-750`).
    let mut world = World::default();
    let islena = islena_npc(1);
    assert!(world.spawn_character(islena, 10, 10));
    let mut killer = player_char(2);
    killer.flags |= CharacterFlags::WON;
    killer.hp = 10_000 * POWERSCALE;
    killer.values[0][CharacterValue::Hp as usize] = 10_000;
    assert!(world.spawn_character(killer, 11, 10));

    world.apply_islena_death(CharacterId(1), CharacterId(2));

    assert!(world.drain_pending_islena_ladykiller_awards().is_empty());
    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(killer.hp < 10_000 * POWERSCALE);
}
