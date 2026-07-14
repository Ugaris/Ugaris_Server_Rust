use super::*;
use crate::character_driver::{ClanclerkDriverData, CDR_CLANCLERK};
use crate::clan::ClanRelation;
use crate::entity::CharacterValue;
use crate::item_driver::{IDR_CLANJEWEL, IDR_FLASK, IDR_POTION};
use crate::world::clanclerk::ClanclerkEvent;

fn clanclerk_npc(id: u32, clan: u16) -> Character {
    let mut clanclerk = character(id);
    clanclerk.name = "Clanclerk".into();
    clanclerk.driver = CDR_CLANCLERK;
    clanclerk.driver_state = Some(CharacterDriverState::Clanclerk(ClanclerkDriverData {
        clan,
    }));
    clanclerk
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER | CharacterFlags::PAID;
    player.name = name.into();
    player
}

fn member(id: u32, name: &str, world: &World, clan: u16, rank: u8) -> Character {
    let mut character = player(id, name);
    let _ = world.clan_registry.serial(clan);
    character.clan = clan;
    character.clan_serial = world.clan_registry.serial(clan);
    character.clan_rank = rank;
    character
}

fn found_clan(world: &mut World, name: &str) -> u16 {
    world.clan_registry.found_clan(name, 0).unwrap()
}

fn clan_jewel_item(id: u32) -> Item {
    let mut jewel = item(id, ItemFlags::empty());
    jewel.driver = IDR_CLANJEWEL;
    jewel
}

/// A finished, shaken alchemy flask matching `add_alc_potion`'s first
/// branch ("Attack, Parry, Immunity+N"), `tier` selecting `mod_value[0]`
/// (`4 + tier*4`, matching `str = min(5, (mod_value[0]/4)-1)`).
fn attack_flask_item(id: u32, tier: i16) -> Item {
    let mut flask = item(id, ItemFlags::empty());
    flask.driver = IDR_FLASK;
    flask.modifier_index[0] = CharacterValue::Attack as i16;
    flask.modifier_index[1] = CharacterValue::Parry as i16;
    flask.modifier_index[2] = CharacterValue::Immunity as i16;
    flask.modifier_value[0] = 4 + tier * 4;
    flask
}

/// A finished, shaken alchemy flask matching `add_alc_potion`'s second
/// branch ("Flash, Magic Shield, Immunity+N").
fn flash_flask_item(id: u32, tier: i16) -> Item {
    let mut flask = item(id, ItemFlags::empty());
    flask.driver = IDR_FLASK;
    flask.modifier_index[0] = CharacterValue::Flash as i16;
    flask.modifier_index[1] = CharacterValue::MagicShield as i16;
    flask.modifier_index[2] = CharacterValue::Immunity as i16;
    flask.modifier_value[0] = 4 + tier * 4;
    flask
}

/// A finished `IDR_POTION` item matching one of `add_simple_potion`'s
/// nine `drdata[1..4]` patterns.
fn simple_potion_item(id: u32, d1: u8, d2: u8, d3: u8) -> Item {
    let mut potion = item(id, ItemFlags::empty());
    potion.driver = IDR_POTION;
    potion.driver_data = vec![0, d1, d2, d3];
    potion
}

#[test]
fn deposit_succeeds_for_non_member() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    let mut visitor = player(2, "Godmode");
    visitor.gold = 100_000;
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "deposit 150");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Thank you, Godmode. I have deposited 150G into the clan treasury.")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().gold,
        100_000 - 15_000
    );
    assert_eq!(world.clan_registry.clan_money(clan), 150);

    // C only clan-logs deposits `>= 100` (`diff >= 100 || diff < 0`).
    let events = world.drain_pending_clanclerk_events();
    assert_eq!(
        events,
        vec![ClanclerkEvent::MoneyChanged {
            clan_nr: clan,
            actor_id: CharacterId(2),
            change: crate::clan::ClanMoneyChange::Deposited(150),
        }]
    );
}

#[test]
fn deposit_rejects_non_positive_amount() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "deposit 0");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("I'm sorry, Godmode, but you must specify a positive amount to deposit.")));
    assert_eq!(world.clan_registry.clan_money(clan), 0);
}

