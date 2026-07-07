use std::collections::HashMap;

use super::*;
use crate::character_driver::{JamesDriverData, CDR_JAMES, NT_CHAR, NT_GIVE};
use crate::world::npc::area1::james::{JamesOutcomeEvent, JamesPlayerFacts};

/// Same rationale as `world::lydia`'s own `BASELINE_TICK`: `NT_CHAR`
/// gates on `ticker < dat->last_talk + TICKS*5`, and a freshly spawned
/// NPC's `last_talk` starts at `0`, so tests must move the clock forward
/// past the boot-time-only quirk.
const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn james_npc(id: u32) -> Character {
    let mut james = character(id);
    james.name = "James".into();
    james.driver = CDR_JAMES;
    james.driver_state = Some(CharacterDriverState::James(JamesDriverData::default()));
    james
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    james_state: i32,
    lydia_state: i32,
    area1_flags: i32,
) -> HashMap<CharacterId, JamesPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        JamesPlayerFacts {
            james_state,
            lydia_state,
            area1_flags,
        },
    );
    map
}

fn james_state(world: &World, james_id: CharacterId) -> JamesDriverData {
    match world
        .characters
        .get(&james_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::James(data)) => data,
        _ => panic!("expected james driver state"),
    }
}

#[test]
fn james_state0_greets_opens_quest_and_advances_to_one() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    assert!(events.contains(&JamesOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&JamesOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Ah, hello there")));
}

#[test]
fn james_state0_skips_to_three_when_lydia_state_at_least_six() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_james_actions(&facts(CharacterId(2), 0, 6, 0), 1);
    assert!(events.contains(&JamesOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, JamesOutcomeEvent::QuestOpen { .. })));
    // No `didsay` in this branch, so no greeting text (the C source
    // itself sets `james_state = 3; break;` before the `quiet_say`).
    let texts = world.drain_pending_area_texts();
    assert!(!texts
        .iter()
        .any(|text| text.message.contains("Ah, hello there")));
}

#[test]
fn james_state0_hardcore_invite_is_unconditional_alongside_greeting() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::PAID;
    godmode.exp = 0;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    // The hardcore-invite line wraps "Hardcore" in `COL_LIGHT_RED`/
    // `COL_LIGHT_BLUE` markers (`gwendylon.c:2972-2974`); goes out via
    // `npc_quiet_say_bytes`.
    let byte_texts = world.drain_pending_area_text_bytes();
    assert!(byte_texts.iter().any(|text| {
        let text = String::from_utf8_lossy(&text.message);
        text.contains("Hardcore") && text.contains("character?")
    }));
    assert!(byte_texts
        .iter()
        .any(|text| text.message.windows(11).any(|w| w == b"\xb0c4Hardcore")));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Ah, hello there")));
}

#[test]
fn james_state2_advances_silently_dead_quiet_say_in_c() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Storage-hint flag pre-set so its own (unconditional) text doesn't
    // interfere with this test's "no text at all" assertion.
    let events = world.process_james_actions(&facts(CharacterId(2), 2, 0, 1 << 1), 1);
    assert!(events.contains(&JamesOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn james_state3_waits_for_lydia_state_six() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Storage-hint flag pre-set, same rationale as the state-2 test above.
    let events = world.process_james_actions(&facts(CharacterId(2), 3, 5, 1 << 1), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JamesOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn james_state3_advances_once_lydia_state_reaches_six() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_james_actions(&facts(CharacterId(2), 3, 6, 0), 1);
    assert!(events.contains(&JamesOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("glad that thou could help Lydia")));
}

#[test]
fn james_storage_hint_fires_once_for_empty_inventory() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // james_state 5 (no-op state) isolates the storage-hint branch.
    let events = world.process_james_actions(&facts(CharacterId(2), 5, 0, 0), 1);
    assert!(events.contains(&JamesOutcomeEvent::SetStorageHint {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("chests in the western corner")));
}

#[test]
fn james_storage_hint_does_not_fire_once_flag_is_set() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_james_actions(&facts(CharacterId(2), 5, 0, 1 << 1), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JamesOutcomeEvent::SetStorageHint { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn james_text_repeat_resets_state_zero_to_three() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_james_actions(&facts(CharacterId(2), 3, 0, 0), 1);
    assert!(events.contains(&JamesOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(james_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn james_text_repeat_outside_zero_to_three_is_a_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_james_actions(&facts(CharacterId(2), 5, 0, 0), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JamesOutcomeEvent::UpdateState { .. })));
}

#[test]
fn james_text_advice_quotes_fee_for_low_level_and_declines_high_level() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    let mut low_level = player(2, "Godmode");
    low_level.level = 10;
    assert!(world.spawn_character(low_level, 12, 10));

    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), "advice");
    }
    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    // The fee line wraps "buy advice" in `COL_LIGHT_BLUE`/`COL_RESET`
    // markers (`gwendylon.c:3058-3060`); goes out via `npc_quiet_say_bytes`.
    let texts = world.drain_pending_area_text_bytes();
    // 10^3 / 100.0 = 10.00G.
    assert!(texts
        .iter()
        .any(|text| String::from_utf8_lossy(&text.message).contains("10.00G")));
    assert!(texts
        .iter()
        .any(|text| text.message.windows(13).any(|w| w == b"\xb0c4buy advice")));

    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    let mut high_level = player(2, "Godmode");
    high_level.level = 80;
    assert!(world.spawn_character(high_level, 12, 10));
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), "advice");
    }
    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("cannot help thee")));
}

