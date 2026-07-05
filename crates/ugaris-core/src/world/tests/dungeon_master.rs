use super::*;
use crate::clan::ClanRelation;
use crate::world::dungeon_master::{DungeonEnterError, DungeonRaidError, DungeonmasterDriverData};

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER | CharacterFlags::PAID;
    player.name = name.into();
    player
}

fn found_clan(world: &mut World, name: &str) -> u16 {
    world.clan_registry.found_clan(name, 0).unwrap()
}

fn declare_war(world: &mut World, a: u16, b: u16) {
    let relations = world.clan_registry.relations_mut();
    relations.set_relation(a, b, ClanRelation::War, 0).unwrap();
    relations.set_relation(b, a, ClanRelation::War, 0).unwrap();
    relations.update(0);
}

fn give_jewels(world: &mut World, clan: u16, count: i32) {
    for _ in 0..count {
        world.clan_registry.add_jewel(clan).unwrap();
    }
}

fn member(id: u32, name: &str, world: &World, clan: u16) -> Character {
    let mut character = player(id, name);
    character.clan = clan;
    character.clan_serial = world.clan_registry.serial(clan);
    character
}

#[test]
fn create_dungeon_rejects_out_of_range_target() {
    let mut world = World::default();
    let raider = player(1, "Raider");
    assert!(world.spawn_character(raider, 10, 10));
    let dat = DungeonmasterDriverData::default();

    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), 0, &dat),
        Err(DungeonRaidError::NoSuchClan)
    );
    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), 32, &dat),
        Err(DungeonRaidError::NoSuchClan)
    );
}

#[test]
fn create_dungeon_rejects_level_too_high() {
    let mut world = World::default();
    let mut raider = player(1, "Raider");
    raider.level = 57;
    assert!(world.spawn_character(raider, 10, 10));
    let dat = DungeonmasterDriverData::default();

    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), 1, &dat),
        Err(DungeonRaidError::LevelTooHigh)
    );
}

#[test]
fn create_dungeon_rejects_when_not_at_war() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));
    let dat = DungeonmasterDriverData::default();

    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), target, &dat),
        Err(DungeonRaidError::NotAtWar)
    );
}

#[test]
fn create_dungeon_god_bypasses_war_requirement_but_not_jewel_checks() {
    let mut world = World::default();
    let target = found_clan(&mut world, "Defenders");
    let mut raider = player(1, "God");
    raider.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(raider, 10, 10));
    let dat = DungeonmasterDriverData::default();

    // No clan at all (own_clan == 0), still blocked on jewels rather
    // than "not at war" since the GOD flag bypasses only that one check.
    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), target, &dat),
        Err(DungeonRaidError::TargetHasNoJewels)
    );
}

#[test]
fn create_dungeon_rejects_target_with_too_few_jewels() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    declare_war(&mut world, own, target);
    give_jewels(&mut world, target, 10);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));
    let dat = DungeonmasterDriverData::default();

    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), target, &dat),
        Err(DungeonRaidError::TargetHasNoJewels)
    );
}

#[test]
fn create_dungeon_rejects_own_clan_with_too_few_jewels() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    declare_war(&mut world, own, target);
    give_jewels(&mut world, target, 11);
    give_jewels(&mut world, own, 11);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));
    let dat = DungeonmasterDriverData::default();

    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), target, &dat),
        Err(DungeonRaidError::OwnClanLacksJewels)
    );
}

#[test]
fn create_dungeon_rejects_when_target_catacomb_already_exists() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    declare_war(&mut world, own, target);
    give_jewels(&mut world, target, 11);
    give_jewels(&mut world, own, 12);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));

    let mut dat = DungeonmasterDriverData::default();
    dat.target[2] = target;

    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), target, &dat),
        Err(DungeonRaidError::CatacombAlreadyExists { slot: 3 })
    );
}

#[test]
fn create_dungeon_rejects_second_raid_from_same_clan() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target_a = found_clan(&mut world, "DefendersA");
    let target_b = found_clan(&mut world, "DefendersB");
    declare_war(&mut world, own, target_a);
    declare_war(&mut world, own, target_b);
    give_jewels(&mut world, target_a, 11);
    give_jewels(&mut world, target_b, 11);
    give_jewels(&mut world, own, 12);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));

    let mut dat = DungeonmasterDriverData::default();
    dat.created_by_clan[4] = own;

    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), target_b, &dat),
        Err(DungeonRaidError::ClanAlreadyRaiding)
    );
}

