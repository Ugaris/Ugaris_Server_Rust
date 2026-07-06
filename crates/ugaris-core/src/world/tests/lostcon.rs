use super::*;

fn player_character(id: u32) -> Character {
    let mut character = character(id);
    character.flags |= CharacterFlags::PLAYER;
    character
}

#[test]
fn enter_lostcon_sets_driver_and_arms_deadline() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));

    assert!(world.enter_lostcon(CharacterId(1), 7_200));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.driver, CDR_LOSTCON);
    assert_eq!(
        character.driver_state,
        Some(CharacterDriverState::Lostcon(LostconDriverData {
            deadline: 7_200
        }))
    );
    assert!(world.is_lostcon(CharacterId(1)));
}

#[test]
fn enter_lostcon_returns_false_for_missing_character() {
    let mut world = World::default();
    assert!(!world.enter_lostcon(CharacterId(99), 100));
}

#[test]
fn lingering_character_stays_on_the_map_and_is_attackable() {
    // C `kick_player` does not call `remove_char`/`exit_char` on
    // disconnect; the character stays fully live until the lagout timer
    // expires or it is reclaimed.
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    world.enter_lostcon(CharacterId(1), 7_200);

    assert!(world.characters.contains_key(&CharacterId(1)));
    let tile = world.map.tile(10, 10).unwrap();
    assert_eq!(tile.character, 1);
}

#[test]
fn reclaim_lostcon_clears_driver_and_state() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    world.enter_lostcon(CharacterId(1), 7_200);

    assert!(world.reclaim_lostcon(CharacterId(1)));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.driver, 0);
    assert_eq!(character.driver_state, None);
    assert!(!world.is_lostcon(CharacterId(1)));
}

#[test]
fn reclaim_lostcon_is_a_no_op_when_not_lingering() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));

    assert!(!world.reclaim_lostcon(CharacterId(1)));
    assert!(!world.reclaim_lostcon(CharacterId(404)));
}

#[test]
fn expired_lostcon_characters_matches_deadline_and_driver() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    assert!(world.spawn_character(player_character(2), 11, 11));
    assert!(world.spawn_character(player_character(3), 12, 12));
    world.enter_lostcon(CharacterId(1), 100);
    world.enter_lostcon(CharacterId(2), 200);
    // Character 3 never disconnected: still player-controlled.

    let expired = world.expired_lostcon_characters(150);
    assert_eq!(expired, vec![CharacterId(1)]);

    let mut expired = world.expired_lostcon_characters(200);
    expired.sort_by_key(|id| id.0);
    assert_eq!(expired, vec![CharacterId(1), CharacterId(2)]);

    let expired = world.expired_lostcon_characters(50);
    assert!(expired.is_empty());
}

#[test]
fn expired_lostcon_characters_ignores_reclaimed_characters() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));
    world.enter_lostcon(CharacterId(1), 100);
    world.reclaim_lostcon(CharacterId(1));

    assert!(world.expired_lostcon_characters(200).is_empty());
}

// `process_lostcon_messages` (C `lostcon_driver`'s per-message loop,
// `src/module/lostcon.c:117-141`).

