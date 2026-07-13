use super::*;
use ugaris_core::character_driver::{
    CDR_ARKHATAPRISON, CDR_ARKHATASKELLY, CDR_BOOKEATER, CDR_CENTINEL, CDR_CLANCLERK,
    CDR_CLANMASTER, CDR_GLADIATOR, CDR_LABGNOMEDRIVER, CDR_NOP, CDR_SHR_WEREWOLF, CDR_SMUGGLELEAD,
    CDR_TUNNELER_GORWIN, CDR_TWOGUARD, CDR_TWOROBBER, CDR_TWOSERVANT, CDR_WARPFIGHTER,
    CDR_WHITEROBBERBOSS,
};
use ugaris_core::world::{CS_ENEMY, CS_GUEST, LS_DEAD, LS_FINE};

pub(crate) fn apply_lab2_undead_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some((grave_item_id, opened_by, opened_by_serial, killer_serial)) = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .and_then(|(target, killer)| {
            let Some(CharacterDriverState::Lab2Undead(data)) = target.driver_state.as_ref() else {
                return None;
            };
            (target.driver == CDR_LAB2UNDEAD && killer.flags.contains(CharacterFlags::PLAYER))
                .then_some((
                    data.grave_item_id?,
                    data.opened_by_character_id?,
                    data.opened_by_serial,
                    killer.serial,
                ))
        })
    else {
        return false;
    };
    if opened_by != event.cause_id || opened_by_serial != killer_serial {
        return false;
    }
    let Some(grave_number) = lab2_grave_number(world, grave_item_id) else {
        return false;
    };
    runtime
        .player_for_character_mut(event.cause_id)
        .is_some_and(|player| player.mark_legacy_lab2_grave_cleared(grave_number))
}

pub(crate) fn apply_caligar_skelly_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some((home_x, home_y)) = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .and_then(|(target, killer)| {
            (target.driver == CDR_CALIGARSKELLY && killer.flags.contains(CharacterFlags::PLAYER))
                .then_some((target.rest_x, target.rest_y))
        })
    else {
        return false;
    };

    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    let message = match player.mark_caligar_skelly_death(home_x, home_y) {
        CaligarSkellyDeathResult::AlreadyUnlocked { .. } => {
            "You expect to hear a click, but nothing happens. Maybe you've been here before?"
                .to_string()
        }
        CaligarSkellyDeathResult::PartiallyUnlocked { .. } => {
            "You hear a faint sound in the distance, as if a lock was partially opened.".to_string()
        }
        CaligarSkellyDeathResult::FullyUnlocked { .. } => {
            "You hear a \"click\" in the distance, as if a lock had opened.".to_string()
        }
        CaligarSkellyDeathResult::Unmapped { x, y } => {
            format!("You have found bug #9824w at {x},{y}. Please report it.")
        }
    };
    world.queue_system_text(event.cause_id, message);
    true
}

pub(crate) fn apply_swamp_monster_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_swamp_monster_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_SWAMPMONSTER && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_swamp_monster_kill {
        return false;
    }

    let mut progressed_clara = false;
    if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
        let clara_state = player.area3_clara_state();
        if (12..=13).contains(&clara_state) {
            player.set_area3_clara_state(14);
            world.queue_system_text(event.cause_id, "Well done. Clara will be proud of thee!");
            progressed_clara = true;
        }
    }

    let upgraded_weapon = world.apply_swamp_monster_death_driver(event.target_id, event.cause_id);
    progressed_clara || upgraded_weapon
}

/// C `ch_died_driver`/`CDR_FORESTMONSTER` dispatch (`forest.c:938-940`)
/// -> `monster_dead` (`:817-853`). Splits like `apply_swamp_monster_
/// death_from_hurt_event`: the `imp_kills`/`hermit_state` counter halves
/// here (need `PlayerRuntime`), the weapon-glow item mutation in
/// [`World::apply_forest_monster_death_driver`].
pub(crate) fn apply_forest_monster_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some((sprite, is_hardkill)) = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .and_then(|(target, killer)| {
            (target.driver == CDR_FORESTMONSTER && killer.flags.contains(CharacterFlags::PLAYER))
                .then_some((
                    target.sprite,
                    target.flags.contains(CharacterFlags::HARDKILL),
                ))
        })
    else {
        return false;
    };

    let mut progressed = false;
    if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
        // C `if ((ch[cn].sprite == 306) && ... ppd->imp_state == 2) {
        // ppd->imp_kills++; if (ppd->imp_kills > 20) { ppd->imp_state = 3;
        // } }` (`forest.c:828-834`) - sprite `306` is the `bear35`
        // template.
        if sprite == 306 && player.area3_imp_state() == 2 {
            let kills = player.area3_imp_kills() + 1;
            player.set_area3_imp_kills(kills);
            if kills > 20 {
                player.set_area3_imp_state(3);
            }
            progressed = true;
        }
        // C `if ((ch[cn].flags & CF_HARDKILL) && ... ppd->hermit_state ==
        // 4) { ppd->hermit_state = 5; log_char(co, LOG_SYSTEM, 0, "Thou
        // hast slain the spider queen."); }` (`forest.c:836-840`).
        if is_hardkill && player.area3_hermit_state() == 4 {
            player.set_area3_hermit_state(5);
            world.queue_system_text(event.cause_id, "Thou hast slain the spider queen.");
            progressed = true;
        }
    }

    let upgraded_weapon = world.apply_forest_monster_death_driver(event.target_id, event.cause_id);
    progressed || upgraded_weapon
}

/// C `ch_died_driver`/`CDR_CAMERON_FORESTMONSTER` dispatch
/// (`gwendylon.c:6212-6214`) -> `monster_dead` (`:5201-5231`). Splits like
/// `apply_swamp_monster_death_from_hurt_event`: the `camhermit_kills`
/// counter here (needs `PlayerRuntime`), the weapon-glow item mutation in
/// [`World::apply_area1_monster_death_driver`].
pub(crate) fn apply_area1_monster_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_forest_monster_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_CAMERON_FORESTMONSTER
                && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_forest_monster_kill {
        return false;
    }

    let mut progressed_camhermit = false;
    if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
        // C `CAMHERMIT_STATE_QUEST1DO` (`npc_states.h:16`, value `5`).
        if player.area1_camhermit_state() == 5 {
            let kills = player.area1_camhermit_kills() + 1;
            player.set_area1_camhermit_kills(kills);
            // C `CAMHERMIT_QUEST1_KILLSNEEDED 10` (`gwendylon.c:677`).
            if kills == 10 {
                world.queue_system_text(
                    event.cause_id,
                    "Thou hast killed 10 big bears as requested by the sweet Hermit. go back to him and claim thy reward.",
                );
            }
            progressed_camhermit = true;
        }
    }

    let upgraded_weapon = world.apply_area1_monster_death_driver(event.target_id, event.cause_id);
    progressed_camhermit || upgraded_weapon
}

