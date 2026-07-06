use super::*;
use crate::character_driver::{CDR_DUNGEONMASTER, NTID_DUNGEON};
use crate::clan::ClanRelation;
use crate::world::dungeon_master::{DungeonEnterError, DungeonRaidError, DungeonmasterDriverData};

fn dungeonmaster_npc(id: u32) -> Character {
    let mut dungeonmaster = character(id);
    dungeonmaster.name = "Dungeonmaster".into();
    dungeonmaster.driver = CDR_DUNGEONMASTER;
    dungeonmaster.driver_state = Some(CharacterDriverState::Dungeonmaster(
        DungeonmasterDriverData::default(),
    ));
    dungeonmaster
}

fn dungeonmaster_data(world: &World, dungeonmaster_id: CharacterId) -> DungeonmasterDriverData {
    match world
        .characters
        .get(&dungeonmaster_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Dungeonmaster(data)) => data,
        _ => panic!("expected Dungeonmaster driver state"),
    }
}

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
fn build_remove_tile_evicts_non_player_character_outright() {
    let mut world = World::default();
    let mut npc = character(9);
    npc.x = 10;
    npc.y = 10;
    assert!(world.spawn_character(npc, 10, 10));

    world.build_remove_tile(10, 10);

    assert!(!world.characters.contains_key(&CharacterId(9)));
    assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
}

#[test]
fn build_remove_tile_teleports_player_to_the_safe_zone_and_warns_them() {
    let mut world = World::default();
    let mut raider = player(1, "Raider");
    raider.x = 10;
    raider.y = 10;
    assert!(world.spawn_character(raider, 10, 10));

    world.build_remove_tile(10, 10);

    let moved = &world.characters[&CharacterId(1)];
    assert_eq!((moved.x, moved.y), (245, 250));
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(1),
            message: "The catacomb collapsed on you.".to_string(),
        }]
    );
}

#[test]
fn build_remove_tile_falls_back_to_rest_point_in_the_same_area_when_the_safe_zone_is_blocked() {
    let mut world = World::default();
    world.area_id = 13;
    let mut raider = player(1, "Raider");
    raider.x = 10;
    raider.y = 10;
    raider.rest_area = 13;
    raider.rest_x = 50;
    raider.rest_y = 60;
    assert!(world.spawn_character(raider, 10, 10));

    // Block every one of `build_remove`'s four candidate safe-zone tiles
    // (and their `drop_char`-style neighbor offsets) with walls so all
    // four `teleport_char_driver` attempts fail, forcing the
    // `change_area` fallback.
    for (x, y) in [(245, 250), (240, 250), (235, 250), (230, 250)] {
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                let tx = (x as i32 + dx) as usize;
                let ty = (y as i32 + dy) as usize;
                world.map.tile_mut(tx, ty).unwrap().flags |= MapFlags::MOVEBLOCK;
            }
        }
    }

    world.build_remove_tile(10, 10);

    let moved = &world.characters[&CharacterId(1)];
    assert_eq!((moved.x, moved.y), (50, 60));
}

#[test]
fn build_remove_tile_queues_a_cross_area_transfer_when_the_rest_point_is_in_a_different_area() {
    let mut world = World::default();
    world.area_id = 13;
    let mut raider = player(1, "Raider");
    raider.x = 10;
    raider.y = 10;
    raider.rest_area = 3; // a different area - queued for cross-area hand-off
    raider.rest_x = 50;
    raider.rest_y = 60;
    assert!(world.spawn_character(raider, 10, 10));

    for (x, y) in [(245, 250), (240, 250), (235, 250), (230, 250)] {
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                let tx = (x as i32 + dx) as usize;
                let ty = (y as i32 + dy) as usize;
                world.map.tile_mut(tx, ty).unwrap().flags |= MapFlags::MOVEBLOCK;
            }
        }
    }

    world.build_remove_tile(10, 10);

    // Not removed outright yet - the transfer is deferred to
    // `ugaris-server`'s `apply_dungeon_eviction_transfers`, which only
    // calls `World::remove_character` if the hand-off itself fails.
    assert!(world.characters.contains_key(&CharacterId(1)));
    let transfers = world.drain_pending_dungeon_eviction_transfers();
    assert_eq!(
        transfers,
        vec![DungeonEvictionTransfer {
            character_id: CharacterId(1),
            target_area: 3,
            target_x: 50,
            target_y: 60,
        }]
    );
}

