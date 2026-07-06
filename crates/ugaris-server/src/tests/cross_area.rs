use super::*;

fn connected_player(character_id: CharacterId, session_id: u64) -> (World, ServerRuntime) {
    let mut world = World::default();
    let mut character = login_character(character_id, &login_block("Traveler"), 3, 50, 60);
    character.x = 50;
    character.y = 60;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(session_id, commands, 0);
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.character_id = Some(character_id);
    }
    (world, runtime)
}

#[tokio::test]
async fn cross_area_transfer_stays_put_without_a_registered_repository_pair() {
    // C `change_area` only ever proceeds past its own `get_area` lookup;
    // without a live `AreaRepository`/`CharacterRepository` pair (no
    // `DATABASE_URL`, matching every other DB-optional codepath in this
    // codebase) there is nothing to resolve the target against, so the
    // character must stay exactly where it is - the caller falls back to
    // the legacy "target area server is down" text.
    let character_id = CharacterId(21);
    let (mut world, mut runtime) = connected_player(character_id, 1);

    let transferred = attempt_cross_area_transfer(
        &mut world,
        &mut runtime,
        &None,
        &None,
        3,
        1,
        character_id,
        6,
        4,
        139,
        75,
    )
    .await;

    assert!(!transferred);
    // The character is untouched: still present in the live world at its
    // original position, and the session is still attached (no
    // disconnect was sent).
    let character = world.characters.get(&character_id).expect("still present");
    assert_eq!((character.x, character.y), (50, 60));
    assert!(runtime.player_for_character(character_id).is_some());
    assert!(runtime.sessions.contains_key(&1));
}