/// C `ch_died_driver`/`CDR_BREDEL` dispatch (`gwendylon.c:6221-6223`) ->
/// `bredel_dead` (`:2825-2842`): killing the robber-operations boss
/// advances `CDR_JESSICA`'s quest chain from `JESSICA_STATE_QUEST2_DO`
/// (`10`) to `JESSICA_STATE_QUEST2_FINISH` (`11`), see `world::jessica`'s
/// module doc comment for the previously-documented gap this closes.
pub(crate) fn apply_bredel_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_bredel_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_BREDEL && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_bredel_kill {
        return false;
    }

    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    // C `JESSICA_STATE_QUEST2_DO 10` (`npc_states.h:94`).
    if player.area1_jessica_state() != 10 {
        return false;
    }
    // C `JESSICA_STATE_QUEST2_FINISH 11` (`npc_states.h:95`).
    player.set_area1_jessica_state(11);
    world.queue_system_text(
        event.cause_id,
        "The local robber leader has been killed by thine hands. Congratulations!",
    );
    true
}

/// C `ch_died_driver`/`CDR_RIVERBEAST` dispatch (`gwendylon.c:6209-6211`)
/// -> `riverbeast_dead` (`:2255-2272`): killing the riverbeast advances
/// `CDR_JIU`'s quest chain from `JIU_STATE_WAIT_FOR_KILL` (`2`) to
/// `JIU_STATE_BEAST_KILLED` (`3`), see `world::jiu`'s module doc comment
/// for the previously-documented gap this closes.
pub(crate) fn apply_riverbeast_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_riverbeast_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_RIVERBEAST && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_riverbeast_kill {
        return false;
    }

    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    // C `JIU_STATE_WAIT_FOR_KILL 2` (`npc_states.h:78`).
    if player.area1_jiu_state() != 2 {
        return false;
    }
    // C `JIU_STATE_BEAST_KILLED 3` (`npc_states.h:79`).
    player.set_area1_jiu_state(3);
    world.queue_system_text(event.cause_id, "Well done. Jiu will be proud of thee!");
    true
}

/// C `ch_died_driver`/`CDR_BIGBADSPIDER` dispatch (`gwendylon.c:6218-6220`)
/// -> `bigbadspider_dead` (`:2850-2870`): killing the spider completes
/// `CDR_BRITHILDIE`'s `QLOG_BRITHILDIE` quest, advancing
/// `BRITHILDIE_STATE_NOMORETALES_QOPEN` (`20`) to `_QDONE` (`21`) via a
/// full `questlog_done` (exp reward + resend), see `world::brithildie`'s
/// module doc comment for the previously-documented gap this closes.
pub(crate) fn apply_bigbadspider_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_bigbadspider_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_BIGBADSPIDER && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_bigbadspider_kill {
        return false;
    }

    let Some(level) = world.characters.get(&event.cause_id).map(|c| c.level) else {
        return false;
    };
    let level_val = level_value(level);
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    // C `BRITHILDIE_STATE_NOMORETALES_QOPEN 20` (`npc_states.h:71`).
    if player.area1_brithildie_state() != 20 {
        return false;
    }
    world.queue_system_text(
        event.cause_id,
        "Well done. Thou hast killed the big bad spider.",
    );
    if let Some(completion) = player
        .quest_log
        .complete_legacy(QLOG_BRITHILDIE, level, level_val)
    {
        let payload = legacy_questlog_payload(player);
        world.give_exp(
            event.cause_id,
            completion.granted_exp,
            u32::from(world.area_id),
        );
        for (session_id, _) in runtime.sessions_for_character(event.cause_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
    // C `BRITHILDIE_STATE_NOMORETALES_QDONE 21` (`npc_states.h:72`).
    if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
        player.set_area1_brithildie_state(21);
    }
    true
}

pub(crate) fn apply_teufel_rat_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some((rat_level, reduced_score)) = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .and_then(|(target, killer)| {
            if target.driver == CDR_TEUFELRAT && killer.flags.contains(CharacterFlags::PLAYER) {
                Some((
                    target.level,
                    killer.flags.contains(CharacterFlags::LAG) || killer.driver == CDR_LOSTCON,
                ))
            } else {
                None
            }
        })
    else {
        return false;
    };

    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    let (kills, score) = player.add_teufel_rat_kill(rat_level, reduced_score);
    world.queue_system_text(event.cause_id, format!("#90 {kills} Rat Kills"));
    world.queue_system_text(event.cause_id, format!("#80 {score} Rat Points"));
    true
}

/// `World::process_gate_fight_actions`'s death-side counterpart: C's
/// `ch_died_driver`/`CDR_GATE_FIGHT` dispatch (`gatekeeper.c:808-810`) routes
/// straight to `gate_fight_dead(cn, co)` (`cn` the dying opponent, `co` its
/// killer). Mirrors `apply_swamp_monster_death_from_hurt_event`'s shape:
/// the killer's `gate_ppd.target_class` (`PlayerRuntime::gate_target_class`)
/// is the one fact `World::apply_gate_fight_reward` cannot read itself.
/// Class 8 (plain Seyan'Du) needs two more things `World` can't reach
/// either: the `"seyan_m"` template's base values (looked up here via
/// `loader`, matching C's own `create_char("seyan_m", 0)`) for
/// `World::apply_turn_seyan`, and `PlayerRuntime::clear_turn_seyan_ppd`
/// for `turn_seyan`'s `del_data` tail once the reroll actually happened
/// (`apply_gate_fight_reward` returning `true` for target_class 8 with a
/// resolved template means `apply_turn_seyan` succeeded - the same
/// `killer_id` lookup that gates the whole function also gates that call,
/// so it cannot fail in between within one single-threaded tick).
pub(crate) fn apply_gate_fight_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
    loader: &ZoneLoader,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_gate_fight_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_GATE_FIGHT && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_gate_fight_kill {
        return false;
    }

    let Some(target_class) = runtime
        .player_for_character(event.cause_id)
        .map(|player| player.gate_target_class)
    else {
        return false;
    };

    let seyan_base_values = (target_class == 8)
        .then(|| loader.character_templates.get("seyan_m"))
        .flatten()
        .map(|template| template.base_values.as_slice());

    let applied = world.apply_gate_fight_reward(event.cause_id, target_class, seyan_base_values);

    if applied && target_class == 8 && seyan_base_values.is_some() {
        if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
            player.clear_turn_seyan_ppd();
        }
    }

    applied
}

/// C `ch_died_driver`/`CDR_GATE_WELCOME` dispatch (`gatekeeper.c:810-811`)
/// routes any death of the welcome NPC to `immortal_dead(cn, co)`
/// (`gatekeeper.c:701-703`), which just writes a server-log-only line via
/// `charlog` (`co`, the killer, is unused). In practice this NPC template
/// carries `CF_IMMORTAL`, so `hurt()` already suppresses lethal damage to
/// it and this path should be unreachable through normal combat - ported
/// anyway for fidelity, matching the `debug!`-as-`charlog` precedent used
/// for `ClientAction::Log` (`main.rs`'s `cl_log` port).
pub(crate) fn apply_gate_welcome_death_from_hurt_event(
    world: &World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(target) = world.characters.get(&event.target_id) else {
        return false;
    };
    if target.driver != CDR_GATE_WELCOME {
        return false;
    }
    debug!(
        target: "client_log",
        "{}",
        format_client_log_message(
            &target.name,
            target.id.0,
            "I JUST DIED! I'M SUPPOSED TO BE IMMORTAL!"
        )
    );
    true
}

