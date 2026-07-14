// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::character_driver::{
    ArenaFighterDriverData, ArenaManagerDriverData, ArenaMasterDriverData,
    ARENA_FIGHTER_MASTER_POS, ARENA_FIGHTER_REST_POS, CDR_ARENAFIGHTER, CDR_ARENAMANAGER,
    CDR_ARENAMASTER, FS_ENTER, FS_FIGHT, FS_LEISURE, FS_REGISTER, FS_START, FS_WAIT, FS_WAIT2,
    MS_FIGHT, MS_IN, MS_PAIR, NTID_ARENA,
};
use crate::player::PlayerRuntime;
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

/// Zone-file-accurate rental arena bounds (`ugaris_data/zones/3/
/// above3_generic.chr`'s first `CDR_ARENAMANAGER` instance: `arg="arenax
/// =233;arenay=122;arenafx=230;arenafy=119;arenatx=242;arenaty=125;"`).
fn arena_manager(id: u32) -> Character {
    let mut manager = character(id);
    manager.name = "Arenamanager".into();
    manager.driver = CDR_ARENAMANAGER;
    manager.driver_state = Some(CharacterDriverState::ArenaManager(ArenaManagerDriverData {
        arena_x: 233,
        arena_y: 122,
        arena_fx: 230,
        arena_fy: 119,
        arena_tx: 242,
        arena_ty: 125,
        ..Default::default()
    }));
    manager
}

fn manager_data(world: &World, manager_id: CharacterId) -> ArenaManagerDriverData {
    match world
        .characters
        .get(&manager_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::ArenaManager(data)) => data,
        _ => panic!("expected arena manager driver state"),
    }
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

    // `empty_arena`'s `teleport_char_driver(co, ch[cn].x, ch[cn].y)`
    // targets the master's own tile - since the master is standing there,
    // the exact tile is occupied, so C's `drop_char` (which always tries
    // the exact tile, then its 8 neighbors, regardless of its `nosteptrap`
    // flag argument - see `World::arena_teleport_char_driver`'s doc
    // comment) lands Alice on whichever neighbor tile it reaches first,
    // not exactly on the master's own tile.
    let alice = world.characters.get(&CharacterId(2)).unwrap();
    assert!(alice.x.abs_diff(236) <= 1 && alice.y.abs_diff(145) <= 1);
    assert_ne!((alice.x, alice.y), (235, 140));
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

// `CDR_ARENAFIGHTER` (`fighter_driver`) tests below.

fn arena_fighter(id: u32) -> Character {
    let mut fighter = character(id);
    fighter.name = "Fighter".into();
    fighter.driver = CDR_ARENAFIGHTER;
    fighter.rest_x = ARENA_FIGHTER_REST_POS.0;
    fighter.rest_y = ARENA_FIGHTER_REST_POS.1;
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        last_act: -(TICKS_PER_SECOND as i64) * 60 * 6,
        ..Default::default()
    }));
    fighter
}

fn fighter_data(world: &World, id: CharacterId) -> ArenaFighterDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::ArenaFighter(data)) => data,
        _ => panic!("expected arena fighter driver state"),
    }
}

#[test]
fn fighter_leisure_advances_to_start_once_home_and_settled() {
    let mut world = World::default();
    let (rest_x, rest_y) = ARENA_FIGHTER_REST_POS;
    assert!(world.spawn_character(arena_fighter(1), rest_x as usize, rest_y as usize));

    world.process_arena_fighter_actions(0);

    assert_eq!(fighter_data(&world, CharacterId(1)).state, FS_START);
}

#[test]
fn fighter_leisure_walks_home_when_far_away() {
    let mut world = World::default();
    let (rest_x, rest_y) = ARENA_FIGHTER_REST_POS;
    world
        .map
        .tile_mut(rest_x as usize, rest_y as usize)
        .unwrap()
        .light = 255;
    assert!(world.spawn_character(arena_fighter(1), (rest_x - 3) as usize, rest_y as usize));

    world.process_arena_fighter_actions(0);

    let fighter = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(fighter.action, action::WALK);
    // The move consumed this tick's action (C's early `return`), so the
    // state hasn't advanced yet.
    assert_eq!(fighter_data(&world, CharacterId(1)).state, FS_LEISURE);
}

#[test]
fn fighter_start_advances_to_register_when_near_master_position() {
    let mut world = World::default();
    let (master_x, master_y) = ARENA_FIGHTER_MASTER_POS;
    let mut fighter = arena_fighter(1);
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        state: FS_START,
        ..Default::default()
    }));
    assert!(world.spawn_character(fighter, master_x as usize, master_y as usize));

    world.process_arena_fighter_actions(0);

    assert_eq!(fighter_data(&world, CharacterId(1)).state, FS_REGISTER);
}

