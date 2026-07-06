use super::*;

pub(crate) fn simple_baddy_enemy_hurtme(enemy: &SimpleBaddyEnemy) -> bool {
    enemy.priority == 1
}

impl World {
    pub fn process_simple_baddy_message_actions(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> Vec<ItemDriverOutcome> {
        let mut seed = self.legacy_random_seed;
        let outcomes =
            self.process_simple_baddy_message_actions_with_random(character_id, area_id, |limit| {
                legacy_random_below_from_seed(&mut seed, limit.max(0) as u32) as i32
            });
        self.legacy_random_seed = seed;
        outcomes
    }

    pub fn process_simple_baddy_message_actions_with_random(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        mut random_below: impl FnMut(i32) -> i32,
    ) -> Vec<ItemDriverOutcome> {
        let carried_items: Vec<Item> = self.items.values().cloned().collect();
        let Some(character) = self.characters.get(&character_id) else {
            return Vec::new();
        };
        if character.action != action::IDLE || character.flags.contains(CharacterFlags::DEAD) {
            return Vec::new();
        }
        self.clear_simple_baddy_bless_friend(character_id);
        let Some(character) = self.characters.get_mut(&character_id) else {
            return Vec::new();
        };
        let message_outcomes = process_simple_baddy_messages(character, &carried_items);

        let mut applied = Vec::new();
        for outcome in message_outcomes {
            match outcome {
                SimpleBaddyMessageOutcome::UseInventoryPotion { item_id, .. } => {
                    let outcome = self.execute_item_driver_request(
                        ItemDriverRequest::Driver {
                            driver: IDR_POTION,
                            item_id,
                            character_id,
                            spec: 0,
                        },
                        area_id,
                    );
                    applied.push(outcome);
                }
                SimpleBaddyMessageOutcome::BlessFriend { target_id } => {
                    if self.simple_baddy_can_bless_friend(character_id, target_id) {
                        self.remember_simple_baddy_bless_friend(character_id, target_id);
                    }
                    applied.push(ItemDriverOutcome::Noop);
                }
                SimpleBaddyMessageOutcome::PoisonHit {
                    target_id,
                    power,
                    poison_type,
                    chance,
                } => {
                    if self.simple_baddy_can_poison_hit(character_id, target_id)
                        && random_below(100) < chance
                    {
                        let _ = self.poison_character(target_id, power, poison_type);
                    }
                    applied.push(ItemDriverOutcome::Noop);
                }
                SimpleBaddyMessageOutcome::AddEnemy {
                    caller_id,
                    target_id,
                } => {
                    let tick = self.tick.0 as i32;
                    if let Some(caller) = self.characters.get(&caller_id).cloned() {
                        let tracking = self
                            .simple_baddy_enemy_tracking(character_id, target_id)
                            .map(|(_, x, y)| (false, x, y));
                        if let Some(character) = self.characters.get_mut(&character_id) {
                            let _ = add_simple_baddy_enemy(character, &caller, target_id, tick);
                            Self::apply_simple_baddy_enemy_tracking(character, target_id, tracking);
                        }
                        self.sort_simple_baddy_enemies_like_c(character_id);
                    }
                    applied.push(ItemDriverOutcome::Noop);
                }
                SimpleBaddyMessageOutcome::RemoveEnemy { target_id } => {
                    self.remove_simple_baddy_enemy(character_id, target_id);
                    applied.push(ItemDriverOutcome::Noop);
                }
                SimpleBaddyMessageOutcome::StandardAggro {
                    target_id,
                    priority,
                    require_visible,
                    hurtme,
                } => {
                    let tick = self.tick.0 as i32;
                    if self.simple_baddy_can_add_standard_enemy(
                        character_id,
                        target_id,
                        require_visible,
                        hurtme,
                    ) {
                        let tracking = self.simple_baddy_enemy_tracking(character_id, target_id);
                        if let Some(character) = self.characters.get_mut(&character_id) {
                            let _ = add_simple_baddy_enemy_unchecked(
                                character, target_id, priority, tick,
                            );
                            Self::apply_simple_baddy_enemy_tracking(character, target_id, tracking);
                        }
                        self.sort_simple_baddy_enemies_like_c(character_id);
                    }
                    applied.push(ItemDriverOutcome::Noop);
                }
                SimpleBaddyMessageOutcome::StandardSeenHit {
                    attacker_id,
                    victim_id,
                } => {
                    let tick = self.tick.0 as i32;
                    if let Some((target_id, hurtme)) =
                        self.simple_baddy_seen_hit_enemy(character_id, attacker_id, victim_id)
                    {
                        let tracking = self.simple_baddy_enemy_tracking(character_id, target_id);
                        if let Some(character) = self.characters.get_mut(&character_id) {
                            let priority = if hurtme { 1 } else { 0 };
                            let _ = add_simple_baddy_enemy_unchecked(
                                character, target_id, priority, tick,
                            );
                            Self::apply_simple_baddy_enemy_tracking(character, target_id, tracking);
                        }
                        self.sort_simple_baddy_enemies_like_c(character_id);
                    }
                    applied.push(ItemDriverOutcome::Noop);
                }
                SimpleBaddyMessageOutcome::TextNotification {
                    speaker_id, text, ..
                } => {
                    if let Some(text) = text.as_deref() {
                        self.apply_tabunga_text_notification(character_id, speaker_id, text);
                    }
                    applied.push(ItemDriverOutcome::Noop);
                }
                SimpleBaddyMessageOutcome::NoteHit => {
                    // C `fight_driver_note_hit` (`drvlib.c:2139-2147`): writes
                    // the independent `DRD_FIGHTDRIVER` slot's `lasthit`,
                    // not `simple_baddy`'s own data.
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        if matches!(
                            character.driver_state.as_ref(),
                            Some(CharacterDriverState::SimpleBaddy(_))
                        ) {
                            character
                                .fight_driver
                                .get_or_insert_with(FightDriverData::default)
                                .last_hit = self.tick.0 as i32;
                        }
                    }
                }
            }
        }
        applied
    }

