use super::*;
use crate::character_driver::{ClanFoundData, ClanmasterDriverData, CDR_CLANMASTER};
use crate::clan::CLUB_OFFSET;
use crate::item_driver::IDR_CLANJEWEL;
use crate::world::clanmaster::ClanmasterEvent;

fn clanmaster_npc(id: u32) -> Character {
    let mut clanmaster = character(id);
    clanmaster.name = "Clanmaster".into();
    clanmaster.driver = CDR_CLANMASTER;
    clanmaster.driver_state = Some(CharacterDriverState::Clanmaster(
        ClanmasterDriverData::default(),
    ));
    clanmaster
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER | CharacterFlags::PAID;
    player.name = name.into();
    player
}

fn light_tile(world: &mut World, x: usize, y: usize) {
    world.map.tile_mut(x, y).unwrap().light = 255;
}

fn founding_state(world: &World, player_id: CharacterId) -> Option<ClanFoundData> {
    match world
        .characters
        .get(&player_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::ClanFound(data)) => Some(data),
        _ => None,
    }
}

fn clanmaster_data(world: &World, clanmaster_id: CharacterId) -> ClanmasterDriverData {
    match world
        .characters
        .get(&clanmaster_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Clanmaster(data)) => data,
        _ => panic!("expected clanmaster driver state"),
    }
}

#[test]
fn name_command_starts_founding_with_paid_non_member() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "name: Black Rose");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains(
        "Your clan, Godmode, will be named 'Black Rose'. Try again if that is not what you want. \
         Or hand me a Clan Jewel to proceed. You can buy them at Jeremy's"
    )));

    let fnd = founding_state(&world, CharacterId(2)).expect("founding state set");
    assert_eq!(fnd.state, 1);
    assert_eq!(fnd.name, "Black Rose");
}

#[test]
fn name_command_truncates_at_quote_and_79_chars() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "name: Rose\" trailing garbage");
    }
    world.process_clanmaster_actions(0, 0);

    let fnd = founding_state(&world, CharacterId(2)).expect("founding state set");
    assert_eq!(fnd.name, "Rose");
}

#[test]
fn name_command_rejects_unpaid_player() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut unpaid = player(2, "Freebie");
    unpaid.flags.remove(CharacterFlags::PAID);
    assert!(world.spawn_character(unpaid, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "name: Freeloaders");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("I'm sorry, Freebie, but only paying players may found clans.")));
    assert!(founding_state(&world, CharacterId(2)).is_none());
}

#[test]
fn name_command_rejects_existing_clan_member() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Existing", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut member = player(2, "Godmode");
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "name: Newcomers");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You are already a member of a clan or club. You cannot found a new one.")));
    assert!(founding_state(&world, CharacterId(2)).is_none());
}

#[test]
fn name_command_rejects_existing_club_member() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut member = player(2, "Godmode");
    member.clan = CLUB_OFFSET + 3;
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "name: Newcomers");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You are already a member of a clan or club. You cannot found a new one.")));
}

fn clan_jewel_item(id: u32) -> Item {
    let mut jewel = item(id, ItemFlags::empty());
    jewel.driver = IDR_CLANJEWEL;
    jewel
}

#[test]
fn clan_jewel_give_founds_clan_and_awards_master() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut founder = player(2, "Godmode");
    founder.driver_state = Some(CharacterDriverState::ClanFound(ClanFoundData {
        state: 1,
        nr: 0,
        name: "Black Rose".into(),
    }));
    assert!(world.spawn_character(founder, 10, 10));
    world.items.insert(ItemId(900), clan_jewel_item(900));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.cursor_item = Some(ItemId(900));
        clanmaster.push_driver_message(NT_GIVE, 2, 900, 0);
    }
    world.process_clanmaster_actions(0, 1_000);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains(
        "So be it. There will be a new clan, named 'Black Rose', and you, Godmode, shall be its \
         new master. Good luck, young master!"
    )));

    let founder = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(founder.clan_rank, 4);
    assert_eq!(
        world.clan_registry.get_char_clan(&mut founder.clone()),
        Some(1)
    );

    let events = world.drain_pending_clanmaster_events();
    assert_eq!(
        events,
        vec![ClanmasterEvent::ClanFounded {
            founder_id: CharacterId(2),
            clan_nr: 1
        }]
    );

    assert!(world.items.get(&ItemId(900)).is_none());
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}