#[test]
fn fighter_start_walks_toward_master_when_far() {
    let mut world = World::default();
    let (master_x, master_y) = ARENA_FIGHTER_MASTER_POS;
    world
        .map
        .tile_mut(master_x as usize, master_y as usize)
        .unwrap()
        .light = 255;
    let mut fighter = arena_fighter(1);
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        state: FS_START,
        ..Default::default()
    }));
    assert!(world.spawn_character(fighter, (master_x - 10) as usize, master_y as usize));

    world.process_arena_fighter_actions(0);

    let fighter = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(fighter.action, action::WALK);
    assert_eq!(fighter_data(&world, CharacterId(1)).state, FS_START);
}

#[test]
fn fighter_register_state_says_register_and_registers_with_nearby_master() {
    let mut world = World::default();
    let (master_x, master_y) = ARENA_FIGHTER_MASTER_POS;
    assert!(world.spawn_character(arena_master(1), master_x as usize, master_y as usize));
    let mut fighter = arena_fighter(2);
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        state: FS_REGISTER,
        last_act: -(TICKS_PER_SECOND as i64) * 60 * 5,
        ..Default::default()
    }));
    assert!(world.spawn_character(fighter, master_x as usize, master_y as usize));

    world.process_arena_fighter_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Fighter says: \"register\"")));
    let data = master_data(&world, CharacterId(1));
    assert_eq!(data.contenders.len(), 1);
    assert_eq!(data.contenders[0].character_id, CharacterId(2));
    assert_eq!(data.contenders[0].score, -2000);
}

#[test]
fn fighter_register_state_keeps_saying_register_until_thirty_seconds_pass() {
    let mut world = World::default();
    let mut fighter = arena_fighter(1);
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        state: FS_REGISTER,
        last_act: 100,
        ..Default::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    world.tick = Tick(100 + TICKS_PER_SECOND * 30 - 1);

    world.process_arena_fighter_actions(0);

    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(fighter_data(&world, CharacterId(1)).last_act, 100);
}

#[test]
fn fighter_wait_transitions_to_enter_on_paired_message() {
    let mut world = World::default();
    let mut fighter = arena_fighter(1);
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        state: FS_WAIT,
        ..Default::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    if let Some(f) = world.characters.get_mut(&CharacterId(1)) {
        f.push_driver_message(NT_NPC, NTID_ARENA, 0, 0);
    }

    world.process_arena_fighter_actions(0);

    // The message handler seeds `last_act` deeply in the past
    // (`-TICKS*60*5`, matching C's `arena.c:955`), which the very same
    // tick's `FS_ENTER` action branch then immediately reads as "long
    // enough ago" and overwrites with the current tick after saying
    // "enter" - exactly like C's own chained same-tick state advance.
    let data = fighter_data(&world, CharacterId(1));
    assert_eq!(data.state, FS_ENTER);
    assert_eq!(data.last_act, world.tick.0 as i64);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Fighter says: \"enter\"")));
}

#[test]
fn fighter_enter_state_enters_the_box_when_invited() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_master(1), 10, 10));
    let mut fighter = arena_fighter(2);
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        state: FS_ENTER,
        last_act: -(TICKS_PER_SECOND as i64) * 60 * 5,
        ..Default::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    let mut master_data_val = ArenaMasterDriverData {
        state: MS_IN,
        fight1: Some(CharacterId(2)),
        fight2: Some(CharacterId(99)),
        ..Default::default()
    };
    master_data_val.timeout = 1000;
    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.driver_state = Some(CharacterDriverState::ArenaMaster(master_data_val));
    }

    world.process_arena_fighter_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Fighter says: \"enter\"")));
    let fighter = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((fighter.x, fighter.y), (235, 140));
}

#[test]
fn fighter_wait2_transitions_to_fight_with_enemy_on_attack_now_message() {
    let mut world = World::default();
    let mut fighter = arena_fighter(1);
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        state: FS_WAIT2,
        ..Default::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    // The assigned enemy must be a real character, or this same tick's
    // `FS_FIGHT` visibility scan (`arena_fighter_update_enemy_visibility`,
    // C's `fight_driver_update` trashing a stale/deleted enemy slot)
    // immediately clears it again.
    assert!(world.spawn_character(character(42), 11, 10));
    if let Some(f) = world.characters.get_mut(&CharacterId(1)) {
        f.push_driver_message(NT_NPC, NTID_ARENA, 1, 42);
    }

    world.process_arena_fighter_actions(0);

    let data = fighter_data(&world, CharacterId(1));
    assert_eq!(data.state, FS_FIGHT);
    assert_eq!(data.enemy, Some(CharacterId(42)));
}