#[test]
fn deposit_rejects_insufficient_gold() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    let mut visitor = player(2, "Godmode");
    visitor.gold = 100;
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "deposit 50");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("I'm afraid you don't have 50G to deposit, Godmode.")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 100);
}

#[test]
fn withdraw_requires_membership() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    world.clan_registry.clan_money_change(clan, 100, false);
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(player(2, "Outsider"), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "withdraw 10");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(!texts.iter().any(|t| t.message.contains("I have withdrawn")));
    assert_eq!(world.clan_registry.clan_money(clan), 100);
}

#[test]
fn withdraw_requires_treasurer_rank() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    world.clan_registry.clan_money_change(clan, 100, false);
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Grunt", &world, clan, 1), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "withdraw 10");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(!texts.iter().any(|t| t.message.contains("I have withdrawn")));
    assert_eq!(world.clan_registry.clan_money(clan), 100);
}

#[test]
fn withdraw_succeeds_for_treasurer() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    world.clan_registry.clan_money_change(clan, 100, false);
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Treasurer", &world, clan, 3), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "withdraw 30");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains(
        "Here you are, Treasurer. I have withdrawn 30G from the clan treasury for you."
    )));
    assert_eq!(world.clan_registry.clan_money(clan), 70);
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 3_000);
}

#[test]
fn withdraw_rejects_insufficient_treasury() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    world.clan_registry.clan_money_change(clan, 10, false);
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Treasurer", &world, clan, 3), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "withdraw 30");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("I'm afraid the clan treasury only holds 10G, Treasurer.")));
    assert_eq!(world.clan_registry.clan_money(clan), 10);
}

#[test]
fn buy_is_always_disabled() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Treasurer", &world, clan, 3), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "buy 5 10");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Buying has been disabled, you have infinite stock.")));
}

#[test]
fn dungeon_use_requires_treasurer_rank() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Grunt", &world, clan, 1), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "use 1 5");
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(world.clan_registry.get_clan_dungeon(clan, 1), 0);
    let texts = world.drain_pending_area_texts();
    assert!(!texts
        .iter()
        .any(|t| t.message.contains("dungeon configuration")));
}

#[test]
fn dungeon_use_succeeds_for_treasurer() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Treasurer", &world, clan, 3), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "use 1 5");
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(world.clan_registry.get_clan_dungeon(clan, 1), 5);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Very well. I have updated the dungeon configuration for your clan.")));

    let events = world.drain_pending_clanclerk_events();
    assert_eq!(
        events,
        vec![ClanclerkEvent::DungeonUseSet {
            clan_nr: clan,
            actor_id: CharacterId(2),
            dungeon_type: 1,
            number: 5,
        }]
    );
}

#[test]
fn dungeon_use_rejects_out_of_range_type() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Treasurer", &world, clan, 3), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "use 99 5");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("The dungeon type must be between 1 and 21, Treasurer.")));
}

#[test]
fn dungeon_use_rejects_out_of_range_quantity() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Treasurer", &world, clan, 3), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "use 1 200");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("The quantity must be between 0 and 100, Treasurer.")));
}

#[test]
fn dungeon_use_rejects_per_type_cap_with_generic_limits_message() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Treasurer", &world, clan, 3), 10, 10));

    // 11 passes the outer 0..=100 quantity check but exceeds the
    // per-type cap of 10 for warrior/mage/seyan slots.
    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "use 1 11");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains(
        "I'm sorry, but the limits are: 0-10 guards of each type, 0-25 teleport traps, 0-1 fake walls, and 0-2 locked doors."
    )));
    assert_eq!(world.clan_registry.get_clan_dungeon(clan, 1), 0);
}

#[test]
fn dungeon_use_reports_cost_when_over_training_budget() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Treasurer", &world, clan, 3), 10, 10));

    // Slots 1-5 (multipliers 1/2/4/8/12) maxed at 10 each = running cost
    // 270; slot 6 (multiplier 16) at 9 would push the total to 414.
    for (dungeon_type, number) in [(1, 10), (2, 10), (3, 10), (4, 10), (5, 10)] {
        world
            .clan_registry
            .set_clan_dungeon_use(clan, dungeon_type, number)
            .unwrap();
    }

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "use 6 9");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("That configuration would cost 414 points, but you may only spend 400 points.")));
    assert_eq!(world.clan_registry.get_clan_dungeon(clan, 6), 0);
}

