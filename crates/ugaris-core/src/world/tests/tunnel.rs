use super::*;
use crate::entity::CharacterValue as V;
use crate::item_driver::{
    IDR_TUNNELDOOR, IDR_TUNNELDOOR2, IID_TUNNELDOOR1, IID_TUNNELENEMY1, IID_TUNNELENEMY2,
    IID_TUNNELENEMY3, IID_TUNNELENEMYALL,
};
use crate::player::MAX_TUNNEL_LEVEL;
use crate::world::{legacy_random_below_from_seed, tunnel_build_fighter_stat_values};

fn facts(reward_level: i32, used: &[(i32, u8)]) -> TunnelRewardFacts {
    let mut tunnel_used = vec![0u8; (MAX_TUNNEL_LEVEL as usize) + 1];
    for &(level, value) in used {
        tunnel_used[level as usize] = value;
    }
    TunnelRewardFacts {
        reward_level,
        tunnel_used,
    }
}

// C `give_reward`'s `DOOR_EXIT_EXP` branch (`tunnel.c:542-547`): a fresh
// (never-completed) level grants `level_value(reward_level) /
// tunnel_exp_base_value_divider / (used[reward_level] + 9)` exp - the
// denominator reads the *post*-increment `used` count (`1 + 9 = 10` here).
#[test]
fn exit_exp_first_completion_grants_expected_exp_and_progress_message() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 60;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.new_used_count, Some((50, 1)));
    assert_eq!(outcome.promote_gorwin_to, None);
    assert!(outcome.award_achievement);
    assert_eq!(
        outcome.messages,
        vec![
            "You have been given experience.".to_string(),
            "Completions at level 50: 1/10 (9 remaining).".to_string(),
        ]
    );
    // level_value(50) = 51^4 - 50^4 = 515201; /5.0 (default divider) =
    // 103040.2; /(1+9) = 10304.02 -> truncated to 10304.
    assert_eq!(world.characters[&CharacterId(1)].exp, 10304);
}

// C `give_reward`'s `DOOR_EXIT_MILITARY` branch (`tunnel.c:548-554`):
// `(tunnel_mill_exp_base_value + reward_level^2/10) / (used + 9)`, all
// integer math.
#[test]
fn exit_military_first_completion_grants_expected_points() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 60;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 3, 33);

    assert_eq!(outcome.new_used_count, Some((50, 1)));
    assert!(outcome.award_achievement);
    assert_eq!(outcome.messages[0], "You have been given military rank.");
    // (100 + 50*50/10) / (1+9) = (100+250)/10 = 35.
    assert_eq!(world.characters[&CharacterId(1)].military_points, 35);
}

// C `give_reward`'s auto-promote-on-mastery branch (`tunnel.c:559-580`),
// "next level found" arm.
#[test]
fn exit_reward_auto_promotes_to_next_available_level_on_final_use() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 100;
    assert!(world.spawn_character(player, 10, 10));

    // 9 completions already recorded - this reward is the 10th (final).
    let facts = facts(50, &[(50, 9)]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.new_used_count, Some((50, 10)));
    assert_eq!(outcome.promote_gorwin_to, Some(51));
    assert!(outcome.award_achievement);
    assert_eq!(
        outcome.messages,
        vec![
            "You have been given experience.".to_string(),
            "Tunnel Mastery! Thou hast conquered all 10 challenges at level 50.".to_string(),
            "Gorwin has advanced thy tunnel level to 51. Onward and upward!".to_string(),
        ]
    );
}

// Same branch, "no next level available" arm (`tunnel.c:572-577`): the
// character's own level caps how high `find_next_available_level` can
// search.
#[test]
fn exit_reward_on_final_use_with_no_higher_level_available_reports_mastery() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 50;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[(50, 9)]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.promote_gorwin_to, None);
    assert_eq!(
        outcome.messages,
        vec![
            "You have been given experience.".to_string(),
            "Tunnel Mastery! Thou hast conquered all 10 challenges at level 50.".to_string(),
            "There are no more tunnel levels available to thee. Thou art a true master of the depths!"
                .to_string(),
        ]
    );
}