#[test]
fn fighter_fight_state_attacks_visible_enemy() {
    let mut world = World::default();
    let mut fighter = arena_fighter(1);
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        state: FS_FIGHT,
        enemy: Some(CharacterId(2)),
        ..Default::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    assert!(world.spawn_character(character(2), 11, 10));

    world.process_arena_fighter_actions(0);

    let fighter = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(fighter.action, action::ATTACK1);
    assert!(fighter_data(&world, CharacterId(1)).enemy_visible);
}

#[test]
fn fighter_fight_state_resets_to_leisure_on_fight_over_message() {
    let mut world = World::default();
    let mut fighter = arena_fighter(1);
    fighter.driver_state = Some(CharacterDriverState::ArenaFighter(ArenaFighterDriverData {
        state: FS_FIGHT,
        enemy: Some(CharacterId(2)),
        ..Default::default()
    }));
    assert!(world.spawn_character(fighter, 10, 10));
    if let Some(f) = world.characters.get_mut(&CharacterId(1)) {
        f.push_driver_message(NT_NPC, NTID_ARENA, 2, 0);
    }
    world.tick = Tick(500);

    world.process_arena_fighter_actions(0);

    let data = fighter_data(&world, CharacterId(1));
    assert_eq!(data.state, FS_LEISURE);
    assert_eq!(data.last_act, 500);
}

#[test]
fn fighter_give_message_destroys_the_item() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_fighter(1), 10, 10));
    let item_id = ItemId(700);
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
    if let Some(f) = world.characters.get_mut(&CharacterId(1)) {
        f.cursor_item = Some(item_id);
        f.push_driver_message(NT_GIVE, 0, 0, 0);
    }

    world.process_arena_fighter_actions(0);

    assert!(!world.items.contains_key(&item_id));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn arena_fighter_score_seeds_newcomer_until_first_recorded_fight() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_fighter(1), 10, 10));

    assert_eq!(world.arena_fighter_score(CharacterId(1)), Some(-2000));
}

#[test]
fn apply_arena_fighter_win_updates_local_ledger() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_fighter(1), 10, 10));

    let new_score = world
        .apply_arena_fighter_win(CharacterId(1), -2000)
        .unwrap();

    let expected = -2000 + PlayerRuntime::arena_fight_worth(0);
    assert_eq!(new_score, expected);
    let data = fighter_data(&world, CharacterId(1));
    assert_eq!(data.score, expected);
    assert_eq!(data.fights, 1);
    assert_eq!(data.wins, 1);
    assert_eq!(data.losses, 0);
}

#[test]
fn apply_arena_fighter_loss_updates_local_ledger() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_fighter(1), 10, 10));

    let new_score = world
        .apply_arena_fighter_loss(CharacterId(1), -2000)
        .unwrap();

    let expected = -2000 - PlayerRuntime::arena_fight_worth(0);
    assert_eq!(new_score, expected);
    let data = fighter_data(&world, CharacterId(1));
    assert_eq!(data.score, expected);
    assert_eq!(data.fights, 1);
    assert_eq!(data.losses, 1);
    assert_eq!(data.wins, 0);
}

#[test]
fn manager_rent_command_reserves_arena_and_teleports_renter_in() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_manager(1), 100, 100));
    // Inside the listening box (`230 < x < 242`) but outside the narrower
    // `232..=238` occupation column `is_anybody_in` scans, so the
    // requester's own presence never falsely reads as "already occupied"
    // (see `manager_rent_command_rejects_when_arena_already_occupied`'s
    // comment for why that column even matters here).
    assert!(world.spawn_character(player(2, "Godmode"), 231, 121));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(2), "rent");
    }
    world.process_arena_manager_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Say 'invite: <name>' to let someone in")));

    assert_eq!(
        manager_data(&world, CharacterId(1)).renter,
        Some(CharacterId(2))
    );
    let renter = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((renter.x, renter.y), (233, 122));
}