#[test]
fn set_bonus_requires_leader_rank() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Treasurer", &world, clan, 3), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "set bonus 2 5");
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(world.clan_registry.bonus_level(clan, 2), 0);
}

#[test]
fn set_bonus_succeeds_for_leader() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "set bonus 2 5");
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(world.clan_registry.bonus_level(clan, 2), 5);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Very well. I have set the Merchant bonus to level 5 for your clan.")));
}

#[test]
fn set_bonus_disable_message_at_level_zero() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    world.clan_registry.set_bonus_level(clan, 0, 4).unwrap();
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "set bonus 0 0");
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(world.clan_registry.bonus_level(clan, 0), 0);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Very well. I have disabled the Pentagram Quest bonus for your clan.")));
}

#[test]
fn set_bonus_rejects_out_of_range_number() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "set bonus 9 5");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains(
        "Invalid bonus number. Available bonuses: 0=Pentagram Quest, 1=Military Advisor, 2=Merchant."
    )));
}

#[test]
fn rank_name_succeeds_for_leader() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "rank name 2 Officer");
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(
        world.clan_registry.identity(clan).unwrap().rank_names[2],
        "Officer"
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Very well. Rank 2 shall now be known as Officer.")));

    let events = world.drain_pending_clanclerk_events();
    assert_eq!(
        events,
        vec![ClanclerkEvent::RankNameSet {
            clan_nr: clan,
            actor_id: CharacterId(2),
            rank: 2,
            name: "Officer".to_string(),
        }]
    );
}

#[test]
fn rank_name_rejects_out_of_range() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "rank name 9 Officer");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("The rank number must be between 0 and 4, Leader.")));
}

#[test]
fn website_strips_trailing_character_like_c() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "website http://example.com/x");
    }
    world.process_clanclerk_actions(0, 0);

    // C's `website[strlen(website)-1] = 0` drops the real last character.
    assert_eq!(
        world.clan_registry.identity(clan).unwrap().website,
        "http://example.com/"
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Very well. I have updated your clan's website to: http://example.com/")));
}

#[test]
fn message_strips_trailing_character_like_c() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "message Hello there!");
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(
        world.clan_registry.identity(clan).unwrap().message,
        "Hello there"
    );
}

#[test]
fn raiding_on_then_off_toggles_pending_timer() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "raiding on");
    }
    world.process_clanclerk_actions(0, 555);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Understood. Raiding has been enabled for your clan. Be prepared for battle!")));
    // `raiding on` only sets the pending timer, not `get_clan_raid` itself.
    assert!(!world.clan_registry.get_clan_raid(clan));

    // Asking again while already pending is a no-op failure in C.
    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "raiding on");
    }
    world.process_clanclerk_actions(0, 600);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("I'm sorry, I was unable to enable raiding for your clan.")));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "raiding off");
    }
    world.process_clanclerk_actions(0, 700);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Understood. Raiding has been disabled for your clan. May peace be with you.")));
}

#[test]
fn raiding_god_toggle_requires_god_flag() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "raiding god on");
    }
    world.process_clanclerk_actions(0, 0);
    assert!(!world.clan_registry.get_clan_raid(clan));

    let mut god_leader = member(3, "GodLeader", &world, clan, 4);
    god_leader.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(god_leader, 10, 10));
    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(3), "raiding god on");
    }
    world.process_clanclerk_actions(0, 0);
    assert!(world.clan_registry.get_clan_raid(clan));

    let events = world.drain_pending_clanclerk_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, ClanclerkEvent::RaidGodToggled { enabled: true, .. })));
}

