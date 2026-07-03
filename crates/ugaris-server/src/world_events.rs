use super::*;

pub(crate) struct RuntimePlayerAttackPolicy<'a> {
    pub(crate) attacker_runtime: &'a PlayerRuntime,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PkRelationSnapshot {
    pub(crate) hate_by_character: HashMap<CharacterId, Vec<u32>>,
}

impl PkRelationSnapshot {
    pub(crate) fn from_runtime(runtime: &ServerRuntime) -> Self {
        let hate_by_character = runtime
            .players
            .values()
            .filter_map(|player| {
                let character_id = player.character_id?;
                Some((character_id, player.pk_hate.clone()))
            })
            .collect();
        Self { hate_by_character }
    }

    pub(crate) fn has_hate(&self, source: CharacterId, target: CharacterId) -> bool {
        target.0 != 0
            && self
                .hate_by_character
                .get(&source)
                .is_some_and(|hate| hate.iter().any(|id| *id == target.0))
    }
}

impl ClanAttackPolicy for RuntimePlayerAttackPolicy<'_> {
    fn has_pk_hate(&self, _attacker: &Character, defender: &Character) -> bool {
        self.attacker_runtime.has_pk_hate_for(defender.id.0)
    }
}

pub(crate) fn remove_stale_pvp_hate_if_effect_check_fails(
    player: &mut PlayerRuntime,
    attacker: &Character,
    target: &Character,
    area_id: u16,
) {
    if area_id == 1 {
        return;
    }
    if !attacker.flags.contains(CharacterFlags::PLAYER)
        || !target.flags.contains(CharacterFlags::PLAYER)
        || !attacker.flags.contains(CharacterFlags::PK)
    {
        return;
    }
    if attacker.id == target.id
        || !target.flags.contains(CharacterFlags::PK)
        || attacker.level.abs_diff(target.level) > 3
    {
        player.remove_pk_hate(target.id.0);
    }
}

pub(crate) fn apply_pk_hate_from_hurt_events(
    runtime: &mut ServerRuntime,
    world: &mut World,
    realtime_seconds: u64,
) -> usize {
    let mut applied = 0;
    let events = world.drain_legacy_hurt_events();
    for event in &events {
        apply_player_fightback_from_hurt_event(runtime, world, *event, world.tick.0);
    }
    for event in events {
        apply_swamp_monster_death_from_hurt_event(runtime, world, event);
        apply_teufel_rat_death_from_hurt_event(runtime, world, event);
        apply_caligar_skelly_death_from_hurt_event(runtime, world, event);
        apply_lab2_undead_death_from_hurt_event(runtime, world, event);

        let eligible = match (
            world.characters.get(&event.target_id),
            world.characters.get(&event.cause_id),
        ) {
            (Some(target), Some(cause)) => {
                target.id != cause.id
                    && target
                        .flags
                        .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
                    && cause
                        .flags
                        .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
                    && target.level.abs_diff(cause.level) <= 3
            }
            _ => false,
        };
        if !eligible {
            continue;
        }
        let Some(player) = runtime.player_for_character_mut(event.target_id) else {
            continue;
        };
        let Some(target) = world.characters.get_mut(&event.target_id) else {
            continue;
        };
        player.add_pk_hate_from_hit(target, event.cause_id.0);
        applied += 1;

        if event.outcome.killed {
            if let Some(player) = runtime.player_for_character_mut(event.target_id) {
                player.add_pk_death(realtime_seconds);
            }
            if let Some(player) = runtime.player_for_character_mut(event.cause_id) {
                player.add_pk_kill(realtime_seconds);
            }
        }
    }
    applied
}

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

pub(crate) fn apply_player_fightback_from_hurt_event(
    runtime: &mut ServerRuntime,
    world: &World,
    event: LegacyHurtEvent,
    current_tick: u64,
) -> bool {
    let Some((attacker_serial, legacy_distance)) = world
        .characters
        .get(&event.target_id)
        .zip(world.characters.get(&event.cause_id))
        .and_then(|(target, attacker)| {
            target
                .flags
                .contains(CharacterFlags::PLAYER)
                .then_some((attacker.serial, char_dist(target, attacker)))
        })
    else {
        return false;
    };
    runtime
        .player_for_character_mut(event.target_id)
        .is_some_and(|player| {
            player.apply_got_hit_fightback(
                event.cause_id,
                attacker_serial,
                legacy_distance,
                current_tick,
            )
        })
}

pub(crate) fn send_pending_world_system_texts(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_system_texts() {
        let payload = ugaris_protocol::packet::system_text(&event.message);
        for (session_id, _) in runtime.sessions_for_character(event.character_id) {
            if runtime.send_to_session(session_id, payload.clone()) {
                sent += 1;
            }
        }
    }
    sent
}

pub(crate) fn send_pending_world_area_texts(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_area_texts() {
        let payload = ugaris_protocol::packet::system_text(&event.message);
        let max_distance = i32::from(event.max_distance);
        let recipients: Vec<_> = world
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                ((i32::from(character.x) - i32::from(event.x)).abs() <= max_distance
                    && (i32::from(character.y) - i32::from(event.y)).abs() <= max_distance)
                    .then_some(character_id)
            })
            .collect();
        for character_id in recipients {
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                if runtime.send_to_session(session_id, payload.clone()) {
                    sent += 1;
                }
            }
        }
    }
    sent
}

pub(crate) fn pk_hate_prerequisites(source: &Character, target: &Character) -> bool {
    source.id != target.id
        && source
            .flags
            .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
        && target
            .flags
            .contains(CharacterFlags::PLAYER | CharacterFlags::PK)
        && source.level.abs_diff(target.level) <= 3
}