/// C `ch_died_driver`'s remaining area-1 `gwendylon_dead` dispatch
/// branches (`gwendylon.c:6180-6206`): `CDR_TERION`/`CDR_JAMES`/
/// `CDR_NOOK`/`CDR_LYDIA`/`CDR_GUIWYNN`/`CDR_LOGAIN`/`CDR_CAMHERMIT`/
/// `CDR_GREETER`/`CDR_JESSICA`/`CDR_BRITHILDIE` all route to the same
/// `gwendylon_dead(cn, co)` (`:3704-3706`), the identical `charlog`-only
/// bug line already ported for `CDR_GATE_WELCOME`/`CDR_DUNGEONMASTER`
/// above - same text, same immortal-so-unreachable-in-practice caveat
/// (every one of these quest-giver NPC templates carries `CF_IMMORTAL`).
pub(crate) fn apply_area1_quest_giver_death_from_hurt_event(
    world: &World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(target) = world.characters.get(&event.target_id) else {
        return false;
    };
    const GWENDYLON_DEAD_DRIVERS: [u16; 10] = [
        CDR_TERION,
        CDR_JAMES,
        CDR_NOOK,
        CDR_LYDIA,
        CDR_GUIWYNN,
        CDR_LOGAIN,
        CDR_CAMHERMIT,
        CDR_GREETER,
        CDR_JESSICA,
        CDR_BRITHILDIE,
    ];
    if !GWENDYLON_DEAD_DRIVERS.contains(&target.driver) {
        return false;
    }
    debug!(
        target: "client_log",
        "{}",
        format_client_log_message(
            &target.name,
            target.id.0,
            "I JUST DIED! I'M SUPPOSED TO BE IMMORTAL!"
        )
    );
    true
}

/// C `ch_died_driver`/`CDR_TUNNELER_GORWIN` dispatch (`src/area/33/
/// tunnel.c:1383-1391`) routes any death of Gorwin to
/// `generic_immortal_dead(cn, co)` (`:1380-1382`), the identical
/// `charlog`-only bug line already ported for `CDR_GATE_WELCOME` above -
/// same text, same immortal-so-unreachable-in-practice caveat (Gorwin's
/// template also carries `CF_IMMORTAL`).
pub(crate) fn apply_gorwin_death_from_hurt_event(world: &World, event: LegacyHurtEvent) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(target) = world.characters.get(&event.target_id) else {
        return false;
    };
    if target.driver != CDR_TUNNELER_GORWIN {
        return false;
    }
    debug!(
        target: "client_log",
        "{}",
        format_client_log_message(
            &target.name,
            target.id.0,
            "I JUST DIED! I'M SUPPOSED TO BE IMMORTAL!"
        )
    );
    true
}

/// C `ch_died_driver`/`CDR_DUNGEONMASTER` dispatch (`area/13/dungeon.c:
/// 2197-2200`) routes any death of the dungeonmaster NPC to
/// `immortal_dead(cn, co)` (`dungeon.c:1735-1737`), the identical
/// `charlog`-only bug line already ported for `CDR_GATE_WELCOME` above
/// (`gatekeeper.c:701-703`) - same text, same immortal-so-unreachable-in-
/// practice caveat (this NPC template also carries `CF_IMMORTAL`).
pub(crate) fn apply_dungeonmaster_death_from_hurt_event(
    world: &World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(target) = world.characters.get(&event.target_id) else {
        return false;
    };
    if target.driver != CDR_DUNGEONMASTER {
        return false;
    }
    debug!(
        target: "client_log",
        "{}",
        format_client_log_message(
            &target.name,
            target.id.0,
            "I JUST DIED! I'M SUPPOSED TO BE IMMORTAL!"
        )
    );
    true
}

/// C `ch_died_driver`'s area-3 immortal-quest-NPC dispatch (`area3.c:
/// 2884-2919`): `CDR_SEYMOUR`/`CDR_LAMPGHOST`/`CDR_KELLY`/`CDR_ASTRO1`/
/// `CDR_ASTRO2`/`CDR_THOMAS`/`CDR_SIRJONES`/`CDR_CARLOS`/`CDR_SUPERMAX`/
/// `CDR_KASSIM` all route to the same `immortal_dead(cn, co)`
/// (`area3.c:2596-2598`), the identical `charlog`-only bug line already
/// ported for `CDR_GATE_WELCOME`/`CDR_DUNGEONMASTER` above - same text,
/// same immortal-so-unreachable-in-practice caveat. `CDR_ASTRO1`/
/// `CDR_ASTRO2`/`CDR_THOMAS`/`CDR_SIRJONES`/`CDR_SEYMOUR`/`CDR_KELLY`/
/// `CDR_CARLOS`/`CDR_KASSIM`/`CDR_SUPERMAX` are ported so far; extend
/// this array as the sibling area-3 NPCs are ported.
pub(crate) fn apply_area3_immortal_death_from_hurt_event(
    world: &World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(target) = world.characters.get(&event.target_id) else {
        return false;
    };
    const AREA3_IMMORTAL_DRIVERS: [u16; 9] = [
        CDR_ASTRO1,
        CDR_ASTRO2,
        CDR_THOMAS,
        CDR_SIRJONES,
        CDR_SEYMOUR,
        CDR_KELLY,
        CDR_CARLOS,
        CDR_KASSIM,
        CDR_SUPERMAX,
    ];
    if !AREA3_IMMORTAL_DRIVERS.contains(&target.driver) {
        return false;
    }
    debug!(
        target: "client_log",
        "{}",
        format_client_log_message(
            &target.name,
            target.id.0,
            "I JUST DIED! I'M SUPPOSED TO BE IMMORTAL!"
        )
    );
    true
}

/// C `ch_died_driver`/`CDR_CLANMASTER`/`CDR_CLANCLERK` dispatch
/// (`clanmaster.c:1537-1549`) both route to `clanmaster_dead(cn, co)`
/// (`:1215-1217`), the identical `charlog`-only bug line already ported
/// for `CDR_GATE_WELCOME`/`CDR_DUNGEONMASTER`/area-1/area-3 above - same
/// text, same immortal-so-unreachable-in-practice caveat (both NPC
/// templates carry `CF_IMMORTAL`).
pub(crate) fn apply_area30_clan_npc_death_from_hurt_event(
    world: &World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(target) = world.characters.get(&event.target_id) else {
        return false;
    };
    if target.driver != CDR_CLANMASTER && target.driver != CDR_CLANCLERK {
        return false;
    }
    debug!(
        target: "client_log",
        "{}",
        format_client_log_message(
            &target.name,
            target.id.0,
            "I JUST DIED! I'M SUPPOSED TO BE IMMORTAL!"
        )
    );
    true
}

/// C `ch_died_driver`/`CDR_LAMPGHOST` dispatch (`area3.c:2936-2938`) ->
/// `lampghost_dead` (`:2741-2752`): unlike every other area-3 quest NPC's
/// shared `immortal_dead` no-op, the lamp-extinguisher ghost releases its
/// claimed lamp (if any) on death so another lampghost can pick it up.
pub(crate) fn apply_lampghost_death_from_hurt_event(
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_lampghost = world
        .characters
        .get(&event.target_id)
        .is_some_and(|target| target.driver == CDR_LAMPGHOST);
    if !is_lampghost {
        return false;
    }
    world.release_lampghost_lamp_claim(event.target_id);
    true
}