#[test]
fn relation_requires_raiding_enabled_on_both_clans() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    let other = found_clan(&mut world, "White Lily");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    let mut leader = member(2, "Leader", &world, clan, 4);
    leader.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(leader, 10, 10));

    // Raiding not enabled anywhere yet: War (4) should be refused.
    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "relation 2 4");
    }
    world.process_clanclerk_actions(0, 0);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Your clan cannot declare War unless you first say 'raiding on'.")));
    assert_eq!(
        world
            .clan_registry
            .relations()
            .current_relation(clan, other),
        ClanRelation::Neutral
    );

    world.clan_registry.set_clan_raid_god(clan, true).unwrap();
    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "relation 2 4");
    }
    world.process_clanclerk_actions(0, 0);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("unless they also have raiding enabled.")));

    world.clan_registry.set_clan_raid_god(other, true).unwrap();
    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "relation 2 4");
    }
    world.process_clanclerk_actions(0, 0);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains(
        "Very well. I have requested War status with White Lily. The change may take time to process."
    )));
}

#[test]
fn relation_rejects_out_of_range_clan_and_level() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "relation 99 3");
    }
    world.process_clanclerk_actions(0, 0);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("The clan number must be between 1 and 31. Use /clan to see the list.")));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "relation 2 9");
    }
    world.process_clanclerk_actions(0, 0);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("The relation must be: 1=Alliance, 2=Peace-Treaty, 3=Neutral, 4=War, 5=Feud.")));
}

#[test]
fn help_shows_leader_section_only_for_leader_member() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(player(2, "Outsider"), 10, 10));
    assert!(world.spawn_character(member(3, "Leader", &world, clan, 4), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "help");
    }
    world.process_clanclerk_actions(0, 0);
    let outsider_lines: Vec<String> = world
        .drain_pending_system_texts()
        .into_iter()
        .filter(|t| t.character_id == CharacterId(2))
        .map(|t| t.message)
        .collect();
    assert!(outsider_lines
        .iter()
        .any(|l| l.contains("deposit <amount>")));
    assert!(!outsider_lines.iter().any(|l| l.contains("Leader Commands")));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(3), "help");
    }
    world.process_clanclerk_actions(0, 0);
    let leader_lines: Vec<String> = world
        .drain_pending_system_texts()
        .into_iter()
        .filter(|t| t.character_id == CharacterId(3))
        .map(|t| t.message)
        .collect();
    assert!(leader_lines.iter().any(|l| l.contains("Leader Commands")));
    assert!(leader_lines.iter().any(|l| l.contains("raiding on/off")));
}

#[test]
fn clan_jewel_give_adds_jewel_and_destroys_item() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));
    world.items.insert(ItemId(900), clan_jewel_item(900));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.cursor_item = Some(ItemId(900));
        clanclerk.push_driver_message(NT_GIVE, 2, 900, 0);
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(world.clan_registry.jewel_count(clan), 1);
    assert!(!world.items.contains_key(&ItemId(900)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Added Jewel.")));

    let events = world.drain_pending_clanclerk_events();
    assert_eq!(
        events,
        vec![ClanclerkEvent::JewelAdded {
            clan_nr: clan,
            actor_id: CharacterId(2),
        }]
    );
}

#[test]
fn jewels_qa_reports_jewel_count() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    world.clan_registry.add_jewel(clan).unwrap();
    world.clan_registry.add_jewel(clan).unwrap();
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    world.map.tile_mut(10, 10).unwrap().light = 255;

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "jewels");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Our clan has 2 jewels.")));
}

#[test]
fn flask_give_adds_attack_potion_and_destroys_item() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));
    world.items.insert(ItemId(900), attack_flask_item(900, 0));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.cursor_item = Some(ItemId(900));
        clanclerk.push_driver_message(NT_GIVE, 2, 900, 0);
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(
        world.clan_registry.identity(clan).unwrap().economy.alc_pot[0][0],
        1
    );
    assert!(!world.items.contains_key(&ItemId(900)));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Added one potion to our storage.")));
}

#[test]
fn flask_give_adds_flash_potion_at_correct_tier() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));
    // tier 5 (mod_value 24) and an over-24 value both clamp to tier 5.
    world.items.insert(ItemId(900), flash_flask_item(900, 7));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.cursor_item = Some(ItemId(900));
        clanclerk.push_driver_message(NT_GIVE, 2, 900, 0);
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(
        world.clan_registry.identity(clan).unwrap().economy.alc_pot[1][5],
        1
    );
}