#[test]
fn manager_rent_command_rejects_when_arena_already_occupied() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_manager(1), 100, 100));
    // `Occupant` stands inside the hardcoded `232..=238` occupation
    // column `is_anybody_in` scans; `Newcomer` deliberately stands
    // *outside* that column (but still inside the wider listening box)
    // so this test isolates "someone else is in the arena" from the
    // requester's own presence also landing in that column (see
    // `manager_rent_command_reserves_arena_and_teleports_renter_in`'s
    // comment).
    assert!(world.spawn_character(player(2, "Occupant"), 234, 120));
    assert!(world.spawn_character(player(3, "Newcomer"), 240, 123));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(3), "rent");
    }
    world.process_arena_manager_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Sorry, this arena is already occupied.")));
    assert_eq!(manager_data(&world, CharacterId(1)).renter, None);
    let newcomer = world.characters.get(&CharacterId(3)).unwrap();
    assert_eq!((newcomer.x, newcomer.y), (240, 123));
}

#[test]
fn manager_leave_command_teleports_speaker_back_and_clears_renter() {
    let mut world = World::default();
    let mut manager = arena_manager(1);
    manager.driver_state = Some(CharacterDriverState::ArenaManager(ArenaManagerDriverData {
        renter: Some(CharacterId(2)),
        invite: "Foo".into(),
        arena_x: 233,
        arena_y: 122,
        arena_fx: 230,
        arena_fy: 119,
        arena_tx: 242,
        arena_ty: 125,
        ..Default::default()
    }));
    assert!(world.spawn_character(manager, 5, 5));
    assert!(world.spawn_character(player(2, "Godmode"), 235, 121));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(2), "leave");
    }
    world.process_arena_manager_actions(0);

    // The manager's own tile (5, 5) is occupied by the manager itself, so
    // C's `drop_char`-style neighbor fallback lands the leaving player on
    // one of its 8 neighbor tiles rather than the exact tile - see
    // `World::arena_teleport_char_driver`'s doc comment.
    let renter = world.characters.get(&CharacterId(2)).unwrap();
    assert!(renter.x.abs_diff(5) <= 1 && renter.y.abs_diff(5) <= 1);
    assert_ne!((renter.x, renter.y), (235, 121));
    let data = manager_data(&world, CharacterId(1));
    assert_eq!(data.renter, None);
    assert!(data.invite.is_empty());
}

#[test]
fn manager_leave_command_from_non_renter_does_not_clear_reservation() {
    let mut world = World::default();
    let mut manager = arena_manager(1);
    manager.driver_state = Some(CharacterDriverState::ArenaManager(ArenaManagerDriverData {
        renter: Some(CharacterId(2)),
        invite: "Foo".into(),
        arena_x: 233,
        arena_y: 122,
        arena_fx: 230,
        arena_fy: 119,
        arena_tx: 242,
        arena_ty: 125,
        ..Default::default()
    }));
    assert!(world.spawn_character(manager, 5, 5));
    assert!(world.spawn_character(player(3, "Other"), 236, 122));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(3), "leave");
    }
    world.process_arena_manager_actions(0);

    let other = world.characters.get(&CharacterId(3)).unwrap();
    assert!(other.x.abs_diff(5) <= 1 && other.y.abs_diff(5) <= 1);
    assert_ne!((other.x, other.y), (236, 122));
    let data = manager_data(&world, CharacterId(1));
    assert_eq!(data.renter, Some(CharacterId(2)));
    assert_eq!(data.invite, "Foo");
}

#[test]
fn manager_enter_command_teleports_invited_player_and_clears_lag() {
    let mut world = World::default();
    let mut manager = arena_manager(1);
    manager.driver_state = Some(CharacterDriverState::ArenaManager(ArenaManagerDriverData {
        renter: Some(CharacterId(2)),
        invite: "Bob".into(),
        arena_x: 233,
        arena_y: 122,
        arena_fx: 230,
        arena_fy: 119,
        arena_tx: 242,
        arena_ty: 125,
        ..Default::default()
    }));
    assert!(world.spawn_character(manager, 100, 100));
    let mut bob = player(4, "Bob");
    bob.flags |= CharacterFlags::LAG;
    assert!(world.spawn_character(bob, 236, 123));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(4), "enter");
    }
    world.process_arena_manager_actions(0);

    let bob = world.characters.get(&CharacterId(4)).unwrap();
    assert_eq!((bob.x, bob.y), (233, 122));
    assert!(!bob.flags.contains(CharacterFlags::LAG));
    assert!(manager_data(&world, CharacterId(1)).invite.is_empty());
}

