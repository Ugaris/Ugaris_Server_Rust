//! `World::process_tutorial_hints` (C `tutorial()`,
//! `player_driver.c:402-711`).

use std::collections::HashMap;

use super::*;
use crate::item_driver::IID_AREA1_WOODPOTION;
use crate::player::TutorialPpd;

fn torch_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
            torch:
              name="Torch"
              sprite=50023
              flag=IF_TAKE
              flag=IF_WNLHAND
              flag=IF_USE
              driver=12
              arg="00007878"
            ;
            "#,
        )
        .unwrap();
    loader
}

/// Facts that fire nothing by default: `hints_disabled=false`, every
/// counter/timestamp at its zero default, and area1 Lydia state set to a
/// value (`1`) that satisfies neither the `Lydia` (`==0`) nor `Thief`
/// (`==4`) hint gates, so tests can flip only the fields they care about.
fn base_facts() -> TutorialPlayerFacts {
    TutorialPlayerFacts {
        hints_disabled: false,
        login_realtime_seconds: 0,
        ppd: TutorialPpd::default(),
        area1_lydia_state: 1,
        area1_lydia_seen_timer_realtime_seconds: 0,
    }
}

fn facts_map(facts: TutorialPlayerFacts) -> HashMap<CharacterId, TutorialPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(CharacterId(1), facts);
    map
}

fn player_character(id: u32) -> Character {
    let mut character = character(id);
    character.name = "Hero".into();
    character.flags |= CharacterFlags::PLAYER;
    character
}

#[test]
fn hints_disabled_fires_nothing() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    assert!(world.spawn_character(player_character(1), 10, 10));
    let mut facts = base_facts();
    facts.hints_disabled = true;
    facts.login_realtime_seconds = 3990;

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 4000);
    assert!(outcomes.is_empty());
}

#[test]
fn outer_throttle_skips_players_not_yet_due() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    assert!(world.spawn_character(player_character(1), 10, 10));
    let mut facts = base_facts();
    facts.login_realtime_seconds = 3990;
    facts.ppd.timer_realtime_seconds = 3995; // now - timer == 5, <= 20

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 4000);
    assert!(outcomes.is_empty());
}

#[test]
fn welcome_hint_fires_within_login_window() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    assert!(world.spawn_character(player_character(1), 10, 10));
    let mut facts = base_facts();
    facts.login_realtime_seconds = 3990; // now - login == 10, < 20

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Welcome));
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("Welcome to Ugaris, Hero"));
}

#[test]
fn lydia_hint_fires_for_low_level_with_lydia_state_zero() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    assert!(world.spawn_character(player_character(1), 10, 10));
    let mut facts = base_facts();
    facts.area1_lydia_state = 0;

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 1000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Lydia));
    let texts = world.drain_pending_system_texts();
    assert!(texts[0].message.contains("James asked you to help Lydia"));
}

#[test]
fn thief_hint_fires_for_lydia_state_four() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    assert!(world.spawn_character(player_character(1), 10, 10));
    let mut facts = base_facts();
    facts.area1_lydia_state = 4;

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 1000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Thief));
    let texts = world.drain_pending_system_texts();
    assert!(texts[0]
        .message
        .contains("find the thieves who stole her potion"));
}

#[test]
fn thief_hint_is_blocked_once_the_woodpotion_is_carried() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    let mut hero = player_character(1);
    let mut potion = item(50, ItemFlags::TAKE);
    potion.template_id = IID_AREA1_WOODPOTION;
    hero.inventory[INVENTORY_START_INVENTORY] = Some(ItemId(50));
    assert!(world.spawn_character(hero, 10, 10));
    world.items.insert(ItemId(50), potion);
    let mut facts = base_facts();
    facts.area1_lydia_state = 4;

    let mut loader = torch_loader();
    // area 2 (not area 1) so the (area-1-only) battle hint can't preempt.
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 2, 1000);
    assert!(outcomes.is_empty());
}