#[test]
fn build_remove_tile_destroys_a_plain_takeable_item() {
    let mut world = World::default();
    let mut sword = item(5, ItemFlags::TAKE);
    assert!(world.map.set_item_map(&mut sword, 10, 10));
    world.add_item(sword);

    world.build_remove_tile(10, 10);

    assert!(!world.items.contains_key(&ItemId(5)));
    assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
}

#[test]
fn build_remove_tile_scatters_a_player_body_nearby_and_arms_its_decay_timer() {
    let mut world = World::default();
    let mut body = item(6, ItemFlags::PLAYERBODY);
    assert!(world.map.set_item_map(&mut body, 10, 10));
    world.add_item(body);

    world.build_remove_tile(10, 10);

    // The body must have moved off the original tile (dropped near
    // (250, 245)) rather than staying at (10, 10) or being destroyed.
    let moved = &world.items[&ItemId(6)];
    assert_ne!((moved.x, moved.y), (10, 10));
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn build_remove_tile_destroys_a_player_body_when_no_space_is_available() {
    let mut world = World::default();
    let mut body = item(6, ItemFlags::PLAYERBODY);
    assert!(world.map.set_item_map(&mut body, 10, 10));
    world.add_item(body);

    // Block every drop-offset candidate around all four scatter origins
    // so none of `drop_item`'s attempts can place the body anywhere.
    for (x, y) in [(250, 245), (250, 240), (250, 235), (250, 230)] {
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                let tx = (x as i32 + dx) as usize;
                let ty = (y as i32 + dy) as usize;
                world.map.tile_mut(tx, ty).unwrap().flags |= MapFlags::MOVEBLOCK;
            }
        }
    }

    world.build_remove_tile(10, 10);

    assert!(!world.items.contains_key(&ItemId(6)));
}

#[test]
fn build_remove_tile_removes_every_effect_anchored_to_the_tile() {
    let mut world = World::default();
    let index = 10 + 10 * world.map.width();
    let mut effect = Effect::new(1, 1, 0, 100);
    effect.fields.push(index as i32);
    world.effects.insert(42, effect);
    world.map.tile_mut(10, 10).unwrap().effects[0] = 42;

    world.build_remove_tile(10, 10);

    assert!(!world.effects.contains_key(&42));
    assert_eq!(world.map.tile(10, 10).unwrap().effects, [0; 4]);
}

#[test]
fn build_empty_tile_destroys_any_remaining_item_and_resets_the_tile() {
    let mut world = World::default();
    let mut junk = item(5, ItemFlags::TAKE);
    assert!(world.map.set_item_map(&mut junk, 10, 11));
    world.add_item(junk);
    {
        let tile = world.map.tile_mut(10, 11).unwrap();
        tile.flags |= MapFlags::MOVEBLOCK;
        tile.foreground_sprite = 12345;
        tile.daylight = 5;
        tile.light = 5;
    }

    world.build_empty_tile(10, 11);

    assert!(!world.items.contains_key(&ItemId(5)));
    let tile = world.map.tile(10, 11).unwrap();
    assert_eq!(tile.flags, MapFlags::INDOORS);
    assert_eq!(tile.foreground_sprite, 0);
    assert_eq!(tile.ground_sprite, 59130 + 10 % 3 + (11 % 3) * 3);
    assert_eq!(tile.daylight, 0);
    assert_eq!(tile.light, 0);
}

