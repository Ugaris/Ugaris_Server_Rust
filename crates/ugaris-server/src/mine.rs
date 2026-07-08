//! Server-side glue for Area 12's (`src/area/12/mine.c`) diggable-wall
//! reward cascade (`handle_mining_result`, wired from
//! `tick_item_use_minewall::dispatch_minewall_outcome` whenever a
//! `MineWallDig` outcome carries `opened: true`): the parts that need
//! `ZoneLoader` (instantiating "silver"/"gold" stack items and golem
//! characters) or `PlayerRuntime`/achievement-repository access (military
//! mission silver tracking, the mined-amount achievement ladders). The
//! pure event-roll/amount-roll/cave-in math lives in `ugaris-core`'s
//! `world/mining.rs`.
//!
//! The orb (`handle_orb_find`) and artifact-relic (`handle_artifact_find`)
//! branches are not ported yet - see `PORTING_TODO.md`'s Area 12 entry.

use super::*;
use ugaris_core::legacy::INVENTORY_START_INVENTORY;
use ugaris_core::world::{CaveInResult, MilitaryMissionSilverProgress, MiningEvent};

/// C `handle_mining_result`'s dispatch tail (`mine.c:229-275`): rolls the
/// weighted event and applies whichever branch fired. Called once per
/// wall that just reached `drdata[3] == 8` (`opened: true`).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_mine_wall_reward(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    match world.roll_mining_event() {
        MiningEvent::Silver => {
            apply_mine_silver_find(
                world,
                zone_loader,
                runtime,
                achievement_repository,
                item_id,
                character_id,
                feedback,
            )
            .await;
        }
        MiningEvent::Gold => {
            apply_mine_gold_find(
                world,
                zone_loader,
                runtime,
                achievement_repository,
                item_id,
                character_id,
                feedback,
            )
            .await;
        }
        MiningEvent::Golem => {
            apply_mine_golem_spawn(world, zone_loader, runtime, item_id, character_id, feedback);
        }
        MiningEvent::CaveIn => {
            apply_mine_cave_in(world, item_id, character_id, feedback);
        }
        // C `handle_orb_find`/`handle_artifact_find` - not ported yet
        // (see this module's doc comment); C's own fall-through
        // ("nothing of value") is also a documented no-op.
        MiningEvent::Orb | MiningEvent::Artifact | MiningEvent::Nothing => {}
    }
}

/// C `handle_silver_find` (`mine.c:290-302`).
async fn apply_mine_silver_find(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    let Some(amount) = world.roll_mining_silver_amount(item_id, character_id) else {
        return;
    };
    let amount = amount.max(0) as u32;
    let Some(result) = give_mine_item(
        world,
        zone_loader,
        character_id,
        StackKind::SilverUnit,
        amount,
    ) else {
        return;
    };
    push_give_mine_item_feedback(feedback, character_id, &result);
    apply_military_mission_silver_check(world, runtime, character_id, amount as i32);
    award_silver_mined_achievement(world, runtime, achievement_repository, character_id, amount)
        .await;
}

/// C `handle_gold_find` (`mine.c:304-318`): identical to the silver
/// branch except gated on `amount > 0` and counting gold double for the
/// military mission check (`check_military_silver(cn, amount * 2)`).
async fn apply_mine_gold_find(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    let Some(amount) = world.roll_mining_gold_amount(item_id, character_id) else {
        return;
    };
    if amount <= 0 {
        return;
    }
    let amount = amount as u32;
    let Some(result) = give_mine_item(
        world,
        zone_loader,
        character_id,
        StackKind::GoldUnit,
        amount,
    ) else {
        return;
    };
    push_give_mine_item_feedback(feedback, character_id, &result);
    apply_military_mission_silver_check(world, runtime, character_id, (amount * 2) as i32);
    award_gold_mined_achievement(world, runtime, achievement_repository, character_id, amount)
        .await;
}

/// C `handle_cave_in` (`mine.c:362-403`).
pub(crate) fn apply_mine_cave_in(
    world: &mut World,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    let Some(result) = world.apply_mining_cave_in(item_id, character_id) else {
        return;
    };
    match result {
        CaveInResult::Avoided => {
            feedback.push((
                character_id,
                "Your mining expertise helped you avoid a cave-in!".to_string(),
            ));
        }
        CaveInResult::Collapsed {
            endurance_loss_units,
            unreduced_loss_units,
            now_exhausted,
        } => {
            let message = if let Some(unreduced) = unreduced_loss_units {
                format!(
                    "The mine wall suddenly collapses! Thanks to your athletic prowess, you swiftly escape, losing only {endurance_loss_units} endurance instead of {unreduced}!"
                )
            } else {
                format!(
                    "The mine wall suddenly collapses! You barely escape, losing {endurance_loss_units} endurance!"
                )
            };
            feedback.push((character_id, message));
            if now_exhausted {
                feedback.push((
                    character_id,
                    "You're exhausted from the cave-in. Be careful!".to_string(),
                ));
            }
        }
    }
}