#[test]
fn process_lostcon_messages_notes_hit_and_adds_the_attacker_as_an_enemy() {
    let mut world = World::default();
    world.tick = Tick(42);
    let mut lingering = player_character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.push_driver_message(NT_GOTHIT, 2, 0, 0);
    assert!(world.spawn_character(lingering, 10, 10));
    assert!(world.spawn_character(character(2), 11, 11));

    world.process_lostcon_messages(CharacterId(1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(character.driver_messages.is_empty());
    let data = character
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.last_hit, 42);
    assert_eq!(data.enemies.len(), 1);
    assert_eq!(data.enemies[0].target_id, CharacterId(2));
    assert_eq!(data.enemies[0].priority, 1);
    assert!(data.enemies[0].visible);
    assert_eq!(data.enemies[0].last_x, 11);
    assert_eq!(data.enemies[0].last_y, 11);
}

#[test]
fn process_lostcon_messages_notes_hit_without_an_attacker_id() {
    // C: `fight_driver_note_hit(cn)` always runs on `NT_GOTHIT`; the
    // `fight_driver_add_enemy` call is skipped when `msg->dat1` (`co`) is
    // `0`.
    let mut world = World::default();
    world.tick = Tick(7);
    let mut lingering = player_character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.push_driver_message(NT_GOTHIT, 0, 0, 0);
    assert!(world.spawn_character(lingering, 10, 10));

    world.process_lostcon_messages(CharacterId(1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    let data = character
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.last_hit, 7);
    assert!(data.enemies.is_empty());
}

#[test]
fn process_lostcon_messages_ignores_sighting_and_text_messages() {
    // C's own message loop leaves `NT_CHAR`'s aggro-on-sight commented out
    // and `NT_TEXT` is a no-op comment - neither message type should touch
    // `fight_driver` at all.
    let mut world = World::default();
    let mut lingering = player_character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.push_driver_message(NT_CHAR, 2, 0, 0);
    lingering.push_driver_message(NT_TEXT, 1, 0, 1);
    assert!(world.spawn_character(lingering, 10, 10));
    assert!(world.spawn_character(character(2), 11, 11));

    world.process_lostcon_messages(CharacterId(1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(character.driver_messages.is_empty());
    assert!(character.fight_driver.is_none());
}

#[test]
fn process_lostcon_messages_is_a_no_op_for_a_normal_playing_character() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.push_driver_message(NT_GOTHIT, 2, 0, 0);
    assert!(world.spawn_character(player, 10, 10));
    assert!(world.spawn_character(character(2), 11, 11));

    world.process_lostcon_messages(CharacterId(1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.driver_messages.len(), 1);
    assert!(character.fight_driver.is_none());
}

// `lostcon_early_exit_characters` (C `lostcon_driver`'s early-exit
// gauntlet, `src/module/lostcon.c:87-104`).

fn lingering_lostcon(id: u32) -> Character {
    let mut character = player_character(id);
    character.driver = CDR_LOSTCON;
    character.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    character
}

#[test]
fn lostcon_early_exit_characters_flags_a_rest_area_tile() {
    let mut world = World::default();
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::RESTAREA);
    assert!(world.spawn_character(lingering_lostcon(1), 10, 10));

    assert_eq!(world.lostcon_early_exit_characters(1), vec![CharacterId(1)]);
}

#[test]
fn lostcon_early_exit_characters_flags_an_arena_tile_outside_area_34() {
    let mut world = World::default();
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::ARENA);
    assert!(world.spawn_character(lingering_lostcon(1), 10, 10));

    assert_eq!(world.lostcon_early_exit_characters(1), vec![CharacterId(1)]);
}

#[test]
fn lostcon_early_exit_characters_ignores_an_arena_tile_inside_area_34() {
    // C `lostcon.c:92-95`'s own `areaID != 34` exemption.
    let mut world = World::default();
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::ARENA);
    assert!(world.spawn_character(lingering_lostcon(1), 10, 10));

    assert!(world.lostcon_early_exit_characters(34).is_empty());
}

#[test]
fn lostcon_early_exit_characters_flags_severe_negative_karma() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    lingering.karma = -12;
    assert!(world.spawn_character(lingering, 10, 10));

    assert_eq!(world.lostcon_early_exit_characters(1), vec![CharacterId(1)]);
}

#[test]
fn lostcon_early_exit_characters_flags_moderate_negative_karma_only_when_unpaid() {
    let mut world = World::default();
    let mut unpaid = lingering_lostcon(1);
    unpaid.flags.remove(CharacterFlags::PAID);
    unpaid.karma = -5;
    assert!(world.spawn_character(unpaid, 10, 10));
    let mut paid = lingering_lostcon(2);
    paid.flags.insert(CharacterFlags::PAID);
    paid.karma = -5;
    assert!(world.spawn_character(paid, 11, 11));

    assert_eq!(world.lostcon_early_exit_characters(1), vec![CharacterId(1)]);
}

#[test]
fn lostcon_early_exit_characters_ignores_a_normal_lingering_character() {
    let mut world = World::default();
    assert!(world.spawn_character(lingering_lostcon(1), 10, 10));

    assert!(world.lostcon_early_exit_characters(1).is_empty());
}

#[test]
fn lostcon_early_exit_characters_ignores_a_reclaimed_character() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    lingering.karma = -20;
    assert!(world.spawn_character(lingering, 10, 10));
    world.reclaim_lostcon(CharacterId(1));

    assert!(world.lostcon_early_exit_characters(1).is_empty());
}

// `process_lostcon_self_care_precascade` (C `lostcon_driver`'s low-hp-
// heal/low-mana-potion/low-magicshield pre-cascade,
// `src/module/lostcon.c:164-197`).

fn potion_item(id: u32, has_hp: u8, has_mana: u8) -> Item {
    let mut potion = item(id, ItemFlags::empty());
    potion.driver = IDR_POTION;
    potion.driver_data = vec![0, has_hp, has_mana, 0];
    potion
}

