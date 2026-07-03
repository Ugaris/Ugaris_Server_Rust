use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldActionCompletion {
    pub character_id: CharacterId,
    pub action_id: u16,
    pub action_item_id: Option<ItemId>,
    pub ok: bool,
    pub legacy_return_code: i32,
    pub item_use: Option<ItemUseRequest>,
    pub old_x: u16,
    pub old_y: u16,
    pub new_x: u16,
    pub new_y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookMapRequest {
    pub character_id: CharacterId,
    pub x: usize,
    pub y: usize,
    pub character_level: u32,
    pub visible: bool,
}

/// C player-driver queue task priorities from `run_queue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueuedTaskClass {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueuedTaskResult {
    /// C return 1: the task set up an action; stop processing.
    Started,
    /// C return 2: the task failed permanently; drop it and keep scanning.
    Discard,
    /// C return 0: transient failure; the task stays queued.
    Keep,
}

fn queued_task_class(action: PlayerActionCode) -> Option<QueuedTaskClass> {
    match action {
        PlayerActionCode::Bless | PlayerActionCode::Heal | PlayerActionCode::MagicShield => {
            Some(QueuedTaskClass::High)
        }
        PlayerActionCode::Freeze
        | PlayerActionCode::Flash
        | PlayerActionCode::Warcry
        | PlayerActionCode::Pulse => Some(QueuedTaskClass::Medium),
        PlayerActionCode::Fireball
        | PlayerActionCode::FireballCharacter
        | PlayerActionCode::Ball
        | PlayerActionCode::BallCharacter => Some(QueuedTaskClass::Low),
        _ => None,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TileSpecialOutcome {
    pub damage: i32,
    pub bubble_effect_id: Option<u32>,
    pub sound_type: Option<u32>,
}

impl World {
    pub fn advance(&mut self) {
        self.tick.0 += 1;
    }

    pub fn drain_look_map_requests(&mut self) -> Vec<LookMapRequest> {
        self.pending_look_maps.drain(..).collect()
    }

    pub fn advance_character_action(&mut self, character_id: CharacterId) -> Option<bool> {
        self.characters
            .get_mut(&character_id)
            .map(advance_action_step)
    }

    pub fn tile_special_check(&mut self, character_id: CharacterId) -> TileSpecialOutcome {
        let Some(character) = self.characters.get(&character_id) else {
            return TileSpecialOutcome::default();
        };
        if !character.flags.contains(CharacterFlags::PLAYER) {
            return TileSpecialOutcome::default();
        }

        let x = usize::from(character.x);
        let y = usize::from(character.y);
        let Some(tile) = self.map.tile(x, y).copied() else {
            return TileSpecialOutcome::default();
        };
        if !tile.flags.contains(MapFlags::SLOWDEATH) {
            return TileSpecialOutcome::default();
        }

        if tile.flags.contains(MapFlags::UNDERWATER) {
            if !character.flags.contains(CharacterFlags::OXYGEN) {
                self.apply_legacy_hurt(character_id, None, 50, 1, 0, 0);
                return TileSpecialOutcome {
                    damage: 50,
                    bubble_effect_id: None,
                    sound_type: None,
                };
            }

            let cadence = self
                .tick
                .0
                .wrapping_add(u64::from(character_id.0).wrapping_mul(32));
            if cadence % 6 == 0 && (cadence / TICKS_PER_SECOND) % 3 == 0 {
                let bubble_effect_id = self.create_bubble_effect(x as i32, y as i32, 45, 1);
                let sound_type = (cadence % 12 == 0).then(|| {
                    44 + legacy_random_variant_below_from_seed(&mut self.legacy_random_seed, 3)
                });
                if let Some(sound_type) = sound_type {
                    self.queue_sound_area(x, y, sound_type);
                }
                return TileSpecialOutcome {
                    damage: 0,
                    bubble_effect_id: Some(bubble_effect_id),
                    sound_type,
                };
            }
            return TileSpecialOutcome::default();
        }

        let sprite = tile.ground_sprite & 0xffff;
        let damage = if (59706..=59709).contains(&sprite) {
            250
        } else {
            100
        };
        self.apply_legacy_hurt(character_id, None, damage, 1, 25, 66);
        TileSpecialOutcome {
            damage,
            bubble_effect_id: None,
            sound_type: None,
        }
    }

    pub fn reset_character_action(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        reset_action_after_act(character);
        true
    }

    pub fn complete_walk(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let before = character.clone();
        remove_character_light(&mut self.map, character);
        let walked = act_walk(character, &mut self.map);
        add_character_light(&mut self.map, character);
        let after = character.clone();
        self.mark_character_light_area(&before);
        self.mark_character_light_area(&after);
        walked
    }

    pub fn complete_take(
        &mut self,
        character_id: CharacterId,
        item_id: ItemId,
        can_carry: bool,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        let before = item.clone();
        if !act_take(character, &mut self.map, item, can_carry) {
            return false;
        }
        remove_item_light(&mut self.map, &before);
        self.mark_item_light_area(&before);
        true
    }

    pub fn complete_drop(&mut self, character_id: CharacterId, item_id: ItemId) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if !act_drop(character, &mut self.map, item) {
            return false;
        }
        let after = item.clone();
        add_item_light(&mut self.map, item);
        self.mark_item_light_area(&after);
        true
    }

    pub fn complete_give(&mut self, giver_id: CharacterId, receiver_id: CharacterId) -> bool {
        let Some(giver) = self.characters.get(&giver_id) else {
            return false;
        };
        let Some(direction) = Direction::try_from(giver.dir).ok() else {
            return false;
        };
        let (dx, dy) = direction.delta();
        let Some(target_x) = offset_coordinate(usize::from(giver.x), dx) else {
            return false;
        };
        let Some(target_y) = offset_coordinate(usize::from(giver.y), dy) else {
            return false;
        };
        if !self.map.legacy_inner_bounds(target_x, target_y)
            || self.map.tile(target_x, target_y).map(|tile| tile.character)
                != Some(receiver_id.0 as u16)
        {
            return false;
        }
        self.transfer_cursor_item(giver_id, receiver_id)
    }

    pub fn complete_use(
        &mut self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Option<ItemUseRequest> {
        let character = self.characters.get_mut(&character_id)?;
        let item = self.items.get(&item_id)?;
        act_use(character, &self.map, item)
    }

    pub(crate) fn map_character_at(&self, x: i32, y: i32) -> Option<CharacterId> {
        if x < 0 || y < 0 {
            return None;
        }
        self.map.tile(x as usize, y as usize).and_then(|tile| {
            (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
        })
    }

    pub fn apply_player_action_setup(&mut self, player: &mut PlayerRuntime, area_id: u16) -> bool {
        let Some(character_id) = player.character_id else {
            return false;
        };

        // C player driver: dead characters cannot act until `die_char`
        // finished; new actions must not cancel the AC_DIE animation.
        if self
            .characters
            .get(&character_id)
            .is_some_and(|character| character.flags.contains(CharacterFlags::DEAD))
        {
            return false;
        }

        // C `run_queue`: queued spells execute before the persistent player
        // action continues, in high/medium/low priority passes.
        if self.run_player_spell_queue(player, character_id, area_id) {
            return true;
        }

        match player.action.action {
            PlayerActionCode::Idle => self
                .characters
                .get_mut(&character_id)
                .is_some_and(|character| do_idle(character, 4).is_ok()),
            PlayerActionCode::Move => {
                let Some((target_x, target_y)) =
                    valid_map_coords(player.action.arg1, player.action.arg2)
                else {
                    return self.set_player_idle(player, character_id);
                };

                if self.setup_player_move(character_id, target_x, target_y, area_id) {
                    true
                } else {
                    self.set_player_idle(player, character_id)
                }
            }
            PlayerActionCode::WalkDir => {
                let Some(character) = self.characters.get_mut(&character_id) else {
                    return false;
                };
                let direction = player.action.arg1 as u8;
                if do_walk(character, &mut self.map, direction, area_id).is_ok() {
                    true
                } else if diagonal_slide_alternates(direction).is_some_and(|(alt1, alt2)| {
                    do_walk(character, &mut self.map, alt1 as u8, area_id).is_ok()
                        || do_walk(character, &mut self.map, alt2 as u8, area_id).is_ok()
                }) {
                    true
                } else {
                    player.action.action = PlayerActionCode::Idle;
                    do_idle(character, 4).is_ok()
                }
            }
            PlayerActionCode::Take => {
                let Some((target_x, target_y)) =
                    valid_map_coords(player.action.arg1, player.action.arg2)
                else {
                    return self.set_player_idle(player, character_id);
                };

                let item_id = self
                    .map
                    .tile(target_x, target_y)
                    .map(|tile| tile.item)
                    .unwrap_or_default();
                if item_id == 0 {
                    return self.set_player_idle(player, character_id);
                }

                let Some(character) = self.characters.get(&character_id) else {
                    return false;
                };
                let direction = adjacent_direction(character.x, character.y, target_x, target_y);

                if let Some(direction) = direction {
                    let Some(item) = self.items.get(&ItemId(item_id)) else {
                        return self.set_player_idle(player, character_id);
                    };
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        return false;
                    };

                    if do_take(character, &self.map, item, direction as u8, true).is_ok() {
                        true
                    } else {
                        self.set_player_idle(player, character_id)
                    }
                } else if self.setup_walk_toward(
                    character_id,
                    target_x,
                    target_y,
                    1,
                    area_id,
                    false,
                ) {
                    true
                } else {
                    self.set_player_idle(player, character_id)
                }
            }
            PlayerActionCode::Drop => {
                let Some((target_x, target_y)) =
                    valid_map_coords(player.action.arg1, player.action.arg2)
                else {
                    return self.set_player_idle(player, character_id);
                };
                let Some(character) = self.characters.get(&character_id) else {
                    return false;
                };
                let Some(item_id) = character.cursor_item else {
                    return self.set_player_idle(player, character_id);
                };

                if self.map.tile(target_x, target_y).is_none_or(|tile| {
                    tile.item != 0 || self.map.blocks_movement(target_x, target_y)
                }) {
                    return self.set_player_idle(player, character_id);
                }

                if let Some(direction) =
                    adjacent_direction(character.x, character.y, target_x, target_y)
                {
                    let Some(item) = self.items.get(&item_id) else {
                        return self.set_player_idle(player, character_id);
                    };
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        return false;
                    };

                    if do_drop(character, &self.map, item, direction as u8).is_ok() {
                        true
                    } else {
                        self.set_player_idle(player, character_id)
                    }
                } else if self.setup_walk_toward(
                    character_id,
                    target_x,
                    target_y,
                    1,
                    area_id,
                    false,
                ) {
                    true
                } else {
                    self.set_player_idle(player, character_id)
                }
            }
            PlayerActionCode::Use => {
                let Some((target_x, target_y)) =
                    valid_map_coords(player.action.arg1, player.action.arg2)
                else {
                    return self.set_player_idle(player, character_id);
                };

                let item_id = self
                    .map
                    .tile(target_x, target_y)
                    .map(|tile| tile.item)
                    .unwrap_or_default();
                let Some(item) = (item_id != 0)
                    .then_some(ItemId(item_id))
                    .and_then(|item_id| self.items.get(&item_id))
                else {
                    return self.set_player_idle(player, character_id);
                };
                if !item.flags.contains(ItemFlags::USE) {
                    return self.set_player_idle(player, character_id);
                }

                let Some(character) = self.characters.get(&character_id) else {
                    return false;
                };
                let direction = adjacent_use_direction(
                    character.x,
                    character.y,
                    target_x,
                    target_y,
                    item.flags.contains(ItemFlags::FRONTWALL),
                );

                if let Some(direction) = direction {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        return false;
                    };
                    let Some(item) = self.items.get(&ItemId(item_id)) else {
                        return self.set_player_idle(player, character_id);
                    };

                    if do_use(character, &self.map, item, direction as u8, 0).is_ok() {
                        true
                    } else {
                        self.set_player_idle(player, character_id)
                    }
                } else if self.setup_walk_toward_use_item(
                    character_id,
                    usize::from(item.x),
                    usize::from(item.y),
                    item.flags,
                    area_id,
                ) {
                    true
                } else {
                    self.set_player_idle(player, character_id)
                }
            }
            PlayerActionCode::Kill => {
                let target_id = CharacterId(player.action.arg1 as u32);
                let Some(target) = self.characters.get(&target_id) else {
                    return self.set_player_idle(player, character_id);
                };
                let target_x = usize::from(target.x);
                let target_y = usize::from(target.y);
                let Some(attacker) = self.characters.get(&character_id) else {
                    return false;
                };
                let attack_policy = RuntimePlayerAttackPolicy {
                    attacker_runtime: player,
                };
                if !can_attack_in_area_with_clan_policy(
                    attacker,
                    target,
                    &self.map,
                    area_id,
                    &attack_policy,
                ) {
                    self.remove_stale_pvp_hate_if_legacy_check_fails(
                        player,
                        character_id,
                        target_id,
                        area_id,
                    );
                    return self.set_player_idle(player, character_id);
                }
                let direction = adjacent_direction(attacker.x, attacker.y, target_x, target_y);

                if let Some(direction) = direction {
                    let target = target.clone();
                    let Some(attacker) = self.characters.get_mut(&character_id) else {
                        return false;
                    };
                    if do_attack(
                        attacker,
                        &self.map,
                        &target,
                        direction as u8,
                        action::ATTACK1,
                    )
                    .is_ok()
                    {
                        true
                    } else {
                        self.set_player_idle(player, character_id)
                    }
                } else if self.setup_walk_toward(
                    character_id,
                    target_x,
                    target_y,
                    1,
                    area_id,
                    false,
                ) {
                    true
                } else {
                    self.set_player_idle(player, character_id)
                }
            }
            PlayerActionCode::Teleport => {
                let Some((item_id, direction)) = self
                    .characters
                    .get(&character_id)
                    .and_then(|character| item_in_facing_direction(character, &self.map))
                else {
                    return self.set_player_idle(player, character_id);
                };
                let Some(item) = self.items.get(&item_id) else {
                    return self.set_player_idle(player, character_id);
                };
                if !item.flags.contains(ItemFlags::USE) {
                    return self.set_player_idle(player, character_id);
                }
                let Some(character) = self.characters.get_mut(&character_id) else {
                    return false;
                };

                if do_use(
                    character,
                    &self.map,
                    item,
                    direction as u8,
                    player.action.arg1 + 1,
                )
                .is_ok()
                {
                    true
                } else {
                    self.set_player_idle(player, character_id)
                }
            }
            PlayerActionCode::LookMap => {
                let Some((target_x, target_y)) =
                    valid_map_coords(player.action.arg1, player.action.arg2)
                else {
                    return self.set_player_idle(player, character_id);
                };
                if !self.map.legacy_index(target_x, target_y).is_some() {
                    return self.set_player_idle(player, character_id);
                }
                let Some(character) = self.characters.get_mut(&character_id) else {
                    return false;
                };
                if character.flags.contains(CharacterFlags::DEAD) {
                    return self.set_player_idle(player, character_id);
                }

                if let Some(direction) = offset_to_direction(
                    usize::from(character.x),
                    usize::from(character.y),
                    target_x,
                    target_y,
                ) {
                    character.dir = direction as u8;
                }
                let visible = self.map.can_see(
                    usize::from(character.x),
                    usize::from(character.y),
                    target_x,
                    target_y,
                    DIST_MAX,
                );
                self.pending_look_maps.push(LookMapRequest {
                    character_id,
                    x: target_x,
                    y: target_y,
                    character_level: character.level,
                    visible,
                });
                self.set_player_idle(player, character_id)
            }
            PlayerActionCode::Give => {
                let receiver_id = CharacterId(player.action.arg1 as u32);
                let Some(receiver) = self.characters.get(&receiver_id) else {
                    return self.set_player_idle(player, character_id);
                };
                let target_x = usize::from(receiver.x);
                let target_y = usize::from(receiver.y);
                let Some(giver) = self.characters.get(&character_id) else {
                    return false;
                };
                let direction = adjacent_direction(giver.x, giver.y, target_x, target_y);

                if let Some(direction) = direction {
                    if self.setup_give(character_id, receiver_id, direction) {
                        true
                    } else {
                        self.set_player_idle(player, character_id)
                    }
                } else if self.setup_walk_toward(
                    character_id,
                    target_x,
                    target_y,
                    1,
                    area_id,
                    false,
                ) {
                    true
                } else {
                    self.set_player_idle(player, character_id)
                }
            }
            PlayerActionCode::MagicShield => {
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| do_magicshield(character).is_ok())
                    || self.set_player_idle(player, character_id)
            }
            PlayerActionCode::Pulse => {
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| do_pulse(character).is_ok())
                    || self.set_player_idle(player, character_id)
            }
            PlayerActionCode::Warcry => {
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| do_warcry(character, &self.items).is_ok())
                    || self.set_player_idle(player, character_id)
            }
            PlayerActionCode::Freeze => {
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| do_freeze(character).is_ok())
                    || self.set_player_idle(player, character_id)
            }
            PlayerActionCode::Flash => {
                let current_tick = self.tick.0 as u32;
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| do_flash(character, &self.items, current_tick).is_ok())
                    || self.set_player_idle(player, character_id)
            }
            PlayerActionCode::Fireball => {
                let Some((target_x, target_y)) =
                    valid_map_coords(player.action.arg1, player.action.arg2)
                else {
                    return self.set_player_idle(player, character_id);
                };
                let current_tick = self.tick.0 as u32;
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| {
                        do_fireball(character, &self.items, target_x, target_y, current_tick)
                            .is_ok()
                    })
                    || self.set_player_idle(player, character_id)
            }
            PlayerActionCode::FireballCharacter => {
                let target_id = CharacterId(player.action.arg1 as u32);
                let target_serial = player.action.arg2 as u32;
                if !self.player_can_attack_target(player, character_id, target_id, area_id) {
                    return self.set_player_idle(player, character_id);
                }
                self.setup_fireball_character(character_id, target_id, target_serial)
                    || self.set_player_idle(player, character_id)
            }
            PlayerActionCode::Ball => {
                let Some((target_x, target_y)) =
                    valid_map_coords(player.action.arg1, player.action.arg2)
                else {
                    return self.set_player_idle(player, character_id);
                };
                let current_tick = self.tick.0 as u32;
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| {
                        do_ball(character, &self.items, target_x, target_y, current_tick).is_ok()
                    })
                    || self.set_player_idle(player, character_id)
            }
            PlayerActionCode::BallCharacter => {
                let target_id = CharacterId(player.action.arg1 as u32);
                let target_serial = player.action.arg2 as u32;
                if !self.player_can_attack_target(player, character_id, target_id, area_id) {
                    return self.set_player_idle(player, character_id);
                }
                self.setup_ball_character(character_id, target_id, target_serial)
                    || self.set_player_idle(player, character_id)
            }
            PlayerActionCode::Bless => {
                let target_id = CharacterId(player.action.arg1 as u32);
                if self.setup_bless_spell(character_id, target_id) {
                    true
                } else {
                    self.set_player_idle(player, character_id)
                }
            }
            PlayerActionCode::Heal => {
                let target_id = CharacterId(player.action.arg1 as u32);
                if self.setup_heal_spell(character_id, target_id) {
                    true
                } else {
                    self.set_player_idle(player, character_id)
                }
            }
        }
    }

    /// C `run_queue` from `src/system/player_driver.c`: scan the queued
    /// spell commands in three priority passes. Started tasks are consumed
    /// and end the pass (`return 1`), permanently failed tasks are consumed
    /// and scanning continues (`return 2`), transient failures stay queued
    /// (`return 0`, only mana-low bless in the idle context).
    pub(crate) fn run_player_spell_queue(
        &mut self,
        player: &mut PlayerRuntime,
        character_id: CharacterId,
        area_id: u16,
    ) -> bool {
        for class in [
            QueuedTaskClass::High,
            QueuedTaskClass::Medium,
            QueuedTaskClass::Low,
        ] {
            let mut index = 0;
            while index < player.queue.len() {
                let entry = player.queue[index];
                if queued_task_class(entry.action) != Some(class) {
                    index += 1;
                    continue;
                }
                match self.try_queued_spell_task(player, character_id, entry, area_id) {
                    QueuedTaskResult::Started => {
                        player.queue.remove(index);
                        return true;
                    }
                    QueuedTaskResult::Discard => {
                        player.queue.remove(index);
                    }
                    QueuedTaskResult::Keep => {
                        index += 1;
                    }
                }
            }
        }
        false
    }

    /// C `check_high_prio_task` / `check_med_prio_task` /
    /// `check_low_prio_task` bodies for a single queued spell entry.
    fn try_queued_spell_task(
        &mut self,
        player: &mut PlayerRuntime,
        character_id: CharacterId,
        entry: crate::player::QueuedAction,
        area_id: u16,
    ) -> QueuedTaskResult {
        let started = |ok: bool| {
            if ok {
                QueuedTaskResult::Started
            } else {
                QueuedTaskResult::Discard
            }
        };
        match entry.action {
            PlayerActionCode::Bless => {
                // C `error_state_mana`: bless waits in the queue for mana.
                if self
                    .characters
                    .get(&character_id)
                    .is_some_and(|character| character.mana < BLESS_COST)
                {
                    return QueuedTaskResult::Keep;
                }
                let target_id = CharacterId(entry.arg1 as u32);
                started(self.setup_bless_spell(character_id, target_id))
            }
            PlayerActionCode::Heal => {
                let target_id = CharacterId(entry.arg1 as u32);
                started(self.setup_heal_spell(character_id, target_id))
            }
            PlayerActionCode::MagicShield => started(
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| do_magicshield(character).is_ok()),
            ),
            PlayerActionCode::Freeze => started(
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| do_freeze(character).is_ok()),
            ),
            PlayerActionCode::Flash => {
                let current_tick = self.tick.0 as u32;
                started(
                    self.characters
                        .get_mut(&character_id)
                        .is_some_and(|character| {
                            do_flash(character, &self.items, current_tick).is_ok()
                        }),
                )
            }
            PlayerActionCode::Warcry => started(
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| do_warcry(character, &self.items).is_ok()),
            ),
            PlayerActionCode::Pulse => started(
                self.characters
                    .get_mut(&character_id)
                    .is_some_and(|character| do_pulse(character).is_ok()),
            ),
            PlayerActionCode::Fireball => {
                let Some((target_x, target_y)) = valid_map_coords(entry.arg1, entry.arg2) else {
                    return QueuedTaskResult::Discard;
                };
                let current_tick = self.tick.0 as u32;
                started(
                    self.characters
                        .get_mut(&character_id)
                        .is_some_and(|character| {
                            do_fireball(character, &self.items, target_x, target_y, current_tick)
                                .is_ok()
                        }),
                )
            }
            PlayerActionCode::FireballCharacter => {
                let target_id = CharacterId(entry.arg1 as u32);
                let target_serial = entry.arg2 as u32;
                if !self.player_can_attack_target(player, character_id, target_id, area_id) {
                    return QueuedTaskResult::Discard;
                }
                started(self.setup_fireball_character(character_id, target_id, target_serial))
            }
            PlayerActionCode::Ball => {
                let Some((target_x, target_y)) = valid_map_coords(entry.arg1, entry.arg2) else {
                    return QueuedTaskResult::Discard;
                };
                let current_tick = self.tick.0 as u32;
                started(
                    self.characters
                        .get_mut(&character_id)
                        .is_some_and(|character| {
                            do_ball(character, &self.items, target_x, target_y, current_tick)
                                .is_ok()
                        }),
                )
            }
            PlayerActionCode::BallCharacter => {
                let target_id = CharacterId(entry.arg1 as u32);
                let target_serial = entry.arg2 as u32;
                if !self.player_can_attack_target(player, character_id, target_id, area_id) {
                    return QueuedTaskResult::Discard;
                }
                started(self.setup_ball_character(character_id, target_id, target_serial))
            }
            _ => QueuedTaskResult::Keep,
        }
    }

    pub(crate) fn setup_give(
        &mut self,
        giver_id: CharacterId,
        receiver_id: CharacterId,
        direction: Direction,
    ) -> bool {
        if giver_id == receiver_id {
            return false;
        }
        let Some(giver) = self.characters.get(&giver_id) else {
            return false;
        };
        let Some(receiver) = self.characters.get(&receiver_id) else {
            return false;
        };
        if giver.flags.contains(CharacterFlags::DEAD)
            || receiver
                .flags
                .intersects(CharacterFlags::DEAD | CharacterFlags::NOGIVE)
        {
            return false;
        }
        let Some(item_id) = giver.cursor_item else {
            return false;
        };
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.flags.contains(ItemFlags::QUEST)
            && !giver
                .flags
                .intersects(CharacterFlags::QUESTITEM | CharacterFlags::GOD)
            && !receiver
                .flags
                .intersects(CharacterFlags::QUESTITEM | CharacterFlags::GOD)
        {
            return false;
        }
        if !can_receive_given_item(receiver) {
            return false;
        }

        if !receiver.flags.contains(CharacterFlags::PLAYER) {
            if let Some(item) = self.items.get_mut(&item_id) {
                item.flags.insert(ItemFlags::GIVEN_ITEM);
            }
        }

        let Some(giver) = self.characters.get_mut(&giver_id) else {
            return false;
        };
        giver.action = action::GIVE;
        giver.act1 = receiver_id.0 as i32;
        giver.duration = speed_ticks(
            character_value(giver, CharacterValue::Speed),
            giver.speed_mode,
            DUR_MISC_ACTION,
        );
        if giver.speed_mode == SpeedMode::Fast {
            giver.endurance -= endurance_cost(giver);
        }
        giver.dir = direction as u8;
        true
    }

    pub(crate) fn setup_player_move(
        &mut self,
        character_id: CharacterId,
        target_x: usize,
        target_y: usize,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let from_x = usize::from(character.x);
        let from_y = usize::from(character.y);
        if (from_x, from_y) == (target_x, target_y) {
            return false;
        }

        if self.setup_walk_toward(character_id, target_x, target_y, 0, area_id, false) {
            return true;
        }
        if manhattan_distance(from_x, from_y, target_x, target_y) < 2 {
            return false;
        }

        self.setup_walk_toward(character_id, target_x, target_y, 1, area_id, false)
            || self.setup_walk_toward(character_id, target_x, target_y, 1, area_id, true)
    }

    pub(crate) fn setup_walk_toward(
        &mut self,
        character_id: CharacterId,
        target_x: usize,
        target_y: usize,
        min_dist: usize,
        area_id: u16,
        ignore_characters: bool,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let from_x = usize::from(character.x);
        let from_y = usize::from(character.y);
        let result = if ignore_characters {
            pathfinder_ignore_characters(
                &self.map, from_x, from_y, target_x, target_y, min_dist, None,
            )
        } else {
            pathfinder(
                &self.map, from_x, from_y, target_x, target_y, min_dist, None,
            )
        };
        let Some(direction) = result.direction else {
            return false;
        };
        self.walk_or_use_driver(character_id, direction, area_id)
    }

    pub(crate) fn walk_or_use_driver(
        &mut self,
        character_id: CharacterId,
        direction: Direction,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_walk(character, &mut self.map, direction as u8, area_id).is_ok() {
            return true;
        }

        let (dx, dy) = direction.delta();
        let Some(x) = offset_coordinate(usize::from(character.x), dx) else {
            return false;
        };
        let Some(y) = offset_coordinate(usize::from(character.y), dy) else {
            return false;
        };
        let item_id = self
            .map
            .tile(x, y)
            .map(|tile| tile.item)
            .unwrap_or_default();
        let Some(item) = (item_id != 0)
            .then_some(ItemId(item_id))
            .and_then(|item_id| self.items.get(&item_id))
        else {
            return false;
        };
        do_use(character, &self.map, item, direction as u8, 0).is_ok()
    }

    pub fn walk_swap_or_use_driver(
        &mut self,
        character_id: CharacterId,
        direction: Direction,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_walk(character, &mut self.map, direction as u8, area_id).is_ok() {
            return true;
        }
        let _ = turn(character, direction as u8);

        if self.char_swap(character_id) {
            return true;
        }

        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let (dx, dy) = direction.delta();
        let Some(x) = offset_coordinate(usize::from(character.x), dx) else {
            return false;
        };
        let Some(y) = offset_coordinate(usize::from(character.y), dy) else {
            return false;
        };
        let item_id = self
            .map
            .tile(x, y)
            .map(|tile| tile.item)
            .unwrap_or_default();
        let Some(item) = (item_id != 0)
            .then_some(ItemId(item_id))
            .and_then(|item_id| self.items.get(&item_id))
        else {
            return false;
        };
        do_use(character, &self.map, item, direction as u8, 0).is_ok()
    }

    pub fn char_swap(&mut self, character_id: CharacterId) -> bool {
        let Some(actor) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Ok(direction) = Direction::try_from(actor.dir) else {
            return false;
        };
        let (dx, dy) = direction.delta();
        let Some(target_x) = offset_coordinate(usize::from(actor.x), dx) else {
            return false;
        };
        let Some(target_y) = offset_coordinate(usize::from(actor.y), dy) else {
            return false;
        };
        if !self.map.legacy_inner_bounds(target_x, target_y) {
            return false;
        }

        let Some(actor_tile) = self
            .map
            .tile(usize::from(actor.x), usize::from(actor.y))
            .copied()
        else {
            return false;
        };
        let Some(target_tile) = self.map.tile(target_x, target_y).copied() else {
            return false;
        };
        let target_id = CharacterId(u32::from(target_tile.character));
        if target_id.0 == 0 {
            return false;
        }
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };

        if !target.flags.intersects(
            CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE | CharacterFlags::ALLOWSWAP,
        ) || target.flags.contains(CharacterFlags::INVISIBLE)
            || actor.action != action::IDLE
            || target.action != action::IDLE
            || actor_tile.flags.contains(MapFlags::PEACE)
                != target_tile.flags.contains(MapFlags::PEACE)
            || actor_tile.flags.contains(MapFlags::UNDERWATER)
                != target_tile.flags.contains(MapFlags::UNDERWATER)
        {
            return false;
        }

        remove_character_light(&mut self.map, &actor);
        remove_character_light(&mut self.map, &target);
        self.mark_character_light_area(&actor);
        self.mark_character_light_area(&target);

        let actor_x = usize::from(actor.x);
        let actor_y = usize::from(actor.y);
        if let Some(tile) = self.map.tile_mut(actor_x, actor_y) {
            tile.character = target_id.0 as u16;
            tile.flags.insert(MapFlags::TMOVEBLOCK);
        }
        if let Some(tile) = self.map.tile_mut(target_x, target_y) {
            tile.character = character_id.0 as u16;
            tile.flags.insert(MapFlags::TMOVEBLOCK);
        }

        if let Some(actor_mut) = self.characters.get_mut(&character_id) {
            actor_mut.x = target_x as u16;
            actor_mut.y = target_y as u16;
            if target_tile.flags.contains(MapFlags::NOMAGIC) {
                actor_mut.flags.insert(CharacterFlags::NOMAGIC);
            } else {
                actor_mut.flags.remove(CharacterFlags::NOMAGIC);
            }
        }
        if let Some(target_mut) = self.characters.get_mut(&target_id) {
            target_mut.x = actor_x as u16;
            target_mut.y = actor_y as u16;
            if actor_tile.flags.contains(MapFlags::NOMAGIC) {
                target_mut.flags.insert(CharacterFlags::NOMAGIC);
            } else {
                target_mut.flags.remove(CharacterFlags::NOMAGIC);
            }
        }

        let actor_after = self.characters.get(&character_id).cloned();
        let target_after = self.characters.get(&target_id).cloned();
        if let Some(actor_after) = actor_after.as_ref() {
            add_character_light(&mut self.map, actor_after);
            self.mark_character_light_area(actor_after);
        }
        if let Some(target_after) = target_after.as_ref() {
            add_character_light(&mut self.map, target_after);
            self.mark_character_light_area(target_after);
        }
        self.mark_dirty_sector(actor_x, actor_y);
        self.mark_dirty_sector(target_x, target_y);
        true
    }

    pub(crate) fn setup_walk_direction(
        &mut self,
        character_id: CharacterId,
        direction: Direction,
        area_id: u16,
    ) -> bool {
        self.characters
            .get_mut(&character_id)
            .is_some_and(|character| {
                do_walk(character, &mut self.map, direction as u8, area_id).is_ok()
            })
    }

    pub(crate) fn setup_walk_toward_use_item(
        &mut self,
        character_id: CharacterId,
        item_x: usize,
        item_y: usize,
        item_flags: ItemFlags,
        area_id: u16,
    ) -> bool {
        if item_flags.contains(ItemFlags::FRONTWALL) {
            if !self.map.blocks_movement(item_x + 1, item_y)
                && self.setup_walk_toward(character_id, item_x + 1, item_y, 0, area_id, false)
            {
                return true;
            }
            if !self.map.blocks_movement(item_x, item_y + 1)
                && self.setup_walk_toward(character_id, item_x, item_y + 1, 0, area_id, false)
            {
                return true;
            }
            return false;
        }

        self.setup_walk_toward(character_id, item_x, item_y, 1, area_id, false)
    }

    pub(crate) fn set_player_idle(
        &mut self,
        player: &mut PlayerRuntime,
        character_id: CharacterId,
    ) -> bool {
        player.action.action = PlayerActionCode::Idle;
        self.characters
            .get_mut(&character_id)
            .is_some_and(|character| do_idle(character, 4).is_ok())
    }

    pub fn tick_basic_actions(&mut self) -> Vec<WorldActionCompletion> {
        self.tick_basic_actions_with_attack_policy(|_, caster, target, map| {
            can_attack(caster, target, map)
        })
    }

    pub fn tick_basic_actions_with_attack_policy(
        &mut self,
        mut can_attack_target: impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) -> Vec<WorldActionCompletion> {
        let character_ids: Vec<CharacterId> = self.characters.keys().copied().collect();
        let mut completed = Vec::new();

        for character_id in character_ids {
            self.tile_special_check(character_id);
            if self
                .characters
                .get(&character_id)
                .is_none_or(|character| character.action == 0)
            {
                continue;
            }
            if self.advance_character_action(character_id) != Some(true) {
                continue;
            }

            let action_id = self
                .characters
                .get(&character_id)
                .map(|character| character.action)
                .unwrap_or_default();

            // C `act()` (act.c:1877): stamp `regen_ticker` for every
            // non-idle/non-passive action, so `act_idle`'s regen delay gate
            // resets while the character is actively doing something.
            if !matches!(
                action_id,
                action::IDLE | action::MAGICSHIELD | action::BLESS_SELF | action::HEAL_SELF
            ) {
                let tick_now = self.tick.0.min(u64::from(u32::MAX)) as u32;
                if let Some(character) = self.characters.get_mut(&character_id) {
                    character.regen_ticker = tick_now;
                }
            }

            let action_item_id = self.characters.get(&character_id).and_then(|character| {
                (character.act1 > 0).then_some(ItemId(character.act1 as u32))
            });
            let mut item_use = None;
            let (old_x, old_y) = self
                .characters
                .get(&character_id)
                .map(|character| (character.x, character.y))
                .unwrap_or_default();
            let ok = match action_id {
                action::IDLE => true,
                action::WALK => self.complete_walk(character_id),
                action::TAKE => action_item_id
                    .is_some_and(|item_id| self.complete_take(character_id, item_id, true)),
                action::DROP => {
                    action_item_id.is_some_and(|item_id| self.complete_drop(character_id, item_id))
                }
                action::USE => action_item_id
                    .and_then(|item_id| self.complete_use(character_id, item_id))
                    .is_some_and(|request| {
                        item_use = Some(request);
                        true
                    }),
                action::ATTACK1 | action::ATTACK2 | action::ATTACK3 => self
                    .characters
                    .get(&character_id)
                    .and_then(|character| {
                        (character.act1 > 0).then_some(CharacterId(character.act1 as u32))
                    })
                    .is_some_and(|defender_id| {
                        let d100_roll = ((self.tick.0 + u64::from(character_id.0)) % 100) as i32;
                        let d6_roll = ((self.tick.0 + u64::from(defender_id.0)) % 6) as i32 + 1;
                        let clash_roll =
                            ((self.tick.0 + u64::from(character_id.0) + u64::from(defender_id.0))
                                % 2) as i32;
                        self.complete_attack_with_rolls_and_clash_roll(
                            character_id,
                            defender_id,
                            d100_roll,
                            d6_roll,
                            clash_roll,
                        )
                    }),
                action::GIVE => self
                    .characters
                    .get(&character_id)
                    .and_then(|character| {
                        (character.act1 > 0).then_some(CharacterId(character.act1 as u32))
                    })
                    .is_some_and(|receiver_id| self.complete_give(character_id, receiver_id)),
                action::MAGICSHIELD => self.complete_magicshield(character_id),
                action::PULSE => self.complete_pulse(character_id, &mut can_attack_target),
                action::FIREBALL1 => self.complete_fireball(character_id),
                action::FIREBALL2 => true,
                action::BALL1 => self.complete_ball(character_id),
                action::BALL2 => true,
                action::EARTHRAIN => self.complete_earthrain(character_id),
                action::EARTHMUD => self.complete_earthmud(character_id),
                action::FIRERING => self.complete_firering(character_id, &mut can_attack_target),
                action::FREEZE => self.complete_freeze(character_id, &mut can_attack_target),
                action::FLASH => self.complete_flash(character_id),
                action::WARCRY => self.complete_warcry(character_id, &mut can_attack_target),
                action::BLESS_SELF | action::BLESS1 | action::BLESS2 => self
                    .characters
                    .get(&character_id)
                    .and_then(|character| {
                        (character.act1 > 0).then_some(CharacterId(character.act1 as u32))
                    })
                    .is_some_and(|target_id| self.complete_bless(character_id, target_id)),
                action::HEAL_SELF | action::HEAL1 | action::HEAL2 => self
                    .characters
                    .get(&character_id)
                    .and_then(|character| {
                        (character.act1 > 0).then_some(CharacterId(character.act1 as u32))
                    })
                    .is_some_and(|target_id| self.complete_heal(character_id, target_id)),
                action::DIE => {
                    // C act_die -> die_char: NPCs are destroyed after the
                    // death animation, players return to their rest spot.
                    self.die_character(character_id);
                    true
                }
                _ => false,
            };

            if self
                .characters
                .get(&character_id)
                .is_some_and(|character| character.action == action_id)
            {
                self.reset_character_action(character_id);
            }
            let (new_x, new_y) = self
                .characters
                .get(&character_id)
                .map(|character| (character.x, character.y))
                .unwrap_or_default();
            completed.push(WorldActionCompletion {
                character_id,
                action_id,
                action_item_id,
                ok,
                legacy_return_code: i32::from(ok),
                item_use,
                old_x,
                old_y,
                new_x,
                new_y,
            });
        }

        completed
    }
}
