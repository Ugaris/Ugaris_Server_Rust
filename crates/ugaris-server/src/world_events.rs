use super::*;

/// Mirrors `ugaris_core::world::combat::RuntimePlayerAttackPolicy`'s shape
/// (see that struct's doc comment) - a separate copy is needed here
/// because these call sites go through `World::tick_effects_with_attack_policy`/
/// `tick_basic_actions_with_attack_policy`'s `FnMut` closures, which cannot
/// hold a live `&World` borrow (the tick call itself needs `&mut World`);
/// callers must clone `world.clan_registry.relations()` before the tick
/// call and move the clone into the closure (see `main.rs`).
pub(crate) struct RuntimePlayerAttackPolicy<'a> {
    pub(crate) attacker_runtime: &'a PlayerRuntime,
    pub(crate) clan_relations: &'a ClanRelations,
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

    fn are_allied(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.clan_relations.alliance(attacker_clan, defender_clan)
    }

    fn can_attack_inside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.clan_relations
            .can_attack_inside(attacker_clan, defender_clan)
    }

    fn can_attack_outside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
        self.clan_relations
            .can_attack_outside(attacker_clan, defender_clan)
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
    loader: &ZoneLoader,
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
        apply_gate_fight_death_from_hurt_event(runtime, world, event, loader);
        apply_gate_welcome_death_from_hurt_event(world, event);
        apply_dungeonmaster_death_from_hurt_event(world, event);
        apply_area1_monster_death_from_hurt_event(runtime, world, event);
        apply_bredel_death_from_hurt_event(runtime, world, event);
        apply_riverbeast_death_from_hurt_event(runtime, world, event);

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

/// C `bank_driver`'s deposit/withdraw/balance handling (`src/module/
/// bank.c`), persistent-balance half: applies each [`BankEvent`] queued
/// by `World::process_bank_actions` (see `world/bank.rs`'s module doc
/// comment for why this split exists - `World` cannot see
/// `PlayerRuntime`'s `DRD_BANK_PPD`-backed `bank_gold`) to the matching
/// player's account balance, mirroring `apply_teufel_rat_death_from_hurt_event`'s
/// `runtime`+`world` shape.
pub(crate) fn apply_bank_events(runtime: &mut ServerRuntime, world: &mut World) -> usize {
    let mut applied = 0;
    for event in world.drain_pending_bank_events() {
        match event {
            BankEvent::Deposit { player_id, amount } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                // C `ppd->imperial_gold += val`; `Character.gold` was
                // already debited synchronously in
                // `World::process_bank_actions`.
                player.bank_gold = player.bank_gold.saturating_add(amount);
                applied += 1;
            }
            BankEvent::Withdraw {
                bank_id,
                player_id,
                amount,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if amount > player.bank_gold {
                    world.npc_quiet_say(
                        bank_id,
                        "Thou dost not have that much gold in thine account.",
                    );
                } else {
                    // C `ppd->imperial_gold -= val;
                    // give_money_silent(co, val, "Bank withdrawal");` - no
                    // generic "give money" helper exists yet
                    // (`world/bank.rs`'s module doc comment), so this
                    // mirrors `world/merchant.rs::merchant_store_sell`'s
                    // existing direct-mutation-plus-`CF_ITEMS` pattern.
                    player.bank_gold -= amount;
                    if let Some(character) = world.characters.get_mut(&player_id) {
                        character.gold = character.gold.saturating_add(amount);
                        character.flags.insert(CharacterFlags::ITEMS);
                    }
                    world.npc_quiet_say(
                        bank_id,
                        &format!("Thou hast withdrawn {} gold coins.", amount / 100),
                    );
                }
                applied += 1;
            }
            BankEvent::Balance { bank_id, player_id } => {
                let Some(player) = runtime.player_for_character(player_id) else {
                    continue;
                };
                let balance = player.bank_gold;
                // C `bank_driver`'s balance branch (`bank.c:379-387`).
                let message = if balance > 100 {
                    format!(
                        "Thou hast {} gold and {} silver in thine account.",
                        balance / 100,
                        balance % 100
                    )
                } else if balance != 0 {
                    format!("Thou hast {balance} silver in thine account.")
                } else {
                    "Thou dost not have any money in thine account.".to_string()
                };
                world.npc_quiet_say(bank_id, &message);
                applied += 1;
            }
        }
    }
    applied
}

/// C `trader_driver`'s "show trade" (`src/module/base.c:443-465`),
/// `NT_GIVE` cross-notify (`base.c:496-523`) item-look output, and the
/// "accept trade" success branch's Trust But Verify achievement award
/// (`base.c:4420-4428`): applies each [`TraderEvent`] queued by
/// `World::process_trader_actions` (see `world/trader.rs`'s module doc
/// comment for why the first two need `legacy_item_look_text`, which lives
/// in this crate, not `ugaris-core`) by formatting each item and queuing
/// the resulting lines as system text to the requesting player, mirroring
/// `apply_bank_events`'s shape - `runtime`/`repository` are only touched by
/// the `DealCompleted` branch (`ShowTrade`/`ItemAddedToTrade` don't touch
/// `PlayerRuntime`).
pub(crate) async fn apply_trader_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
) -> usize {
    let mut applied = 0;
    for event in world.drain_pending_trader_events() {
        match event {
            TraderEvent::ShowTrade {
                viewer_id,
                c1_items,
                c2_items,
            } => {
                let Some(viewer) = world.characters.get(&viewer_id).cloned() else {
                    continue;
                };
                world.queue_system_text(viewer_id, "Trading:");
                for item_id in c1_items {
                    if let Some(item) = world.items.get(&item_id).cloned() {
                        for line in legacy_item_look_text(&item, &viewer).lines() {
                            world.queue_system_text(viewer_id, line.to_string());
                        }
                    }
                }
                world.queue_system_text(viewer_id, "For:");
                for item_id in c2_items {
                    if let Some(item) = world.items.get(&item_id).cloned() {
                        for line in legacy_item_look_text(&item, &viewer).lines() {
                            world.queue_system_text(viewer_id, line.to_string());
                        }
                    }
                }
                applied += 1;
            }
            TraderEvent::ItemAddedToTrade {
                notify_id,
                giver_name,
                item_id,
            } => {
                let Some(viewer) = world.characters.get(&notify_id).cloned() else {
                    continue;
                };
                // C `log_char(c2, LOG_SYSTEM, 0, COL_LIGHT_GREEN "%s gave
                // me:", giver_name)` - color marker dropped (see
                // `world/trader.rs`'s module doc comment).
                world.queue_system_text(notify_id, format!("{giver_name} gave me:"));
                if let Some(item) = world.items.get(&item_id).cloned() {
                    for line in legacy_item_look_text(&item, &viewer).lines() {
                        world.queue_system_text(notify_id, line.to_string());
                    }
                }
                applied += 1;
            }
            TraderEvent::DealCompleted { c1_id, c2_id } => {
                award_trader_deal_achievement(world, runtime, repository, c1_id, c2_id).await;
                applied += 1;
            }
        }
    }
    applied
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

/// Byte-payload sibling of [`send_pending_world_system_texts`] - see
/// `WorldSystemTextBytes`.
pub(crate) fn send_pending_world_system_text_bytes(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_system_text_bytes() {
        let payload = ugaris_protocol::packet::system_text_bytes(&event.message);
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

/// C `server_chat(channel, text)` (`src/system/chat/chat.c:827-834`),
/// consumer half: drains `World::drain_pending_channel_broadcasts` and fans
/// each message out to every connected player who has joined that channel,
/// matching the channel-bit delivery rule `apply_chat_command`
/// (`commands_chat.rs`) uses for player-authored channel messages (no
/// clan/mirror/area/ignore filters apply to channel 6 "Grats", so a plain
/// join-bit check is sufficient here).
pub(crate) fn send_pending_world_channel_broadcasts(
    runtime: &mut ServerRuntime,
    world: &mut World,
) -> usize {
    let mut sent = 0;
    for event in world.drain_pending_channel_broadcasts() {
        let payload = ugaris_protocol::packet::system_text_bytes(&event.message_bytes);
        let bit = 1_u32 << (event.channel.saturating_sub(1));
        let recipients: Vec<CharacterId> = runtime
            .players
            .values()
            .filter(|player| player.chat_channels & bit != 0)
            .filter_map(|player| player.character_id)
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

/// `World::process_gate_welcome_actions`'s input half: snapshots the two
/// `PlayerRuntime`-owned facts (`gate_ppd.welcome_state`,
/// `teleport_next_lab`'s truthiness) the gate-welcome greeting dialogue
/// needs, for every currently-spawned player, mirroring
/// `PkRelationSnapshot::from_runtime`'s shape (see `world/gatekeeper.rs`'s
/// module doc comment for why `World` cannot read these itself).
pub(crate) fn gate_welcome_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, GateWelcomePlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                GateWelcomePlayerFacts {
                    welcome_state: player.gate_welcome_state,
                    needs_lab: needs_next_lab(player.lab_solved_bits),
                },
            ))
        })
        .collect()
}

/// `World::process_gate_welcome_actions`'s output half: applies each
/// [`GateWelcomeOutcomeEvent`] (see `world/gatekeeper.rs`'s module doc
/// comment) to the matching player's `PlayerRuntime`, mirroring
/// `apply_bank_events`'s shape.
pub(crate) fn apply_gate_welcome_events(
    runtime: &mut ServerRuntime,
    world: &mut World,
    loader: &mut ZoneLoader,
    events: Vec<GateWelcomeOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            GateWelcomeOutcomeEvent::UpdateWelcomeState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.gate_welcome_state = new_state;
                applied += 1;
            }
            GateWelcomeOutcomeEvent::ResetLabPpd { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                // C `del_data(co, DRD_LAB_PPD)`: fully clears the block.
                player.lab_solved_bits = 0;
                player.lab_ppd.clear();
                applied += 1;
            }
            GateWelcomeOutcomeEvent::EnterTestReady { player_id, class } => {
                if gate_enter_test_spawn_room(world, loader, runtime, player_id, class) {
                    applied += 1;
                }
            }
        }
    }
    applied
}