/// C `ch_died_driver`/`CDR_LABGNOMEDRIVER` dispatch (`lab1.c:615-623`) ->
/// `labgnome_died_driver` (`:388-406`). Only the `dat->text` speech branch
/// is ported here; the `dat->master` `create_lab_exit` reward branch is a
/// documented gap shared by all five lab areas - see
/// `world::npc::area22::lab1_gnome`'s own module doc comment.
pub(crate) fn apply_labgnome_death_from_hurt_event(
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_labgnome = world
        .characters
        .get(&event.target_id)
        .is_some_and(|target| target.driver == CDR_LABGNOMEDRIVER);
    if !is_labgnome {
        return false;
    }
    world.apply_labgnome_death_driver(event.target_id, event.cause_id);
    true
}

/// C `ch_died_driver`/`CDR_ASTURIN` dispatch (`gwendylon.c:6105-6107`) ->
/// `asturin_dead` (`:4535-4542`). C's `set_data(co, DRD_AREA1_PPD, ...)`
/// succeeds for *any* live character `co` (the generic per-character
/// memory-slot allocator has no player-only restriction), so the
/// `quiet_say` line fires regardless of who the killer is - only the
/// persistent `asturin_state = 4` write is player-only in this port
/// (`PlayerRuntime` only exists for real players), matching the
/// observable difference (an NPC killer's shadow `ppd` write is
/// discarded/never read again in C anyway).
/// C `guard_dead(cn, co)` (`src/area/17/two.c:744-769`): the Exkordon city
/// guard's death hook - `cn` is the dead guard, `co` its killer.
pub(crate) fn apply_two_guard_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_guard_kill = world
        .characters
        .get(&event.target_id)
        .is_some_and(|target| target.driver == CDR_TWOGUARD);
    if !is_guard_kill {
        return false;
    }
    let Some((killer_name, killer_is_player)) =
        world.characters.get(&event.cause_id).map(|killer| {
            (
                killer.name.clone(),
                killer.flags.contains(CharacterFlags::PLAYER),
            )
        })
    else {
        return false;
    };
    if !killer_is_player {
        return false;
    }

    world.npc_say(
        event.target_id,
        &format!("Thou shalt be punished for this misdeed, {killer_name}."),
    );

    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    // C `if (ppd->legal_status == LS_DEAD) return;` (`two.c:760-762`).
    if player.twocity_legal_status() == LS_DEAD {
        return true;
    }
    player.set_twocity_legal_status(LS_FINE);
    player.set_twocity_legal_fine(player.twocity_legal_fine() + 5000);
    if player.twocity_citizen_status() == CS_GUEST {
        player.set_twocity_citizen_status(CS_ENEMY);
    }
    true
}

/// C `robber_dead(cn, co)` (`src/area/17/two.c:2211-2247`): the Exkordon
/// forest-camp robbers' death hook. Only tracks a kill (bumping
/// `thief_killed[N]` for the still-unported `thiefmaster_driver`'s bounty
/// scoreboard, `two.c:1856-1857`/`1931`) while the killer's own
/// `thief_state` sits in the narrow `6..=9` window (the active
/// bounty-hunt phase of that still-unported quest chain) - matching C
/// exactly, this hook is a complete no-op for any killer outside that
/// window, including one with no `twocity_ppd` at all yet. `co == 0`
/// (no killer) and a non-player killer both return early in C before ever
/// touching `co`'s ppd; `event.cause_id` not resolving to a `CF_PLAYER`
/// character reproduces both cases (a dead/removed non-existent character
/// id behaves exactly like "no killer" here, since neither can be a
/// player). C's `default:` switch arm (`elog("unlisted robber level...")`)
/// is log-only and not ported, same precedent as every other bare
/// `elog(...)` call in this file.
pub(crate) fn apply_two_robber_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(level) = world
        .characters
        .get(&event.target_id)
        .filter(|target| target.driver == CDR_TWOROBBER)
        .map(|target| target.level)
    else {
        return false;
    };
    let is_player_kill = world
        .characters
        .get(&event.cause_id)
        .is_some_and(|killer| killer.flags.contains(CharacterFlags::PLAYER));
    if !is_player_kill {
        return false;
    }
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    if !(6..=9).contains(&player.twocity_thief_state()) {
        return true;
    }
    // C `switch (ch[cn].level) { case 35: ppd->thief_killed[0]++; ... }`
    // (`two.c:2223-2245`).
    let index = match level {
        35 => Some(0),
        39 => Some(1),
        43 => Some(2),
        47 => Some(3),
        51 => Some(4),
        55 => Some(5),
        _ => None,
    };
    if let Some(index) = index {
        player.set_twocity_thief_killed(index, player.twocity_thief_killed(index) + 1);
    }
    true
}

/// C `smugglelead_died(cn, co)` (`src/area/26/staffer.c:658-674`): quest-37
/// completion tail for `CDR_SMUGGLELEAD`, the Contraband quest chain's
/// final kill target. Only advances `smugglecom_state` from `8` (waiting
/// for the kill) to `9` (ready for `smugglecom_driver`'s own `NT_CHAR`
/// case `9` to speak the "thank you" line and mark quest 37 done) - unlike
/// `world::npc::area26::smugglecom`'s own dialogue-driven `QuestDone`
/// event, this hook never touches the quest log itself, matching C
/// exactly (`questlog_done(co, 37)` only ever runs from `smugglecom_
/// driver`'s `case 9`, not from this death hook).
pub(crate) fn apply_smugglelead_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_smugglelead = world
        .characters
        .get(&event.target_id)
        .is_some_and(|target| target.driver == CDR_SMUGGLELEAD);
    if !is_smugglelead {
        return false;
    }
    // C `if (!(ch[co].flags & CF_PLAYER)) return;` (`staffer.c:661-663`).
    let is_player_kill = world
        .characters
        .get(&event.cause_id)
        .is_some_and(|killer| killer.flags.contains(CharacterFlags::PLAYER));
    if !is_player_kill {
        return false;
    }
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    // C `if (ppd->smugglecom_state != 8) return;` (`staffer.c:669-671`).
    if player.staffer_smugglecom_state() == 8 {
        player.set_staffer_smugglecom_state(9);
    }
    true
}

