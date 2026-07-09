use crate::character_driver::CDR_LQNPC;
use crate::world::npc::area20::LqNpcDriverData;

use super::*;

fn god(world: &mut World, id: u32, x: u16, y: u16) -> CharacterId {
    let character_id = CharacterId(id);
    let mut spawned = character(id);
    spawned.flags = CharacterFlags::USED | CharacterFlags::GOD;
    spawned.x = x;
    spawned.y = y;
    world.characters.insert(character_id, spawned);
    character_id
}

fn plain_player(world: &mut World, id: u32, x: u16, y: u16) -> CharacterId {
    let character_id = CharacterId(id);
    let mut spawned = character(id);
    spawned.x = x;
    spawned.y = y;
    world.characters.insert(character_id, spawned);
    character_id
}

/// A live `CDR_LQNPC` character (no map placement needed - these tests
/// exercise `apply_lq_usurp_command`'s pure `World::characters` scan, not
/// `process_lqnpc_tick`'s movement, which has its own coverage in
/// `world::tests::lqnpc`).
fn lq_npc(world: &mut World, id: u32, name: &str, x: u16, y: u16) -> CharacterId {
    let character_id = CharacterId(id);
    let mut spawned = character(id);
    spawned.name = name.to_string();
    spawned.driver = CDR_LQNPC;
    spawned.x = x;
    spawned.y = y;
    spawned.driver_state = Some(CharacterDriverState::LqNpc(LqNpcDriverData::default()));
    world.characters.insert(character_id, spawned);
    character_id
}

fn npc_data(world: &World, id: u32) -> LqNpcDriverData {
    match world
        .characters
        .get(&CharacterId(id))
        .and_then(|character| character.driver_state.as_ref())
    {
        Some(CharacterDriverState::LqNpc(data)) => data.clone(),
        other => panic!("expected LqNpc driver state for {id}, got {other:?}"),
    }
}

fn plain_texts(world: &mut World) -> Vec<String> {
    world
        .drain_pending_system_texts()
        .into_iter()
        .map(|event| event.message)
        .collect()
}

#[test]
fn wimp_is_not_recognized_outside_area_20_or_35() {
    let mut world = World::default();
    let caller = plain_player(&mut world, 1, 50, 50);
    assert!(!world.apply_lq_usurp_command(caller, 1, "#wimp"));
    assert!(world.drain_pending_lq_wimps().is_empty());
}

#[test]
fn wimp_needs_no_god_or_lqmaster_flag() {
    let mut world = World::default();
    let caller = plain_player(&mut world, 1, 50, 50);
    assert!(world.apply_lq_usurp_command(caller, 20, "#wimp"));
    assert_eq!(plain_texts(&mut world), vec!["You wimped out.".to_string()]);
    assert_eq!(world.drain_pending_lq_wimps(), vec![caller]);
    // C `cmd_wimp`'s candidate list starts at (240,240) and the caller
    // wasn't near any of it, so the very first candidate succeeds.
    let character = world.characters.get(&caller).unwrap();
    assert_eq!((character.x, character.y), (240, 240));
}

#[test]
fn wimp_also_works_via_slash_prefix_and_in_area_35() {
    let mut world = World::default();
    let caller = plain_player(&mut world, 1, 50, 50);
    assert!(world.apply_lq_usurp_command(caller, 35, "/wimp"));
}

#[test]
fn usurp_requires_god_or_lqmaster() {
    let mut world = World::default();
    let caller = plain_player(&mut world, 1, 5, 5);
    lq_npc(&mut world, 2, "Gate Guard", 6, 5);
    assert!(!world.apply_lq_usurp_command(caller, 20, "#usurp guard"));
    assert!(world.characters.get(&caller).unwrap().lq_usurp.is_none());
}

#[test]
fn usurp_picks_the_nearest_name_match_and_links_both_sides() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    let far = lq_npc(&mut world, 2, "Gate Guard", 20, 10);
    let near = lq_npc(&mut world, 3, "Gate Guard", 12, 10);

    assert!(world.apply_lq_usurp_command(caller, 20, "#usurp guard"));
    assert_eq!(plain_texts(&mut world), vec!["Done.".to_string()]);

    assert_eq!(world.characters.get(&caller).unwrap().lq_usurp, Some(near));
    let near_data = npc_data(&world, 3);
    assert_eq!(near_data.usurp, Some(caller));
    // C `dat->udx = ch[cn].x - ch[bco].x;` etc (`lq.c:2197-2198`).
    assert_eq!(near_data.udx, 10 - 12);
    assert_eq!(near_data.udy, 0);
    // The farther match is untouched.
    assert_eq!(npc_data(&world, 2).usurp, None);
    let _ = far;
}

#[test]
fn usurp_reports_npc_not_found() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(world.apply_lq_usurp_command(caller, 20, "#usurp nobody"));
    assert_eq!(plain_texts(&mut world), vec!["NPC not found.".to_string()]);
}

#[test]
fn usurp_out_of_range_npc_is_not_a_candidate() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    lq_npc(&mut world, 2, "Gate Guard", 40, 40);
    assert!(world.apply_lq_usurp_command(caller, 20, "#usurp guard"));
    assert_eq!(plain_texts(&mut world), vec!["NPC not found.".to_string()]);
}

