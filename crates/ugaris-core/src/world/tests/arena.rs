use super::*;
use crate::character_driver::{
    ArenaMasterDriverData, CDR_ARENAMASTER, MS_FIGHT, MS_IN, MS_PAIR, NTID_ARENA,
};
use crate::world::arena::{ArenaMasterEvent, ArenaToplistRecord, ARENA_TOPLIST_SIZE};

fn arena_master(id: u32) -> Character {
    let mut master = character(id);
    master.name = "Arenamaster".into();
    master.driver = CDR_ARENAMASTER;
    master.driver_state = Some(CharacterDriverState::ArenaMaster(
        ArenaMasterDriverData::default(),
    ));
    master
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn master_data(world: &World, master_id: CharacterId) -> ArenaMasterDriverData {
    match world
        .characters
        .get(&master_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::ArenaMaster(data)) => data,
        _ => panic!("expected arena master driver state"),
    }
}

fn no_score(_: CharacterId) -> i32 {
    -2000
}

#[test]
fn register_command_adds_contender_and_notifies() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "register");
    }
    world.process_arena_master_actions(0, no_score);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Good luck, Godmode. I will call you")));

    let data = master_data(&world, CharacterId(1));
    assert_eq!(data.contenders.len(), 1);
    assert_eq!(data.contenders[0].character_id, CharacterId(2));
    assert_eq!(data.contenders[0].score, -2000);

    let registrant = world.characters.get(&CharacterId(2)).unwrap();
    assert!(registrant
        .driver_messages
        .iter()
        .any(|m| m.message_type == NT_NPC && m.dat1 == NTID_ARENA && m.dat2 == 3));
}

#[test]
fn register_command_rejects_duplicate_registration() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "register");
    }
    world.process_arena_master_actions(0, no_score);
    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "register");
    }
    world.process_arena_master_actions(0, no_score);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You're already registered for this tournament")));
    assert_eq!(master_data(&world, CharacterId(1)).contenders.len(), 1);
}

#[test]
fn register_command_rejects_when_no_free_slots() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 10, 10));

    // Every fake contender must be a real, still-existing character or
    // `arena_find_contender`'s own stale-slot pruning (which also runs
    // this same tick, since the master's default state is `MS_PAIR`)
    // would drop them all before the final count assertion below.
    let mut data = ArenaMasterDriverData::default();
    for n in 0..crate::character_driver::ARENA_MAX_CONTENDER {
        let id = 1000 + n as u32;
        assert!(world.spawn_character(player(id, "Filler"), 50 + n, 50));
        data.contenders
            .push(crate::character_driver::ArenaContender {
                character_id: CharacterId(id),
                score: 0,
                reg_time: 0,
            });
    }
    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.driver_state = Some(CharacterDriverState::ArenaMaster(data));
    }
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "register");
    }
    world.process_arena_master_actions(0, no_score);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("there are no free slots at the moment")));
    assert_eq!(
        master_data(&world, CharacterId(1)).contenders.len(),
        crate::character_driver::ARENA_MAX_CONTENDER
    );
}

#[test]
fn find_contender_pairs_closest_scores_and_starts_thirty_second_timer() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 10, 10));
    assert!(world.spawn_character(player(2, "Alice"), 10, 10));
    assert!(world.spawn_character(player(3, "Bob"), 10, 10));
    assert!(world.spawn_character(player(4, "Carol"), 10, 10));
    world.tick.0 = 100;

    let mut data = ArenaMasterDriverData::default();
    data.contenders
        .push(crate::character_driver::ArenaContender {
            character_id: CharacterId(2),
            score: 0,
            reg_time: 100,
        });
    data.contenders
        .push(crate::character_driver::ArenaContender {
            character_id: CharacterId(3),
            score: 5000,
            reg_time: 100,
        });
    data.contenders
        .push(crate::character_driver::ArenaContender {
            character_id: CharacterId(4),
            score: 10,
            reg_time: 100,
        });
    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.driver_state = Some(CharacterDriverState::ArenaMaster(data));
    }

    world.process_arena_master_actions(0, no_score);

    let data = master_data(&world, CharacterId(1));
    assert_eq!(data.state, MS_IN);
    // Alice (score 0) and Carol (score 10) are the closest match.
    assert_eq!(data.fight1, Some(CharacterId(2)));
    assert_eq!(data.fight2, Some(CharacterId(4)));
    assert_eq!(data.timeout, 100 + TICKS_PER_SECOND * 30);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Next fight is: Alice versus Carol.")));
}

fn paired_master(world: &mut World, fight1: CharacterId, fight2: CharacterId, tick: u64) {
    let mut data = ArenaMasterDriverData::default();
    data.state = MS_IN;
    data.fight1 = Some(fight1);
    data.fight2 = Some(fight2);
    data.timeout = tick + TICKS_PER_SECOND * 30;
    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.driver_state = Some(CharacterDriverState::ArenaMaster(data));
    }
    world.tick.0 = tick;
}

