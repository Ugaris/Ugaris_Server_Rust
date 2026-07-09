//! Character death, dead bodies, loot containers, and NPC respawn.
//!
//! Ports `kill_char`, `die_char`, `respawn_callback`, and `drop_grave` from
//! `src/system/death.c` plus the kill-score tables. Bodies are ordinary map
//! items whose loot lives in items with `contained_in == Some(body_id)`, so
//! the existing container view protocol can open them.

use super::*;

pub(crate) const NPC_RESPAWN_TIMER: &str = "npc_respawn";
pub(crate) const EXPIRE_ITEM_TIMER: &str = "expire_item";

/// C `die_char` death animation length in ticks (`ch[cn].duration = 12`).
pub const DEATH_ANIMATION_TICKS: i32 = 12;

/// One registered NPC respawn point, mirroring the C respawn registry keyed
/// by `(tmp, tmpx, tmpy)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcRespawnSlot {
    pub template_key: String,
    pub x: u16,
    pub y: u16,
}

/// A due respawn the server runtime should instantiate from templates,
/// mirroring C `respawn_callback` template creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcRespawnRequest {
    pub slot: usize,
    pub template_key: String,
    pub x: u16,
    pub y: u16,
}

/// Kill experience award queued for the server give_exp path, mirroring the
/// C `kill_char` `give_exp(co, val)` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillExpAward {
    pub killer_id: CharacterId,
    pub exp: u32,
}

/// Kill achievement award queued for the server achievement path, mirroring
/// C `kill_char`'s `if (ch[co].flags & CF_PLAYER) { achievement_add_enemy_
/// killed(co); if (ch[cn].flags & CF_DEMON) achievement_add_demons(co,
/// areaID, 1); }` (`death.c:417-422`). Unlike [`KillExpAward`], this fires
/// for *any* kill by a player character - including killing other players -
/// since C's condition only gates on the killer, not the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillAchievementAward {
    pub killer_id: CharacterId,
    pub area_id: i32,
    pub target_is_demon: bool,
}

/// Queued `give_first_kill(cn, co)` check (`death.c:196-254`), fired when a
/// player kills an NPC whose `ch.class` is set (`1..=1023`). Server-side
/// (which owns `PlayerRuntime::first_kill_ppd`) drains this, bit-tests/sets
/// the class via `PlayerRuntime::mark_first_kill`, and on a genuine first
/// kill grants the `kill_score * 5` exp bonus, sends the matching congrats
/// text, and checks the Slayer of Demon Lords achievement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstKillCheck {
    pub killer_id: CharacterId,
    pub victim_class: i32,
    pub victim_level: u32,
    pub victim_has_name: bool,
    pub victim_name: String,
}

/// Queued `check_military_solve(cn, co)` check (`death.c:290-383`), fired
/// for every kill by a player character (no class-range restriction,
/// unlike [`FirstKillCheck`] - C's own guard is only `CF_PLAYER` on the
/// killer). Server-side (which owns `PlayerRuntime::military_ppd`) drains
/// this and applies `PlayerRuntime::check_military_solve`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MilitaryMissionKillCheck {
    pub killer_id: CharacterId,
    pub victim_class: i32,
    pub victim_level: u32,
}

/// Queued `apply_death_loot_for_template(ct, co, tmp)` call
/// (`src/system/create.c:1569-1572`, invoked from `death.c:741` right
/// after the natural inventory transfer into the corpse container),
/// fired for every non-player death that got a lootable body/container.
/// Server-side (which owns the parsed `LootRegistry` behind
/// [`World::loot_apply_to_container`] and the killer's `PlayerRuntime`
/// quest log for condition evaluation) drains this and rolls the dead
/// character template's `loot_table_death` (if any) into the container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDeathLootRoll {
    pub container_id: ItemId,
    pub killer_id: Option<CharacterId>,
    pub template_key: String,
}