// C `give_reward`'s `else` branch (`tunnel.c:587-599`): the level was
// already fully completed before this use, so no reward is granted at
// all, but a still-reachable higher level auto-promotes anyway.
#[test]
fn exit_reward_on_already_maxed_level_grants_no_reward_but_still_promotes() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 100;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[(50, 10)]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.new_used_count, None);
    assert!(!outcome.award_achievement);
    assert_eq!(outcome.promote_gorwin_to, Some(51));
    assert_eq!(
        outcome.messages,
        vec![
            "You have used all 10 completions at level 50. No reward given.".to_string(),
            "Gorwin has advanced thy tunnel level to 51. Speak with him for details.".to_string(),
        ]
    );
    // No exp/military points were granted.
    assert_eq!(world.characters[&CharacterId(1)].exp, 0);
    assert_eq!(world.characters[&CharacterId(1)].military_points, 0);
}

// Same "already maxed" branch, but no higher level exists either - only
// the "no reward given" line is emitted.
#[test]
fn exit_reward_on_already_maxed_level_with_no_promotion_available() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 50;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[(50, 10)]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.promote_gorwin_to, None);
    assert_eq!(
        outcome.messages,
        vec!["You have used all 10 completions at level 50. No reward given.".to_string()]
    );
}

// C `check_area_clear` (`tunnel.c:750-762`): an empty rectangle in front
// of the door is clear.
#[test]
fn mean_door_area_clear_is_true_when_the_rectangle_ahead_is_empty() {
    let world = World::default();
    assert!(world.tunnel_mean_door_area_clear(10, 10));
}

// A non-player character anywhere in the `DOOR_RANGE`x`DOOR_DEPTH`
// rectangle blocks the door from opening.
#[test]
fn mean_door_area_clear_is_false_when_a_non_player_character_is_in_range() {
    let mut world = World::default();
    let mut baddy = character(1);
    baddy.flags = CharacterFlags::USED;
    assert!(world.spawn_character(baddy, 10, 15));

    assert!(!world.tunnel_mean_door_area_clear(10, 10));
}

// Players in the rectangle don't block the door - only non-player
// characters count (`ch[co].flags & CF_PLAYER`).
#[test]
fn mean_door_area_clear_ignores_player_characters_in_range() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    assert!(world.spawn_character(player, 10, 15));

    assert!(world.tunnel_mean_door_area_clear(10, 10));
}

// Characters outside the rectangle (too far horizontally, or above/at the
// door's own row) don't block it.
#[test]
fn mean_door_area_clear_ignores_characters_outside_the_rectangle() {
    let mut world = World::default();
    // Horizontally out of DOOR_RANGE (4) from x=10.
    let mut far = character(1);
    far.flags = CharacterFlags::USED;
    assert!(world.spawn_character(far, 20, 15));
    // At the door's own row (y+1 is the first checked row).
    let mut same_row = character(2);
    same_row.flags = CharacterFlags::USED;
    assert!(world.spawn_character(same_row, 10, 10));

    assert!(world.tunnel_mean_door_area_clear(10, 10));
}

// C `build_fighter`'s per-skill stat formula (`tunnel.c:334-393`) groups
// skills differently from `area32::mission_start`'s own `build_fighter`:
// `V_HP` alone at `diff-15`, `V_ENDURANCE`/`V_MANA` together at `diff-30`
// (missions.c groups all three at `diff-15`), and `V_FREEZE` at the bare
// `diff` (missions.c groups it with `V_BLESS`/`V_FIREBALL`/
// `V_MAGICSHIELD` at `diff-5`).
#[test]
fn tunnel_build_fighter_stat_values_matches_c_grouping() {
    let mut markers = vec![0i16; crate::entity::CHARACTER_VALUE_COUNT];
    markers[V::Hp as usize] = 10;
    markers[V::Endurance as usize] = 10;
    markers[V::Mana as usize] = 10;
    markers[V::Freeze as usize] = 1;
    markers[V::Bless as usize] = 1;
    markers[V::ArmorSkill as usize] = 1;
    markers[V::Tactics as usize] = 1;
    markers[V::SpeedSkill as usize] = 1;
    markers[V::Warcry as usize] = 1;
    markers[V::Surround as usize] = 1;
    markers[V::Wisdom as usize] = 0; // untouched: marker is zero

    let scaled = tunnel_build_fighter_stat_values(&markers, 40);
    assert_eq!(scaled[V::Hp as usize], 25); // max(10, 40-15)
    assert_eq!(scaled[V::Endurance as usize], 10); // max(10, 40-30)
    assert_eq!(scaled[V::Mana as usize], 10); // max(10, 40-30)
    assert_eq!(scaled[V::Freeze as usize], 40); // max(1, 40) - bare diff
    assert_eq!(scaled[V::Bless as usize], 35); // max(1, 40-5)
    assert_eq!(scaled[V::ArmorSkill as usize], 40); // (40/10)*10
    assert_eq!(scaled[V::Tactics as usize], 35); // max(1, 40-5)
    assert_eq!(scaled[V::SpeedSkill as usize], 35); // max(1, 40-5)
    assert_eq!(scaled[V::Warcry as usize], 25); // max(1, 40-15)
    assert_eq!(scaled[V::Surround as usize], 20); // max(1, 40-20)
    assert_eq!(scaled[V::Wisdom as usize], 0);
}