/// Applies each [`ClanmasterEvent`] queued by `World::process_clanmaster_actions`:
/// the clan-log entries and achievement awards C's `found_clan`/
/// `add_member`/`remove_member` perform internally, which the pure
/// `ClanRegistry` methods leave to the caller (see `crate::world_events`'s
/// module doc comment shape, mirroring `apply_trader_events`/
/// `apply_bank_events`), plus (for `OfflineRankLookup`/`OfflineFire`) the
/// DB-backed offline-target lookup/validation/mutation C performs via its
/// `task_set_clan_rank`/`task_fire_from_clan` async DB-task queue - see
/// [`apply_offline_clan_rank`]/[`apply_offline_clan_fire`].
pub(crate) async fn apply_clanmaster_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    now_unix: i64,
) -> usize {
    let mut applied = 0;
    for event in world.drain_pending_clanmaster_events() {
        match event {
            ClanmasterEvent::ClanFounded {
                founder_id,
                clan_nr,
            } => {
                let Some(founder_name) = world.characters.get(&founder_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `found_clan` (`clan.c:489`): "Clan was founded by %s".
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    founder_id,
                    1,
                    format!("Clan was founded by {founder_name}"),
                    now_unix,
                )
                .await;
                // C `add_member` (`clan.c:1192`): "%s was added to clan by
                // %s" (master = the founder's own name, `clanmaster.c:570`).
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    founder_id,
                    15,
                    format!("{founder_name} was added to clan by {founder_name}"),
                    now_unix,
                )
                .await;
                award_clanmaster_member_achievement(
                    world,
                    runtime,
                    achievement_repository,
                    founder_id,
                )
                .await;
                award_clanmaster_master_achievement(
                    world,
                    runtime,
                    achievement_repository,
                    founder_id,
                )
                .await;
                applied += 1;
            }
            ClanmasterEvent::MemberAdded {
                member_id,
                clan_nr,
                master_name,
            } => {
                let Some(member_name) = world.characters.get(&member_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    member_id,
                    15,
                    format!("{member_name} was added to clan by {master_name}"),
                    now_unix,
                )
                .await;
                award_clanmaster_member_achievement(
                    world,
                    runtime,
                    achievement_repository,
                    member_id,
                )
                .await;
                applied += 1;
            }
            ClanmasterEvent::MemberLeft { member_id, clan_nr } => {
                let Some(member_name) = world.characters.get(&member_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `remove_member(co, co)` via `leave!`
                // (`clanmaster.c:435-441`): master is the leaving member
                // themself.
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    member_id,
                    15,
                    format!("{member_name} was fired from clan by {member_name}"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanmasterEvent::RankSet {
                clan_nr,
                target_id,
                rank,
                setter_name,
            } => {
                let Some(target_name) = world.characters.get(&target_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `clanmaster_driver`'s `rank:` handler's own
                // `add_clanlog` call (`clanmaster.c:493-494`, prio 30):
                // "%s rank was set to %d by %s".
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    target_id,
                    30,
                    format!("{target_name} rank was set to {rank} by {setter_name}"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanmasterEvent::MemberFired {
                member_id,
                clan_nr,
                firer_name,
            } => {
                let Some(member_name) = world.characters.get(&member_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `remove_member(cc, co)` via `fire:` (`clanmaster.c:
                // 539`): master = the firing leader, not the fired member
                // themself (contrast `ClanmasterEvent::MemberLeft`).
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    member_id,
                    15,
                    format!("{member_name} was fired from clan by {firer_name}"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanmasterEvent::OfflineRankLookup {
                clanmaster_id,
                clan_nr,
                target_name,
                rank,
                setter_name,
            } => {
                apply_offline_clan_rank(
                    world,
                    character_repository,
                    clan_log_repository,
                    clanmaster_id,
                    clan_nr,
                    &target_name,
                    rank,
                    &setter_name,
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanmasterEvent::OfflineFire {
                clanmaster_id,
                clan_nr,
                target_name,
                setter_name,
            } => {
                apply_offline_clan_fire(
                    world,
                    character_repository,
                    clan_log_repository,
                    clanmaster_id,
                    clan_nr,
                    &target_name,
                    &setter_name,
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanmasterEvent::JewelWonFromSpawner {
                player_id,
                clan_nr,
                level,
            } => {
                let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `clan_dungeon_chat`'s `'X'` case (`clan.c:1358-1372`,
                // prio 5): "%s won a jewel from level %d spawn".
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    player_id,
                    5,
                    format!("{player_name} won a jewel from level {level} spawn"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
        }
    }
    applied
}

/// C `clanmaster_driver`'s `rank:` offline fallback
/// (`clanmaster.c:481-499`, `task_set_clan_rank`/`set_clan_rank`,
/// `task.c:87-101,213-295,333-345`): resolves `target_name` against the
/// DB directly (this codebase's synchronous stand-in for C's cached
/// `lookup_name` + async task-queue worker - see
/// `ClanmasterEvent::OfflineRankLookup`'s doc comment), then mirrors
/// `set_clan_rank`'s validation/mutation/clan-log/feedback exactly:
/// - no DB row at all -> "Sorry, no player by the name %s found."
///   (`uID == -1`).
/// - a row found -> immediate "Update scheduled (%s,%d)." feedback
///   (`clanmaster.c:497`), matching C's fire-and-forget
///   `task_set_clan_rank` semantics (sent regardless of whether the
///   mutation below actually succeeds).
/// - target already online elsewhere -> silent no-op (C's `set_task`
///   "online somewhere else" guard, `task.c:238-243`, only `xlog`s).
/// - target not a member of `clan_nr` / not paid for rank > 1 -> the
///   same rejection messages `set_clan_rank` sends via `tell_chat`.
/// - otherwise -> mutate, guarded save (`CharacterSaveMode::Backup`
///   with `expected_current_area`/`expected_current_mirror` pinned to
///   the loaded snapshot's own offline `0`/`0`, so a concurrent login
///   between the load and the save aborts the write exactly like C's
///   `UPDATE ... WHERE current_area = ...`), clan-log entry, and "Set
///   %s's rank to %d." feedback.
async fn apply_offline_clan_rank(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    clanmaster_id: CharacterId,
    clan_nr: u16,
    target_name: &str,
    rank: u8,
    setter_name: &str,
    now_unix: i64,
) {
    let Some(repository) = character_repository else {
        return;
    };
    let Ok(Some(summary)) = repository.find_login_target(target_name).await else {
        world.npc_quiet_say(
            clanmaster_id,
            &format!("Sorry, no player by the name {target_name} found."),
        );
        return;
    };
    world.npc_quiet_say(
        clanmaster_id,
        &format!("Update scheduled ({target_name},{rank})."),
    );

    let Ok(Some(snapshot)) = repository.load_character_snapshot(summary.id).await else {
        return;
    };
    // C `set_task`'s "online somewhere else" guard (`task.c:238-243`):
    // silent no-op (only an `xlog`, no player-facing message).
    if snapshot.current_area != 0 {
        return;
    }

    let mut character = snapshot.character;
    if world.clan_registry.get_char_clan(&mut character) != Some(clan_nr) {
        world.npc_quiet_say(
            clanmaster_id,
            &format!(
                "{} is not a member of your clan, you cannot set the rank.",
                character.name
            ),
        );
        return;
    }
    if !character.flags.contains(CharacterFlags::PAID) && rank > 1 {
        world.npc_quiet_say(
            clanmaster_id,
            &format!(
                "{} is not a paying player, you cannot set the rank higher than 1.",
                character.name
            ),
        );
        return;
    }
    character.clan_rank = rank;
    let target_id = character.id;
    let target_display_name = character.name.clone();

    let request = ugaris_db::CharacterSaveRequest {
        character,
        items: snapshot.items,
        ppd_blob: snapshot.ppd_blob,
        subscriber_blob: snapshot.subscriber_blob,
        mode: ugaris_db::CharacterSaveMode::Backup {
            expected_current_area: snapshot.current_area,
            expected_current_mirror: snapshot.current_mirror,
            mirror: snapshot.mirror,
        },
    };
    if !matches!(repository.save_character_snapshot(request).await, Ok(true)) {
        return;
    }

    let serial = world.clan_registry.serial(clan_nr);
    crate::clan_log::write_clan_log_entry(
        clan_log_repository,
        clan_nr,
        serial,
        target_id,
        30,
        format!("{target_display_name} rank was set to {rank} by {setter_name}"),
        now_unix,
    )
    .await;
    world.npc_quiet_say(
        clanmaster_id,
        &format!("Set {target_display_name}'s rank to {rank}."),
    );
}

/// Same shape as [`apply_offline_clan_rank`] but for `fire:`'s offline
/// fallback (`clanmaster.c:525-546`, `task_fire_from_clan`/
/// `fire_from_clan`, `task.c:117-133,347-356`): "Update scheduled (%s)."
/// carries no rank, and a successful mutation clears `clan`/`clan_rank`
/// (`remove_member`'s effect) rather than setting a rank, with the
/// clan-log prio-15 "was fired from clan by" shape (matching
/// `ClanmasterEvent::MemberFired`).
async fn apply_offline_clan_fire(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    clanmaster_id: CharacterId,
    clan_nr: u16,
    target_name: &str,
    setter_name: &str,
    now_unix: i64,
) {
    let Some(repository) = character_repository else {
        return;
    };
    let Ok(Some(summary)) = repository.find_login_target(target_name).await else {
        world.npc_quiet_say(
            clanmaster_id,
            &format!("Sorry, no player by the name {target_name} found."),
        );
        return;
    };
    world.npc_quiet_say(clanmaster_id, &format!("Update scheduled ({target_name})."));

    let Ok(Some(snapshot)) = repository.load_character_snapshot(summary.id).await else {
        return;
    };
    if snapshot.current_area != 0 {
        return;
    }

    let mut character = snapshot.character;
    if world.clan_registry.get_char_clan(&mut character) != Some(clan_nr) {
        world.npc_quiet_say(
            clanmaster_id,
            &format!(
                "{} is not a member of your clan, you cannot fire him/her.",
                character.name
            ),
        );
        return;
    }
    character.clan = 0;
    character.clan_rank = 0;
    character.clan_serial = 0;
    let target_id = character.id;
    let target_display_name = character.name.clone();

    let request = ugaris_db::CharacterSaveRequest {
        character,
        items: snapshot.items,
        ppd_blob: snapshot.ppd_blob,
        subscriber_blob: snapshot.subscriber_blob,
        mode: ugaris_db::CharacterSaveMode::Backup {
            expected_current_area: snapshot.current_area,
            expected_current_mirror: snapshot.current_mirror,
            mirror: snapshot.mirror,
        },
    };
    if !matches!(repository.save_character_snapshot(request).await, Ok(true)) {
        return;
    }

    let serial = world.clan_registry.serial(clan_nr);
    crate::clan_log::write_clan_log_entry(
        clan_log_repository,
        clan_nr,
        serial,
        target_id,
        15,
        format!("{target_display_name} was fired from clan by {setter_name}"),
        now_unix,
    )
    .await;
    world.npc_quiet_say(clanmaster_id, &format!("Fired {target_display_name}."));
}

/// Applies each [`ClubmasterEvent`] queued by `World::process_clubmaster_actions`:
/// the `ACHIEVEMENT_CLUB_MEMBER`/`ACHIEVEMENT_CLUB_MASTER` awards C's
/// `clubmaster_driver` performs inline at its `found:`/`join:` success
/// sites (`src/system/clubmaster.c:305-306,364`) - same shape as
/// [`apply_clanmaster_events`], minus any clan-log persistence (club
/// founding/deposit/withdraw only ever hit C's bare, non-persisted
/// `dlog`, see `crate::world::clubmaster`'s module doc comment) - plus
/// (for `OfflineRankLookup`/`OfflineFire`) the DB-backed offline-target
/// lookup/validation/mutation C performs via its shared
/// `task_set_clan_rank`/`task_fire_from_clan` async DB-task queue, same
/// shape as [`apply_offline_clan_rank`]/[`apply_offline_clan_fire`] but
/// following `set_clan_rank`/`fire_from_clan`'s `else` (club) branch - see
/// [`apply_offline_club_rank`]/[`apply_offline_club_fire`].
pub(crate) async fn apply_clubmaster_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let mut applied = 0;
    for event in world.drain_pending_clubmaster_events() {
        match event {
            ClubmasterEvent::ClubFounded { founder_id } => {
                award_clubmaster_member_achievement(
                    world,
                    runtime,
                    achievement_repository,
                    founder_id,
                )
                .await;
                award_clubmaster_master_achievement(
                    world,
                    runtime,
                    achievement_repository,
                    founder_id,
                )
                .await;
                applied += 1;
            }
            ClubmasterEvent::MemberAdded { member_id } => {
                award_clubmaster_member_achievement(
                    world,
                    runtime,
                    achievement_repository,
                    member_id,
                )
                .await;
                applied += 1;
            }
            ClubmasterEvent::OfflineRankLookup {
                clubmaster_id,
                club_nr,
                target_name,
                rank,
                setter_name,
            } => {
                apply_offline_club_rank(
                    world,
                    character_repository,
                    clubmaster_id,
                    club_nr,
                    &target_name,
                    rank,
                    &setter_name,
                )
                .await;
                applied += 1;
            }
            ClubmasterEvent::OfflineFire {
                clubmaster_id,
                club_nr,
                target_name,
                setter_name,
            } => {
                apply_offline_club_fire(
                    world,
                    character_repository,
                    clubmaster_id,
                    club_nr,
                    &target_name,
                    &setter_name,
                )
                .await;
                applied += 1;
            }
        }
    }
    applied
}

/// C `clubmaster_driver`'s `rank:` offline fallback (`clubmaster.c:
/// 420-432`, `task_set_clan_rank`/`set_clan_rank`'s `else` (club) branch,
/// `task.c:96-124`): resolves `target_name` against the DB directly (this
/// codebase's synchronous stand-in for C's cached `lookup_name` + async
/// task-queue worker - see `ClubmasterEvent::OfflineRankLookup`'s doc
/// comment), then mirrors `set_clan_rank`'s club-branch validation/
/// mutation/feedback exactly (no clan-log entry - clubs have none, see
/// `apply_clubmaster_events`'s doc comment):
/// - no DB row at all -> "Sorry, no player by the name %s found."
/// - a row found -> immediate "Update scheduled (%s,%d)." feedback,
///   matching C's fire-and-forget `task_set_clan_rank` semantics.
/// - target already online elsewhere -> silent no-op (`task.c:238-243`).
/// - not a member of `club_nr` -> "%s is not a member of your club, you
///   cannot set the rank."
/// - not paid and `rank > 0` -> "%s is not a paying player, you cannot
///   set the rank higher than 0."
/// - target is the founder (`clan_rank == 2`) -> "%s is the club's
///   founder, can't change rank."
/// - otherwise -> mutate, guarded save, "Set %s's rank to %d." feedback.
async fn apply_offline_club_rank(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    clubmaster_id: CharacterId,
    club_nr: u16,
    target_name: &str,
    rank: u8,
    // C's own `set_clan_rank` (`task.c:87-124`) never reads `set->
    // master_name` in its club (`else`) branch either - there is no
    // club-log equivalent of `add_clanlog` to attribute it to - so this
    // is genuinely dead here, kept only for call-site symmetry with
    // `apply_offline_clan_rank`.
    _setter_name: &str,
) {
    let Some(repository) = character_repository else {
        return;
    };
    let Ok(Some(summary)) = repository.find_login_target(target_name).await else {
        world.npc_quiet_say(
            clubmaster_id,
            &format!("Sorry, no player by the name {target_name} found."),
        );
        return;
    };
    world.npc_quiet_say(
        clubmaster_id,
        &format!("Update scheduled ({target_name},{rank})."),
    );

    let Ok(Some(snapshot)) = repository.load_character_snapshot(summary.id).await else {
        return;
    };
    if snapshot.current_area != 0 {
        return;
    }

    let mut character = snapshot.character;
    if world.club_registry.get_char_club(&mut character) != Some(club_nr) {
        world.npc_quiet_say(
            clubmaster_id,
            &format!(
                "{} is not a member of your club, you cannot set the rank.",
                character.name
            ),
        );
        return;
    }
    if !character.flags.contains(CharacterFlags::PAID) && rank > 0 {
        world.npc_quiet_say(
            clubmaster_id,
            &format!(
                "{} is not a paying player, you cannot set the rank higher than 0.",
                character.name
            ),
        );
        return;
    }
    if character.clan_rank == 2 {
        world.npc_quiet_say(
            clubmaster_id,
            &format!(
                "{} is the club's founder, can't change rank.",
                character.name
            ),
        );
        return;
    }
    character.clan_rank = rank;
    let target_display_name = character.name.clone();

    let request = ugaris_db::CharacterSaveRequest {
        character,
        items: snapshot.items,
        ppd_blob: snapshot.ppd_blob,
        subscriber_blob: snapshot.subscriber_blob,
        mode: ugaris_db::CharacterSaveMode::Backup {
            expected_current_area: snapshot.current_area,
            expected_current_mirror: snapshot.current_mirror,
            mirror: snapshot.mirror,
        },
    };
    if !matches!(repository.save_character_snapshot(request).await, Ok(true)) {
        return;
    }

    world.npc_quiet_say(
        clubmaster_id,
        &format!("Set {target_display_name}'s rank to {rank}."),
    );
}

/// Same shape as [`apply_offline_club_rank`] but for `fire:`'s offline
/// fallback (`clubmaster.c:468-481`, `task_fire_from_clan`/
/// `fire_from_clan`'s `else` (club) branch, `task.c:142-168`): "Update
/// scheduled (%s)." carries no rank, a successful mutation clears
/// `clan`/`clan_rank` (`remove_member`'s effect), and the founder
/// (`clan_rank > 1`) cannot be fired ("You cannot fire %s, he is the
/// founder of the club.").
async fn apply_offline_club_fire(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    clubmaster_id: CharacterId,
    club_nr: u16,
    target_name: &str,
    setter_name: &str,
) {
    let _ = setter_name;
    let Some(repository) = character_repository else {
        return;
    };
    let Ok(Some(summary)) = repository.find_login_target(target_name).await else {
        world.npc_quiet_say(
            clubmaster_id,
            &format!("Sorry, no player by the name {target_name} found."),
        );
        return;
    };
    world.npc_quiet_say(clubmaster_id, &format!("Update scheduled ({target_name})."));

    let Ok(Some(snapshot)) = repository.load_character_snapshot(summary.id).await else {
        return;
    };
    if snapshot.current_area != 0 {
        return;
    }

    let mut character = snapshot.character;
    if world.club_registry.get_char_club(&mut character) != Some(club_nr) {
        world.npc_quiet_say(
            clubmaster_id,
            &format!(
                "{} is not a member of your club, you cannot fire him/her.",
                character.name
            ),
        );
        return;
    }
    if character.clan_rank > 1 {
        world.npc_quiet_say(
            clubmaster_id,
            &format!(
                "You cannot fire {}, he is the founder of the club.",
                character.name
            ),
        );
        return;
    }
    character.clan = 0;
    character.clan_rank = 0;
    character.clan_serial = 0;
    let target_display_name = character.name.clone();

    let request = ugaris_db::CharacterSaveRequest {
        character,
        items: snapshot.items,
        ppd_blob: snapshot.ppd_blob,
        subscriber_blob: snapshot.subscriber_blob,
        mode: ugaris_db::CharacterSaveMode::Backup {
            expected_current_area: snapshot.current_area,
            expected_current_mirror: snapshot.current_mirror,
            mirror: snapshot.mirror,
        },
    };
    if !matches!(repository.save_character_snapshot(request).await, Ok(true)) {
        return;
    }

    world.npc_quiet_say(clubmaster_id, &format!("Fired {target_display_name}."));
}

/// Applies each [`ClanclerkEvent`] queued by `World::process_clanclerk_actions`:
/// the clan-log entries C's `clan_money_change`/`set_clan_rankname`/
/// `set_clan_website`/`set_clan_message`/`add_jewel`/`set_clan_raid`/
/// `set_clan_raid_god` perform internally, which the pure `ClanRegistry`
/// methods leave to the caller - same shape as [`apply_clanmaster_events`].
pub(crate) async fn apply_clanclerk_events(
    world: &mut World,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    now_unix: i64,
) -> usize {
    let mut applied = 0;
    for event in world.drain_pending_clanclerk_events() {
        match event {
            ClanclerkEvent::MoneyChanged {
                clan_nr,
                actor_id,
                change,
            } => {
                let Some(actor_name) = world.characters.get(&actor_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    actor_id,
                    28,
                    change.log_message(&actor_name),
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanclerkEvent::RankNameSet {
                clan_nr,
                actor_id,
                rank,
                name,
            } => {
                let Some(actor_name) = world.characters.get(&actor_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `set_clan_rankname` (`clan.c:875`): "%s set rank name
                // %d to %s".
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    actor_id,
                    33,
                    format!("{actor_name} set rank name {rank} to {name}"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanclerkEvent::WebsiteSet {
                clan_nr,
                actor_id,
                site,
            } => {
                let Some(actor_name) = world.characters.get(&actor_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `set_clan_website` (`clan.c:590`): "%s set website %s".
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    actor_id,
                    35,
                    format!("{actor_name} set website {site}"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanclerkEvent::MessageSet {
                clan_nr,
                actor_id,
                message,
            } => {
                let Some(actor_name) = world.characters.get(&actor_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `set_clan_message` (`clan.c:601`): "%s set message %s".
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    actor_id,
                    35,
                    format!("{actor_name} set message {message}"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanclerkEvent::JewelAdded { clan_nr, actor_id } => {
                let Some(actor_name) = world.characters.get(&actor_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `add_jewel` (`clan.c:495`): "%s added a jewel".
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    actor_id,
                    1,
                    format!("{actor_name} added a jewel"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanclerkEvent::RaidToggled {
                clan_nr,
                actor_id,
                enabled,
            }
            | ClanclerkEvent::RaidGodToggled {
                clan_nr,
                actor_id,
                enabled,
            } => {
                let Some(actor_name) = world.characters.get(&actor_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `set_clan_raid`/`set_clan_raid_god` (`clan.c:550,557,
                // 568,575`): "%s set raiding to ON"/"%s canceled raiding".
                let message = if enabled {
                    format!("{actor_name} set raiding to ON")
                } else {
                    format!("{actor_name} canceled raiding")
                };
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    actor_id,
                    1,
                    message,
                    now_unix,
                )
                .await;
                applied += 1;
            }
            ClanclerkEvent::DungeonUseSet {
                clan_nr,
                actor_id,
                dungeon_type,
                number,
            } => {
                let Some(actor_name) = world.characters.get(&actor_id).map(|c| c.name.clone())
                else {
                    continue;
                };
                let serial = world.clan_registry.serial(clan_nr);
                // C `set_clan_dungeon_use` (`clan.c:722`): "%s set
                // dungeon use of type %d to %d".
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan_nr,
                    serial,
                    actor_id,
                    35,
                    format!("{actor_name} set dungeon use of type {dungeon_type} to {number}"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
        }
    }
    applied
}

/// C `tick_clan`'s three per-clan economy sub-ticks (`clan.c:358-436`,
/// states 3/4), minus the multi-process storage load/save state machine
/// C wraps them in (that side is handled separately by `main.rs`'s own
/// once-a-minute `clan_repository`/`ClanRegistry::dirty` save, which has
/// no C equivalent - see that call site's own comment): the daily
/// relation escalation/de-escalation tick (`update_relations`,
/// `clan.c:936-1089`, [`ClanRelations::update`]), the treasury tick
/// (`update_treasure`, `clan.c:1105-1159`, [`ClanRegistry::
/// update_treasure`] - bonus affordability, weekly upkeep, debt accrual/
/// auto-pay, bankrupt-clan deletion), and the dungeon training-score
/// decay tick (`update_training`, `clan.c:1166-1182`,
/// [`ClanRegistry::update_training`]). Each function internally gates on
/// its own `payed_till`/`want_date`/`last_training_update` timers (see
/// their doc comments for the exact windows), so calling this every
/// server tick - like C's own `tick_clan`, called every tick once area
/// 3's clan storage load completes - is correct and cheap.
pub(crate) async fn apply_clan_economy_tick(
    world: &mut World,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    now_unix: i64,
) -> usize {
    let mut applied = 0;

    let relation_events = world.clan_registry.relations_mut().update(now_unix);
    for event in relation_events {
        let (Some(name_a), Some(name_b)) = (
            world.clan_registry.name(event.clan_a).map(str::to_string),
            world.clan_registry.name(event.clan_b).map(str::to_string),
        ) else {
            continue;
        };
        let serial_a = world.clan_registry.serial(event.clan_a);
        let serial_b = world.clan_registry.serial(event.clan_b);
        // C `add_clanlog(n, ..., 0, 10, ...)`/`add_clanlog(m, ..., 0, 10,
        // ...)` (`clan.c:980-1083`): both sides of the pair get the
        // message, actor character ID 0 meaning "system".
        crate::clan_log::write_clan_log_entry(
            clan_log_repository,
            event.clan_a,
            serial_a,
            CharacterId(0),
            10,
            event.change.log_message(&name_b, event.clan_b),
            now_unix,
        )
        .await;
        crate::clan_log::write_clan_log_entry(
            clan_log_repository,
            event.clan_b,
            serial_b,
            CharacterId(0),
            10,
            event.change.log_message(&name_a, event.clan_a),
            now_unix,
        )
        .await;
        applied += 1;
    }

    let treasury_events = world.clan_registry.update_treasure(now_unix);
    for event in treasury_events {
        match event {
            // C `xlog(...)` only (`clan.c:1151`) - server debug log, no
            // player-facing `add_clanlog` entry.
            ClanTreasuryEvent::PaidDebtWithJewels { .. } => {}
            ClanTreasuryEvent::WentBroke { clan, serial, name } => {
                // C `add_clanlog(cnr, clan_serial(cnr), 0, 1, "Clan %s
                // went broke and was deleted", get_clan_name(cnr))`
                // (`clan.c:1156`), logged *before* the name is cleared
                // and the serial bumped - `serial`/`name` are the
                // pre-deletion values the event already carries, matching
                // that ordering (see `ClanRegistry::update_treasure`'s
                // doc comment on the `WentBroke` push site).
                crate::clan_log::write_clan_log_entry(
                    clan_log_repository,
                    clan,
                    serial,
                    CharacterId(0),
                    1,
                    format!("Clan {name} went broke and was deleted"),
                    now_unix,
                )
                .await;
                applied += 1;
            }
        }
    }

    // C `update_training` (`clan.c:1166-1182`): server-debug-log-only, no
    // player-facing clan-log entry, so no events to apply here.
    world.clan_registry.update_training(now_unix);

    applied
}

/// C `score_fight`'s `PlayerRuntime`-touching half (`arena.c:432-534`),
/// applied once `World::process_arena_master_actions`'s `check_fight` has
/// already determined a winner/loser this tick (queued as
/// `ArenaMasterEvent::FightScored` since `World` cannot reach
/// `ServerRuntime::players` - see `crates/ugaris-core/src/world/
/// arena.rs`'s module doc comment). Reads both combatants' pre-fight
/// scores first, then mutates each side with a single `&mut
/// PlayerRuntime` borrow at a time (`PlayerRuntime::apply_arena_win`/
/// `apply_arena_loss`, see their own doc comments for why that split
/// exists), and finally folds the resulting post-fight scores into
/// `World::arena_update_toplist` (C's `update_toplist` call inside
/// `score_fight` itself, `arena.c:533`).
///
/// A combatant may instead be a `CDR_ARENAFIGHTER` practice bot (no
/// `PlayerRuntime` at all) - `runtime.player_for_character` returns
/// `None` for it, so each side falls back to
/// `World::arena_fighter_score`/`apply_arena_fighter_win`/
/// `apply_arena_fighter_loss` (the bot's own local win/loss ledger, see
/// `ArenaFighterDriverData`'s doc comment) instead of skipping the event
/// outright.
pub(crate) fn apply_arena_master_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    now_unix: i64,
) -> usize {
    let mut applied = 0;
    for event in world.drain_pending_arena_master_events() {
        let ArenaMasterEvent::FightScored {
            winner_id,
            loser_id,
        } = event;
        let (Some(winner_name), Some(loser_name)) = (
            world.characters.get(&winner_id).map(|c| c.name.clone()),
            world.characters.get(&loser_id).map(|c| c.name.clone()),
        ) else {
            continue;
        };
        let winner_score_before = match runtime.player_for_character(winner_id) {
            Some(player) => Some(player.arena_score()),
            None => world.arena_fighter_score(winner_id),
        };
        let Some(winner_score_before) = winner_score_before else {
            continue;
        };
        let loser_score_before = match runtime.player_for_character(loser_id) {
            Some(player) => Some(player.arena_score()),
            None => world.arena_fighter_score(loser_id),
        };
        let Some(loser_score_before) = loser_score_before else {
            continue;
        };
        let now = i32::try_from(now_unix).unwrap_or(i32::MAX);
        let new_winner_score = if runtime.player_for_character(winner_id).is_some() {
            runtime
                .player_for_character_mut(winner_id)
                .map(|p| p.apply_arena_win(loser_score_before, now))
        } else {
            world.apply_arena_fighter_win(winner_id, loser_score_before)
        };
        let Some(new_winner_score) = new_winner_score else {
            continue;
        };
        let new_loser_score = if runtime.player_for_character(loser_id).is_some() {
            runtime
                .player_for_character_mut(loser_id)
                .map(|p| p.apply_arena_loss(winner_score_before, now))
        } else {
            world.apply_arena_fighter_loss(loser_id, winner_score_before)
        };
        let Some(new_loser_score) = new_loser_score else {
            continue;
        };
        world.arena_update_toplist(
            &winner_name,
            &loser_name,
            new_winner_score,
            new_loser_score,
            now_unix,
        );
        applied += 1;
    }
    applied
}

/// C `command.c`'s `lastseen:` handler's async DB round-trip
/// (`lastseen`/`db_lastseen`, `database_lookup.c:142-157` +
/// `database_notes.c:352-390`): resolves every `World::
/// drain_pending_lastseen_lookups` entry (queued by validly-shaped
/// `/lastseen <name>` arguments - see `World::queue_lastseen_lookup`'s
/// doc comment for the synchronous invalid-name fast path) against the
/// DB and delivers the reply via `World::queue_system_text` (C's
/// `tell_chat(0, rID, 1, ...)`, this codebase's direct-to-character
/// system-text channel).
///
/// Message shape mirrors `db_lastseen` exactly:
/// - no DB row -> "No character by the name %s." - the exact same text
///   the command dispatcher's own `lookup_name` `== -1` branch uses
///   (`command.c:9041`), since a player can't tell the two cases apart.
/// - `CF_GOD` row -> "%s was seen quite recently." (C never computes an
///   elapsed time for staff, `database_notes.c:378-379`).
/// - otherwise -> "%s was last seen %d days, %d hours, %d minutes ago.",
///   from `now - last_activity` where `last_activity` is `LastSeenInfo::
///   last_activity_unix` (already `max(login_time, logout_time,
///   created_at)`, computed in SQL - see `ugaris-db`'s `FIND_LAST_SEEN_SQL`
///   doc comment).
///
/// No-ops entirely (silent, matching every other offline-DB-lookup event
/// in this file) when no `character_repository` is configured or the
/// query itself errors.
pub(crate) async fn apply_lastseen_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    now_unix: i64,
) -> usize {
    let lookups = world.drain_pending_lastseen_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let reply = match repository.find_last_seen(&lookup.target_name).await {
            Ok(Some(info)) => lastseen_reply_message(&info, now_unix),
            Ok(None) => format!("No character by the name {}.", lookup.target_name),
            Err(_) => continue,
        };
        world.queue_system_text(lookup.requester_id, reply);
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_lastseen_events`], split out so
/// the day/hour/minute arithmetic (`database_notes.c:381-386`) can be
/// unit-tested without a live database.
fn lastseen_reply_message(info: &ugaris_db::LastSeenInfo, now_unix: i64) -> String {
    if info.is_god {
        return format!("{} was seen quite recently.", info.name);
    }
    let elapsed = now_unix - info.last_activity_unix;
    format!(
        "{} was last seen {} days, {} hours, {} minutes ago.",
        info.name,
        elapsed / (60 * 60 * 24),
        (elapsed / (60 * 60)) % 24,
        (elapsed / 60) % 60
    )
}

/// `#acstatus <name>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for why this needs a Postgres
/// round trip in this codebase where C reads its `player[nr]->ac` struct
/// synchronously. Reproduces `ac_cmd_status`'s display block
/// (`anticheat.c:492-516`) as a sequence of `World::queue_system_text`
/// calls, one per line (matching that C function's own one-`log_char`-
/// per-line shape) - see `ac_status_lines` for the exact text. A session
/// row that no longer exists (deleted, or a stale id) is silently
/// skipped, matching every other offline-DB-lookup event in this file.
pub(crate) async fn apply_ac_status_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_status_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(info)) = repository.find_session(lookup.session_id).await else {
            continue;
        };
        for line in ac_status_lines(&lookup.target_name, &info) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_status_events`], split out
/// so it can be unit-tested without a live database. C `ac_cmd_status`
/// (`anticheat.c:492-516`) - color wrapping dropped, matching `/global`'s
/// established plain-text simplification for admin-only displays.
fn ac_status_lines(target_name: &str, info: &ugaris_db::AntiCheatSessionInfo) -> Vec<String> {
    let mut lines = vec![
        format!("--- Anti-Cheat Status for {target_name} ---"),
        format!("Status: {}", ac_status_string(info.status)),
        format!("Heartbeat violations: {}", info.heartbeat_violations),
        format!("State violations: {}", info.state_violations),
        format!("Challenge failures: {}", info.challenge_failures),
        format!("Bot score: {:.2}", info.bot_score),
        format!("Timeout count: {}", info.timeout_count),
    ];
    if let (Some(major), Some(minor), Some(patch)) =
        (info.mod_major, info.mod_minor, info.mod_patch)
    {
        lines.push(format!("Mod version: {major}.{minor}.{patch}"));
        let os_name = match info.os_type {
            Some(1) => "Windows",
            Some(2) => "Linux",
            Some(3) => "macOS",
            _ => "Unknown",
        };
        lines.push(format!("OS: {os_name}"));
        lines.push(format!(
            "Screen: {}x{}",
            info.screen_w.unwrap_or(0),
            info.screen_h.unwrap_or(0)
        ));
    } else {
        lines.push("Fingerprint: not received".to_string());
    }
    lines
}

/// `#acstats`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_stats`
/// (`anticheat.c:604-628`): per-status tallies over every online
/// `CF_PLAYER` character with a known anticheat session (see the module
/// doc comment for why a session-less online player is simply omitted,
/// not counted as "unverified" by default), plus the single highest
/// `bot_score` and its owner's name. A target whose session row has
/// vanished between the command and this tick is omitted from every
/// tally, matching `find_sessions`'s own silent-omission contract.
pub(crate) async fn apply_ac_stats_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_stats_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let session_ids: Vec<i64> = lookup
            .targets
            .iter()
            .map(|target| target.session_id)
            .collect();
        let Ok(sessions) = repository.find_sessions(&session_ids).await else {
            continue;
        };
        let sessions_by_id: HashMap<i64, ugaris_db::AntiCheatSessionInfo> =
            sessions.into_iter().collect();

        let mut total_players = 0;
        let mut verified = 0;
        let mut unverified = 0;
        let mut suspicious = 0;
        let mut flagged = 0;
        let mut with_fingerprint = 0;
        let mut max_bot_score = 0.0f32;
        let mut max_bot_player = String::new();
        for target in &lookup.targets {
            let Some(info) = sessions_by_id.get(&target.session_id) else {
                continue;
            };
            total_players += 1;
            match info.status {
                1 => verified += 1,
                0 => unverified += 1,
                2 => suspicious += 1,
                3 => flagged += 1,
                _ => {}
            }
            if info.mod_major.is_some() {
                with_fingerprint += 1;
            }
            if info.bot_score > max_bot_score {
                max_bot_score = info.bot_score;
                max_bot_player = target.name.clone();
            }
        }

        world.queue_system_text(
            lookup.caller_id,
            "--- Anti-Cheat Global Statistics ---".to_string(),
        );
        world.queue_system_text(lookup.caller_id, format!("Total players: {total_players}"));
        world.queue_system_text(lookup.caller_id, format!("Verified: {verified}"));
        world.queue_system_text(lookup.caller_id, format!("Unverified: {unverified}"));
        world.queue_system_text(lookup.caller_id, format!("Suspicious: {suspicious}"));
        world.queue_system_text(lookup.caller_id, format!("Flagged: {flagged}"));
        world.queue_system_text(
            lookup.caller_id,
            format!("With fingerprint: {with_fingerprint}"),
        );
        if max_bot_score > 0.0 {
            world.queue_system_text(
                lookup.caller_id,
                format!("Highest bot score: {max_bot_score:.2} ({max_bot_player})"),
            );
        }
        applied += 1;
    }
    applied
}

/// `#aclist`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_list`
/// (`anticheat.c:721-753`): one line per online `CF_PLAYER` character
/// with a known anticheat session (padding/color dropped, matching
/// `/global`'s established plain-text simplification), in the same
/// ascending-character-id order the command handler gathered `targets`
/// in, followed by a trailing "Total: N players" count.
pub(crate) async fn apply_ac_list_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_list_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let session_ids: Vec<i64> = lookup
            .targets
            .iter()
            .map(|target| target.session_id)
            .collect();
        let Ok(sessions) = repository.find_sessions(&session_ids).await else {
            continue;
        };
        let sessions_by_id: HashMap<i64, ugaris_db::AntiCheatSessionInfo> =
            sessions.into_iter().collect();

        world.queue_system_text(
            lookup.caller_id,
            "--- Online Players AC Status ---".to_string(),
        );
        let mut count = 0;
        for target in &lookup.targets {
            let Some(info) = sessions_by_id.get(&target.session_id) else {
                continue;
            };
            world.queue_system_text(
                lookup.caller_id,
                format!(
                    "{:<16} {:<10} Bot:{:.2} HB:{} St:{} Ch:{}",
                    target.name,
                    ac_status_string(info.status),
                    info.bot_score,
                    info.heartbeat_violations,
                    info.state_violations,
                    info.challenge_failures
                ),
            );
            count += 1;
        }
        world.queue_system_text(lookup.caller_id, format!("Total: {count} players"));
        applied += 1;
    }
    applied
}

/// `#acsuspicious`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_suspicious`
/// (`anticheat.c:754-780`): one line per online `CF_PLAYER` character
/// with a known anticheat session whose status is
/// `>= AC_STATUS_SUSPICIOUS` (padding/color dropped, matching `/global`'s
/// established plain-text simplification), in the same ascending-
/// character-id order the command handler gathered `targets` in,
/// followed by a trailing "Total: N players" count - or, if none
/// qualify, C's own "No suspicious or flagged players online." (the
/// zero-count message is genuinely different text from `#aclist`'s,
/// copied letter for letter).
pub(crate) async fn apply_ac_suspicious_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_suspicious_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let session_ids: Vec<i64> = lookup
            .targets
            .iter()
            .map(|target| target.session_id)
            .collect();
        let Ok(sessions) = repository.find_sessions(&session_ids).await else {
            continue;
        };
        let sessions_by_id: HashMap<i64, ugaris_db::AntiCheatSessionInfo> =
            sessions.into_iter().collect();

        world.queue_system_text(
            lookup.caller_id,
            "--- Suspicious/Flagged Players ---".to_string(),
        );
        let mut count = 0;
        for target in &lookup.targets {
            let Some(info) = sessions_by_id.get(&target.session_id) else {
                continue;
            };
            if info.status < AC_STATUS_SUSPICIOUS {
                continue;
            }
            world.queue_system_text(
                lookup.caller_id,
                format!(
                    "{} - {} (Bot: {:.2}, HB: {}, State: {}, Chal: {})",
                    target.name,
                    ac_status_string(info.status),
                    info.bot_score,
                    info.heartbeat_violations,
                    info.state_violations,
                    info.challenge_failures
                ),
            );
            count += 1;
        }
        if count == 0 {
            world.queue_system_text(
                lookup.caller_id,
                "No suspicious or flagged players online.".to_string(),
            );
        } else {
            world.queue_system_text(lookup.caller_id, format!("Total: {count} players"));
        }
        applied += 1;
    }
    applied
}

/// `#accleanup <days>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_cleanup`
/// (`anticheat.c:1267-1285`): deletes `anticheat_sessions` rows older
/// than `days` (`AntiCheatRepository::cleanup_old_records`, already
/// ported in iteration 196) and reports the row count back to the
/// caller. C also deletes from a separate `ac_heartbeat_log` table
/// (`db_ac_cleanup_heartbeat_logs`) this codebase has no equivalent of
/// (heartbeat counters live on the session row itself) - the reported
/// count for that half is always `0`, matching C's own always-present
/// "%d heartbeat logs deleted" clause rather than dropping it. A failed
/// delete (DB error) is silently skipped, matching every other offline-
/// DB-lookup event in this file - no error message reaches the caller,
/// same as a vanished session row elsewhere in this module.
pub(crate) async fn apply_ac_cleanup_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_cleanup_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(deleted) = repository.cleanup_old_records(lookup.days).await else {
            continue;
        };
        let heartbeat_logs_deleted = 0;
        world.queue_system_text(
            lookup.caller_id,
            format!(
                "Cleanup complete: {deleted} sessions, {heartbeat_logs_deleted} heartbeat logs deleted."
            ),
        );
        applied += 1;
    }
    applied
}

/// `#acreset <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_reset`
/// (`anticheat.c:527-561`): zeroes the target session's violation
/// counters/bot score and restores `status` to `AC_STATUS_VERIFIED`
/// (`AntiCheatRepository::reset_session`). C's confirmation is
/// unconditional and same-thread (mutating an in-memory struct always
/// succeeds); here the "Reset anti-cheat data for {name}." message is
/// only queued once the async update actually reports a row was
/// touched, matching every other offline-DB-mutation event in this
/// file's silent-skip-on-failure convention (a vanished session row
/// between the command and the tick loop draining it, or a DB error,
/// produces no reply at all).
pub(crate) async fn apply_ac_reset_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_reset_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(true) = repository.reset_session(lookup.session_id).await else {
            continue;
        };
        world.queue_system_text(
            lookup.caller_id,
            format!("Reset anti-cheat data for {}.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

/// `#acflag <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_flag`
/// (`anticheat.c:568-593`): sets the target session's `status` to
/// `AC_STATUS_FLAGGED` (`AntiCheatRepository::set_status`). C's
/// confirmation is unconditional and same-thread; here the "Manually
/// flagged {name} for review." message is only queued once the async
/// update actually reports a row was touched, matching every other
/// offline-DB-mutation event in this file's silent-skip-on-failure
/// convention.
pub(crate) async fn apply_ac_flag_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_flag_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(true) = repository
            .set_status(lookup.session_id, AC_STATUS_FLAGGED)
            .await
        else {
            continue;
        };
        world.queue_system_text(
            lookup.caller_id,
            format!("Manually flagged {} for review.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

/// `#acunflag <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_unflag`
/// (`anticheat.c:790-823`): unlike every other member of this family,
/// C's own handler gates on the target's *current* status before
/// mutating anything (`status != AC_STATUS_FLAGGED` -> "is not flagged",
/// a synchronous in-memory read there) - here that gate has to happen
/// after the async `find_session` round trip instead, since this
/// codebase has no in-memory struct to read status from synchronously.
/// A vanished session row is silently skipped (matching every other
/// offline-DB-lookup event's convention), but a session that exists and
/// simply isn't flagged still gets the "is not flagged" reply - a
/// genuine (documented) deviation from the pure silent-skip convention,
/// justified because C's own equivalent branch produces user-facing
/// text too, not a silent no-op. Once past the gate: restores `status`
/// to `AC_STATUS_VERIFIED` (`AntiCheatRepository::set_status`, same as
/// `#acreset`) and flips `ac_player_stats.is_flagged` to `false` for the
/// target's subscriber id (`AntiCheatRepository::set_flagged`, resolved
/// via `account_id_for_session` - see that method's doc comment for why
/// account id isn't threaded through `PlayerRuntime` instead). C's own
/// confirmation is unconditional once past the status gate, even when
/// `target_subscriber <= 0` skips the DB writes entirely
/// (`anticheat.c:816-821`); reproduced here by queuing the confirmation
/// regardless of whether `account_id_for_session` resolved anything,
/// since only the session-status update (guaranteed to succeed, the row
/// having just been read a moment earlier) gates the reply, matching
/// C's real branching exactly rather than this file's usual "reply only
/// once the mutation succeeds" simplification.
pub(crate) async fn apply_ac_unflag_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_unflag_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(info)) = repository.find_session(lookup.session_id).await else {
            continue;
        };
        if info.status != AC_STATUS_FLAGGED {
            world.queue_system_text(
                lookup.caller_id,
                format!("Player '{}' is not flagged.", lookup.target_name),
            );
            continue;
        }
        let Ok(true) = repository
            .set_status(lookup.session_id, AC_STATUS_VERIFIED)
            .await
        else {
            continue;
        };
        if let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await {
            let _ = repository.set_flagged(account_id, false).await;
        }
        world.queue_system_text(
            lookup.caller_id,
            format!("Removed flagged status from {}.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

/// `#actrust <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_trust`
/// (`anticheat.c:827-849`): no status gate at all (unlike `#acunflag`),
/// just flips `ac_player_stats.is_trusted` to `true` for the target's
/// subscriber id, resolved via `account_id_for_session` from the
/// already-known session id. Unlike `#acunflag`'s unconditional-once-
/// past-the-gate reply, this codebase's confirmation is only queued once
/// the subscriber id actually resolves and the write succeeds - a
/// documented simplification vs. C's true unconditional reply
/// (`anticheat.c:847-848`, sent even when `target_subscriber <= 0` skips
/// the DB write), justified because a real character's account id is
/// essentially always resolvable here (unlike C's genuinely-fallible
/// synchronous DB lookup at the time `ac_cmd_trust` runs), so the gap
/// only matters for an already-vanished session row - the same case
/// every other offline-DB-mutation event in this file silently skips.
pub(crate) async fn apply_ac_trust_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_trust_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            continue;
        };
        if repository.set_trusted(account_id, true).await.is_err() {
            continue;
        }
        world.queue_system_text(
            lookup.caller_id,
            format!("Marked {} as trusted.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

/// `#acuntrust <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. The "untrust" mirror of
/// `apply_ac_trust_events` (`ac_cmd_untrust`, `anticheat.c:860-882`):
/// identical shape, `set_trusted(account_id, false)` instead of `true`.
pub(crate) async fn apply_ac_untrust_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_untrust_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            continue;
        };
        if repository.set_trusted(account_id, false).await.is_err() {
            continue;
        }
        world.queue_system_text(
            lookup.caller_id,
            format!("Removed trusted status from {}.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

/// `#acwarn <player> [reason]`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_warn`
/// (`anticheat.c:1291-1314`): resolves the subscriber id
/// (`get_subscriberId_from_character`, here `account_id_for_session`) -
/// a `None` result mirrors C's synchronous `subscriber_id <= 0` ->
/// "Could not find subscriber for '{name}'." branch, the one case this
/// event actually skips the rest of the work for. Once a subscriber id
/// is found, C calls `db_ac_issue_warning` *without checking its return
/// value* and then unconditionally sends all four messages (two to the
/// target, two to the caller) - reproduced as-is here too (the `issue_
/// warning` DB write's `Result` is deliberately ignored, matching C's own
/// disregard for it, rather than this file's usual "reply only once the
/// mutation succeeds" convention).
pub(crate) async fn apply_ac_warn_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_warn_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            world.queue_system_text(
                lookup.caller_id,
                format!("Could not find subscriber for '{}'.", lookup.target_name),
            );
            continue;
        };
        let _ = repository.issue_warning(account_id).await;
        world.queue_system_text_bytes(
            lookup.target_id,
            legacy_light_red_text_bytes("*** WARNING ***"),
        );
        world.queue_system_text(
            lookup.target_id,
            format!("You have received an anti-cheat warning: {}", lookup.reason),
        );
        world.queue_system_text(
            lookup.target_id,
            "Further violations may result in suspension.".to_string(),
        );
        world.queue_system_text(
            lookup.caller_id,
            format!(
                "Issued warning to {}: {}",
                lookup.target_name, lookup.reason
            ),
        );
        applied += 1;
    }
    applied
}

/// `#acsessions <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_sessions`
/// (`anticheat.c:975-1017`): resolves the subscriber id the same way
/// `apply_ac_trust_events` does (`account_id_for_session`), then queries
/// up to 10 recent sessions (`AntiCheatRepository::recent_sessions`,
/// matching C's own `sessions[10]` stack array / `db_ac_get_recent_
/// sessions(..., 10)` call). An unresolvable subscriber id is silently
/// skipped (no reply at all), matching the module doc comment's
/// established "row deleted or unknown id -> silent skip" convention
/// (unlike `#acwarn`, this command has no C-side `subscriber_id <= 0`
/// branch to reproduce, since C's own `ac_find_player` guarantees an
/// online connection exists and thus always has *some* row).
pub(crate) async fn apply_ac_sessions_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sessions_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            continue;
        };
        let Ok(rows) = repository.recent_sessions(account_id, 10).await else {
            continue;
        };
        for line in ac_sessions_lines(&lookup.target_name, &rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_sessions_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_status_lines`. C `ac_cmd_sessions`
/// (`anticheat.c:993-1017`) - color wrapping dropped, matching `ac_
/// status_lines`'s/`/global`'s plain-text simplification for admin-only
/// displays.
fn ac_sessions_lines(
    target_name: &str,
    rows: &[ugaris_db::AntiCheatSessionHistoryRow],
) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("No sessions found for {target_name}.")];
    }
    let mut lines = vec![format!("--- Recent Sessions for {target_name} ---")];
    for row in rows {
        lines.push(format!(
            "{} ({}m) {} Bot:{:.2} V:{}/{}/{}/{}",
            row.start_time,
            row.duration_minutes,
            ac_status_string(row.status),
            row.bot_score,
            row.heartbeat_violations,
            row.state_violations,
            row.challenge_failures,
            row.anomaly_count,
        ));
    }
    lines
}

/// `#acviolations <player>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcViolationsLookup`'s own
/// doc comment for why this is the same single-name-target resolution
/// shape as `#acsessions`.
pub(crate) async fn apply_ac_violations_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_violations_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            continue;
        };
        let Ok(rows) = repository.recent_violations(account_id, 15).await else {
            continue;
        };
        for line in ac_violations_lines(&lookup.target_name, &rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_violations_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_sessions_lines`. C `ac_cmd_violations`
/// (`anticheat.c:1043-1053`) - color wrapping (severity-based
/// red/orange/yellow) dropped, matching `ac_sessions_lines`'s/`ac_
/// status_lines`'s plain-text simplification for admin-only displays;
/// the numeric severity is kept in the line itself instead so the
/// information isn't lost entirely.
fn ac_violations_lines(
    target_name: &str,
    rows: &[ugaris_db::AntiCheatViolationRow],
) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("No violations found for {target_name}.")];
    }
    let mut lines = vec![format!("--- Recent Violations for {target_name} ---")];
    for row in rows {
        lines.push(format!(
            "{} [{}] sev={} {}",
            row.detected_at,
            row.type_name,
            row.severity,
            row.details.as_deref().unwrap_or(""),
        ));
    }
    lines
}

/// `#achistory <player>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcHistoryLookup`'s own
/// doc comment for why this is the same single-name-target resolution
/// shape as `#acsessions`/`#acviolations`. Unlike those two siblings,
/// this reads a single lifetime rollup row
/// (`AntiCheatRepository::find_player_stats`) rather than a list of
/// per-event rows.
pub(crate) async fn apply_ac_history_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_history_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            continue;
        };
        let Ok(stats) = repository.find_player_stats(account_id).await else {
            continue;
        };
        for line in ac_history_lines(&lookup.target_name, account_id, stats.as_ref()) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_history_events`], split out
/// so it can be unit-tested without a live database - same established
/// convention as `ac_sessions_lines`/`ac_violations_lines`. C `ac_cmd_
/// history` (`anticheat.c:924-972`) - color wrapping (risk-level-based
/// red/orange/yellow/green) dropped, matching this file's established
/// plain-text simplification for admin-only displays. Reproduces C's
/// exact 7-line body (plus the header) digit for digit, including the
/// `%d flagged, %d suspicious` comma placement.
fn ac_history_lines(
    target_name: &str,
    subscriber_id: i64,
    stats: Option<&ugaris_db::AntiCheatPlayerStatsRow>,
) -> Vec<String> {
    let Some(stats) = stats else {
        return vec![format!("No AC history found for {target_name}.")];
    };
    vec![
        format!("--- AC History for {target_name} (ID: {subscriber_id}) ---"),
        format!(
            "Sessions: {} total, {} flagged, {} suspicious",
            stats.total_sessions, stats.flagged_sessions, stats.suspicious_sessions
        ),
        format!(
            "Violations: HB={}, State={}, Challenge={}, Anomalies={}",
            stats.total_heartbeat_violations,
            stats.total_state_violations,
            stats.total_challenge_failures,
            stats.total_anomalies
        ),
        format!(
            "Bot Score: max={:.2}, avg={:.2}",
            stats.max_session_bot_score, stats.avg_session_bot_score
        ),
        format!("Risk Level: {}", stats.risk_level),
        format!(
            "Flagged: {}, Trusted: {}, Warnings: {}",
            if stats.is_flagged { "YES" } else { "no" },
            if stats.is_trusted { "YES" } else { "no" },
            stats.warnings_issued
        ),
        format!("First seen: {}", stats.first_seen),
        format!("Last seen: {}", stats.last_seen.as_deref().unwrap_or("")),
    ]
}

/// `#acsharedip <player>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcSharedIpLookup`'s own
/// doc comment for why this is the same single-name-target resolution
/// shape as `#acsessions`/`#acviolations`/`#achistory`.
pub(crate) async fn apply_ac_sharedip_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sharedip_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            continue;
        };
        let Ok(rows) = repository.shared_ips(account_id, 20).await else {
            continue;
        };
        for line in ac_sharedip_lines(&lookup.target_name, &rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_sharedip_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_sessions_lines`/`ac_violations_lines`.
/// C `ac_cmd_sharedip` (`anticheat.c:1058-1088`) - color wrapping
/// dropped, matching this file's established plain-text simplification;
/// the trailing "Found %d accounts sharing IPs." summary line is
/// reproduced as-is. `email` is replaced by `username` throughout - see
/// `AntiCheatSharedIpRow`'s doc comment.
fn ac_sharedip_lines(target_name: &str, rows: &[ugaris_db::AntiCheatSharedIpRow]) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("No shared IPs found for {target_name}.")];
    }
    let mut lines = vec![format!("--- Accounts Sharing IP with {target_name} ---")];
    for row in rows {
        lines.push(format!(
            "{} - {} (sessions: {}, last: {})",
            row.username,
            std::net::Ipv4Addr::from(row.ip_address as u32),
            row.session_count,
            row.last_seen
        ));
    }
    lines.push(format!("Found {} accounts sharing IPs.", rows.len()));
    lines
}

/// `#acsharedhw <player>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcSharedHwLookup`'s own
/// doc comment for why this is the same single-name-target resolution
/// shape as `#acsharedip` above.
pub(crate) async fn apply_ac_sharedhw_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sharedhw_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            continue;
        };
        let Ok(rows) = repository.shared_hardware(account_id, 20).await else {
            continue;
        };
        for line in ac_sharedhw_lines(&lookup.target_name, &rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_sharedhw_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_sharedip_lines` above. C `ac_cmd_
/// sharedhw` (`anticheat.c:1096-1126`) - color wrapping dropped; `email`
/// replaced by `username`, matching `ac_sharedip_lines`.
fn ac_sharedhw_lines(target_name: &str, rows: &[ugaris_db::AntiCheatSharedHwRow]) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("No shared hardware found for {target_name}.")];
    }
    let mut lines = vec![format!(
        "--- Accounts Sharing Hardware with {target_name} ---"
    )];
    for row in rows {
        lines.push(format!(
            "{} - Hash: {}, Screen: {}x{} (last: {})",
            row.username,
            row.hardware_hash,
            row.screen_w.unwrap_or(0),
            row.screen_h.unwrap_or(0),
            row.last_seen
        ));
    }
    lines.push(format!("Found {} accounts sharing hardware.", rows.len()));
    lines
}

/// `#achighrisk`'s async round trip - see `ugaris-core`'s `world/
/// anticheat.rs` module doc comment for the general async-DB-round-trip
/// pattern this family shares. No name/session resolution at all (unlike
/// every other member of the family except `#acsiglist`/`#accleanup`),
/// so this simply lists every high-risk `ac_player_stats` row.
pub(crate) async fn apply_ac_highrisk_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_highrisk_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(rows) = repository.high_risk_players(20).await else {
            continue;
        };
        for line in ac_highrisk_lines(&rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_highrisk_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_siglist_lines`. C `ac_cmd_highrisk`
/// (`anticheat.c:1134-1157`) - risk-level-based color wrapping dropped;
/// `email` replaced by `username`, matching `ac_sharedip_lines`.
fn ac_highrisk_lines(rows: &[ugaris_db::AntiCheatHighRiskRow]) -> Vec<String> {
    if rows.is_empty() {
        return vec!["No high-risk players found.".to_string()];
    }
    let mut lines = vec!["--- High-Risk Players ---".to_string()];
    for row in rows {
        lines.push(format!(
            "[{}] {} - {} Bot:{:.2} Flag:{} (seen: {})",
            row.subscriber_id,
            row.username,
            row.risk_level,
            row.max_bot_score,
            row.flagged_sessions,
            row.last_seen.as_deref().unwrap_or("")
        ));
    }
    lines
}

/// `#aclookup <subscriber_id>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcLookupLookup`'s own doc
/// comment for why `subscriber_id` is parsed directly rather than
/// resolved from an online character name.
pub(crate) async fn apply_ac_lookup_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_lookup_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(result) = repository.lookup_subscriber(lookup.subscriber_id).await else {
            continue;
        };
        for line in ac_lookup_lines(lookup.subscriber_id, result.as_ref()) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_lookup_events`], split out
/// so it can be unit-tested without a live database - same established
/// convention as `ac_history_lines`. C `ac_cmd_lookup` (`anticheat.c:
/// 1158-1191`); the `"Email: %s"` line has no equivalent in this
/// codebase's schema (see `AntiCheatSubscriberLookup`'s doc comment) so
/// it is folded into the header line as `"--- Subscriber {id} ({
/// username}) ---"` instead of being dropped outright, keeping the
/// account's identity visible in the reply.
fn ac_lookup_lines(
    subscriber_id: i64,
    result: Option<&ugaris_db::AntiCheatSubscriberLookup>,
) -> Vec<String> {
    let Some(result) = result else {
        return vec![format!("Subscriber ID {subscriber_id} not found.")];
    };
    let mut lines = vec![format!(
        "--- Subscriber {subscriber_id} ({}) ---",
        result.username
    )];
    let Some(stats) = &result.stats else {
        lines.push("No AC data for this subscriber.".to_string());
        return lines;
    };
    lines.push(format!(
        "Sessions: {} total, {} flagged",
        stats.total_sessions, stats.flagged_sessions
    ));
    lines.push(format!(
        "Max Bot Score: {:.2}, Risk: {}",
        stats.max_session_bot_score, stats.risk_level
    ));
    lines.push(format!(
        "Flagged: {}, Trusted: {}",
        if stats.is_flagged { "YES" } else { "no" },
        if stats.is_trusted { "YES" } else { "no" }
    ));
    lines.push(format!(
        "First: {}, Last: {}",
        stats.first_seen,
        stats.last_seen.as_deref().unwrap_or("")
    ));
    lines
}

/// `#acsiglist`'s async round trip - see `ugaris-core`'s `world/
/// anticheat.rs` module doc comment for the general async-DB-round-trip
/// pattern this family shares. No name/session resolution at all (unlike
/// every other member of the family except `#accleanup`), so this simply
/// lists every row in the new `ac_known_signatures` table.
pub(crate) async fn apply_ac_siglist_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_siglist_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(rows) = repository.list_signatures(20).await else {
            continue;
        };
        for line in ac_siglist_lines(&rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_siglist_events`], split out
/// so it can be unit-tested without a live database - same established
/// convention as `ac_sessions_lines`/`ac_violations_lines`. C `ac_cmd_
/// siglist` (`anticheat.c:1192-1215`) - color wrapping (severity-based
/// red/orange/yellow highlighting on the name/severity) dropped, matching
/// this file's established plain-text simplification for admin-only
/// displays; the literal double-space quirk before `Det:` when a
/// signature has neither `auto_flag` nor `auto_ban` set (C's own format
/// string has a bare `" "` literal immediately followed by the two
/// optional `"Flag "`/`"Ban "` tokens, then another literal `" Det:"`) is
/// reproduced as-is, not "cleaned up".
fn ac_siglist_lines(rows: &[ugaris_db::AntiCheatSignatureRow]) -> Vec<String> {
    if rows.is_empty() {
        return vec!["No signatures defined.".to_string()];
    }
    let mut lines = vec!["--- Known Bad Signatures ---".to_string()];
    for row in rows {
        let flag = if row.auto_flag { "Flag " } else { "" };
        let ban = if row.auto_ban { "Ban " } else { "" };
        lines.push(format!(
            "[{}] {} ({}) Sev:{} {}{} Det:{}",
            row.id, row.name, row.signature_type, row.severity, flag, ban, row.times_detected,
        ));
    }
    lines
}

/// `#acsigadd <type> <value> <name>`'s async round trip - see `ugaris-
/// core`'s `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_
/// sigadd` (`anticheat.c:1216-1245`): inserts a new `ac_known_signatures`
/// row (`AntiCheatRepository::add_signature`). C's confirmation is
/// unconditional and same-thread; here the "Added signature: ..." message
/// is only queued once the async insert actually succeeds, matching every
/// other offline-DB-mutation event in this file's silent-skip-on-failure
/// convention (C's own "Failed to add signature." error path is likewise
/// only reachable when the query itself fails, so silently skipping the
/// reply on an `Err` here - rather than sending that exact text - loses
/// no user-facing branch C didn't already gate the same way).
pub(crate) async fn apply_ac_sigadd_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sigadd_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        if repository
            .add_signature(
                &lookup.sig_type,
                &lookup.sig_value,
                &lookup.name,
                &lookup.created_by,
            )
            .await
            .is_err()
        {
            continue;
        }
        world.queue_system_text(
            lookup.caller_id,
            format!(
                "Added signature: {} ({}) = {}",
                lookup.name, lookup.sig_type, lookup.sig_value
            ),
        );
        applied += 1;
    }
    applied
}

/// `#acsigdel <id>`'s async round trip - see `ugaris-core`'s `world/
/// anticheat.rs` module doc comment. Reproduces `ac_cmd_sigdel`
/// (`anticheat.c:1246-1266`): deletes the named `ac_known_signatures` row
/// (`AntiCheatRepository::delete_signature`). Unlike most siblings in
/// this family, C's own "not found" branch (`affected == 0`) is itself
/// user-facing text, not a silent skip - reproduced here by checking the
/// mutator's `bool` result rather than only its `Result::Ok`-ness.
pub(crate) async fn apply_ac_sigdel_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sigdel_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(found) = repository.delete_signature(lookup.signature_id).await else {
            continue;
        };
        world.queue_system_text(
            lookup.caller_id,
            if found {
                format!("Deleted signature ID {}.", lookup.signature_id)
            } else {
                format!("Signature ID {} not found.", lookup.signature_id)
            },
        );
        applied += 1;
    }
    applied
}

/// `#querystats`/`/querystats`'s async round trip - see `ugaris-core`'s
/// `world/querystats.rs` module doc comment for exactly which C counters
/// this scoped-down port tracks (and why the rest are omitted rather than
/// faked). `PgCharacterRepository::query_stats` is a synchronous
/// in-memory atomic read, not a real query, but is still routed through
/// this tick-loop drain (rather than answered directly in
/// `commands_admin.rs`) since command dispatch has no visibility into
/// `character_repository` - the same architectural constraint every
/// other DB-backed command in this file works around.
///
/// No-ops entirely (silent) when no `character_repository` is configured,
/// matching every sibling offline-DB-lookup event's convention.
pub(crate) fn apply_querystats_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_querystats_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let stats = repository.query_stats();
        for line in querystats_lines(stats) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure formatting half of `apply_querystats_events`, split out for unit
/// testing without needing a live `PgCharacterRepository` - matches
/// `ac_status_lines`'s established pattern for this file. Reproduces C's
/// `"Database Query Statistics:"` header and `"Character operations:"`
/// subheader/line verbatim (`command.c:6596,6601-604`); every other C
/// line (`Total queries`/`Average query time`/`Other operations`/`Query
/// type statistics`) is omitted, not faked, since nothing in `ugaris-db`
/// increments those counters - see `ugaris-core`'s `world/querystats.rs`
/// module doc comment.
fn querystats_lines(stats: ugaris_db::CharacterQueryStats) -> Vec<String> {
    vec![
        "Database Query Statistics:".to_string(),
        "Character operations:".to_string(),
        format!(
            "Save chars: {}, Exit chars: {}, Load chars: {}",
            stats.save_char_cnt, stats.exit_char_cnt, stats.load_char_cnt
        ),
    ]
}

/// `/jail`/`/unjail`'s async DB round trip (C `lookup_name`,
/// `system/lookup.c:42-98` + `system/database/database_lookup.c:57-83`):
/// resolves every `World::drain_pending_jail_lookups` entry (queued by a
/// validly-shaped `/jail`/`/unjail <name>` argument - see `World::
/// queue_jail_lookup`'s and `apply_admin_character_command`'s doc
/// comments) against the DB.
///
/// - no DB row -> "No character by the name %s." (C's dispatcher-level
///   `lookup_name == -1` branch, `command.c:9041`-equivalent for
///   `jail`/`unjail`).
/// - a row found -> hands off to `World::resolve_jail_lookup`, which
///   reproduces `cmd_jail_player`/`cmd_unjail_player`'s own separate
///   online-only `CF_PLAYER` name scan and, on a match, applies the
///   jail/unjail mutation (no match -> "No player by that name.", the
///   exact text both C functions share).
///
/// No-ops entirely (silent) when no `character_repository` is configured
/// or a query errors, matching every sibling offline-DB-lookup event.
pub(crate) async fn apply_jail_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_jail_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository.find_login_target(&lookup.target_name).await {
            Ok(Some(_)) => {
                world.resolve_jail_lookup(lookup.caller_id, &lookup.target_name, lookup.action);
            }
            Ok(None) => {
                world.queue_system_text(
                    lookup.caller_id,
                    format!("No character by the name {}.", lookup.target_name),
                );
            }
            Err(_) => continue,
        }
        applied += 1;
    }
    applied
}

/// `/jail`/`/unjail`'s cross-area hand-off (C `change_area(cn, resta,
/// restx, resty)`, `src/system/tool.c:4392-4425`'s tail): resolves every
/// `World::drain_pending_jail_cross_area_transfers` entry (queued by
/// `World::apply_jail_action` when the jail/aston destination area
/// differs from this area server's own `area_id` - see `world/jail.rs`'s
/// module doc comment) via the shared `attempt_cross_area_transfer`
/// helper, same as the `TransportTravel`/`ClanSpawnExit`/`MineGateway`/
/// `/office`+`/goto` call sites. The destination mirror always equals
/// this process's own `mirror_id`: neither jail nor aston locations carry
/// a mirror field of their own (matching C's `change_area` reading
/// `ch[cn].mirror`, i.e. the target character's *own current* mirror,
/// which under this codebase's single-process-per-area-mirror stance is
/// always this process's `mirror_id`).
pub(crate) async fn apply_jail_cross_area_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_jail_cross_area_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        let transferred = attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.target_id,
            transfer.target_area,
            u32::from(mirror_id),
            transfer.target_x,
            transfer.target_y,
        )
        .await;
        if !transferred {
            world.queue_system_text(
                transfer.caller_id,
                "Nothing happens - target area server is down.".to_string(),
            );
        }
        applied += 1;
    }
    applied
}

/// The Macro Daemon's cross-server "challenge room" hand-off (C
/// `change_area`, `src/module/base.c:1110` for the suspicion-triggered
/// banishment, `848-850` for the correct-answer return trip): resolves
/// every `World::drain_pending_macro_cross_area_transfers` entry (queued
/// by `ugaris-server/src/macro_daemon.rs` when the challenge-room/
/// original-area destination differs from this area server's own
/// `area_id` - see `world/macro_npc.rs`'s module doc comment) via the
/// shared `attempt_cross_area_transfer` helper, same as every other
/// cross-area call site. Like C's own `change_area` call sites here, a
/// failed hand-off is not specially handled - C never checks `change_
/// area`'s return value at either call site either, so a down target
/// area server simply leaves the character in place with no message
/// (weaker than `apply_dungeon_eviction_transfers`'s "system-triggered,
/// no caller to notify" precedent, which at least falls back to
/// `remove_character` - not needed here since `attempt_cross_area_
/// transfer` itself already guarantees no despawn happened on a lookup
/// failure, so "leave the character exactly where they were" is already
/// the correct fallback with no extra code).
pub(crate) async fn apply_macro_cross_area_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_macro_cross_area_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.character_id,
            transfer.target_area,
            u32::from(mirror_id),
            transfer.target_x,
            transfer.target_y,
        )
        .await;
        applied += 1;
    }
    applied
}

/// `build_remove_tile`'s evicted-player cross-area rescue (C
/// `change_area(cn, ch[cn].resta, ch[cn].restx, ch[cn].resty)`,
/// `src/area/13/dungeon.c:754`'s tail): resolves every `World::
/// drain_pending_dungeon_eviction_transfers` entry (queued by
/// `World::build_remove_tile` when the evicted player's own `rest_area`
/// differs from this area server's own `area_id` - see
/// `world/dungeon_master.rs`'s module doc comment) via the shared
/// `attempt_cross_area_transfer` helper, same as every other cross-area
/// call site. The destination mirror always equals this process's own
/// `mirror_id` (rest points carry no mirror field of their own, matching
/// C's `change_area` reading `ch[cn].mirror`). Unlike every other
/// call site, C's own fallback on failure is `exit_char(cn)` (no
/// message - the character has no "down" feedback path here since
/// `exit_char` disconnects them entirely), so a failed hand-off calls
/// `World::remove_character` instead of queuing a system text.
pub(crate) async fn apply_dungeon_eviction_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_dungeon_eviction_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        let transferred = attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.character_id,
            transfer.target_area,
            u32::from(mirror_id),
            transfer.target_x,
            transfer.target_y,
        )
        .await;
        if !transferred {
            world.remove_character(transfer.character_id);
        }
        applied += 1;
    }
    applied
}

