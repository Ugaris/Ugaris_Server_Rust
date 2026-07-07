use super::*;

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

/// C `ch_died_driver`/`CDR_ASTURIN` dispatch (`gwendylon.c:6105-6107`) ->
/// `asturin_dead` (`:4535-4542`). C's `set_data(co, DRD_AREA1_PPD, ...)`
/// succeeds for *any* live character `co` (the generic per-character
/// memory-slot allocator has no player-only restriction), so the
/// `quiet_say` line fires regardless of who the killer is - only the
/// persistent `asturin_state = 4` write is player-only in this port
/// (`PlayerRuntime` only exists for real players), matching the
/// observable difference (an NPC killer's shadow `ppd` write is
/// discarded/never read again in C anyway).
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
