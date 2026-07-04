use std::collections::HashMap;

use super::*;
use crate::character_driver::{GateWelcomeDriverData, CDR_GATE_WELCOME, NT_CHAR, NT_GIVE};
use crate::world::gatekeeper::{GateWelcomeOutcomeEvent, GateWelcomePlayerFacts};

const TALK_MIN: u64 = TICKS_PER_SECOND * 5;
const TALK_VICTIM: u64 = TICKS_PER_SECOND * 10;
const RETURN_TO_POST: u64 = TICKS_PER_SECOND * 30;

fn gate_npc(id: u32) -> Character {
    let mut gate = character(id);
    gate.name = "Ishtar".into();
    gate.driver = CDR_GATE_WELCOME;
    gate.driver_state = Some(CharacterDriverState::GateWelcome(
        GateWelcomeDriverData::default(),
    ));
    gate
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    welcome_state: i32,
    needs_lab: bool,
) -> HashMap<CharacterId, GateWelcomePlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        GateWelcomePlayerFacts {
            welcome_state,
            needs_lab,
        },
    );
    map
}

fn gate_state(world: &World, gate_id: CharacterId) -> GateWelcomeDriverData {
    match world
        .characters
        .get(&gate_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::GateWelcome(data)) => data,
        _ => panic!("expected gate-welcome driver state"),
    }
}

#[test]
fn gate_welcome_greets_visible_player_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(TALK_MIN);
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false), 0);
    assert_eq!(
        events,
        vec![GateWelcomeOutcomeEvent::UpdateWelcomeState {
            player_id: CharacterId(2),
            new_state: 1
        }]
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Be greeted, Godmode")));
    assert_eq!(
        gate_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn gate_welcome_ignores_players_out_of_range() {
    let mut world = World::default();
    world.map.tile_mut(25, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 25, 10));

    world.tick = Tick(TALK_MIN);
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false), 0);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn gate_welcome_throttles_repeated_greetings() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.driver_state = Some(CharacterDriverState::GateWelcome(GateWelcomeDriverData {
            last_talk: 0,
            current_victim: Some(CharacterId(2)),
            amgivingback: 0,
        }));
        gate.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.tick = Tick(TALK_MIN - 1);

    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false), 0);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn gate_welcome_ignores_a_different_player_while_a_victim_conversation_is_fresh() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 12, 10));

    // Within `TALK_MIN..TALK_VICTIM` of a real `current_victim`, C skips
    // any other player entirely (`gatekeeper.c:454-457`).
    world.tick = Tick(TALK_MIN);
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.driver_state = Some(CharacterDriverState::GateWelcome(GateWelcomeDriverData {
            last_talk: 0,
            current_victim: Some(CharacterId(2)),
            amgivingback: 0,
        }));
        gate.push_driver_message(NT_CHAR, 3, 0, 0);
    }
    assert!(TALK_MIN < TALK_VICTIM);

    let events = world.process_gate_welcome_actions(&facts(CharacterId(3), 0, false), 0);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn gate_welcome_needs_lab_says_labyrinth_message_and_waits() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(TALK_MIN);
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 2, true), 0);
    assert_eq!(
        events,
        vec![GateWelcomeOutcomeEvent::UpdateWelcomeState {
            player_id: CharacterId(2),
            new_state: 3
        }]
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("solve the Labyrinth built by Ishtar")));
}

#[test]
fn gate_welcome_replies_to_small_talk_keyword() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false), 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn gate_welcome_repeat_resets_welcome_state_below_seven() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 6, false), 0);
    assert_eq!(
        events,
        vec![GateWelcomeOutcomeEvent::UpdateWelcomeState {
            player_id: CharacterId(2),
            new_state: 0
        }]
    );
}

#[test]
fn gate_welcome_god_reset_clears_lab_ppd() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "reset");
    }
    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false), 0);
    assert_eq!(
        events,
        vec![GateWelcomeOutcomeEvent::ResetLabPpd {
            player_id: CharacterId(2)
        }]
    );
}