impl World {
    /// C `kill_char(cn, co)` follow-up run once `hurt` marked the character
    /// dead: death-driver dispatch and NT_DEAD fan-out already happened in
    /// the hurt path, so this ports the respawn registration, killer
    /// experience, and the timed `AC_DIE` action.
    pub(crate) fn kill_character_followup(
        &mut self,
        target_id: CharacterId,
        cause_id: Option<CharacterId>,
    ) {
        let Some(target) = self.characters.get(&target_id) else {
            return;
        };
        let target_level = target.level;
        let target_is_player = target.flags.contains(CharacterFlags::PLAYER);
        let target_is_demon = target.flags.contains(CharacterFlags::DEMON);
        let target_class = target.class;
        let target_has_name = target.flags.contains(CharacterFlags::HASNAME);
        let target_name = target.name.clone();
        let target_driver = target.driver;

        // C: if (ch[cn].flags & CF_RESPAWN) set_timer(ticker + ch[cn].respawn, respawn_callback, ...)
        if target.flags.contains(CharacterFlags::RESPAWN) && !target.template_key.is_empty() {
            let template_key = target.template_key.clone();
            let (x, y) = if target.rest_x != 0 {
                (target.rest_x, target.rest_y)
            } else {
                (target.x, target.y)
            };
            let delay = target.respawn_ticks.max(1) as u64;
            self.schedule_npc_respawn(&template_key, x, y, delay);
        }

        // C `ch_died_driver`'s `CDR_PENTER` case (`pents.c:1889-1893`,
        // `handle_demon_death`): dispatched unconditionally at every death
        // (unlike `apply_character_death_driver`, only called below when
        // `cause_id` is `Some`), since C's power-level reduction runs
        // regardless of whether a killer exists.
        if target_driver == crate::character_driver::CDR_PENTER {
            self.apply_penter_demon_death(target_id, cause_id);
        }

        // C `ch_died_driver`'s `CDR_PALACEISLENA` case (`palace.c:824-826`,
        // `islena_dead`): `if (!co) return;` - only runs when there's a
        // killer, same guard shape as the `CDR_PENTER` case above.
        if target_driver == crate::character_driver::CDR_PALACEISLENA {
            if let Some(killer_id) = cause_id {
                self.apply_islena_death(target_id, killer_id);
            }
        }

        // C: killer experience via kill_score with hardcore/lag caps.
        let killer = cause_id.and_then(|id| self.characters.get(&id));
        if let (Some(cause_id), Some(killer)) = (cause_id, killer) {
            if !target_is_player {
                let mut exp =
                    i64::from(crate::attack::kill_score_level(target_level, killer.level));
                if killer.flags.contains(CharacterFlags::HARDCORE) {
                    exp = (exp as f64 * self.settings.hardcore_kill_exp_bonus) as i64;
                }
                if killer.flags.contains(CharacterFlags::LAG) {
                    exp = exp.min(1);
                }
                if exp > 0 {
                    self.pending_kill_exp.push(KillExpAward {
                        killer_id: cause_id,
                        exp: exp as u32,
                    });
                }
            }
        }

        let killer_is_player = cause_id
            .and_then(|id| self.characters.get(&id))
            .is_some_and(|killer| killer.flags.contains(CharacterFlags::PLAYER));

        // C: `if (ch[co].flags & CF_PLAYER) { achievement_add_enemy_killed(co);
        // if (ch[cn].flags & CF_DEMON) achievement_add_demons(co, areaID, 1); }`
        // (`death.c:417-422`) - fires for any kill by a player, independent of
        // whether the target was a player (unlike the `give_exp` branch above).
        if let Some(cause_id) = cause_id {
            if killer_is_player {
                self.pending_kill_achievements.push(KillAchievementAward {
                    killer_id: cause_id,
                    area_id: i32::from(self.area_id),
                    target_is_demon,
                });

                // C `give_first_kill(co, cn)` guard (`death.c:196-203`):
                // `if (!(ch[cn].flags & CF_PLAYER)) return;` (already
                // ensured by `killer_is_player`) and `if (ch[co].class < 1
                // || ch[co].class > 1023) return;`.
                if (1..=1023).contains(&target_class) {
                    self.pending_first_kill_checks.push(FirstKillCheck {
                        killer_id: cause_id,
                        victim_class: target_class,
                        victim_level: target_level,
                        victim_has_name: target_has_name,
                        victim_name: target_name,
                    });
                }

                // C `check_military_solve(co, cn)` (`death.c:416`): runs
                // unconditionally alongside `give_first_kill` in the same
                // `if (co && ch[cn].flags)` block - no class-range gate of
                // its own (`check_military_solve`'s only guard is
                // `CF_PLAYER` on the killer, already ensured above).
                self.pending_military_mission_checks
                    .push(MilitaryMissionKillCheck {
                        killer_id: cause_id,
                        victim_class: target_class,
                        victim_level: target_level,
                    });
            }
        }

        if let Some(target) = self.characters.get_mut(&target_id) {
            // C: ch[cn].action = AC_DIE; act1 = killer; act2 = ispk;
            //    duration = 12; step = 0;
            target.action = action::DIE;
            target.act1 = cause_id.map(|id| id.0 as i32).unwrap_or_default();
            target.act2 = i32::from(killer_is_player);
            target.duration = DEATH_ANIMATION_TICKS;
            target.step = 0;
        }
    }