/// `/rmdeath`'s async DB round trip (C `lookup_name`, `system/lookup.c:
/// 42-98` + `system/database/database_lookup.c:57-83`): resolves every
/// `World::drain_pending_rmdeath_lookups` entry (queued by a
/// validly-shaped `/rmdeath <name>` argument - see `World::
/// queue_rmdeath_lookup`'s and `apply_admin_character_command`'s doc
/// comments) against the DB.
///
/// - no DB row -> "No character by the name %s." (C's dispatcher-level
///   `lookup_name == -1` branch, `command.c:8896`-equivalent).
/// - a row found -> hands off to `World::resolve_rmdeath_lookup`, which
///   reproduces `cmd_removedeath`'s online-only deviation (see
///   `world/rmdeath.rs`'s module doc comment) and, on a match, decrements
///   the target's `deaths` counter (no match -> "No player by that
///   name.").
///
/// No-ops entirely (silent) when no `character_repository` is configured
/// or a query errors, matching every sibling offline-DB-lookup event.
pub(crate) async fn apply_rmdeath_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_rmdeath_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository.find_login_target(&lookup.target_name).await {
            Ok(Some(_)) => {
                world.resolve_rmdeath_lookup(lookup.caller_id, &lookup.target_name);
            }
            Ok(None) => {
                world.queue_system_text(
                    lookup.caller_id,
                    format!("No character by the name {}.", lookup.target_name),
                );
            }
            Err(_) => continue,
        }
        applied += 1;
    }
    applied
}

