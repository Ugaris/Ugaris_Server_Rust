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
