use super::*;
use ugaris_core::character_driver::CDR_LAMPGHOST;

#[test]
fn hurt_events_add_pk_hate_and_clear_lag_for_valid_player_hit() {
    let mut world = World::default();
    let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    target
        .flags
        .insert(CharacterFlags::PK | CharacterFlags::LAG);
    target.level = 10;
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.flags.insert(CharacterFlags::PK);
    attacker.level = 12;
    world.add_character(target);
    world.add_character(attacker);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);

    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 123, &ZoneLoader::new()),
        1
    );
    assert!(runtime
        .player_for_character(CharacterId(1))
        .unwrap()
        .has_pk_hate_for(2));
    assert!(!world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .flags
        .contains(CharacterFlags::LAG));
}

#[test]
fn lethal_teufel_rat_hurt_updates_legacy_rat_ppd_score() {
    let mut world = World::default();
    let mut rat = login_character(CharacterId(1), &login_block("Rat"), 34, 10, 10);
    rat.flags.remove(CharacterFlags::PLAYER);
    rat.driver = CDR_TEUFELRAT;
    rat.level = 80;
    rat.hp = POWERSCALE;
    let mut killer = login_character(CharacterId(2), &login_block("Killer"), 34, 11, 10);
    killer.flags.insert(CharacterFlags::LAG);
    world.add_character(rat);
    world.add_character(killer);

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
    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new()),
        0
    );

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.teufel_rat_kills, 1);
    assert_eq!(player.teufel_rat_score, 1);
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message == "#90 1 Rat Kills"));
    assert!(texts.iter().any(|text| text.message == "#80 1 Rat Points"));
}

#[test]
fn lethal_caligar_skelly_hurt_marks_killer_door_lock_ppd() {
    let mut world = World::default();
    let mut skelly = login_character(CharacterId(1), &login_block("Skelly"), 36, 10, 10);
    skelly.flags.remove(CharacterFlags::PLAYER);
    skelly.driver = CDR_CALIGARSKELLY;
    skelly.rest_x = 103;
    skelly.rest_y = 224;
    skelly.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 36, 11, 10);
    world.add_character(skelly);
    world.add_character(killer);

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
    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new()),
        0
    );

    assert!(!runtime
        .player_for_character(CharacterId(2))
        .unwrap()
        .caligar_skelly_door_unlocked(0));
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(2)
            && text.message
                == "You hear a faint sound in the distance, as if a lock was partially opened."
    }));
}

#[test]
fn lethal_caligar_skelly_hurt_reports_completed_and_repeated_locks() {
    let mut world = World::default();
    let mut killer = login_character(CharacterId(2), &login_block("Killer"), 36, 11, 10);
    killer.hp = POWERSCALE * 10;
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.mark_caligar_skelly_death(103, 224);
    player.mark_caligar_skelly_death(103, 211);
    runtime.players.insert(1, player);

    for (id, y) in [(1, 198), (3, 198)] {
        let mut skelly = login_character(CharacterId(id), &login_block("Skelly"), 36, 10, 10);
        skelly.flags.remove(CharacterFlags::PLAYER);
        skelly.driver = CDR_CALIGARSKELLY;
        skelly.rest_x = 103;
        skelly.rest_y = y;
        skelly.hp = POWERSCALE;
        world.add_character(skelly);
        world.apply_legacy_hurt(
            CharacterId(id),
            Some(CharacterId(2)),
            POWERSCALE * 2,
            1,
            0,
            0,
        );
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());
    }

    assert!(runtime
        .player_for_character(CharacterId(2))
        .unwrap()
        .caligar_skelly_door_unlocked(0));
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.message == "You hear a \"click\" in the distance, as if a lock had opened."
    }));
    assert!(texts.iter().any(|text| {
        text.message
            == "You expect to hear a click, but nothing happens. Maybe you've been here before?"
    }));
}