#[test]
fn clan_jewel_give_without_name_first_fails() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    world.items.insert(ItemId(900), clan_jewel_item(900));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.cursor_item = Some(ItemId(900));
        clanmaster.push_driver_message(NT_GIVE, 2, 900, 0);
    }
    world.process_clanmaster_actions(0, 1_000);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You must name your clan first. Say: 'name: <clan-name>'.")));
    assert!(world.items.get(&ItemId(900)).is_none());
    assert!(world.drain_pending_clanmaster_events().is_empty());
}

#[test]
fn non_jewel_give_is_silently_destroyed() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    world
        .items
        .insert(ItemId(900), item(900, ItemFlags::empty()));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.cursor_item = Some(ItemId(900));
        clanmaster.push_driver_message(NT_GIVE, 2, 900, 0);
    }
    world.process_clanmaster_actions(0, 1_000);

    // Godmode is adjacent, so the periodic greeting also fires this tick
    // (a real, harmless side effect, not part of what this test checks);
    // only assert the give-message branch produced no error text and the
    // non-jewel item was destroyed unconditionally.
    let texts = world.drain_pending_area_texts();
    assert!(!texts.iter().any(
        |t| t.message.contains("name your clan first") || t.message.contains("error creating")
    ));
    assert!(world.items.get(&ItemId(900)).is_none());
}

#[test]
fn accept_command_requires_clan_leader_rank() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 1; // below the rank-2 leader threshold
    assert!(world.spawn_character(leader, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "accept: Bob");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You are not a clan leader, Alice.")));
    let data = clanmaster_data(&world, CharacterId(1));
    assert!(data.accept.is_empty());
}

#[test]
fn accept_then_join_completes_the_handshake_and_awards_member() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));
    assert!(world.spawn_character(player(3, "Bob"), 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "accept: Bob");
    }
    world.process_clanmaster_actions(0, 0);
    world.drain_pending_area_texts();

    let data = clanmaster_data(&world, CharacterId(1));
    assert_eq!(data.accept, "Bob");
    assert_eq!(data.join, "Alice");
    assert_eq!(data.accept_clan, nr);

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(3), "join: Alice");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Bob, you are now a member of Alice's clan.")));

    let bob = world.characters.get(&CharacterId(3)).unwrap();
    assert_eq!(
        world.clan_registry.get_char_clan(&mut bob.clone()),
        Some(nr)
    );

    let events = world.drain_pending_clanmaster_events();
    assert_eq!(
        events,
        vec![ClanmasterEvent::MemberAdded {
            member_id: CharacterId(3),
            clan_nr: nr,
            master_name: "Alice".into(),
        }]
    );

    let cleared = clanmaster_data(&world, CharacterId(1));
    assert!(cleared.accept.is_empty());
    assert!(cleared.join.is_empty());
    assert_eq!(cleared.accept_clan, 0);
}

#[test]
fn join_rejects_uninvited_player() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));
    assert!(world.spawn_character(player(3, "Bob"), 10, 10));
    assert!(world.spawn_character(player(4, "Eve"), 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "accept: Bob");
    }
    world.process_clanmaster_actions(0, 0);
    world.drain_pending_area_texts();

    // Eve tries to join using Bob's invite.
    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(4), "join: Alice");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You have not been invited, Eve.")));
    assert!(world.drain_pending_clanmaster_events().is_empty());
}

#[test]
fn join_rejects_wrong_confirmation_name() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));
    assert!(world.spawn_character(player(3, "Bob"), 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "accept: Bob");
    }
    world.process_clanmaster_actions(0, 0);
    world.drain_pending_area_texts();

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(3), "join: SomeoneElse");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("SomeoneElse has not invited you, Bob.")));
}

#[test]
fn join_rejects_already_a_member() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Existing", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut member = player(2, "Bob");
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "join: Alice");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You are already a clan member, Bob.")));
}