#[test]
fn destroy_dungeon_tears_down_only_the_given_slots_own_81x81_block() {
    let mut world = World::default();

    // Slot 4 (index 4, the center slot): xoff = (4%3)*81+2 = 83, yoff =
    // (4/3)*81+2 = 83, so the block spans x,y in [83, 163).
    let mut inside = character(1);
    inside.x = 100;
    inside.y = 100;
    assert!(world.spawn_character(inside, 100, 100));

    let mut outside = character(2);
    outside.x = 10;
    outside.y = 10;
    assert!(world.spawn_character(outside, 10, 10));
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    world.destroy_dungeon(4);

    assert!(!world.characters.contains_key(&CharacterId(1)));
    assert!(world.characters.contains_key(&CharacterId(2)));
    let inside_tile = world.map.tile(100, 100).unwrap();
    assert_eq!(inside_tile.flags, MapFlags::INDOORS);
    let outside_tile = world.map.tile(10, 10).unwrap();
    assert!(outside_tile.flags.contains(MapFlags::MOVEBLOCK));
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

fn light_tile(world: &mut World, x: usize, y: usize) {
    world.map.tile_mut(x, y).unwrap().light = 255;
}

// C `dungeonmaster`'s `NT_CHAR` greeting branch (`dungeon.c:1597-1620`).
#[test]
fn npc_char_message_greets_visible_nearby_player_once() {
    let mut world = World::default();
    light_tile(&mut world, 10, 10);
    light_tile(&mut world, 15, 10);
    assert!(world.spawn_character(dungeonmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 15, 10));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        dungeonmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_dungeonmaster_actions();

    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains(
        "Hello Godmode! Welcome to the clan catacombs. Be warned, there is a fee of 3500 \
         gold for attacking now. Say help for details."
    ));

    // Same speaker again: the driver-memory slot suppresses the repeat.
    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        dungeonmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_dungeonmaster_actions();
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn npc_char_message_ignores_speaker_beyond_ten_tiles() {
    let mut world = World::default();
    light_tile(&mut world, 10, 10);
    light_tile(&mut world, 25, 10);
    assert!(world.spawn_character(dungeonmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Faraway"), 25, 10));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        dungeonmaster.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_dungeonmaster_actions();

    assert!(world.drain_pending_area_texts().is_empty());
}

// C `dungeonmaster`'s `NT_TEXT` help/list small-talk (`dungeon.c:1636-
// 1646`).
#[test]
fn text_help_and_list_reply_with_exact_c_wording() {
    let mut world = World::default();
    assert!(world.spawn_character(dungeonmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        dungeonmaster.push_driver_text_message(CharacterId(2), "help");
    }
    world.process_dungeonmaster_actions();
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains(
        "Use: 'attack <nr>' to attack clan <nr>, 'enter <nr>' to enter catacomb <nr> or 'list' \
         to get a listing of all catacombs."
    )));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        dungeonmaster.push_driver_text_message(CharacterId(2), "list");
    }
    world.process_dungeonmaster_actions();
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains("No catacombs.")));
}

// C `dungeonmaster`'s `attack` handler, calling into `create_dungeon`
// (`dungeon.c:1648`).
#[test]
fn attack_command_success_charges_fee_updates_slot_and_queues_build_request() {
    let mut world = World::default();
    assert!(world.spawn_character(dungeonmaster_npc(1), 10, 10));

    let own_clan = found_clan(&mut world, "Raiders");
    let target_clan = found_clan(&mut world, "Targets");
    declare_war(&mut world, own_clan, target_clan);
    give_jewels(&mut world, target_clan, 11);
    give_jewels(&mut world, own_clan, 12);

    let mut raider = member(2, "Raider", &world, own_clan);
    raider.gold = 10_000_000;
    assert!(world.spawn_character(raider, 10, 10));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        dungeonmaster.push_driver_text_message(CharacterId(2), &format!("attack {target_clan}"));
    }
    world.process_dungeonmaster_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t.message.contains(
        "Very well, I have created the catacomb for you. Thank you for paying 3500 gold."
    )));

    let raider_gold = world.characters.get(&CharacterId(2)).unwrap().gold;
    assert_eq!(raider_gold, 10_000_000 - 350_000);

    let dat = dungeonmaster_data(&world, CharacterId(1));
    assert_eq!(dat.target[0], target_clan);
    assert_eq!(dat.created_by_clan[0], own_clan);
    assert_eq!(dat.owner[0], 2);

    let builds = world.drain_pending_dungeon_raid_builds();
    assert_eq!(builds.len(), 1);
    assert_eq!(builds[0].target_clan, target_clan);
    assert_eq!(builds[0].own_clan, own_clan);
    assert_eq!(builds[0].player_id, CharacterId(2));
    assert_eq!(builds[0].dungeonmaster_id, CharacterId(1));
    assert_eq!(builds[0].slot, 0);
}

