use super::*;
use ugaris_core::character_driver::{
    ArenaFighterDriverData, ArenaMasterDriverData, CDR_ARENAFIGHTER, CDR_ARENAMASTER,
    CDR_LAMPGHOST, CDR_SHR_WEREWOLF, CDR_WARPFIGHTER, MS_FIGHT,
};
use ugaris_core::world::npc::area25::WarpFighterDriverData;
use ugaris_core::world::LegacyHurtOutcome;

#[tokio::test]
async fn clan_economy_tick_escalates_mutual_relation_request_immediately() {
    // `apply_clan_economy_tick`'s relation half wires `ClanRelations::
    // update` (`clan.c:936-1089`) into the live tick loop; the escalation/
    // de-escalation state machine itself is exhaustively unit-tested in
    // `ugaris-core`'s `clan.rs`, so this only checks the wiring: the
    // registry's live relation state actually advances and the returned
    // `applied` count reflects the one pair-level change.
    let mut world = World::default();
    let a = world.clan_registry.found_clan("Alpha", 0).unwrap();
    let b = world.clan_registry.found_clan("Beta", 0).unwrap();
    world
        .clan_registry
        .relations_mut()
        .set_relation(a, b, ugaris_core::clan::ClanRelation::War, 0)
        .unwrap();
    world
        .clan_registry
        .relations_mut()
        .set_relation(b, a, ugaris_core::clan::ClanRelation::War, 0)
        .unwrap();

    let applied = apply_clan_economy_tick(&mut world, &None, 0).await;

    assert_eq!(applied, 1);
    assert_eq!(
        world.clan_registry.relations().current_relation(a, b),
        ugaris_core::clan::ClanRelation::War
    );
}

#[tokio::test]
async fn clan_economy_tick_deletes_a_clan_that_goes_broke() {
    // Wires `ClanRegistry::update_treasure` (`clan.c:1105-1159`) into the
    // live tick loop: a freshly founded clan with no jewels and a huge
    // elapsed `payed_till` gap accrues enough debt in one tick to be
    // deleted, matching what `/killclan`'s huge-debt trick eventually
    // triggers in C (`kill_clan`, `clan.c:1413-1416`).
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Broke", 0).unwrap();
    assert!(world.clan_registry.exists(nr));

    // cost = 5000, step = 120; diff = 250_000 => n = 250000/120 + 1 = 2084,
    // landing debt at 2084 (>= 2000) with zero jewels to pay it off (same
    // arithmetic as `ugaris-core`'s own
    // `update_treasure_deletes_clan_that_goes_broke_with_no_jewels` test).
    let applied = apply_clan_economy_tick(&mut world, &None, 250_000).await;

    assert_eq!(applied, 1);
    assert!(!world.clan_registry.exists(nr));
}

#[tokio::test]
async fn clan_economy_tick_advances_training_update_timestamp_after_an_hour() {
    // Wires `ClanRegistry::update_training` (`clan.c:1166-1182`) into the
    // live tick loop. `training_score` itself only ever decays (nothing
    // feeds it yet - the dungeon system that would is unported, see the
    // module doc comment), so a freshly founded clan's score stays `0`
    // either way; `last_training_update` advancing is the observable
    // signal that the sub-tick actually ran (exact 5%-decay arithmetic
    // is unit-tested directly in `ugaris-core`'s
    // `update_training_decays_score_by_five_percent_after_one_hour`).
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Trainers", 0).unwrap();
    assert_eq!(
        world
            .clan_registry
            .identity(nr)
            .unwrap()
            .economy
            .last_training_update,
        0
    );

    apply_clan_economy_tick(&mut world, &None, 3_600).await;

    assert_eq!(
        world
            .clan_registry
            .identity(nr)
            .unwrap()
            .economy
            .last_training_update,
        3_600
    );
}