#[test]
fn lethal_riverbeast_hurt_advances_jiu_state_to_beast_killed() {
    let mut world = World::default();
    let mut riverbeast = login_character(CharacterId(1), &login_block("Riverbeast"), 1, 10, 10);
    riverbeast.flags.remove(CharacterFlags::PLAYER);
    riverbeast.driver = CDR_RIVERBEAST;
    riverbeast.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(riverbeast);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `JIU_STATE_WAIT_FOR_KILL 2` (`npc_states.h:78`).
    player.set_area1_jiu_state(2);
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

    // C `JIU_STATE_BEAST_KILLED 3` (`npc_states.h:79`).
    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_jiu_state(),
        3
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(2)
            && text.message == "Well done. Jiu will be proud of thee!"
    }));
}

#[test]
fn lethal_riverbeast_hurt_ignores_players_not_awaiting_the_kill() {
    let mut world = World::default();
    let mut riverbeast = login_character(CharacterId(1), &login_block("Riverbeast"), 1, 10, 10);
    riverbeast.flags.remove(CharacterFlags::PLAYER);
    riverbeast.driver = CDR_RIVERBEAST;
    riverbeast.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(riverbeast);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `JIU_STATE_ENTRY 0` (`npc_states.h:76`) - hasn't taken the quest.
    player.set_area1_jiu_state(0);
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
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_jiu_state(),
        0
    );
    assert!(world.drain_pending_system_texts().is_empty());
}

/// C `ch_died_driver`/`CDR_LAMPGHOST` -> `lampghost_dead` (`area3.c:2741-
/// 2752`): the dying lampghost's claimed lamp is released so another
/// lampghost (or the same one on respawn) can pick it up.
#[test]
fn lethal_lampghost_hurt_releases_its_lamp_claim() {
    let mut world = World::default();
    let mut lampghost = login_character(CharacterId(1), &login_block("Lampghost"), 1, 10, 10);
    lampghost.flags.remove(CharacterFlags::PLAYER);
    lampghost.driver = CDR_LAMPGHOST;
    lampghost.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(lampghost);
    world.add_character(killer);
    world
        .area3_lamp_claims
        .insert(ItemId(9), (CharacterId(1), 5));
    // An unrelated claim by a different lampghost must survive untouched.
    world
        .area3_lamp_claims
        .insert(ItemId(10), (CharacterId(3), 2));

    let mut runtime = ServerRuntime::default();
    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    assert!(!world.area3_lamp_claims.contains_key(&ItemId(9)));
    assert_eq!(
        world.area3_lamp_claims.get(&ItemId(10)),
        Some(&(CharacterId(3), 2))
    );
}

#[test]
fn lethal_bredel_hurt_advances_jessica_state_to_quest2_finish() {
    let mut world = World::default();
    let mut bredel = login_character(CharacterId(1), &login_block("Bredel"), 1, 10, 10);
    bredel.flags.remove(CharacterFlags::PLAYER);
    bredel.driver = CDR_BREDEL;
    bredel.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(bredel);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `JESSICA_STATE_QUEST2_DO 10` (`npc_states.h:94`).
    player.set_area1_jessica_state(10);
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

    // C `JESSICA_STATE_QUEST2_FINISH 11` (`npc_states.h:95`).
    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_jessica_state(),
        11
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(2)
            && text.message
                == "The local robber leader has been killed by thine hands. Congratulations!"
    }));
}

#[test]
fn lethal_bigbadspider_hurt_completes_brithildie_quest_and_advances_state() {
    let mut world = World::default();
    let mut bigbadspider = login_character(CharacterId(1), &login_block("Spider"), 1, 10, 10);
    bigbadspider.flags.remove(CharacterFlags::PLAYER);
    bigbadspider.driver = CDR_BIGBADSPIDER;
    bigbadspider.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(bigbadspider);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `BRITHILDIE_STATE_NOMORETALES_QOPEN 20` (`npc_states.h:71`).
    player.set_area1_brithildie_state(20);
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

    // C `BRITHILDIE_STATE_NOMORETALES_QDONE 21` (`npc_states.h:72`).
    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_brithildie_state(),
        21
    );
    assert!(runtime
        .player_for_character(CharacterId(2))
        .unwrap()
        .quest_log
        .is_done(QLOG_BRITHILDIE));
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(2)
            && text.message == "Well done. Thou hast killed the big bad spider."
    }));
}