#[test]
fn gate_welcome_non_god_reset_is_ignored() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "reset");
    }
    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false), 0);
    assert!(events.is_empty());
}

#[test]
fn gate_welcome_gives_item_back_with_flavor_text_once() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.cursor_item = Some(ItemId(900));
        gate.push_driver_message(NT_GIVE, 2, 900, 0);
    }

    world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false), 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Thou hast better use for this than I do")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(900))
    );
    // `amgivingback` resets to `0` every tick (C `gatekeeper.c:621`), so a
    // second give-back on a later tick shows the flavor text again.
    assert_eq!(gate_state(&world, CharacterId(1)).amgivingback, 0);
}

#[test]
fn gate_welcome_class_choice_rejects_unpaid_player() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "arch warrior");
    }
    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 6, false), 0);
    // Not paid: C's `enter_test` returns early via `log_char`, not `say`,
    // so no `UpdateWelcomeState`/area-text event fires.
    assert!(events.is_empty());
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.character_id == CharacterId(2)
        && text.message == "Sorry, only paying players may take the test."));
}

#[test]
fn gate_welcome_class_choice_rejects_unsolved_labyrinth() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    let mut paid = player(2, "Godmode");
    paid.flags |= CharacterFlags::PAID;
    assert!(world.spawn_character(paid, 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "arch warrior");
    }
    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 6, true), 0);
    assert!(events.is_empty());
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.character_id == CharacterId(2)
        && text.message == "Sorry, you may not enter before you have solved the labyrinth."));
}

#[test]
fn gate_welcome_class_choice_rejects_noexp_mode() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    let mut paid = player(2, "Godmode");
    paid.flags |= CharacterFlags::PAID | CharacterFlags::NOEXP;
    assert!(world.spawn_character(paid, 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "arch warrior");
    }
    world.process_gate_welcome_actions(&facts(CharacterId(2), 6, false), 0);
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.character_id == CharacterId(2)
        && text.message == "Sorry, you may not enter if you have the /noexp mode turned on."));
}

#[test]
fn gate_welcome_class_choice_reports_carried_items() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    let mut paid = player(2, "Godmode");
    paid.flags |= CharacterFlags::PAID;
    paid.inventory[INVENTORY_START_INVENTORY] = Some(ItemId(1));
    assert!(world.spawn_character(paid, 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "arch warrior");
    }
    world.process_gate_welcome_actions(&facts(CharacterId(2), 6, false), 0);
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(2)
        && text.message
            == "Sorry, you may not enter while you are carrying items. You currently have 1 items."
    }));
}

#[test]
fn gate_welcome_class_choice_says_not_possible_for_invalid_class() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    let mut paid = player(2, "Godmode");
    // Already a mage: "arch warrior" is not a valid choice
    // (`gatekeeper.c:339`).
    paid.flags |= CharacterFlags::PAID | CharacterFlags::MAGE;
    assert!(world.spawn_character(paid, 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "arch warrior");
    }
    world.process_gate_welcome_actions(&facts(CharacterId(2), 6, false), 0);
    let area_texts = world.drain_pending_area_texts();
    assert!(area_texts
        .iter()
        .any(|text| text.message.contains("That is not a possible choice.")));
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn gate_welcome_class_choice_ready_emits_enter_test_ready_event() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    let mut paid = player(2, "Godmode");
    paid.flags |= CharacterFlags::PAID;
    assert!(world.spawn_character(paid, 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "arch warrior");
    }
    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 6, false), 0);
    assert_eq!(
        events,
        vec![GateWelcomeOutcomeEvent::EnterTestReady {
            player_id: CharacterId(2),
            class: 5,
        }]
    );
    assert!(world.drain_pending_system_texts().is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
    // `didsay` still fires (updates `current_victim`/`last_talk`), matching
    // C: `enter_test` always returns `1` on this path.
    assert_eq!(
        gate_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn gate_welcome_destroys_item_when_giver_inventory_is_full() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    let mut full_player = player(2, "Godmode");
    full_player.cursor_item = Some(ItemId(1));
    for slot in full_player
        .inventory
        .iter_mut()
        .skip(INVENTORY_START_INVENTORY)
    {
        *slot = Some(ItemId(1));
    }
    assert!(world.spawn_character(full_player, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.cursor_item = Some(ItemId(900));
        gate.push_driver_message(NT_GIVE, 2, 900, 0);
    }

    world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false), 0);

    assert!(world.items.get(&ItemId(900)).is_none());
}

