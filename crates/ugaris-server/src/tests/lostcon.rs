use super::*;
use ugaris_core::character_driver::LostconDriverData;

fn lingering_character(character_id: CharacterId) -> Character {
    login_character(character_id, &login_block("Tester"), 1, 10, 10)
}

fn stashed_player(session_id: u64, character_id: CharacterId) -> PlayerRuntime {
    let mut player = PlayerRuntime::connected(session_id, 0);
    player.character_id = Some(character_id);
    player.character_number = character_id.0;
    player.ppd_blob = vec![7, 7, 7];
    player
}

#[test]
fn enter_lostcon_on_disconnect_arms_deadline_and_stashes_player() {
    let mut world = World::default();
    let character_id = CharacterId(1);
    world.add_character(lingering_character(character_id));
    let mut runtime = ServerRuntime::default();
    let player = stashed_player(1, character_id);
    let account_depot = AccountDepotState::default();

    let leftover = enter_lostcon_on_disconnect(
        &mut world,
        &mut runtime,
        character_id,
        player,
        Some(account_depot),
        1_000,
        7_200,
    );

    assert!(leftover.is_none(), "character should linger, not fall back");
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.driver, CDR_LOSTCON);
    assert_eq!(
        character.driver_state,
        Some(CharacterDriverState::Lostcon(LostconDriverData {
            deadline: 8_200
        }))
    );
    let stashed = runtime.lostcon_players.get(&character_id).unwrap();
    assert_eq!(stashed.ppd_blob, vec![7, 7, 7]);
    assert!(runtime.account_depots.contains_key(&character_id));
}

#[test]
fn enter_lostcon_on_disconnect_falls_back_when_character_missing() {
    let mut world = World::default();
    let character_id = CharacterId(1);
    let mut runtime = ServerRuntime::default();
    let player = stashed_player(1, character_id);

    let leftover = enter_lostcon_on_disconnect(
        &mut world,
        &mut runtime,
        character_id,
        player,
        None,
        1_000,
        7_200,
    );

    assert!(
        leftover.is_some(),
        "missing character should fall back to immediate save"
    );
    assert!(!runtime.lostcon_players.contains_key(&character_id));
}

#[test]
fn reclaim_lostcon_on_login_restores_stashed_player_and_clears_driver() {
    let mut world = World::default();
    let character_id = CharacterId(1);
    world.add_character(lingering_character(character_id));
    world.enter_lostcon(character_id, 8_200);
    let mut runtime = ServerRuntime::default();
    runtime
        .lostcon_players
        .insert(character_id, stashed_player(1, character_id));

    let new_session_id = 2;
    let reclaimed = reclaim_lostcon_on_login(
        &mut world,
        &mut runtime,
        new_session_id,
        character_id,
        9_000,
    );

    assert!(reclaimed);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.driver, 0);
    assert_eq!(character.driver_state, None);
    assert!(!runtime.lostcon_players.contains_key(&character_id));
    let restored = runtime.players.get(&new_session_id).unwrap();
    assert_eq!(restored.character_id, Some(character_id));
    assert_eq!(restored.ppd_blob, vec![7, 7, 7]);
    assert_eq!(restored.session_id, new_session_id);
}

#[test]
fn reclaim_lostcon_on_login_returns_false_when_not_lingering() {
    let mut world = World::default();
    let character_id = CharacterId(1);
    world.add_character(lingering_character(character_id));
    let mut runtime = ServerRuntime::default();

    let reclaimed = reclaim_lostcon_on_login(&mut world, &mut runtime, 2, character_id, 9_000);

    assert!(!reclaimed);
    assert!(runtime.players.get(&2).is_none());
}

#[test]
fn take_expired_lostcon_characters_returns_and_clears_only_expired_entries() {
    let mut world = World::default();
    let early_id = CharacterId(1);
    let late_id = CharacterId(2);
    world.add_character(lingering_character(early_id));
    world.add_character(lingering_character(late_id));
    world.enter_lostcon(early_id, 100);
    world.enter_lostcon(late_id, 500);
    let mut runtime = ServerRuntime::default();
    runtime
        .lostcon_players
        .insert(early_id, stashed_player(1, early_id));
    runtime
        .lostcon_players
        .insert(late_id, stashed_player(2, late_id));
    runtime
        .account_depots
        .insert(early_id, AccountDepotState::default());

    let expired = take_expired_lostcon_characters(&world, &mut runtime, 200);

    assert_eq!(expired.len(), 1);
    let (character_id, player, account_depot) = &expired[0];
    assert_eq!(*character_id, early_id);
    assert_eq!(player.ppd_blob, vec![7, 7, 7]);
    assert!(account_depot.is_some());
    assert!(!runtime.lostcon_players.contains_key(&early_id));
    assert!(!runtime.account_depots.contains_key(&early_id));
    // Not-yet-expired entry is left untouched.
    assert!(runtime.lostcon_players.contains_key(&late_id));
}
