use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    CharacterDriverMessage, CDR_NOMAD, NTID_DICE, NT_CHAR, NT_GIVE, NT_NPC, NT_TEXT,
};
use crate::item_driver::{
    IID_AREA19_KIR, IID_AREA19_KIRLETTER, IID_AREA19_SALT, IID_AREA19_WOLFSSKIN,
    IID_AREA19_WOLFSSKIN2,
};
use crate::world::npc::area19::{
    parse_nomad_driver_args, NomadDriverData, NomadOutcomeEvent, NomadPlayerFacts, TM_TRIBE1,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn nomad_npc(id: u32, name: &str, nr: i32) -> Character {
    let mut nomad = character(id);
    nomad.name = name.into();
    nomad.driver = CDR_NOMAD;
    nomad.driver_state = Some(CharacterDriverState::Nomad(NomadDriverData {
        nr,
        dice_skill: 0,
        min_bet: 25,
        max_bet: 200,
        max_loss: 2000,
        ..Default::default()
    }));
    nomad
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(player_id: CharacterId, nr: usize, state: i32) -> HashMap<CharacterId, NomadPlayerFacts> {
    let mut nomad_state = [0i32; 10];
    nomad_state[nr] = state;
    let mut map = HashMap::new();
    map.insert(
        player_id,
        NomadPlayerFacts {
            nomad_state,
            nomad_win: [0i32; 10],
            tribe_member: 0,
            open_bet: 0,
            open_roll: (0, 0, 0),
        },
    );
    map
}

fn nomad_state(world: &World, nomad_id: CharacterId) -> NomadDriverData {
    match world
        .characters
        .get(&nomad_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Nomad(data)) => data,
        _ => panic!("expected nomad driver state"),
    }
}

fn salt_item(id: u32, amount: u32) -> Item {
    let mut salt = item(id, ItemFlags::empty());
    salt.name = "Salt".into();
    salt.template_id = IID_AREA19_SALT;
    salt.value = 10 * amount;
    salt.driver_data = amount.to_le_bytes().to_vec();
    salt
}

#[test]
fn parse_nomad_driver_args_reads_every_field() {
    let data = parse_nomad_driver_args("nr=1;diceskill=2;minbet=25;maxbet=200;maxloss=2000;");
    assert_eq!(
        data,
        NomadDriverData {
            nr: 1,
            dice_skill: 2,
            min_bet: 25,
            max_bet: 200,
            max_loss: 2000,
            ..Default::default()
        }
    );
}

#[test]
fn nomad_1_state0_greets_and_opens_quest_32() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nomad_npc(1, "Kalanur", 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 1, 0), 19);
    assert!(events.contains(&NomadOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest_id: 32,
    }));
    assert!(events.contains(&NomadOutcomeEvent::UpdateNomadState {
        player_id: CharacterId(2),
        nr: 1,
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Sul vana ley, Godmode. I am Kalanur.")));
}

#[test]
fn nomad_1_state8_is_silent_but_still_updates_last_talk() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nomad_npc(1, "Kalanur", 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 1, 8), 19);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts.is_empty());
    assert_eq!(
        nomad_state(&world, CharacterId(1)).last_talk_tick,
        BASELINE_TICK
    );
}

#[test]
fn nomad_2_refuses_greeting_a_non_tribe_member() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nomad_npc(1, "Irakar", 2), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 2, 0), 19);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn nomad_text_repeat_resets_state_to_zero_for_state_below_nine() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nomad_npc(1, "Kalanur", 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 1, 5), 19);
    assert!(events.contains(&NomadOutcomeEvent::UpdateNomadState {
        player_id: CharacterId(2),
        nr: 1,
        new_state: 0,
    }));
}

#[test]
fn nomad_text_hello_gets_the_canned_greeting() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nomad_npc(1, "Kalanur", 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("hello".to_string()),
        });
    }

    world.process_nomad_actions(&facts(CharacterId(2), 1, 5), 19);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Sul vana ley, Godmode.")));
}