#[test]
fn precascade_heals_self_when_hp_low_and_mana_available() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    lingering.values[1][CharacterValue::Hp as usize] = 10;
    lingering.values[0][CharacterValue::Hp as usize] = 10;
    lingering.values[1][CharacterValue::Mana as usize] = 10;
    lingering.values[0][CharacterValue::Heal as usize] = 5;
    lingering.hp = 5_000; // below 10*1000*3/4 = 7500
    lingering.mana = 8_000; // above 10*1000/2 = 5000
    assert!(world.spawn_character(lingering, 10, 10));

    assert!(world.process_lostcon_self_care_precascade(
        CharacterId(1),
        1,
        LostconSelfCareSuppressions::default()
    ));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::HEAL_SELF);
    assert!(character.mana < 8_000);
}

#[test]
fn precascade_respects_noheal_and_takes_no_action_when_nothing_else_is_low() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    lingering.values[1][CharacterValue::Hp as usize] = 10;
    lingering.values[0][CharacterValue::Hp as usize] = 10;
    lingering.values[1][CharacterValue::Mana as usize] = 10;
    lingering.values[0][CharacterValue::Heal as usize] = 5;
    lingering.hp = 5_000;
    lingering.mana = 8_000;
    assert!(world.spawn_character(lingering, 10, 10));

    let suppressions = LostconSelfCareSuppressions {
        noheal: true,
        ..Default::default()
    };
    assert!(!world.process_lostcon_self_care_precascade(CharacterId(1), 1, suppressions));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, 0);
    assert_eq!(character.mana, 8_000);
}

#[test]
fn precascade_drinks_a_pure_mana_potion_without_returning_early() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    // Not low on hp (skips the heal branch entirely).
    lingering.values[1][CharacterValue::Hp as usize] = 10;
    lingering.hp = 8_000;
    lingering.values[1][CharacterValue::Mana as usize] = 10;
    lingering.values[0][CharacterValue::Mana as usize] = 10;
    lingering.mana = 200; // below 10*1000/4 = 2500
    let mut potion = potion_item(900, 0, 5);
    potion.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(900), potion);
    lingering.inventory[30] = Some(ItemId(900));
    assert!(world.spawn_character(lingering, 10, 10));

    // C never `return`s from the potion branch - the caller should still
    // be free to run the attack cascade/postcascade this same tick.
    assert!(!world.process_lostcon_self_care_precascade(
        CharacterId(1),
        1,
        LostconSelfCareSuppressions::default()
    ));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(character.mana > 200);
    assert_eq!(character.inventory[30], None);
    assert_eq!(world.items.get(&ItemId(900)).unwrap().carried_by, None);
}

#[test]
fn precascade_skips_a_combo_potion_when_nocombo_and_falls_back_to_a_pure_mana_potion() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    lingering.values[1][CharacterValue::Hp as usize] = 10;
    lingering.hp = 8_000;
    lingering.values[1][CharacterValue::Mana as usize] = 10;
    lingering.mana = 200;
    let mut combo = potion_item(900, 5, 5);
    combo.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(900), combo);
    let mut pure_mana = potion_item(901, 0, 5);
    pure_mana.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(901), pure_mana);
    lingering.inventory[30] = Some(ItemId(900));
    lingering.inventory[31] = Some(ItemId(901));
    assert!(world.spawn_character(lingering, 10, 10));

    let suppressions = LostconSelfCareSuppressions {
        nocombo: true,
        ..Default::default()
    };
    world.process_lostcon_self_care_precascade(CharacterId(1), 1, suppressions);

    let character = world.characters.get(&CharacterId(1)).unwrap();
    // The combo potion (slot 30) was skipped; the pure mana potion (slot
    // 31) was drunk instead.
    assert_eq!(character.inventory[30], Some(ItemId(900)));
    assert_eq!(character.inventory[31], None);
    assert!(world.items.contains_key(&ItemId(900)));
}

#[test]
fn precascade_casts_magicshield_when_low_and_nothing_else_fired() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    // Hp/mana not low - only the magicshield branch should fire.
    lingering.values[1][CharacterValue::Hp as usize] = 10;
    lingering.hp = 8_000;
    lingering.values[1][CharacterValue::Mana as usize] = 10;
    lingering.mana = 8_000;
    lingering.values[1][CharacterValue::MagicShield as usize] = 10;
    lingering.values[0][CharacterValue::MagicShield as usize] = 8;
    lingering.lifeshield = 0; // below 10*1000/4 = 2500
    assert!(world.spawn_character(lingering, 10, 10));

    assert!(world.process_lostcon_self_care_precascade(
        CharacterId(1),
        1,
        LostconSelfCareSuppressions::default()
    ));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::MAGICSHIELD);
    // `do_magicshield` deducts mana immediately and stashes the pending
    // shield amount in `act1`; the `lifeshield` gain itself applies later
    // when the action completes (matching C's `ch[cn].act1 = str;`).
    assert!(character.mana < 8_000);
}