#[test]
fn torch_hint_prompts_lighting_an_equipped_unlit_torch() {
    let mut world = World::default();
    // Tile stays dark (default MapTile light/daylight both 0).
    let mut hero = player_character(1);
    hero.inventory[worn_slot::LEFT_HAND] = Some(ItemId(50));
    assert!(world.spawn_character(hero, 10, 10));
    let mut torch = item(50, ItemFlags::TAKE | ItemFlags::USE);
    torch.driver = IDR_TORCH;
    world.items.insert(ItemId(50), torch);
    let mut facts = base_facts();
    facts.login_realtime_seconds = 0;

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Torch));
    let texts = world.drain_pending_system_texts();
    assert!(texts[0].message.contains("light the torch you're holding"));
    let specials = world.drain_pending_player_specials();
    assert_eq!(specials.len(), 1);
    assert_eq!(specials[0].opt1, 5);
}

#[test]
fn torch_hint_creates_a_torch_into_the_empty_left_hand() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    let facts = base_facts();

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Torch));

    let hero = world.characters.get(&CharacterId(1)).unwrap();
    let torch_id = hero.inventory[worn_slot::LEFT_HAND].expect("torch granted to left hand");
    let torch = world.items.get(&torch_id).expect("torch item exists");
    assert_eq!(torch.driver, IDR_TORCH);
    assert!(hero.cursor_item.is_none());
}

#[test]
fn torch_hint_creates_a_torch_on_the_cursor_when_right_hand_is_twohanded() {
    let mut world = World::default();
    let mut hero = player_character(1);
    hero.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(60));
    assert!(world.spawn_character(hero, 10, 10));
    let mut twohander = item(60, ItemFlags::TAKE | ItemFlags::WNTWOHANDED);
    twohander.driver = 0;
    world.items.insert(ItemId(60), twohander);
    let facts = base_facts();

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Torch));

    let hero = world.characters.get(&CharacterId(1)).unwrap();
    assert!(hero.inventory[worn_slot::LEFT_HAND].is_none());
    let torch_id = hero.cursor_item.expect("torch granted to cursor");
    let torch = world.items.get(&torch_id).expect("torch item exists");
    assert_eq!(torch.driver, IDR_TORCH);
}

#[test]
fn torch_hint_is_skipped_once_the_budget_is_spent() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    let mut facts = base_facts();
    facts.ppd.torch_cnt = 5;
    facts.ppd.shop_last_realtime_seconds = 0;

    let mut loader = torch_loader();
    // Non-area-1 so the (area-1-only) battle/chest hints can't preempt;
    // `now` stays under `TF_TIMEOUT` so the generic-hints tail (whose
    // gates would otherwise all trip at once from their shared `0`
    // defaults) can't preempt either.
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 2, 1000);
    // No torch, no other condition holds (empty inventory/no merchant),
    // so nothing fires - but crucially, no torch is granted either.
    assert!(outcomes.is_empty());
    let hero = world.characters.get(&CharacterId(1)).unwrap();
    assert!(hero.inventory[worn_slot::LEFT_HAND].is_none());
    assert!(hero.cursor_item.is_none());
}

#[test]
fn battle_hint_fires_for_warriors_outside_the_village() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    let mut hero = player_character(1);
    hero.flags |= CharacterFlags::WARRIOR;
    assert!(world.spawn_character(hero, 10, 10));
    let facts = base_facts();

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Battle));
    let texts = world.drain_pending_system_texts();
    assert!(texts[0].message.contains("Warcry"));
}

#[test]
fn battle_hint_fires_for_mages_needing_bless_and_shield() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    let hero = player_character(1);
    assert!(world.spawn_character(hero, 10, 10));
    let facts = base_facts();

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Battle));
    let texts = world.drain_pending_system_texts();
    assert!(texts[0].message.contains("Bless"));
}