    /// Register or reuse a respawn slot and schedule the respawn timer.
    pub fn schedule_npc_respawn(&mut self, template_key: &str, x: u16, y: u16, delay_ticks: u64) {
        let slot = self
            .npc_respawn_slots
            .iter()
            .position(|slot| slot.template_key == template_key && slot.x == x && slot.y == y)
            .unwrap_or_else(|| {
                self.npc_respawn_slots.push(NpcRespawnSlot {
                    template_key: template_key.to_string(),
                    x,
                    y,
                });
                self.npc_respawn_slots.len() - 1
            });
        self.timers.set_timer(
            self.tick.0 + delay_ticks,
            NPC_RESPAWN_TIMER,
            TimerPayload([slot as i32, 0, 0, 0, 0]),
        );
    }

    /// Retry a blocked respawn slot, mirroring the C ten-second retry.
    pub fn schedule_npc_respawn_retry(&mut self, slot: usize) {
        if slot >= self.npc_respawn_slots.len() {
            return;
        }
        self.timers.set_timer(
            self.tick.0 + TICKS_PER_SECOND * 10,
            NPC_RESPAWN_TIMER,
            TimerPayload([slot as i32, 0, 0, 0, 0]),
        );
    }

    pub(crate) fn queue_npc_respawn_from_timer(&mut self, slot: i32) {
        if slot < 0 {
            return;
        }
        let slot = slot as usize;
        let Some(entry) = self.npc_respawn_slots.get(slot) else {
            return;
        };
        self.pending_npc_respawns.push(NpcRespawnRequest {
            slot,
            template_key: entry.template_key.clone(),
            x: entry.x,
            y: entry.y,
        });
    }

    pub fn drain_pending_npc_respawns(&mut self) -> Vec<NpcRespawnRequest> {
        std::mem::take(&mut self.pending_npc_respawns)
    }

    pub fn drain_pending_kill_exp(&mut self) -> Vec<KillExpAward> {
        std::mem::take(&mut self.pending_kill_exp)
    }

    /// Drain queued kill achievement awards for the server achievement path.
    pub fn drain_pending_kill_achievements(&mut self) -> Vec<KillAchievementAward> {
        std::mem::take(&mut self.pending_kill_achievements)
    }

    /// Drain queued `give_first_kill` checks for the server achievement path.
    pub fn drain_pending_first_kill_checks(&mut self) -> Vec<FirstKillCheck> {
        std::mem::take(&mut self.pending_first_kill_checks)
    }

    /// Drain queued `check_military_solve` checks for the server mission
    /// progress path.
    pub fn drain_pending_military_mission_checks(&mut self) -> Vec<MilitaryMissionKillCheck> {
        std::mem::take(&mut self.pending_military_mission_checks)
    }

    /// Drain queued `apply_death_loot_for_template` calls for the server
    /// loot-table roll path.
    pub fn drain_pending_death_loot_rolls(&mut self) -> Vec<PendingDeathLootRoll> {
        std::mem::take(&mut self.pending_death_loot_rolls)
    }

    /// Schedule item destruction, mirroring C `set_expire` for bodies.
    pub fn set_item_expire(&mut self, item_id: ItemId, delay_ticks: u64) {
        self.timers.set_timer(
            self.tick.0 + delay_ticks.max(1),
            EXPIRE_ITEM_TIMER,
            TimerPayload([item_id.0 as i32, 0, 0, 0, 0]),
        );
    }

    pub(crate) fn expire_item_from_timer(&mut self, item_id: i32) {
        if item_id <= 0 {
            return;
        }
        let item_id = ItemId(item_id as u32);
        // Destroy contained loot first so no orphans stay behind.
        let contained: Vec<ItemId> = self
            .items
            .values()
            .filter(|item| item.contained_in == Some(item_id))
            .map(|item| item.id)
            .collect();
        for id in contained {
            self.destroy_item(id);
        }
        self.destroy_item(item_id);
    }