    pub(crate) fn simple_baddy_recorded_enemy_ids(
        &self,
        character_id: CharacterId,
    ) -> Vec<CharacterId> {
        // C `DRD_FIGHTDRIVER` is a driver-independent slot (see
        // `FightDriverData`'s doc comment) - no `driver_state` gate here,
        // so a `CDR_LOSTCON`/player caller's own recorded enemies are seen
        // too.
        self.characters
            .get(&character_id)
            .and_then(|character| character.fight_driver.as_ref())
            .map(|data| data.enemies.iter().map(|enemy| enemy.target_id).collect())
            .unwrap_or_default()
    }

    pub(crate) fn apply_simple_baddy_enemy_tracking(
        character: &mut Character,
        target_id: CharacterId,
        tracking: Option<(bool, u16, u16)>,
    ) {
        let Some((visible, last_x, last_y)) = tracking else {
            return;
        };
        let Some(data) = character.fight_driver.as_mut() else {
            return;
        };
        if let Some(enemy) = data
            .enemies
            .iter_mut()
            .find(|enemy| enemy.target_id == target_id)
        {
            enemy.visible = visible;
            enemy.last_x = last_x;
            enemy.last_y = last_y;
        }
    }

    pub(crate) fn refresh_simple_baddy_enemy_tracking(
        &mut self,
        attacker: &Character,
    ) -> Vec<SimpleBaddyEnemy> {
        // C `fight_driver_update` (`drvlib.c:2170`) reads/writes the
        // driver-independent `DRD_FIGHTDRIVER` slot for any `cn` - no
        // `driver_state` gate, matching `CDR_LOSTCON`/player callers too.
        let enemies = match attacker.fight_driver.as_ref() {
            Some(data) => data.enemies.clone(),
            None => return Vec::new(),
        };
        let mut updated = Vec::new();
        for mut enemy in enemies {
            let Some(target) = self.characters.get(&enemy.target_id).cloned() else {
                continue;
            };
            if target.flags.contains(CharacterFlags::DEAD)
                || !can_attack(&attacker, &target, &self.map)
            {
                continue;
            }
            enemy.visible = char_see_char(attacker, &target, &self.map, self.date.daylight);
            if enemy.visible && self.simple_baddy_enemy_past_stop_dist(&attacker, &target) {
                continue;
            }
            if enemy.visible {
                enemy.last_x = target.x;
                enemy.last_y = target.y;
            }
            updated.push(enemy);
        }

        if let Some(character) = self.characters.get_mut(&attacker.id) {
            if let Some(data) = character.fight_driver.as_mut() {
                data.enemies = updated.clone();
            }
        }
        self.sort_simple_baddy_enemies_like_c(attacker.id);
        if let Some(data) = self
            .characters
            .get(&attacker.id)
            .and_then(|character| character.fight_driver.as_ref())
        {
            return data.enemies.clone();
        }
        updated
    }