/// C `robberboss_dead(cn, co)` (`src/area/28/brannington_forest.c:634-663`):
/// the Brannington robber camp's final kill target. Completes quest 46 ("A
/// Miner's Vengeance") and destroys the boss-kill quest-chain items for
/// whichever killer's `broklin_state` sits in `5..=10` - `broklin_state`
/// itself belongs to `src/area/29/brannington.c`'s (unported) `Broklin`
/// dialogue driver, read directly off `PlayerRuntime` here, same "read
/// state owned by another area's unported driver" precedent as
/// `world::npc::area26::rouven`'s `carlos2_state` read.
pub(crate) fn apply_robberboss_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    // C `if (!co) return; if (!(ch[co].flags & CF_PLAYER)) return;`
    // (`brannington_forest.c:637-643`).
    let is_robberboss_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_WHITEROBBERBOSS && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_robberboss_kill {
        return false;
    }

    let Some(level) = world.characters.get(&event.cause_id).map(|c| c.level) else {
        return false;
    };
    let level_val = level_value(level);
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    // C `if (ppd->broklin_state >= 5 && ppd->broklin_state <= 10)`
    // (`brannington_forest.c:648`).
    if !(5..=10).contains(&player.staffer_broklin_state()) {
        return false;
    }
    // C `ppd->broklin_state = 11;` (`brannington_forest.c:649`).
    player.set_staffer_broklin_state(11);
    world.queue_system_text(
        event.cause_id,
        "Well done. You've killed the head robber! Now go see Broklin...",
    );
    if let Some(completion) = player.quest_log.complete_legacy(46, level, level_val) {
        let payload = legacy_questlog_payload(player);
        world.give_exp(
            event.cause_id,
            completion.granted_exp,
            u32::from(world.area_id),
        );
        for (session_id, _) in runtime.sessions_for_character(event.cause_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_BOSSMASTER);
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_BOSSLAIR);
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_ROBBERKEY1);
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_ROBBERKEY2);
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_ROBBERKEY3);
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_ROBBERKEY4);
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_ROBBERKEY5);
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_ROBBERKEY6);
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_ROBBERKEY7);
    world.destroy_items_by_template_id(event.cause_id, IID_STAFF_ROBBERKEY8);
    true
}

/// C `centinel_dead(cn, co)` (`src/area/29/brannington.c:2725-2758`): the
/// wooden marionette sentinels guarding the Brannington tower
/// (`zones/29/wrtower.chr`'s `centinel_count` template, `CDR_CENTINEL`).
/// Increments the killer's `staffer_ppd.centinel_count` (capped at `30`,
/// C `if (ppd->centinel_count > 30) ppd->centinel_count = 30;`), reports
/// milestone progress at kills `1`/`10`/`20`, and on the `30`th kill
/// teleports the killer to `(33,143)` - but only resets the counter back
/// to `0` if the teleport actually moved the character (C's `if
/// (teleport_char_driver(co, 33, 143)) { ppd->centinel_count = 0; }`), so a
/// blocked teleport leaves the counter at `30` for a retry on the next
/// kill. C's own `if (!(ch[co].flags & CF_PLAYER)) return;` guard is
/// folded into the `killer.flags.contains(CharacterFlags::PLAYER)` check
/// below (`co` is the killer, `cn` the dead centinel - `set_data` operates
/// on `co`, matching `event.cause_id` here, same "target died, cause
/// killed" mapping as `apply_robberboss_death_from_hurt_event` above).
pub(crate) fn apply_centinel_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_centinel_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_CENTINEL && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_centinel_kill {
        return false;
    }

    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    let count = (player.staffer_centinel_count() + 1).min(30);
    player.set_staffer_centinel_count(count);

    let message = match count {
        1 => Some("You have killed the first sentinel on this floor, kill 29 more!"),
        10 => Some("You have killed 10 sentinels, 20 more to go!"),
        20 => Some("You have killed 20 sentinels, 10 more to go!"),
        30 => Some("Congratulations, you have killed 30 sentinels! Continue your journey."),
        _ => None,
    };
    if let Some(message) = message {
        world.queue_system_text(event.cause_id, message);
    }
    if count == 30 && world.teleport_char_driver(event.cause_id, 33, 143) {
        if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
            player.set_staffer_centinel_count(0);
        }
    }
    true
}

/// C `servant_dead(cn, co)` (`src/area/17/two.c:1324-1351`): the
/// forbidden-territory servants' death hook. Only the `nr == 4` "governor's
/// double" applies the harsh `citizen_status`/`legal_status` punishment
/// (matching `guard_dead`'s own fields); every other servant just says
/// "Arrgh! GUARDS!". Both branches always call `call_guard(cn, co)`
/// afterward - reachable via `World::two_city_call_guard` (`pub`, not
/// `pub(crate)`, specifically for this call site - see that method's own
/// doc comment), reading the dead servant's own `x`/`y` from `world.
/// characters` (still present at this point, since `LegacyHurtEvent`
/// fires before the corpse is fully removed, same precedent as `guard_
/// dead`/`robber_dead` reading `event.target_id` back above).
pub(crate) fn apply_two_servant_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(nr) = world.characters.get(&event.target_id).and_then(|target| {
        if target.driver != CDR_TWOSERVANT {
            return None;
        }
        let Some(CharacterDriverState::TwoServant(data)) = target.driver_state.as_ref() else {
            return None;
        };
        Some(data.nr)
    }) else {
        return false;
    };
    // C `if (!co) return; if (!(ch[co].flags & CF_PLAYER)) return;`
    // (`two.c:1328-1333`).
    let Some(killer_name) = world
        .characters
        .get(&event.cause_id)
        .filter(|killer| killer.flags.contains(CharacterFlags::PLAYER))
        .map(|killer| killer.name.clone())
    else {
        return false;
    };

    if nr == 4 {
        // C `ppd = set_data(co, DRD_TWOCITY_PPD, ...); if (ppd) { ppd->
        // citizen_status = CS_ENEMY; ppd->legal_status = LS_DEAD; say(...);
        // }` (`two.c:1336-1345`).
        if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
            player.set_twocity_citizen_status(CS_ENEMY);
            player.set_twocity_legal_status(LS_DEAD);
            world.npc_say(
                event.target_id,
                &format!(
                    "Thou shalt pay dearly for this, {killer_name}. Even though I am just the governors double, he wilt have thine head just for trying to kill him!"
                ),
            );
        }
    } else {
        // C `else { say(cn, "Arrgh! GUARDS!"); }` (`two.c:1346-1348`).
        world.npc_say(event.target_id, "Arrgh! GUARDS!");
    }

    // C `call_guard(cn, co);` (`two.c:1350`), unconditional.
    world.two_city_call_guard(event.target_id, event.cause_id);
    true
}

pub(crate) fn apply_asturin_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_asturin_kill = world
        .characters
        .get(&event.target_id)
        .is_some_and(|target| target.driver == CDR_ASTURIN);
    if !is_asturin_kill {
        return false;
    }
    let Some(killer_name) = world
        .characters
        .get(&event.cause_id)
        .map(|killer| killer.name.clone())
    else {
        return false;
    };

    world.npc_quiet_say(
        event.target_id,
        &format!("I'll remember that, {killer_name}!"),
    );
    if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
        player.set_area1_asturin_state(4);
    }
    true
}