/// `cmd_complain`'s async DB round trip (C `command.c:2320-2350`,
/// `lookup_name`/`db_lookup_name`, `system/lookup.c:42-98` +
/// `system/database/database_lookup.c:57-83`): resolves every `World::
/// drain_pending_complain_lookups` entry (queued by a validly-shaped
/// `/complain <name>` argument - see `World::queue_complain_lookup`'s and
/// `ugaris-server`'s `apply_complain_command`'s doc comments for every
/// other, purely synchronous branch) against the DB.
///
/// - no DB row -> "Sorry, no player by the name '%s' found." delivered
///   via `World::queue_system_text` (matching `cmd_complain`'s own
///   `ret < 0` branch, `command.c:2341-2343`).
/// - a row found -> `ppd->complaint_date = realtime;` (`command.c:2346`)
///   is applied to the *requester's* own `PlayerRuntime` if they're still
///   online (a real gap from C, where the whole function runs inside one
///   blocking call so the caller can never have logged out mid-lookup;
///   silently skipped here otherwise, matching every other
///   offline-DB-lookup event in this file) plus the "Your complaint about
///   '%s' has been sent to game management." confirmation, using the
///   DB's properly-capitalized name (C's `realname` out-parameter).
///   `write_scrollback` (emailing the complaint) has no Rust equivalent -
///   see `apply_complain_command`'s doc comment.
///
/// No-ops entirely (silent) when no `character_repository` is configured
/// or a query errors, matching every sibling offline-DB-lookup event.
pub(crate) async fn apply_complain_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    now_unix: i64,
) -> usize {
    let lookups = world.drain_pending_complain_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let found_name = match repository.find_login_target(&lookup.target_name).await {
            Ok(Some(summary)) => summary.name,
            Ok(None) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Sorry, no player by the name '{}' found.",
                        lookup.target_name
                    ),
                );
                continue;
            }
            Err(_) => continue,
        };
        if let Some(player) = runtime.player_for_character_mut(lookup.requester_id) {
            player.record_complaint(now_unix as i32);
        }
        world.queue_system_text(
            lookup.requester_id,
            format!("Your complaint about '{found_name}' has been sent to game management."),
        );
        applied += 1;
    }
    applied
}