    /// C `god_save_char(cn)` (`src/system/death.c`): divine intervention
    /// that rescues a player from a fatal blow. Called from
    /// `World::apply_legacy_hurt` at the exact point C calls it - inside
    /// `hurt()`, before `kill_char`/the death animation ever starts - when
    /// the dying character is a player with `saves > 0` and the death is
    /// not a PK kill (C: `cc && CF_PLAYER(cn) && CF_PLAYER(cc)` is checked
    /// first and takes priority over the save). The normal `die_character`
    /// body/item/exp-loss sequence never runs for a god-saved character.
    pub(crate) fn god_save_character(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return;
        };
        let x = character.x;
        let y = character.y;
        // C: ch[cn].saves--; if (ch[cn].saves > 10) ch[cn].saves = 10;
        character.saves = character.saves.saturating_sub(1);
        if character.saves > 10 {
            character.saves = 10;
        }
        // C: ch[cn].got_saved++;
        character.got_saved = character.got_saved.saturating_add(1);
        // C: ch[cn].hp = 1 * POWERSCALE;
        character.hp = POWERSCALE;
        let saves_left = character.saves;
        let rest = (character.rest_x, character.rest_y);

        // C: remove_all_poison(cn); extinguish(cn);
        self.remove_all_poison(character_id);
        self.remove_show_effect_type(character_id, EF_BURN);

        self.queue_system_text(
            character_id,
            "Ishtar's hand reaches down and saves thee from certain death.",
        );
        self.queue_system_text(
            character_id,
            format!("Thou hast {} saves left.", legacy_save_number(saves_left)),
        );

