//! Area 8 (`src/area/8/fdemon.c`) server-side glue for `CDR_FDEMON_ARMY`:
//! the actual soldier spawning (`take_soldiers`/`drop_soldiers`, C
//! `fdemon.c:394-625`), which needs `ZoneLoader`/`ServerRuntime::
//! allocate_character_id` that `ugaris_core::world::World` doesn't have -
//! same split as `pents.rs`'s `spawn_demons_at_pentagram` glue. The pure
//! per-player recruitment planning (`plan_soldier_recruitment`) and the
//! follow-driver/tick/emote logic once a soldier exists all live in
//! `ugaris_core::world::npc::area8::fdemon_army`/`fdemon_army_combat`/
//! `fdemon_army_emote` - see those modules' own doc comments for the full
//! split rationale and remaining gaps (soldier exp/promotion, emote
//! relationship state not yet surviving a drop/re-recruit cycle - see
//! `fdemon_army_emote.rs`'s own doc comment for that one).

use super::*;
use ugaris_core::{
    character_driver::{CharacterDriverState, FightDriverData, CDR_FDEMON_ARMY},
    world::npc::area8::fdemon_army::{
        assign_profile, finalize_soldier_exp_and_level, plan_soldier_recruitment,
        scale_soldier_values, soldier_base_strength, soldier_equipment_items, FarmyData,
        MAXSOLDIER, MIS_FOLLOW, SOLDIER_PROFILES, SOLDIER_TYPE_WARRIOR,
    },
};

/// C `drop_soldiers(cn)` (`fdemon.c:592-625`): destroys every currently
/// alive recruited soldier and folds its unspent exp back into the
/// `farmy_ppd` PPD record. C scans every character in the player's group;
/// this port instead uses the direct `cn`/`serial` index already carried
/// by `PlayerRuntime::farmy_soldier_cn`/`_serial` (see those accessors'
/// own doc comments) - behaviorally identical for the up-to-`MAXSOLDIER`
/// soldiers a player can ever have, just without the O(all characters)
/// scan C's simpler data model required.
pub(crate) fn drop_soldiers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    player_id: CharacterId,
) {
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return;
    };
    for slot in 0..MAXSOLDIER {
        let cn = player.farmy_soldier_cn(slot);
        let serial = player.farmy_soldier_serial(slot);
        if cn != 0 {
            let character_id = CharacterId(cn as u32);
            if let Some(character) = world.characters.get(&character_id) {
                if character.serial == serial as u32 {
                    let exp_gained = character.exp as i32 - character.exp_used as i32;
                    let prior_exp = player.farmy_soldier_exp(slot);
                    player.set_farmy_soldier_exp(slot, prior_exp + exp_gained);
                    world.remove_character(character_id);
                }
            }
        }
        // C only resets `serial`, not `cn`/`type`/`rank`/`profile`/`exp` -
        // a later `take_soldiers` rebuilds the character body from those
        // still-persisted fields (see that function's own doc comment).
        player.set_farmy_soldier_serial(slot, 0);
    }
}