#[test]
fn lethal_bigbadspider_hurt_ignores_players_not_awaiting_the_reward() {
    let mut world = World::default();
    let mut bigbadspider = login_character(CharacterId(1), &login_block("Spider"), 1, 10, 10);
    bigbadspider.flags.remove(CharacterFlags::PLAYER);
    bigbadspider.driver = CDR_BIGBADSPIDER;
    bigbadspider.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(bigbadspider);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `BRITHILDIE_STATE_ENTRY 0` (`npc_states.h:51`) - never took the
    // quest.
    player.set_area1_brithildie_state(0);
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
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_brithildie_state(),
        0
    );
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn lethal_forest_monster_hurt_counts_camhermit_kills_and_reports_at_ten() {
    let mut world = World::default();
    let killer = login_character(CharacterId(99), &login_block("Killer"), 1, 20, 60);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(99));
    // C `CAMHERMIT_STATE_QUEST1DO 5` (`npc_states.h:16`).
    player.set_area1_camhermit_state(5);
    player.set_area1_camhermit_kills(9);
    runtime.players.insert(1, player);

    let mut bear = login_character(CharacterId(1), &login_block("Bear"), 1, 10, 10);
    bear.flags.remove(CharacterFlags::PLAYER);
    bear.driver = CDR_CAMERON_FORESTMONSTER;
    bear.hp = POWERSCALE;
    world.add_character(bear);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(99)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    assert_eq!(
        runtime
            .player_for_character(CharacterId(99))
            .unwrap()
            .area1_camhermit_kills(),
        10
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(99)
            && text.message
                == "Thou hast killed 10 big bears as requested by the sweet Hermit. go back to him and claim thy reward."
    }));
}

#[test]
fn lethal_forest_monster_hurt_counts_imp_kills_and_advances_state_past_twenty() {
    let mut world = World::default();
    let killer = login_character(CharacterId(99), &login_block("Killer"), 16, 20, 60);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(99));
    player.set_area3_imp_state(2);
    player.set_area3_imp_kills(20);
    runtime.players.insert(1, player);

    // C `ch[cn].sprite == 306` (`forest.c:828`) - the `bear35` template.
    let mut bear = login_character(CharacterId(1), &login_block("Bear"), 16, 10, 10);
    bear.flags.remove(CharacterFlags::PLAYER);
    bear.driver = CDR_FORESTMONSTER;
    bear.sprite = 306;
    bear.hp = POWERSCALE;
    world.add_character(bear);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(99)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let player = runtime.player_for_character(CharacterId(99)).unwrap();
    assert_eq!(player.area3_imp_kills(), 21);
    assert_eq!(player.area3_imp_state(), 3);
}