    pub(crate) fn sort_simple_baddy_enemies_like_c(&mut self, character_id: CharacterId) {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return;
        };
        let mut enemies = match attacker.fight_driver.as_ref() {
            Some(data) => data.enemies.clone(),
            None => return,
        };
        enemies.sort_by(|left, right| {
            let left_distance = attacker.x.abs_diff(left.last_x) + attacker.y.abs_diff(left.last_y);
            let right_distance =
                attacker.x.abs_diff(right.last_x) + attacker.y.abs_diff(right.last_y);
            let left_facing = self
                .characters
                .get(&left.target_id)
                .is_some_and(|target| character_is_facing(&attacker, target));
            let right_facing = self
                .characters
                .get(&right.target_id)
                .is_some_and(|target| character_is_facing(&attacker, target));

            right
                .visible
                .cmp(&left.visible)
                .then_with(|| {
                    simple_baddy_enemy_hurtme(right).cmp(&simple_baddy_enemy_hurtme(left))
                })
                .then_with(|| left_distance.cmp(&right_distance))
                .then_with(|| right_facing.cmp(&left_facing))
        });
        enemies.truncate(10);
        if let Some(data) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.fight_driver.as_mut())
        {
            data.enemies = enemies;
        }
    }

    pub(crate) fn simple_baddy_enemy_past_stop_dist(
        &self,
        character: &Character,
        target: &Character,
    ) -> bool {
        let Some(data) = character.fight_driver.as_ref() else {
            return false;
        };
        data.stop_dist != 0
            && self.simple_baddy_target_home_dist(character, target) > data.stop_dist
    }

    pub fn set_simple_baddy_home(&mut self, character_id: CharacterId, x: u16, y: u16) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if !matches!(
            character.driver_state.as_ref(),
            Some(CharacterDriverState::SimpleBaddy(_))
        ) {
            return false;
        }
        let data = character
            .fight_driver
            .get_or_insert_with(FightDriverData::default);
        data.home_x = x;
        data.home_y = y;
        true
    }

    pub(crate) fn remove_simple_baddy_enemy(
        &mut self,
        character_id: CharacterId,
        target_id: CharacterId,
    ) {
        if let Some(character) = self.characters.get_mut(&character_id) {
            let _ = remove_simple_baddy_enemy_state(character, target_id);
        }
    }

    pub(crate) fn simple_baddy_can_poison_hit(
        &self,
        attacker_id: CharacterId,
        target_id: CharacterId,
    ) -> bool {
        let Some(attacker) = self.characters.get(&attacker_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        can_attack(attacker, target, &self.map)
    }

    pub(crate) fn simple_baddy_can_bless_friend(
        &self,
        caster_id: CharacterId,
        target_id: CharacterId,
    ) -> bool {
        let Some(caster) = self.characters.get(&caster_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        caster_id != target_id
            && caster.group == target.group
            && character_value(caster, CharacterValue::Bless) > 0
            && caster.mana >= BLESS_COST
            && !target.flags.contains(CharacterFlags::DEAD)
            && char_see_char(caster, target, &self.map, self.date.daylight)
            && may_add_spell(target, &self.items, IDR_BLESS, self.tick.0 as u32).is_some()
    }

    pub(crate) fn simple_baddy_can_add_standard_enemy(
        &self,
        character_id: CharacterId,
        target_id: CharacterId,
        require_visible: bool,
        hurtme: bool,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        if target.x < 1
            || target.y < 1
            || target.x as usize >= MAX_MAP
            || target.y as usize >= MAX_MAP
        {
            return false;
        }
        if character.group == target.group || !can_attack(character, target, &self.map) {
            return false;
        }
        if !hurtme
            && self
                .map
                .tile(usize::from(target.x), usize::from(target.y))
                .is_some_and(|tile| tile.flags.contains(MapFlags::NEUTRAL))
        {
            return false;
        }
        (!require_visible || char_see_char(character, target, &self.map, self.date.daylight))
            && (hurtme || self.simple_baddy_enemy_within_start_limits(character, target))
    }

    pub(crate) fn simple_baddy_enemy_within_start_limits(
        &self,
        character: &Character,
        target: &Character,
    ) -> bool {
        let Some(data) = character.fight_driver.as_ref() else {
            return false;
        };
        if data.start_dist != 0
            && self.simple_baddy_target_home_dist(character, target) > data.start_dist
        {
            return false;
        }
        if data.char_dist != 0 && char_dist(character, target) > data.char_dist {
            return false;
        }
        true
    }

    pub(crate) fn simple_baddy_target_home_dist(
        &self,
        character: &Character,
        target: &Character,
    ) -> i32 {
        let (home_x, home_y) = match character.fight_driver.as_ref() {
            Some(data) if data.home_x != 0 => (data.home_x, data.home_y),
            _ if character.rest_x != 0 => (character.rest_x, character.rest_y),
            _ => (character.x, character.y),
        };
        map_dist(home_x, home_y, target.x, target.y)
    }

    pub(crate) fn simple_baddy_seen_hit_enemy(
        &self,
        character_id: CharacterId,
        attacker_id: CharacterId,
        victim_id: CharacterId,
    ) -> Option<(CharacterId, bool)> {
        let character = self.characters.get(&character_id)?;
        let attacker = self.characters.get(&attacker_id)?;
        let victim = self.characters.get(&victim_id)?;

        if victim.id != character.id
            && victim.group == character.group
            && self.simple_baddy_can_add_standard_enemy(character_id, attacker_id, true, true)
        {
            return Some((attacker_id, true));
        }

        if attacker.id != character.id
            && attacker.group == character.group
            && self.simple_baddy_can_add_standard_enemy(character_id, victim_id, true, false)
        {
            return Some((victim_id, false));
        }

        None
    }
}