#[test]
fn manager_enter_command_rejects_uninvited_player() {
    let mut world = World::default();
    let mut manager = arena_manager(1);
    manager.driver_state = Some(CharacterDriverState::ArenaManager(ArenaManagerDriverData {
        renter: Some(CharacterId(2)),
        invite: "Bob".into(),
        arena_x: 233,
        arena_y: 122,
        arena_fx: 230,
        arena_fy: 119,
        arena_tx: 242,
        arena_ty: 125,
        ..Default::default()
    }));
    assert!(world.spawn_character(manager, 100, 100));
    assert!(world.spawn_character(player(5, "Eve"), 236, 123));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(5), "enter");
    }
    world.process_arena_manager_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You have not been invited, Eve.")));
    let eve = world.characters.get(&CharacterId(5)).unwrap();
    assert_eq!((eve.x, eve.y), (236, 123));
    assert_eq!(manager_data(&world, CharacterId(1)).invite, "Bob");
}

#[test]
fn manager_invite_command_sets_invite_and_notifies() {
    let mut world = World::default();
    let mut manager = arena_manager(1);
    manager.driver_state = Some(CharacterDriverState::ArenaManager(ArenaManagerDriverData {
        renter: Some(CharacterId(2)),
        arena_x: 233,
        arena_y: 122,
        arena_fx: 230,
        arena_fy: 119,
        arena_tx: 242,
        arena_ty: 125,
        ..Default::default()
    }));
    assert!(world.spawn_character(manager, 100, 100));
    assert!(world.spawn_character(player(2, "Godmode"), 235, 121));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(2), "invite: Bob");
    }
    world.process_arena_manager_actions(0);

    assert_eq!(manager_data(&world, CharacterId(1)).invite, "Bob");
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Bob, say 'enter' if you wish to enter the arena")));
}

#[test]
fn manager_invite_command_rejects_non_renter() {
    let mut world = World::default();
    let mut manager = arena_manager(1);
    manager.driver_state = Some(CharacterDriverState::ArenaManager(ArenaManagerDriverData {
        renter: Some(CharacterId(2)),
        arena_x: 233,
        arena_y: 122,
        arena_fx: 230,
        arena_fy: 119,
        arena_tx: 242,
        arena_ty: 125,
        ..Default::default()
    }));
    assert!(world.spawn_character(manager, 100, 100));
    assert!(world.spawn_character(player(3, "Trespasser"), 236, 122));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(3), "invite: Eve");
    }
    world.process_arena_manager_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("This is not your arena, Trespasser.")));
    assert!(manager_data(&world, CharacterId(1)).invite.is_empty());
}

#[test]
fn manager_ignores_messages_from_outside_the_listening_box() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_manager(1), 100, 100));
    assert!(world.spawn_character(player(2, "Godmode"), 50, 50));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(2), "rent");
    }
    world.process_arena_manager_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(!texts
        .iter()
        .any(|t| t.message.contains("occupied") || t.message.contains("Say 'invite")));
    assert_eq!(manager_data(&world, CharacterId(1)).renter, None);
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((godmode.x, godmode.y), (50, 50));
}

#[test]
fn manager_evicts_renter_who_wandered_outside_the_narrow_rental_band() {
    let mut world = World::default();
    let mut manager = arena_manager(1);
    manager.driver_state = Some(CharacterDriverState::ArenaManager(ArenaManagerDriverData {
        renter: Some(CharacterId(2)),
        invite: "Bob".into(),
        arena_x: 233,
        arena_y: 122,
        arena_fx: 230,
        arena_fy: 119,
        arena_tx: 242,
        arena_ty: 125,
        ..Default::default()
    }));
    assert!(world.spawn_character(manager, 100, 100));
    // Still strictly inside the listening box (230 < 239 < 242, 119 < 121
    // < 125) but outside the narrower hardcoded `232..=238` rental band.
    assert!(world.spawn_character(player(2, "Godmode"), 239, 121));

    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.push_driver_text_message(CharacterId(2), "test");
    }
    world.process_arena_manager_actions(0);

    let data = manager_data(&world, CharacterId(1));
    assert_eq!(data.renter, None);
    assert!(data.invite.is_empty());
}

#[test]
fn manager_give_message_destroys_the_item() {
    let mut world = World::default();
    assert!(world.spawn_character(arena_manager(1), 100, 100));
    let item_id = ItemId(701);
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
    if let Some(manager) = world.characters.get_mut(&CharacterId(1)) {
        manager.cursor_item = Some(item_id);
        manager.push_driver_message(NT_GIVE, 0, 0, 0);
    }

    world.process_arena_manager_actions(0);

    assert!(!world.items.contains_key(&item_id));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Thou hast better use for this than I do")));
}