/// C `cmd_flag`'s offline fallback, `task_set_flags`/`set_flags`
/// (`task.c:198-211,385-394`), resolved for every `World::
/// drain_pending_admin_flag_toggles` entry queued by `World::
/// apply_cmd_flag_command` (see that method's doc comment and
/// `world/admin_flag.rs`'s module doc comment for the full message-shape
/// breakdown):
/// - no DB row at all -> "Sorry, no player by the name %s." (C's
///   synchronous `lookup_name == -1` case, deferred here since this
///   codebase has no synchronous name-index cache to check first).
/// - a row found -> immediate "Update scheduled." feedback
///   (`command.c:2896`), sent regardless of whether the mutation below
///   actually succeeds (C's fire-and-forget `task_set_flags` semantics).
/// - target already online elsewhere -> silent no-op beyond the above
///   (C `set_task`'s "online somewhere else" guard, `task.c:250-253`,
///   only `xlog`s).
/// - otherwise -> mutate the flag, guarded save
///   (`CharacterSaveMode::Backup`, pinning the expected offline
///   `current_area`/`current_mirror` exactly like every other
///   offline-DB-mutation event in this file), then `"Set flag on %s to
///   %s."` (`task.c:208` - genuinely different wording from the online
///   branch's `"Set %s %s to %s."`, since `set_flags`'s task-queue
///   completion handler has no access to `cmd_flag`'s `fptr` name
///   lookup; preserved as-is, not "fixed").
pub(crate) async fn apply_admin_flag_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let toggles = world.drain_pending_admin_flag_toggles();
    if toggles.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for toggle in toggles {
        let Ok(Some(summary)) = repository.find_login_target(&toggle.target_name).await else {
            world.queue_system_text(
                toggle.caller_id,
                format!("Sorry, no player by the name {}.", toggle.target_name),
            );
            continue;
        };
        world.queue_system_text(toggle.caller_id, "Update scheduled.".to_string());

        let Ok(Some(snapshot)) = repository.load_character_snapshot(summary.id).await else {
            continue;
        };
        // C `set_task`'s "online somewhere else" guard (`task.c:250-253`):
        // silent no-op (only an `xlog`, no player-facing message).
        if snapshot.current_area != 0 {
            continue;
        }

        let mut character = snapshot.character;
        character.flags.toggle(toggle.flag);
        let state = if character.flags.contains(toggle.flag) {
            "on"
        } else {
            "off"
        };
        let target_display_name = character.name.clone();

        let request = ugaris_db::CharacterSaveRequest {
            character,
            items: snapshot.items,
            ppd_blob: snapshot.ppd_blob,
            subscriber_blob: snapshot.subscriber_blob,
            mode: ugaris_db::CharacterSaveMode::Backup {
                expected_current_area: snapshot.current_area,
                expected_current_mirror: snapshot.current_mirror,
                mirror: snapshot.mirror,
            },
        };
        if !matches!(repository.save_character_snapshot(request).await, Ok(true)) {
            continue;
        }

        world.queue_system_text(
            toggle.caller_id,
            format!("Set flag on {target_display_name} to {state}."),
        );
        applied += 1;
    }
    applied
}

/// `/punish <name> <level> <reason>`'s async DB round trip (C
/// `task_punish_player`/`punish_player`/`punish`, `src/system/task.c:
/// 171-188,213-295,358-373` + `src/system/punish.c:41-107`): resolves
/// every `World::drain_pending_punish_requests` entry (queued by
/// `World::queue_punish_command` - see `world/punish.rs`'s module doc
/// comment) the same "online (any loaded character) first, else read/
/// mutate/write the persisted row, else silently no-op if logged in
/// elsewhere" way `apply_admin_flag_events` already established, with
/// [`apply_punishment`] providing the shared karma/exp mutation for both
/// branches.
///
/// - no DB row at all -> "Sorry, no player by the name %s." (C's
///   synchronous `lookup_name == -1` case).
/// - online target -> mutated immediately in `World::characters`; if the
///   result triggers a lock or kick (`PunishmentOutcome::lock`/`kick`)
///   and the target has a live session, sends the exit message and
///   requests a disconnect - this funnels through the exact same
///   `SessionEvent::Disconnected` -> `enter_lostcon_on_disconnect`
///   machinery a real network drop uses, matching C `kick_player`
///   (`player.c:174-202`) far more closely than a `/kick`-style full
///   `exit_char` teardown would (see `world/punish.rs`'s module doc
///   comment).
/// - offline target already logged in elsewhere (`current_area != 0`) ->
///   silent no-op (C `set_task`'s "online somewhere else" guard,
///   `task.c:238-243`, only `xlog`s).
/// - offline target -> loaded, mutated, and saved back
///   (`CharacterSaveMode::Backup`, pinning the expected offline
///   `current_area`/`current_mirror` like every other offline-DB-
///   mutation event in this file); a lock/kick outcome only updates the
///   persisted `locked` column here (there is no live session to
///   disconnect).
///
/// Both branches write the `kind = 1` punishment `notes` row (best
/// effort - a write failure does not roll back the mutation or suppress
/// the player-facing messages, see the module doc comment in
/// `world/punish.rs` for why) and message the caller with "Punished %s
/// with a level %d punishment for %s"; an online target additionally
/// gets the level-specific warning/punishment text (C `punish_player`,
/// `task.c:171-188`) - an offline target has no live session to deliver
/// that second message to, so it is silently skipped (matching every
/// other offline-mutation event's caller-only feedback in this file,
/// e.g. `apply_rename_events`).
///
/// No-ops entirely (silent, but still drains the queue) when no
/// `character_repository` is configured, matching every sibling
/// offline-DB-mutation event in this file.
pub(crate) async fn apply_punish_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    now_unix: i64,
) -> usize {
    let requests = world.drain_pending_punish_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        if let Some(target_id) = world.find_punish_target_online(&request.target_name) {
            let Some(character) = world.characters.get_mut(&target_id) else {
                continue;
            };
            let outcome = apply_punishment(character, request.level);
            let target_name = character.name.clone();
            let paid = character.flags.contains(CharacterFlags::PAID);
            let karma_after = character.karma;

            if let Some(notes_repository) = notes_repository {
                let note = PunishmentNote {
                    level: request.level as i32,
                    exp: outcome.exp_loss as i32,
                    karma: outcome.karma_loss,
                    reason: request.reason.clone(),
                };
                let _ = notes_repository
                    .add_note(
                        target_id,
                        PUNISHMENT_NOTE_KIND,
                        request.caller_id,
                        &encode_punishment_note(&note),
                        now_unix,
                    )
                    .await;
            }

            world.queue_system_text(
                request.caller_id,
                format!(
                    "Punished {target_name} with a level {} punishment for {}",
                    request.level, request.reason
                ),
            );
            if request.level == 0 {
                world.queue_system_text(
                    target_id,
                    format!(
                        "You have been warned for {}. You will not be warned again. Next time you will lose experience and karma.",
                        request.reason
                    ),
                );
            } else {
                let threshold = if paid { -12 } else { -5 };
                world.queue_system_text(
                    target_id,
                    format!(
                        "You have just been punished for {}. You have lost experience and karma. Your karma is now down to {karma_after}. If your karma reaches {threshold}, you will be banned from this game.",
                        request.reason
                    ),
                );
            }

            if outcome.lock || outcome.kick {
                let _ = character_repository
                    .set_character_locked(target_id, true)
                    .await;
                let mut builder = PacketBuilder::new();
                builder.exit("You have been locked as a result of your punishment.");
                let payload = builder.into_payload();
                for (session_id, _) in runtime.sessions_for_character(target_id) {
                    runtime.send_to_session(session_id, payload.clone());
                    runtime.flush_session(session_id);
                    if let Some(commands) = runtime.sessions.get(&session_id) {
                        let _ = commands.try_send(SessionCommand::Disconnect);
                    }
                }
            }
            applied += 1;
            continue;
        }

        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(
                request.caller_id,
                format!("Sorry, no player by the name {}.", request.target_name),
            );
            continue;
        };
        let Ok(Some(snapshot)) = character_repository
            .load_character_snapshot(summary.id)
            .await
        else {
            continue;
        };
        // C `set_task`'s "online somewhere else" guard (`task.c:238-243`):
        // silent no-op (only an `xlog`, no player-facing message).
        if snapshot.current_area != 0 {
            continue;
        }

        let mut character = snapshot.character;
        let outcome = apply_punishment(&mut character, request.level);
        let target_name = character.name.clone();
        let target_id = character.id;

        let save_request = ugaris_db::CharacterSaveRequest {
            character,
            items: snapshot.items,
            ppd_blob: snapshot.ppd_blob,
            subscriber_blob: snapshot.subscriber_blob,
            mode: ugaris_db::CharacterSaveMode::Backup {
                expected_current_area: snapshot.current_area,
                expected_current_mirror: snapshot.current_mirror,
                mirror: snapshot.mirror,
            },
        };
        if !matches!(
            character_repository
                .save_character_snapshot(save_request)
                .await,
            Ok(true)
        ) {
            continue;
        }

        if let Some(notes_repository) = notes_repository {
            let note = PunishmentNote {
                level: request.level as i32,
                exp: outcome.exp_loss as i32,
                karma: outcome.karma_loss,
                reason: request.reason.clone(),
            };
            let _ = notes_repository
                .add_note(
                    target_id,
                    PUNISHMENT_NOTE_KIND,
                    request.caller_id,
                    &encode_punishment_note(&note),
                    now_unix,
                )
                .await;
        }
        if outcome.lock || outcome.kick {
            let _ = character_repository
                .set_character_locked(target_id, true)
                .await;
        }

        world.queue_system_text(
            request.caller_id,
            format!(
                "Punished {target_name} with a level {} punishment for {}",
                request.level, request.reason
            ),
        );
        applied += 1;
    }
    applied
}