#[test]
fn tunnel_build_fighter_stat_values_caps_at_250() {
    let mut markers = vec![0i16; crate::entity::CHARACTER_VALUE_COUNT];
    markers[V::Hand as usize] = 1;
    let scaled = tunnel_build_fighter_stat_values(&markers, 1000);
    assert_eq!(scaled[V::Hand as usize], 250);
}

// C `find_unused_sector` (`tunnel.c:488-512`): the first `(xoff, yoff)`
// with no other player standing in it - an empty world returns the very
// first grid slot.
#[test]
fn find_unused_tunnel_sector_returns_first_offset_when_all_empty() {
    let world = World::default();
    assert_eq!(
        world.find_unused_tunnel_sector(CharacterId(1)),
        Some((1, 1))
    );
}

// A player occupying the first sector's instance area is skipped in
// favor of the next; the entering player's own character is excluded
// from the "busy" check (C `co != cn`).
#[test]
fn find_unused_tunnel_sector_skips_occupied_and_excludes_self() {
    let mut world = World::default();
    let mut other = character(2);
    other.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(other, 5, 5));

    // The entering player (id 1) standing in the same first sector does
    // not count against itself.
    let mut me = character(1);
    me.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(me, 6, 6));

    assert_eq!(
        world.find_unused_tunnel_sector(CharacterId(1)),
        Some((1, 128))
    );
}

fn marker_item(id: u32, template_id: u32) -> Item {
    let mut marker = item(id, ItemFlags::USED);
    marker.template_id = template_id;
    marker
}

// C `handle_block_marker` (`tunnel.c:460-467`): exactly one of the
// `BLOCK_MARKER_1` copies (the seeded `RANDOM(3)` choice) opens, the
// rest become solid walls.
#[test]
fn plan_tunnel_entry_opens_the_seeded_block_marker_and_walls_the_rest() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.x = 250;
    player.y = 250;
    player.values[0][V::Hp as usize] = 20;
    player.values[1][V::Hp as usize] = 20;
    assert!(world.spawn_character(player, 250, 250));

    // Same x, increasing y - matches the scan's own x-major/y-minor
    // iteration order (`tunnel.c:681-682`).
    for (id, y) in [(10u32, 5u16), (11, 6), (12, 7)] {
        let mut marker = marker_item(id, IID_TUNNELDOOR1);
        assert!(world.map.set_item_map(&mut marker, 5, usize::from(y)));
        world.add_item(marker);
    }

    let clevel = 10;
    let mut seed = 1u32.wrapping_mul(clevel as u32);
    let b1 = legacy_random_below_from_seed(&mut seed, 3);

    let plan = world
        .plan_tunnel_entry(CharacterId(1), clevel, 0, 0)
        .expect("first sector should be free");
    assert!(plan.creepers.is_empty());

    for (idx, (id, y)) in [(10u32, 5u16), (11, 6), (12, 7)].into_iter().enumerate() {
        let marker = &world.items[&ItemId(id)];
        assert_eq!(marker.sprite, 0, "marker sprite always hidden");
        let tile = world.map.tile(5, usize::from(y)).unwrap();
        if idx as u32 == b1 {
            assert_eq!(tile.foreground_sprite, 0, "chosen marker opens");
            assert!(!tile.flags.contains(MapFlags::TMOVEBLOCK));
        } else {
            assert_eq!(tile.foreground_sprite, 59791, "unchosen marker walls");
            assert!(tile
                .flags
                .contains(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK));
        }
    }
}