#[test]
fn create_dungeon_rejects_second_raid_from_same_player() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    declare_war(&mut world, own, target);
    give_jewels(&mut world, target, 11);
    give_jewels(&mut world, own, 12);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));

    let mut dat = DungeonmasterDriverData::default();
    dat.owner[5] = 1;

    assert_eq!(
        world.plan_create_dungeon(CharacterId(1), target, &dat),
        Err(DungeonRaidError::PlayerAlreadyRaiding)
    );
}

#[test]
fn create_dungeon_rejects_when_all_catacombs_are_busy() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    declare_war(&mut world, own, target);
    give_jewels(&mut world, target, 11);
    give_jewels(&mut world, own, 12);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));
    world.tick = Tick(100);

    // Every slot "created" a moment ago: nowhere near
    // `settings.dungeon_time` old yet, so all 9 are still busy.
    let mut dat = DungeonmasterDriverData::default();
    for created in dat.created.iter_mut() {
        *created = 100;
    }

    let result = world.plan_create_dungeon(CharacterId(1), target, &dat);
    assert!(matches!(
        result,
        Err(DungeonRaidError::AllCatacombsBusy { .. })
    ));
}

#[test]
fn create_dungeon_succeeds_and_selects_least_recently_used_slot() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    declare_war(&mut world, own, target);
    give_jewels(&mut world, target, 11);
    give_jewels(&mut world, own, 12);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));

    world
        .clan_registry
        .set_clan_dungeon_use(target, 1, 3)
        .unwrap(); // warrior tier 0 count
    world
        .clan_registry
        .set_clan_dungeon_use(target, 19, 2)
        .unwrap(); // teleport count
    world
        .clan_registry
        .set_clan_dungeon_use(target, 20, 1)
        .unwrap(); // fake wall
    world
        .clan_registry
        .set_clan_dungeon_use(target, 21, 2)
        .unwrap(); // two keys

    // All slots empty -> every slot's "how long has it been idle" reads
    // as `dungeon_time` exactly (C's `max(get_dungeon_time(), ticker -
    // 0)`), so the very first (index 0) slot with the max value wins.
    let dat = DungeonmasterDriverData::default();

    let plan = world
        .plan_create_dungeon(CharacterId(1), target, &dat)
        .expect("should succeed");
    assert_eq!(plan.slot, 0);
    assert_eq!(plan.fee, 3500);
    assert_eq!(plan.own_clan, own);
    assert_eq!(plan.level, 56); // training_score defaults to 0
    assert_eq!(plan.xoff, 2);
    assert_eq!(plan.yoff, 2);
    assert_eq!(plan.warrior[0], 3);
    assert_eq!(plan.teleport, 2);
    assert_eq!(plan.fake, 1);
    assert_eq!(plan.key, 2);
}

#[test]
fn create_dungeon_evicts_the_oldest_occupied_slot_when_all_are_expired() {
    // C's slot-selection floor (`max(get_dungeon_time(), ticker -
    // created)` for an *empty* slot) always dominates any occupied
    // slot's raw `ticker - created` unless every single slot is
    // occupied - only then does the oldest (smallest `created`)
    // occupied slot become the eviction target.
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    declare_war(&mut world, own, target);
    give_jewels(&mut world, target, 11);
    give_jewels(&mut world, own, 12);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));

    let dungeon_time = i64::from(world.settings.dungeon_time);
    world.tick = Tick((dungeon_time * 3) as u64);

    let mut dat = DungeonmasterDriverData::default();
    // Every slot occupied (nonzero `created`), slot 4 created longest
    // ago (tick 1) - every other slot only `dungeon_time` ticks old
    // (still "fresh", below the busy threshold).
    for created in dat.created.iter_mut() {
        *created = dungeon_time as u64;
    }
    dat.created[4] = 1;

    let plan = world
        .plan_create_dungeon(CharacterId(1), target, &dat)
        .expect("should succeed");
    assert_eq!(plan.slot, 4);
}