        // C `transfer_to_restarea` (same-area case only; cross-server
        // transfer is out of scope - see the "Cross-area transfer" P3 task).
        // Fall back to the current position if no rest position is set yet
        // (matches the same fallback already used below in `die_character`).
        let target = if rest.0 != 0 { rest } else { (x, y) };
        self.remove_character_from_map(character_id);
        self.place_character_on_map(character_id, usize::from(target.0), usize::from(target.1));
    }

    /// C `die_char(cn, co, ispk)`: the death animation finished. NPCs drop a
    /// lootable body and are destroyed; players lose experience/items and
    /// return to their rest position.
    pub(crate) fn die_character(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let is_pk_death = character.act2 != 0;
        let killer_id = (character.act1 > 0).then(|| CharacterId(character.act1 as u32));
        let is_player = character.flags.contains(CharacterFlags::PLAYER);
        let x = character.x;
        let y = character.y;
        let sprite = character.sprite;
        let dir = character.dir;
        let name = character.name.clone();
        let flags = character.flags;
        let template_key = character.template_key.clone();

        // C: remove_char(cn) + destroy_chareffects(cn).
        self.remove_character_from_map(character_id);
        self.remove_character_effects(character_id);

        // Body creation rules.
        let mut body_id: Option<ItemId> = None;
        if flags.contains(CharacterFlags::NOBODY) {
            // No body; given items drop, everything else is destroyed below.
        } else if flags.contains(CharacterFlags::ITEMDEATH) {
            // The slot 30 item becomes the drop.
            if let Some(item_id) = self
                .characters
                .get_mut(&character_id)
                .and_then(|character| character.inventory.get_mut(30).and_then(Option::take))
            {
                body_id = Some(item_id);
            }
        } else {
            let has_loot = self.character_has_death_loot(character_id);
            if has_loot || is_player {
                let body = self.create_dead_body_item(&name, sprite, dir, is_player, character_id);
                body_id = Some(body);
            }
        }

        // Drop the body like C drop_grave (extended drop, distance 5).
        let mut dropped_body = None;
        if let Some(id) = body_id {
            if self.drop_body_item(id, usize::from(x), usize::from(y)) {
                dropped_body = Some(id);
                let takeable = self
                    .items
                    .get(&id)
                    .is_some_and(|item| item.flags.contains(ItemFlags::TAKE));
                if !takeable {
                    let decay = if flags.contains(CharacterFlags::ITEMDEATH) {
                        self.settings.npc_body_decay_time
                    } else if is_player {
                        self.settings.player_body_decay_time
                    } else {
                        self.settings.npc_body_decay_time
                    };
                    self.set_item_expire(id, decay.max(1) as u64);
                }
            } else {
                self.destroy_item(id);
            }
        }

        // Inventory disposal.
        if flags.contains(CharacterFlags::NOBODY) {
            self.drop_given_items_and_destroy_rest(character_id, usize::from(x), usize::from(y));
        } else if !flags.contains(CharacterFlags::ITEMDEATH) {
            if let Some(body) = dropped_body {
                self.fill_body_container(character_id, body, killer_id, is_player);
                // C `die_char` (`death.c:741`): `if (!(ch[cn].flags &
                // CF_PLAYER)) apply_death_loot_for_template(ct, co, tmp);`
                // - runs after the natural inventory transfer so existing
                // drops aren't displaced. Deferred to the server drain
                // since rolling needs the killer's `PlayerRuntime` quest
                // log for condition evaluation (see `PendingDeathLootRoll`).
                if !is_player {
                    self.pending_death_loot_rolls.push(PendingDeathLootRoll {
                        container_id: body,
                        killer_id,
                        template_key,
                    });
                }
            } else {
                self.destroy_all_carried_items(character_id);
            }
        } else {
            self.destroy_all_carried_items(character_id);
        }

        if is_player {
            // C player death: exp loss, resource restore, return to rest.
            let mut messages: Vec<String> = Vec::new();
            if let Some(character) = self.characters.get_mut(&character_id) {
                if is_pk_death {
                    messages.push(
                        "Thou died by the hands of a player. Thou did not lose any experience points."
                            .to_string(),
                    );
                } else {
                    let loss = if character.flags.contains(CharacterFlags::HARDCORE) {
                        character.exp / 4
                    } else {
                        let mut loss = character.exp / 25;
                        if loss < 25 {
                            loss = 0;
                        } else {
                            let minus = i64::from(character.exp_used) - i64::from(character.exp);
                            if minus > 0 {
                                let cnt = minus / i64::from(loss) + 4;
                                loss = (i64::from(loss) * 3 / cnt) as u32;
                            }
                        }
                        loss
                    };
                    if loss > 0 {
                        messages.push("Thou died and lost some experience points.".to_string());
                    } else {
                        messages.push(
                            "Thou died, but since thou art still a Newbie, thou did not lose any experience points."
                                .to_string(),
                        );
                    }
                    character.exp = character.exp.saturating_sub(loss);
                }

                character.flags.remove(CharacterFlags::DEAD);
                character.flags.insert(CharacterFlags::ALIVE);
                character.hp = i32::from(character.values[0][0]) * POWERSCALE;
                character.endurance = i32::from(character.values[0][1]) * POWERSCALE;
                character.mana = i32::from(character.values[0][2]) * POWERSCALE;
                character.action = 0;
                character.duration = 0;
                character.step = 0;
                character.act1 = 0;
                character.act2 = 0;
                character.driver_messages.clear();
                character
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            for message in messages {
                self.queue_system_text(character_id, message);
            }

            // C transfer_to_restarea: drop at rest position (same-area only).
            let rest = self
                .characters
                .get(&character_id)
                .map(|character| (character.rest_x, character.rest_y))
                .unwrap_or_default();
            let target = if rest.0 != 0 { rest } else { (x, y) };
            if self.place_character_on_map(
                character_id,
                usize::from(target.0),
                usize::from(target.1),
            ) {
                // C `die_char` (`src/system/death.c:807`): `update_char(cn)`
                // once the player is back at the respawn point (skipped on
                // the cross-area-handoff early return, which Rust's
                // same-area-only `place_character_on_map` failure mirrors).
                self.update_character(character_id);
            }
            false
        } else {
            // C destroy_char(cn).
            self.destroy_all_carried_items(character_id);
            self.remove_character(character_id);
            true
        }
    }

    fn character_has_death_loot(&self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        if character.gold > 0 || character.cursor_item.is_some() {
            return true;
        }
        character
            .inventory
            .iter()
            .skip(INVENTORY_START_INVENTORY)
            .any(Option::is_some)
    }

    /// C dead body creation with the legacy sprite formula and description.
    fn create_dead_body_item(
        &mut self,
        name: &str,
        sprite: i32,
        dir: u8,
        is_player: bool,
        character_id: CharacterId,
    ) -> ItemId {
        let id = self.next_runtime_item_id();
        let mut flags = ItemFlags::USED | ItemFlags::USE;
        if is_player {
            flags |= ItemFlags::PLAYERBODY;
        }
        let colors = self
            .characters
            .get(&character_id)
            .map(|character| (character.c1, character.c2, character.c3))
            .unwrap_or_default();
        let mut driver_data = vec![0u8; 12];
        if is_player {
            driver_data[2..4].copy_from_slice(&colors.0.to_le_bytes());
            driver_data[4..6].copy_from_slice(&colors.1.to_le_bytes());
            driver_data[6..8].copy_from_slice(&colors.2.to_le_bytes());
        }
        let body = Item {
            id,
            name: "Body".to_string(),
            description: format!("{name}'s body."),
            flags,
            // C: it[in].sprite = 100000 + sprite * 1000 + (dir - 1) / 2 * 8 + 335;
            sprite: 100_000 + sprite * 1000 + (i32::from(dir).max(1) - 1) / 2 * 8 + 335,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; MAX_MODIFIERS],
            modifier_value: [0; MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 1,
            driver: 0,
            driver_data,
            serial: id.0,
        };
        self.items.insert(id, body);
        id
    }

    pub(super) fn drop_body_item(&mut self, item_id: ItemId, x: usize, y: usize) -> bool {
        let Some(mut item) = self.items.remove(&item_id) else {
            return false;
        };
        item.carried_by = None;
        item.contained_in = None;
        if self.map.drop_item_extended(&mut item, x, y, 5) {
            let (ix, iy) = (usize::from(item.x), usize::from(item.y));
            self.add_item(item);
            self.mark_dirty_sector(ix, iy);
            true
        } else {
            self.items.insert(item_id, item);
            false
        }
    }

    /// C die_char inventory transfer: spells destroyed, worn equipment is
    /// kept except two random pieces (never the weapon slot), everything
    /// else moves into the body container, gold becomes a contained money
    /// item. Kept NPC equipment is freed later by `destroy_char`.
    fn fill_body_container(
        &mut self,
        character_id: CharacterId,
        body_id: ItemId,
        killer_id: Option<CharacterId>,
        _is_player: bool,
    ) {
        // C `die_char`'s `create_item_container(in)` success branch
        // (`death.c:684-691`): `con[ct].owner = charID(cn); ... con[ct].
        // killer = charID(co); con[ct].access = 0;` - the grave-access ACL
        // triad, run once right before the equipment-loss shuffle below.
        if let Some(body) = self.items.get_mut(&body_id) {
            crate::item_driver::set_grave_acl(body, character_id, killer_id);
        }

        // C: shuffle of {0,1,2,3,4,5,7,8,9,10,11}, take first two.
        let mut slots = [0usize, 1, 2, 3, 4, 5, 7, 8, 9, 10, 11];
        for i in (1..slots.len()).rev() {
            let j = legacy_random_below_from_seed(&mut self.legacy_random_seed, (i + 1) as u32)
                as usize;
            slots.swap(i, j);
        }
        let eq_loss = [slots[0], slots[1]];

        let Some(character) = self.characters.get_mut(&character_id) else {
            return;
        };
        let mut to_container: Vec<ItemId> = Vec::new();
        let mut to_destroy: Vec<ItemId> = Vec::new();
        for slot in 0..character.inventory.len() {
            let Some(item_id) = character.inventory[slot] else {
                continue;
            };
            if (SPELL_SLOT_START..SPELL_SLOT_END).contains(&slot) {
                to_destroy.push(item_id);
                character.inventory[slot] = None;
            } else if LEGACY_EQUIPMENT_SLOTS.contains(&slot) {
                if slot == eq_loss[0] || slot == eq_loss[1] {
                    to_container.push(item_id);
                    character.inventory[slot] = None;
                }
                // otherwise the equipment stays with the character
            } else {
                to_container.push(item_id);
                character.inventory[slot] = None;
            }
        }
        if let Some(item_id) = character.cursor_item.take() {
            to_container.push(item_id);
        }
        let gold = std::mem::take(&mut character.gold);

        for item_id in to_destroy {
            self.destroy_item(item_id);
        }
        for item_id in to_container {
            if let Some(item) = self.items.get_mut(&item_id) {
                item.carried_by = None;
                item.contained_in = Some(body_id);
                item.x = 0;
                item.y = 0;
            }
        }
        if gold > 0 {
            let money_id = self.next_runtime_item_id();
            let money = Item {
                id: money_id,
                name: "Money".to_string(),
                description: format!("{gold} gold coins."),
                flags: ItemFlags::USED | ItemFlags::TAKE | ItemFlags::MONEY,
                sprite: legacy_money_sprite(gold),
                value: gold,
                min_level: 0,
                max_level: 0,
                needs_class: 0,
                template_id: 0,
                owner_id: 0,
                modifier_index: [0; MAX_MODIFIERS],
                modifier_value: [0; MAX_MODIFIERS],
                x: 0,
                y: 0,
                carried_by: None,
                contained_in: Some(body_id),
                content_id: 0,
                driver: 0,
                driver_data: Vec::new(),
                serial: gold,
            };
            self.items.insert(money_id, money);
        }
    }

    /// C NOBODY handling: given items drop on the ground, the rest is freed.
    fn drop_given_items_and_destroy_rest(&mut self, character_id: CharacterId, x: usize, y: usize) {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return;
        };
        let mut carried: Vec<ItemId> = character.inventory.iter().flatten().copied().collect();
        carried.extend(character.cursor_item.take());
        for slot in character.inventory.iter_mut() {
            *slot = None;
        }
        character.gold = 0;
        for item_id in carried {
            let given = self
                .items
                .get(&item_id)
                .is_some_and(|item| item.flags.contains(ItemFlags::GIVEN_ITEM));
            if given {
                if let Some(mut item) = self.items.remove(&item_id) {
                    item.carried_by = None;
                    if self.map.drop_item_extended(&mut item, x, y, 2) {
                        let (ix, iy) = (usize::from(item.x), usize::from(item.y));
                        self.add_item(item);
                        self.mark_dirty_sector(ix, iy);
                    }
                }
            } else {
                self.destroy_item(item_id);
            }
        }
    }

    fn destroy_all_carried_items(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return;
        };
        let mut carried: Vec<ItemId> = character.inventory.iter().flatten().copied().collect();
        carried.extend(character.cursor_item.take());
        for slot in character.inventory.iter_mut() {
            *slot = None;
        }
        character.gold = 0;
        for item_id in carried {
            self.destroy_item(item_id);
        }
    }

    /// C `remove_char`: unhook the character from the map without deleting
    /// runtime state (players keep existing while dead).
    fn remove_character_from_map(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return;
        };
        let old_x = usize::from(character.x);
        let old_y = usize::from(character.y);
        if old_x == 0 && old_y == 0 {
            return;
        }
        let mut character_copy = character.clone();
        remove_character_light(&mut self.map, &character_copy);
        self.map.remove_char(&mut character_copy);
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.x = 0;
            character.y = 0;
        }
        self.mark_character_light_area(&character_copy);
        self.mark_dirty_sector(old_x, old_y);
    }

    fn remove_character_effects(&mut self, character_id: CharacterId) {
        let attached: Vec<u32> = self
            .effects
            .iter()
            .filter(|(_, effect)| effect.target_character == Some(character_id))
            .map(|(id, _)| *id)
            .collect();
        for effect_id in attached {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
        }
    }

    fn place_character_on_map(&mut self, character_id: CharacterId, x: usize, y: usize) -> bool {
        let Some(mut character) = self.characters.remove(&character_id) else {
            return false;
        };
        let placed = self.map.drop_char_extended(&mut character, x, y, 10);
        if placed {
            add_character_light(&mut self.map, &character);
            self.mark_character_light_area(&character);
            self.mark_dirty_sector(usize::from(character.x), usize::from(character.y));
        }
        self.characters.insert(character_id, character);
        placed
    }
}

/// C `save_number(nr)` (`src/system/tool.c`): spells out save counts for the
/// death/save feedback text ("Thou hast six saves left.").
pub fn legacy_save_number(saves: u8) -> String {
    match saves {
        0 => "no".to_string(),
        1 => "one".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        6 => "six".to_string(),
        7 => "seven".to_string(),
        8 => "eight".to_string(),
        9 => "nine".to_string(),
        10 => "ten".to_string(),
        other => other.to_string(),
    }
}

/// C `create_money_item` sprite ladder.
pub(crate) fn legacy_money_sprite(amount: u32) -> i32 {
    if amount > 9_999_999 {
        109
    } else if amount > 999_999 {
        108
    } else if amount > 99_999 {
        107
    } else if amount > 9_999 {
        106
    } else if amount > 999 {
        105
    } else if amount > 99 {
        104
    } else if amount > 9 {
        103
    } else {
        102
    }
}