#[test]
fn usurp_replaces_any_existing_usurp_with_a_single_done_message() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    let first = lq_npc(&mut world, 2, "Alpha", 11, 10);
    let second = lq_npc(&mut world, 3, "Beta", 12, 10);

    assert!(world.apply_lq_usurp_command(caller, 20, "#usurp alpha"));
    plain_texts(&mut world);
    assert!(world.apply_lq_usurp_command(caller, 20, "#usurp beta"));

    // Only the second command's "Done." - `cmd_exit`'s internal pre-clear
    // (`ptr == NULL`) never prints anything (`lq.c:2191`).
    assert_eq!(plain_texts(&mut world), vec!["Done.".to_string()]);
    assert_eq!(
        world.characters.get(&caller).unwrap().lq_usurp,
        Some(second)
    );
    assert_eq!(npc_data(&world, 2).usurp, None);
    assert_eq!(npc_data(&world, 3).usurp, Some(caller));
    let _ = first;
}

#[test]
fn follow_sets_follow_on_every_matching_npc_and_reports_the_count() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    lq_npc(&mut world, 2, "Soldier Alpha", 11, 10);
    lq_npc(&mut world, 3, "Soldier Beta", 12, 10);
    lq_npc(&mut world, 4, "Merchant", 13, 10);

    assert!(world.apply_lq_usurp_command(caller, 20, "#follow soldier"));
    assert_eq!(
        plain_texts(&mut world),
        vec!["Set 2 NPCs to follow.".to_string()]
    );
    assert_eq!(npc_data(&world, 2).follow, Some(caller));
    assert_eq!(npc_data(&world, 3).follow, Some(caller));
    assert_eq!(npc_data(&world, 4).follow, None);
}

#[test]
fn stop_clears_follow_on_matching_npcs() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    let target = lq_npc(&mut world, 2, "Soldier", 11, 10);
    if let Some(CharacterDriverState::LqNpc(data)) = world
        .characters
        .get_mut(&target)
        .and_then(|character| character.driver_state.as_mut())
    {
        data.follow = Some(caller);
    }

    assert!(world.apply_lq_usurp_command(caller, 20, "#stop soldier"));
    assert_eq!(
        plain_texts(&mut world),
        vec!["Set 1 NPCs to stop.".to_string()]
    );
    assert_eq!(npc_data(&world, 2).follow, None);
}

#[test]
fn exit_clears_an_active_usurp_and_always_shows_done() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    let target = lq_npc(&mut world, 2, "Gate Guard", 11, 10);
    assert!(world.apply_lq_usurp_command(caller, 20, "#usurp guard"));
    plain_texts(&mut world);

    assert!(world.apply_lq_usurp_command(caller, 20, "#exit"));
    assert_eq!(plain_texts(&mut world), vec!["Done.".to_string()]);
    assert!(world.characters.get(&caller).unwrap().lq_usurp.is_none());
    assert_eq!(npc_data(&world, 2).usurp, None);
    let _ = target;
}

#[test]
fn exit_with_no_active_usurp_still_shows_done() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    assert!(world.apply_lq_usurp_command(caller, 20, "#exit"));
    assert_eq!(plain_texts(&mut world), vec!["Done.".to_string()]);
}

#[test]
fn plain_speech_relays_as_the_possessed_npcs_own_say_while_usurping() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    lq_npc(&mut world, 2, "Gate Guard", 11, 10);
    assert!(world.apply_lq_usurp_command(caller, 20, "#usurp guard"));
    plain_texts(&mut world);

    assert!(world.apply_lq_usurp_command(caller, 20, "open the gate"));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Gate Guard") && text.message.contains("open the gate")));
    // The player's own name never appears as the speaker.
    assert!(!texts
        .iter()
        .any(|text| text.message.starts_with("Character")));
}

#[test]
fn plain_speech_is_not_relayed_without_an_active_usurp() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    assert!(!world.apply_lq_usurp_command(caller, 20, "hello there"));
}

#[test]
fn plain_speech_relay_requires_mutual_pairing() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    let npc_id = lq_npc(&mut world, 2, "Gate Guard", 11, 10);
    // The player thinks it's usurping, but the NPC's own `usurp` was
    // reassigned elsewhere (e.g. another `#usurp` overwrote it) - C's own
    // `dat->usurp == cn` check (`lq.c:2734-2735`) would reject this too.
    if let Some(character) = world.characters.get_mut(&caller) {
        character.lq_usurp = Some(npc_id);
    }
    assert!(!world.apply_lq_usurp_command(caller, 20, "hello there"));
}

#[test]
fn me_and_emote_relay_to_the_possessed_npcs_emote() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    lq_npc(&mut world, 2, "Gate Guard", 11, 10);
    assert!(world.apply_lq_usurp_command(caller, 20, "#usurp guard"));
    plain_texts(&mut world);

    assert!(world.apply_lq_usurp_command(caller, 20, "#me waves"));
    let texts = world.drain_pending_area_texts();
    // C `emote(co, "%s", ptr + len)` doesn't trim the leading space right
    // after the matched "me"/"emote" word (`lq.c:2707`) - the doubled
    // space is a real, preserved C quirk, not a test typo.
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Gate Guard  waves.")));
}

#[test]
fn me_relay_is_not_reachable_without_an_active_usurp() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 10, 10);
    assert!(!world.apply_lq_usurp_command(caller, 20, "#me waves"));
}
