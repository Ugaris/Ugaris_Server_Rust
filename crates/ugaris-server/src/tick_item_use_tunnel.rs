//! Completed-action-outcome handling: the Long Tunnels (`src/area/33/
//! tunnel.c`) `IDR_TUNNELDOOR` exit-pillar family
//! (`TunnelDoorExitReward`) and entrance/next-level-door family
//! (`TunnelDoorEnter`). Split out of the giant `match outcome { ... }`
//! block per the same P0.5 "Finish main() phase decomposition" precedent
//! as every other `tick_item_use_*` sibling.
//!
//! `TunnelRewardFacts`/`TunnelRewardOutcome` (`ugaris_core::world::tunnel`)
//! carry the `PlayerRuntime` snapshot in and the `PlayerRuntime` writes/
//! feedback lines back out, the same split `area33.rs`'s Gorwin wiring
//! (`GorwinPlayerFacts`/`GorwinOutcomeEvent`) already established for this
//! area. [`dispatch_tunnel_enter_outcome`] follows the same shape for
//! `tunneldoor`'s `DOOR_ENTRY`/`DOOR_CONTINUE` branches (`tunnel.c:
//! 638-734`): it resolves the target `clevel` from `PlayerRuntime`
//! (`gorwin_ppd`/`tunnel_ppd`, needing the entry guard + level-selection
//! logic `World` cannot run on its own), then hands off to `World::
//! plan_tunnel_entry` for the actual map-scan/creeper-spawn-planning
//! (pure `World` state), and finally instantiates each planned creeper
//! via `ZoneLoader` (`spawn_tunnel_creeper`, mirroring `area32.rs`'s own
//! `spawn_mission_fighter`).

use super::*;
use ugaris_core::player::find_next_available_tunnel_level;
use ugaris_core::world::{calc_exp, tunnel_build_fighter_stat_values, TunnelCreeperSpawnSpec};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_tunnel_outcome(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    area_id: u16,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
) {
    let ugaris_core::item_driver::ItemDriverOutcome::TunnelDoorExitReward {
        character_id,
        door_type,
        ..
    } = outcome
    else {
        return;
    };

    // C `if (teleport_char_driver(cn, 250, 250)) { give_reward(...); ppd->
    // clevel = MIN_TUNNEL_LEVEL; }` (`tunnel.c:631-634`): both the reward
    // and the `clevel` reset only happen when the teleport actually moves
    // the player (C's own `teleport_char_driver` returns `0`/no-op when
    // already within 1 tile of the target).
    if !world.teleport_char_driver(character_id, 250, 250) {
        *blocked += 1;
        return;
    }

    let Some(facts) = runtime
        .player_for_character(character_id)
        .map(|player| TunnelRewardFacts {
            reward_level: player.gorwin_tunnel_level(),
            tunnel_used: (0..=MAX_TUNNEL_LEVEL)
                .map(|level| player.tunnel_used(level))
                .collect(),
        })
    else {
        *executed += 1;
        return;
    };

    let result = world.apply_tunnel_reward(character_id, &facts, door_type, u32::from(area_id));

    if let Some(player) = runtime.player_for_character_mut(character_id) {
        if let Some((level, used)) = result.new_used_count {
            player.set_tunnel_used(level, used);
        }
        if let Some(next) = result.promote_gorwin_to {
            player.set_gorwin_tunnel_level(next);
        }
        player.set_tunnel_clevel(MIN_TUNNEL_LEVEL);
    }

    for message in result.messages {
        feedback.push((character_id, message));
    }

    if result.award_achievement {
        award_tunnel_level_achievement(world, runtime, achievement_repository, character_id).await;
    }

    *executed += 1;
}