#[test]
fn nomad_1_give_exactly_100_salt_grants_tribe_membership() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kalanur", 1);
    nomad.cursor_item = Some(ItemId(50));
    world.add_character(nomad);
    let mut salt = salt_item(50, 100);
    salt.carried_by = Some(CharacterId(1));
    world.add_item(salt);
    world.add_character(player(2, "Godmode"));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 1, 8), 19);
    assert!(events.contains(&NomadOutcomeEvent::SetTribeMember {
        player_id: CharacterId(2),
        flag: TM_TRIBE1,
    }));
    assert!(events.contains(&NomadOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest_id: 32,
    }));
    assert!(events.contains(&NomadOutcomeEvent::UpdateNomadState {
        player_id: CharacterId(2),
        nr: 1,
        new_state: 9,
    }));
    assert!(!world.items.contains_key(&ItemId(50)));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Welcome to the tribe of the Vana Kiru")));
}

#[test]
fn nomad_1_give_insufficient_salt_hands_it_back() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kalanur", 1);
    nomad.cursor_item = Some(ItemId(50));
    world.add_character(nomad);
    let mut salt = salt_item(50, 50);
    salt.carried_by = Some(CharacterId(1));
    world.add_item(salt);
    world.add_character(player(2, "Godmode"));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 1, 8), 19);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("This is not enough")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn nomad_1_give_wolf_skin_queues_a_salt_exchange_event() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kalanur", 1);
    nomad.cursor_item = Some(ItemId(50));
    world.add_character(nomad);
    let mut skin = item(50, ItemFlags::empty());
    skin.template_id = IID_AREA19_WOLFSSKIN;
    skin.driver_data = 3u32.to_le_bytes().to_vec();
    skin.carried_by = Some(CharacterId(1));
    world.add_item(skin);
    world.add_character(player(2, "Godmode"));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 1, 8), 19);
    assert!(events.contains(&NomadOutcomeEvent::GiveSaltForSkin {
        nomad_id: CharacterId(1),
        player_id: CharacterId(2),
        skin_item_id: ItemId(50),
        amount: 15,
    }));
}

#[test]
fn nomad_1_give_white_wolf_skin_uses_the_higher_exchange_rate() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kalanur", 1);
    nomad.cursor_item = Some(ItemId(50));
    world.add_character(nomad);
    let mut skin = item(50, ItemFlags::empty());
    skin.template_id = IID_AREA19_WOLFSSKIN2;
    skin.driver_data = 2u32.to_le_bytes().to_vec();
    skin.carried_by = Some(CharacterId(1));
    world.add_item(skin);
    world.add_character(player(2, "Godmode"));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 1, 8), 19);
    assert!(events.contains(&NomadOutcomeEvent::GiveSaltForSkin {
        nomad_id: CharacterId(1),
        player_id: CharacterId(2),
        skin_item_id: ItemId(50),
        amount: 40,
    }));
}

#[test]
fn nomad_4_give_kirletter_completes_quest_33() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kir Laas", 4);
    nomad.cursor_item = Some(ItemId(50));
    world.add_character(nomad);
    let mut letter = item(50, ItemFlags::empty());
    letter.template_id = IID_AREA19_KIRLETTER;
    letter.carried_by = Some(CharacterId(1));
    world.add_item(letter);
    world.add_character(player(2, "Godmode"));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 4, 2), 19);
    assert!(events.contains(&NomadOutcomeEvent::UpdateNomadState {
        player_id: CharacterId(2),
        nr: 4,
        new_state: 4,
    }));
    assert!(events.contains(&NomadOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest_id: 33,
    }));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn nomad_5_give_first_statue_completes_quest_34() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kir Garan", 5);
    nomad.cursor_item = Some(ItemId(50));
    world.add_character(nomad);
    let mut statue = item(50, ItemFlags::empty());
    statue.template_id = IID_AREA19_KIR;
    statue.carried_by = Some(CharacterId(1));
    world.add_item(statue);
    world.add_character(player(2, "Godmode"));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 5, 2), 19);
    assert!(events.contains(&NomadOutcomeEvent::UpdateNomadState {
        player_id: CharacterId(2),
        nr: 5,
        new_state: 4,
    }));
    assert!(events.contains(&NomadOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest_id: 34,
    }));
}