/// `/unpunish <name> <note id>`'s async DB round trip (C
/// `task_unpunish_player`/`unpunish_player`/`unpunish`, `src/system/
/// task.c:171,190-193,213-295,374-382` + `src/system/punish.c:109-131`):
/// resolves every `World::drain_pending_unpunish_requests` entry (queued
/// by `World::queue_unpunish_command`) the same online-first/offline-
/// fallback way [`apply_punish_events`] does.
///
/// - no DB row at all -> "Sorry, no player by the name %s.".
/// - a row found -> "UnPunishment scheduled." (C's unconditional,
///   fire-and-forget acknowledgement, `command.c:2729`), then:
///   - no `notes` row exists for `note_id` (already unpunished, wrong
///     id, or a note against a *different* character - C's `db_unpunish`
///     has no `uID` scoping either, see `crates/ugaris-db/src/notes.rs`'s
///     module doc comment) -> no further mutation or message (C's
///     `unpunish()` returning `0` short-circuits `unpunish_player`'s own
///     "UnPunished %s ID %d." message too).
///   - a row exists -> refunds the exp/karma it recorded
///     ([`apply_unpunishment`]), unconditionally unlocks the account
///     (C `plock = -1`, `punish.c:127-129`), and messages the caller
///     "UnPunished %s ID %d." (no message to the target - a real
///     asymmetry with `/punish`, preserved as-is).
///
/// No-ops entirely (silent, but still drains the queue) when no
/// `character_repository` is configured.
pub(crate) async fn apply_unpunish_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
) -> usize {
    let requests = world.drain_pending_unpunish_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        if let Some(target_id) = world.find_punish_target_online(&request.target_name) {
            let Some(character) = world.characters.get_mut(&target_id) else {
                continue;
            };
            let target_name = character.name.clone();
            world.queue_system_text(request.caller_id, "UnPunishment scheduled.".to_string());

            let Some(notes_repository) = notes_repository else {
                continue;
            };
            let Ok(Some(content)) = notes_repository.take_note(request.note_id).await else {
                continue;
            };
            let Some(note) = decode_punishment_note(&content) else {
                continue;
            };
            let Some(character) = world.characters.get_mut(&target_id) else {
                continue;
            };
            apply_unpunishment(character, &note);
            let _ = character_repository
                .set_character_locked(target_id, false)
                .await;
            world.queue_system_text(
                request.caller_id,
                format!("UnPunished {target_name} ID {}.", request.note_id),
            );
            applied += 1;
            continue;
        }

        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(
                request.caller_id,
                format!("Sorry, no player by the name {}.", request.target_name),
            );
            continue;
        };
        world.queue_system_text(request.caller_id, "UnPunishment scheduled.".to_string());

        let Ok(Some(snapshot)) = character_repository
            .load_character_snapshot(summary.id)
            .await
        else {
            continue;
        };
        if snapshot.current_area != 0 {
            continue;
        }
        let Some(notes_repository) = notes_repository else {
            continue;
        };
        let Ok(Some(content)) = notes_repository.take_note(request.note_id).await else {
            continue;
        };
        let Some(note) = decode_punishment_note(&content) else {
            continue;
        };

        let mut character = snapshot.character;
        apply_unpunishment(&mut character, &note);
        let target_name = character.name.clone();
        let target_id = character.id;

        let save_request = ugaris_db::CharacterSaveRequest {
            character,
            items: snapshot.items,
            ppd_blob: snapshot.ppd_blob,
            subscriber_blob: snapshot.subscriber_blob,
            mode: ugaris_db::CharacterSaveMode::Backup {
                expected_current_area: snapshot.current_area,
                expected_current_mirror: snapshot.current_mirror,
                mirror: snapshot.mirror,
            },
        };
        if !matches!(
            character_repository
                .save_character_snapshot(save_request)
                .await,
            Ok(true)
        ) {
            continue;
        }
        let _ = character_repository
            .set_character_locked(target_id, false)
            .await;
        world.queue_system_text(
            request.caller_id,
            format!("UnPunished {target_name} ID {}.", request.note_id),
        );
        applied += 1;
    }
    applied
}

/// `/look <name>`'s async DB round trip (C `command.c:8990-9019`'s inline
/// handler + `read_notes`/`db_read_notes`/`list_punishment`,
/// `src/system/database/database_lookup.c:116-124` + `database_notes.c:
/// 164-215` + `src/system/punish.c:26-38`): resolves every `World::
/// drain_pending_look_requests` entry (queued by `World::
/// queue_look_command`) by name via `find_login_target` (C's synchronous
/// `lookup_name`), then lists every `kind = 1` note filed against the
/// resolved character, each row's creator name resolved via
/// `find_name_by_id` (C `lookup_ID`).
///
/// - no matching character -> "No character by the name %s." (folds C's
///   `ID == -1` case; C's `ID == 0` "lookup in progress" case has no
///   analogue here, see `World::queue_look_command`'s doc comment).
/// - a match -> "Looking up character: %s (ID: %d)" (C's own immediate
///   confirmation line, `command.c:9016`), then "Start of Notes:", one
///   `format_look_note_line` per `kind = 1` row (oldest first, matching
///   `NotesRepository::list_notes_for_character`'s `order by id`), then
///   "End of Notes" - every other note `kind` is silently skipped (C's
///   own `default: xlog(...)` branch never reaches the player either).
///
/// No-ops entirely (silent, but still drains the queue) when either
/// `character_repository` or `notes_repository` is unconfigured.
pub(crate) async fn apply_look_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
) -> usize {
    let requests = world.drain_pending_look_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let Some(notes_repository) = notes_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(
                request.requester_id,
                format!("No character by the name {}.", request.target_name),
            );
            continue;
        };
        world.queue_system_text(
            request.requester_id,
            format!(
                "Looking up character: {} (ID: {})",
                summary.name, summary.id.0
            ),
        );
        let Ok(notes) = notes_repository.list_notes_for_character(summary.id).await else {
            continue;
        };
        world.queue_system_text(request.requester_id, "Start of Notes:".to_string());
        for note in &notes {
            if note.kind != PUNISHMENT_NOTE_KIND {
                continue;
            }
            let Some(punishment) = decode_punishment_note(&note.content) else {
                continue;
            };
            let creator_name = match character_repository.find_name_by_id(note.creator_id).await {
                Ok(Some(name)) => name,
                _ => "*unknown*".to_string(),
            };
            world.queue_system_text(
                request.requester_id,
                format_look_note_line(note.id, &punishment, &creator_name, note.created_at),
            );
        }
        world.queue_system_text(request.requester_id, "End of Notes".to_string());
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_look_events`], split out so
/// the date arithmetic can be unit-tested without a live database. C
/// `list_punishment` (`src/system/punish.c:26-38`)'s `localtime` is
/// approximated in UTC, matching this codebase's established convention
/// (see `clan_log.rs`'s `format_clan_log_entries` doc comment) since no
/// `chrono`/timezone-database dependency exists in this workspace.
fn format_look_note_line(
    note_id: i64,
    note: &PunishmentNote,
    creator_name: &str,
    created_at: i64,
) -> String {
    let (year, month, day) = civil_from_unix_seconds(created_at.max(0) as u64);
    let seconds_of_day = created_at.max(0) as u64 % 86_400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!(
        "P{note_id}: Level: {}, Exp: {}, Karma: {}, Creator: {creator_name}, Date: {month:02}/{day:02}/{year:04} {hour:02}:{minute:02}:{second:02}, Reason: {}",
        note.level, note.exp, note.karma, note.reason
    )
}

/// `/klog`'s async DB round trip (C `command.c:9022-9024` -> `karmalog`
/// -> `db_karmalog`/`karmalog_s`, `src/system/database/database_notes.c:
/// 230-275`): resolves every `World::drain_pending_klog_requests` entry
/// (queued by `World::queue_klog_command`, which takes no argument -
/// unlike `/look`, there is nothing to validate before queuing) against
/// a single shared `NotesRepository::list_recent_notes` query (the last
/// 24 hours, matching C's `date >= now - 86400` cutoff), reused across
/// every requester in the drained batch rather than re-querying per
/// caller.
///
/// Replies "Karmalog:", one `format_klog_line` per `kind = 1` row (newest
/// first, matching `list_recent_notes`'s `order by date desc`), then
/// "---" (C's own trailing separator, `database_notes.c:273`) - every
/// other note `kind` is silently skipped, same as `/look`. A row whose
/// target or creator id no longer resolves to a name falls back to
/// `"*unknown*"`, matching C `lookup_ID`'s own `"*unknown*"` fallback for
/// a cache slot with no name recorded (this codebase has no analogue of
/// C's separate `"**deleted**"` case, since it has no in-memory
/// name/ID cache to distinguish "never resolved" from "resolved to a
/// numeric placeholder" - see `CharacterRepository::find_name_by_id`'s
/// doc comment).
///
/// No-ops entirely (silent, but still drains the queue) when either
/// `character_repository` or `notes_repository` is unconfigured.
pub(crate) async fn apply_klog_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    now_unix: i64,
) -> usize {
    let requesters = world.drain_pending_klog_requests();
    if requesters.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let Some(notes_repository) = notes_repository else {
        return 0;
    };
    let since_unix = now_unix - 60 * 60 * 24;
    let Ok(notes) = notes_repository.list_recent_notes(since_unix).await else {
        return 0;
    };
    let mut applied = 0;
    for requester_id in requesters {
        world.queue_system_text(requester_id, "Karmalog:".to_string());
        for note in &notes {
            if note.kind != PUNISHMENT_NOTE_KIND {
                continue;
            }
            let Some(target_id) = note.target_id else {
                continue;
            };
            let Some(punishment) = decode_punishment_note(&note.content) else {
                continue;
            };
            let offender_name = match character_repository.find_name_by_id(target_id).await {
                Ok(Some(name)) => name,
                _ => "*unknown*".to_string(),
            };
            let creator_name = match character_repository.find_name_by_id(note.creator_id).await {
                Ok(Some(name)) => name,
                _ => "*unknown*".to_string(),
            };
            world.queue_system_text(
                requester_id,
                format_klog_line(
                    &offender_name,
                    punishment.karma,
                    &creator_name,
                    &punishment.reason,
                    note.created_at,
                ),
            );
        }
        world.queue_system_text(requester_id, "---".to_string());
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_klog_events`] - see that
/// function's doc comment. C `karmalog_s` (`database_notes.c:227-244`)
/// prints only the time of day, not the full date (unlike
/// [`format_look_note_line`]'s sibling `list_punishment` format).
fn format_klog_line(
    offender_name: &str,
    karma: i32,
    creator_name: &str,
    reason: &str,
    created_at: i64,
) -> String {
    let seconds_of_day = created_at.max(0) as u64 % 86_400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!(
        "{offender_name}, {karma} Karma from {creator_name} for {reason} at {hour:02}:{minute:02}:{second:02}."
    )
}

/// `/showvalues <name>`'s async DB round trip (C `command.c:8401-8409` ->
/// `show_values`, `command.c:521-537` + its `server_chat` body
/// `show_values_bg`, `src/system/tool.c:2940-3096`): resolves every
/// `World::drain_pending_showvalues_requests` entry (queued by `World::
/// queue_showvalues_command`) by name via `find_login_target` (C's
/// synchronous `lookup_name`).
///
/// - no matching character -> "No player by that name." (C's `ID == -1`
///   branch; C's `ID == 0` "lookup in progress" case has no analogue
///   here, same as every sibling name-lookup command in this codebase).
/// - a match -> the caller always gets the "Sent." confirmation (C logs
///   this unconditionally once `lookup_name` succeeds, regardless of
///   which area server - if any - currently has the target loaded), then
///   the caller's own `show_values_lines` stat block is delivered to the
///   target *only if the target happens to be loaded in this process's
///   `World`* - C's real delivery goes through `tell_chat`'s own
///   cross-area chat relay, which this codebase does not have yet (see
///   the "Cross-area transfer" `PORTING_TODO.md` entry's gap (2) and
///   `world/values.rs`'s module doc comment for the full single-process
///   caveat).
///
/// No-ops entirely (silent, but still drains the queue) when
/// `character_repository` is unconfigured.
pub(crate) async fn apply_showvalues_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let requests = world.drain_pending_showvalues_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(request.caller_id, "No player by that name.".to_string());
            continue;
        };
        let Some(caller) = world.characters.get(&request.caller_id) else {
            continue;
        };
        let lines = show_values_lines(caller, &world.items);
        world.queue_system_text(request.caller_id, "Sent.".to_string());
        if world.characters.contains_key(&summary.id) {
            for line in lines {
                world.queue_system_text(summary.id, line);
            }
        }
        applied += 1;
    }
    applied
}

/// `/values <name>`'s async DB round trip (C `command.c:8391-8399` ->
/// `look_values`, `command.c:501-519` + its `server_chat` body
/// `look_values_bg`, `src/system/tool.c:2882-2939`): resolves every
/// `World::drain_pending_values_requests` entry (queued by `World::
/// queue_values_command`) by name via `find_login_target` (C's
/// synchronous `lookup_name`), same as `/showvalues` above.
///
/// Unlike `/showvalues`'s caller/target role swap, `/values` keeps the
/// caller as the caller throughout: every reply line goes back to
/// `request.caller_id`, showing the *resolved target's* stats (see
/// `world/values.rs`'s module doc comment for the contrast, and C's own
/// `tell_chat(0, cnID, 1, ...)` calls in `look_values_bg`, all addressed
/// to the caller `cnID`, never the target `coID`).
///
/// - no matching character -> "No player by that name." (C's `ID == -1`
///   branch).
/// - a match not currently loaded in this process's `World` -> silent
///   no-op (C's `if (!co) return;` in `look_values_bg` - no message at
///   all, matching this codebase's single-process-only cross-area chat
///   caveat, see `world/values.rs`'s module doc comment).
/// - a match with no resolvable `find_paid_until_info` row (a data
///   inconsistency - a live `World` character with no matching DB
///   `accounts` join - never hit for a real player) -> silent no-op,
///   same as the offline case.
/// - a loaded match -> the caller receives every `values_lines` line:
///   `PlayerRuntime::stats_online_time`/`bank_gold`/`current_mirror_id`
///   come from `ServerRuntime::player_for_character` when the target has
///   a live session (defaulting to `0`/`0`/this server's own `mirror_id`
///   when it does not - e.g. an offline-but-somehow-`World`-resident
///   NPC, never hit for a real logged-in player); the mirror-area
///   section name comes from `section_at(area_id, x, y)` (C's
///   `get_section_name`, implicitly scoped to this server process's own
///   `areaID`), falling back to `""` when no section matches (see
///   `values_lines`'s own doc comment for why).
///
/// No-ops entirely (silent, but still drains the queue) when
/// `character_repository` is unconfigured.
pub(crate) async fn apply_values_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_id: u16,
    mirror_id: u16,
    now_unix: i64,
) -> usize {
    let requests = world.drain_pending_values_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(request.caller_id, "No player by that name.".to_string());
            continue;
        };
        let Some(target) = world.characters.get(&summary.id).cloned() else {
            continue;
        };
        let Ok(Some(paid_info)) = character_repository.find_paid_until_info(summary.id).await
        else {
            continue;
        };
        let (paid_till, is_paid) = compute_paid_till(
            paid_info.raw_paid_until_unix,
            paid_info.account_created_at_unix,
            now_unix,
        );
        let (online_minutes, bank_gold, current_mirror) = runtime
            .player_for_character(summary.id)
            .map(|player| {
                (
                    player.stats_online_time(),
                    player.bank_gold,
                    player.current_mirror_id,
                )
            })
            .unwrap_or((0, 0, mirror_id));
        let section_name = section_at(area_id, usize::from(target.x), usize::from(target.y))
            .map(|section| section.name)
            .unwrap_or("");
        let lines = values_lines(
            &target,
            &world.items,
            is_paid,
            paid_till,
            now_unix,
            online_minutes,
            bank_gold,
            current_mirror,
            mirror_id,
            area_id,
            section_name,
        );
        for line in lines {
            world.queue_system_text(request.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// `/allow <name>`'s async DB round trip (C `command.c:8371-8378` ->
/// `allow_body`, `src/system/death.c:1013-1029` + its `server_chat` body
/// `allow_body_db`, `death.c:1045-1067`): resolves every `World::
/// drain_pending_allow_requests` entry (queued by `World::
/// queue_allow_command`) by name via `find_login_target` (C's
/// synchronous `lookup_name`), then grants the resolved target access to
/// every grave `World::grant_grave_access_to` finds owned by the caller
/// in this process's own `World` (see `world/allow.rs`'s module doc
/// comment for the single-process-only caveat shared with every other
/// name-lookup command here).
///
/// - no matching character -> "No player by that name." (C's `coID ==
///   -1` branch).
/// - a match -> C's `allow_body` unconditionally logs "Order
///   scheduled." once `lookup_name` resolves, then `allow_body_db`
///   (run per-area-server against the broadcast) replies "Area %d:
///   Allowed access to %d corpses." with its own local count - both
///   lines are sent here, in that order, once resolution completes
///   (this codebase collapses C's two-step broadcast-then-local-reply
///   into one async round trip, matching every other documented
///   cross-area gap).
///
/// No-ops entirely (silent, but still drains the queue) when
/// `character_repository` is unconfigured.
pub(crate) async fn apply_allow_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_id: u16,
) -> usize {
    let requests = world.drain_pending_allow_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(request.caller_id, "No player by that name.".to_string());
            continue;
        };
        world.queue_system_text(request.caller_id, "Order scheduled.".to_string());
        let count = world.grant_grave_access_to(request.caller_id, summary.id);
        world.queue_system_text(
            request.caller_id,
            format!("Area {area_id}: Allowed access to {count} corpses."),
        );
        applied += 1;
    }
    applied
}