#[test]
fn apply_arena_master_events_falls_back_to_fighter_bots_own_ledger_when_a_combatant_has_no_player_runtime(
) {
    // A real player (winner) and a `CDR_ARENAFIGHTER` practice bot (loser,
    // no `PlayerRuntime`) just finished an arena fight - the bot fled the
    // box, so `check_fight` scores the player as the winner.
    let mut world = World::default();
    let master_id = CharacterId(1);
    let winner_id = CharacterId(2);
    let loser_id = CharacterId(3);

    let mut master = login_character(master_id, &login_block("Arenamaster"), 3, 236, 145);
    master.flags.remove(CharacterFlags::PLAYER);
    master.driver = CDR_ARENAMASTER;
    master.driver_state = Some(CharacterDriverState::ArenaMaster(ArenaMasterDriverData {
        state: MS_FIGHT,
        fight1: Some(winner_id),
        fight2: Some(loser_id),
        timeout: 1_000,
        ..Default::default()
    }));
    world.add_character(master);

    let mut winner = login_character(winner_id, &login_block("Godmode"), 3, 235, 140);
    winner.x = 235;
    winner.y = 140;
    world.add_character(winner);

    // The fighter bot fled the arena box (outside the `234..=242,
    // 133..=141` bounds), so it loses by default this tick.
    let mut loser = login_character(loser_id, &login_block("Fighter"), 3, 10, 10);
    loser.flags.remove(CharacterFlags::PLAYER);
    loser.x = 10;
    loser.y = 10;
    loser.driver = CDR_ARENAFIGHTER;
    loser.driver_state = Some(CharacterDriverState::ArenaFighter(
        ArenaFighterDriverData::default(),
    ));
    world.add_character(loser);

    let mut runtime = ServerRuntime::default();
    let mut winner_player = PlayerRuntime::connected(20, 0);
    winner_player.character_id = Some(winner_id);
    runtime.players.insert(20, winner_player);

    world.process_arena_master_actions(0, |character_id| {
        runtime
            .player_for_character(character_id)
            .map(|player| player.arena_score())
            .unwrap_or(ARENA_PPD_NEWCOMER_SCORE)
    });

    let applied = apply_arena_master_events(&mut world, &mut runtime, 1_000_000);

    assert_eq!(applied, 1);
    // The winner's real `PlayerRuntime` arena_ppd was updated.
    let new_winner_score = runtime
        .player_for_character(winner_id)
        .unwrap()
        .arena_score();
    assert_eq!(
        new_winner_score,
        ARENA_PPD_NEWCOMER_SCORE + ugaris_core::player::PlayerRuntime::arena_fight_worth(0)
    );
    // The loser has no `PlayerRuntime` at all - its own local ledger
    // (`ArenaFighterDriverData`) was updated instead.
    assert_eq!(
        world.arena_fighter_score(loser_id),
        Some(ARENA_PPD_NEWCOMER_SCORE - ugaris_core::player::PlayerRuntime::arena_fight_worth(0))
    );
    let entries = world.arena_toplist_entries();
    assert!(entries.iter().any(|e| e.name == "Godmode"));
    assert!(entries.iter().any(|e| e.name == "Fighter"));
}

#[test]
fn lethal_lqnpc_hurt_schedules_respawn_and_marks_a_player_killer() {
    let mut world = World::default();
    let mut npc = login_character(CharacterId(1), &login_block("Quest Guard"), 20, 10, 10);
    npc.flags.remove(CharacterFlags::PLAYER);
    npc.driver = CDR_LQNPC;
    npc.hp = POWERSCALE;
    npc.driver_state = Some(CharacterDriverState::LqNpc(
        ugaris_core::world::npc::area20::LqNpcDriverData {
            slot: 3,
            kill_mark_id: 2,
            hurt_mark_id: 5,
            ..Default::default()
        },
    ));
    let killer = login_character(CharacterId(2), &login_block("Killer"), 20, 11, 10);
    world.add_character(npc);
    world.add_character(killer);
    assert!(world.configure_lq_npc(ugaris_core::world::LqNpcState {
        slot: 3,
        basename: "guard".to_string(),
        x: 10,
        y: 10,
        dir: 0,
        level: 1,
        mode: b'f',
        respawn_seconds: 30,
        name: String::new(),
        description: String::new(),
        nick: [String::new(), String::new()],
        character_id: None,
        character_serial: 0,
        sprite: 0,
        greeting: String::new(),
        trigger: [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new()
        ],
        reply: [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new()
        ],
        want_key_id: 0,
        reward_item: ugaris_core::world::LqItemSpec::default(),
        reward_mark_id: 0,
        kill_mark_id: 0,
        hurt_mark_id: 0,
        carry_item: ugaris_core::world::LqItemSpec::default(),
        carry_gold: 0,
    }));
    assert!(world.apply_lq_npc_spawn_result(3, CharacterId(1), 1));

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    assert_eq!(
        world.lq_npc_respawns,
        vec![(3, world.tick.0 + 30 * TICKS_PER_SECOND)]
    );
    let npc = world.lq_npcs.iter().find(|npc| npc.slot == 3).unwrap();
    assert_eq!(npc.character_id, None);
    assert_eq!(npc.character_serial, 0);
    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert!(player.lq_mark(2));
    assert!(player.lq_mark(5));
}

#[test]
fn lqnpc_death_handler_ignores_non_matching_driver_and_non_lethal_hits() {
    let mut world = World::default();
    let mut other_npc = login_character(CharacterId(1), &login_block("Other"), 20, 10, 10);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    other_npc.hp = POWERSCALE * 5;
    world.add_character(other_npc);

    let non_lethal = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: false,
            ..Default::default()
        },
    };
    let mut runtime = ServerRuntime::default();
    assert!(!apply_lqnpc_death_from_hurt_event(
        &mut runtime,
        &mut world,
        non_lethal
    ));

    let mut world2 = World::default();
    let mut wrong_driver_npc = login_character(CharacterId(1), &login_block("Other"), 20, 10, 10);
    wrong_driver_npc.flags.remove(CharacterFlags::PLAYER);
    wrong_driver_npc.driver = CDR_LAMPGHOST; // not CDR_LQNPC
    world2.add_character(wrong_driver_npc);

    let lethal_wrong_driver = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_lqnpc_death_from_hurt_event(
        &mut runtime,
        &mut world2,
        lethal_wrong_driver
    ));
}