#[test]
fn check_inside_starts_fight_when_both_entered_the_box() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 236, 145));
    assert!(world.spawn_character(player(2, "Alice"), 235, 140));
    assert!(world.spawn_character(player(3, "Bob"), 241, 134));
    paired_master(&mut world, CharacterId(2), CharacterId(3), 100);

    world.process_arena_master_actions(0, no_score);

    let data = master_data(&world, CharacterId(1));
    assert_eq!(data.state, MS_FIGHT);
    assert_eq!(data.timeout, 100 + TICKS_PER_SECOND * 60 * 2);

    let alice = world.characters.get(&CharacterId(2)).unwrap();
    assert!(alice
        .driver_messages
        .iter()
        .any(|m| m.message_type == NT_NPC && m.dat2 == 1 && m.dat3 == 3));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Let the fight begin!")));
}

#[test]
fn check_inside_keeps_waiting_while_a_fighter_has_not_entered_yet() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 236, 145));
    // Alice never left the master's rest position (outside the arena box).
    let alice_start = world.spawn_character(player(2, "Alice"), 236, 145);
    assert!(alice_start);
    assert!(world.spawn_character(player(3, "Bob"), 241, 134));
    paired_master(&mut world, CharacterId(2), CharacterId(3), 100);

    world.process_arena_master_actions(0, no_score);

    // Still within the 30-second entry window and Alice hasn't stepped
    // into the box yet - stays MS_IN, matching C's early `return`.
    assert_eq!(master_data(&world, CharacterId(1)).state, MS_IN);
}

#[test]
fn check_inside_advances_to_fight_after_timeout_even_if_a_fighter_never_entered() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 236, 145));
    // Alice never left the master's rest position (outside the arena box).
    assert!(world.spawn_character(player(2, "Alice"), 236, 145));
    assert!(world.spawn_character(player(3, "Bob"), 241, 134));
    paired_master(&mut world, CharacterId(2), CharacterId(3), 100);
    world.tick.0 = master_data(&world, CharacterId(1)).timeout + 1;

    world.process_arena_master_actions(0, no_score);

    // Timeout expired - transitions to MS_FIGHT anyway; the missing
    // fighter loses on the very next check_fight tick instead.
    assert_eq!(master_data(&world, CharacterId(1)).state, MS_FIGHT);
}

fn fighting_master(world: &mut World, fight1: CharacterId, fight2: CharacterId, tick: u64) {
    let mut data = ArenaMasterDriverData::default();
    data.state = MS_FIGHT;
    data.fight1 = Some(fight1);
    data.fight2 = Some(fight2);
    data.timeout = tick + TICKS_PER_SECOND * 60 * 2;
    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.driver_state = Some(CharacterDriverState::ArenaMaster(data));
    }
    world.tick.0 = tick;
}

#[test]
fn check_fight_continues_while_both_fighters_remain_inside() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 236, 145));
    assert!(world.spawn_character(player(2, "Alice"), 235, 140));
    assert!(world.spawn_character(player(3, "Bob"), 241, 134));
    fighting_master(&mut world, CharacterId(2), CharacterId(3), 100);

    world.process_arena_master_actions(0, no_score);

    assert_eq!(master_data(&world, CharacterId(1)).state, MS_FIGHT);
    assert!(world.drain_pending_arena_master_events().is_empty());
}

#[test]
fn check_fight_scores_the_survivor_when_the_loser_leaves_the_box() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 236, 145));
    assert!(world.spawn_character(player(2, "Alice"), 235, 140));
    // Bob fled the arena box.
    assert!(world.spawn_character(player(3, "Bob"), 10, 10));
    fighting_master(&mut world, CharacterId(2), CharacterId(3), 100);

    world.process_arena_master_actions(0, no_score);

    let data = master_data(&world, CharacterId(1));
    assert_eq!(data.state, MS_PAIR);

    let events = world.drain_pending_arena_master_events();
    assert_eq!(
        events,
        vec![ArenaMasterEvent::FightScored {
            winner_id: CharacterId(2),
            loser_id: CharacterId(3),
        }]
    );

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("And the winner is Alice.")));

    // empty_arena's `teleport_char_driver(co, ch[cn].x, ch[cn].y)` targets
    // the master's own tile - since the master is standing there, C's
    // `drop_char(..., 0)` (radius 0, no nearby-tile fallback) genuinely
    // fails in this exact scenario (a single arena master NPC, no other
    // occupant of its own tile to bump), so the winner is left exactly
    // where the fight ended. This matches C's real behavior, not a gap in
    // this port.
    let alice = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((alice.x, alice.y), (235, 140));
}