/// `/rename <from> <to>`'s async DB round trip (C `do_rename`/
/// `db_rename`, `src/system/database/database_admin.c:291-355`):
/// resolves every `World::drain_pending_rename_lookups` entry (queued by
/// a validly-shaped `to` name - see `World::queue_rename_command`'s and
/// `world/rename.rs`'s module doc comment) against `PgCharacterRepository
/// ::rename_character`.
///
/// - a query error (including a unique-name-constraint violation on
///   `to`, which C's own query would likewise fail on if `chars.name` is
///   unique) -> "Failed to change name."
/// - no row matched `from` -> "Didn't work, most probable cause: %s not
///   found."
/// - success -> "Changed %s to %s. The change will be visible after the
///   next login."
///
/// No-ops entirely (silent, but still drains the queue) when no
/// `character_repository` is configured, matching every sibling
/// offline-DB-mutation event in this file.
pub(crate) async fn apply_rename_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_rename_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository
            .rename_character(&lookup.from_name, &lookup.to_name)
            .await
        {
            Ok(true) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Changed {} to {}. The change will be visible after the next login.",
                        lookup.from_name, lookup.to_name
                    ),
                );
            }
            Ok(false) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Didn't work, most probable cause: {} not found.",
                        lookup.from_name
                    ),
                );
            }
            Err(_) => {
                world.queue_system_text(lookup.requester_id, "Failed to change name.".to_string());
            }
        }
        applied += 1;
    }
    applied
}

/// `/lockname <name>`'s async DB round trip (C `do_lockname`/
/// `db_lockname`, `src/system/database/database_admin.c:357-398`):
/// resolves every `World::drain_pending_lockname_lookups` entry against
/// `PgCharacterRepository::lock_name` - see `world/lockname.rs`'s module
/// doc comment for the shared validation this queue entry already
/// passed.
///
/// - a query error -> "Failed to insert name."
/// - already locked (no new row inserted) -> "Didn't work, most probable
///   cause: %s already in bad name database."
/// - success -> "Added %s to bad name database."
///
/// Every message uses the *original* (un-lowercased) name, matching C's
/// own `name` parameter (not its `lowercase_name` scratch buffer). No-ops
/// entirely (silent, but still drains the queue) when no
/// `character_repository` is configured, matching every sibling
/// offline-DB-mutation event in this file.
pub(crate) async fn apply_lockname_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_lockname_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository.lock_name(&lookup.lookup_name).await {
            Ok(true) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!("Added {} to bad name database.", lookup.original_name),
                );
            }
            Ok(false) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Didn't work, most probable cause: {} already in bad name database.",
                        lookup.original_name
                    ),
                );
            }
            Err(_) => {
                world.queue_system_text(lookup.requester_id, "Failed to insert name.".to_string());
            }
        }
        applied += 1;
    }
    applied
}

/// `/unlockname <name>`'s async DB round trip (C `do_unlockname`/
/// `db_unlockname`, `src/system/database/database_admin.c:436-467`), the
/// mirror image of [`apply_lockname_events`].
///
/// - a query error -> "Failed to delete name."
/// - not locked (no row deleted) -> "Didn't work, most probable cause:
///   %s not in bad name database."
/// - success -> "Deleted %s from bad name database."
pub(crate) async fn apply_unlockname_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_unlockname_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository.unlock_name(&lookup.lookup_name).await {
            Ok(true) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!("Deleted {} from bad name database.", lookup.original_name),
                );
            }
            Ok(false) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Didn't work, most probable cause: {} not in bad name database.",
                        lookup.original_name
                    ),
                );
            }
            Err(_) => {
                world.queue_system_text(lookup.requester_id, "Failed to delete name.".to_string());
            }
        }
        applied += 1;
    }
    applied
}

/// `/exterminate <name>`'s async DB round trip (C `exterminate`/
/// `db_exterminate`, `src/system/database/database_admin.c:29-95,
/// 503-507`) - see `world/exterminate.rs`'s module doc comment for why
/// this is a direct account lock + IP ban rather than a `server_chat`
/// relay.
///
/// - target not found -> "Player '%s' not found." (C's exact text,
///   `database_admin.c:92`).
/// - query error -> "Failed to exterminate %s." (this codebase's own
///   error-path convention, matching `apply_lockname_events`/
///   `apply_rename_events` - C has no equivalent distinct message since
///   `db_exterminate` only ever `elog`s and returns on a query failure).
/// - success -> "Locked %d accounts and %d IP addresses." (C's exact
///   wording, `database_admin.c:83`, `nrc`/`nrb` renamed to this
///   codebase's `locked_accounts`/`banned_ips`).
pub(crate) async fn apply_exterminate_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let requests = world.drain_pending_exterminate_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        match repository.exterminate_account(&request.target_name).await {
            Ok(Some(outcome)) => {
                world.queue_system_text(
                    request.caller_id,
                    format!(
                        "Locked {} accounts and {} IP addresses.",
                        outcome.locked_accounts, outcome.banned_ips
                    ),
                );
            }
            Ok(None) => {
                world.queue_system_text(
                    request.caller_id,
                    format!("Player '{}' not found.", request.target_name),
                );
            }
            Err(_) => {
                world.queue_system_text(
                    request.caller_id,
                    format!("Failed to exterminate {}.", request.target_name),
                );
            }
        }
        applied += 1;
    }
    applied
}

#[cfg(test)]
mod lastseen_tests {
    use super::*;
    use ugaris_db::LastSeenInfo;

    #[test]
    fn god_characters_get_the_fixed_recently_message() {
        let info = LastSeenInfo {
            name: "Godmode".to_string(),
            is_god: true,
            last_activity_unix: 0,
        };
        assert_eq!(
            lastseen_reply_message(&info, 1_000_000),
            "Godmode was seen quite recently."
        );
    }

    #[test]
    fn elapsed_time_is_broken_into_days_hours_minutes() {
        let info = LastSeenInfo {
            name: "Player".to_string(),
            is_god: false,
            last_activity_unix: 0,
        };
        // 2 days, 3 hours, 4 minutes = 2*86400 + 3*3600 + 4*60 seconds.
        let now = 2 * 86_400 + 3 * 3_600 + 4 * 60;
        assert_eq!(
            lastseen_reply_message(&info, now),
            "Player was last seen 2 days, 3 hours, 4 minutes ago."
        );
    }

    #[test]
    fn recently_active_player_reports_zero_across_the_board() {
        let info = LastSeenInfo {
            name: "Player".to_string(),
            is_god: false,
            last_activity_unix: 500,
        };
        assert_eq!(
            lastseen_reply_message(&info, 500),
            "Player was last seen 0 days, 0 hours, 0 minutes ago."
        );
    }
}

#[cfg(test)]
mod ac_status_tests {
    use super::*;
    use ugaris_db::AntiCheatSessionInfo;

    fn info(status: i32, bot_score: f32) -> AntiCheatSessionInfo {
        AntiCheatSessionInfo {
            status,
            bot_score,
            heartbeat_violations: 1,
            state_violations: 2,
            challenge_failures: 3,
            timeout_count: 4,
            mod_major: None,
            mod_minor: None,
            mod_patch: None,
            os_type: None,
            screen_w: None,
            screen_h: None,
        }
    }

    #[test]
    fn without_fingerprint_shows_not_received() {
        let lines = ac_status_lines("Baddie", &info(2, 0.75));
        assert_eq!(
            lines,
            vec![
                "--- Anti-Cheat Status for Baddie ---".to_string(),
                "Status: suspicious".to_string(),
                "Heartbeat violations: 1".to_string(),
                "State violations: 2".to_string(),
                "Challenge failures: 3".to_string(),
                "Bot score: 0.75".to_string(),
                "Timeout count: 4".to_string(),
                "Fingerprint: not received".to_string(),
            ]
        );
    }

    #[test]
    fn with_fingerprint_shows_mod_version_os_and_screen() {
        let mut session_info = info(1, 0.0);
        session_info.mod_major = Some(1);
        session_info.mod_minor = Some(2);
        session_info.mod_patch = Some(3);
        session_info.os_type = Some(2);
        session_info.screen_w = Some(1920);
        session_info.screen_h = Some(1080);
        let lines = ac_status_lines("Godmode", &session_info);
        assert_eq!(lines[1], "Status: verified");
        assert_eq!(lines[7], "Mod version: 1.2.3");
        assert_eq!(lines[8], "OS: Linux");
        assert_eq!(lines[9], "Screen: 1920x1080");
    }

    #[test]
    fn unknown_os_type_falls_back_to_unknown() {
        let mut session_info = info(0, 0.0);
        session_info.mod_major = Some(0);
        session_info.mod_minor = Some(0);
        session_info.mod_patch = Some(0);
        session_info.os_type = Some(99);
        let lines = ac_status_lines("Player", &session_info);
        assert_eq!(lines[1], "Status: unverified");
        assert!(lines.contains(&"OS: Unknown".to_string()));
    }

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_status_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_status_lookup(CharacterId(7), "Baddie".to_string(), 42);

        let applied = apply_ac_status_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_status_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_stats_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_stats_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_stats_lookup(
            CharacterId(7),
            vec![AcOnlineTarget {
                name: "Baddie".to_string(),
                session_id: 42,
            }],
        );

        let applied = apply_ac_stats_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_stats_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_list_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_list_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_list_lookup(
            CharacterId(7),
            vec![AcOnlineTarget {
                name: "Baddie".to_string(),
                session_id: 42,
            }],
        );

        let applied = apply_ac_list_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_list_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_suspicious_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_suspicious_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_suspicious_lookup(
            CharacterId(7),
            vec![AcOnlineTarget {
                name: "Baddie".to_string(),
                session_id: 42,
            }],
        );

        let applied = apply_ac_suspicious_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_suspicious_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_cleanup_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_cleanup_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_cleanup_lookup(CharacterId(7), 30);

        let applied = apply_ac_cleanup_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_cleanup_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_reset_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_reset_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_reset_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_reset_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_reset_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_flag_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_flag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_flag_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_flag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_flag_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_unflag_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_unflag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_unflag_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_unflag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_unflag_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_trust_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_trust_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_trust_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_trust_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_trust_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_untrust_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_untrust_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_untrust_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_untrust_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_untrust_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_warn_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_warn_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_warn_lookup(
            CharacterId(7),
            CharacterId(8),
            "Baddie".to_string(),
            30,
            "Speedhacking".to_string(),
        );

        let applied = apply_ac_warn_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_warn_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_sessions_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sessions_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sessions_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_sessions_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sessions_lookups().is_empty());
    }

    #[test]
    fn ac_sessions_lines_reports_no_sessions_when_empty() {
        let lines = ac_sessions_lines("Baddie", &[]);
        assert_eq!(lines, vec!["No sessions found for Baddie.".to_string()]);
    }

    #[test]
    fn ac_sessions_lines_formats_header_and_rows() {
        let rows = vec![
            ugaris_db::AntiCheatSessionHistoryRow {
                start_time: "07-06 10:00".to_string(),
                duration_minutes: 15,
                status: 3,
                bot_score: 0.91,
                heartbeat_violations: 2,
                state_violations: 3,
                challenge_failures: 4,
                anomaly_count: 5,
            },
            ugaris_db::AntiCheatSessionHistoryRow {
                start_time: "07-05 09:00".to_string(),
                duration_minutes: 60,
                status: 1,
                bot_score: 0.0,
                heartbeat_violations: 0,
                state_violations: 0,
                challenge_failures: 0,
                anomaly_count: 0,
            },
        ];
        let lines = ac_sessions_lines("Baddie", &rows);
        assert_eq!(
            lines,
            vec![
                "--- Recent Sessions for Baddie ---".to_string(),
                "07-06 10:00 (15m) flagged Bot:0.91 V:2/3/4/5".to_string(),
                "07-05 09:00 (60m) verified Bot:0.00 V:0/0/0/0".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_violations_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_violations_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_violations_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_violations_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_violations_lookups().is_empty());
    }

    #[test]
    fn ac_violations_lines_reports_no_violations_when_empty() {
        let lines = ac_violations_lines("Baddie", &[]);
        assert_eq!(lines, vec!["No violations found for Baddie.".to_string()]);
    }

    #[test]
    fn ac_violations_lines_formats_header_and_rows() {
        let rows = vec![
            ugaris_db::AntiCheatViolationRow {
                detected_at: "07-06 10:00".to_string(),
                type_name: "teleport".to_string(),
                severity: 2,
                details: Some("impossible jump".to_string()),
            },
            ugaris_db::AntiCheatViolationRow {
                detected_at: "07-05 09:00".to_string(),
                type_name: "speedhack".to_string(),
                severity: 1,
                details: None,
            },
        ];
        let lines = ac_violations_lines("Baddie", &rows);
        assert_eq!(
            lines,
            vec![
                "--- Recent Violations for Baddie ---".to_string(),
                "07-06 10:00 [teleport] sev=2 impossible jump".to_string(),
                "07-05 09:00 [speedhack] sev=1 ".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_history_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_history_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_history_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_history_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_history_lookups().is_empty());
    }

    #[test]
    fn ac_history_lines_reports_no_history_when_row_missing() {
        let lines = ac_history_lines("Baddie", 42, None);
        assert_eq!(lines, vec!["No AC history found for Baddie.".to_string()]);
    }

    #[test]
    fn ac_history_lines_formats_every_field() {
        let stats = ugaris_db::AntiCheatPlayerStatsRow {
            total_sessions: 12,
            flagged_sessions: 2,
            suspicious_sessions: 3,
            total_heartbeat_violations: 4,
            total_state_violations: 5,
            total_challenge_failures: 6,
            total_anomalies: 7,
            max_session_bot_score: 0.91,
            avg_session_bot_score: 0.4,
            risk_level: "high".to_string(),
            is_flagged: true,
            is_trusted: false,
            warnings_issued: 3,
            first_seen: "01-01 00:00".to_string(),
            last_seen: Some("07-06 10:00".to_string()),
        };
        let lines = ac_history_lines("Baddie", 42, Some(&stats));
        assert_eq!(
            lines,
            vec![
                "--- AC History for Baddie (ID: 42) ---".to_string(),
                "Sessions: 12 total, 2 flagged, 3 suspicious".to_string(),
                "Violations: HB=4, State=5, Challenge=6, Anomalies=7".to_string(),
                "Bot Score: max=0.91, avg=0.40".to_string(),
                "Risk Level: high".to_string(),
                "Flagged: YES, Trusted: no, Warnings: 3".to_string(),
                "First seen: 01-01 00:00".to_string(),
                "Last seen: 07-06 10:00".to_string(),
            ]
        );
    }

    #[test]
    fn ac_history_lines_handles_a_missing_last_seen() {
        let stats = ugaris_db::AntiCheatPlayerStatsRow {
            total_sessions: 1,
            flagged_sessions: 0,
            suspicious_sessions: 0,
            total_heartbeat_violations: 0,
            total_state_violations: 0,
            total_challenge_failures: 0,
            total_anomalies: 0,
            max_session_bot_score: 0.0,
            avg_session_bot_score: 0.0,
            risk_level: "low".to_string(),
            is_flagged: false,
            is_trusted: false,
            warnings_issued: 0,
            first_seen: "01-01 00:00".to_string(),
            last_seen: None,
        };
        let lines = ac_history_lines("Newbie", 7, Some(&stats));
        assert_eq!(lines.last().unwrap(), "Last seen: ");
    }
}

#[cfg(test)]
mod ac_sharedip_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sharedip_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sharedip_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_sharedip_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sharedip_lookups().is_empty());
    }

    #[test]
    fn ac_sharedip_lines_reports_no_shared_ips_when_empty() {
        let lines = ac_sharedip_lines("Baddie", &[]);
        assert_eq!(lines, vec!["No shared IPs found for Baddie.".to_string()]);
    }

    #[test]
    fn ac_sharedip_lines_formats_header_rows_and_summary() {
        let rows = vec![
            ugaris_db::AntiCheatSharedIpRow {
                username: "altaccount".to_string(),
                ip_address: 0x7f00_0001u32 as i32, // 127.0.0.1
                session_count: 3,
                last_seen: "2026-07-06".to_string(),
            },
            ugaris_db::AntiCheatSharedIpRow {
                username: "another".to_string(),
                ip_address: 0xc0a8_0102u32 as i32, // 192.168.1.2
                session_count: 1,
                last_seen: "2026-07-01".to_string(),
            },
        ];
        let lines = ac_sharedip_lines("Baddie", &rows);
        assert_eq!(
            lines,
            vec![
                "--- Accounts Sharing IP with Baddie ---".to_string(),
                "altaccount - 127.0.0.1 (sessions: 3, last: 2026-07-06)".to_string(),
                "another - 192.168.1.2 (sessions: 1, last: 2026-07-01)".to_string(),
                "Found 2 accounts sharing IPs.".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_sharedhw_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sharedhw_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sharedhw_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_sharedhw_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sharedhw_lookups().is_empty());
    }

    #[test]
    fn ac_sharedhw_lines_reports_no_shared_hardware_when_empty() {
        let lines = ac_sharedhw_lines("Baddie", &[]);
        assert_eq!(
            lines,
            vec!["No shared hardware found for Baddie.".to_string()]
        );
    }

    #[test]
    fn ac_sharedhw_lines_formats_header_rows_and_summary() {
        let rows = vec![ugaris_db::AntiCheatSharedHwRow {
            username: "altaccount".to_string(),
            hardware_hash: 123456789,
            screen_w: Some(1920),
            screen_h: Some(1080),
            last_seen: "2026-07-06".to_string(),
        }];
        let lines = ac_sharedhw_lines("Baddie", &rows);
        assert_eq!(
            lines,
            vec![
                "--- Accounts Sharing Hardware with Baddie ---".to_string(),
                "altaccount - Hash: 123456789, Screen: 1920x1080 (last: 2026-07-06)".to_string(),
                "Found 1 accounts sharing hardware.".to_string(),
            ]
        );
    }

    #[test]
    fn ac_sharedhw_lines_defaults_missing_screen_dimensions_to_zero() {
        let rows = vec![ugaris_db::AntiCheatSharedHwRow {
            username: "altaccount".to_string(),
            hardware_hash: 42,
            screen_w: None,
            screen_h: None,
            last_seen: "2026-07-06".to_string(),
        }];
        let lines = ac_sharedhw_lines("Baddie", &rows);
        assert_eq!(
            lines[1],
            "altaccount - Hash: 42, Screen: 0x0 (last: 2026-07-06)"
        );
    }
}

