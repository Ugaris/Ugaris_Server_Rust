use super::*;
use crate::character_driver::{ClubmasterDriverData, CDR_CLUBMASTER};
use crate::clan::CLUB_OFFSET;
use crate::world::clubmaster::ClubmasterEvent;

fn clubmaster_npc(id: u32) -> Character {
    let mut clubmaster = character(id);
    clubmaster.name = "Clubmaster".into();
    clubmaster.driver = CDR_CLUBMASTER;
    clubmaster.driver_state = Some(CharacterDriverState::Clubmaster(
        ClubmasterDriverData::default(),
    ));
    clubmaster
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER | CharacterFlags::PAID;
    player.name = name.into();
    player
}

fn clubmaster_data(world: &World, clubmaster_id: CharacterId) -> ClubmasterDriverData {
    match world
        .characters
        .get(&clubmaster_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Clubmaster(data)) => data,
        _ => panic!("expected clubmaster driver state"),
    }
}

#[test]
fn found_command_creates_club_and_installs_founder() {
    let mut world = World::default();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut founder = player(2, "Godmode");
    founder.gold = 10_000 * 100;
    assert!(world.spawn_character(founder, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "found: Black Rose");
    }
    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Congratulations, Godmode, you are now the leader of the club Black Rose.")));

    let founder = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(founder.gold, 0);
    assert_eq!(founder.clan_rank, 2);
    assert!(founder.clan >= CLUB_OFFSET);
    let club_nr = founder.clan - CLUB_OFFSET;
    assert_eq!(world.club_registry.name(club_nr), Some("Black Rose"));
    assert_eq!(founder.clan_serial, world.club_registry.serial(club_nr));
}

#[test]
fn found_command_queues_achievement_event() {
    let mut world = World::default();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut founder = player(2, "Godmode");
    founder.gold = 10_000 * 100;
    assert!(world.spawn_character(founder, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "found: Black Rose");
    }
    world.process_clubmaster_actions(0, 0);

    let events = world.drain_pending_clubmaster_events();
    assert_eq!(
        events,
        vec![ClubmasterEvent::ClubFounded {
            founder_id: CharacterId(2)
        }]
    );
}

#[test]
fn found_command_truncates_name_at_first_invalid_character() {
    let mut world = World::default();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut founder = player(2, "Godmode");
    founder.gold = 10_000 * 100;
    assert!(world.spawn_character(founder, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "found: Rose123 trailing");
    }
    world.process_clubmaster_actions(0, 0);

    let founder = world.characters.get(&CharacterId(2)).unwrap();
    let club_nr = founder.clan - CLUB_OFFSET;
    assert_eq!(world.club_registry.name(club_nr), Some("Rose"));
}

#[test]
fn found_command_rejects_unpaid_player() {
    let mut world = World::default();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut unpaid = player(2, "Freebie");
    unpaid.flags.remove(CharacterFlags::PAID);
    unpaid.gold = 10_000 * 100;
    assert!(world.spawn_character(unpaid, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "found: Freeloaders");
    }
    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("I'm sorry, Freebie, but only paying players may found clubs.")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().clan, 0);
}

#[test]
fn found_command_rejects_existing_clan_member() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Existing", 0).unwrap();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut member = player(2, "Godmode");
    member.gold = 10_000 * 100;
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "found: Newcomers");
    }
    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You are already a member of a clan or club. You cannot found a new one.")));
}

#[test]
fn found_command_rejects_insufficient_gold() {
    let mut world = World::default();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut founder = player(2, "Godmode");
    founder.gold = 5_000 * 100;
    assert!(world.spawn_character(founder, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "found: Black Rose");
    }
    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You cannot pay the fee of 10,000 gold.")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().gold,
        5_000 * 100
    );
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().clan, 0);
}

#[test]
fn found_command_rejects_duplicate_name() {
    let mut world = World::default();
    world.club_registry.create_club("Black Rose", 0).unwrap();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut founder = player(2, "Godmode");
    founder.gold = 10_000 * 100;
    assert!(world.spawn_character(founder, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "found: Black Rose");
    }
    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Something's wrong with the name.")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().gold,
        10_000 * 100
    );
}