#[test]
fn battle2_hint_fires_at_most_once() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    let hero = player_character(1);
    assert!(world.spawn_character(hero, 10, 10));
    let mut facts = base_facts();
    facts.ppd.battle_cnt = 3; // exhausted, forces the battle2 branch

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts.clone()), &mut loader, 1, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Battle2));

    // Once battle2_cnt has already been bumped to 1, it never fires
    // again (documented simplification of the C ticker/realtime bug).
    // `now` stays under `TF_TIMEOUT` so the generic-hints tail can't
    // preempt either.
    facts.ppd.battle2_cnt = 1;
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 1000);
    assert!(outcomes.is_empty());
}

#[test]
fn shop_hint_fires_while_a_merchant_window_is_open() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    let mut hero = player_character(1);
    hero.merchant = Some(CharacterId(2));
    assert!(world.spawn_character(hero, 10, 10));
    let facts = base_facts();

    let mut loader = torch_loader();
    // area 2 so the (area-1-only) battle hint can't preempt.
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 2, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Shop));
}

#[test]
fn chest_hint_fires_near_a_tutorial_chest_box() {
    let mut world = World::default();
    world.map.tile_mut(76, 150).unwrap().light = 100;
    let hero = player_character(1);
    assert!(world.spawn_character(hero, 76, 150));
    let mut facts = base_facts();
    facts.ppd.battle_cnt = 3;
    facts.ppd.battle2_cnt = 1;

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 1, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Chest));
}

#[test]
fn citem_start_is_recorded_then_the_hint_fires_after_thirty_seconds() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    let mut hero = player_character(1);
    hero.cursor_item = Some(ItemId(70));
    assert!(world.spawn_character(hero, 10, 10));
    world.items.insert(ItemId(70), item(70, ItemFlags::TAKE));
    let mut facts = base_facts();
    facts.ppd.battle_cnt = 3;
    facts.ppd.battle2_cnt = 1;

    let mut loader = torch_loader();
    // Phase A: citem_start not yet tracked - gets initialized, no hint.
    // `now` stays under `TF_TIMEOUT` so the generic-hints tail can't
    // preempt.
    let outcomes = world.process_tutorial_hints(&facts_map(facts.clone()), &mut loader, 2, 100);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, None);
    assert_eq!(outcomes[0].citem_start, Some(100));

    // Phase B: past the 30s minimum and TF_TIMEOUT (this hint returns
    // before the generic-hints tail is ever reached, so a bigger `now`
    // is fine here).
    facts.ppd.citem_start_realtime_seconds = 100;
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 2, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Citem));
}

#[test]
fn citem_start_resets_once_the_cursor_is_emptied() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    let hero = player_character(1);
    assert!(world.spawn_character(hero, 10, 10));
    let mut facts = base_facts();
    facts.ppd.citem_start_realtime_seconds = 500;

    let mut loader = torch_loader();
    // `now` stays under `TF_TIMEOUT` so the generic-hints tail can't
    // preempt.
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 2, 100);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, None);
    assert_eq!(outcomes[0].citem_start, Some(0));
}

#[test]
fn raise_hint_suggests_sword_when_affordable_and_balanced() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    let mut hero = player_character(1);
    hero.flags |= CharacterFlags::WARRIOR;
    hero.exp = 100_000;
    hero.values[1][CharacterValue::Sword as usize] = 5;
    hero.values[1][CharacterValue::Attack as usize] = 10;
    hero.values[1][CharacterValue::Parry as usize] = 10;
    hero.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(80));
    assert!(world.spawn_character(hero, 10, 10));
    world
        .items
        .insert(ItemId(80), item(80, ItemFlags::TAKE | ItemFlags::SWORD));
    let facts = base_facts();

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 2, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Raise));
    let texts = world.drain_pending_system_texts();
    assert!(texts[0].message.contains("'Sword'"));
}

#[test]
fn generic_shift_hint_fires_after_three_idle_minutes() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().light = 100;
    let hero = player_character(1);
    assert!(world.spawn_character(hero, 10, 10));
    let facts = base_facts();

    let mut loader = torch_loader();
    let outcomes = world.process_tutorial_hints(&facts_map(facts), &mut loader, 2, 4000);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].fired, Some(TutorialHintKind::Shift));
}