/// C `ch_died_driver`/`CDR_SHR_WEREWOLF` dispatch (`shrike.c:415-419`) ->
/// `shr_werewolf_dead` (`:344-354`):
/// ```c
/// void shr_werewolf_dead(int cn, int co) {
///     struct area1_ppd *ppd;
///     create_mist(ch[cn].x, ch[cn].y);
///     ch[cn].sprite = 6;
///     if ((ch[co].flags & CF_PLAYER) && (ppd = set_data(co, DRD_AREA1_PPD, sizeof(struct area1_ppd)))) {
///         ppd->shrike_fails++;
///         say(cn, "I have deserved death. But still... I was hoping for something better.");
///     }
/// }
/// ```
/// Needs `PlayerRuntime::area1_shrike_fails` (a plain legacy `area1_ppd`
/// blob offset accessor, not a fresh field - see `crate::player::area1`),
/// so it can't live purely in `World` - same precedent as
/// `apply_asturin_death_from_hurt_event` above.
pub(crate) fn apply_shr_werewolf_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(werewolf) = world.characters.get(&event.target_id).cloned() else {
        return false;
    };
    if werewolf.driver != CDR_SHR_WEREWOLF {
        return false;
    }

    // C `create_mist(ch[cn].x, ch[cn].y); ch[cn].sprite = 6;`
    // (`shrike.c:347-348`).
    world.create_mist_effect(i32::from(werewolf.x), i32::from(werewolf.y));
    if let Some(werewolf_mut) = world.characters.get_mut(&event.target_id) {
        werewolf_mut.sprite = 6;
    }

    // C `if ((ch[co].flags & CF_PLAYER) && (ppd = set_data(co,
    // DRD_AREA1_PPD, ...))) { ppd->shrike_fails++; say(cn, "I have
    // deserved death. But still... I was hoping for something better."); }`
    // (`shrike.c:350-353`).
    let killer_is_player = world
        .characters
        .get(&event.cause_id)
        .is_some_and(|killer| killer.flags.contains(CharacterFlags::PLAYER));
    if killer_is_player {
        if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
            player.set_area1_shrike_fails(player.area1_shrike_fails() + 1);
            world.npc_say(
                event.target_id,
                "I have deserved death. But still... I was hoping for something better.",
            );
        }
    }
    true
}

/// C `ch_died_driver`/`CDR_VAMPIRE` dispatch (`area2.c:1039-1041`) ->
/// `vampire_dead_driver` (`:941-965`): only completes "The Toughest
/// Monster" (`questlog_done(co, 18)`) and destroys every sun-amulet piece
/// the killer is still carrying while `area3_ppd.crypt_state` sits in the
/// narrow `8..=9` window (the crypt puzzle state that means "the Vampire
/// Lord is the next expected kill") - reuses the same `area3_crypt_state`
/// accessor `world::vampire2`'s own death hook and the "Underground Park
/// Shrines"/crypt puzzle (P4 area3, unported) share.
pub(crate) fn apply_vampire_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_vampire_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_VAMPIRE && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_vampire_kill {
        return false;
    }

    let Some(level) = world.characters.get(&event.cause_id).map(|c| c.level) else {
        return false;
    };
    let level_val = level_value(level);
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    if !(8..=9).contains(&player.area3_crypt_state()) {
        return false;
    }
    world.queue_system_text(
        event.cause_id,
        "Congratulations on slaying the toughest(?) creature down here!",
    );
    if let Some(completion) = player.quest_log.complete_legacy(18, level, level_val) {
        let payload = legacy_questlog_payload(player);
        world.give_exp(
            event.cause_id,
            completion.granted_exp,
            u32::from(world.area_id),
        );
        for (session_id, _) in runtime.sessions_for_character(event.cause_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
    world.destroy_items_by_template_id(event.cause_id, IID_AREA2_SUN1);
    world.destroy_items_by_template_id(event.cause_id, IID_AREA2_SUN2);
    world.destroy_items_by_template_id(event.cause_id, IID_AREA2_SUN3);
    world.destroy_items_by_template_id(event.cause_id, IID_AREA2_SUN12);
    world.destroy_items_by_template_id(event.cause_id, IID_AREA2_SUN13);
    world.destroy_items_by_template_id(event.cause_id, IID_AREA2_SUN23);
    world.destroy_items_by_template_id(event.cause_id, IID_AREA2_SUN123);
    if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
        player.set_area3_crypt_state(10);
    }
    true
}

/// C `ch_died_driver`/`CDR_VAMPIRE2` dispatch (`area2.c:1044-1046`) ->
/// `vampire2_dead_driver` (`:967-984`): completes "The Toughestest
/// Monster" (`questlog_done(co, 19)`) while `area3_ppd.crypt_state` sits
/// in the `12..=14` window.
pub(crate) fn apply_vampire2_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_vampire2_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_VAMPIRE2 && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_vampire2_kill {
        return false;
    }

    let Some(level) = world.characters.get(&event.cause_id).map(|c| c.level) else {
        return false;
    };
    let level_val = level_value(level);
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    if !(12..=14).contains(&player.area3_crypt_state()) {
        return false;
    }
    world.queue_system_text(
        event.cause_id,
        "Congratulations on slaying the toughest creature down here!",
    );
    if let Some(completion) = player.quest_log.complete_legacy(19, level, level_val) {
        let payload = legacy_questlog_payload(player);
        world.give_exp(
            event.cause_id,
            completion.granted_exp,
            u32::from(world.area_id),
        );
        for (session_id, _) in runtime.sessions_for_character(event.cause_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
    if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
        player.set_area3_crypt_state(15);
    }
    true
}

/// C `ch_died_driver`'s `CDR_FDEMON_DEMON` case (`fdemon.c:3074-3076`) ->
/// `fdemon_demon_dead` (`:2851-2879`): slaying the `sprite==190` "Fire
/// Golem" boss variant advances the killer's `farmy_ppd.boss_stage` from
/// `16`/`17` to `18` (see [`PlayerRuntime::advance_farmy_golem_kill_stage`]'s
/// doc comment for the platoon-leader-credit gap this doesn't reproduce
/// yet). Matched by `world.area_id == 8 && target.sprite == 190` rather
/// than `target.driver == CDR_FDEMON_DEMON`: the "Fire Golem" template is
/// spawned with `driver = CDR_SIMPLEBADDY` directly (see
/// `zone.rs`'s `CDR_FDEMON_DEMON` branch and
/// `world::npc::area8::fdemon_demon`'s module doc comment for why), so
/// `sprite` is the only remaining discriminator - `area_id` (this port's
/// one-process-per-area invariant, see `World::area_id`'s doc comment)
/// guards against an unrelated area reusing sprite `190` for something
/// else.
pub(crate) fn apply_fdemon_demon_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed || world.area_id != 8 {
        return false;
    }
    let is_fire_golem_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.sprite == 190 && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_fire_golem_kill {
        return false;
    }
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    if player.advance_farmy_golem_kill_stage() {
        world.queue_system_text(event.cause_id, "Well done. Now go back to the Commander.");
        true
    } else {
        false
    }
}