#[test]
fn james_buy_advice_without_enough_money_declines() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    let mut poor = player(2, "Godmode");
    poor.level = 10;
    poor.gold = 0;
    assert!(world.spawn_character(poor, 12, 10));

    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), "buy advice");
    }
    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("dost not have enough money")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 0);
}

#[test]
fn james_buy_advice_charges_money_and_produces_raise_hint() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    let mut buyer = player(2, "Godmode");
    buyer.level = 10;
    buyer.gold = 10_000;
    // Only V_ATTACK is raisable; every other value stays 0 (`can_raise`
    // requires a nonzero bare value), so it dominates the weighted
    // priority computation (`mr == raise[V_ATTACK]`).
    buyer.values[1][CharacterValue::Attack as usize] = 10;
    assert!(world.spawn_character(buyer, 12, 10));

    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), "buy advice");
    }
    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);

    // C `take_money(co, 10*10*10)` = 1000 raw money units charged.
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 9_000);

    let system_texts: Vec<String> = world
        .drain_pending_system_texts()
        .into_iter()
        .filter(|text| text.character_id == CharacterId(2))
        .map(|text| text.message)
        .collect();
    assert!(system_texts
        .iter()
        .any(|message| message.contains("You should definitely raise Attack.")));
    assert!(system_texts
        .iter()
        .any(|message| message.contains("very well balanced indeed")));
    assert!(system_texts
        .iter()
        .any(|message| message.contains("Please rely on your own judgement")));
}

#[test]
fn james_buy_advice_with_nothing_raisable_stays_silent() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    let mut buyer = player(2, "Godmode");
    buyer.level = 10;
    buyer.gold = 10_000;
    assert!(world.spawn_character(buyer, 12, 10));

    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), "buy advice");
    }
    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    // C `if (mr == 0) return 0;`: no messages at all, not even the
    // closing line.
    assert!(world
        .drain_pending_system_texts()
        .iter()
        .all(|text| text.character_id != CharacterId(2)));
}

#[test]
fn james_hardcore_command_shows_the_rules() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), "hardcore");
    }
    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hardcore is an option")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I accept the rules")));
}

#[test]
fn james_accept_rules_requires_paid_no_hardcore_and_no_exp() {
    let accept_text = "i accept the rules and wish to become a hardcore character";

    // Not a paying player.
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), accept_text);
    }
    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    assert!(!world.characters[&CharacterId(2)]
        .flags
        .contains(CharacterFlags::HARDCORE));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("not a paying player")));

    // Paid, no exp: becomes hardcore.
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    let mut paid = player(2, "Godmode");
    paid.flags |= CharacterFlags::PAID;
    paid.saves = 5;
    assert!(world.spawn_character(paid, 12, 10));
    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), accept_text);
    }
    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    let target = &world.characters[&CharacterId(2)];
    assert!(target.flags.contains(CharacterFlags::HARDCORE));
    assert_eq!(target.saves, 0);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Good luck")));
}

#[test]
fn james_raiseme_command_is_recognized_but_a_documented_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(james_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    god.gold = 0;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_text_message(CharacterId(2), "raiseme");
    }
    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    // Deliberately not ported (see the module doc comment): no gold
    // spent, no equipment granted, no text at all.
    assert_eq!(world.characters[&CharacterId(2)].gold, 0);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn james_give_returns_cursor_item_to_giver() {
    let mut world = World::default();
    let mut james = james_npc(1);
    james.cursor_item = Some(ItemId(50));
    world.add_character(james);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(james) = world.characters.get_mut(&CharacterId(1)) {
        james.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_james_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}