#[test]
fn nomad_5_give_second_statue_restores_lost_exp() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kir Garan", 5);
    nomad.cursor_item = Some(ItemId(50));
    world.add_character(nomad);
    let mut statue = item(50, ItemFlags::empty());
    statue.template_id = IID_AREA19_KIR;
    statue.carried_by = Some(CharacterId(1));
    world.add_item(statue);
    let mut godmode = player(2, "Godmode");
    godmode.exp_used = 1000;
    godmode.exp = 400;
    world.add_character(godmode);

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 5, 4), 19);
    // C `diff = exp_used - exp; give_exp(co, diff/2)` for non-hardcore.
    assert!(events.contains(&NomadOutcomeEvent::GiveExp {
        player_id: CharacterId(2),
        base_exp: 300,
    }));
}

#[test]
fn nomad_5_give_rejects_a_statue_when_no_exp_is_lost() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kir Garan", 5);
    nomad.cursor_item = Some(ItemId(50));
    world.add_character(nomad);
    let mut statue = item(50, ItemFlags::empty());
    statue.template_id = IID_AREA19_KIR;
    statue.carried_by = Some(CharacterId(1));
    world.add_item(statue);
    let mut godmode = player(2, "Godmode");
    // C `if (ch[co].exp > ch[co].exp_used)` - nothing lost since current
    // `exp` already exceeds the historical `exp_used` high-water mark.
    godmode.exp_used = 400;
    godmode.exp = 500;
    world.add_character(godmode);

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_nomad_actions(&facts(CharacterId(2), 5, 4), 19);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("dost not have lost any experience")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn count_salt_sums_every_carried_stack_past_the_worn_slots() {
    let mut world = World::default();
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    godmode.inventory[31] = Some(ItemId(51));
    world.add_character(godmode);
    let mut salt1 = salt_item(50, 40);
    salt1.carried_by = Some(CharacterId(2));
    world.add_item(salt1);
    let mut salt2 = salt_item(51, 25);
    salt2.carried_by = Some(CharacterId(2));
    world.add_item(salt2);

    assert_eq!(world.count_salt(CharacterId(2)), 65);
}

#[test]
fn remove_salt_destroys_a_fully_consumed_stack_and_shrinks_a_partial_one() {
    let mut world = World::default();
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    godmode.inventory[31] = Some(ItemId(51));
    world.add_character(godmode);
    let mut salt1 = salt_item(50, 10);
    salt1.carried_by = Some(CharacterId(2));
    world.add_item(salt1);
    let mut salt2 = salt_item(51, 100);
    salt2.carried_by = Some(CharacterId(2));
    world.add_item(salt2);

    world.remove_salt(CharacterId(2), 30);

    assert!(!world.items.contains_key(&ItemId(50)));
    let remaining = world.items.get(&ItemId(51)).unwrap();
    assert_eq!(world.salt_amount(ItemId(51)), 80);
    assert_eq!(remaining.value, 800);
}

fn bet_facts(player_id: CharacterId, nomad_state_1: i32) -> HashMap<CharacterId, NomadPlayerFacts> {
    let mut nomad_state = [0i32; 10];
    nomad_state[1] = nomad_state_1;
    let mut map = HashMap::new();
    map.insert(
        player_id,
        NomadPlayerFacts {
            nomad_state,
            nomad_win: [0i32; 10],
            tribe_member: 0,
            open_bet: 0,
            open_roll: (0, 0, 0),
        },
    );
    map
}

#[test]
fn nomad_bet_rejects_a_bet_below_the_minimum() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nomad_npc(1, "Kalanur", 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("bet 10".to_string()),
        });
    }

    // `nomad_state[1] >= 9` is `CDR_NOMAD`'s own "already a tribe member"
    // gate for the `"bet "` trigger (`nomad.c:1041-1046`).
    world.process_nomad_actions(&bet_facts(CharacterId(2), 9), 19);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("too cheap")));
}