/// C `tunneldoor`'s `DOOR_ENTRY`/`DOOR_CONTINUE` branches (`src/area/33/
/// tunnel.c:603-734`, minus the `DOOR_EXIT_EXP`/`DOOR_EXIT_MILITARY`
/// branches [`dispatch_tunnel_outcome`] already owns). `door_type == 0`
/// is `DOOR_ENTRY` (the lobby entrance column), `door_type == 1` is
/// `DOOR_CONTINUE` (an in-tunnel "door to next level" column) - see C's
/// own `enum TunnelDoorType` (`tunnel.h:16`).
pub(crate) fn dispatch_tunnel_enter_outcome(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
) {
    let ugaris_core::item_driver::ItemDriverOutcome::TunnelDoorEnter {
        character_id,
        door_type,
        ..
    } = outcome
    else {
        return;
    };

    let Some(player) = runtime.player_for_character(character_id) else {
        *blocked += 1;
        return;
    };
    let gorwin_level = player.gorwin_tunnel_level();

    // C `if (door_type == DOOR_ENTRY && gorwin_ppd->tunnel_level == 0) {
    // ...; return; }` (`:625-628`).
    if door_type == 0 && gorwin_level == 0 {
        feedback.push((
            character_id,
            "Thou must first speak with Gorwin before thou canst enter the long tunnels."
                .to_string(),
        ));
        *blocked += 1;
        return;
    }

    let char_level = world
        .characters
        .get(&character_id)
        .map(|character| character.level as i32)
        .unwrap_or(0);
    let tunnel_used: Vec<u8> = (0..=MAX_TUNNEL_LEVEL)
        .map(|level| player.tunnel_used(level))
        .collect();
    let clevel_before = player.tunnel_clevel();

    let new_clevel;
    if door_type == 0 {
        // C `ppd->clevel = min(gorwin_ppd->tunnel_level, ch[cn].level);`
        // plus the maxed-out-level warning (`:640-656`).
        let level = gorwin_level.min(char_level);
        new_clevel = level;
        if tunnel_used.get(level as usize).copied().unwrap_or(0) >= MAX_TUNNEL_USES {
            let message = match find_next_available_tunnel_level(&tunnel_used, level, char_level)
            {
                Some(_) => format!(
                    "Warning: Thou hast used all {MAX_TUNNEL_USES} completions at level {level}. Rewards will not be granted. Speak with Gorwin to change thy level."
                ),
                None => format!(
                    "Warning: All tunnel levels up to {level} are fully completed. No rewards remain."
                ),
            };
            feedback.push((character_id, message));
        }
    } else {
        // C `ppd->clevel++; if (ppd->clevel > MAX_TUNNEL_LEVEL) { ...;
        // ppd->clevel = MAX_TUNNEL_LEVEL; return; }` (`:658-664`) - the
        // clamp is persisted even though entry itself is refused.
        let next = clevel_before + 1;
        if next > MAX_TUNNEL_LEVEL {
            feedback.push((
                character_id,
                "Thou hast reached the deepest depths of the tunnels. There is no way forward."
                    .to_string(),
            ));
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                player.set_tunnel_clevel(MAX_TUNNEL_LEVEL);
            }
            *blocked += 1;
            return;
        }
        new_clevel = next;
    }

    // C persists `ppd->clevel` before the `find_unused_sector` busy
    // check (`:667-670`) - so the level change "sticks" even if entry
    // itself fails below.
    if let Some(player) = runtime.player_for_character_mut(character_id) {
        player.set_tunnel_clevel(new_clevel);
    }

    let used_at_clevel = tunnel_used.get(new_clevel as usize).copied().unwrap_or(0);
    let Some(plan) = world.plan_tunnel_entry(character_id, new_clevel, door_type, used_at_clevel)
    else {
        feedback.push((
            character_id,
            "All tunnels are busy. Please try again later.".to_string(),
        ));
        *blocked += 1;
        return;
    };

    for spec in &plan.creepers {
        spawn_tunnel_creeper(world, loader, runtime, spec);
    }

    *executed += 1;
}

