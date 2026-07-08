use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum FightDriverTaskKind {
    Freeze,
    Fireball,
    Ball,
    Flash,
    Warcry,
    Attack,
    MoveRight,
    MoveLeft,
    MoveUp,
    MoveDown,
    Regenerate,
    Distance3,
    Distance7,
    Bless,
    EarthRain,
    EarthMud,
    Heal,
    MagicShield,
    Pulse,
    AttackBack,
    Flee,
    FireRing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct FightDriverTask {
    pub(crate) kind: FightDriverTaskKind,
    pub(crate) value: i32,
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const FIGHT_DRIVER_LOW_PRIO: i32 = 1;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const FIGHT_DRIVER_MED_PRIO: i32 = 500;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const FIGHT_DRIVER_HIGH_PRIO: i32 = 750;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn order_fight_driver_tasks(
    tasks: &mut [FightDriverTask],
    character_level: i32,
    mut random_below: impl FnMut(i32) -> i32,
) {
    let silliness = character_level / 2 + 5;
    if silliness > 1 {
        for task in tasks.iter_mut() {
            task.value += random_below(silliness).clamp(0, silliness - 1);
        }
    }
    tasks.sort_by(|left, right| right.value.cmp(&left.value));
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn fight_driver_attackback_may_run(tasks: &[FightDriverTask], index: usize) -> bool {
    tasks
        .get(index + 1)
        .is_some_and(|task| task.kind == FightDriverTaskKind::Attack)
}

/// C `fight_driver_attack_enemy`'s 10 positional `no*` suppression
/// parameters (`src/system/drvlib.c:1682`). The NPC-side `CDR_SIMPLEBADDY`/
/// `CDR_DUNGEONFIGHTER` callers always pass all-zero (see
/// `fight_driver_attack_visible`'s `else` branch that calls
/// `fight_driver_attack_enemy(cn, ..., 0,0,0,0,0,0,0,0,0,0)` when the
/// attacker has no `lostcon_ppd`), which `Default`/`From<bool>` (mapping a
/// bare `nomove` bool, matching every existing call site before this type
/// existed) both reproduce. The player-side caller (`ppd->nobless`, etc.)
/// is `process_lostcon_attack_action_with_random`'s `suppressions`
/// argument, built by `ugaris-server` from the lingering `PlayerRuntime`'s
/// `no*` toggles - public so that crate can construct one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FightDriverSuppressions {
    pub nomove: bool,
    pub nobless: bool,
    pub noheal: bool,
    pub noflash: bool,
    pub nofireball: bool,
    pub noball: bool,
    pub noshield: bool,
    pub nowarcry: bool,
    pub nofreeze: bool,
    pub nopulse: bool,
}

impl From<bool> for FightDriverSuppressions {
    /// Every pre-existing call site only ever suppressed movement
    /// (`nomove`); keep those call sites compiling unchanged.
    fn from(nomove: bool) -> Self {
        Self {
            nomove,
            ..Self::default()
        }
    }
}

impl World {
    pub fn process_simple_baddy_attack_action(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> bool {
        let mut seed = self.legacy_random_seed;
        let processed =
            self.process_simple_baddy_attack_action_with_random(character_id, area_id, |below| {
                legacy_random_below_from_seed(&mut seed, below)
            });
        self.legacy_random_seed = seed;
        processed
    }

    pub fn process_simple_baddy_attack_action_with_random(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        mut random: impl FnMut(u32) -> u32,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        // C: `dungeonfighter`'s own tail `char_driver(CDR_SIMPLEBADDY,
        // CDT_DRIVER, cn, ret, lastact)` call (`dungeon.c:2161`) reuses this
        // exact attack logic for `CDR_DUNGEONFIGHTER` guard NPCs too - see
        // `Character::dungeonfighter`'s doc comment. `CDR_PENTER` pentagram
        // demons (`pents.c::demon_character_driver`) do the same tail call
        // (`char_driver(CDR_SIMPLEBADDY, ...)`), same precedent.
        if (attacker.driver != CDR_SIMPLEBADDY
            && attacker.driver != CDR_DUNGEONFIGHTER
            && attacker.driver != CDR_PENTER)
            || attacker.action != 0
            || attacker.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }

        self.fight_driver_attack_visible_and_follow(
            character_id,
            &attacker,
            area_id,
            FightDriverSuppressions::default(),
            &mut random,
        )
    }

    /// C `fight_driver_attack_visible`+`fight_driver_follow_invisible`
    /// (`src/system/drvlib.c:2222-2320`), player-side wiring: the
    /// `CDR_LOSTCON` self-defense driver (`lostcon_driver`'s own
    /// `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
    /// ppd->nomove)) return; if (!ppd->nomove &&
    /// fight_driver_follow_invisible(cn)) return;` cascade,
    /// `lostcon.c:200-203`) calls this with the lingering `PlayerRuntime`'s
    /// `no*` toggles converted to `suppressions`. Returns `true` if an
    /// action was queued (caller should not also run its idle fallback).
    pub fn process_lostcon_attack_action_with_random(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        suppressions: FightDriverSuppressions,
        mut random: impl FnMut(u32) -> u32,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if attacker.driver != CDR_LOSTCON
            || attacker.action != 0
            || attacker.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }

        self.fight_driver_attack_visible_and_follow(
            character_id,
            &attacker,
            area_id,
            suppressions,
            &mut random,
        )
    }

    /// Shared body of `fight_driver_attack_visible`+
    /// `fight_driver_follow_invisible` (`src/system/drvlib.c:2222-2320`):
    /// score/attempt every visible enemy in score order (highest `(999 -
    /// dist) * 10 [+5 if facing]` first), falling back to pathfinding
    /// toward the last known position of one invisible enemy when nothing
    /// visible could be attacked and `!suppressions.nomove` (C's `if
    /// (!ppd->nomove && fight_driver_follow_invisible(cn))` gate - the
    /// always-all-`false`-suppressions NPC caller never sets `nomove`, so
    /// this preserves its behavior unchanged).
    pub(crate) fn fight_driver_attack_visible_and_follow(
        &mut self,
        character_id: CharacterId,
        attacker: &Character,
        area_id: u16,
        suppressions: FightDriverSuppressions,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let enemies = self.refresh_simple_baddy_enemy_tracking(attacker);
        if enemies.is_empty() {
            return false;
        }
        let mut visible_enemies: Vec<_> = enemies
            .iter()
            .filter(|enemy| enemy.visible)
            .copied()
            .collect();
        visible_enemies.sort_by(|left, right| {
            self.simple_baddy_visible_enemy_score(attacker, right)
                .cmp(&self.simple_baddy_visible_enemy_score(attacker, left))
        });

        for enemy in visible_enemies {
            let previous_lastfight = self
                .simple_baddy_lastfight(character_id)
                .unwrap_or_default();
            let Some(target) = self.characters.get(&enemy.target_id).cloned() else {
                continue;
            };
            if !can_attack_in_area(attacker, &target, &self.map, area_id) {
                continue;
            }
            if self.setup_weighted_fight_task(character_id, &target, area_id, suppressions, random)
            {
                self.queue_simple_baddy_attack_sound(character_id, previous_lastfight);
                return true;
            }
        }

        if suppressions.nomove {
            return false;
        }

        for enemy in enemies.into_iter().filter(|enemy| !enemy.visible) {
            if attacker.x.abs_diff(enemy.last_x) < 2 && attacker.y.abs_diff(enemy.last_y) < 2 {
                self.remove_simple_baddy_enemy(character_id, enemy.target_id);
                continue;
            }
            if self.setup_walk_toward(
                character_id,
                usize::from(enemy.last_x),
                usize::from(enemy.last_y),
                0,
                area_id,
                false,
            ) || self.setup_walk_toward(
                character_id,
                usize::from(enemy.last_x),
                usize::from(enemy.last_y),
                0,
                area_id,
                true,
            ) {
                if let Some(character) = self.characters.get_mut(&character_id) {
                    if let Some(CharacterDriverState::SimpleBaddy(data)) =
                        character.driver_state.as_mut()
                    {
                        data.lastfight = self.tick.0 as i32;
                    }
                }
                return true;
            }
            self.remove_simple_baddy_enemy(character_id, enemy.target_id);
        }

        false
    }

    pub(crate) fn simple_baddy_visible_enemy_score(
        &self,
        attacker: &Character,
        enemy: &SimpleBaddyEnemy,
    ) -> i32 {
        let Some(target) = self.characters.get(&enemy.target_id) else {
            return i32::MIN;
        };
        let mut score = (999 - char_dist(attacker, target)) * 10;
        if character_is_facing(attacker, target) {
            score += 5;
        }
        score
    }

    pub(crate) fn setup_simple_baddy_attack_move(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let from_x = usize::from(attacker.x);
        let from_y = usize::from(attacker.y);
        let target_x = usize::from(target.x);
        let target_y = usize::from(target.y);
        let direct = pathfinder(&self.map, from_x, from_y, target_x, target_y, 1, None);
        let moving = (target.tox != 0).then(|| {
            pathfinder(
                &self.map,
                from_x,
                from_y,
                usize::from(target.tox),
                usize::from(target.toy),
                1,
                None,
            )
        });

        let best_partial = moving.unwrap_or(direct);
        let direction = match (direct.direction, moving) {
            (Some(_direct_direction), Some(moving_result))
                if moving_result.direction.is_some() && direct.cost >= moving_result.cost =>
            {
                moving_result.direction.expect("checked above")
            }
            (Some(direct_direction), _) => direct_direction,
            (None, Some(moving_result)) if moving_result.direction.is_some() => {
                moving_result.direction.expect("checked above")
            }
            (None, _) => {
                let current_distance = manhattan_distance(from_x, from_y, target_x, target_y);
                if best_partial.best_direction.is_some()
                    && best_partial.best_distance < current_distance
                {
                    best_partial.best_direction.unwrap()
                } else if self.setup_adjacent_use_toward_target(character_id, target_x, target_y) {
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        if let Some(CharacterDriverState::SimpleBaddy(data)) =
                            character.driver_state.as_mut()
                        {
                            data.lastfight = self.tick.0 as i32;
                        }
                    }
                    return true;
                } else {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        return false;
                    };
                    return do_idle(character, (TICKS_PER_SECOND / 4) as i32).is_ok();
                }
            }
        };

        if !self.walk_or_use_driver(character_id, direction, area_id) {
            if !self.setup_adjacent_use_toward_target(character_id, target_x, target_y) {
                return false;
            }
            if let Some(attacker_mut) = self.characters.get_mut(&character_id) {
                if let Some(CharacterDriverState::SimpleBaddy(data)) =
                    attacker_mut.driver_state.as_mut()
                {
                    data.lastfight = self.tick.0 as i32;
                }
            }
            return true;
        }
        if let Some(attacker_mut) = self.characters.get_mut(&character_id) {
            if let Some(CharacterDriverState::SimpleBaddy(data)) =
                attacker_mut.driver_state.as_mut()
            {
                data.lastfight = self.tick.0 as i32;
            }
        }
        true
    }

    pub(crate) fn setup_adjacent_use_toward_target(
        &mut self,
        character_id: CharacterId,
        target_x: usize,
        target_y: usize,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let from_x = usize::from(character.x);
        let from_y = usize::from(character.y);
        let current_distance = manhattan_distance(from_x, from_y, target_x, target_y);

        let mut best: Option<(Direction, ItemId, usize)> = None;
        for direction in [
            Direction::Right,
            Direction::Left,
            Direction::Down,
            Direction::Up,
        ] {
            let (dx, dy) = direction.delta();
            let Some(x) = offset_coordinate(from_x, dx) else {
                continue;
            };
            let Some(y) = offset_coordinate(from_y, dy) else {
                continue;
            };
            let Some(tile) = self.map.tile(x, y) else {
                continue;
            };
            if !tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            {
                continue;
            }
            let item_id = ItemId(tile.item);
            let Some(item) = (tile.item != 0).then(|| self.items.get(&item_id)).flatten() else {
                continue;
            };
            if !item.flags.contains(ItemFlags::USE) {
                continue;
            }
            let distance = manhattan_distance(x, y, target_x, target_y);
            if distance >= current_distance {
                continue;
            }
            if best.is_none_or(|(_, _, best_distance)| distance < best_distance) {
                best = Some((direction, item_id, distance));
            }
        }

        let Some((direction, item_id, _)) = best else {
            return false;
        };
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        do_use(
            character,
            &self.map,
            item,
            direction as u8,
            0,
            self.settings.weather_movement_percent,
        )
        .is_ok()
    }

    pub(crate) fn setup_simple_baddy_attack_driver(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };

        let direction = adjacent_direction(
            attacker.x,
            attacker.y,
            usize::from(target.x),
            usize::from(target.y),
        )
        .or_else(|| {
            (target.tox != 0).then(|| {
                adjacent_direction(
                    attacker.x,
                    attacker.y,
                    usize::from(target.tox),
                    usize::from(target.toy),
                )
            })?
        });
        let Some(direction) = direction else {
            return false;
        };
        let Some(attacker_mut) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_attack(
            attacker_mut,
            &self.map,
            target,
            direction as u8,
            action::ATTACK1,
            self.settings.weather_movement_percent,
        )
        .is_err()
        {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = attacker_mut.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub fn attack_driver_direct(
        &mut self,
        character_id: CharacterId,
        target_id: CharacterId,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        if attacker.id == target.id
            || !char_see_char(&attacker, &target, &self.map, self.date.daylight)
            || !can_attack_in_area(&attacker, &target, &self.map, area_id)
        {
            return false;
        }

        if let Some(direction) = adjacent_direction(
            attacker.x,
            attacker.y,
            usize::from(target.x),
            usize::from(target.y),
        )
        .or_else(|| {
            (target.tox != 0).then(|| {
                adjacent_direction(
                    attacker.x,
                    attacker.y,
                    usize::from(target.tox),
                    usize::from(target.toy),
                )
            })?
        }) {
            let Some(attacker_mut) = self.characters.get_mut(&character_id) else {
                return false;
            };
            return do_attack(
                attacker_mut,
                &self.map,
                &target,
                direction as u8,
                action::ATTACK1,
                self.settings.weather_movement_percent,
            )
            .is_ok();
        }

        let path = pathfinder(
            &self.map,
            usize::from(attacker.x),
            usize::from(attacker.y),
            usize::from(target.x),
            usize::from(target.y),
            1,
            None,
        );
        let Some(direction) = path.direction else {
            return false;
        };
        self.walk_or_use_driver(character_id, direction, area_id)
    }

    pub(crate) fn simple_baddy_lastfight(&self, character_id: CharacterId) -> Option<i32> {
        let character = self.characters.get(&character_id)?;
        match character.driver_state.as_ref()? {
            CharacterDriverState::SimpleBaddy(data) => Some(data.lastfight),
            CharacterDriverState::Clara(_)
            | CharacterDriverState::TwoSkelly(_)
            | CharacterDriverState::Lab2Undead(_)
            | CharacterDriverState::Merchant(_)
            | CharacterDriverState::Aclerk(_)
            | CharacterDriverState::Lostcon(_)
            | CharacterDriverState::Bank(_)
            | CharacterDriverState::Trader(_)
            | CharacterDriverState::Janitor(_)
            | CharacterDriverState::GateWelcome(_)
            | CharacterDriverState::GateFight(_)
            | CharacterDriverState::Clanmaster(_)
            | CharacterDriverState::ClanFound(_)
            | CharacterDriverState::Clanclerk(_)
            | CharacterDriverState::Clubmaster(_)
            | CharacterDriverState::MilitaryMaster(_)
            | CharacterDriverState::MilitaryAdvisor(_)
            | CharacterDriverState::ArenaMaster(_)
            | CharacterDriverState::ArenaFighter(_)
            | CharacterDriverState::ArenaManager(_)
            | CharacterDriverState::Dungeonmaster(_)
            | CharacterDriverState::Dungeonfighter(_)
            | CharacterDriverState::Macro(_)
            | CharacterDriverState::Camhermit(_)
            | CharacterDriverState::Yoakin(_)
            | CharacterDriverState::Terion(_)
            | CharacterDriverState::Gwendylon(_)
            | CharacterDriverState::Greeter(_)
            | CharacterDriverState::Jessica(_)
            | CharacterDriverState::Jiu(_)
            | CharacterDriverState::ForestRanger(_)
            | CharacterDriverState::Brithildie(_)
            | CharacterDriverState::Nook(_)
            | CharacterDriverState::Lydia(_)
            | CharacterDriverState::Robber(_)
            | CharacterDriverState::Sanoa(_)
            | CharacterDriverState::Asturin(_)
            | CharacterDriverState::Reskin(_)
            | CharacterDriverState::Guiwynn(_)
            | CharacterDriverState::James(_)
            | CharacterDriverState::Balltrap(_)
            | CharacterDriverState::Logain(_)
            | CharacterDriverState::Superior(_)
            | CharacterDriverState::Moonie(_)
            | CharacterDriverState::Vampire(_)
            | CharacterDriverState::Vampire2(_)
            | CharacterDriverState::Astro1(_)
            | CharacterDriverState::Astro2(_)
            | CharacterDriverState::Thomas(_)
            | CharacterDriverState::SirJones(_)
            | CharacterDriverState::Seymour(_)
            | CharacterDriverState::Kelly(_)
            | CharacterDriverState::Lampghost(_)
            | CharacterDriverState::Carlos(_)
            | CharacterDriverState::Kassim(_)
            | CharacterDriverState::Supermax(_)
            | CharacterDriverState::Tester(_)
            | CharacterDriverState::Engrave(_)
            | CharacterDriverState::FdemonArmy(_)
            | CharacterDriverState::Islena(_)
            | CharacterDriverState::PalaceGuard(_) => None,
        }
    }

    pub(crate) fn queue_simple_baddy_attack_sound(
        &mut self,
        character_id: CharacterId,
        previous_lastfight: i32,
    ) {
        let current_tick = self.tick.0 as i32;
        if current_tick - previous_lastfight <= (TICKS_PER_SECOND * 10) as i32 {
            return;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        self.queue_sound_area(usize::from(character.x), usize::from(character.y), 1);
    }

    pub(crate) fn setup_simple_baddy_earthmud_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let max_hp = character_value(&attacker, CharacterValue::Hp) * POWERSCALE;
        let strength = character_value_present(&attacker, CharacterValue::Demon);
        if !attacker.flags.contains(CharacterFlags::EDEMON)
            || strength != 30
            || attacker.hp < max_hp / 2
            || self.simple_baddy_earthmud_value(target) == 0
        {
            return false;
        }

        let (target_x, target_y) = simple_baddy_earth_spell_target(target);
        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_earthmud(
            character,
            &self.map,
            target_x,
            target_y,
            strength,
            weather_movement_percent,
        )
        .is_err()
        {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub(crate) fn simple_baddy_earthmud_value(&self, target: &Character) -> i32 {
        let (target_x, target_y) = simple_baddy_earth_spell_target(target);
        let mut good = 0;
        for (x, y) in [
            (target_x, target_y),
            (target_x.saturating_add(1), target_y),
            (target_x.saturating_sub(1), target_y),
            (target_x, target_y.saturating_add(1)),
            (target_x, target_y.saturating_sub(1)),
        ] {
            if self.simple_baddy_can_place_earthmud(x, y) {
                good += 1;
            }
        }

        if good > 0 {
            good
        } else {
            0
        }
    }

    pub(crate) fn simple_baddy_can_place_earthmud(&self, x: usize, y: usize) -> bool {
        self.map.tile(x, y).is_some_and(|tile| {
            !tile
                .flags
                .intersects(MapFlags::SIGHTBLOCK | MapFlags::TSIGHTBLOCK)
                && tile.effects.iter().all(|&effect_id| {
                    effect_id == 0
                        || self
                            .effects
                            .get(&u32::from(effect_id))
                            .is_none_or(|effect| effect.effect_type != EF_EARTHMUD)
                })
        })
    }

    pub(crate) fn simple_baddy_can_heal_self(&self, character: &Character) -> bool {
        character_value(character, CharacterValue::Heal) > 1
            && character.mana >= POWERSCALE * 2
            && character.hp < character_value(character, CharacterValue::Hp) * POWERSCALE / 2
    }

    pub(crate) fn setup_simple_baddy_heal_action(&mut self, character_id: CharacterId) -> bool {
        let Some(target) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_heal_self(&target) {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, _items, _tick, map, weather_movement_percent| {
                do_heal(character, &target, None, map, weather_movement_percent)
            },
        )
    }

    pub(crate) fn simple_baddy_can_magicshield_self(&self, character: &Character) -> bool {
        character_value(character, CharacterValue::MagicShield) > 1
            && character.mana >= POWERSCALE * 2
            && character.lifeshield
                < character_value(character, CharacterValue::MagicShield) * POWERSCALE / 2
    }

    pub(crate) fn setup_simple_baddy_magicshield_action(
        &mut self,
        character_id: CharacterId,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_magicshield_self(&character) {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, _items, _tick, map, weather_movement_percent| {
                do_magicshield(character, map, weather_movement_percent)
            },
        )
    }

    pub(crate) fn simple_baddy_can_bless_self(&self, character: &Character) -> bool {
        character_value(character, CharacterValue::Bless) > 1
            && character.mana >= BLESS_COST
            && may_add_spell(character, &self.items, IDR_BLESS, self.tick.0 as u32).is_some()
    }

    pub(crate) fn setup_simple_baddy_self_bless_action(
        &mut self,
        character_id: CharacterId,
    ) -> bool {
        let Some(target) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_bless_self(&target) {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, items, tick, map, weather_movement_percent| {
                do_bless(
                    character,
                    &target,
                    items,
                    tick,
                    None,
                    map,
                    weather_movement_percent,
                )
            },
        )
    }

    pub(crate) fn simple_baddy_needs_regeneration(&self, character: &Character) -> bool {
        character.mana < character_value(character, CharacterValue::Mana) * POWERSCALE
            || character.hp < character_value(character, CharacterValue::Hp) * POWERSCALE
    }

    pub(crate) fn setup_simple_baddy_regenerate_action(
        &mut self,
        character_id: CharacterId,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_needs_regeneration(&character) {
            return false;
        }
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_idle(character, (TICKS_PER_SECOND / 2) as i32).is_err() {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub(crate) fn simple_baddy_regenerate_task_value(&self, character: &Character) -> i32 {
        let base = character_value(character, CharacterValue::Fireball)
            .max(character_value(character, CharacterValue::Flash))
            .max(character_value(character, CharacterValue::Freeze))
            .max(character_value(character, CharacterValue::Attack))
            * 2;
        let last_hit = character
            .fight_driver
            .as_ref()
            .map(|data| data.last_hit)
            .unwrap_or(0);
        let tick = self.tick.0 as i32;
        let regen_time = TICKS_PER_SECOND as i32;
        let regen_diff = character.regen_ticker as i32 + regen_time - tick;
        if regen_diff <= 0 {
            return base + FIGHT_DRIVER_HIGH_PRIO;
        }
        let hit_diff = last_hit + regen_time * 2 - tick;
        if hit_diff <= 0 {
            return base + FIGHT_DRIVER_LOW_PRIO;
        }
        (base * regen_time * 2 - base * hit_diff) / (regen_time * 2) + FIGHT_DRIVER_LOW_PRIO
    }

    pub(crate) fn simple_baddy_freeze_modifier(
        &self,
        attacker: &Character,
        target: &Character,
    ) -> i32 {
        freeze_speed_modifier(
            spell_power(
                character_value(attacker, CharacterValue::Freeze),
                character_value(attacker, CharacterValue::Tactics),
            ),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            character_value_present(target, CharacterValue::Tactics) != 0,
            attacker.flags.contains(CharacterFlags::IDEMON),
            // C: freeze_value (tool.c) reads the caster's V_DEMON from value[1]
            // (the base/present value, not the sunlight/combat-reducible current
            // value[0]).
            character_value_present(attacker, CharacterValue::Demon),
            character_value(target, CharacterValue::Cold),
        )
    }

    pub(crate) fn setup_simple_baddy_freeze_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character_value(&attacker, CharacterValue::Freeze) <= 1
            || attacker.mana < FREEZE_COST
            || tile_char_dist(&attacker, target) >= 4
            || may_add_spell(target, &self.items, IDR_FREEZE, self.tick.0 as u32).is_none()
            || self.simple_baddy_freeze_modifier(&attacker, target) >= -10
        {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, _items, _tick, map, weather_movement_percent| {
                do_freeze(character, map, weather_movement_percent)
            },
        )
    }

    pub(crate) fn setup_simple_baddy_ball_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let target_x = usize::from(target.x).saturating_sub(1)
            + usize::try_from(random(3).min(2)).unwrap_or(0);
        let target_y = usize::from(target.y).saturating_sub(1)
            + usize::try_from(random(3).min(2)).unwrap_or(0);
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, items, tick, map, weather_movement_percent| {
                do_ball(
                    character,
                    items,
                    target_x,
                    target_y,
                    tick,
                    map,
                    weather_movement_percent,
                )
            },
        )
    }

    pub(crate) fn simple_baddy_calc_ball_steps(
        &self,
        caster_id: CharacterId,
        from_x: usize,
        from_y: usize,
        target_x: usize,
        target_y: usize,
    ) -> i32 {
        let mut dx = target_x as i32 - from_x as i32;
        let mut dy = target_y as i32 - from_y as i32;
        if dx == 0 && dy == 0 {
            return 0;
        }

        let mut x = from_x as i32 * 1024 + 512;
        let mut y = from_y as i32 * 1024 + 512;
        if dx.abs() > dy.abs() {
            dy = dy * 512 / dx.abs();
            dx = dx * 512 / dx.abs();
        } else {
            dx = dx * 512 / dy.abs();
            dy = dy * 512 / dy.abs();
        }

        let max_steps = (TICKS_PER_SECOND * 5 / 4) as i32;
        for step in 0..max_steps {
            x += dx;
            y += dy;
            let tile_x = x / 1024;
            let tile_y = y / 1024;
            if self.ball_path_blocked_for_caster(tile_x, tile_y, caster_id) {
                return step;
            }
        }
        max_steps
    }

    pub(crate) fn ball_path_blocked_for_caster(
        &self,
        x: i32,
        y: i32,
        caster_id: CharacterId,
    ) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return true;
        };
        let Some(tile) = self.map.tile(x, y) else {
            return true;
        };
        let map_blocks = tile.flags.contains(MapFlags::TMOVEBLOCK)
            || (!tile.flags.contains(MapFlags::FIRETHRU)
                && tile.flags.contains(MapFlags::MOVEBLOCK));
        map_blocks && tile.character != caster_id.0 as u16
    }

    pub(crate) fn setup_simple_baddy_flash_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if tile_char_dist(&attacker, target) >= 4
            || may_add_spell(&attacker, &self.items, IDR_FLASH, self.tick.0 as u32).is_none()
        {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, items, tick, map, weather_movement_percent| {
                do_flash(character, items, tick, map, weather_movement_percent)
            },
        )
    }

    pub(crate) fn simple_baddy_can_warcry(&self, attacker: &Character, target: &Character) -> bool {
        if character_value(attacker, CharacterValue::Warcry) <= 1
            || attacker.endurance
                <= character_value(attacker, CharacterValue::Warcry) * POWERSCALE / 3
            || char_dist(attacker, target) >= 8
        {
            return false;
        }
        let target_accepts =
            may_add_spell(target, &self.items, IDR_WARCRY, self.tick.0 as u32).is_some();
        let caster_needs_shield = character_value_present(attacker, CharacterValue::MagicShield)
            == 0
            && attacker.lifeshield
                < character_value(attacker, CharacterValue::Warcry) * POWERSCALE / 4;
        target_accepts || caster_needs_shield
    }

    pub(crate) fn setup_simple_baddy_warcry_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_warcry(&attacker, target) {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, items, _tick, map, weather_movement_percent| {
                do_warcry(character, items, map, weather_movement_percent)
            },
        )
    }

    #[allow(dead_code)]
    pub(crate) fn setup_simple_baddy_distance_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let current_tick = self.tick.0 as u32;
        let target_has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let tile_distance = tile_char_dist(&attacker, target);

        let freeze_spacing = character_value_present(&attacker, CharacterValue::Freeze) != 0
            && attacker.mana > POWERSCALE * 3
            && tile_distance > 3
            && may_add_spell(target, &self.items, IDR_FREEZE, current_tick).is_some()
            && freeze_speed_modifier(
                spell_power(
                    character_value(&attacker, CharacterValue::Freeze),
                    character_value(&attacker, CharacterValue::Tactics),
                ),
                character_value(target, CharacterValue::Immunity),
                character_value(target, CharacterValue::Tactics),
                target_has_tactics,
                attacker.flags.contains(CharacterFlags::IDEMON),
                // C: freeze_value (tool.c) reads the caster's V_DEMON from value[1].
                character_value_present(&attacker, CharacterValue::Demon),
                character_value(target, CharacterValue::Cold),
            ) < -10;
        let flash_spacing = character_value_present(&attacker, CharacterValue::Flash) != 0
            && attacker.mana > POWERSCALE * 3
            && may_add_spell(&attacker, &self.items, IDR_FLASH, current_tick).is_none();

        if !freeze_spacing && !flash_spacing {
            return false;
        }

        self.setup_simple_baddy_distance_driver(character_id, target, 3, area_id, true)
    }

    #[allow(dead_code)]
    pub(crate) fn setup_simple_baddy_fireball_distance_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if attacker.mana <= FIREBALL_COST
            || character_value_present(&attacker, CharacterValue::Fireball) == 0
            || character_value_present(&attacker, CharacterValue::Fireball)
                <= character_value_present(&attacker, CharacterValue::Flash)
            || may_add_spell(&attacker, &self.items, IDR_FLASH, self.tick.0 as u32).is_none()
        {
            return false;
        }

        let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let damage = fireball_damage(
            character_value(&attacker, CharacterValue::Fireball),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            has_tactics,
        );
        if damage < POWERSCALE {
            return false;
        }

        self.setup_simple_baddy_distance_driver(character_id, target, 7, area_id, false)
    }

    pub(crate) fn setup_simple_baddy_distance_driver(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        distance: u16,
        area_id: u16,
        idle_when_already_there: bool,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if step_char_dist(&attacker, target) == distance {
            if !idle_when_already_there {
                return false;
            }
            let Some(character) = self.characters.get_mut(&character_id) else {
                return false;
            };
            if do_idle(character, (TICKS_PER_SECOND / 4) as i32).is_err() {
                return false;
            }
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
                data.lastfight = self.tick.0 as i32;
            }
            return true;
        }

        let target_positions = if target.tox != 0 {
            [
                (usize::from(target.tox), usize::from(target.toy)),
                (usize::from(target.x), usize::from(target.y)),
            ]
        } else {
            [
                (usize::from(target.x), usize::from(target.y)),
                (usize::from(target.x), usize::from(target.y)),
            ]
        };
        for (target_x, target_y) in target_positions {
            if self.setup_walk_toward(
                character_id,
                target_x,
                target_y,
                usize::from(distance),
                area_id,
                false,
            ) {
                if let Some(character) = self.characters.get_mut(&character_id) {
                    if let Some(CharacterDriverState::SimpleBaddy(data)) =
                        character.driver_state.as_mut()
                    {
                        data.lastfight = self.tick.0 as i32;
                    }
                }
                return true;
            }
        }

        let target_x = usize::from(target.x);
        let target_y = usize::from(target.y);
        let partial = pathfinder(
            &self.map,
            usize::from(attacker.x),
            usize::from(attacker.y),
            target_x,
            target_y,
            usize::from(distance),
            None,
        );
        if let Some(direction) = partial.best_direction {
            if self.walk_or_use_driver(character_id, direction, area_id) {
                if let Some(character) = self.characters.get_mut(&character_id) {
                    if let Some(CharacterDriverState::SimpleBaddy(data)) =
                        character.driver_state.as_mut()
                    {
                        data.lastfight = self.tick.0 as i32;
                    }
                }
                return true;
            }
        }

        false
    }

    pub fn distance_driver(
        &mut self,
        character_id: CharacterId,
        target_id: CharacterId,
        distance: u16,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        if attacker.id == target.id
            || !char_see_char(&attacker, &target, &self.map, self.date.daylight)
        {
            return false;
        }
        if step_char_dist(&attacker, &target) == distance {
            return false;
        }

        if target.tox != 0
            && self.setup_walk_toward(
                character_id,
                usize::from(target.tox),
                usize::from(target.toy),
                usize::from(distance),
                area_id,
                false,
            )
        {
            return true;
        }
        if self.setup_walk_toward(
            character_id,
            usize::from(target.x),
            usize::from(target.y),
            usize::from(distance),
            area_id,
            false,
        ) {
            return true;
        }

        let partial = pathfinder(
            &self.map,
            usize::from(attacker.x),
            usize::from(attacker.y),
            usize::from(target.x),
            usize::from(target.y),
            usize::from(distance),
            None,
        );
        partial
            .best_direction
            .is_some_and(|direction| self.walk_or_use_driver(character_id, direction, area_id))
    }

    pub(crate) fn setup_simple_baddy_attack_back_move(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Ok(direction) = Direction::try_from(target.dir) else {
            return false;
        };
        let (dx, dy) = direction.delta();
        if dx != 0 && dy != 0 {
            return false;
        }

        let Some(back_x) = offset_coordinate(usize::from(target.x), -dx) else {
            return false;
        };
        let Some(back_y) = offset_coordinate(usize::from(target.y), -dy) else {
            return false;
        };
        if back_x < 1 || back_y < 1 || back_x >= MAX_MAP || back_y >= MAX_MAP {
            return false;
        }
        if self.map.blocks_movement(back_x, back_y) {
            return false;
        }

        let Some(front_x) = offset_coordinate(usize::from(target.x), dx) else {
            return false;
        };
        let Some(front_y) = offset_coordinate(usize::from(target.y), dy) else {
            return false;
        };
        if front_x < 1 || front_y < 1 || front_x >= MAX_MAP || front_y >= MAX_MAP {
            return false;
        }

        let front_occupied = self
            .map
            .tile(front_x, front_y)
            .is_some_and(|tile| tile.character != 0);
        if self.characters.get(&character_id).is_some_and(|attacker| {
            usize::from(attacker.x) == front_x && usize::from(attacker.y) == front_y
        }) {
            return false;
        }

        let Some(side_x) = offset_coordinate(usize::from(target.x), dy) else {
            return false;
        };
        let Some(side_y) = offset_coordinate(usize::from(target.y), dx) else {
            return false;
        };
        if side_x < 1 || side_y < 1 || side_x >= MAX_MAP || side_y >= MAX_MAP {
            return false;
        }
        let same_group_side_occupied = self
            .map
            .tile(side_x, side_y)
            .and_then(|tile| {
                (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
            })
            .and_then(|side_id| self.characters.get(&side_id))
            .is_some_and(|side_character| {
                side_character.id != character_id
                    && self
                        .characters
                        .get(&character_id)
                        .is_some_and(|attacker| side_character.group == attacker.group)
            });
        if same_group_side_occupied {
            return false;
        }

        let idle_target = target.action == action::IDLE
            && self.tick.0.saturating_sub(u64::from(target.regen_ticker)) > TICKS_PER_SECOND / 2;
        if !idle_target && !front_occupied {
            return false;
        }

        if !self.setup_walk_toward(character_id, back_x, back_y, 0, area_id, false) {
            return false;
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
                data.lastfight = self.tick.0 as i32;
            }
        }
        true
    }

    pub fn setup_simple_baddy_flee_action(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let enemies = self.refresh_simple_baddy_enemy_tracking(&attacker);
        let mut direction_scores = [0i32; 9];
        let mut min_distance = 99;

        for enemy in enemies.into_iter().filter(|enemy| enemy.visible) {
            let Some(target) = self.characters.get(&enemy.target_id) else {
                continue;
            };
            let distance = char_dist(&attacker, target);
            if distance > 30 {
                continue;
            }
            min_distance = min_distance.min(distance);
            let score = 5000 - distance * 50;
            let dx = i32::from(target.x) - i32::from(attacker.x);
            let dy = i32::from(target.y) - i32::from(attacker.y);
            let total_delta = dx.abs() + dy.abs();
            if total_delta == 0 {
                continue;
            }

            if dx > 0 {
                direction_scores[Direction::Right as usize] -= score * dx.abs() / total_delta;
                direction_scores[Direction::RightUp as usize] -= score * dx.abs() / total_delta / 2;
                direction_scores[Direction::RightDown as usize] -=
                    score * dx.abs() / total_delta / 2;
                direction_scores[Direction::Left as usize] += score * dx.abs() / total_delta / 4;
                direction_scores[Direction::LeftUp as usize] += score * dx.abs() / total_delta / 8;
                direction_scores[Direction::LeftDown as usize] +=
                    score * dx.abs() / total_delta / 8;
            }
            if dx < 0 {
                direction_scores[Direction::Left as usize] -= score * dx.abs() / total_delta;
                direction_scores[Direction::LeftUp as usize] -= score * dx.abs() / total_delta / 2;
                direction_scores[Direction::LeftDown as usize] -=
                    score * dx.abs() / total_delta / 2;
                direction_scores[Direction::Right as usize] -= score * dx.abs() / total_delta / 4;
                direction_scores[Direction::RightUp as usize] -= score * dx.abs() / total_delta / 8;
                direction_scores[Direction::RightDown as usize] -=
                    score * dx.abs() / total_delta / 8;
            }
            if dy > 0 {
                direction_scores[Direction::Down as usize] -= score * dy.abs() / total_delta;
                direction_scores[Direction::LeftDown as usize] -=
                    score * dy.abs() / total_delta / 2;
                direction_scores[Direction::RightDown as usize] -=
                    score * dy.abs() / total_delta / 2;
                direction_scores[Direction::Up as usize] -= score * dy.abs() / total_delta / 4;
                direction_scores[Direction::LeftUp as usize] -= score * dy.abs() / total_delta / 8;
                direction_scores[Direction::RightUp as usize] -= score * dy.abs() / total_delta / 8;
            }
            if dy < 0 {
                direction_scores[Direction::Up as usize] -= score * dy.abs() / total_delta;
                direction_scores[Direction::LeftUp as usize] -= score * dy.abs() / total_delta / 2;
                direction_scores[Direction::RightUp as usize] -= score * dy.abs() / total_delta / 2;
                direction_scores[Direction::Down as usize] -= score * dy.abs() / total_delta / 4;
                direction_scores[Direction::LeftDown as usize] -=
                    score * dy.abs() / total_delta / 8;
                direction_scores[Direction::RightDown as usize] -=
                    score * dy.abs() / total_delta / 8;
            }
        }
        if min_distance > 30 {
            return false;
        }

        if let Some(character) = self.characters.get_mut(&character_id) {
            if min_distance < 10
                && (character.endurance > 4 * POWERSCALE || character.speed_mode == SpeedMode::Fast)
            {
                character.speed_mode = SpeedMode::Fast;
            } else if min_distance < 10 {
                character.speed_mode = SpeedMode::Normal;
            } else {
                character.speed_mode = SpeedMode::Stealth;
            }
        }

        let mut best_direction = None;
        let mut best_score = i32::MIN;
        for direction_id in 1..=8 {
            let direction =
                Direction::try_from(direction_id as u8).expect("valid legacy direction");
            let score = direction_scores[direction_id]
                + self.simple_baddy_flee_eval_path(
                    usize::from(attacker.x),
                    usize::from(attacker.y),
                    direction,
                );
            if score > best_score {
                best_direction = Some(direction);
                best_score = score;
            }
        }
        let Some(direction) = best_direction else {
            return false;
        };
        if !self.setup_walk_direction(character_id, direction, area_id) {
            return false;
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
                data.lastfight = self.tick.0 as i32;
            }
        }
        true
    }

    pub(crate) fn simple_baddy_flee_eval_path(
        &self,
        x: usize,
        y: usize,
        direction: Direction,
    ) -> i32 {
        let (dx, dy) = direction.delta();
        let mut x = x;
        let mut y = y;
        let mut score = 0;

        for _ in 0..10 {
            let Some(next_x) = offset_coordinate(x, dx) else {
                return score;
            };
            let Some(next_y) = offset_coordinate(y, dy) else {
                return score;
            };
            x = next_x;
            y = next_y;
            if x < 1 || x >= MAX_MAP - 1 || y < 1 || y >= MAX_MAP - 1 {
                return score;
            }
            let Some(tile) = self.map.tile(x, y) else {
                return score;
            };
            if tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            {
                return score;
            }
            let daylight = (i32::from(tile.daylight) * self.date.daylight) / 256;
            score += 300 - i32::from(tile.light).max(daylight);
        }

        score
    }

    pub(crate) fn setup_simple_baddy_pulse_attack(&mut self, character_id: CharacterId) -> bool {
        if self.simple_baddy_pulse_value(character_id) == 0 {
            return false;
        }

        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_pulse(character, &self.map, weather_movement_percent).is_err() {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    /// C `fight_driver_attack_enemy`: score every applicable task, apply
    /// the level-based silliness rolls, sort, and run tasks in order until
    /// one succeeds. `suppressions` generalizes the `nomove`/`nobless`/...
    /// positional arguments so both the NPC driver (always all-`false`)
    /// and a future player/lostcon caller (`lostcon_ppd`-backed toggles)
    /// can share this engine.
    pub(crate) fn setup_weighted_fight_task(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
        suppressions: FightDriverSuppressions,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let mut tasks = self.simple_baddy_fight_tasks(character_id, target, area_id, suppressions);
        let level = self
            .characters
            .get(&character_id)
            .map(|character| character.level as i32)
            .unwrap_or_default();
        order_fight_driver_tasks(&mut tasks, level, |below| random(below as u32) as i32);

        for (index, task) in tasks.iter().copied().enumerate() {
            let ret = match task.kind {
                FightDriverTaskKind::Freeze => {
                    self.setup_simple_baddy_freeze_attack(character_id, target)
                }
                FightDriverTaskKind::Heal => self.setup_simple_baddy_heal_action(character_id),
                FightDriverTaskKind::MagicShield => {
                    self.setup_simple_baddy_magicshield_action(character_id)
                }
                FightDriverTaskKind::EarthMud => {
                    self.setup_simple_baddy_earthmud_attack(character_id, target)
                }
                FightDriverTaskKind::Bless => {
                    self.setup_simple_baddy_self_bless_action(character_id)
                }
                FightDriverTaskKind::Fireball => {
                    self.setup_simple_baddy_fireball_attack(character_id, target, area_id)
                }
                FightDriverTaskKind::FireRing => {
                    self.setup_simple_baddy_firering_attack(character_id, target)
                }
                FightDriverTaskKind::Ball => {
                    self.setup_simple_baddy_ball_attack(character_id, target, random)
                }
                FightDriverTaskKind::Flash => {
                    self.setup_simple_baddy_flash_attack(character_id, target)
                }
                FightDriverTaskKind::Warcry => {
                    self.setup_simple_baddy_warcry_attack(character_id, target)
                }
                FightDriverTaskKind::Attack => {
                    self.setup_simple_baddy_attack_driver(character_id, target)
                        || self.setup_simple_baddy_attack_move(character_id, target, area_id)
                }
                FightDriverTaskKind::Regenerate => {
                    self.setup_simple_baddy_regenerate_action(character_id)
                }
                FightDriverTaskKind::Distance3 => {
                    self.setup_simple_baddy_distance_driver(character_id, target, 3, area_id, true)
                }
                FightDriverTaskKind::Distance7 => {
                    self.setup_simple_baddy_distance_driver(character_id, target, 7, area_id, false)
                }
                FightDriverTaskKind::Pulse => self.setup_simple_baddy_pulse_attack(character_id),
                FightDriverTaskKind::AttackBack => {
                    fight_driver_attackback_may_run(&tasks, index)
                        && self.setup_simple_baddy_attack_back_move(character_id, target, area_id)
                }
                FightDriverTaskKind::MoveRight => {
                    self.setup_simple_baddy_lane_walk(character_id, Direction::Right, area_id)
                }
                FightDriverTaskKind::MoveLeft => {
                    self.setup_simple_baddy_lane_walk(character_id, Direction::Left, area_id)
                }
                FightDriverTaskKind::MoveUp => {
                    self.setup_simple_baddy_lane_walk(character_id, Direction::Up, area_id)
                }
                FightDriverTaskKind::MoveDown => {
                    self.setup_simple_baddy_lane_walk(character_id, Direction::Down, area_id)
                }
                FightDriverTaskKind::Flee => {
                    self.setup_simple_baddy_flee_action(character_id, area_id)
                }
                FightDriverTaskKind::EarthRain => false,
            };
            if ret {
                return true;
            }
        }

        false
    }

    pub(crate) fn simple_baddy_fight_tasks(
        &self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
        suppressions: impl Into<FightDriverSuppressions>,
    ) -> Vec<FightDriverTask> {
        let suppressions = suppressions.into();
        let Some(attacker) = self.characters.get(&character_id) else {
            return Vec::new();
        };
        let mut tasks = Vec::new();
        let character_distance = char_dist(attacker, target);
        let tile_distance = tile_char_dist(attacker, target);
        let current_tick = self.tick.0 as u32;
        let target_has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;

        if !suppressions.nofreeze
            && character_value(attacker, CharacterValue::Freeze) > 1
            && attacker.mana >= FREEZE_COST
            && tile_distance < 4
            && may_add_spell(target, &self.items, IDR_FREEZE, current_tick).is_some()
            && self.simple_baddy_freeze_modifier(attacker, target) < -10
        {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Freeze,
                value: FIGHT_DRIVER_HIGH_PRIO + character_value(attacker, CharacterValue::Freeze),
            });
        }
        if !suppressions.noheal && self.simple_baddy_can_heal_self(attacker) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Heal,
                value: FIGHT_DRIVER_HIGH_PRIO + character_value(attacker, CharacterValue::Heal),
            });
        }
        if !suppressions.noshield && self.simple_baddy_can_magicshield_self(attacker) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::MagicShield,
                value: FIGHT_DRIVER_HIGH_PRIO
                    + character_value(attacker, CharacterValue::MagicShield),
            });
        }
        let earthmud_good = self.simple_baddy_earthmud_value(target);
        if attacker.flags.contains(CharacterFlags::EDEMON)
            && character_value_present(attacker, CharacterValue::Demon) == 30
            && attacker.hp >= character_value(attacker, CharacterValue::Hp) * POWERSCALE / 2
            && earthmud_good > 0
        {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::EarthMud,
                value: simple_baddy_earth_task_value(earthmud_good),
            });
        }
        if !suppressions.nobless && self.simple_baddy_can_bless_self(attacker) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Bless,
                value: FIGHT_DRIVER_HIGH_PRIO + character_value(attacker, CharacterValue::Bless),
            });
        }
        let fireball_damage_value = fireball_damage(
            character_value(attacker, CharacterValue::Fireball),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            target_has_tactics,
        );
        if !suppressions.nofireball
            && character_value(attacker, CharacterValue::Fireball) > 1
            && fireball_damage_value >= POWERSCALE
            && attacker.mana >= FIREBALL_COST
        {
            let fireball_value =
                FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Fireball);
            if tile_distance < 2
                && may_add_spell(attacker, &self.items, IDR_FIRERING, current_tick).is_some()
            {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::FireRing,
                    value: fireball_value,
                });
            } else if self.fireball_line_hits_target(
                character_id,
                target.id,
                usize::from(attacker.x),
                usize::from(attacker.y),
                usize::from(target.x),
                usize::from(target.y),
            ) {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Fireball,
                    value: fireball_value,
                });
            } else if let Some((kind, distance)) =
                self.simple_baddy_fireball_lane_task(attacker, target)
            {
                tasks.push(FightDriverTask {
                    kind,
                    value: fireball_value / distance + 1,
                });
            }
        }
        if character_value(attacker, CharacterValue::Flash) > 1
            && strike_damage(
                spell_power(
                    character_value(attacker, CharacterValue::Flash),
                    character_value(attacker, CharacterValue::Tactics),
                ),
                character_value(target, CharacterValue::Immunity),
                character_value(target, CharacterValue::Tactics),
                target_has_tactics,
            ) > POWERSCALE
            && attacker.mana >= FLASH_COST
        {
            let ball_reaches_target = self.simple_baddy_calc_ball_steps(
                attacker.id,
                usize::from(attacker.x),
                usize::from(attacker.y),
                usize::from(target.x),
                usize::from(target.y),
            ) > i32::from(tile_distance) * 2 - 5;
            if !suppressions.noball
                && character_distance > 10
                && character_distance < 30
                && ball_reaches_target
            {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Ball,
                    value: FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Flash),
                });
            }
            if !suppressions.noflash
                && tile_distance < 4
                && may_add_spell(attacker, &self.items, IDR_FLASH, current_tick).is_some()
            {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Flash,
                    value: FIGHT_DRIVER_MED_PRIO
                        + character_value(attacker, CharacterValue::Flash)
                        + character_value(attacker, CharacterValue::Flash) / 2,
                });
            }
        }
        if !suppressions.nowarcry && self.simple_baddy_can_warcry(attacker, target) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Warcry,
                value: FIGHT_DRIVER_HIGH_PRIO
                    + character_value(attacker, CharacterValue::Warcry) / 2,
            });
        }
        if !suppressions.nomove || character_distance == 2 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Attack,
                value: simple_baddy_attack_task_value(attacker, &self.items),
            });
        }
        if area_id != 33 && self.simple_baddy_needs_regeneration(attacker) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Regenerate,
                value: self.simple_baddy_regenerate_task_value(attacker),
            });
        }
        if !suppressions.nomove {
            let distance3 = self.simple_baddy_distance3_task_value(attacker, target, suppressions);
            if distance3 > 0 {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Distance3,
                    value: distance3,
                });
            }
            let distance7 = self.simple_baddy_distance7_task_value(attacker, target, suppressions);
            if distance7 > 0 {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Distance7,
                    value: distance7,
                });
            }
        }
        let pulse = self.simple_baddy_pulse_value(character_id);
        if !suppressions.nopulse && pulse > 0 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Pulse,
                value: FIGHT_DRIVER_HIGH_PRIO + pulse,
            });
        }
        if !suppressions.nomove && self.simple_baddy_attackback_value(character_id, target) > 0 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::AttackBack,
                value: FIGHT_DRIVER_HIGH_PRIO,
            });
        }
        if attacker.hp < character_value(attacker, CharacterValue::Hp) * POWERSCALE / 2 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Flee,
                value: FIGHT_DRIVER_HIGH_PRIO,
            });
        }

        tasks
    }

    pub(crate) fn simple_baddy_pulse_value(&self, character_id: CharacterId) -> i32 {
        let Some(caster) = self.characters.get(&character_id) else {
            return 0;
        };
        let pulse_value = character_value(caster, CharacterValue::Pulse);
        if pulse_value == 0 || caster.mana <= POWERSCALE {
            return 0;
        }

        let pulse_power = spell_power(
            pulse_value,
            character_value(caster, CharacterValue::Tactics),
        );
        let Some(spend) = pulse_spend(pulse_power, caster.mana) else {
            return 0;
        };

        let mut damageable_total = 0;
        for dx in -2..=2 {
            for dy in -2..=2 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                damageable_total +=
                    self.simple_baddy_pulse_field_value(caster, dx, dy, spend.amount);
            }
        }

        if damageable_total * 95 > spend.mana_cost * 100 {
            pulse_value
        } else {
            0
        }
    }

    pub(crate) fn simple_baddy_pulse_field_value(
        &self,
        caster: &Character,
        dx: i32,
        dy: i32,
        pulse_amount: i32,
    ) -> i32 {
        let x = i32::from(caster.x) + dx;
        let y = i32::from(caster.y) + dy;
        if x < 0 || y < 0 {
            return 0;
        }
        let Some(tile) = self.map.tile(x as usize, y as usize) else {
            return 0;
        };
        let target_id = CharacterId(u32::from(tile.character));
        let Some(target) = self.characters.get(&target_id) else {
            return 0;
        };
        if !can_attack(caster, target, &self.map)
            || !self.map.can_see(
                usize::from(caster.x),
                usize::from(caster.y),
                x as usize,
                y as usize,
                DIST_MAX,
            )
        {
            return 0;
        }
        if target.mana > POWERSCALE * 4
            && (character_value(target, CharacterValue::MagicShield) != 0
                || character_value(target, CharacterValue::Heal) != 0)
        {
            return 0;
        }
        if target.action == action::HEAL_SELF || target.action == action::MAGICSHIELD {
            return 0;
        }

        let has = target.hp + target.lifeshield;
        let total = character_value(target, CharacterValue::Hp) * POWERSCALE
            + character_value(target, CharacterValue::MagicShield) * POWERSCALE
            + 1;
        if total <= 0 || has * 100 / total > 72 {
            return 0;
        }

        let target_has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let damage = pulse_damage(
            character_value(caster, CharacterValue::Pulse),
            pulse_amount,
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            target_has_tactics,
        );
        if damage * 95 < has * 100 {
            return 0;
        }
        has
    }

    pub(crate) fn setup_simple_baddy_fireball_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character_value(&attacker, CharacterValue::Fireball) <= 1
            || attacker.mana < FIREBALL_COST
        {
            return false;
        }

        let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let damage = fireball_damage(
            character_value(&attacker, CharacterValue::Fireball),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            has_tactics,
        );
        if damage < POWERSCALE {
            return false;
        }

        let target_dx = attacker.x.abs_diff(target.x);
        let target_dy = attacker.y.abs_diff(target.y);
        let (target_x, target_y) = if target_dx <= 1 && target_dy <= 1 {
            if may_add_spell(&attacker, &self.items, IDR_FIRERING, self.tick.0 as u32).is_none() {
                return false;
            }
            (usize::from(attacker.x), usize::from(attacker.y))
        } else {
            if !self.fireball_line_hits_target(
                character_id,
                target.id,
                usize::from(attacker.x),
                usize::from(attacker.y),
                usize::from(target.x),
                usize::from(target.y),
            ) {
                return self.setup_simple_baddy_fireball_lane_move(character_id, target, area_id);
            }
            predicted_fireball_target(&attacker, target)
        };

        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(attacker_mut) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_fireball(
            attacker_mut,
            &self.items,
            target_x,
            target_y,
            self.tick.0 as u32,
            &self.map,
            weather_movement_percent,
        )
        .is_err()
        {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = attacker_mut.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub(crate) fn setup_simple_baddy_firering_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character_value(&attacker, CharacterValue::Fireball) <= 1
            || attacker.mana < FIREBALL_COST
            || tile_char_dist(&attacker, target) >= 2
            || may_add_spell(&attacker, &self.items, IDR_FIRERING, self.tick.0 as u32).is_none()
        {
            return false;
        }

        let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let damage = fireball_damage(
            character_value(&attacker, CharacterValue::Fireball),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            has_tactics,
        );
        if damage < POWERSCALE {
            return false;
        }

        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(attacker_mut) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_fireball(
            attacker_mut,
            &self.items,
            usize::from(attacker.x),
            usize::from(attacker.y),
            self.tick.0 as u32,
            &self.map,
            weather_movement_percent,
        )
        .is_err()
        {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = attacker_mut.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub(crate) fn setup_simple_baddy_fireball_lane_move(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let directions = [
            Direction::Right,
            Direction::Left,
            Direction::Down,
            Direction::Up,
        ];
        let mut blocked_directions = [false; 4];

        for distance in 1..5 {
            for (index, direction) in directions.into_iter().enumerate() {
                if blocked_directions[index] {
                    continue;
                }
                let (dx, dy) = direction.delta();
                let Some(x) = offset_coordinate(usize::from(attacker.x), dx * distance as i16)
                else {
                    blocked_directions[index] = true;
                    continue;
                };
                let Some(y) = offset_coordinate(usize::from(attacker.y), dy * distance as i16)
                else {
                    blocked_directions[index] = true;
                    continue;
                };
                if x >= MAX_MAP || y >= MAX_MAP || self.map.blocks_movement(x, y) {
                    blocked_directions[index] = true;
                    continue;
                }
                if !self.fireball_line_hits_target(
                    character_id,
                    target.id,
                    x,
                    y,
                    usize::from(target.x),
                    usize::from(target.y),
                ) {
                    continue;
                }
                if self.setup_walk_direction(character_id, direction, area_id) {
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        if let Some(CharacterDriverState::SimpleBaddy(data)) =
                            character.driver_state.as_mut()
                        {
                            data.lastfight = self.tick.0 as i32;
                        }
                    }
                    return true;
                }
            }
        }

        false
    }

    pub(crate) fn simple_baddy_fireball_lane_task(
        &self,
        attacker: &Character,
        target: &Character,
    ) -> Option<(FightDriverTaskKind, i32)> {
        let directions = [
            (Direction::Right, FightDriverTaskKind::MoveRight),
            (Direction::Left, FightDriverTaskKind::MoveLeft),
            (Direction::Down, FightDriverTaskKind::MoveDown),
            (Direction::Up, FightDriverTaskKind::MoveUp),
        ];
        let mut blocked_directions = [false; 4];

        for distance in 1..5 {
            for (index, (direction, kind)) in directions.into_iter().enumerate() {
                if blocked_directions[index] {
                    continue;
                }
                let (dx, dy) = direction.delta();
                let Some(x) = offset_coordinate(usize::from(attacker.x), dx * distance as i16)
                else {
                    blocked_directions[index] = true;
                    continue;
                };
                let Some(y) = offset_coordinate(usize::from(attacker.y), dy * distance as i16)
                else {
                    blocked_directions[index] = true;
                    continue;
                };
                if x >= MAX_MAP || y >= MAX_MAP || self.map.blocks_movement(x, y) {
                    blocked_directions[index] = true;
                    continue;
                }
                if self.fireball_line_hits_target(
                    attacker.id,
                    target.id,
                    x,
                    y,
                    usize::from(target.x),
                    usize::from(target.y),
                ) {
                    return Some((kind, distance));
                }
            }
        }

        None
    }

    pub(crate) fn setup_simple_baddy_lane_walk(
        &mut self,
        character_id: CharacterId,
        direction: Direction,
        area_id: u16,
    ) -> bool {
        if !self.setup_walk_direction(character_id, direction, area_id) {
            return false;
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
                data.lastfight = self.tick.0 as i32;
            }
        }
        true
    }

    pub(crate) fn fireball_line_hits_target(
        &self,
        attacker_id: CharacterId,
        target_id: CharacterId,
        from_x: usize,
        from_y: usize,
        target_x: usize,
        target_y: usize,
    ) -> bool {
        let mut x = from_x as i32 * 1024 + 512;
        let mut y = from_y as i32 * 1024 + 512;
        let mut dx = target_x as i32 - from_x as i32;
        let mut dy = target_y as i32 - from_y as i32;

        if dx.abs() < 2 && dy.abs() < 2 {
            return false;
        }
        if dx.abs() > dy.abs() {
            dy = dy * 512 / dx.abs();
            dx = dx * 512 / dx.abs();
        } else {
            dx = dx * 512 / dy.abs();
            dy = dy * 512 / dy.abs();
        }

        for _ in 0..48 {
            let cx = x / 1024;
            let cy = y / 1024;
            let Ok(cx_usize) = usize::try_from(cx) else {
                return false;
            };
            let Ok(cy_usize) = usize::try_from(cy) else {
                return false;
            };
            let Some(tile) = self.map.tile(cx_usize, cy_usize) else {
                return false;
            };
            let fire_block = tile.flags.contains(MapFlags::TMOVEBLOCK)
                || (!tile.flags.contains(MapFlags::FIRETHRU)
                    && tile.flags.contains(MapFlags::MOVEBLOCK));
            if fire_block && tile.character != attacker_id.0 as u16 {
                return self.fireball_block_hits_recorded_enemy(
                    attacker_id,
                    target_id,
                    cx_usize,
                    cy_usize,
                );
            }
            x += dx;
            y += dy;
        }

        false
    }

    pub(crate) fn fireball_block_hits_recorded_enemy(
        &self,
        attacker_id: CharacterId,
        target_id: CharacterId,
        x: usize,
        y: usize,
    ) -> bool {
        let recorded_enemies = self.simple_baddy_recorded_enemy_ids(attacker_id);
        let mut hits_enemy = false;
        for (dx, dy) in [
            (0, 0),
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ] {
            let Some(check_x) = offset_coordinate(x, dx) else {
                continue;
            };
            let Some(check_y) = offset_coordinate(y, dy) else {
                continue;
            };
            let Some(character_id) = self
                .map
                .tile(check_x, check_y)
                .map(|tile| CharacterId(u32::from(tile.character)))
                .filter(|id| id.0 != 0)
            else {
                continue;
            };
            if character_id == attacker_id {
                continue;
            }
            if character_id == target_id || recorded_enemies.contains(&character_id) {
                hits_enemy = true;
            } else {
                return false;
            }
        }
        hits_enemy
    }

    #[allow(dead_code)]
    pub(crate) fn setup_simple_baddy_spell_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let current_tick = self.tick.0 as u32;
        let target_has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let tile_distance = tile_char_dist(&attacker, target);
        let character_distance = char_dist(&attacker, target);

        if character_value(&attacker, CharacterValue::Freeze) > 1
            && attacker.mana >= FREEZE_COST
            && tile_distance < 4
            && may_add_spell(target, &self.items, IDR_FREEZE, current_tick).is_some()
        {
            let modifier = freeze_speed_modifier(
                spell_power(
                    character_value(&attacker, CharacterValue::Freeze),
                    character_value(&attacker, CharacterValue::Tactics),
                ),
                character_value(target, CharacterValue::Immunity),
                character_value(target, CharacterValue::Tactics),
                target_has_tactics,
                attacker.flags.contains(CharacterFlags::IDEMON),
                // C: freeze_value (tool.c) reads the caster's V_DEMON from value[1].
                character_value_present(&attacker, CharacterValue::Demon),
                character_value(target, CharacterValue::Cold),
            );
            if modifier < -10 {
                return self.setup_simple_baddy_spell_action(
                    character_id,
                    |character, _items, _tick, map, weather_movement_percent| {
                        do_freeze(character, map, weather_movement_percent)
                    },
                );
            }
        }

        if character_value(&attacker, CharacterValue::Flash) > 1
            && attacker.mana >= FLASH_COST
            && strike_damage(
                spell_power(
                    character_value(&attacker, CharacterValue::Flash),
                    character_value(&attacker, CharacterValue::Tactics),
                ),
                character_value(target, CharacterValue::Immunity),
                character_value(target, CharacterValue::Tactics),
                target_has_tactics,
            ) > POWERSCALE
        {
            if character_distance > 10 && character_distance < 30 {
                let target_x = usize::from(target.x).saturating_sub(1)
                    + usize::try_from(random(3).min(2)).unwrap_or(0);
                let target_y = usize::from(target.y).saturating_sub(1)
                    + usize::try_from(random(3).min(2)).unwrap_or(0);
                return self.setup_simple_baddy_spell_action(
                    character_id,
                    |character, items, tick, map, weather_movement_percent| {
                        do_ball(
                            character,
                            items,
                            target_x,
                            target_y,
                            tick,
                            map,
                            weather_movement_percent,
                        )
                    },
                );
            }

            if tile_distance < 4
                && may_add_spell(&attacker, &self.items, IDR_FLASH, current_tick).is_some()
            {
                return self.setup_simple_baddy_spell_action(
                    character_id,
                    |character, items, tick, map, weather_movement_percent| {
                        do_flash(character, items, tick, map, weather_movement_percent)
                    },
                );
            }
        }

        if character_value(&attacker, CharacterValue::Warcry) > 1
            && attacker.endurance
                > character_value(&attacker, CharacterValue::Warcry) * POWERSCALE / 3
            && character_distance < 8
        {
            let modifier = warcry_speed_modifier(
                spell_power(
                    character_value(&attacker, CharacterValue::Warcry),
                    character_value(&attacker, CharacterValue::Tactics),
                ),
                character_value(target, CharacterValue::Immunity),
                character_value(target, CharacterValue::Tactics),
                target_has_tactics,
            );
            let target_accepts_warcry = modifier < -10
                && may_add_spell(target, &self.items, IDR_WARCRY, current_tick).is_some();
            let caster_needs_shield =
                character_value_present(&attacker, CharacterValue::MagicShield) == 0
                    && attacker.lifeshield
                        < character_value(&attacker, CharacterValue::Warcry) * POWERSCALE / 4;
            if target_accepts_warcry || caster_needs_shield {
                return self.setup_simple_baddy_spell_action(
                    character_id,
                    |character, items, _tick, map, weather_movement_percent| {
                        do_warcry(character, items, map, weather_movement_percent)
                    },
                );
            }
        }

        false
    }

    pub(crate) fn setup_simple_baddy_spell_action(
        &mut self,
        character_id: CharacterId,
        action: impl FnOnce(
            &mut Character,
            &HashMap<ItemId, Item>,
            u32,
            &MapGrid,
            i32,
        ) -> Result<(), crate::do_action::DoError>,
    ) -> bool {
        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if action(
            character,
            &self.items,
            self.tick.0 as u32,
            &self.map,
            weather_movement_percent,
        )
        .is_err()
        {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub(crate) fn simple_baddy_distance3_task_value(
        &self,
        attacker: &Character,
        target: &Character,
        suppressions: FightDriverSuppressions,
    ) -> i32 {
        let current_tick = self.tick.0 as u32;
        let mut value = 0;
        if !suppressions.nofreeze
            && attacker.mana > POWERSCALE * 3
            && character_value_present(attacker, CharacterValue::Freeze) != 0
            && tile_char_dist(attacker, target) > 3
            && may_add_spell(target, &self.items, IDR_FREEZE, current_tick).is_some()
            && self.simple_baddy_freeze_modifier(attacker, target) < -10
        {
            value += if character_value_present(attacker, CharacterValue::Attack) != 0 {
                FIGHT_DRIVER_LOW_PRIO + character_value(attacker, CharacterValue::Freeze)
            } else {
                FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Freeze)
            };
        }
        if !suppressions.noflash
            && attacker.mana > POWERSCALE * 3
            && character_value_present(attacker, CharacterValue::Flash) != 0
            && may_add_spell(attacker, &self.items, IDR_FLASH, current_tick).is_none()
        {
            value += if character_value_present(attacker, CharacterValue::Attack) != 0 {
                FIGHT_DRIVER_LOW_PRIO + character_value(attacker, CharacterValue::Flash)
            } else {
                FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Flash)
            };
        }
        value
    }

    pub(crate) fn simple_baddy_distance7_task_value(
        &self,
        attacker: &Character,
        target: &Character,
        suppressions: FightDriverSuppressions,
    ) -> i32 {
        if suppressions.nofireball
            || attacker.mana <= FIREBALL_COST
            || character_value_present(attacker, CharacterValue::Fireball) == 0
            || character_value_present(attacker, CharacterValue::Fireball)
                <= character_value_present(attacker, CharacterValue::Flash)
            || may_add_spell(attacker, &self.items, IDR_FLASH, self.tick.0 as u32).is_none()
        {
            return 0;
        }
        let damage = fireball_damage(
            character_value(attacker, CharacterValue::Fireball),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            character_value_present(target, CharacterValue::Tactics) != 0,
        );
        if damage < POWERSCALE {
            return 0;
        }
        if character_value_present(attacker, CharacterValue::Attack) != 0 {
            FIGHT_DRIVER_LOW_PRIO + character_value(attacker, CharacterValue::Fireball)
        } else {
            FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Fireball)
        }
    }

    pub(crate) fn simple_baddy_attackback_value(
        &self,
        character_id: CharacterId,
        target: &Character,
    ) -> i32 {
        let Some(attacker) = self.characters.get(&character_id) else {
            return 0;
        };
        let Ok(direction) = Direction::try_from(target.dir) else {
            return 0;
        };
        let (dx, dy) = direction.delta();
        if dx != 0 && dy != 0 {
            return 0;
        }
        let Some(back_x) = offset_coordinate(usize::from(target.x), -dx) else {
            return 0;
        };
        let Some(back_y) = offset_coordinate(usize::from(target.y), -dy) else {
            return 0;
        };
        if back_x < 1 || back_y < 1 || back_x >= MAX_MAP || back_y >= MAX_MAP {
            return 0;
        }
        if self.map.blocks_movement(back_x, back_y) {
            return 0;
        }
        let Some(front_x) = offset_coordinate(usize::from(target.x), dx) else {
            return 0;
        };
        let Some(front_y) = offset_coordinate(usize::from(target.y), dy) else {
            return 0;
        };
        if usize::from(attacker.x) == front_x && usize::from(attacker.y) == front_y {
            return 0;
        }
        if target.action == action::IDLE
            && self.tick.0.saturating_sub(u64::from(target.regen_ticker)) > TICKS_PER_SECOND / 2
        {
            return FIGHT_DRIVER_HIGH_PRIO;
        }
        let front_occupied = self
            .map
            .tile(front_x, front_y)
            .is_some_and(|tile| tile.character != 0);
        if !front_occupied {
            return 0;
        }
        let Some(side_x) = offset_coordinate(usize::from(target.x), dy) else {
            return 0;
        };
        let Some(side_y) = offset_coordinate(usize::from(target.y), dx) else {
            return 0;
        };
        let same_group_side_occupied = self
            .map
            .tile(side_x, side_y)
            .and_then(|tile| {
                (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
            })
            .and_then(|side_id| self.characters.get(&side_id))
            .is_some_and(|side_character| {
                side_character.id != character_id && side_character.group == attacker.group
            });
        if same_group_side_occupied {
            0
        } else {
            FIGHT_DRIVER_HIGH_PRIO
        }
    }

    pub(crate) fn simple_baddy_enemy_tracking(
        &self,
        character_id: CharacterId,
        target_id: CharacterId,
    ) -> Option<(bool, u16, u16)> {
        let character = self.characters.get(&character_id)?;
        let target = self.characters.get(&target_id)?;
        let visible = char_see_char(character, target, &self.map, self.date.daylight);
        Some((visible, target.x, target.y))
    }

    pub fn process_simple_baddy_attack_actions(&mut self, area_id: u16) -> usize {
        let mut seed = self.legacy_random_seed;
        let count = self.process_simple_baddy_attack_actions_with_random(area_id, |below| {
            legacy_random_below_from_seed(&mut seed, below)
        });
        self.legacy_random_seed = seed;
        count
    }

    pub fn process_simple_baddy_attack_actions_with_random(
        &mut self,
        area_id: u16,
        mut random: impl FnMut(u32) -> u32,
    ) -> usize {
        let character_ids: Vec<_> = self
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                ((character.driver == CDR_SIMPLEBADDY
                    || character.driver == CDR_DUNGEONFIGHTER
                    || character.driver == CDR_PENTER)
                    && matches!(
                        character.driver_state,
                        Some(CharacterDriverState::SimpleBaddy(_))
                    ))
                .then_some(character_id)
            })
            .collect();

        character_ids
            .into_iter()
            .filter(|&character_id| {
                self.process_simple_baddy_attack_action_with_random(
                    character_id,
                    area_id,
                    &mut random,
                )
            })
            .count()
    }
}

