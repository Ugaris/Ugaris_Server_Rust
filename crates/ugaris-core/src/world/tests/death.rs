use super::*;

fn lethal_hurt(world: &mut World, target: u32, killer: u32) {
    world
        .apply_legacy_hurt(
            CharacterId(target),
            Some(CharacterId(killer)),
            1_000 * POWERSCALE,
            1,
            0,
            0,
        )
        .unwrap();
}

fn run_death_animation(world: &mut World) {
    for _ in 0..DEATH_ANIMATION_TICKS + 1 {
        world.tick = Tick(world.tick.0 + 1);
        world.tick_basic_actions();
    }
}

#[test]
fn kill_sets_legacy_death_action_and_killer_metadata() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = POWERSCALE;
    assert!(world.spawn_character(target, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);

    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert!(target.flags.contains(CharacterFlags::DEAD));
    assert_eq!(target.action, action::DIE);
    assert_eq!(target.act1, 2);
    assert_eq!(target.act2, 1, "player kills mark the C ispk flag");
    assert_eq!(target.duration, DEATH_ANIMATION_TICKS);
    assert_eq!(target.step, 0);
}

#[test]
fn kill_awards_killer_experience_from_c_kill_score_table() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = POWERSCALE;
    target.level = 20;
    assert!(world.spawn_character(target, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    killer.level = 20;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);

    // C kill_score_level(20, 20) = ceil(20 * 0.75) = 15.
    assert_eq!(
        world.drain_pending_kill_exp(),
        vec![KillExpAward {
            killer_id: CharacterId(2),
            exp: 15,
        }]
    );
}

#[test]
fn kill_queues_achievement_award_for_player_killer_regardless_of_target_kind() {
    let mut world = World::default();
    world.area_id = 4;
    let mut target = character(1);
    target.hp = POWERSCALE;
    // A player kill (not just an NPC kill) should still queue the
    // achievement award: C's `achievement_add_enemy_killed`/`_demons` gate
    // only checks `ch[co].flags & CF_PLAYER`, not the target's kind.
    target.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(target, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);

    assert_eq!(
        world.drain_pending_kill_achievements(),
        vec![KillAchievementAward {
            killer_id: CharacterId(2),
            area_id: 4,
            target_is_demon: false,
        }]
    );
}

#[test]
fn kill_achievement_award_flags_demon_targets() {
    let mut world = World::default();
    world.area_id = 34;
    let mut target = character(1);
    target.hp = POWERSCALE;
    target.flags |= CharacterFlags::DEMON;
    assert!(world.spawn_character(target, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);

    assert_eq!(
        world.drain_pending_kill_achievements(),
        vec![KillAchievementAward {
            killer_id: CharacterId(2),
            area_id: 34,
            target_is_demon: true,
        }]
    );
}

#[test]
fn kill_achievement_award_is_not_queued_for_non_player_killer() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = POWERSCALE;
    assert!(world.spawn_character(target, 10, 10));
    let killer = character(2); // no CF_PLAYER flag.
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);

    assert!(world.drain_pending_kill_achievements().is_empty());
}

#[test]
fn kill_queues_first_kill_check_for_player_killer_when_victim_class_is_set() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = POWERSCALE;
    target.level = 42;
    target.class = 258; // demon lord range.
    target.name = "Demon Lord".to_string();
    assert!(world.spawn_character(target, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);

    assert_eq!(
        world.drain_pending_first_kill_checks(),
        vec![FirstKillCheck {
            killer_id: CharacterId(2),
            victim_class: 258,
            victim_level: 42,
            victim_has_name: false,
            victim_name: "Demon Lord".to_string(),
        }]
    );
}

#[test]
fn kill_does_not_queue_first_kill_check_when_victim_class_is_unset() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = POWERSCALE;
    // `class` defaults to 0, matching C's `ch.class < 1` no-op guard.
    assert!(world.spawn_character(target, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);

    assert!(world.drain_pending_first_kill_checks().is_empty());
}

#[test]
fn kill_does_not_queue_first_kill_check_for_non_player_killer() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = POWERSCALE;
    target.class = 60;
    assert!(world.spawn_character(target, 10, 10));
    let killer = character(2); // no CF_PLAYER flag.
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);

    assert!(world.drain_pending_first_kill_checks().is_empty());
}

#[test]
fn kill_score_level_matches_c_taper_table() {
    assert_eq!(crate::attack::kill_score_level(20, 0), 20);
    assert_eq!(crate::attack::kill_score_level(20, 15), 20);
    assert_eq!(crate::attack::kill_score_level(20, 16), 19);
    assert_eq!(crate::attack::kill_score_level(20, 19), 16);
    assert_eq!(crate::attack::kill_score_level(20, 20), 15);
    assert_eq!(crate::attack::kill_score_level(20, 21), 14);
    assert_eq!(crate::attack::kill_score_level(20, 24), 8);
    assert_eq!(crate::attack::kill_score_level(20, 25), 4);
    assert_eq!(crate::attack::kill_score_level(20, 26), 0);
    assert_eq!(crate::attack::kill_score_level(0, 0), 1);
    // Levels cap at 118 before the diff calculation.
    assert_eq!(crate::attack::kill_score_level(150, 150), 89);
}

