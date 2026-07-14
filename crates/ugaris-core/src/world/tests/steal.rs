// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;

fn thief(id: u32) -> Character {
    let mut character = character(id);
    character.professions[profession::THIEF] = 20;
    character
}

fn place_facing(world: &mut World, thief: Character, victim: Character) {
    // Push the world tick well past `TICKS_PER_SECOND` so the default
    // `regen_ticker == 0` victim isn't spuriously treated as "just
    // regenerated" (C `ticker - ch[co].regen_ticker < TICKS`).
    world.tick = Tick(TICKS_PER_SECOND * 10);
    assert!(world.spawn_character(thief, 10, 10));
    assert!(world.spawn_character(victim, 11, 10));
    let attacker = world.characters.get_mut(&CharacterId(1)).unwrap();
    attacker.dir = Direction::Right as u8;
}

#[test]
fn not_a_thief_without_profession_points() {
    let mut world = World::default();
    let attacker = character(1);
    let victim = character(2);
    place_facing(&mut world, attacker, victim);

    assert_eq!(world.attempt_steal(CharacterId(1)), StealOutcome::NotAThief);
}

#[test]
fn busy_thief_cannot_steal() {
    let mut world = World::default();
    let mut attacker = thief(1);
    attacker.action = 5;
    let victim = character(2);
    place_facing(&mut world, attacker, victim);

    assert_eq!(world.attempt_steal(CharacterId(1)), StealOutcome::NotIdle);
}

#[test]
fn full_hand_blocks_steal() {
    let mut world = World::default();
    let mut attacker = thief(1);
    attacker.cursor_item = Some(ItemId(900));
    let victim = character(2);
    place_facing(&mut world, attacker, victim);

    assert_eq!(world.attempt_steal(CharacterId(1)), StealOutcome::HandFull);
}

#[test]
fn no_one_to_steal_from() {
    let mut world = World::default();
    let attacker = thief(1);
    assert!(world.spawn_character(attacker, 10, 10));
    let attacker = world.characters.get_mut(&CharacterId(1)).unwrap();
    attacker.dir = Direction::Right as u8;

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::NoOneThere
    );
}

#[test]
fn cannot_steal_from_arena_or_clan_tile() {
    let mut world = World::default();
    let attacker = thief(1);
    let victim = character(2);
    place_facing(&mut world, attacker, victim);
    // Both tiles need `ARENA` (and to be flood-fill-connected) for
    // `can_attack` itself to allow it - only then does `cmd_steal`'s own
    // separate `map[m].flags & (MF_ARENA|MF_CLAN)` check on the victim's
    // tile reject the steal (C `prof.c:159-162`).
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::ARENA);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::ARENA);

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::ArenaOrClan
    );
}

#[test]
fn cannot_steal_from_npc() {
    let mut world = World::default();
    let attacker = thief(1);
    let victim = character(2);
    place_facing(&mut world, attacker, victim);

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::NotAPlayer
    );
}

#[test]
fn cannot_steal_from_lagging_player() {
    let mut world = World::default();
    let attacker = thief(1);
    let mut victim = character(2);
    victim.flags.insert(CharacterFlags::PLAYER);
    victim.driver = CDR_LOSTCON;
    place_facing(&mut world, attacker, victim);

    assert_eq!(world.attempt_steal(CharacterId(1)), StealOutcome::Lagging);
}

#[test]
fn cannot_steal_in_live_quests_area() {
    let mut world = World::default();
    world.area_id = 20;
    let attacker = thief(1);
    let mut victim = character(2);
    victim.flags.insert(CharacterFlags::PLAYER);
    place_facing(&mut world, attacker, victim);

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::LiveQuests
    );
}

#[test]
fn victim_not_idle_blocks_steal() {
    let mut world = World::default();
    let attacker = thief(1);
    let mut victim = character(2);
    victim.flags.insert(CharacterFlags::PLAYER);
    victim.action = 5;
    place_facing(&mut world, attacker, victim);

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::VictimBusy
    );
}

#[test]
fn victim_recently_regenerated_blocks_steal() {
    let mut world = World::default();
    let attacker = thief(1);
    let victim = character(2);
    place_facing(&mut world, attacker, victim);
    let tick = world.tick.0;
    let victim = world.characters.get_mut(&CharacterId(2)).unwrap();
    victim.flags.insert(CharacterFlags::PLAYER);
    victim.regen_ticker = (tick - 1) as u32;

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::VictimBusy
    );
}

#[test]
fn nothing_to_steal_from_empty_victim() {
    let mut world = World::default();
    let attacker = thief(1);
    let mut victim = character(2);
    victim.flags.insert(CharacterFlags::PLAYER);
    place_facing(&mut world, attacker, victim);

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::NothingToSteal
    );
}