#[test]
fn lethal_forest_monster_hurt_with_hardkill_flag_advances_hermit_state() {
    let mut world = World::default();
    let mut killer = login_character(CharacterId(99), &login_block("Killer"), 16, 20, 60);
    // The spider queen's `CF_HARDKILL` flag nullifies damage from any
    // weapon but the forged `IID_HARDKILL` one (`hurt.rs:119-125`) - give
    // the killer one at a sufficient level so the hit actually lands.
    killer.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(500));
    world.add_character(killer);
    let mut weapon = ugaris_core::entity::Item {
        id: ItemId(500),
        name: "Hardkill Weapon".into(),
        description: String::new(),
        flags: ugaris_core::entity::ItemFlags::empty(),
        sprite: 0,
        value: 0,
        min_level: 0,
        max_level: 99,
        needs_class: 0,
        template_id: IID_HARDKILL,
        owner_id: 0,
        modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
        modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: Some(CharacterId(99)),
        contained_in: None,
        content_id: 0,
        driver: 0,
        driver_data: vec![0; 38],
        serial: 0,
    };
    weapon.driver_data[37] = 99;
    world.add_item(weapon);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(99));
    player.set_area3_hermit_state(4);
    runtime.players.insert(1, player);

    let mut queen = login_character(CharacterId(1), &login_block("Spider Queen"), 16, 10, 10);
    queen.flags.remove(CharacterFlags::PLAYER);
    queen.driver = CDR_FORESTMONSTER;
    queen.flags.insert(CharacterFlags::HARDKILL);
    queen.level = 1;
    queen.hp = POWERSCALE;
    world.add_character(queen);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(99)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    assert_eq!(
        runtime
            .player_for_character(CharacterId(99))
            .unwrap()
            .area3_hermit_state(),
        5
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(99) && text.message == "Thou hast slain the spider queen."
    }));
}

#[test]
fn hurt_events_start_legacy_player_fightback_for_nearby_attacker() {
    let mut world = World::default();
    let target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.serial = 99;
    world.add_character(target);
    world.add_character(attacker);
    world.tick.0 = TICKS_PER_SECOND * 4;

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);
    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new()),
        0
    );

    let player = runtime.player_for_character(CharacterId(1)).unwrap();
    assert_eq!(player.action.action, PlayerActionCode::Kill);
    assert_eq!((player.action.arg1, player.action.arg2), (2, 99));
}

#[test]
fn hurt_events_defer_legacy_player_fightback_while_busy() {
    let mut world = World::default();
    let target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.serial = 99;
    world.add_character(target);
    world.add_character(attacker);
    world.tick.0 = TICKS_PER_SECOND * 4;

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.driver_move(12, 10);
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let player = runtime.player_for_character(CharacterId(1)).unwrap();
    assert_eq!(player.action.action, PlayerActionCode::Move);
    assert_eq!(player.next_fightback_character, Some(CharacterId(2)));
    assert_eq!(player.next_fightback_serial, 99);
}

#[test]
fn setup_world_actions_promotes_deferred_legacy_player_fightback() {
    let mut world = World::default();
    let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    target.x = 10;
    target.y = 10;
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.flags.remove(CharacterFlags::PLAYER);
    attacker.x = 11;
    attacker.y = 10;
    attacker.serial = 99;
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world.map.tile_mut(11, 10).unwrap().character = 2;
    world.add_character(target);
    world.add_character(attacker);
    world.tick.0 = TICKS_PER_SECOND * 4 + 1;

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.next_fightback_character = Some(CharacterId(2));
    player.next_fightback_serial = 99;
    player.next_fightback_tick = TICKS_PER_SECOND * 4;
    runtime.players.insert(1, player);

    assert_eq!(runtime.setup_world_actions(&mut world, 1), 1);

    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.action, action::ATTACK1);
    assert_eq!(target.act1, 2);
}

#[test]
fn hurt_events_respect_legacy_pk_hate_level_gate() {
    let mut world = World::default();
    let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    target
        .flags
        .insert(CharacterFlags::PK | CharacterFlags::LAG);
    target.level = 10;
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.flags.insert(CharacterFlags::PK);
    attacker.level = 14;
    world.add_character(target);
    world.add_character(attacker);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);

    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 123, &ZoneLoader::new()),
        0
    );
    assert!(!runtime
        .player_for_character(CharacterId(1))
        .unwrap()
        .has_pk_hate_for(2));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .flags
        .contains(CharacterFlags::LAG));
}