/// Result of [`give_mine_item`] (C `give_mine_item`, `mine.c:506-557`).
#[derive(Debug)]
pub(crate) enum GiveMineItemResult {
    /// Merged into an existing pile in the digger's inventory
    /// (`give_mine_item`'s "found a same pile" branch).
    MergedIntoPile {
        amount: u32,
        total: u32,
        name: String,
    },
    /// Placed fresh on the digger's cursor (either the 2%-chance roll, or
    /// no matching pile was found).
    Cursor { amount: u32, name: String },
}

fn push_give_mine_item_feedback(
    feedback: &mut Vec<(CharacterId, String)>,
    character_id: CharacterId,
    result: &GiveMineItemResult,
) {
    match result {
        GiveMineItemResult::MergedIntoPile {
            amount,
            total,
            name,
        } => {
            feedback.push((
                character_id,
                format!(
                    "You found {amount} units of {name}. You now have a total of {total} {name} units."
                ),
            ));
        }
        GiveMineItemResult::Cursor { amount, name } => {
            feedback.push((character_id, format!("You found {amount} units of {name}.")));
        }
    }
}

/// C `give_mine_item(in, cn, item_name, amount)` (`mine.c:506-557`):
/// creates a fresh "silver"/"gold" stack item worth `amount` units, then
/// either merges it into a matching pile already in the digger's
/// inventory (slots `INVENTORY_START_INVENTORY..`) or places it on their
/// (guaranteed-empty, since digging requires an empty cursor) cursor -
/// with a 2% chance to always go straight to the cursor regardless of any
/// existing pile. Returns `None` only if the "silver"/"gold" template
/// fails to instantiate (C: `elog` + early return, no
/// `check_military_silver`/achievement tail).
pub(crate) fn give_mine_item(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    kind: StackKind,
    amount: u32,
) -> Option<GiveMineItemResult> {
    let add_to_cursor = world.roll_legacy_random(100) < 2;

    let mut new_item = loader
        .instantiate_item_template(stack_template(kind), Some(character_id))
        .ok()?;
    new_item.value = new_item.value.saturating_mul(amount);
    set_stack_count(&mut new_item, amount, kind);
    let new_item_id = new_item.id;
    let item_name = new_item.name.clone();

    if !add_to_cursor {
        let existing = world.characters.get(&character_id)?.inventory[INVENTORY_START_INVENTORY..]
            .iter()
            .flatten()
            .find(|id| {
                world
                    .items
                    .get(id)
                    .is_some_and(|item| stack_kind(item) == Some(kind))
            })
            .copied();
        if let Some(existing_id) = existing {
            let added_value = new_item.value;
            let existing_item = world.items.get_mut(&existing_id)?;
            existing_item.value = existing_item.value.saturating_add(added_value);
            let total = stack_count(existing_item).saturating_add(amount);
            set_stack_count(existing_item, total, kind);
            let name = existing_item.name.clone();
            if let Some(character) = world.characters.get_mut(&character_id) {
                character.flags.insert(CharacterFlags::ITEMS);
            }
            // C `destroy_item(in2)`: the freshly-created item is never
            // inserted into the world at all.
            return Some(GiveMineItemResult::MergedIntoPile {
                amount,
                total,
                name,
            });
        }
    }

    let character = world.characters.get_mut(&character_id)?;
    character.cursor_item = Some(new_item_id);
    character.flags.insert(CharacterFlags::ITEMS);
    world.add_item(new_item);
    Some(GiveMineItemResult::Cursor {
        amount,
        name: item_name,
    })
}