#[test]
fn accept_then_join_completes_membership_handshake() {
    let mut world = World::default();
    let club_nr = world.club_registry.create_club("Rovers", 0).unwrap();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut founder = player(2, "Leader");
    founder.clan = CLUB_OFFSET + club_nr;
    founder.clan_serial = world.club_registry.serial(club_nr);
    founder.clan_rank = 2;
    assert!(world.spawn_character(founder, 10, 10));
    assert!(world.spawn_character(player(3, "Newbie"), 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "accept: Newbie");
    }
    world.process_clubmaster_actions(0, 0);

    let data = clubmaster_data(&world, CharacterId(1));
    assert_eq!(data.accept, "Newbie");
    assert_eq!(data.join, "Leader");
    assert_eq!(data.accept_clan, club_nr);

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(3), "join: Leader");
    }
    world.process_clubmaster_actions(0, 0);

    let newbie = world.characters.get(&CharacterId(3)).unwrap();
    assert_eq!(newbie.clan, CLUB_OFFSET + club_nr);
    assert_eq!(newbie.clan_rank, 0);
    assert_eq!(newbie.clan_serial, world.club_registry.serial(club_nr));

    let events = world.drain_pending_clubmaster_events();
    assert_eq!(
        events,
        vec![ClubmasterEvent::MemberAdded {
            member_id: CharacterId(3)
        }]
    );

    // Handshake state is cleared after a successful join.
    let data = clubmaster_data(&world, CharacterId(1));
    assert!(data.accept.is_empty());
    assert_eq!(data.accept_clan, 0);
    assert!(data.join.is_empty());
}

#[test]
fn accept_requires_club_rank_at_least_one() {
    let mut world = World::default();
    let club_nr = world.club_registry.create_club("Rovers", 0).unwrap();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut member = player(2, "Plain");
    member.clan = CLUB_OFFSET + club_nr;
    member.clan_serial = world.club_registry.serial(club_nr);
    member.clan_rank = 0;
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "accept: Someone");
    }
    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You are not a club leader, Plain.")));
}

#[test]
fn join_rejects_uninvited_player() {
    let mut world = World::default();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Newbie"), 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "join: Leader");
    }
    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You have not been invited, Newbie.")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().clan, 0);
}

#[test]
fn leave_command_clears_membership_with_no_event() {
    let mut world = World::default();
    let club_nr = world.club_registry.create_club("Rovers", 0).unwrap();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut member = player(2, "Departing");
    member.clan = CLUB_OFFSET + club_nr;
    member.clan_serial = world.club_registry.serial(club_nr);
    member.clan_rank = 0;
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "leave!");
    }
    world.process_clubmaster_actions(0, 0);

    let departed = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(departed.clan, 0);
    assert_eq!(departed.clan_rank, 0);
    assert_eq!(departed.clan_serial, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You are no longer a member of any club, Departing")));
    // No achievement/log event is queued for a club departure.
    assert!(world.drain_pending_clubmaster_events().is_empty());
}

#[test]
fn leave_command_rejects_non_member() {
    let mut world = World::default();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Loner"), 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "leave!");
    }
    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You are not a club member, Loner.")));
}

#[test]
fn deposit_command_works_for_any_member() {
    let mut world = World::default();
    let club_nr = world.club_registry.create_club("Rovers", 0).unwrap();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut member = player(2, "Payer");
    member.clan = CLUB_OFFSET + club_nr;
    member.clan_serial = world.club_registry.serial(club_nr);
    member.clan_rank = 0;
    member.gold = 500 * 100;
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "deposit: 200");
    }
    world.process_clubmaster_actions(0, 0);

    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().gold,
        300 * 100
    );
    assert_eq!(world.club_registry.club_money(club_nr), 200 * 100);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You have deposited 200G, for a total of 200G, Payer.")));
}