pub(crate) fn simple_baddy_earth_task_value(good_fields: i32) -> i32 {
    if good_fields < 1 {
        0
    } else if good_fields < 2 {
        FIGHT_DRIVER_LOW_PRIO
    } else if good_fields < 4 {
        FIGHT_DRIVER_MED_PRIO
    } else {
        FIGHT_DRIVER_HIGH_PRIO + good_fields * 20
    }
}

pub(crate) fn simple_baddy_attack_task_value(
    character: &Character,
    items: &HashMap<ItemId, Item>,
) -> i32 {
    let attack = simple_baddy_attack_skill(character, items);
    if character_value_present(character, CharacterValue::Attack) != 0 {
        FIGHT_DRIVER_MED_PRIO + attack * 2 / 7 + 10
    } else {
        FIGHT_DRIVER_LOW_PRIO + attack / 3
    }
}

pub(crate) fn simple_baddy_attack_skill(
    character: &Character,
    items: &HashMap<ItemId, Item>,
) -> i32 {
    let spell_average = spell_average(
        character_value(character, CharacterValue::Bless),
        character_value(character, CharacterValue::Heal),
        character_value(character, CharacterValue::Freeze),
        character_value(character, CharacterValue::MagicShield),
        character_value(character, CharacterValue::Flash),
        character_value(character, CharacterValue::Fireball),
        character_value(character, CharacterValue::Pulse),
    );
    attack_skill(
        character_value_present(character, CharacterValue::Attack) != 0,
        simple_baddy_fight_skill(character, items),
        character_value(character, CharacterValue::Attack),
        character_value(character, CharacterValue::Tactics),
        0,
        character.flags.contains(CharacterFlags::EDEMON),
        character.level as i32,
        spell_average,
    )
}