#[test]
fn quest_items_are_not_stealable() {
    let mut world = World::default();
    let attacker = thief(1);
    let mut victim = character(2);
    victim.flags.insert(CharacterFlags::PLAYER);
    victim.inventory[30] = Some(ItemId(900));
    place_facing(&mut world, attacker, victim);
    let mut quest_item = item(900, ItemFlags::QUEST | ItemFlags::USED);
    quest_item.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(900), quest_item);

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::NothingToSteal
    );
}

#[test]
fn would_be_caught_when_percept_far_outweighs_stealth() {
    let mut world = World::default();
    let attacker = thief(1);
    let mut victim = character(2);
    victim.flags.insert(CharacterFlags::PLAYER);
    victim.inventory[30] = Some(ItemId(900));
    victim.values[0][CharacterValue::Percept as usize] = 200;
    place_facing(&mut world, attacker, victim);
    let mut stealable = item(900, ItemFlags::USED);
    stealable.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(900), stealable);

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::WouldBeCaught
    );
    // Would-be-caught still runs the C `cnt = RANDOM(cnt);` item-pick draw
    // before the `chance < 10` check, but never rolls the theft dice or
    // moves the item.
    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(victim.inventory[30], Some(ItemId(900)));
}

/// Shared setup for the three dice-roll outcomes below: `chance` is
/// pinned to exactly 50 (`stealth - percept == 20` -> `diff == 10` ->
/// `40 + 10 == 50`, capped by `THIEF * 3 == 60` so the cap never bites),
/// so only the RNG seed decides which of the three `diff = chance - dice`
/// buckets is hit (seeds 0/1/2 below were brute-forced against the exact
/// `legacy_random_below_from_seed` LCG to land in each bucket).
fn setup_dice_scenario(seed: u32) -> World {
    let mut world = World::default();
    world.legacy_random_seed = seed;
    let mut attacker = thief(1);
    attacker.values[0][CharacterValue::Stealth as usize] = 20;
    let mut victim = character(2);
    victim.flags.insert(CharacterFlags::PLAYER);
    victim.inventory[30] = Some(ItemId(900));
    place_facing(&mut world, attacker, victim);
    let mut stealable = item(900, ItemFlags::USED);
    stealable.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(900), stealable);
    world
}

#[test]
fn stolen_unnoticed_transfers_item_silently() {
    let mut world = setup_dice_scenario(1);

    let outcome = world.attempt_steal(CharacterId(1));
    assert_eq!(
        outcome,
        StealOutcome::StolenUnnoticed {
            victim_name: "Character".to_string(),
            item_name: "Item".to_string(),
        }
    );
    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert!(victim.inventory[30].is_none());
    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert!(attacker.inventory.contains(&Some(ItemId(900))));
    assert_eq!(
        world.items.get(&ItemId(900)).unwrap().carried_by,
        Some(CharacterId(1))
    );
}

#[test]
fn stolen_noticed_transfers_item_and_notifies_victim() {
    let mut world = setup_dice_scenario(0);

    let outcome = world.attempt_steal(CharacterId(1));
    assert_eq!(
        outcome,
        StealOutcome::StolenNoticed {
            victim_id: CharacterId(2),
            victim_name: "Character".to_string(),
            item_name: "Item".to_string(),
        }
    );
    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert!(attacker.inventory.contains(&Some(ItemId(900))));
    assert_eq!(attacker.endurance, 1);
}

#[test]
fn caught_leaves_item_with_victim() {
    let mut world = setup_dice_scenario(2);

    let outcome = world.attempt_steal(CharacterId(1));
    assert_eq!(
        outcome,
        StealOutcome::Caught {
            victim_id: CharacterId(2),
            victim_name: "Character".to_string(),
        }
    );
    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(victim.inventory[30], Some(ItemId(900)));
    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.endurance, 1);
    assert!(!attacker.inventory.contains(&Some(ItemId(900))));
}

#[test]
fn one_carry_item_already_held_is_not_stealable() {
    let mut world = World::default();
    let mut attacker = thief(1);
    attacker.inventory[30] = Some(ItemId(1));
    let mut victim = character(2);
    victim.flags.insert(CharacterFlags::PLAYER);
    victim.inventory[31] = Some(ItemId(900));
    place_facing(&mut world, attacker, victim);
    let mut held = item(1, ItemFlags::USED);
    held.driver = crate::item_driver::IDR_CLANJEWEL;
    world.items.insert(ItemId(1), held);
    let mut stealable = item(900, ItemFlags::USED);
    stealable.driver = crate::item_driver::IDR_CLANJEWEL;
    world.items.insert(ItemId(900), stealable);

    assert_eq!(
        world.attempt_steal(CharacterId(1)),
        StealOutcome::NothingToSteal
    );
}