#[test]
fn attack_command_failure_says_error_and_never_charges_or_queues() {
    let mut world = World::default();
    assert!(world.spawn_character(dungeonmaster_npc(1), 10, 10));

    let mut raider = player(2, "Raider");
    raider.gold = 10_000_000;
    assert!(world.spawn_character(raider, 10, 10));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        dungeonmaster.push_driver_text_message(CharacterId(2), "attack 5");
    }
    world.process_dungeonmaster_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You are not at war with that clan.")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().gold,
        10_000_000
    );
    assert!(world.drain_pending_dungeon_raid_builds().is_empty());
}

#[test]
fn attack_command_says_cannot_afford_fee_without_charging_when_gold_is_short() {
    let mut world = World::default();
    assert!(world.spawn_character(dungeonmaster_npc(1), 10, 10));

    let own_clan = found_clan(&mut world, "Raiders");
    let target_clan = found_clan(&mut world, "Targets");
    declare_war(&mut world, own_clan, target_clan);
    give_jewels(&mut world, target_clan, 11);
    give_jewels(&mut world, own_clan, 12);

    let mut raider = member(2, "Raider", &world, own_clan);
    raider.gold = 1;
    assert!(world.spawn_character(raider, 10, 10));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        dungeonmaster.push_driver_text_message(CharacterId(2), &format!("attack {target_clan}"));
    }
    world.process_dungeonmaster_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|t| t
        .message
        .contains("Sorry, you cannot afford the fee of 3500G.")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 1);
    assert!(world.drain_pending_dungeon_raid_builds().is_empty());
    let dat = dungeonmaster_data(&world, CharacterId(1));
    assert_eq!(dat.target[0], 0);
}

// C `dungeonmaster`'s `enter` handler, calling into `enter_dungeon`
// (`dungeon.c:1655`).
#[test]
fn enter_command_success_teleports_and_says_collapse_time() {
    let mut world = World::default();
    assert!(world.spawn_character(dungeonmaster_npc(1), 250, 250));

    let own_clan = found_clan(&mut world, "Raiders");
    let target_clan = found_clan(&mut world, "Targets");
    declare_war(&mut world, own_clan, target_clan);
    let raider = member(2, "Raider", &world, own_clan);
    assert!(world.spawn_character(raider, 250, 250));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        if let Some(CharacterDriverState::Dungeonmaster(data)) = dungeonmaster.driver_state.as_mut()
        {
            data.target[0] = target_clan;
            data.level[0] = 30;
            data.created[0] = 0;
        }
        dungeonmaster.push_driver_text_message(CharacterId(2), "enter 1");
    }
    world.process_dungeonmaster_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("This catacomb will collapse in")));
    let raider = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((raider.x, raider.y), (4, 80));
}

#[test]
fn enter_command_out_of_bounds_says_exact_c_message() {
    let mut world = World::default();
    assert!(world.spawn_character(dungeonmaster_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Raider"), 10, 10));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        dungeonmaster.push_driver_text_message(CharacterId(2), "enter 99");
    }
    world.process_dungeonmaster_actions();

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("Sorry, the target is out of bounds.")));
}

// C `dungeonmaster`'s GM-only `destroy` handler (`dungeon.c:1657-1668`).
#[test]
fn destroy_command_requires_god_flag() {
    let mut world = World::default();
    assert!(world.spawn_character(dungeonmaster_npc(1), 250, 250));
    let mut mortal = player(2, "Mortal");
    mortal.flags.remove(CharacterFlags::GOD);
    assert!(world.spawn_character(mortal, 250, 250));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        if let Some(CharacterDriverState::Dungeonmaster(data)) = dungeonmaster.driver_state.as_mut()
        {
            data.target[0] = 5;
        }
        dungeonmaster.push_driver_text_message(CharacterId(2), "destroy 1");
    }
    world.process_dungeonmaster_actions();

    let dat = dungeonmaster_data(&world, CharacterId(1));
    assert_eq!(
        dat.target[0], 5,
        "non-GOD speaker must not destroy a catacomb"
    );
}

