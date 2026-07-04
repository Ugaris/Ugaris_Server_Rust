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
        }
    }
    applied
}
