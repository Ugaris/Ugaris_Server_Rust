use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    ReskinDriverData, CDR_RESKIN, NTID_DIDSAY, NTID_TERION, NT_CHAR, NT_GIVE, NT_NPC,
};
use crate::item_driver::{drdata, set_drdata, IID_ALCHEMY_INGREDIENT};
use crate::world::reskin::{ReskinOutcomeEvent, ReskinPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn reskin_npc(id: u32) -> Character {
    let mut reskin = character(id);
    reskin.name = "Reskin".into();
    reskin.driver = CDR_RESKIN;
    reskin.driver_state = Some(CharacterDriverState::Reskin(ReskinDriverData::default()));
    // Match the spawn tile used by every test in this module so the
    // "return to post" idle branch (`secure_move_driver` toward
    // `rest_x`/`rest_y`) is a no-op instead of relocating the NPC away
    // from the position the rest of the test asserts against - `World::
    // spawn_character` (unlike zone-file loading via `zone.rs`) never
    // seeds `rest_x`/`rest_y` from the spawn position on its own.
    reskin.rest_x = 10;
    reskin.rest_y = 10;
    reskin
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
    gwendy_state: i32,
    terion_state: i32,
    logain_state: i32,
    got_bits: u32,
    killed_guild_master: bool,
) -> HashMap<CharacterId, ReskinPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        ReskinPlayerFacts {
            state,
            seen_timer: 0,
            gwendy_state,
            terion_state,
            logain_state,
            got_bits,
            killed_guild_master,
        },
    );
    map
}

fn reskin_state(world: &World, reskin_id: CharacterId) -> ReskinDriverData {
    match world
        .characters
        .get(&reskin_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Reskin(data)) => data,
        _ => panic!("expected reskin driver state"),
    }
}