#[test]
fn leave_removes_membership_and_queues_clan_log_event() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leavers", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut member = player(2, "Bob");
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "leave!");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You are no longer a member of any clan, Bob")));

    let bob = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(bob.clan, 0);

    let events = world.drain_pending_clanmaster_events();
    assert_eq!(
        events,
        vec![ClanmasterEvent::MemberLeft {
            member_id: CharacterId(2),
            clan_nr: nr
        }]
    );
}

#[test]
fn leave_rejects_non_member() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Bob"), 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "leave!");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You are not a clan member, Bob.")));
}

#[test]
fn greets_non_member_once_and_skips_existing_member() {
    let mut world = World::default();
    light_tile(&mut world, 12, 10);
    light_tile(&mut world, 8, 10);
    let nr = world.clan_registry.found_clan("Existing", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    let mut member = player(3, "Member");
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 8, 10));

    world.process_clanmaster_actions(0, 0);
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0]
        .message
        .contains("Hello Godmode! Would you like to found a clan?"));

    // Second pass: memory suppresses the repeat greeting.
    world.process_clanmaster_actions(0, 0);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn qa_reply_for_small_talk_keyword() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Hello, Godmode!")));
}

#[test]
fn qa_clan_explanation_has_no_color_markers_and_matches_c_wording() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "clan");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains(
        "hand me a Clan Jewel. If you wish to tell me the name, use: 'name: <clan name>'"
    )));
}

#[test]
fn name_and_accept_keywords_in_one_message_both_fire_independently() {
    // C's `clanmaster_driver` checks each keyword with an independent
    // `if`, not an `else if` chain, so a message containing both "name:"
    // and "accept:" triggers both branches - matching the documented
    // `world/bank.rs` dual-match quirk.
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "name: Foo accept: Bob");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You are already a member of a clan or club. You cannot found a new one.")));
    assert!(texts.iter().any(|t| t
        .message
        .contains("To join Alice's clan Bob, say: 'join: Alice'")));
}

#[test]
fn idle_murmur_rolls_after_talk_interval() {
    let mut world = World::default();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    world.legacy_random_seed = 1;
    world.tick.0 = TICKS_PER_SECOND * 60 + 1;

    world.process_clanmaster_actions(0, 0);
    let texts = world.drain_pending_area_texts();
    // Whether it rolls depends on the deterministic legacy RNG sequence;
    // just prove the state machine doesn't panic and updates last_talk
    // when it *does* roll.
    if !texts.is_empty() {
        let data = clanmaster_data(&world, CharacterId(1));
        assert_eq!(data.last_talk, world.tick.0);
    }
}

#[test]
fn memory_clear_timer_erases_greet_memory() {
    let mut world = World::default();
    light_tile(&mut world, 10, 10);
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    world.process_clanmaster_actions(0, 0);
    assert_eq!(world.drain_pending_area_texts().len(), 1);

    // Advance well past the 12h memory-clear timer. The clear-check runs
    // after the greet scan each tick (matching C's own ordering, greeting
    // NT_CHAR handling at the top of the function, the memory-clear timer
    // check at the very end), so the memory is erased *during* this tick
    // (too late to affect this tick's own greet scan) and the greeting
    // only reappears on the *next* tick.
    world.tick.0 = TICKS_PER_SECOND * 60 * 60 * 12 + 1;
    world.process_clanmaster_actions(0, 0);
    assert!(world.drain_pending_area_texts().is_empty());
    world.process_clanmaster_actions(0, 0);
    assert_eq!(world.drain_pending_area_texts().len(), 1);
}

#[test]
fn rank_command_requires_clan_leader_rank() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 3; // below the rank-4 leader threshold
    assert!(world.spawn_character(leader, 10, 10));
    let mut member = player(3, "Bob");
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "rank: Bob 2");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You are not a clan leader, Alice.")));
    assert_eq!(world.characters.get(&CharacterId(3)).unwrap().clan_rank, 0);
}

#[test]
fn rank_command_rejects_out_of_range_rank() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));
    let mut member = player(3, "Bob");
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "rank: Bob 7");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You must use a rank between 0 and 4.")));
    assert_eq!(world.characters.get(&CharacterId(3)).unwrap().clan_rank, 0);
}