// C `handle_creeper_marker` (`tunnel.c:469-482`): only the seeded
// `RANDOM(3)` choice among `CREEPER_MARKER_1/2/3` queues a creeper spawn;
// every marker cell (spawned or not) ends up open/unblocked.
#[test]
fn plan_tunnel_entry_queues_only_the_seeded_creeper_marker() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.x = 250;
    player.y = 250;
    player.values[0][V::Hp as usize] = 20;
    player.values[1][V::Hp as usize] = 20;
    assert!(world.spawn_character(player, 250, 250));

    for (id, y, template_id) in [
        (20u32, 5u16, IID_TUNNELENEMY1),
        (21, 6, IID_TUNNELENEMY2),
        (22, 7, IID_TUNNELENEMY3),
    ] {
        let mut marker = marker_item(id, template_id);
        assert!(world.map.set_item_map(&mut marker, 5, usize::from(y)));
        world.add_item(marker);
    }

    let clevel = 10;
    let mut seed = 1u32.wrapping_mul(clevel as u32);
    let _b1 = legacy_random_below_from_seed(&mut seed, 3);
    let _b2 = legacy_random_below_from_seed(&mut seed, 2);
    let c = legacy_random_below_from_seed(&mut seed, 3);

    let plan = world
        .plan_tunnel_entry(CharacterId(1), clevel, 0, 0)
        .expect("first sector should be free");

    assert_eq!(plan.creepers.len(), 1);
    let spawned = &plan.creepers[0];
    assert_eq!(spawned.y, 5 + c as u16);
    assert_eq!(spawned.level, clevel);
    // creeper_tab[10 - MIN_TUNNEL_LEVEL(10)] = creeper_tab[0] = 13.
    assert_eq!(spawned.diff, 13);

    // Every marker cell ends up open, regardless of whether it spawned.
    for y in [5u16, 6, 7] {
        let tile = world.map.tile(5, usize::from(y)).unwrap();
        assert_eq!(tile.foreground_sprite, 0);
    }
}

// `CREEPER_MARKER_ALL` always spawns, regardless of the seeded choice.
#[test]
fn plan_tunnel_entry_creeper_marker_all_always_spawns() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.x = 250;
    player.y = 250;
    player.values[0][V::Hp as usize] = 20;
    player.values[1][V::Hp as usize] = 20;
    assert!(world.spawn_character(player, 250, 250));

    let mut marker = marker_item(30, IID_TUNNELENEMYALL);
    assert!(world.map.set_item_map(&mut marker, 5, 5));
    world.add_item(marker);

    let plan = world
        .plan_tunnel_entry(CharacterId(1), 10, 0, 0)
        .expect("first sector should be free");
    assert_eq!(plan.creepers.len(), 1);
    assert_eq!((plan.creepers[0].x, plan.creepers[0].y), (5, 5));
}

// C `update_exit_door` (`tunnel.c:484-486`): relabels an `IDR_TUNNELDOOR`
// exit pillar encountered mid-scan with the entering level and the
// player's completion count at that level.
#[test]
fn plan_tunnel_entry_relabels_exit_door_pillars() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.x = 250;
    player.y = 250;
    player.values[0][V::Hp as usize] = 20;
    player.values[1][V::Hp as usize] = 20;
    assert!(world.spawn_character(player, 250, 250));

    let mut door = item(40, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_TUNNELDOOR;
    door.driver_data = vec![2]; // DOOR_EXIT_EXP
    assert!(world.map.set_item_map(&mut door, 5, 5));
    world.add_item(door);

    world
        .plan_tunnel_entry(CharacterId(1), 42, 0, 7)
        .expect("first sector should be free");

    assert_eq!(world.items[&ItemId(40)].name, "Column 42, used 7 times");
}