#[test]
fn npc_death_drops_lootable_body_with_inventory_and_money() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = POWERSCALE;
    npc.sprite = 7;
    npc.dir = 1;
    npc.gold = 250;
    npc.name = "Grolm".into();
    npc.inventory[30] = Some(ItemId(900));
    assert!(world.spawn_character(npc, 10, 10));
    let mut loot = item(900, ItemFlags::TAKE);
    loot.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(900), loot);
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);
    run_death_animation(&mut world);

    assert!(
        !world.characters.contains_key(&CharacterId(1)),
        "NPCs are destroyed after the death animation"
    );
    let body = world
        .items
        .values()
        .find(|item| item.description == "Grolm's body.")
        .expect("dead body dropped");
    // C: 100000 + sprite * 1000 + (dir - 1) / 2 * 8 + 335.
    assert_eq!(body.sprite, 100_000 + 7 * 1000 + 335);
    assert!(body.flags.contains(ItemFlags::USE));
    assert_eq!((body.x, body.y), (10, 10));
    let loot = world.items.get(&ItemId(900)).unwrap();
    assert_eq!(loot.contained_in, Some(body.id));
    assert_eq!(loot.carried_by, None);
    let money = world
        .items
        .values()
        .find(|item| item.flags.contains(ItemFlags::MONEY))
        .expect("gold becomes a contained money item");
    assert_eq!(money.value, 250);
    assert_eq!(money.sprite, 104);
    assert_eq!(money.contained_in, Some(body.id));
}

#[test]
fn npc_death_without_loot_leaves_no_body() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = POWERSCALE;
    assert!(world.spawn_character(npc, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);
    run_death_animation(&mut world);

    assert!(!world.characters.contains_key(&CharacterId(1)));
    assert!(
        world.items.values().all(|item| item.name != "Body"),
        "empty NPCs leave no body like C"
    );
}

#[test]
fn nobody_npc_drops_only_given_items() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = POWERSCALE;
    npc.flags |= CharacterFlags::NOBODY;
    npc.gold = 99;
    npc.inventory[30] = Some(ItemId(900));
    npc.inventory[31] = Some(ItemId(901));
    assert!(world.spawn_character(npc, 10, 10));
    let mut given = item(900, ItemFlags::TAKE | ItemFlags::GIVEN_ITEM);
    given.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(900), given);
    let mut normal = item(901, ItemFlags::TAKE);
    normal.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(901), normal);
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);
    run_death_animation(&mut world);

    let given = world.items.get(&ItemId(900)).expect("given item dropped");
    assert_eq!(given.carried_by, None);
    assert_eq!(given.contained_in, None);
    assert_ne!((given.x, given.y), (0, 0));
    assert!(
        !world.items.contains_key(&ItemId(901)),
        "non-given NOBODY inventory is destroyed"
    );
    assert!(world.items.values().all(|item| item.name != "Body"));
}

#[test]
fn itemdeath_npc_drops_slot_thirty_item_instead_of_body() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = POWERSCALE;
    npc.flags |= CharacterFlags::ITEMDEATH;
    npc.inventory[30] = Some(ItemId(900));
    assert!(world.spawn_character(npc, 10, 10));
    let mut drop = item(900, ItemFlags::TAKE);
    drop.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(900), drop);
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);
    run_death_animation(&mut world);

    let drop = world.items.get(&ItemId(900)).expect("slot 30 item dropped");
    assert_eq!(drop.carried_by, None);
    assert_eq!((drop.x, drop.y), (10, 10));
    assert!(world.items.values().all(|item| item.name != "Body"));
}

#[test]
fn respawn_flag_schedules_template_respawn_request() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = POWERSCALE;
    npc.flags |= CharacterFlags::RESPAWN;
    npc.template_key = "grolm".into();
    npc.respawn_ticks = 48;
    npc.rest_x = 10;
    npc.rest_y = 10;
    assert!(world.spawn_character(npc, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);
    run_death_animation(&mut world);
    assert!(world.drain_pending_npc_respawns().is_empty());

    world.tick = Tick(60);
    world.process_due_timers(1);
    let requests = world.drain_pending_npc_respawns();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].template_key, "grolm");
    assert_eq!((requests[0].x, requests[0].y), (10, 10));

    // Blocked respawns retry after the legacy ten seconds.
    world.schedule_npc_respawn_retry(requests[0].slot);
    world.tick = Tick(60 + TICKS_PER_SECOND * 10);
    world.process_due_timers(1);
    assert_eq!(world.drain_pending_npc_respawns().len(), 1);
}