/// C `ch_died_driver`/`CDR_LQNPC` dispatch (`lq.c:3013-3021`) ->
/// `lqnpc_died` (`:2929-2958`): schedules this NPC's respawn (guarded by
/// matching its live-instance identity against `World::lq_npcs`, exactly
/// like C's own `lq_npc[dat->n].cn == cn && ... .cserial == ch[cn]
/// .serial` check, in case the slot has already been reused by a
/// different spawn) and sets the killer's Live Quest kill/hurt marks.
pub(crate) fn apply_lqnpc_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(target) = world.characters.get(&event.target_id) else {
        return false;
    };
    if target.driver != CDR_LQNPC {
        return false;
    }
    let (slot, kill_mark_id, hurt_mark_id) = match target.driver_state.as_ref() {
        Some(CharacterDriverState::LqNpc(data)) => {
            (data.slot, data.kill_mark_id, data.hurt_mark_id)
        }
        _ => return false,
    };
    let target_id = target.id;
    let target_serial = target.serial;

    // C `if (lq_npc[dat->n].cn == cn && lq_npc[dat->n].cserial ==
    // ch[cn].serial) { if (lq_npc[dat->n].respawn) lq_respawn[dat->n] =
    // ticker + lq_npc[dat->n].respawn * TICKS; lq_npc[dat->n].cn =
    // lq_npc[dat->n].cserial = 0; }` (`lq.c:2938-2944`).
    let identity_match = world
        .lq_npcs
        .iter()
        .find(|npc| npc.slot == slot)
        .and_then(|npc| {
            (npc.character_id == Some(target_id) && npc.character_serial == target_serial)
                .then_some(npc.respawn_seconds)
        });
    if let Some(respawn_seconds) = identity_match {
        if respawn_seconds > 0 {
            let due_tick = world.tick.0 + u64::from(respawn_seconds) * TICKS_PER_SECOND;
            world.schedule_lq_npc_respawn(slot, due_tick);
        }
        if let Some(npc) = world.lq_npcs.iter_mut().find(|npc| npc.slot == slot) {
            npc.character_id = None;
            npc.character_serial = 0;
        }
    }

    // C `if (co && (ch[co].flags & CF_PLAYER) && (dat->kill_markID ||
    // dat->hurt_markID)) { ... }` (`lq.c:2945-2956`).
    if (kill_mark_id != 0 || hurt_mark_id != 0) && event.cause_id.0 != 0 {
        let killer_is_player = world
            .characters
            .get(&event.cause_id)
            .is_some_and(|killer| killer.flags.contains(CharacterFlags::PLAYER));
        if killer_is_player {
            if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
                if kill_mark_id > 0 {
                    player.set_lq_mark(kill_mark_id);
                }
                if hurt_mark_id > 0 {
                    player.set_lq_mark(hurt_mark_id);
                }
            }
        }
    }
    true
}

/// C `warpfighter_died(cn, co)` (`src/area/25/warped.c:971-991`): only the
/// *owning player's own killing blow* (`dat->co == co`) teleports them
/// back through the trial-room door - a kill by anyone/anything else, or
/// the owner having already left the room bounds by the time the fighter
/// dies, is a silent no-op in C (`xlog(...)`-only, not player-visible).
pub(crate) fn apply_warpfighter_death_from_hurt_event(
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(data) = world.characters.get(&event.target_id).and_then(|target| {
        if target.driver != CDR_WARPFIGHTER {
            return None;
        }
        match target.driver_state.as_ref() {
            Some(CharacterDriverState::WarpFighter(data)) => Some(*data),
            _ => None,
        }
    }) else {
        return false;
    };

    // C `if (dat->co != co) { xlog("1"); return; }` (`warped.c:979-982`).
    if data.owner != event.cause_id {
        return false;
    }
    // C `if (!ch[co].flags || ch[co].serial != dat->cser || ch[co].x <
    // dat->xs || ch[co].y < dat->ys || ch[co].x > dat->xe || ch[co].y >
    // dat->ye) { xlog("2"); return; }` (`warped.c:983-987`).
    let owner_valid = world.characters.get(&event.cause_id).is_some_and(|owner| {
        owner.serial == data.owner_serial
            && owner.x >= data.xs
            && owner.x <= data.xe
            && owner.y >= data.ys
            && owner.y <= data.ye
    });
    if !owner_valid {
        return false;
    }

    // C `teleport_char_driver(co, dat->tx, dat->ty);` (`warped.c:989`).
    world.teleport_char_driver(event.cause_id, data.tx, data.ty);
    true
}

/// C `ch_died_driver`'s `CDR_MISSIONFIGHT` case (`missions.c:1911-1913`)
/// -> `mission_fighter_dead(cn, co)` (`:1852-1881`): `nr = ch[cn].deaths`
/// (the dying fighter's `fID` tier tag, [`Character::deaths`]) increments
/// the matching kill counter on the *killer's* (`co`) `governor` ppd
/// (`ppd = set_data(co, DRD_MISSION_PPD, ...)`), then re-prints the job's
/// `mission_status` HUD lines and runs `mission_done` - which, once every
/// objective is complete, promotes `active` to `solved` and announces it.
/// C has no `if (!co) return;` guard here (unlike `missionchest_driver`),
/// but a `CDT_DEAD` dispatch with no killer never reaches this file's
/// event-driven port at all ([`LegacyHurtEvent::cause_id`] is only
/// produced for player-caused kills), so the "no killer" case is already
/// unreachable rather than silently dropped.
pub(crate) fn apply_mission_fighter_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    use ugaris_core::character_driver::CDR_MISSIONFIGHT;
    use ugaris_core::world::npc::area32::governor::MISSION_TEMPLATES;
    use ugaris_core::world::npc::area32::mission_start::{
        mission_status_lines, record_mission_fighter_kill, try_solve_mission, MISSION_FIGHTER_DATA,
    };

    if !event.outcome.killed {
        return false;
    }
    let Some(fighter_kind) = world
        .characters
        .get(&event.target_id)
        .and_then(|target| (target.driver == CDR_MISSIONFIGHT).then_some(target.deaths as u8))
    else {
        return false;
    };
    if !world
        .characters
        .get(&event.cause_id)
        .is_some_and(|killer| killer.flags.contains(CharacterFlags::PLAYER))
    {
        return false;
    }
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };

    record_mission_fighter_kill(&mut player.governor, fighter_kind);
    let ppd = player.governor;
    let md_idx = ppd.md_idx.clamp(0, MISSION_FIGHTER_DATA.len() as i32 - 1) as usize;
    let title = MISSION_TEMPLATES[md_idx].title;
    for line in mission_status_lines(&ppd, title, &MISSION_FIGHTER_DATA[md_idx]) {
        world.queue_system_text(event.cause_id, line);
    }

    if try_solve_mission(&mut player.governor) {
        let killer_name = world
            .characters
            .get(&event.cause_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        world.queue_system_text(
            event.cause_id,
            format!(
                "You've finished the job. Good work, {killer_name}. Now talk to Mr. Jones for your reward."
            ),
        );
    }
    true
}

/// C `ch_died_driver`/`CDR_ARKHATAPRISON` dispatch (`arkhata.c:4698-
/// 4700`) routes any death of the Fortress prisoner to `prisoner_dead(cn,
/// co)` (`:4490-4492`), a plain unconditional `say(cn, "I know the
/// secret, it's right here!")` - no `co`/killer checks at all, unlike the
/// `immortal_dead` family above.
pub(crate) fn apply_arkhata_prisoner_death_from_hurt_event(
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_prisoner_kill = world
        .characters
        .get(&event.target_id)
        .is_some_and(|target| target.driver == CDR_ARKHATAPRISON);
    if !is_prisoner_kill {
        return false;
    }
    world.npc_say(event.target_id, "I know the secret, it's right here!");
    true
}