#[test]
fn destroy_command_resets_slot_for_god_speaker() {
    let mut world = World::default();
    assert!(world.spawn_character(dungeonmaster_npc(1), 250, 250));
    let mut god = player(2, "Godmode");
    god.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(god, 250, 250));

    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        if let Some(CharacterDriverState::Dungeonmaster(data)) = dungeonmaster.driver_state.as_mut()
        {
            data.target[0] = 5;
            data.created_by_clan[0] = 7;
        }
        dungeonmaster.push_driver_text_message(CharacterId(2), "destroy 1");
    }
    world.process_dungeonmaster_actions();

    let dat = dungeonmaster_data(&world, CharacterId(1));
    assert_eq!(dat.target[0], 0);
    assert_eq!(dat.created_by_clan[0], 0);
}

// C `dungeonmaster`'s per-slot expiry tick (`dungeon.c:1706-1718`).
#[test]
fn tick_destroys_expired_slot_and_resets_tracking_fields() {
    let mut world = World::default();
    assert!(world.spawn_character(dungeonmaster_npc(1), 250, 250));
    if let Some(dungeonmaster) = world.characters.get_mut(&CharacterId(1)) {
        if let Some(CharacterDriverState::Dungeonmaster(data)) = dungeonmaster.driver_state.as_mut()
        {
            data.target[0] = 5;
            data.created_by_clan[0] = 7;
            data.owner[0] = 2;
            data.created[0] = 1;
        }
    }
    // Past the default dungeon-collapse window.
    world.tick = Tick(world.settings.dungeon_time as u64 + 2);

    world.process_dungeonmaster_actions();

    let dat = dungeonmaster_data(&world, CharacterId(1));
    assert_eq!(dat.target[0], 0);
    assert_eq!(dat.created_by_clan[0], 0);
    assert_eq!(dat.owner[0], 0);
    assert_eq!(dat.created[0], 0);
}

// C `dungeondoor`'s `first_solve` block (`area/13/dungeon.c:1855-1891`).
#[test]
fn resolve_dungeon_door_first_solve_steals_jewels_and_notifies_slot_and_dungeonmaster() {
    let mut world = World::default();
    let attacker_clan = found_clan(&mut world, "Attacker");
    let defender_clan = found_clan(&mut world, "Defender");
    give_jewels(&mut world, attacker_clan, 12);
    give_jewels(&mut world, defender_clan, 14);

    let mut winner = member(1, "Winner", &world, attacker_clan);
    winner.x = 10;
    winner.y = 10;
    assert!(world.spawn_character(winner, 10, 10));
    let mut bystander = player(2, "Bystander");
    bystander.x = 20;
    bystander.y = 20;
    assert!(world.spawn_character(bystander, 20, 20));
    assert!(world.spawn_character(dungeonmaster_npc(3), 250, 250));
    world.tick = Tick(777);

    world.resolve_dungeon_door_first_solve(CharacterId(1), u32::from(defender_clan), 0);

    // `cnt = min(cnt_jewels(cnr)-11, 3) = min(3,3) = 3`.
    assert_eq!(
        world
            .clan_registry
            .identity(defender_clan)
            .unwrap()
            .economy
            .training_score,
        150
    );
    assert_eq!(
        world
            .clan_registry
            .identity(defender_clan)
            .unwrap()
            .economy
            .treasure
            .debt,
        4000
    );
    assert_eq!(world.clan_registry.jewel_count(attacker_clan), 15);

    let texts = world.drain_pending_system_texts();
    assert!(texts.contains(&WorldSystemText {
        character_id: CharacterId(1),
        message: "You won. You stole 3 jewels for your clan's storage.".to_string(),
    }));
    assert!(texts.contains(&WorldSystemText {
        character_id: CharacterId(1),
        message: "This catacomb has been solved and will collapse.".to_string(),
    }));
    assert!(texts.contains(&WorldSystemText {
        character_id: CharacterId(2),
        message: "This catacomb has been solved and will collapse.".to_string(),
    }));

    let dungeonmaster = &world.characters[&CharacterId(3)];
    assert_eq!(dungeonmaster.driver_messages.len(), 1);
    assert_eq!(dungeonmaster.driver_messages[0].message_type, NT_NPC);
    assert_eq!(dungeonmaster.driver_messages[0].dat1, NTID_DUNGEON);
    assert_eq!(dungeonmaster.driver_messages[0].dat2, 0);
    assert_eq!(dungeonmaster.driver_messages[0].dat3, 777);

    let events = world.drain_pending_dungeon_jewel_steals();
    assert_eq!(
        events,
        vec![DungeonJewelStealEvent {
            player_id: CharacterId(1),
            defender_clan,
            attacker_clan,
            stolen: 3,
        }]
    );
}