#[test]
fn reskin_entry_silent_before_gwendy_state_first_skull_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // gwendy_state = 5 < GWENDYLON_STATE_FIRST_SKULL_DONE (6): stay silent.
    let events = world.process_reskin_actions(&facts(CharacterId(2), 0, 5, 4, 0, 0, false), 1, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReskinOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn reskin_entry_silent_before_terion_reaches_hordes_greet() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // gwendy_state satisfied, but terion_state = 3 < 4: still silent.
    let events = world.process_reskin_actions(&facts(CharacterId(2), 0, 6, 3, 0, 0, false), 1, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReskinOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn reskin_entry_greets_and_advances_when_both_gates_open() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_reskin_actions(&facts(CharacterId(2), 0, 6, 4, 0, 0, false), 1, 1);
    assert!(events.contains(&ReskinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Hello, Godmode! I am Reskin, the bartender.")));
    assert_eq!(
        reskin_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn reskin_state3_reminder_after_seen_timer_window() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer = 0, now = 1000 (> 600 seconds ago): reminder line fires,
    // state stays at 3 (no UpdateState).
    let mut player_facts = facts(CharacterId(2), 3, 6, 4, 0, 0, false);
    player_facts.get_mut(&CharacterId(2)).unwrap().seen_timer = 0;
    let events = world.process_reskin_actions(&player_facts, 1000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReskinOutcomeEvent::UpdateState { .. })));
    // C `case 3:` wraps "repeat" in `COL_LIGHT_BLUE`/`COL_RESET` markers
    // (`gwendylon.c:4204`); goes out via `npc_quiet_say_bytes`.
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts
        .iter()
        .any(|text| String::from_utf8_lossy(&text.message)
            .contains("Didst thou find any new ingredients")));
    assert!(texts
        .iter()
        .any(|text| text.message.windows(9).any(|w| w == b"\xb0c4repeat")));
}

#[test]
fn reskin_state3_advances_silently_once_logain_state_unlocks() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer recent (now - seen_timer <= 600) but logain_state > 8:
    // silent advance to state 4, no dialogue.
    let mut player_facts = facts(CharacterId(2), 3, 6, 4, 9, 0, false);
    player_facts.get_mut(&CharacterId(2)).unwrap().seen_timer = 100;
    let events = world.process_reskin_actions(&player_facts, 200, 1);
    assert!(events.contains(&ReskinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn reskin_state4_opens_quest_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_reskin_actions(&facts(CharacterId(2), 4, 6, 4, 9, 0, false), 1, 1);
    assert!(events.contains(&ReskinOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ReskinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
}

#[test]
fn reskin_state7_waits_silently_without_didsay_until_guild_master_killed() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Not yet killed: stays at 7, silent.
    let events = world.process_reskin_actions(&facts(CharacterId(2), 7, 6, 4, 9, 0, false), 1, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReskinOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn reskin_state7_completes_quest_but_does_not_set_didsay() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_reskin_actions(&facts(CharacterId(2), 7, 6, 4, 9, 0, true), 1, 1);
    assert!(events.contains(&ReskinOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ReskinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    // C's own bug: `case 7` never sets `didsay`, so the line is said but
    // `last_talk`/`current_victim`/`NTID_DIDSAY` never update - preserved
    // verbatim, see the module doc comment.
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("thank you for talking to the Guild Master")));
    assert_eq!(reskin_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn reskin_state8_gives_warrior_recipe_for_pure_warriors() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::WARRIOR;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_reskin_actions(&facts(CharacterId(2), 8, 6, 4, 9, 0, false), 1, 1);
    assert!(events.contains(&ReskinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Adygalah, Chrysado, Domari, Beelough")));
}

#[test]
fn reskin_state8_gives_non_warrior_recipe_for_mages_and_hybrids() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::MAGE;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_reskin_actions(&facts(CharacterId(2), 8, 6, 4, 9, 0, false), 1, 1);
    assert!(events.contains(&ReskinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("two parts Elithah, one part Firuba")));
}

#[test]
fn reskin_state9_is_a_permanent_noop() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_reskin_actions(&facts(CharacterId(2), 9, 6, 4, 9, 0, false), 1, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReskinOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn reskin_text_repeat_resets_state_bucket_5_to_7_back_to_4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.driver_state = Some(CharacterDriverState::Reskin(ReskinDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        reskin.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events =
        world.process_reskin_actions(&facts(CharacterId(2), 6, 20, 20, 20, 0, false), 1, 1);
    assert!(events.contains(&ReskinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    assert_eq!(reskin_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn reskin_text_repeat_resets_state_bucket_8_to_9_back_to_8() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.driver_state = Some(CharacterDriverState::Reskin(ReskinDriverData::default()));
        reskin.push_driver_text_message(CharacterId(2), "restart");
    }

    let events =
        world.process_reskin_actions(&facts(CharacterId(2), 9, 20, 20, 20, 0, false), 1, 1);
    assert!(events.contains(&ReskinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
}

#[test]
fn reskin_npc_message_from_terion_replies_and_faces_terion() {
    let mut world = World::default();
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(3, "Terion"), 10, 12));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_NPC, NTID_TERION, 3, 5);
    }

    world.process_reskin_actions(&HashMap::new(), 1, 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("No Terion, no beer.")));
    assert_eq!(
        reskin_state(&world, CharacterId(1)).last_talk,
        BASELINE_TICK
    );
}

fn alchemy_item(id: u32, ingredient_type: u8, value: u32, name: &str) -> Item {
    let mut ingredient = item(id, ItemFlags::empty());
    ingredient.template_id = IID_ALCHEMY_INGREDIENT;
    ingredient.name = name.into();
    ingredient.value = value;
    set_drdata(&mut ingredient, 0, ingredient_type);
    assert_eq!(drdata(&ingredient, 0), ingredient_type);
    ingredient
}

#[test]
fn reskin_give_alchemy_ingredient_pays_and_destroys_item() {
    let mut world = World::default();
    let mut reskin = reskin_npc(1);
    reskin.cursor_item = Some(ItemId(50));
    world.add_character(reskin);
    let mut ingredient = alchemy_item(50, 1, 10, "Flower");
    ingredient.carried_by = Some(CharacterId(1));
    world.add_item(ingredient);
    world.add_character(player(2, "Godmode"));

    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let player_facts = facts(CharacterId(2), 0, 0, 0, 0, 0, false);
    let events = world.process_reskin_actions(&player_facts, 1, 1);

    assert!(events.contains(&ReskinOutcomeEvent::UpdateGotBits {
        player_id: CharacterId(2),
        value: 0b10,
    }));
    assert!(events.contains(&ReskinOutcomeEvent::GoldEarned {
        player_id: CharacterId(2),
        amount: 50,
    }));
    assert!(!events.iter().any(|event| matches!(
        event,
        ReskinOutcomeEvent::WellPaidGathererAchievement { .. }
    )));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, 50);
    assert!(world.items.get(&ItemId(50)).is_none());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Ah, a nice Flower thou found there")));
}

#[test]
fn reskin_give_alchemy_ingredient_awards_achievement_on_last_bit() {
    let mut world = World::default();
    let mut reskin = reskin_npc(1);
    reskin.cursor_item = Some(ItemId(50));
    world.add_character(reskin);
    let mut ingredient = alchemy_item(50, 1, 10, "Flower");
    ingredient.carried_by = Some(CharacterId(1));
    world.add_item(ingredient);
    world.add_character(player(2, "Godmode"));

    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // Already has every bit except bit 1 (0x1FFFFFE with bit 1 cleared).
    let almost_all_bits = 0x1FFF_FFEu32 & !0b10;
    let player_facts = facts(CharacterId(2), 0, 0, 0, 0, almost_all_bits, false);
    let events = world.process_reskin_actions(&player_facts, 1, 1);

    assert!(events.contains(&ReskinOutcomeEvent::UpdateGotBits {
        player_id: CharacterId(2),
        value: 0x1FFF_FFE,
    }));
    assert!(
        events.contains(&ReskinOutcomeEvent::WellPaidGathererAchievement {
            player_id: CharacterId(2),
        })
    );
}

#[test]
fn reskin_give_alchemy_ingredient_declines_when_below_level_gate() {
    let mut world = World::default();
    let mut reskin = reskin_npc(1);
    reskin.cursor_item = Some(ItemId(50));
    world.add_character(reskin);
    // Type 24 (earth stone) requires level >= 80.
    let mut ingredient = alchemy_item(50, 24, 500, "Earth Stone");
    ingredient.carried_by = Some(CharacterId(1));
    world.add_item(ingredient);
    let mut godmode = player(2, "Godmode");
    godmode.level = 10;
    world.add_character(godmode);

    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let player_facts = facts(CharacterId(2), 0, 0, 0, 0, 0, false);
    let events = world.process_reskin_actions(&player_facts, 1, 1);

    // Declined: no payment/bit update, item handed back (not destroyed).
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReskinOutcomeEvent::UpdateGotBits { .. })));
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReskinOutcomeEvent::GoldEarned { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, 0);
    assert_eq!(godmode.inventory[30], Some(ItemId(50)));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("very nice stone")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
}

#[test]
fn reskin_give_alchemy_ingredient_already_turned_in_hands_it_back() {
    let mut world = World::default();
    let mut reskin = reskin_npc(1);
    reskin.cursor_item = Some(ItemId(50));
    world.add_character(reskin);
    let mut ingredient = alchemy_item(50, 1, 10, "Flower");
    ingredient.carried_by = Some(CharacterId(1));
    world.add_item(ingredient);
    world.add_character(player(2, "Godmode"));

    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // Bit 1 already set: "brought this one before".
    let player_facts = facts(CharacterId(2), 0, 0, 0, 0, 0b10, false);
    let events = world.process_reskin_actions(&player_facts, 1, 1);

    assert!(!events
        .iter()
        .any(|event| matches!(event, ReskinOutcomeEvent::GoldEarned { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.inventory[30], Some(ItemId(50)));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("brought me this one before")));
}

#[test]
fn reskin_give_non_alchemy_item_hands_it_back() {
    let mut world = World::default();
    let mut reskin = reskin_npc(1);
    reskin.cursor_item = Some(ItemId(50));
    world.add_character(reskin);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_reskin_actions(&HashMap::new(), 1, 1);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.inventory[30], Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn reskin_didsay_broadcast_throttles_next_reskin_tick() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(reskin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(reskin) = world.characters.get_mut(&CharacterId(1)) {
        reskin.driver_state = Some(CharacterDriverState::Reskin(ReskinDriverData {
            last_talk: 0,
            current_victim: None,
        }));
        reskin.push_driver_message(NT_NPC, NTID_DIDSAY, 99, 0);
        reskin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_reskin_actions(&facts(CharacterId(2), 0, 6, 4, 0, 0, false), 1, 1);
    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(
        reskin_state(&world, CharacterId(1)).last_talk,
        BASELINE_TICK
    );
}