/// C `take_soldiers(cn)` (`fdemon.c:451-590`): rolls newly-eligible
/// recruit slots via [`plan_soldier_recruitment`], then (re)spawns every
/// slot that has a `type` set - including ones recruited in a prior call,
/// matching C's own `if (ppd->soldier[n].type) { ...recreate... }`
/// unconditional-on-existing-type rebuild.
pub(crate) fn take_soldiers(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    player_id: CharacterId,
) {
    let Some(character) = world.characters.get(&player_id) else {
        return;
    };
    let army_rank = army_rank_for_points(character.military_points);
    let is_warrior = character.flags.contains(CharacterFlags::WARRIOR);
    let is_male = character.flags.contains(CharacterFlags::MALE);
    let (px, py, pgroup) = (character.x, character.y, character.group);

    let Some((existing_type, existing_profile)) =
        runtime.player_for_character(player_id).map(|player| {
            (
                [
                    player.farmy_soldier_type(0),
                    player.farmy_soldier_type(1),
                    player.farmy_soldier_type(2),
                ],
                [
                    player.farmy_soldier_profile(0),
                    player.farmy_soldier_profile(1),
                    player.farmy_soldier_profile(2),
                ],
            )
        })
    else {
        return;
    };

    let plans = plan_soldier_recruitment(
        army_rank,
        is_warrior,
        is_male,
        existing_type,
        existing_profile,
        |max| runtime_random_below(max as i32) as u32,
    );

    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return;
    };
    for plan in plans.into_iter().flatten() {
        player.set_farmy_soldier_type(plan.slot, plan.soldier_type);
        player.set_farmy_soldier_rank(plan.slot, 1);
        player.set_farmy_soldier_profile(plan.slot, plan.profile as i32);
    }

    let mut spawned = [CharacterId(0); MAXSOLDIER];
    for slot in 0..MAXSOLDIER {
        let Some((soldier_type, profile_index, rank)) =
            runtime.player_for_character(player_id).map(|player| {
                (
                    player.farmy_soldier_type(slot),
                    player.farmy_soldier_profile(slot) as usize,
                    player.farmy_soldier_rank(slot),
                )
            })
        else {
            return;
        };
        if soldier_type == 0 {
            continue;
        }

        let Some(character_id) = spawn_army_soldier(
            world,
            loader,
            runtime,
            soldier_type,
            profile_index,
            rank,
            px,
            py,
            pgroup,
            player_id,
        ) else {
            continue;
        };
        spawned[slot] = character_id;

        let serial = world
            .characters
            .get(&character_id)
            .map(|character| character.serial)
            .unwrap_or(0);
        if let Some(player) = runtime.player_for_character_mut(player_id) {
            player.set_farmy_soldier_cn(slot, character_id.0 as i32);
            player.set_farmy_soldier_serial(slot, serial as i32);
        }
    }

    // C `fdemon.c:573-589`: rebuild `dat->platoon[]` on every soldier just
    // (re)spawned this call.
    let mut platoon = [CharacterId(0); MAXSOLDIER + 1];
    platoon[..MAXSOLDIER].copy_from_slice(&spawned);
    platoon[MAXSOLDIER] = player_id;
    for &soldier_id in &spawned {
        if soldier_id.0 == 0 {
            continue;
        }
        if let Some(CharacterDriverState::FdemonArmy(dat)) = world
            .characters
            .get_mut(&soldier_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            dat.platoon = platoon;
        }
    }
}