/// C `ch_died_driver`/`CDR_BOOKEATER` dispatch (`arkhata.c:4666-4668`)
/// routes any death of "The Book Eater" monster to `bookeater_dead(cn,
/// co)` (`:4333-4351`): killer must be a player (`co`/`CF_PLAYER` check),
/// must have `arkhata_ppd.monk_state == 19` (i.e. already sent by Tracy
/// to slay it - the still-unported `arkhatamonk_driver`'s own dialogue
/// state machine, see `PlayerRuntime::arkhata_monk_state`'s doc comment),
/// then completes quest 70 ("The Book Eater") and advances `monk_state`
/// to `20` so the killer can report back to Tracy.
pub(crate) fn apply_arkhata_bookeater_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_bookeater_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_BOOKEATER && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_bookeater_kill {
        return false;
    }
    let Some(level) = world.characters.get(&event.cause_id).map(|c| c.level) else {
        return false;
    };
    let level_val = level_value(level);
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    if player.arkhata_monk_state() != 19 {
        return false;
    }
    if let Some(completion) = player.quest_log.complete_legacy(70, level, level_val) {
        let payload = legacy_questlog_payload(player);
        world.give_exp(
            event.cause_id,
            completion.granted_exp,
            u32::from(world.area_id),
        );
        for (session_id, _) in runtime.sessions_for_character(event.cause_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
    if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
        player.set_arkhata_monk_state(20);
    }
    world.queue_system_text(
        event.cause_id,
        "Well done, you've solved Tracy's quest. Now report back to her.",
    );
    true
}

/// C `ch_died_driver`/`CDR_ARKHATASKELLY` dispatch (`arkhata.c:4620-
/// 4622`) routes any death of a Fighting School skeleton
/// (`Skeleton_for_final_area`) to `arkhataskelly_dead(cn, co)`
/// (`:1612-1646`): silently no-ops (leaving the generic respawn timer
/// alone) unless the killer is a player with `arkhata_ppd.ramin_state ==
/// 6` (i.e. Ramin already sent them to clear out the infestation - the
/// still-unported `ramin_driver`'s own dialogue state, see
/// `PlayerRuntime::arkhata_ramin_state`'s doc comment). Once that gate
/// passes, C counts every other still-alive `CDR_ARKHATASKELLY`
/// character via a purely internal idle-tick bookkeeping array
/// (`skelly_cn[]`, not ported - see `CDR_ARKHATASKELLY`'s own doc
/// comment) - ported here as a direct count over `world.characters`
/// (behaviorally equivalent, no `arg=`/position hashing needed). While
/// any remain, a progress message is shown only every 5th kill or once
/// fewer than 10 remain (`(undead % 5) == 0 || undead < 10`); once none
/// remain, quest 68 ("A Shopkeeper's Fright") completes via the standard
/// `questlog_done` exp path and `ramin_state` advances to `7` so the
/// killer can report back to Ramin.
pub(crate) fn apply_arkhataskelly_death_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_skelly_kill = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .is_some_and(|(target, killer)| {
            target.driver == CDR_ARKHATASKELLY && killer.flags.contains(CharacterFlags::PLAYER)
        });
    if !is_skelly_kill {
        return false;
    }
    let Some(level) = world.characters.get(&event.cause_id).map(|c| c.level) else {
        return false;
    };
    let level_val = level_value(level);
    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return false;
    };
    if player.arkhata_ramin_state() != 6 {
        return false;
    }

    // C's `ch[cc].flags` truthy check (`arkhata.c:1633`) is C's "is this
    // array slot still occupied" test; in this codebase `world.characters`
    // only ever holds occupied slots, so no separate flags check is
    // needed here - `character_id != event.target_id` alone matches C's
    // `cc != cn` exclusion of the character that just died.
    let undead = world
        .characters
        .iter()
        .filter(|(&character_id, character)| {
            character_id != event.target_id && character.driver == CDR_ARKHATASKELLY
        })
        .count();

    if undead > 0 {
        if undead % 5 == 0 || undead < 10 {
            world.queue_system_text(
                event.cause_id,
                format!("{} down, {undead} to go. Beware of respawns!", 80 - undead),
            );
        }
        return true;
    }

    let Some(player) = runtime.player_for_character_mut(event.cause_id) else {
        return true;
    };
    if let Some(completion) = player.quest_log.complete_legacy(68, level, level_val) {
        let payload = legacy_questlog_payload(player);
        world.give_exp(
            event.cause_id,
            completion.granted_exp,
            u32::from(world.area_id),
        );
        for (session_id, _) in runtime.sessions_for_character(event.cause_id) {
            runtime.send_to_session(session_id, payload.clone());
        }
    }
    if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
        player.set_arkhata_ramin_state(7);
    }
    world.queue_system_text(
        event.cause_id,
        "Well done, you've solved Ramin's quest. Now report back to him.",
    );
    true
}

/// C `ch_died_driver`/`CDR_NOP` dispatch (`arkhata.c:4657-4659`) routes
/// any death of a Fighting School "Student" NPC to `immortal_dead(cn,
/// co)` (`:4486-4488`), the identical `charlog`-only bug line already
/// ported for `CDR_GATE_WELCOME` above - same text. Unlike most of that
/// family, the `Student` template (`zones/37/Fighting_School.chr`)
/// carries no `CF_IMMORTAL` flag, so this path is genuinely reachable in
/// practice, not just fidelity-for-dead-code. Most of `arkhata.c`'s
/// other drivers (`rammy`/`jaz`/`fiona`/`arkhatamonk`/`captain`/`judge`/
/// `jada`/`potmaker`/`hunter`/`thaipan`/`trainer`/`kidnappee`/`clerk`/
/// `krenach`) route to this same `immortal_dead` function too
/// (`arkhata.c:4643-4703`) - extend this driver list as each one is
/// ported, same "grow the array" precedent as
/// `apply_area1_quest_giver_death_from_hurt_event`'s
/// `GWENDYLON_DEAD_DRIVERS`.
pub(crate) fn apply_arkhata_immortal_death_from_hurt_event(
    world: &World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let Some(target) = world.characters.get(&event.target_id) else {
        return false;
    };
    if target.driver != CDR_NOP {
        return false;
    }
    debug!(
        target: "client_log",
        "{}",
        format_client_log_message(
            &target.name,
            target.id.0,
            "I JUST DIED! I'M SUPPOSED TO BE IMMORTAL!"
        )
    );
    true
}

/// C `ch_died_driver`/`CDR_GLADIATOR` dispatch (`arkhata.c:4653-4655`)
/// routes any death of a `world::npc::area37::gladiator` student to
/// `gladiator_dead(cn, co)` (`:1176-1178`) - a plain `notify_area`
/// broadcast reporting the killer back to any nearby Fiona, ported as
/// `World::apply_gladiator_death` (see that method's own doc comment for
/// why this lives outside `world_events::death_hooks`'s usual
/// `PlayerRuntime`-touching shape).
pub(crate) fn apply_gladiator_death_from_hurt_event(
    world: &mut World,
    event: LegacyHurtEvent,
) -> bool {
    if !event.outcome.killed {
        return false;
    }
    let is_gladiator_kill = world
        .characters
        .get(&event.target_id)
        .is_some_and(|target| target.driver == CDR_GLADIATOR);
    if !is_gladiator_kill {
        return false;
    }
    world.apply_gladiator_death(event.target_id, event.cause_id);
    true
}