#[test]
fn enter_dungeon_rejects_out_of_bounds_target() {
    let mut world = World::default();
    let raider = player(1, "Raider");
    assert!(world.spawn_character(raider, 10, 10));
    let dat = DungeonmasterDriverData::default();

    assert_eq!(
        world.plan_enter_dungeon(CharacterId(1), 0, &dat),
        Err(DungeonEnterError::TargetOutOfBounds)
    );
    assert_eq!(
        world.plan_enter_dungeon(CharacterId(1), 10, &dat),
        Err(DungeonEnterError::TargetOutOfBounds)
    );
}

#[test]
fn enter_dungeon_rejects_level_too_high() {
    let mut world = World::default();
    let mut raider = player(1, "Raider");
    raider.level = 57;
    assert!(world.spawn_character(raider, 10, 10));
    let mut dat = DungeonmasterDriverData::default();
    dat.level[0] = 80;

    assert_eq!(
        world.plan_enter_dungeon(CharacterId(1), 1, &dat),
        Err(DungeonEnterError::LevelTooHigh { max_level: 80 })
    );
}

#[test]
fn enter_dungeon_rejects_when_not_at_war_with_slot_owner() {
    let mut world = World::default();
    let target = found_clan(&mut world, "Defenders");
    let raider = player(1, "Raider");
    assert!(world.spawn_character(raider, 10, 10));
    let mut dat = DungeonmasterDriverData::default();
    dat.target[0] = target;

    assert_eq!(
        world.plan_enter_dungeon(CharacterId(1), 1, &dat),
        Err(DungeonEnterError::NotAtWar)
    );
}

#[test]
fn enter_dungeon_rejects_when_about_to_collapse() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    declare_war(&mut world, own, target);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));

    let dungeon_time = i64::from(world.settings.dungeon_time);
    world.tick = Tick(dungeon_time as u64);

    let mut dat = DungeonmasterDriverData::default();
    dat.target[0] = target;
    dat.created[0] = 0; // created exactly `dungeon_time` ago -> tmp == 0

    assert_eq!(
        world.plan_enter_dungeon(CharacterId(1), 1, &dat),
        Err(DungeonEnterError::AboutToCollapse)
    );
}

#[test]
fn enter_dungeon_succeeds_and_computes_slot_coordinates() {
    let mut world = World::default();
    let own = found_clan(&mut world, "Attackers");
    let target = found_clan(&mut world, "Defenders");
    declare_war(&mut world, own, target);
    let raider = member(1, "Raider", &world, own);
    assert!(world.spawn_character(raider, 10, 10));

    let mut dat = DungeonmasterDriverData::default();
    // Slot 5 (index 4): x = (4%3)*81+4 = 85, y = (4/3)*81+80 = 161.
    dat.target[4] = target;
    dat.created[4] = 0;
    world.tick = Tick(0);

    let plan = world
        .plan_enter_dungeon(CharacterId(1), 5, &dat)
        .expect("should succeed");
    assert_eq!(plan.x, 85);
    assert_eq!(plan.y, 161);
    assert_eq!(plan.remaining_ticks, i64::from(world.settings.dungeon_time));
}

#[test]
fn list_dungeon_lines_reports_no_catacombs_when_empty() {
    let world = World::default();
    let dat = DungeonmasterDriverData::default();
    assert_eq!(
        world.list_dungeon_lines(&dat),
        vec!["No catacombs.".to_string()]
    );
}

#[test]
fn list_dungeon_lines_formats_each_occupied_slot() {
    let mut world = World::default();
    world.tick = Tick(0);
    let mut dat = DungeonmasterDriverData::default();
    dat.target[0] = 7;
    dat.level[0] = 80;
    dat.created[0] = 0;

    let lines = world.list_dungeon_lines(&dat);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("Catacomb 1: Clan 7, level 80, remaining time:"));
}

#[test]
fn characters_in_dungeon_slot_filters_by_area_block_and_player_flag() {
    let mut world = World::default();
    let mut inside = player(1, "Inside");
    inside.x = 4;
    inside.y = 4; // slot 0's 81x81 block
    assert!(world.spawn_character(inside, 4, 4));

    let mut outside = player(2, "Outside");
    outside.x = 85; // slot 1's block
    outside.y = 4;
    assert!(world.spawn_character(outside, 85, 4));

    let mut npc = character(3);
    npc.x = 4;
    npc.y = 4;
    assert!(world.spawn_character(npc, 4, 4));

    let found = world.characters_in_dungeon_slot(0);
    assert_eq!(found, vec![CharacterId(1)]);
}