fn warpfighter_npc(
    id: CharacterId,
    owner: CharacterId,
    owner_serial: u32,
    xs: u16,
    xe: u16,
    ys: u16,
    ye: u16,
) -> Character {
    let mut fighter = login_character(id, &login_block("Hrus-tak-lan"), 25, 15, 15);
    fighter.flags.remove(CharacterFlags::PLAYER);
    fighter.driver = CDR_WARPFIGHTER;
    fighter.driver_state = Some(CharacterDriverState::WarpFighter(WarpFighterDriverData {
        owner,
        owner_serial,
        tx: 40,
        ty: 41,
        xs,
        xe,
        ys,
        ye,
        creation_time: 0,
        pot_done: 0,
    }));
    fighter
}

#[test]
fn warpfighter_death_teleports_the_owner_who_landed_the_killing_blow() {
    let mut world = World::default();
    world.add_character(warpfighter_npc(
        CharacterId(1),
        CharacterId(2),
        2,
        10,
        20,
        10,
        20,
    ));
    let mut owner = login_character(CharacterId(2), &login_block("Godmode"), 25, 15, 15);
    owner.x = 15;
    owner.y = 15;
    world.add_character(owner);

    let lethal_by_owner = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(apply_warpfighter_death_from_hurt_event(
        &mut world,
        lethal_by_owner
    ));
    let owner = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((owner.x, owner.y), (40, 41));
}

#[test]
fn warpfighter_death_ignores_a_kill_by_someone_other_than_the_owner() {
    let mut world = World::default();
    world.add_character(warpfighter_npc(
        CharacterId(1),
        CharacterId(2),
        2,
        10,
        20,
        10,
        20,
    ));
    let mut owner = login_character(CharacterId(2), &login_block("Godmode"), 25, 15, 15);
    owner.x = 15;
    owner.y = 15;
    world.add_character(owner);
    let mut someone_else = login_character(CharacterId(3), &login_block("Bystander"), 25, 16, 15);
    world.add_character(someone_else.clone());
    someone_else.x = 16;

    let lethal_by_someone_else = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(3),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_warpfighter_death_from_hurt_event(
        &mut world,
        lethal_by_someone_else
    ));
    let owner = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((owner.x, owner.y), (15, 15));
}

#[test]
fn warpfighter_death_ignores_the_owner_having_already_left_the_room() {
    let mut world = World::default();
    world.add_character(warpfighter_npc(
        CharacterId(1),
        CharacterId(2),
        2,
        10,
        20,
        10,
        20,
    ));
    let mut owner = login_character(CharacterId(2), &login_block("Godmode"), 25, 15, 15);
    owner.x = 99; // outside xs=10..xe=20
    owner.y = 15;
    world.add_character(owner);

    let lethal_by_owner = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_warpfighter_death_from_hurt_event(
        &mut world,
        lethal_by_owner
    ));
}

fn shr_werewolf_npc(character_id: CharacterId) -> Character {
    let mut werewolf = login_character(character_id, &login_block("Werewolf"), 38, 120, 235);
    werewolf.flags.remove(CharacterFlags::PLAYER);
    werewolf.driver = CDR_SHR_WEREWOLF;
    werewolf.hp = POWERSCALE;
    werewolf
}

#[test]
fn shr_werewolf_death_drops_mist_sets_death_sprite_and_grumbles_at_its_player_killer() {
    // C `shr_werewolf_dead` (`shrike.c:344-354`).
    let mut world = World::default();
    world.add_character(shr_werewolf_npc(CharacterId(1)));
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        38,
        121,
        235,
    ));

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let werewolf = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(werewolf.sprite, 6);
    assert!(
        !world.effects.is_empty(),
        "create_mist should add an effect"
    );

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.area1_shrike_fails(), 1);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| {
        text.message
            .contains("I have deserved death. But still... I was hoping for something better.")
    }));
}

#[test]
fn shr_werewolf_death_handler_ignores_non_matching_driver_and_non_lethal_hits() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player);

    // Non-`CDR_SHR_WEREWOLF` driver: no counter/sprite change even on a
    // lethal hit.
    let mut other_npc = login_character(CharacterId(1), &login_block("Other"), 38, 120, 235);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    other_npc.hp = POWERSCALE;
    world.add_character(other_npc);
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        38,
        121,
        235,
    ));
    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());
    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_shrike_fails(),
        0
    );

    // Non-lethal hit on a real werewolf: no counter/sprite change.
    world.add_character(shr_werewolf_npc(CharacterId(3)));
    world.apply_legacy_hurt(CharacterId(3), Some(CharacterId(2)), 1, 1, 0, 0);
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());
    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_shrike_fails(),
        0
    );
    let werewolf = world.characters.get(&CharacterId(3)).unwrap();
    assert_ne!(werewolf.sprite, 6);
}