pub(crate) fn simple_baddy_fight_skill(
    character: &Character,
    items: &HashMap<ItemId, Item>,
) -> i32 {
    let Some(item_id) = character
        .inventory
        .get(worn_slot::RIGHT_HAND)
        .and_then(|slot| *slot)
    else {
        return character_value(character, CharacterValue::Hand);
    };
    let Some(item) = items.get(&item_id) else {
        return character_value(character, CharacterValue::Hand);
    };
    if !item.flags.intersects(ItemFlags::WEAPON) {
        return character_value(character, CharacterValue::Hand);
    }

    let mut value = 0;
    if item.flags.contains(ItemFlags::HAND) {
        value = value.max(character_value(character, CharacterValue::Hand));
    }
    if item.flags.contains(ItemFlags::DAGGER) {
        value = value.max(character_value(character, CharacterValue::Dagger));
    }
    if item.flags.contains(ItemFlags::STAFF) {
        value = value.max(character_value(character, CharacterValue::Staff));
    }
    if item.flags.contains(ItemFlags::SWORD) {
        value = value.max(character_value(character, CharacterValue::Sword));
    }
    if item.flags.contains(ItemFlags::TWOHAND) {
        value = value.max(character_value(character, CharacterValue::TwoHand));
    }
    value
}

pub(crate) fn predicted_fireball_target(caster: &Character, target: &Character) -> (usize, usize) {
    if target.action != action::WALK {
        return (usize::from(target.x), usize::from(target.y));
    }

    let Some(direction) = Direction::try_from(target.dir).ok() else {
        return (usize::from(target.x), usize::from(target.y));
    };
    let (dx, dy) = direction.delta();
    let mut eta = char_dist(caster, target) / 2
        + speed_ticks(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            8,
        );

    eta -= target.duration - target.step;
    if eta <= 0 {
        return (usize::from(target.x), usize::from(target.y));
    }

    for n in 1..6 {
        eta -= target.duration;
        if eta <= 0 {
            let target_x =
                offset_coordinate(usize::from(target.x), dx * n).unwrap_or(usize::from(target.x));
            let target_y =
                offset_coordinate(usize::from(target.y), dy * n).unwrap_or(usize::from(target.y));
            return (target_x, target_y);
        }
    }

    (usize::from(target.x), usize::from(target.y))
}

pub(crate) fn simple_baddy_earth_spell_target(target: &Character) -> (usize, usize) {
    if target.action != action::WALK {
        return (usize::from(target.x), usize::from(target.y));
    }

    let x = i32::from(target.tox) + i32::from(target.tox) - i32::from(target.x);
    let y = i32::from(target.toy) + i32::from(target.toy) - i32::from(target.y);
    (
        usize::try_from(x).unwrap_or(0),
        usize::try_from(y).unwrap_or(0),
    )
}

pub(crate) fn ball_target_damage_multiplier(enemy_count: i32) -> i32 {
    match enemy_count.clamp(1, 10) {
        1 => 100,
        2 => 95,
        3 => 90,
        4 => 85,
        5 => 80,
        6 => 75,
        7 => 70,
        8 => 65,
        9 => 60,
        _ => 55,
    }
}