/// C `create_char("army1s"/"army2s", 0)` + `update_soldier(co, n, ppd)` +
/// the equipment/name/sprite/position tail of `take_soldiers`
/// (`fdemon.c:394-449,516-570`).
#[allow(clippy::too_many_arguments)]
fn spawn_army_soldier(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    soldier_type: i32,
    profile_index: usize,
    rank: i32,
    x: u16,
    y: u16,
    group: u16,
    leader_id: CharacterId,
) -> Option<CharacterId> {
    let template_key = if soldier_type == SOLDIER_TYPE_WARRIOR {
        "army1s"
    } else {
        "army2s"
    };
    let character_id = runtime.allocate_character_id();
    let Ok((mut soldier, mut inventory_items)) =
        loader.instantiate_character_template(template_key, character_id)
    else {
        return None;
    };

    // C `update_soldier`: the just-instantiated template's own `value[1]`
    // IS the marker array C reads from a throwaway second instance (see
    // `fdemon_army.rs`'s module doc comment) - no second instantiation
    // needed.
    let base = soldier_base_strength(rank);
    let markers: Vec<i32> = soldier.values[1].iter().map(|v| i32::from(*v)).collect();
    let mut scaled = markers.clone();
    scale_soldier_values(&markers, base, &mut scaled);
    for (index, value) in scaled.into_iter().enumerate() {
        soldier.values[1][index] = value as i16;
    }

    finalize_soldier_exp_and_level(&mut soldier);

    let armor_skill = i32::from(soldier.values[1][CharacterValue::ArmorSkill as usize]);
    let sword_skill = i32::from(soldier.values[1][CharacterValue::Sword as usize]);
    let dagger_skill = i32::from(soldier.values[1][CharacterValue::Dagger as usize]);
    for (slot, item_template) in
        soldier_equipment_items(soldier_type, armor_skill, sword_skill, dagger_skill)
    {
        if let Ok(item) = loader.instantiate_item_template(&item_template, Some(character_id)) {
            soldier.inventory[slot] = Some(item.id);
            inventory_items.push(item);
        }
    }

    if let Some(profile) = SOLDIER_PROFILES.get(profile_index) {
        soldier.name = profile.name.to_string();
        if profile.gender == 'M' {
            soldier.flags.insert(CharacterFlags::MALE);
        } else {
            soldier.flags.insert(CharacterFlags::FEMALE);
        }
        soldier.sprite = if soldier_type == SOLDIER_TYPE_WARRIOR {
            profile.sprite
        } else {
            profile.sprite + 1
        };
    }

    soldier.dir = Direction::RightDown as u8;
    soldier.group = group;
    // C `set_army_rank(co, ppd->soldier[n].rank)`: no separate persisted
    // rank field for arbitrary characters in this port - army rank is
    // derived from `military_points` everywhere else in this codebase
    // (see `army_rank_for_points`'s own doc comment), so `rank^3` is the
    // `military_points` value that formula derives back to `rank` from.
    // Same "set_army_rank via military_points" precedent as `area3::
    // seymour`'s `set_army_rank(co, 1)` deviation note.
    soldier.military_points = rank.max(1).pow(3);
    soldier.driver = CDR_FDEMON_ARMY;
    // C `take_soldiers`: `ppd->soldier[n].emote.boredom = 0; ppd->soldier[n]
    // .emote.fear = 0; ppd->soldier[n].emote.praise = 0; dat->emote = ppd->
    // soldier[n].emote;` (`fdemon.c:559-563`) - a freshly (re-)spawned
    // soldier's four base personality tendencies come from its assigned
    // profile ([`assign_profile`]); the "current"/relationship fields all
    // start at `0` (documented cross-recruit-cycle gap, see `fdemon_army_
    // emote.rs`'s own module doc comment - this port doesn't yet persist
    // `ppd->soldier[n].emote` to carry those over instead).
    let emote_base = assign_profile(profile_index);
    soldier.driver_state = Some(CharacterDriverState::FdemonArmy(FarmyData {
        leader_cn: leader_id,
        mission: MIS_FOLLOW,
        emote: ugaris_core::world::npc::area8::fdemon_army_emote::SoldierEmote {
            cuddly: emote_base.cuddly,
            angst: emote_base.angst,
            bore: emote_base.bore,
            bigmouth: emote_base.bigmouth,
            ..Default::default()
        },
        ..FarmyData::default()
    }));
    // C `fdemon_army`'s own `NT_CREATE` handler: `fight_driver_set_dist(cn,
    // 0, 20, 0)` (`fdemon.c:1346`) - seeds the driver-independent
    // `DRD_FIGHTDRIVER` slot's distance config so the combat fallback's
    // `fight_driver_add_enemy`-based sighting/self-defense (`world::npc::
    // area8::fdemon_army_combat`) has somewhere to record enemies at all;
    // same "seed `DRD_FIGHTDRIVER` from a driver's own NT_CREATE args"
    // precedent `apply_simple_baddy_create_message` already establishes.
    soldier.fight_driver = Some(FightDriverData {
        start_dist: 0,
        char_dist: 20,
        stop_dist: 0,
        ..FightDriverData::default()
    });

    if !world.spawn_character(soldier, usize::from(x), usize::from(y)) {
        return None;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.update_character(character_id);
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
        character.endurance =
            i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
        character.mana = i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    }
    Some(character_id)
}

/// C `ch_driver`'s `CDR_FDEMON_ARMY` case, run once per live soldier per
/// tick: the `NT_TEXT`/`NT_GOTHIT`/`NT_SEEHIT` message handling
/// (`world::npc::area8::fdemon_army_combat::fdemon_army_process_messages`,
/// C `fdemon_army`'s message loop, `fdemon.c:1338-1431`) followed by the
/// mission-dispatch/self-defense/leader-lost tick
/// (`world::npc::area8::fdemon_army::fdemon_army_tick`, C `fdemon.c:1433-
/// 1532`) - matching C's own per-character ordering (message loop first,
/// then "do something"). See `fdemon_army_tick`'s own doc comment for the
/// deferred emote/soldier-exp portions.
pub(crate) fn apply_fdemon_army_tick(world: &mut World, area_id: u16) -> usize {
    let mut disintegrated = 0;
    for character_id in world.fdemon_army_character_ids() {
        world.fdemon_army_process_messages(character_id);
        if world.fdemon_army_tick(character_id, area_id) {
            disintegrated += 1;
        }
    }
    disintegrated
}