#[test]
fn player_death_loses_experience_and_returns_to_rest_position() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags |= CharacterFlags::PLAYER;
    player.hp = POWERSCALE;
    player.exp = 10_000;
    player.exp_used = 10_000;
    player.rest_x = 40;
    player.rest_y = 40;
    player.values[0][CharacterValue::Hp as usize] = 50;
    player.values[0][CharacterValue::Endurance as usize] = 40;
    player.values[0][CharacterValue::Mana as usize] = 30;
    // C `update_char` (now wired into `die_char`'s post-respawn recompute,
    // `src/system/death.c:807`) recomputes `value[0]` from the raised
    // `value[1]` baseline; without it, the direct `value[0]` pokes above
    // would be overwritten back to 0.
    player.values[1][CharacterValue::Hp as usize] = 50;
    player.values[1][CharacterValue::Endurance as usize] = 40;
    player.values[1][CharacterValue::Mana as usize] = 30;
    player.inventory[0] = Some(ItemId(900));
    player.inventory[12] = Some(ItemId(901));
    player.inventory[30] = Some(ItemId(902));
    player.gold = 77;
    assert!(world.spawn_character(player, 10, 10));
    for (id, flags) in [
        (900, ItemFlags::TAKE | ItemFlags::WNHEAD),
        (901, ItemFlags::empty()),
        (902, ItemFlags::TAKE),
    ] {
        let mut carried = item(id, flags);
        carried.carried_by = Some(CharacterId(1));
        world.items.insert(ItemId(id), carried);
    }
    let killer = character(2);
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);
    run_death_animation(&mut world);

    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!player.flags.contains(CharacterFlags::DEAD));
    assert!(player.flags.contains(CharacterFlags::ALIVE));
    // C death_loss: 10000 / 25 = 400, no used-exp taper because minus <= 0.
    assert_eq!(player.exp, 9_600);
    assert_eq!(player.hp, 50 * POWERSCALE);
    assert_eq!(player.endurance, 40 * POWERSCALE);
    assert_eq!(player.mana, 30 * POWERSCALE);
    assert_eq!((player.x, player.y), (40, 40), "back at the rest position");
    assert_eq!(player.gold, 0);
    assert!(
        !world.items.contains_key(&ItemId(901)),
        "spell items are destroyed on death"
    );
    let body = world
        .items
        .values()
        .find(|item| item.flags.contains(ItemFlags::PLAYERBODY))
        .expect("player body dropped");
    let contained: Vec<u32> = world
        .items
        .values()
        .filter(|item| item.contained_in == Some(body.id))
        .map(|item| item.id.0)
        .collect();
    assert!(
        contained.contains(&902),
        "inventory items land in the body container"
    );
    let feedback = world.drain_pending_system_texts();
    assert!(feedback
        .iter()
        .any(|text| text.message.contains("lost some experience points")));
}

#[test]
fn pk_death_keeps_player_experience() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags |= CharacterFlags::PLAYER;
    player.hp = POWERSCALE;
    player.exp = 10_000;
    player.rest_x = 40;
    player.rest_y = 40;
    player.values[0][CharacterValue::Hp as usize] = 50;
    assert!(world.spawn_character(player, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);
    run_death_animation(&mut world);

    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(player.exp, 10_000, "PK deaths do not lose experience");
    let feedback = world.drain_pending_system_texts();
    assert!(feedback
        .iter()
        .any(|text| text.message.contains("died by the hands of a player")));
}

#[test]
fn body_expire_timer_destroys_body_and_contents() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = POWERSCALE;
    npc.gold = 10;
    npc.name = "Grolm".into();
    assert!(world.spawn_character(npc, 10, 10));
    let mut killer = character(2);
    killer.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(killer, 11, 10));

    lethal_hurt(&mut world, 1, 2);
    run_death_animation(&mut world);

    let body_id = world
        .items
        .values()
        .find(|item| item.description == "Grolm's body.")
        .map(|item| item.id)
        .expect("body dropped");

    // C npc_body_decay_time defaults to two minutes.
    world.tick = Tick(world.tick.0 + world.settings.npc_body_decay_time as u64 + 1);
    world.process_due_timers(1);

    assert!(!world.items.contains_key(&body_id));
    assert!(
        world.items.values().all(|item| item.contained_in.is_none()),
        "contained loot expires with the body"
    );
}

#[test]
fn legacy_money_sprites_follow_c_ladder() {
    assert_eq!(legacy_money_sprite(5), 102);
    assert_eq!(legacy_money_sprite(50), 103);
    assert_eq!(legacy_money_sprite(500), 104);
    assert_eq!(legacy_money_sprite(5_000), 105);
    assert_eq!(legacy_money_sprite(50_000), 106);
    assert_eq!(legacy_money_sprite(500_000), 107);
    assert_eq!(legacy_money_sprite(5_000_000), 108);
    assert_eq!(legacy_money_sprite(50_000_000), 109);
}