/// C `build_fighter` (`src/area/33/tunnel.c:325-449`)'s actual
/// `create_char`/equip-item instantiation - the pure stat formula lives
/// in `ugaris_core::world::tunnel::tunnel_build_fighter_stat_values`
/// (`World::plan_tunnel_entry` cannot reach `ZoneLoader` to spawn a real
/// character itself), mirroring `area32.rs::spawn_mission_fighter`'s own
/// plan/spawn split. Unlike mission fighters, a tunnel creeper keeps its
/// template's own driver (`CDR_SIMPLEBADDY`, `zones/33/tunnel.chr`'s
/// `driver=7`) - no `apply_simple_baddy_create_message` re-init needed,
/// `ZoneLoader::instantiate_character_template` already ran it once
/// during template instantiation (`zone.rs`'s own `create_character_with_id`).
fn spawn_tunnel_creeper(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    spec: &TunnelCreeperSpawnSpec,
) -> bool {
    let character_id = runtime.allocate_character_id();
    let Ok((mut fighter, mut inventory_items)) =
        loader.instantiate_character_template("tunnel_creeper", character_id)
    else {
        return false;
    };

    let markers = fighter.values[1].clone();
    fighter.values[1] = tunnel_build_fighter_stat_values(&markers, spec.diff);

    // C `ch[cn].x = ch[cn].tmpx = x; ch[cn].y = ch[cn].tmpy = y; ch[cn].
    // dir = DX_RIGHTDOWN;` (`:396-398`).
    fighter.x = spec.x;
    fighter.y = spec.y;
    fighter.dir = Direction::RightDown as u8;

    // C `ch[cn].exp = ch[cn].exp_used = calc_exp(cn); ch[cn].level =
    // level;` (`:401-402`) - `level` is set directly, *not* derived from
    // `exp2level` like `area32`'s mission fighters.
    fighter.exp = calc_exp(&fighter);
    fighter.exp_used = fighter.exp;
    fighter.level = spec.level as u32;

    // C `equip1`/`equip2` (`:405-421`): both stamp all 5 of their
    // `mod_value[]` slots to `1 + ch[cn].level/3`.
    let equip_value = (1 + spec.level / 3) as i16;
    if let Ok(mut equip1) = loader.instantiate_item_template("equip1", Some(character_id)) {
        for slot in 0..5 {
            equip1.modifier_value[slot] = equip_value;
        }
        fighter.inventory[13] = Some(equip1.id);
        inventory_items.push(equip1);
    }
    if let Ok(mut equip2) = loader.instantiate_item_template("equip2", Some(character_id)) {
        for slot in 0..5 {
            equip2.modifier_value[slot] = equip_value;
        }
        fighter.inventory[14] = Some(equip2.id);
        inventory_items.push(equip2);
    }
    // C `armor_spell`/`weapon_spell` (`:424-436`) - note the weapon
    // modifier is divided by 2 here, unlike `area32`'s mission fighters
    // (a real, deliberate C difference between the two `build_fighter`s).
    if let Ok(mut armor) = loader.instantiate_item_template("armor_spell", Some(character_id)) {
        let armor_skill = i32::from(fighter.values[1][CharacterValue::ArmorSkill as usize]);
        armor.modifier_value[0] = (armor_skill.clamp(13, 113) * 20) as i16;
        fighter.inventory[15] = Some(armor.id);
        inventory_items.push(armor);
    }
    if let Ok(mut weapon) = loader.instantiate_item_template("weapon_spell", Some(character_id)) {
        let hand_skill = i32::from(fighter.values[1][CharacterValue::Hand as usize]);
        weapon.modifier_value[0] = (hand_skill.clamp(13, 113) / 2) as i16;
        fighter.inventory[16] = Some(weapon.id);
        inventory_items.push(weapon);
    }

    if !world.spawn_character(fighter, usize::from(spec.x), usize::from(spec.y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.update_character(character_id);
    // C `ch[cn].hp = value[0][V_HP] * POWERSCALE; ... endurance ...;
    // ... mana ...;` (`:441-443`).
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
        character.endurance =
            i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
        character.mana = i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    }
    true
}