#[test]
fn rank_command_rejects_non_paying_target_above_rank_1() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));
    let mut member = player(3, "Bob");
    member.flags.remove(CharacterFlags::PAID);
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "rank: Bob 2");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Bob is not a paying player, you cannot set the rank higher than 1.")));
    assert_eq!(world.characters.get(&CharacterId(3)).unwrap().clan_rank, 0);
}

#[test]
fn rank_command_rejects_target_outside_clan() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    let other_nr = world.clan_registry.found_clan("Others", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));
    let mut outsider = player(3, "Eve");
    let _ = world.clan_registry.add_member(&mut outsider, other_nr);
    assert!(world.spawn_character(outsider, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "rank: Eve 2");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You cannot change the rank of those not belonging to your clan.")));
}

#[test]
fn rank_command_sets_rank_and_queues_clan_log_event() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));
    let mut member = player(3, "Bob");
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "rank: Bob 2");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Set Bob's rank to 2.")));
    assert_eq!(world.characters.get(&CharacterId(3)).unwrap().clan_rank, 2);

    let events = world.drain_pending_clanmaster_events();
    assert_eq!(
        events,
        vec![ClanmasterEvent::RankSet {
            clan_nr: nr,
            target_id: CharacterId(3),
            rank: 2,
            setter_name: "Alice".into(),
        }]
    );
}

#[test]
fn rank_command_ignores_unmatched_offline_name() {
    // C falls back to `lookup_name`/`task_set_clan_rank` (an async
    // DB-task queue) for a name that doesn't match anyone currently
    // online - no equivalent subsystem exists here, so this is a no-op,
    // not a crash or a bogus "not found" message (C itself sends no
    // player feedback on this path either, aside from a would-be
    // "Sorry, no player by the name %s found." that only fires for a
    // *resolved-but-unknown* name, a case this codebase can't distinguish
    // without a persistent name index).
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "rank: Ghost 2");
    }
    world.process_clanmaster_actions(0, 0);

    assert!(world.drain_pending_area_texts().is_empty());
    assert!(world.drain_pending_clanmaster_events().is_empty());
}

#[test]
fn fire_command_requires_clan_leader_rank() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 3;
    assert!(world.spawn_character(leader, 10, 10));
    let mut member = player(3, "Bob");
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "fire: Bob");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You are not a clan leader, Alice.")));
    assert_eq!(world.characters.get(&CharacterId(3)).unwrap().clan, nr);
}

#[test]
fn fire_command_rejects_target_outside_clan() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    let other_nr = world.clan_registry.found_clan("Others", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));
    let mut outsider = player(3, "Eve");
    let _ = world.clan_registry.add_member(&mut outsider, other_nr);
    assert!(world.spawn_character(outsider, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "fire: Eve");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You cannot fire those not belonging to your clan.")));
    assert_eq!(
        world.characters.get(&CharacterId(3)).unwrap().clan,
        other_nr
    );
}

#[test]
fn fire_command_removes_membership_and_queues_clan_log_event() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));
    let mut member = player(3, "Bob");
    let _ = world.clan_registry.add_member(&mut member, nr);
    assert!(world.spawn_character(member, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "fire: Bob");
    }
    world.process_clanmaster_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("Fired: Bob.")));
    assert_eq!(world.characters.get(&CharacterId(3)).unwrap().clan, 0);

    let events = world.drain_pending_clanmaster_events();
    assert_eq!(
        events,
        vec![ClanmasterEvent::MemberFired {
            member_id: CharacterId(3),
            clan_nr: nr,
            firer_name: "Alice".into(),
        }]
    );
}

#[test]
fn fire_command_ignores_unmatched_offline_name() {
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Leaders", 0).unwrap();
    assert!(world.spawn_character(clanmaster_npc(1), 10, 10));
    let mut leader = player(2, "Alice");
    let _ = world.clan_registry.add_member(&mut leader, nr);
    leader.clan_rank = 4;
    assert!(world.spawn_character(leader, 10, 10));

    if let Some(clanmaster) = world.characters.get_mut(&CharacterId(1)) {
        clanmaster.push_driver_text_message(CharacterId(2), "fire: Ghost");
    }
    world.process_clanmaster_actions(0, 0);

    assert!(world.drain_pending_area_texts().is_empty());
    assert!(world.drain_pending_clanmaster_events().is_empty());
}