#[test]
fn precascade_returns_false_when_nothing_is_low() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    lingering.values[1][CharacterValue::Hp as usize] = 10;
    lingering.hp = 10_000;
    lingering.values[1][CharacterValue::Mana as usize] = 10;
    lingering.mana = 10_000;
    lingering.values[1][CharacterValue::MagicShield as usize] = 10;
    lingering.lifeshield = 10_000;
    assert!(world.spawn_character(lingering, 10, 10));

    assert!(!world.process_lostcon_self_care_precascade(
        CharacterId(1),
        1,
        LostconSelfCareSuppressions::default()
    ));
}

#[test]
fn precascade_is_a_no_op_for_a_normal_playing_character() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.hp = 0;
    player.values[1][CharacterValue::Hp as usize] = 10;
    assert!(world.spawn_character(player, 10, 10));

    assert!(!world.process_lostcon_self_care_precascade(
        CharacterId(1),
        1,
        LostconSelfCareSuppressions::default()
    ));
}

// `process_lostcon_self_care_postcascade` (C `lostcon_driver`'s
// bless/magicshield/heal fallback, `src/module/lostcon.c:207-218`).

#[test]
fn postcascade_blesses_self_when_unblessed() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    lingering.values[0][CharacterValue::Bless as usize] = 20;
    lingering.mana = BLESS_COST;
    assert!(world.spawn_character(lingering, 10, 10));

    assert!(world.process_lostcon_self_care_postcascade(
        CharacterId(1),
        LostconSelfCareSuppressions::default()
    ));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::BLESS_SELF);
    assert_eq!(character.mana, 0);
}

#[test]
fn postcascade_respects_nobless_and_falls_back_to_magicshield() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    lingering.values[0][CharacterValue::Bless as usize] = 20;
    lingering.values[0][CharacterValue::MagicShield as usize] = 8;
    lingering.mana = 5_000;
    lingering.lifeshield = 0;
    assert!(world.spawn_character(lingering, 10, 10));

    let suppressions = LostconSelfCareSuppressions {
        nobless: true,
        ..Default::default()
    };
    assert!(world.process_lostcon_self_care_postcascade(CharacterId(1), suppressions));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::MAGICSHIELD);
}

#[test]
fn postcascade_heals_self_as_a_last_resort() {
    let mut world = World::default();
    let mut lingering = lingering_lostcon(1);
    lingering.values[0][CharacterValue::Heal as usize] = 5;
    lingering.values[0][CharacterValue::Hp as usize] = 10;
    lingering.hp = 1_000; // below 10*1000/2 = 5000
    lingering.mana = 5_000;
    assert!(world.spawn_character(lingering, 10, 10));

    let suppressions = LostconSelfCareSuppressions {
        nobless: true,
        noshield: true,
        ..Default::default()
    };
    assert!(world.process_lostcon_self_care_postcascade(CharacterId(1), suppressions));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::HEAL_SELF);
}

#[test]
fn postcascade_returns_false_when_no_spell_applies() {
    let mut world = World::default();
    let lingering = lingering_lostcon(1);
    assert!(world.spawn_character(lingering, 10, 10));

    assert!(!world.process_lostcon_self_care_postcascade(
        CharacterId(1),
        LostconSelfCareSuppressions::default()
    ));
}

// `queue_lostcon_idle` (C `lostcon_driver`'s tail `do_idle(cn, TICKS)`,
// `src/module/lostcon.c:220`).

#[test]
fn queue_lostcon_idle_sets_idle_action_for_a_lingering_character() {
    let mut world = World::default();
    assert!(world.spawn_character(lingering_lostcon(1), 10, 10));

    assert!(world.queue_lostcon_idle(CharacterId(1)));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::IDLE);
}

#[test]
fn queue_lostcon_idle_is_a_no_op_for_a_normal_playing_character() {
    let mut world = World::default();
    assert!(world.spawn_character(player_character(1), 10, 10));

    assert!(!world.queue_lostcon_idle(CharacterId(1)));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, 0);
}
