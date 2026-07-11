//! Server-side wiring for the Area 32 governor job-board NPC
//! (`CDR_MISSIONGIVE`, "Mister Jones",
//! `ugaris_core::world::npc::area32::governor::process_mission_giver_actions`).
//!
//! Mirrors `area29.rs`'s `apply_countbran_events`/`apply_daughterbran_events`
//! shape: `apply_mission_giver_events` needs `loader` (generic reward-item
//! creation) and `legacy_item_look_text` (reward preview), both
//! `ugaris-server`-only capabilities `ugaris-core`'s `World` cannot reach -
//! see `governor`'s module doc comment for the full ported/remaining slice
//! breakdown.

use std::collections::HashMap;

use super::*;
use ugaris_core::character_driver::{apply_simple_baddy_create_message, CDR_MISSIONFIGHT};
use ugaris_core::world::calc_exp;
use ugaris_core::world::npc::area32::governor::{
    MissionGiveOutcomeEvent, MissionGivePlayerFacts, MISSION_TEMPLATES, MIS_REWARDS,
};
use ugaris_core::world::npc::area32::mission_start::{
    build_fighter_stat_values, mission_status_lines, special_item_tier_for_level,
    try_solve_mission, FighterSpawnSpec, MISSION_FIGHTER_DATA,
};

pub(crate) fn mission_giver_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, MissionGivePlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                MissionGivePlayerFacts {
                    ppd: player.governor,
                },
            ))
        })
        .collect()
}

/// Applies each [`MissionGiveOutcomeEvent`] queued by `World::
/// process_mission_giver_actions`. `UpdatePpd` is always applied first
/// within a single event batch (see that function's own doc comment on
/// why event order matters here): `GiveItemReward`'s own point deduction
/// mutates `PlayerRuntime` directly, since it isn't known whether the
/// generic item-template create/give will even succeed until this
/// function runs.
pub(crate) fn apply_mission_giver_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<MissionGiveOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            MissionGiveOutcomeEvent::UpdatePpd { player_id, ppd } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.governor = ppd;
                applied += 1;
            }
            // C `mission_show_reward`'s generic branch (`missions.c:1272-
            // 1287`): `create_item`+`look_item`+`destroy_item`, then the
            // trailing "This could be yours for..." line.
            MissionGiveOutcomeEvent::ShowItemReward {
                player_id,
                npc_id,
                reward_index,
            } => {
                let Some(reward) = MIS_REWARDS.get(reward_index) else {
                    continue;
                };
                let Some(viewer) = world.characters.get(&player_id).cloned() else {
                    continue;
                };
                let Ok(item) = loader.instantiate_item_template(reward.itmtmp, Some(player_id))
                else {
                    world.npc_quiet_say(
                        npc_id,
                        "Oops. I've run out of stock. Please choose something else.",
                    );
                    continue;
                };
                for line in legacy_item_look_text(&item, &viewer).lines() {
                    world.queue_system_text(player_id, line.to_string());
                }
                let points = runtime
                    .player_for_character(player_id)
                    .map(|player| player.governor.points)
                    .unwrap_or(0);
                world.npc_quiet_say(
                    npc_id,
                    &format!(
                        "This could be yours for {} points (you have {points} points). Say ibuy {} to buy it.",
                        reward.value, reward.code
                    ),
                );
                applied += 1;
            }
            // C `mission_give_reward`'s generic branch (`missions.c:1212-
            // 1237`): `create_item`, `IF_BONDTAKE` owner stamping,
            // `give_char_item`, and only on success the point deduction +
            // "here you go" line.
            MissionGiveOutcomeEvent::GiveItemReward {
                player_id,
                npc_id,
                reward_index,
            } => {
                let Some(reward) = MIS_REWARDS.get(reward_index) else {
                    continue;
                };
                let Ok(mut item) = loader.instantiate_item_template(reward.itmtmp, Some(player_id))
                else {
                    world.npc_quiet_say(
                        npc_id,
                        "Oops. I've run out of stock. Please choose something else.",
                    );
                    continue;
                };
                if item.flags.contains(ItemFlags::BONDTAKE) {
                    item.owner_id = player_id.0 as i32;
                }
                let item_id = item.id;
                world.add_item(item);
                if !world.give_char_item(player_id, item_id) {
                    world.destroy_item(item_id);
                    world.npc_quiet_say(
                        npc_id,
                        "Hey, sleepy head, there's no room in your hand or inventory to give you an item!",
                    );
                    continue;
                }
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.governor.points -= reward.value;
                let points_left = player.governor.points;
                let Some(character) = world.characters.get(&player_id) else {
                    continue;
                };
                let player_name = character.name.clone();
                world.npc_quiet_say(
                    npc_id,
                    &format!(
                        "Here you go, {player_name}, one {} ({}) for {} points. You now have {points_left} points left.",
                        reward.code, reward.desc, reward.value
                    ),
                );
                applied += 1;
            }
            // C `start_mission`'s `build_fighter` calls (`missions.c:
            // 1030-1115`).
            MissionGiveOutcomeEvent::SpawnMissionFighters { fighters } => {
                for spec in &fighters {
                    spawn_mission_fighter(world, loader, runtime, spec);
                }
                applied += 1;
            }
        }
    }
    applied
}

