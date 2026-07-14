//! The giant admin `/command` multiplexer (`src/system/command.c` god
//! sections): `apply_admin_character_command` keeps its original prelude
//! and checks each command-family module in the original top-to-bottom
//! order; each `dispatch_*` is a verbatim slice of the pre-split `if`
//! chain (`ControlFlow::Break` = the original `return`).

mod anticheat;
mod diagnostics;
mod dungeon_tunnel;
mod exp_military;
mod flags_clan;
mod inspection;
mod macro_ac;
mod moderation;
mod movement;
mod pentagram;
mod ppd_misc;
mod progression;

pub(crate) use movement::*;
pub(crate) use pentagram::*;

use std::ops::ControlFlow;

use super::*;

pub(crate) fn apply_admin_character_command(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    command: &str,
    area_id: u32,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();

    if let Some(result) = if world
        .characters
        .get(&character_id)
        .is_some_and(|caller| caller.flags.contains(CharacterFlags::GOD))
    {
        apply_legacy_tick_tuning_command(runtime, &lower, rest)
            .or_else(|| apply_legacy_communication_tuning_command(runtime, &lower, rest))
            .or_else(|| apply_legacy_game_settings_tuning_command(world, &lower, rest))
            .or_else(|| apply_global_settings_command(world, &lower))
    } else {
        None
    } {
        return Some(result);
    }

    if matches!(
        lower.as_str(),
        "setdecaytime"
            | "setplayerbodytime"
            | "setnpcbodytime"
            | "setnpcbodytimearea32"
            | "setrespawntime"
            | "setsewerrespawntime"
            | "setlagouttime"
            | "setregentime"
            | "sethollerdist"
            | "setshoutdist"
            | "setsaydist"
            | "setemotedist"
            | "setquietsaydist"
            | "setwhisperdist"
            | "sethollercost"
            | "setshoutcost"
            | "setsplots"
            | "setspmany"
            | "setspsome"
            | "setspfew"
            | "setsprare"
            | "setspultra"
            | "setorbrespawndays"
            | "setmaxjewelcount"
            | "settunnelexpdivider"
            | "settunnelmillexp"
            | "setraregolemchance"
            | "setdungeontime"
            | "setbranfoexpbase"
            | "setbranexpbase"
            | "setpentvismaxpents"
            | "setpentmaxpower"
            | "setmaxsilvergolemtype"
            | "setnormaldropchance"
            | "setraredropchance"
            | "setraredropmultiplier"
            | "setbasedropmultiplier"
            | "setleveldivisor"
            | "setraregolemboost"
            | "setgolemhpmultiplier"
            | "setdemonlordaccess"
            | "setsolvemaxdivisor"
            | "setdemonpowerdeduction"
            | "setpentvaluemultiplier"
            | "setpentworthdivisor"
            | "setluckypentchance"
            | "setpowerincrement"
            | "setpentmaxtraining"
            | "setpentrandomspawn"
            | "setpentspawncount"
            | "setexpsolve"
            | "setclanreflection"
            | "setmaxclanbonus"
            | "setjaillocation"
            | "setastonlocation"
            | "setspecialdropmult"
            | "setdropproblow"
            | "setdropprobmid"
            | "setdropprobhigh"
            | "reloadloot"
            | "setlootmod"
            | "global"
    ) {
        return None;
    }

    if let ControlFlow::Break(result) =
        diagnostics::dispatch_diagnostics(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        progression::dispatch_progression(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        exp_military::dispatch_exp_military(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        dungeon_tunnel::dispatch_dungeon_tunnel(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        flags_clan::dispatch_flags_clan(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        movement::dispatch_movement(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        moderation::dispatch_moderation(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        inspection::dispatch_inspection(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        pentagram::dispatch_pentagram(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        macro_ac::dispatch_macro_ac(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        ppd_misc::dispatch_ppd_misc(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        anticheat::dispatch_anticheat(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    if let ControlFlow::Break(result) =
        ppd_misc::dispatch_clearppd(world, runtime, character_id, area_id, &lower, rest)
    {
        return result;
    }

    None
}