#[test]
fn deposit_command_rejects_insufficient_gold() {
    let mut world = World::default();
    let club_nr = world.club_registry.create_club("Rovers", 0).unwrap();
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut member = player(2, "Payer");
    member.clan = CLUB_OFFSET + club_nr;
    member.clan_serial = world.club_registry.serial(club_nr);
    member.gold = 50 * 100;
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "deposit: 200");
    }
    world.process_clubmaster_actions(0, 0);

    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().gold,
        50 * 100
    );
    assert_eq!(world.club_registry.club_money(club_nr), 0);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You do not have that much gold, Payer.")));
}

#[test]
fn withdraw_command_requires_founder_rank() {
    let mut world = World::default();
    let club_nr = world.club_registry.create_club("Rovers", 0).unwrap();
    world.club_registry.club_money_change(club_nr, 1000 * 100);
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut member = player(2, "Plain");
    member.clan = CLUB_OFFSET + club_nr;
    member.clan_serial = world.club_registry.serial(club_nr);
    member.clan_rank = 1;
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "withdraw: 100");
    }
    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You are not a club founder, Plain.")));
    assert_eq!(world.club_registry.club_money(club_nr), 1000 * 100);
}

#[test]
fn withdraw_command_pays_founder_from_treasury() {
    let mut world = World::default();
    let club_nr = world.club_registry.create_club("Rovers", 0).unwrap();
    world.club_registry.club_money_change(club_nr, 1000 * 100);
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut founder = player(2, "Leader");
    founder.clan = CLUB_OFFSET + club_nr;
    founder.clan_serial = world.club_registry.serial(club_nr);
    founder.clan_rank = 2;
    assert!(world.spawn_character(founder, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "withdraw: 400");
    }
    world.process_clubmaster_actions(0, 0);

    assert_eq!(world.club_registry.club_money(club_nr), 600 * 100);
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().gold,
        400 * 100
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You have withdrawn 400G, money left in club 600G, Leader.")));
}

#[test]
fn withdraw_command_rejects_insufficient_treasury() {
    let mut world = World::default();
    let club_nr = world.club_registry.create_club("Rovers", 0).unwrap();
    world.club_registry.club_money_change(club_nr, 100 * 100);
    assert!(world.spawn_character(clubmaster_npc(1), 10, 10));
    let mut founder = player(2, "Leader");
    founder.clan = CLUB_OFFSET + club_nr;
    founder.clan_serial = world.club_registry.serial(club_nr);
    founder.clan_rank = 2;
    assert!(world.spawn_character(founder, 10, 10));

    if let Some(clubmaster) = world.characters.get_mut(&CharacterId(1)) {
        clubmaster.push_driver_text_message(CharacterId(2), "withdraw: 400");
    }
    world.process_clubmaster_actions(0, 0);

    assert_eq!(world.club_registry.club_money(club_nr), 100 * 100);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("The club does not have that much gold, Leader.")));
}

#[test]
fn greeting_fires_for_every_nearby_player_matching_c_bug() {
    // C's own `if (!get_char_club(cn) && !get_char_clan(cn))` checks the
    // clubmaster NPC's own membership (always false), not the visitor's
    // - so unlike `clanmaster_driver`, this greeting never stops firing
    // for an existing member. See the module doc comment.
    let mut world = World::default();
    let club_nr = world.club_registry.create_club("Rovers", 0).unwrap();
    let mut clubmaster = clubmaster_npc(1);
    clubmaster.driver_memory = Default::default();
    assert!(world.spawn_character(clubmaster, 10, 10));
    let mut member = player(2, "AlreadyIn");
    member.clan = CLUB_OFFSET + club_nr;
    member.clan_serial = world.club_registry.serial(club_nr);
    light_tile(&mut world, 10, 10);
    assert!(world.spawn_character(member, 10, 10));

    world.process_clubmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Hello AlreadyIn! Would you like to found a club?")));
}

fn light_tile(world: &mut World, x: usize, y: usize) {
    world.map.tile_mut(x, y).unwrap().light = 255;
}