// C's `default:` scan-loop arm for `IDR_TUNNELDOOR2` (`tunnel.c:710-713`):
// always reset to closed whenever the instance regenerates.
#[test]
fn plan_tunnel_entry_closes_mean_doors_encountered_mid_scan() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.x = 250;
    player.y = 250;
    player.values[0][V::Hp as usize] = 20;
    player.values[1][V::Hp as usize] = 20;
    assert!(world.spawn_character(player, 250, 250));

    let mut mean_door = item(50, ItemFlags::USED | ItemFlags::USE);
    mean_door.driver = IDR_TUNNELDOOR2;
    assert!(world.map.set_item_map(&mut mean_door, 5, 5));
    world.add_item(mean_door);

    world
        .plan_tunnel_entry(CharacterId(1), 10, 0, 0)
        .expect("first sector should be free");

    assert_eq!(world.items[&ItemId(50)].sprite, 0);
    let tile = world.map.tile(5, 5).unwrap();
    assert_eq!(tile.foreground_sprite, 59791);
    assert!(tile
        .flags
        .contains(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK));
}

// C's HP update (`tunnel.c:727-731`): `DOOR_ENTRY` fully refills HP,
// `DOOR_CONTINUE` only regenerates half of `value[1][V_HP]`, capped at
// max HP.
#[test]
fn plan_tunnel_entry_hp_update_differs_between_entry_and_continue() {
    let mut world = World::default();
    let mut entry_player = character(1);
    entry_player.flags.insert(CharacterFlags::PLAYER);
    entry_player.x = 250;
    entry_player.y = 250;
    entry_player.hp = 500;
    entry_player.values[0][V::Hp as usize] = 20; // max hp = 20_000
    entry_player.values[1][V::Hp as usize] = 20;
    assert!(world.spawn_character(entry_player, 250, 250));

    world
        .plan_tunnel_entry(CharacterId(1), 10, 0, 0)
        .expect("first sector should be free");
    assert_eq!(world.characters[&CharacterId(1)].hp, 20_000);

    let mut continue_player = character(2);
    continue_player.flags.insert(CharacterFlags::PLAYER);
    continue_player.x = 250;
    continue_player.y = 250;
    continue_player.hp = 1_000;
    continue_player.values[0][V::Hp as usize] = 20; // max hp = 20_000
    continue_player.values[1][V::Hp as usize] = 20; // regen = 20*1000/2 = 10_000
    assert!(world.spawn_character(continue_player, 250, 250));

    world
        .plan_tunnel_entry(CharacterId(2), 10, 1, 0)
        .expect("first sector should be free");
    // min(20_000, 1_000 + 10_000) = 11_000.
    assert_eq!(world.characters[&CharacterId(2)].hp, 11_000);
}

// C `teleport_char_driver(cn, 16 + xoff, 123 + yoff)` (`tunnel.c:733`).
#[test]
fn plan_tunnel_entry_teleports_to_the_chosen_sector_landing_tile() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.x = 250;
    player.y = 250;
    player.values[0][V::Hp as usize] = 20;
    player.values[1][V::Hp as usize] = 20;
    assert!(world.spawn_character(player, 250, 250));

    world
        .plan_tunnel_entry(CharacterId(1), 10, 0, 0)
        .expect("first sector should be free");

    let landed = &world.characters[&CharacterId(1)];
    assert_eq!((landed.x, landed.y), (17, 124));
}

// C's own "All tunnels are busy" refusal (`tunnel.c:667-670`): every
// instance sector already has another player in it.
#[test]
fn plan_tunnel_entry_returns_none_when_every_sector_is_busy() {
    let mut world = World::default();
    let mut entering = character(1);
    entering.flags.insert(CharacterFlags::PLAYER);
    entering.x = 250;
    entering.y = 250;
    assert!(world.spawn_character(entering, 250, 250));

    let mut xoff = 1u16;
    let mut next_id = 100u32;
    while xoff < 210 {
        let mut yoff = 1u16;
        while yoff < 255 {
            if !(xoff == 218 && yoff == 128) {
                let mut occupant = character(next_id);
                occupant.flags.insert(CharacterFlags::PLAYER);
                assert!(world.spawn_character(
                    occupant,
                    usize::from(1 + xoff),
                    usize::from(1 + yoff)
                ));
                next_id += 1;
            }
            yoff += 127;
        }
        xoff += 31;
    }

    assert_eq!(world.plan_tunnel_entry(CharacterId(1), 10, 0, 0), None);
}