#[test]
fn check_fight_declares_a_draw_on_timeout_with_both_still_inside() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 236, 145));
    assert!(world.spawn_character(player(2, "Alice"), 235, 140));
    assert!(world.spawn_character(player(3, "Bob"), 241, 134));
    fighting_master(&mut world, CharacterId(2), CharacterId(3), 100);
    world.tick.0 = master_data(&world, CharacterId(1)).timeout + 1;

    world.process_arena_master_actions(0, no_score);

    assert_eq!(master_data(&world, CharacterId(1)).state, MS_PAIR);
    assert!(world.drain_pending_arena_master_events().is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Hu? No one won? Oh well...")));
}

#[test]
fn enter_command_teleports_invited_fighters_into_the_box() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 236, 145));
    assert!(world.spawn_character(player(2, "Alice"), 236, 145));
    assert!(world.spawn_character(player(3, "Bob"), 236, 145));
    paired_master(&mut world, CharacterId(2), CharacterId(3), 100);

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "enter");
    }
    world.process_arena_master_actions(0, no_score);

    let alice = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((alice.x, alice.y), (235, 140));
}

#[test]
fn enter_command_rejects_uninvited_player() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 236, 145));
    assert!(world.spawn_character(player(2, "Alice"), 236, 145));
    assert!(world.spawn_character(player(3, "Bob"), 236, 145));
    assert!(world.spawn_character(player(4, "Eve"), 236, 145));
    paired_master(&mut world, CharacterId(2), CharacterId(3), 100);

    // Multiple characters requested the same nominal tile above, so the
    // map's drop-nearby-tile fallback may have placed Eve a step away -
    // capture her actual position rather than assuming the exact
    // requested coordinates.
    let eve_start = {
        let eve = world.characters.get(&CharacterId(4)).unwrap();
        (eve.x, eve.y)
    };

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(4), "enter");
    }
    world.process_arena_master_actions(0, no_score);

    let eve = world.characters.get(&CharacterId(4)).unwrap();
    assert_eq!((eve.x, eve.y), eve_start);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("You are not invited to this fight, Eve.")));
}

#[test]
fn leave_command_teleports_to_the_rest_area() {
    let mut world = World::default();
    // Kept adjacent to the master (matching every other text-command
    // test) so `char_see_char` doesn't need extra map lighting set up -
    // this command only cares about the speaker's identity, not their
    // starting position.
    assert!(world.spawn_character(arena_master(1), 10, 10));
    assert!(world.spawn_character(player(2, "Alice"), 10, 10));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "leave");
    }
    world.process_arena_master_actions(0, no_score);

    let alice = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((alice.x, alice.y), (238, 146));
}

#[test]
fn give_message_says_once_then_destroys_the_item() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 10, 10));
    let item_id = ItemId(500);
    world.items.insert(
        item_id,
        crate::entity::Item {
            id: item_id,
            name: "Junk".into(),
            description: String::new(),
            flags: ItemFlags::empty(),
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; MAX_MODIFIERS],
            modifier_value: [0; MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: 0,
        },
    );
    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.cursor_item = Some(item_id);
        master.push_driver_message(NT_GIVE, 2, 0, 0);
    }

    world.process_arena_master_actions(0, no_score);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Thou hast better use for this than I do.")));
    assert!(!world.items.contains_key(&item_id));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert_eq!(master_data(&world, CharacterId(1)).amgivingback, 0);
}

#[test]
fn arena_update_toplist_dedups_existing_names_and_sorts_descending() {
    let mut world = World::default();
    world.arena_update_toplist("Alice", "Bob", 100, 50, 1_000);
    world.arena_update_toplist("Carol", "Alice", 200, 150, 2_000);

    let entries = world.arena_toplist_entries();
    assert_eq!(entries[0].name, "Carol");
    assert_eq!(entries[0].score, 200);
    assert_eq!(entries[1].name, "Alice");
    assert_eq!(entries[1].score, 150);
    assert_eq!(entries[2].name, "Bob");
    assert_eq!(entries[2].score, 50);
    // Alice must appear exactly once even though she fought twice.
    assert_eq!(entries.iter().filter(|e| e.name == "Alice").count(), 1);
}

#[test]
fn arena_update_toplist_evicts_entries_stale_for_over_a_week() {
    let mut world = World::default();
    world.arena_toplist = vec![ArenaToplistRecord::default(); ARENA_TOPLIST_SIZE];
    world.arena_toplist[0] = ArenaToplistRecord {
        name: "Stale".into(),
        score: 9999,
        updated: 0,
    };

    let one_week_and_a_bit = 60 * 60 * 24 * 7 + 1;
    world.arena_update_toplist("Newcomer1", "Newcomer2", 1, 1, one_week_and_a_bit);

    let entries = world.arena_toplist_entries();
    assert!(!entries.iter().any(|e| e.name == "Stale"));
}