/// C `enter_room`'s room-clear scan (`gatekeeper.c:233-240`).
#[test]
fn gate_room_is_clear_rejects_occupied_and_takeable_item_tiles() {
    let mut world = World::default();
    assert!(world.gate_room_is_clear(50, 50));

    assert!(world.spawn_character(character(1), 52, 55));
    assert!(!world.gate_room_is_clear(50, 50));
    assert!(world.remove_character(CharacterId(1)).is_some());
    assert!(world.gate_room_is_clear(50, 50));

    // A takeable item blocks the room...
    let mut takeable = item(900, ItemFlags::TAKE);
    takeable.x = 53;
    takeable.y = 56;
    world.map.tile_mut(53, 56).unwrap().item = 900;
    world.items.insert(ItemId(900), takeable);
    assert!(!world.gate_room_is_clear(50, 50));

    // ...but fixed furniture (no `IF_TAKE`) does not.
    world.items.get_mut(&ItemId(900)).unwrap().flags = ItemFlags::empty();
    assert!(world.gate_room_is_clear(50, 50));
}

/// C `take_money`/`give_money_silent` (`src/system/tool.c:1441-1449,
/// 3820-3826`).
#[test]
fn gate_take_money_and_give_money_silent_match_c() {
    let mut world = World::default();
    let mut broke = player(2, "Godmode");
    broke.gold = 50;
    assert!(world.spawn_character(broke, 10, 10));

    assert!(!world.gate_take_money(CharacterId(2), 10000));
    assert_eq!(world.characters[&CharacterId(2)].gold, 50);

    world.characters.get_mut(&CharacterId(2)).unwrap().gold = 10000;
    assert!(world.gate_take_money(CharacterId(2), 10000));
    assert_eq!(world.characters[&CharacterId(2)].gold, 0);
    assert!(world.characters[&CharacterId(2)]
        .flags
        .contains(CharacterFlags::ITEMS));

    world.gate_give_money_silent(CharacterId(2), 10000);
    assert_eq!(world.characters[&CharacterId(2)].gold, 10000);
}

/// The player-side tail of `enter_room`'s success path (`gatekeeper.c:
/// 277-303`): teleport, spell-slot stripping, notices, and HP/mana/
/// endurance/`regen_ticker` reset.
#[test]
fn gate_finish_enter_room_teleports_strips_spells_and_resets_resources() {
    let mut world = World::default();
    world.tick = Tick(500);
    let mut hero = player(2, "Godmode");
    hero.hp = 5 * POWERSCALE;
    hero.mana = 5 * POWERSCALE;
    hero.endurance = 5 * POWERSCALE;
    hero.inventory[12] = Some(ItemId(1));
    hero.inventory[29] = Some(ItemId(2));
    hero.inventory[30] = Some(ItemId(3));
    assert!(world.spawn_character(hero, 10, 10));
    world.items.insert(ItemId(1), item(1, ItemFlags::empty()));
    world.items.insert(ItemId(2), item(2, ItemFlags::empty()));
    world.items.insert(ItemId(3), item(3, ItemFlags::empty()));

    assert!(world.gate_finish_enter_room(CharacterId(2), 186, 196));

    let hero = &world.characters[&CharacterId(2)];
    assert_eq!((hero.x, hero.y), (190, 200));
    assert!(hero.inventory[12].is_none());
    assert!(hero.inventory[29].is_none());
    // Slot 30 (regular inventory, not a spell slot) is untouched.
    assert_eq!(hero.inventory[30], Some(ItemId(3)));
    assert_eq!(hero.hp, POWERSCALE);
    assert_eq!(hero.mana, POWERSCALE);
    assert_eq!(hero.endurance, POWERSCALE);
    assert_eq!(hero.regen_ticker, 500);
    assert!(world.items.get(&ItemId(1)).is_none());
    assert!(world.items.get(&ItemId(2)).is_none());
    assert!(world.items.get(&ItemId(3)).is_some());

    let texts: Vec<String> = world
        .drain_pending_system_texts()
        .into_iter()
        .map(|entry| entry.message)
        .collect();
    assert!(texts
        .iter()
        .any(|text| text == "All your spells have been removed."));
    assert!(texts.iter().any(|text| text
        .contains("use the door to the south-west to enter the room containing your opponent")));
}