/// C `check_military_silver(cn, amount)` (`mine.c:102-134`), applied
/// after a successful silver/gold grant: resends the questlog display
/// (`sendquestlog`) whenever there's any active unsolved mission, and
/// pushes the C `log_char` progress/completion text via the queued
/// system-text mechanism (matching `military::apply_military_mission_
/// kill_check`'s established pattern for this same PPD).
pub(crate) fn apply_military_mission_silver_check(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    amount: i32,
) {
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    let outcome = player.check_military_silver(amount);
    let questlog_payload = (!matches!(outcome, MilitaryMissionSilverProgress::NoMission))
        .then(|| legacy_questlog_payload(player));

    let message: Option<&'static str> = match outcome {
        MilitaryMissionSilverProgress::NoMission
        | MilitaryMissionSilverProgress::NotSilverMission => None,
        MilitaryMissionSilverProgress::Progress { .. } => None,
        MilitaryMissionSilverProgress::Solved => {
            Some("You solved your mission. Talk to the governor to claim your reward.")
        }
    };

    if let Some(payload) = questlog_payload {
        for (session_id, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
    if let MilitaryMissionSilverProgress::Progress { remaining } = outcome {
        world.queue_system_text(
            character_id,
            format!("You fulfilled part of your mission, you still need {remaining} silver."),
        );
    } else if let Some(message) = message {
        world.queue_system_text(character_id, message.to_string());
    }
}

/// C `handle_golem_spawn` (`mine.c:320-326`).
pub(crate) fn apply_mine_golem_spawn(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    character_id: CharacterId,
    feedback: &mut Vec<(CharacterId, String)>,
) {
    let golem_type = world
        .items
        .get(&item_id)
        .and_then(|item| item.driver_data.get(2).copied())
        .unwrap_or_default();
    if world.roll_mining_golem_rare() {
        if spawn_rare_golem(world, loader, runtime, item_id, golem_type) {
            feedback.push((
                character_id,
                "A rare, stronger golem has appeared!".to_string(),
            ));
        }
    } else {
        spawn_normal_golem(world, loader, runtime, item_id, golem_type);
    }
}

/// Shared silver-vs-gold drop template selection
/// (`spawn_normal_golem`/`spawn_rare_golem`'s `it[in].drdata[2] <=
/// get_max_silver_golem_type()` check, `mine.c:589,627`).
fn golem_drop_stack_kind(world: &World, golem_type: u8) -> StackKind {
    if i32::from(golem_type) <= world.settings.max_silver_golem_type {
        StackKind::SilverUnit
    } else {
        StackKind::GoldUnit
    }
}

/// Golem loot-drop instantiation shared by [`spawn_normal_golem`]/
/// [`spawn_rare_golem`] (`mine.c:585-601`/`:623-639`): places a fresh
/// silver/gold stack item worth `amount` directly into the golem's
/// `golem_inventory_slot`.
fn grant_golem_drop(
    world: &mut World,
    loader: &mut ZoneLoader,
    character_id: CharacterId,
    golem_type: u8,
    amount: u32,
) {
    let kind = golem_drop_stack_kind(world, golem_type);
    let Ok(mut drop_item) =
        loader.instantiate_item_template(stack_template(kind), Some(character_id))
    else {
        return;
    };
    drop_item.value = amount;
    set_stack_count(&mut drop_item, amount, kind);
    let drop_item_id = drop_item.id;
    let slot = world.settings.golem_inventory_slot.max(0) as usize;
    let Some(character) = world.characters.get_mut(&character_id) else {
        return;
    };
    if slot >= character.inventory.len() {
        return;
    }
    character.inventory[slot] = Some(drop_item_id);
    world.add_item(drop_item);
}

/// C `spawn_normal_golem` (`mine.c:571-607`).
pub(crate) fn spawn_normal_golem(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    golem_type: u8,
) -> bool {
    let Some((x, y)) = world.items.get(&item_id).map(|item| (item.x, item.y)) else {
        return false;
    };
    let template = format!("miner{golem_type}");
    let character_id = runtime.allocate_character_id();
    let Ok((mut golem, inventory_items)) =
        loader.instantiate_character_template(&template, character_id)
    else {
        return false;
    };
    golem.dir = Direction::RightDown as u8;
    golem.hp = i32::from(golem.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
    golem.endurance = i32::from(golem.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
    golem.mana = i32::from(golem.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
    let level = golem.level;
    if !world.spawn_character(golem, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }

    if (world.roll_legacy_random(100) as i32) < world.settings.normal_drop_chance {
        let amount = world
            .calculate_golem_drop_amount(level as i32, false)
            .max(0) as u32;
        grant_golem_drop(world, loader, character_id, golem_type, amount);
    }
    true
}

/// C `spawn_rare_golem` (`mine.c:609-647`): unlike the normal variant,
/// only `hp` is explicitly (re-)set (scaled by both `POWERSCALE` and
/// `get_rare_golem_hp_multiplier()`) - `endurance`/`mana` are left at
/// whatever `create_char`/`instantiate_character_template` already
/// initialized them to (the template's full `value[0][V_ENDURANCE]`/
/// `value[0][V_MANA]`, same as the normal golem's redundant explicit
/// assignment would produce anyway). The level boost is applied *after*
/// `hp` is computed from the template's pre-boost `value[0][V_HP]` (a
/// real C quirk: `update_char`/value recompute is never re-run after the
/// level bump, so the boosted level has no effect on the golem's own
/// combat stats here - only on `calculate_drop_amount`'s loot roll,
/// which reads the already-boosted `ch[co].level`).
pub(crate) fn spawn_rare_golem(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    item_id: ItemId,
    golem_type: u8,
) -> bool {
    let Some((x, y)) = world.items.get(&item_id).map(|item| (item.x, item.y)) else {
        return false;
    };
    let template = format!("miner{golem_type}");
    let character_id = runtime.allocate_character_id();
    let Ok((mut golem, inventory_items)) =
        loader.instantiate_character_template(&template, character_id)
    else {
        return false;
    };
    golem.dir = Direction::RightDown as u8;
    golem.hp = i32::from(golem.values[0][CharacterValue::Hp as usize])
        * POWERSCALE
        * world.settings.rare_golem_hp_multiplier;
    golem.level = golem
        .level
        .saturating_add(world.settings.rare_golem_level_boost.max(0) as u32);
    let level = golem.level;
    if !world.spawn_character(golem, usize::from(x), usize::from(y)) {
        return false;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }

    if (world.roll_legacy_random(100) as i32) < world.settings.rare_drop_chance {
        let amount = world.calculate_golem_drop_amount(level as i32, true).max(0) as u32;
        grant_golem_drop(world, loader, character_id, golem_type, amount);
    }
    true
}