#[test]
fn lethal_pk_hurt_events_update_kill_and_death_counters() {
    let mut world = World::default();
    let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    target.flags.insert(CharacterFlags::PK);
    target.level = 10;
    target.hp = 100;
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.flags.insert(CharacterFlags::PK);
    attacker.level = 11;
    world.add_character(target);
    world.add_character(attacker);

    let mut runtime = ServerRuntime::default();
    let mut target_player = PlayerRuntime::connected(1, 0);
    target_player.character_id = Some(CharacterId(1));
    let mut attacker_player = PlayerRuntime::connected(2, 0);
    attacker_player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, target_player);
    runtime.players.insert(2, attacker_player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 1_000, 1, 0, 0);

    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 12_345, &ZoneLoader::new()),
        1
    );
    let target_player = runtime.player_for_character(CharacterId(1)).unwrap();
    assert_eq!(target_player.pk_deaths, 1);
    assert_eq!(target_player.pk_last_death, 12_345);
    let attacker_player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(attacker_player.pk_kills, 1);
    assert_eq!(attacker_player.pk_last_kill, 12_345);
}

#[test]
fn lethal_gate_fight_hurt_grants_arch_warrior_and_teleports_killer() {
    let mut world = World::default();
    let mut opponent = login_character(CharacterId(1), &login_block("Gatekeeper"), 1, 190, 200);
    opponent.flags.remove(CharacterFlags::PLAYER);
    opponent.driver = CDR_GATE_FIGHT;
    opponent.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 1, 191, 200);
    world.add_character(opponent);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.gate_target_class = 5;
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

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(killer.flags.contains(CharacterFlags::ARCH));
    assert_eq!((killer.x, killer.y), (181, 198));
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "You are an Arch-Warrior now."));
}

#[test]
fn lethal_gate_fight_hurt_by_non_player_does_not_grant_reward() {
    let mut world = World::default();
    let mut opponent = login_character(CharacterId(1), &login_block("Gatekeeper"), 1, 190, 200);
    opponent.flags.remove(CharacterFlags::PLAYER);
    opponent.driver = CDR_GATE_FIGHT;
    opponent.hp = POWERSCALE;
    let mut other_npc = login_character(CharacterId(2), &login_block("Other"), 1, 191, 200);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    world.add_character(opponent);
    world.add_character(other_npc);

    let mut runtime = ServerRuntime::default();

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let other_npc = world.characters.get(&CharacterId(2)).unwrap();
    assert!(!other_npc.flags.contains(CharacterFlags::ARCH));
    let texts = world.drain_pending_system_texts();
    assert!(!texts.iter().any(|text| text.message == "Well done."));
}

#[test]
fn lethal_gate_fight_hurt_class_eight_turns_killer_seyan_and_clears_turn_seyan_ppd() {
    let mut world = World::default();
    let mut opponent = login_character(CharacterId(1), &login_block("Gatekeeper"), 1, 190, 200);
    opponent.flags.remove(CharacterFlags::PLAYER);
    opponent.driver = CDR_GATE_FIGHT;
    opponent.hp = POWERSCALE;
    let mut killer = login_character(CharacterId(2), &login_block("Godmode"), 40, 191, 200);
    killer.flags.insert(CharacterFlags::ARCH);
    killer.exp = 500_000;
    world.add_character(opponent);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.gate_target_class = 8;
    player.demonshrines.push(77);
    runtime.players.insert(1, player);

    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                seyan_m:
                  name="Seyan'Du"
                  description="A Seyan'Du"
                  V_HP=10
                  V_ENDURANCE=8
                  V_MANA=6
                ;
            "#,
        )
        .unwrap();

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &loader);

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(killer.level, 1);
    assert_eq!(killer.exp, 0);
    assert!(killer.flags.contains(CharacterFlags::MAGE));
    assert!(killer.flags.contains(CharacterFlags::WARRIOR));
    assert_eq!((killer.x, killer.y), (181, 198));

    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "You are a Seyan'Du now."));

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert!(player.demonshrines.is_empty());
}