/// C's mana-only-if-nonzero guard (`gatekeeper.c:299-301`): a manaless
/// character's mana stays `0`.
#[test]
fn gate_finish_enter_room_leaves_zero_mana_untouched() {
    let mut world = World::default();
    let mut hero = player(2, "Godmode");
    hero.mana = 0;
    assert!(world.spawn_character(hero, 10, 10));

    assert!(world.gate_finish_enter_room(CharacterId(2), 186, 196));

    assert_eq!(world.characters[&CharacterId(2)].mana, 0);
}

/// C `teleport_char_driver`'s "already close enough" guard
/// (`abs(dx) + abs(dy) < 2`, `drvlib.c:2652-2654`): when the player is
/// already essentially at the door tile, `enter_room` fails and the
/// caller must try the next room.
#[test]
fn gate_finish_enter_room_fails_when_already_at_target() {
    let mut world = World::default();
    let mut hero = player(2, "Godmode");
    hero.x = 190;
    hero.y = 200;
    assert!(world.spawn_character(hero, 190, 200));

    assert!(!world.gate_finish_enter_room(CharacterId(2), 186, 196));
    assert_eq!(
        (
            world.characters[&CharacterId(2)].x,
            world.characters[&CharacterId(2)].y
        ),
        (190, 200)
    );
}

/// C `gate_welcome_driver`'s idle "return to post" safety net
/// (`gatekeeper.c:627-631`): once `TICKS * 30` have passed since the last
/// time the NPC spoke, it walks back toward its spawn tile (`rest_x`/
/// `rest_y`, substituting C's `tmpx`/`tmpy`).
#[test]
fn gate_welcome_returns_to_post_after_thirty_seconds_idle() {
    let mut world = World::default();
    let mut gate = gate_npc(1);
    gate.rest_x = 10;
    gate.rest_y = 10;
    gate.values[0][CharacterValue::Speed as usize] = 50;
    assert!(world.spawn_character(gate, 12, 10));
    world.tick = Tick(RETURN_TO_POST + 1);

    world.process_gate_welcome_actions(&HashMap::new(), 0);

    let gate = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(gate.action, action::WALK);
    assert_eq!((gate.tox, gate.toy), (11, 10));
}

/// The same idle check must not fire while `last_talk` is recent (C's
/// `dat->last_talk + TICKS*30 < ticker` guard).
#[test]
fn gate_welcome_stays_put_shortly_after_talking() {
    let mut world = World::default();
    let mut gate = gate_npc(1);
    gate.rest_x = 10;
    gate.rest_y = 10;
    gate.values[0][CharacterValue::Speed as usize] = 50;
    assert!(world.spawn_character(gate, 12, 10));
    world.tick = Tick(RETURN_TO_POST + 1);
    if let Some(CharacterDriverState::GateWelcome(data)) = world
        .characters
        .get_mut(&CharacterId(1))
        .and_then(|c| c.driver_state.as_mut())
    {
        data.last_talk = world.tick.0;
    }

    world.process_gate_welcome_actions(&HashMap::new(), 0);

    let gate = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(gate.action, 0);
    assert_eq!((gate.x, gate.y), (12, 10));
}