#[test]
fn resolve_dungeon_door_first_solve_reports_nothing_left_to_steal_but_still_broadcasts() {
    let mut world = World::default();
    let attacker_clan = found_clan(&mut world, "Attacker");
    let defender_clan = found_clan(&mut world, "Defender");
    give_jewels(&mut world, attacker_clan, 12);
    give_jewels(&mut world, defender_clan, 11); // cnt = min(11-11,3) = 0

    let mut winner = member(1, "Winner", &world, attacker_clan);
    winner.x = 10;
    winner.y = 10;
    assert!(world.spawn_character(winner, 10, 10));
    assert!(world.spawn_character(dungeonmaster_npc(3), 250, 250));

    world.resolve_dungeon_door_first_solve(CharacterId(1), u32::from(defender_clan), 0);

    assert_eq!(
        world
            .clan_registry
            .identity(defender_clan)
            .unwrap()
            .economy
            .training_score,
        0
    );
    assert_eq!(world.clan_registry.jewel_count(attacker_clan), 12);

    let texts = world.drain_pending_system_texts();
    assert!(texts.contains(&WorldSystemText {
        character_id: CharacterId(1),
        message: "You won. Unfortunately there's nothing left to steal.".to_string(),
    }));
    assert!(texts.contains(&WorldSystemText {
        character_id: CharacterId(1),
        message: "This catacomb has been solved and will collapse.".to_string(),
    }));
    // No jewels moved, so no clan-log event is queued (matches C's
    // `add_clanlog` calls living inside the `if (cnt > 0)` block only).
    assert!(world.drain_pending_dungeon_jewel_steals().is_empty());
    assert_eq!(world.characters[&CharacterId(3)].driver_messages.len(), 1);
}

#[test]
fn resolve_dungeon_door_first_solve_rejects_a_non_clan_member_without_broadcast() {
    let mut world = World::default();
    let defender_clan = found_clan(&mut world, "Defender");
    give_jewels(&mut world, defender_clan, 14);

    let mut winner = player(1, "Winner");
    winner.x = 10;
    winner.y = 10;
    assert!(world.spawn_character(winner, 10, 10));
    assert!(world.spawn_character(dungeonmaster_npc(3), 250, 250));

    world.resolve_dungeon_door_first_solve(CharacterId(1), u32::from(defender_clan), 0);

    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(1),
            message: "You're not supposed to be here.".to_string(),
        }]
    );
    assert!(world.drain_pending_dungeon_jewel_steals().is_empty());
    // C returns before reaching the notify loop (`dungeon.c:1857-1859`).
    assert!(world.characters[&CharacterId(3)].driver_messages.is_empty());
}

#[test]
fn resolve_dungeon_door_first_solve_rejects_own_clan_with_too_few_jewels_without_broadcast() {
    let mut world = World::default();
    let attacker_clan = found_clan(&mut world, "Attacker");
    let defender_clan = found_clan(&mut world, "Defender");
    give_jewels(&mut world, attacker_clan, 11); // below the 12 threshold
    give_jewels(&mut world, defender_clan, 14);

    let mut winner = member(1, "Winner", &world, attacker_clan);
    winner.x = 10;
    winner.y = 10;
    assert!(world.spawn_character(winner, 10, 10));
    assert!(world.spawn_character(dungeonmaster_npc(3), 250, 250));

    world.resolve_dungeon_door_first_solve(CharacterId(1), u32::from(defender_clan), 0);

    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(1),
            message: "You can't steal jewels while your own clan has less than 12 of them."
                .to_string(),
        }]
    );
    assert_eq!(world.clan_registry.jewel_count(defender_clan), 14);
    assert!(world.drain_pending_dungeon_jewel_steals().is_empty());
    assert!(world.characters[&CharacterId(3)].driver_messages.is_empty());
}