#[cfg(test)]
mod ac_highrisk_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_highrisk_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_highrisk_lookup(CharacterId(7));

        let applied = apply_ac_highrisk_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_highrisk_lookups().is_empty());
    }

    #[test]
    fn ac_highrisk_lines_reports_no_players_when_empty() {
        let lines = ac_highrisk_lines(&[]);
        assert_eq!(lines, vec!["No high-risk players found.".to_string()]);
    }

    #[test]
    fn ac_highrisk_lines_formats_header_and_rows() {
        let rows = vec![
            ugaris_db::AntiCheatHighRiskRow {
                subscriber_id: 3,
                username: "cheater".to_string(),
                risk_level: "critical".to_string(),
                max_bot_score: 1.0,
                flagged_sessions: 4,
                last_seen: Some("07-06 10:00".to_string()),
            },
            ugaris_db::AntiCheatHighRiskRow {
                subscriber_id: 5,
                username: "suspect".to_string(),
                risk_level: "high".to_string(),
                max_bot_score: 0.85,
                flagged_sessions: 1,
                last_seen: None,
            },
        ];
        let lines = ac_highrisk_lines(&rows);
        assert_eq!(
            lines,
            vec![
                "--- High-Risk Players ---".to_string(),
                "[3] cheater - critical Bot:1.00 Flag:4 (seen: 07-06 10:00)".to_string(),
                "[5] suspect - high Bot:0.85 Flag:1 (seen: )".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_lookup_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_lookup_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_lookup_lookup(CharacterId(7), 99);

        let applied = apply_ac_lookup_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_lookup_lookups().is_empty());
    }

    #[test]
    fn ac_lookup_lines_reports_not_found_when_subscriber_missing() {
        let lines = ac_lookup_lines(99, None);
        assert_eq!(lines, vec!["Subscriber ID 99 not found.".to_string()]);
    }

    #[test]
    fn ac_lookup_lines_reports_no_ac_data_when_stats_missing() {
        let result = ugaris_db::AntiCheatSubscriberLookup {
            username: "newbie".to_string(),
            stats: None,
        };
        let lines = ac_lookup_lines(7, Some(&result));
        assert_eq!(
            lines,
            vec![
                "--- Subscriber 7 (newbie) ---".to_string(),
                "No AC data for this subscriber.".to_string(),
            ]
        );
    }

    #[test]
    fn ac_lookup_lines_formats_every_field_when_stats_present() {
        let stats = ugaris_db::AntiCheatPlayerStatsRow {
            total_sessions: 12,
            flagged_sessions: 2,
            suspicious_sessions: 3,
            total_heartbeat_violations: 4,
            total_state_violations: 5,
            total_challenge_failures: 6,
            total_anomalies: 7,
            max_session_bot_score: 0.91,
            avg_session_bot_score: 0.4,
            risk_level: "high".to_string(),
            is_flagged: true,
            is_trusted: false,
            warnings_issued: 3,
            first_seen: "01-01 00:00".to_string(),
            last_seen: Some("07-06 10:00".to_string()),
        };
        let result = ugaris_db::AntiCheatSubscriberLookup {
            username: "cheater".to_string(),
            stats: Some(stats),
        };
        let lines = ac_lookup_lines(3, Some(&result));
        assert_eq!(
            lines,
            vec![
                "--- Subscriber 3 (cheater) ---".to_string(),
                "Sessions: 12 total, 2 flagged".to_string(),
                "Max Bot Score: 0.91, Risk: high".to_string(),
                "Flagged: YES, Trusted: no".to_string(),
                "First: 01-01 00:00, Last: 07-06 10:00".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_siglist_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_siglist_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_siglist_lookup(CharacterId(7));

        let applied = apply_ac_siglist_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_siglist_lookups().is_empty());
    }

    #[test]
    fn ac_siglist_lines_reports_no_signatures_when_empty() {
        let lines = ac_siglist_lines(&[]);
        assert_eq!(lines, vec!["No signatures defined.".to_string()]);
    }

    #[test]
    fn ac_siglist_lines_formats_header_and_rows_including_the_double_space_quirk() {
        let rows = vec![
            ugaris_db::AntiCheatSignatureRow {
                id: 3,
                signature_type: "hardware_hash".to_string(),
                name: "Known Cheat Tool".to_string(),
                severity: 2,
                auto_flag: true,
                auto_ban: true,
                times_detected: 12,
                is_active: true,
            },
            ugaris_db::AntiCheatSignatureRow {
                id: 5,
                signature_type: "process_name".to_string(),
                name: "cheatengine.exe".to_string(),
                severity: 0,
                auto_flag: false,
                auto_ban: false,
                times_detected: 0,
                is_active: true,
            },
        ];
        let lines = ac_siglist_lines(&rows);
        assert_eq!(
            lines,
            vec![
                "--- Known Bad Signatures ---".to_string(),
                "[3] Known Cheat Tool (hardware_hash) Sev:2 Flag Ban  Det:12".to_string(),
                "[5] cheatengine.exe (process_name) Sev:0  Det:0".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_sigadd_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sigadd_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sigadd_lookup(
            CharacterId(7),
            "hardware_hash".to_string(),
            "deadbeef".to_string(),
            "Known Cheat Tool".to_string(),
            "TestGod".to_string(),
        );

        let applied = apply_ac_sigadd_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sigadd_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_sigdel_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sigdel_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sigdel_lookup(CharacterId(7), 42);

        let applied = apply_ac_sigdel_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sigdel_lookups().is_empty());
    }
}

#[cfg(test)]
mod querystats_tests {
    use super::*;

    #[test]
    fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_querystats_events(&mut world, &None);
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[test]
    fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_querystats_lookup(CharacterId(7));

        let applied = apply_querystats_events(&mut world, &None);
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_querystats_lookups().is_empty());
    }

    #[test]
    fn querystats_lines_reproduce_the_scoped_c_header_and_counters() {
        let stats = ugaris_db::CharacterQueryStats {
            save_char_cnt: 12,
            exit_char_cnt: 3,
            load_char_cnt: 7,
        };
        assert_eq!(
            querystats_lines(stats),
            vec![
                "Database Query Statistics:".to_string(),
                "Character operations:".to_string(),
                "Save chars: 12, Exit chars: 3, Load chars: 7".to_string(),
            ]
        );
    }

    #[test]
    fn querystats_lines_reports_zero_counters_faithfully() {
        let stats = ugaris_db::CharacterQueryStats::default();
        assert_eq!(
            querystats_lines(stats),
            vec![
                "Database Query Statistics:".to_string(),
                "Character operations:".to_string(),
                "Save chars: 0, Exit chars: 0, Load chars: 0".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod jail_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_jail_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        // Matches every other offline-DB-lookup event in this file: with
        // no `character_repository` configured, the queue is still
        // drained (so it doesn't grow unboundedly) but nothing is
        // resolved and no player-facing message is sent.
        let mut world = World::default();
        world.queue_jail_lookup(
            CharacterId(7),
            "Godmode",
            ugaris_core::world::JailAction::Jail,
        );

        let applied = apply_jail_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_jail_lookups().is_empty());
    }
}

#[cfg(test)]
mod jail_cross_area_transfer_tests {
    use super::*;

    #[tokio::test]
    async fn no_transfers_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied =
            apply_jail_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_pair_falls_back_to_the_shared_down_message() {
        // Mirrors `attempt_cross_area_transfer`'s own
        // `cross_area_transfer_stays_put_without_a_registered_repository_pair`
        // coverage (`tests/cross_area.rs`): without a live
        // `AreaRepository`/`CharacterRepository` pair, the shared helper
        // can't resolve the target, so the caller gets the legacy
        // "Nothing happens - target area server is down." text - the
        // exact fallback `World::apply_jail_action` used to send
        // eagerly before this hand-off was deferred.
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.area_id = 1; // current server is NOT the jail area
        world.settings.jail_x = 186;
        world.settings.jail_y = 234;
        world.settings.jail_area = 3;
        let login = LoginBlock {
            name: "Godmode".to_string(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        world.add_character(login_character(CharacterId(1), &login, 1, 10, 10));
        let mut target_login = login.clone();
        target_login.name = "Baddie".to_string();
        world.add_character(login_character(CharacterId(2), &target_login, 1, 50, 50));
        world.resolve_jail_lookup(
            CharacterId(1),
            "Baddie",
            ugaris_core::world::JailAction::Jail,
        );
        // The synchronous jail/unjail messages (`You have jailed
        // .../You have been jailed by ...`) are not this hand-off's
        // concern - drain them so only the transfer's own feedback
        // remains below.
        world.drain_pending_system_texts();

        let applied =
            apply_jail_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 1);
        let texts = world.drain_pending_system_texts();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].character_id, CharacterId(1));
        assert_eq!(
            texts[0].message,
            "Nothing happens - target area server is down."
        );
        assert!(world.drain_pending_jail_cross_area_transfers().is_empty());
    }
}

#[cfg(test)]
mod dungeon_eviction_transfer_tests {
    use super::*;

    #[tokio::test]
    async fn no_transfers_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied =
            apply_dungeon_eviction_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_pair_falls_back_to_removing_the_character() {
        // Mirrors `attempt_cross_area_transfer`'s own
        // `cross_area_transfer_stays_put_without_a_registered_repository_pair`
        // coverage (`tests/cross_area.rs`): without a live
        // `AreaRepository`/`CharacterRepository` pair, the shared helper
        // can't resolve the target, so - unlike every other cross-area
        // call site, which sends "Nothing happens - target area server
        // is down." - this one mirrors C's `exit_char(cn)` fallback and
        // removes the character outright instead (see
        // `world/dungeon_master.rs`'s module doc comment).
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.area_id = 13;
        let login = LoginBlock {
            name: "Raider".to_string(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        let mut raider = login_character(CharacterId(1), &login, 13, 10, 10);
        raider.rest_area = 3; // a different area - queues a cross-area transfer
        raider.rest_x = 50;
        raider.rest_y = 60;
        assert!(world.spawn_character(raider, 10, 10));
        for (x, y) in [(245, 250), (240, 250), (235, 250), (230, 250)] {
            for dx in -1..=1_i32 {
                for dy in -1..=1_i32 {
                    let tx = (x as i32 + dx) as usize;
                    let ty = (y as i32 + dy) as usize;
                    world.map.tile_mut(tx, ty).unwrap().flags |=
                        ugaris_core::map::MapFlags::MOVEBLOCK;
                }
            }
        }
        world.build_remove_tile(10, 10);
        world.drain_pending_system_texts();

        let applied =
            apply_dungeon_eviction_transfers(&mut world, &mut runtime, &None, &None, 13, 0).await;
        assert_eq!(applied, 1);
        assert!(!world.characters.contains_key(&CharacterId(1)));
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_dungeon_eviction_transfers().is_empty());
    }
}

#[cfg(test)]
mod macro_cross_area_transfer_tests {
    use super::*;

    #[tokio::test]
    async fn no_transfers_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied =
            apply_macro_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_pair_leaves_the_character_in_place_with_no_message() {
        // Mirrors `attempt_cross_area_transfer`'s own
        // `cross_area_transfer_stays_put_without_a_registered_repository_pair`
        // coverage (`tests/cross_area.rs`): without a live
        // `AreaRepository`/`CharacterRepository` pair, the shared helper
        // can't resolve the target and never despawns the character - C
        // never checks `change_area`'s return value at either macro-
        // daemon call site either, so this hand-off has no "target area
        // server is down" message to send and no fallback action beyond
        // leaving the character exactly where it already was.
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.area_id = 1;
        let login = LoginBlock {
            name: "Victim".to_string(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        assert!(world.spawn_character(login_character(CharacterId(1), &login, 1, 10, 10), 10, 10));
        world.queue_macro_cross_area_transfer(CharacterId(1), 3, 178, 248);

        let applied =
            apply_macro_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 1);
        assert!(world.characters.contains_key(&CharacterId(1)));
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_macro_cross_area_transfers().is_empty());
    }
}

#[cfg(test)]
mod rmdeath_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_rmdeath_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        // Matches every other offline-DB-lookup event in this file: with
        // no `character_repository` configured, the queue is still
        // drained (so it doesn't grow unboundedly) but nothing is
        // resolved and no player-facing message is sent.
        let mut world = World::default();
        world.queue_rmdeath_lookup(CharacterId(7), "Godmode");

        let applied = apply_rmdeath_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_rmdeath_lookups().is_empty());
    }
}

#[cfg(test)]
mod complain_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied = apply_complain_events(&mut world, &mut runtime, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        // Matches every other offline-DB-lookup event in this file: with
        // no `character_repository` configured, the queue is still
        // drained (so it doesn't grow unboundedly) but nothing is
        // resolved and no player-facing message is sent.
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.queue_complain_lookup(CharacterId(7), "Godmode");

        let applied = apply_complain_events(&mut world, &mut runtime, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_complain_lookups().is_empty());
    }
}

#[cfg(test)]
mod admin_flag_tests {
    use super::*;

    #[tokio::test]
    async fn no_toggles_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_admin_flag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_toggle_queued_state_untouched_but_drained() {
        // Matches every other offline-DB-lookup event in this file: with
        // no `character_repository` configured, the queue is still
        // drained (so it doesn't grow unboundedly) but nothing is
        // resolved and no player-facing message is sent.
        let mut world = World::default();
        let messages =
            world.apply_cmd_flag_command(CharacterId(1), "Nobodyhome", CharacterFlags::GOD, "god");
        assert!(messages.is_empty());

        let applied = apply_admin_flag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_admin_flag_toggles().is_empty());
    }
}

#[cfg(test)]
mod rename_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_rename_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_rename_command(CharacterId(1), "Baddie", "Newname");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_rename_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_rename_lookups().is_empty());
    }
}

#[cfg(test)]
mod lockname_tests {
    use super::*;

    #[tokio::test]
    async fn no_lockname_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_lockname_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_lockname_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_lockname_command(CharacterId(1), "BadName");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_lockname_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_lockname_lookups().is_empty());
    }

    #[tokio::test]
    async fn no_unlockname_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_unlockname_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_unlockname_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_unlockname_command(CharacterId(1), "BadName");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_unlockname_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_unlockname_lookups().is_empty());
    }
}

#[cfg(test)]
mod exterminate_tests {
    use super::*;

    #[tokio::test]
    async fn no_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_exterminate_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_exterminate_command(CharacterId(1), "Baddie");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_exterminate_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_exterminate_requests().is_empty());
    }
}

#[cfg(test)]
mod punish_tests {
    use super::*;

    #[tokio::test]
    async fn no_punish_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied = apply_punish_events(&mut world, &mut runtime, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_punish_queue_without_a_reply() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.queue_punish_command(CharacterId(1), "Baddie", 3, "being quite mean", false);
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_punish_events(&mut world, &mut runtime, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_punish_requests().is_empty());
    }

    #[tokio::test]
    async fn no_unpunish_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_unpunish_events(&mut world, &None, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_unpunish_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_unpunish_command(CharacterId(1), "Baddie", 42);
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_unpunish_events(&mut world, &None, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_unpunish_requests().is_empty());
    }
}

#[cfg(test)]
mod look_tests {
    use super::*;

    #[test]
    fn format_look_note_line_matches_c_list_punishment_shape() {
        let note = PunishmentNote {
            level: 3,
            exp: 400,
            karma: 4,
            reason: "being mean".to_string(),
        };
        // 1_000_000_000 unix seconds = 2001-09-09 01:46:40 UTC.
        let line = format_look_note_line(7, &note, "Godmode", 1_000_000_000);
        assert_eq!(
            line,
            "P7: Level: 3, Exp: 400, Karma: 4, Creator: Godmode, Date: 09/09/2001 01:46:40, Reason: being mean"
        );
    }

    #[tokio::test]
    async fn no_look_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_look_events(&mut world, &None, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_look_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_look_command(CharacterId(1), "Baddie");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_look_events(&mut world, &None, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_look_requests().is_empty());
    }
}

#[cfg(test)]
mod klog_tests {
    use super::*;

    #[test]
    fn format_klog_line_matches_c_karmalog_s_shape_time_only_no_date() {
        // 1_000_000_000 unix seconds = 2001-09-09 01:46:40 UTC.
        let line = format_klog_line("Baddie", -4, "Godmode", "being mean", 1_000_000_000);
        assert_eq!(
            line,
            "Baddie, -4 Karma from Godmode for being mean at 01:46:40."
        );
    }

    #[tokio::test]
    async fn no_klog_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_klog_events(&mut world, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_klog_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_klog_command(CharacterId(1));
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_klog_events(&mut world, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_klog_requests().is_empty());
    }
}

#[cfg(test)]
mod showvalues_tests {
    use super::*;

    #[tokio::test]
    async fn no_showvalues_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_showvalues_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_showvalues_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_showvalues_command(CharacterId(1), "Someone");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_showvalues_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_showvalues_requests().is_empty());
    }
}

#[cfg(test)]
mod values_tests {
    use super::*;

    #[tokio::test]
    async fn no_values_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied = apply_values_events(&mut world, &mut runtime, &None, 1, 1, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_values_queue_without_a_reply() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.queue_values_command(CharacterId(1), "Someone");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_values_events(&mut world, &mut runtime, &None, 1, 1, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_values_requests().is_empty());
    }
}

#[cfg(test)]
mod allow_tests {
    use super::*;

    #[tokio::test]
    async fn no_allow_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_allow_events(&mut world, &None, 1).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_allow_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_allow_command(CharacterId(1), "Someone");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_allow_events(&mut world, &None, 1).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_allow_requests().is_empty());
    }
}