#[test]
fn nomad_bet_refuses_a_stranger_who_has_not_joined_the_tribe() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nomad_npc(1, "Kalanur", 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("bet 50".to_string()),
        });
    }

    world.process_nomad_actions(&bet_facts(CharacterId(2), 0), 19);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I do not play with strangers")));
}

#[test]
fn nomad_bet_accepts_a_valid_bet_and_records_the_player() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nomad_npc(1, "Kalanur", 1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    assert!(world.spawn_character(godmode, 11, 10));
    let mut salt = salt_item(50, 100);
    salt.carried_by = Some(CharacterId(2));
    world.add_item(salt);

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("bet 50".to_string()),
        });
    }

    world.process_nomad_actions(&bet_facts(CharacterId(2), 9), 19);
    assert_eq!(
        nomad_state(&world, CharacterId(1)).play_with,
        Some(CharacterId(2))
    );
    assert_eq!(nomad_state(&world, CharacterId(1)).bet, 50);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("USE the dice")));
}

#[test]
fn nomad_roll_win_removes_the_bet_and_credits_nomad_win() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kalanur", 1);
    nomad.driver_state = Some(CharacterDriverState::Nomad(NomadDriverData {
        nr: 1,
        bet: 50,
        my_throw: 15,
        play_with: Some(CharacterId(2)),
        ..Default::default()
    }));
    world.add_character(nomad);
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    world.add_character(godmode);
    let mut salt = salt_item(50, 100);
    salt.carried_by = Some(CharacterId(2));
    world.add_item(salt);

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_NPC, NTID_DICE, 2, 10);
    }

    let events = world.process_nomad_actions(&bet_facts(CharacterId(2), 9), 19);
    assert!(events.contains(&NomadOutcomeEvent::AdjustNomadWin {
        player_id: CharacterId(2),
        nr: 1,
        delta: 50,
    }));
    assert_eq!(world.count_salt(CharacterId(2)), 50);
    assert_eq!(nomad_state(&world, CharacterId(1)).play_with, None);
}

#[test]
fn nomad_roll_loss_queues_a_salt_payout() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kalanur", 1);
    nomad.driver_state = Some(CharacterDriverState::Nomad(NomadDriverData {
        nr: 1,
        bet: 50,
        my_throw: 5,
        play_with: Some(CharacterId(2)),
        ..Default::default()
    }));
    world.add_character(nomad);
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    world.add_character(godmode);
    let mut salt = salt_item(50, 100);
    salt.carried_by = Some(CharacterId(2));
    world.add_item(salt);

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        nomad.push_driver_message(NT_NPC, NTID_DICE, 2, 20);
    }

    let events = world.process_nomad_actions(&bet_facts(CharacterId(2), 9), 19);
    assert!(events.contains(&NomadOutcomeEvent::PaySaltWinnings {
        nomad_id: CharacterId(1),
        player_id: CharacterId(2),
        amount: 50,
        nr: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Dang. Lost again.")));
}

#[test]
fn nomad_npc_message_dice_roll_only_reacts_to_the_current_opponent() {
    let mut world = World::default();
    let mut nomad = nomad_npc(1, "Kalanur", 1);
    nomad.driver_state = Some(CharacterDriverState::Nomad(NomadDriverData {
        nr: 1,
        bet: 50,
        my_throw: 15,
        play_with: Some(CharacterId(2)),
        ..Default::default()
    }));
    world.add_character(nomad);
    world.add_character(player(2, "Godmode"));
    world.add_character(player(3, "Bystander"));

    if let Some(nomad) = world.characters.get_mut(&CharacterId(1)) {
        // Bystander (3) isn't who Kalanur is playing with (2) - the C
        // `co == dat->play_with` guard (`nomad.c:1129`) ignores it.
        nomad.push_driver_message(NT_NPC, NTID_DICE, 3, 10);
    }

    let events = world.process_nomad_actions(&bet_facts(CharacterId(3), 9), 19);
    assert!(events.is_empty());
    assert_eq!(
        nomad_state(&world, CharacterId(1)).play_with,
        Some(CharacterId(2))
    );
}