/// C `build_fighter` (`missions.c:678-865`): instantiate the fighter's
/// base template, rescale its raisable skills for `spec.diff`
/// ([`build_fighter_stat_values`]), overwrite name/description/sprite/
/// flags, attach the `mis_key`/big-boss special item/`armor_spell`/
/// `weapon_spell` items, finalize exp/level, and drop it on the map.
///
/// C's `mission_fighter_driver`'s own dispatch is an unconditional tail
/// call to `char_driver(CDR_SIMPLEBADDY, ...)` (`missions.c:1849-1851`) -
/// same "reuse SimpleBaddy AI wholesale, keep a distinguishable driver id
/// only for the death hook" precedent as `CDR_PENTER`/`CDR_WARPFIGHTER`
/// (`zone.rs`'s template-instantiation special cases): the spawned
/// fighter's own `driver` is `CDR_MISSIONFIGHT`, not `CDR_SIMPLEBADDY`
/// directly, so `world_events::death_hooks::
/// apply_mission_fighter_death_from_hurt_event` (`mission_fighter_dead`,
/// `missions.c:1852-1881`) can tell a mission fighter apart from any
/// other SimpleBaddy-driven NPC. The SimpleBaddy AI gates in
/// `world/npc_fight.rs`/`world/npc_idle.rs` are widened to also accept
/// `CDR_MISSIONFIGHT`, same as every other driver on that list.
pub(crate) fn spawn_mission_fighter(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    spec: &FighterSpawnSpec,
) -> bool {
    let character_id = runtime.allocate_character_id();
    let Ok((mut fighter, mut inventory_items)) =
        loader.instantiate_character_template(spec.temp, character_id)
    else {
        return false;
    };

    let simple_baddy_args = loader
        .character_templates
        .get(spec.temp)
        .map(|template| template.args.clone());
    fighter.driver = CDR_MISSIONFIGHT;
    fighter.push_driver_message(ugaris_core::character_driver::NT_CREATE, 0, 0, 0);
    apply_simple_baddy_create_message(&mut fighter, simple_baddy_args.as_deref(), 0);

    let markers = fighter.values[1].clone();
    fighter.values[1] = build_fighter_stat_values(&markers, spec.diff);

    fighter.x = spec.x;
    fighter.y = spec.y;
    fighter.rest_x = spec.x;
    fighter.rest_y = spec.y;
    fighter.dir = Direction::RightDown as u8;
    fighter.deaths = u32::from(spec.fighter_kind);
    fighter.sprite = spec.sprite;
    fighter.flags.insert(spec.extra_flags);
    fighter.name = spec.name.clone();
    fighter.description = spec.desc.to_string();

    fighter.exp = calc_exp(&fighter);
    fighter.exp_used = fighter.exp;
    fighter.level = ugaris_core::world::exp2level(fighter.exp);
    if (spec.diff > 100 && fighter.level < 10) || fighter.level > 200 {
        fighter.level = 200;
    }

    if spec.key_id != 0 {
        if let Ok(mut key_item) = loader.instantiate_item_template("mis_key", Some(character_id)) {
            key_item.template_id = spec.key_id;
            key_item.name = spec.key_name.to_string();
            fighter.inventory[30] = Some(key_item.id);
            inventory_items.push(key_item);
        }
    }

    if spec.has_special_item {
        let (strength, base) = special_item_tier_for_level(fighter.level as i32);
        if let Some(mut special_item) = world.create_special_item(loader, strength, base, 1, 10000)
        {
            special_item.carried_by = Some(character_id);
            fighter.inventory[31] = Some(special_item.id);
            inventory_items.push(special_item);
        }
    }

    if let Ok(mut armor) = loader.instantiate_item_template("armor_spell", Some(character_id)) {
        let armor_skill = i32::from(fighter.values[1][CharacterValue::ArmorSkill as usize]);
        armor.modifier_value[0] = (armor_skill.clamp(13, 113) * 20) as i16;
        fighter.inventory[14] = Some(armor.id);
        inventory_items.push(armor);
    }
    if let Ok(mut weapon) = loader.instantiate_item_template("weapon_spell", Some(character_id)) {
        let hand_skill = i32::from(fighter.values[1][CharacterValue::Hand as usize]);
        weapon.modifier_value[0] = hand_skill.clamp(13, 113) as i16;
        fighter.inventory[15] = Some(weapon.id);
        inventory_items.push(weapon);
    }

    if !world.spawn_character(fighter, usize::from(spec.x), usize::from(spec.y)) {
        return false;
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
    true
}

/// Outcome of [`apply_mission_chest_open`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MissionChestApplyResult {
    /// The reward item was created and placed on the cursor. `status_lines`
    /// is C `mission_status`'s re-printed HUD (`missions.c:1842`);
    /// `solved_message` is `mission_done`'s "You've finished the job..."
    /// line, only present the one time this call flips the job from
    /// `active` to `solved` (`try_solve_mission`'s own `bool` return).
    Granted {
        item_name: String,
        key_name: Option<String>,
        status_lines: Vec<String>,
        solved_message: Option<String>,
    },
    /// C `if (!md->itemtemp) { ... "The chest is empty." ... }`
    /// (`missions.c:1806-1809`).
    Empty,
    /// C's "You need a key to open this chest." branch (`:1821`).
    KeyRequired,
    /// C's "Please empty your 'hand' (mouse cursor) first." branch
    /// (`:1829`). `key_name` is `Some` in the one real C quirk this
    /// preserves: if the only carried copy of the required key sits on
    /// the cursor itself, C still prints the "You use ... to unlock the
    /// chest." line (the key search/unlock message runs *before* the
    /// cursor-occupied check, `missions.c:1811-1831`) even though the very
    /// same non-empty cursor then blocks the reward item a few lines
    /// later.
    CursorOccupied {
        key_name: Option<String>,
    },
    MissingPlayer,
}

