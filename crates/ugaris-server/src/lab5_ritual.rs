//! `ugaris-server`-side `ZoneLoader`-touching half of the Lab 5 mage's
//! force-summon ritual (`ugaris_core::world::npc::area22::lab5_mage`):
//! spawning the planned demons and finishing the ritual attempt. See that
//! module's own doc comment for why this is split out of `World`.

use super::*;
use ugaris_core::world::npc::area22::lab5_mage::{Lab5RitualDemonSpawn, Lab5RitualPlan};

/// C `ritual_create_char` (`lab5.c:131-168`), the character-instantiation
/// half `World::attempt_ritual_start` cannot do itself (needs
/// `ZoneLoader`). Mirrors `mine::spawn_normal_golem`/`area8_army::
/// spawn_army_soldier`'s instantiate-then-place-then-recompute pattern.
fn ritual_create_char(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    spawn: &Lab5RitualDemonSpawn,
) {
    let character_id = runtime.allocate_character_id();
    let Ok((mut character, inventory_items)) =
        loader.instantiate_character_template(spawn.template, character_id)
    else {
        return;
    };
    // C `ch[cn].dir = dir;` (`lab5.c:141`).
    character.dir = spawn.dir;
    // C `ch[cn].flags &= ~CF_RESPAWN;` (`lab5.c:142`).
    character.flags.remove(CharacterFlags::RESPAWN);
    if !world.spawn_character(character, spawn.x as usize, spawn.y as usize) {
        return;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    // C `update_char(cn);` (`lab5.c:143`).
    world.update_character(character_id);
    if let Some(character) = world.characters.get_mut(&character_id) {
        // C `ch[cn].hp/endurance/mana = value[0][...] * POWERSCALE;`
        // (`lab5.c:145-147`).
        character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
        character.endurance =
            i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
        character.mana = i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
        // C `ch[cn].tmpx = ch[cn].x; ch[cn].tmpy = ch[cn].y;` (`lab5.c:
        // 159-160`) - `tmpx`/`tmpy` reuse `rest_x`/`rest_y`, same
        // substitution `world::npc::area22::lab5_daemon`'s own
        // `secure_move_driver` call already established.
        character.rest_x = character.x;
        character.rest_y = character.y;
    }
    // C `dat = set_data(cn, DRD_LAB5_DAEMON, ...); dat->dir = dir;
    // dat->attackstart = attackstart * TICKS;` (`lab5.c:163-167`) - the
    // template instantiation above already installed a `Lab5Daemon`
    // driver state (from the template's own `arg="type=N;"`, via
    // `apply_lab5_daemon_create_message`) with `attackstart: 0`; override
    // `dir`/`attackstart` to the ritual's own values here. The character's
    // own first-tick `NT_CREATE` (already queued by that same apply
    // function) then converts this relative `attackstart` into an
    // absolute deadline via `+= ticker`, exactly like the always-present
    // zone-spawned demons do - see `world::npc::area22::lab5_daemon`'s
    // module doc comment.
    if let Some(CharacterDriverState::Lab5Daemon(data)) = world
        .characters
        .get_mut(&character_id)
        .and_then(|character| character.driver_state.as_mut())
    {
        data.dir = spawn.dir;
        data.attackstart = (spawn.attackstart_seconds.max(0) as u64) * TICKS_PER_SECOND;
    }
}

/// Applies `ugaris_core::world::Lab5MageOutcomeEvent::
/// AttemptRitualStart`: spawns every planned demon, then calls
/// `World::finish_ritual_start` to do C's own spawn-before-teleport-check
/// tail, writing `PlayerRuntime::lab5_ritual_state` back to `0` only on
/// success (matching C's `ritual_start`'s own conditional `pd->
/// ritualstate = 0;`).
pub(crate) fn apply_ritual_start(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    player_id: CharacterId,
    mage_id: CharacterId,
    plan: &Lab5RitualPlan,
) -> bool {
    for spawn in &plan.spawns {
        ritual_create_char(world, loader, runtime, spawn);
    }
    let success =
        world.finish_ritual_start(player_id, mage_id, plan.door_x, plan.door_y, plan.daemon);
    if success {
        if let Some(player) = runtime.player_for_character_mut(player_id) {
            player.lab5_ritual_state = 0;
        }
    }
    success
}