#[test]
fn flask_give_rejects_unmatched_modifiers_and_destroys_item() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));
    let mut flask = item(900, ItemFlags::empty());
    flask.driver = IDR_FLASK;
    // No modifiers set at all - doesn't match either recipe.
    world.items.insert(ItemId(900), flask);

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.cursor_item = Some(ItemId(900));
        clanclerk.push_driver_message(NT_GIVE, 2, 900, 0);
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(
        world.clan_registry.identity(clan).unwrap().economy.alc_pot,
        [[0; 6], [0; 6]]
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Failed to add potion to storage, please try again.")));
    // Out-of-scope "give it back" fallback: the item still vanishes.
    assert!(!world.items.contains_key(&ItemId(900)));
}

#[test]
fn non_jewel_non_flask_give_is_silently_destroyed() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(member(2, "Leader", &world, clan, 4), 10, 10));
    world
        .items
        .insert(ItemId(900), item(900, ItemFlags::empty()));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.cursor_item = Some(ItemId(900));
        clanclerk.push_driver_message(NT_GIVE, 2, 900, 0);
    }
    world.process_clanclerk_actions(0, 0);

    assert!(!world.items.contains_key(&ItemId(900)));
    let texts = world.drain_pending_area_texts();
    assert!(texts.is_empty());
}

#[test]
fn add_potions_adds_matching_potions_and_reports_count() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    let mut small_healing = simple_potion_item(900, 8, 0, 0); // Small healing
    small_healing.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(900), small_healing);
    let mut medium_combo = simple_potion_item(901, 16, 16, 16); // Medium combo
    medium_combo.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(901), medium_combo);
    let mut no_match = simple_potion_item(902, 1, 2, 3); // no match
    no_match.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(902), no_match);

    if let Some(godmode) = world.characters.get_mut(&CharacterId(2)) {
        godmode.inventory[30] = Some(ItemId(900));
        godmode.inventory[31] = Some(ItemId(901));
        godmode.inventory[32] = Some(ItemId(902));
    }
    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "add potions");
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(
        world
            .clan_registry
            .identity(clan)
            .unwrap()
            .economy
            .simple_pot[0][0],
        1
    );
    assert_eq!(
        world
            .clan_registry
            .identity(clan)
            .unwrap()
            .economy
            .simple_pot[2][1],
        1
    );
    assert!(!world.items.contains_key(&ItemId(900)));
    assert!(!world.items.contains_key(&ItemId(901)));
    assert!(world.items.contains_key(&ItemId(902)));
    let inventory = &world.characters.get(&CharacterId(2)).unwrap().inventory;
    assert_eq!(inventory[30], None);
    assert_eq!(inventory[31], None);
    assert_eq!(inventory[32], Some(ItemId(902)));

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Very well. I have added 2 potions to the clan stores.")));
}

#[test]
fn add_potions_reports_no_potions_message_when_nothing_matches() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "add potions");
    }
    world.process_clanclerk_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("I'm sorry, there were no potions to add.")));
}

#[test]
fn add_potions_works_for_non_clan_member() {
    let mut world = World::default();
    let clan = found_clan(&mut world, "Black Rose");
    assert!(world.spawn_character(clanclerk_npc(1, clan), 10, 10));
    assert!(world.spawn_character(player(2, "Visitor"), 10, 10));
    let mut potion = simple_potion_item(900, 8, 0, 0);
    potion.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(900), potion);
    if let Some(visitor) = world.characters.get_mut(&CharacterId(2)) {
        visitor.inventory[30] = Some(ItemId(900));
    }
    if let Some(clanclerk) = world.characters.get_mut(&CharacterId(1)) {
        clanclerk.push_driver_text_message(CharacterId(2), "add potions");
    }
    world.process_clanclerk_actions(0, 0);

    assert_eq!(
        world
            .clan_registry
            .identity(clan)
            .unwrap()
            .economy
            .simple_pot[0][0],
        1
    );
    assert!(!world.items.contains_key(&ItemId(900)));
}