/// C `missionchest_driver` (`missions.c:1790-1847`), minus the `if (!cn)
/// return;` guard already applied by the pure
/// `item_driver::area32_missions::missionchest_driver` gate. Needs both
/// the acting player's `governor: MissionPpd` (to resolve `mdtab[ppd->
/// md_idx]` and write `find_item[0]`/re-run `mission_status`/
/// `mission_done`) and a `ZoneLoader` (`create_item`), neither of which
/// `ugaris-core`'s pure item drivers can reach - see
/// `ItemDriverOutcome::MissionChestOpen`'s own doc comment.
pub(crate) fn apply_mission_chest_open(
    world: &mut World,
    loader: &mut ZoneLoader,
    player: Option<&mut PlayerRuntime>,
    item_id: ItemId,
    character_id: CharacterId,
) -> MissionChestApplyResult {
    let Some(player) = player else {
        return MissionChestApplyResult::MissingPlayer;
    };
    let mut ppd = player.governor;
    let md_idx = ppd.md_idx.clamp(0, MISSION_FIGHTER_DATA.len() as i32 - 1) as usize;
    let md = &MISSION_FIGHTER_DATA[md_idx];
    let Some(itemtemp) = md.itemtemp else {
        return MissionChestApplyResult::Empty;
    };

    // C `if (it[in].drdata[1] || it[in].drdata[2]) { ... }`
    // (`missions.c:1811-1826`): `chest_required_key_id` reads the full
    // little-endian `u32` at `drdata[1..5]` rather than just its low two
    // bytes, a harmless deviation in practice - `start_mission` only ever
    // writes small `DEV_ID_MISSION`-prefixed key IDs whose low 16 bits are
    // nonzero for any realistic `mcnt`.
    let required_key_id = world
        .items
        .get(&item_id)
        .map(chest_required_key_id)
        .unwrap_or_default();
    let key_name = if required_key_id != 0 {
        match exact_carried_door_key_access(world, character_id, required_key_id) {
            Some(access) => Some(access.name),
            None => return MissionChestApplyResult::KeyRequired,
        }
    } else {
        None
    };

    if world
        .characters
        .get(&character_id)
        .is_none_or(|character| character.cursor_item.is_some())
    {
        return MissionChestApplyResult::CursorOccupied { key_name };
    }

    let Ok(mut item) = loader.instantiate_item_template(itemtemp, Some(character_id)) else {
        return MissionChestApplyResult::Empty;
    };
    item.name = md.itemname.unwrap_or_default().to_string();
    item.description = md.itemdesc.unwrap_or_default().to_string();
    let item_id_new = item.id;
    let item_name = item.name.clone();

    let Some(character) = world.characters.get_mut(&character_id) else {
        return MissionChestApplyResult::MissingPlayer;
    };
    if character.cursor_item.is_some() {
        return MissionChestApplyResult::CursorOccupied { key_name };
    }
    character.cursor_item = Some(item_id_new);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(item);

    ppd.find_item[0] = 1;
    let title = MISSION_TEMPLATES[md_idx].title;
    let status_lines = mission_status_lines(&ppd, title, md);

    let solved_message = if try_solve_mission(&mut ppd) {
        let killer_name = world
            .characters
            .get(&character_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        Some(format!(
            "You've finished the job. Good work, {killer_name}. Now talk to Mr. Jones for your reward."
        ))
    } else {
        None
    };
    player.governor = ppd;

    MissionChestApplyResult::Granted {
        item_name,
        key_name,
        status_lines,
        solved_message,
    }
}
