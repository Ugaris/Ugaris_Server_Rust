use std::collections::HashMap;

use crate::{
    area_sound::AreaSoundSpecial,
    direction::Direction,
    do_action::{
        act_attack, act_drop, act_heal, act_magicshield, act_take, act_use, act_walk,
        advance_action_step, can_attack, do_attack, do_ball, do_bless, do_drop, do_fireball,
        do_flash, do_freeze, do_heal, do_idle, do_magicshield, do_pulse, do_take, do_use, do_walk,
        do_warcry, endurance_cost, reset_action_after_act, speed_ticks, ItemUseRequest,
        DUR_MISC_ACTION,
    },
    drvlib::char_dist,
    effect::Effect,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, SpeedMode, MAX_MODIFIERS,
        POWERSCALE,
    },
    game_time::GameDate,
    ids::{CharacterId, ItemId},
    item_driver::{
        execute_item_driver_with_context, use_item, ItemDriverContext, ItemDriverOutcome,
        ItemDriverRequest, UseItemError, UseItemOutcome, IDR_FLAMETHROW, IDR_NIGHTLIGHT,
        IDR_STEPTRAP, IDR_TORCH,
    },
    item_ops::{consume_item, give_item_to_character, GiveItemFlags, GiveItemResult},
    legacy::{action, DIST_MAX, INVENTORY_START_INVENTORY, MAX_FIELD, MAX_MAP},
    light::{
        add_character_light, add_effect_light, add_item_light, compute_dlight, compute_groundlight,
        compute_shadow_with_random, remove_character_light, remove_effect_light, remove_item_light,
        reset_dlight, LIGHT_DISTANCE,
    },
    log_text::LOG_TALK,
    map::{manhattan_distance, MapFlags, MapGrid},
    path::{pathfinder, pathfinder_ignore_characters},
    player::{PlayerActionCode, PlayerRuntime},
    scheduler::{TaskScheduler, TimerPayload, TimerQueue},
    sector::{DirtySectors, SoundSectors},
    spell::{
        fireball_damage, freeze_speed_modifier, is_timed_spell_driver, may_add_spell, pulse_damage,
        read_spell_expire_tick, spell_power, strike_damage, warcry_damage, warcry_speed_modifier,
        BLESS_DURATION, EF_BALL, EF_BLESS, EF_BUBBLE, EF_BURN, EF_EARTHMUD, EF_EARTHRAIN,
        EF_EXPLODE, EF_FIREBALL, EF_FIRERING, EF_FLASH, EF_FREEZE, EF_HEAL, EF_MAGICSHIELD,
        EF_MIST, EF_POTION, EF_PULSE, EF_PULSEBACK, EF_STRIKE, EF_WARCRY, FLASH_DURATION,
        FREEZE_DURATION, IDR_BLESS, IDR_FIRERING, IDR_FLASH, IDR_FREEZE, IDR_INFRARED, IDR_POISON0,
        IDR_POISON3, IDR_POTION_SP, IDR_WARCRY, POISON_DURATION, WARCRY_DURATION,
    },
    tick::TICKS_PER_SECOND,
    Tick,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldActionCompletion {
    pub character_id: CharacterId,
    pub action_id: u16,
    pub action_item_id: Option<ItemId>,
    pub ok: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSoundSpecial {
    pub character_id: CharacterId,
    pub special: AreaSoundSpecial,
}

const ITEM_DRIVER_TIMER: &str = "item_driver";
const REMOVE_SPELL_TIMER: &str = "remove_spell";
const POISON_CALLBACK_TIMER: &str = "poison_callback";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoorToggleResult {
    Toggled,
    Blocked,
    Failed,
}

fn item_light_may_have_changed(outcome: &ItemDriverOutcome) -> bool {
    matches!(
        outcome,
        ItemDriverOutcome::LightChanged { .. }
            | ItemDriverOutcome::FlameThrowerPulse { .. }
            | ItemDriverOutcome::FlameThrowerExtinguished { .. }
            | ItemDriverOutcome::DecayItemToggled { .. }
    )
}

fn item_light_value(item: &Item) -> i16 {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .filter_map(|(&index, &value)| (index == CharacterValue::Light as i16).then_some(value))
        .sum()
}

fn character_light_value(character: &Character) -> i16 {
    character
        .values
        .first()
        .and_then(|values| values.get(CharacterValue::Light as usize))
        .copied()
        .unwrap_or_default()
}

fn integer_sqrt_for_light(strength: i16) -> usize {
    let strength = i32::from(strength).unsigned_abs().min(100) as usize;
    (strength.saturating_sub(1) as f64).sqrt() as usize + 1
}

#[derive(Debug, Default)]
pub struct World {
    pub tick: Tick,
    pub date: GameDate,
    pub timers: TimerQueue,
    pub scheduler: TaskScheduler,
    pub map: MapGrid,
    pub dirty_sectors: DirtySectors,
    pub characters: HashMap<CharacterId, Character>,
    pub items: HashMap<ItemId, Item>,
    pub effects: HashMap<u32, Effect>,
    pending_look_maps: Vec<LookMapRequest>,
}

impl World {
    pub fn advance(&mut self) {
        self.tick.0 += 1;
    }

    pub fn add_character(&mut self, character: Character) {
        if self
            .map
            .tile(usize::from(character.x), usize::from(character.y))
            .is_some_and(|tile| tile.character == character.id.0 as u16)
        {
            add_character_light(&mut self.map, &character);
            self.mark_character_light_area(&character);
        }
        self.characters.insert(character.id, character);
    }

    pub fn spawn_character(&mut self, mut character: Character, x: usize, y: usize) -> bool {
        if self.characters.contains_key(&character.id) {
            return false;
        }
        if !self.map.drop_char(&mut character, x, y) {
            return false;
        }
        self.add_character(character);
        true
    }

    pub fn remove_character(&mut self, character_id: CharacterId) -> Option<Character> {
        let mut character = self.characters.remove(&character_id)?;
        let old_x = usize::from(character.x);
        let old_y = usize::from(character.y);
        remove_character_light(&mut self.map, &character);
        self.mark_character_light_area(&character);
        self.map.remove_char(&mut character);
        self.mark_dirty_sector(old_x, old_y);
        Some(character)
    }

    pub fn sound_area_specials(
        &self,
        x: usize,
        y: usize,
        sound_type: u32,
    ) -> Vec<WorldSoundSpecial> {
        let min_x = x.saturating_sub(16);
        let max_x = x.saturating_add(16).min(self.map.width().saturating_sub(1));
        let min_y = y.saturating_sub(16);
        let max_y = y
            .saturating_add(16)
            .min(self.map.height().saturating_sub(1));
        let sectors = (sound_type == u32::from(LOG_TALK)).then(|| SoundSectors::build(&self.map));

        let mut specials = Vec::new();
        for character in self.characters.values() {
            if !character
                .flags
                .contains(CharacterFlags::USED | CharacterFlags::PLAYER)
            {
                continue;
            }
            let character_x = usize::from(character.x);
            let character_y = usize::from(character.y);
            if character_x < min_x
                || character_x > max_x
                || character_y < min_y
                || character_y > max_y
            {
                continue;
            }
            if sectors.as_ref().is_some_and(|sectors| {
                !sectors.sector_hear(&self.map, x, y, character_x, character_y)
            }) {
                continue;
            }

            let dist_x = i32::from(character.x) - x as i32;
            let dist_y = i32::from(character.y) - y as i32;
            let dist = (dist_x * dist_x + dist_y * dist_y) * 10;
            specials.push(WorldSoundSpecial {
                character_id: character.id,
                special: AreaSoundSpecial {
                    special_type: sound_type,
                    opt1: -dist,
                    opt2: dist_x * 100,
                },
            });
        }
        specials
    }

    pub fn add_item(&mut self, item: Item) {
        if let Some(old) = self.items.remove(&item.id) {
            remove_item_light(&mut self.map, &old);
            self.mark_item_light_area(&old);
        }
        add_item_light(&mut self.map, &item);
        self.mark_item_light_area(&item);
        self.items.insert(item.id, item);
    }

    pub fn skip_x_sector(&self, x: isize, y: isize, ticker: u64) -> usize {
        self.dirty_sectors.skip_x_sector(x, y, ticker)
    }

    fn mark_dirty_sector(&mut self, x: usize, y: usize) {
        self.dirty_sectors
            .set_sector(x as isize, y as isize, self.tick.0.max(1) as u64);
    }

    fn mark_light_area(&mut self, x: usize, y: usize, strength: i16) {
        if strength == 0 || self.map.tile(x, y).is_none() {
            return;
        }
        let radius = integer_sqrt_for_light(strength).min(LIGHT_DISTANCE);
        let min_x = x.saturating_sub(radius);
        let min_y = y.saturating_sub(radius);
        let max_x = x
            .saturating_add(radius)
            .min(self.map.width().saturating_sub(1));
        let max_y = y
            .saturating_add(radius)
            .min(self.map.height().saturating_sub(1));
        for ty in min_y..=max_y {
            for tx in min_x..=max_x {
                self.mark_dirty_sector(tx, ty);
            }
        }
    }

    fn mark_character_light_area(&mut self, character: &Character) {
        self.mark_light_area(
            usize::from(character.x),
            usize::from(character.y),
            character_light_value(character),
        );
    }

    fn mark_item_light_area(&mut self, item: &Item) {
        if item.x == 0 || item.y == 0 {
            return;
        }
        self.mark_light_area(
            usize::from(item.x),
            usize::from(item.y),
            item_light_value(item),
        );
    }

    pub fn compute_groundlight_at(&mut self, x: usize, y: usize) -> bool {
        let old_light = self.map.tile(x, y).map(|tile| tile.light);
        compute_groundlight(&mut self.map, x, y);
        let changed = self.map.tile(x, y).map(|tile| tile.light) != old_light;
        if changed {
            self.mark_dirty_sector(x, y);
        }
        changed
    }

    pub fn compute_shadow_at(&mut self, x: usize, y: usize) -> bool {
        self.compute_shadow_at_with_random(x, y, |_| 0)
    }

    pub fn compute_shadow_at_with_random(
        &mut self,
        x: usize,
        y: usize,
        random_below: impl FnMut(i32) -> i32,
    ) -> bool {
        let old_daylight = self.map.tile(x, y).map(|tile| tile.daylight);
        compute_shadow_with_random(&mut self.map, x, y, random_below);
        let changed = self.map.tile(x, y).map(|tile| tile.daylight) != old_daylight;
        if changed {
            self.mark_dirty_sector(x, y);
        }
        changed
    }

    pub fn compute_dlight_at(&mut self, x: usize, y: usize) -> bool {
        let changed = compute_dlight(&mut self.map, x, y);
        if changed {
            self.mark_dirty_sector(x, y);
        }
        changed
    }

    pub fn reset_dlight_around(&mut self, x: usize, y: usize) -> bool {
        if self.map.tile(x, y).is_none() {
            return false;
        }

        let xs = x.saturating_sub(LIGHT_DISTANCE);
        let ys = y.saturating_sub(LIGHT_DISTANCE);
        let xe = (x + 1 + LIGHT_DISTANCE).min(self.map.width().saturating_sub(1));
        let ye = (y + 1 + LIGHT_DISTANCE).min(self.map.height().saturating_sub(1));

        let mut before = HashMap::new();
        for ty in ys..ye {
            for tx in xs..xe {
                if let Some(tile) = self.map.tile(tx, ty) {
                    before.insert((tx, ty), tile.daylight);
                }
            }
        }

        if !reset_dlight(&mut self.map, x, y) {
            return false;
        }

        for ((tx, ty), old_daylight) in before {
            if self
                .map
                .tile(tx, ty)
                .is_some_and(|tile| tile.daylight != old_daylight)
            {
                self.mark_dirty_sector(tx, ty);
            }
        }
        true
    }

    fn next_effect_id(&self) -> u32 {
        self.effects.keys().copied().max().unwrap_or(0) + 1
    }

    fn create_fireball_effect(&mut self, caster: &Character) -> u32 {
        let effect_id = self.next_effect_id();
        let power = spell_power(
            character_value(caster, CharacterValue::Fireball),
            character_value(caster, CharacterValue::Tactics),
        );
        let mut effect = Effect::new(
            EF_FIREBALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND) as i32,
        );
        effect.strength = power;
        effect.light = 200;
        effect.from_x = i32::from(caster.x);
        effect.from_y = i32::from(caster.y);
        effect.to_x = caster.act1;
        effect.to_y = caster.act2;
        effect.caster = Some(caster.id);
        effect.caster_serial = caster.id.0 as i32;
        effect.x = i32::from(caster.x) * 1024 + 512;
        effect.y = i32::from(caster.y) * 1024 + 512;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    fn create_ball_effect(&mut self, caster: &Character) -> u32 {
        let effect_id = self.next_effect_id();
        let power = spell_power(
            character_value(caster, CharacterValue::Flash),
            character_value(caster, CharacterValue::Tactics),
        );
        let mut effect = Effect::new(
            EF_BALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 5) as i32,
        );
        effect.strength = power;
        effect.light = 80;
        effect.from_x = i32::from(caster.x);
        effect.from_y = i32::from(caster.y);
        effect.to_x = caster.act1;
        effect.to_y = caster.act2;
        effect.caster = Some(caster.id);
        effect.caster_serial = caster.id.0 as i32;
        effect.x = i32::from(caster.x) * 1024 + 512;
        effect.y = i32::from(caster.y) * 1024 + 512;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    fn create_ball_trap_effect(
        &mut self,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        power: u8,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_BALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 5) as i32,
        );
        effect.strength = i32::from(power);
        effect.light = 80;
        effect.from_x = i32::from(start_x);
        effect.from_y = i32::from(start_y);
        effect.to_x = i32::from(target_x);
        effect.to_y = i32::from(target_y);
        effect.x = i32::from(start_x) * 1024 + 512;
        effect.y = i32::from(start_y) * 1024 + 512;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    fn create_or_refresh_strike_effect(
        &mut self,
        target_id: CharacterId,
        x: i32,
        y: i32,
        strength: i32,
    ) -> u32 {
        let effect_id = self
            .effects
            .iter()
            .find_map(|(&effect_id, effect)| {
                (effect.effect_type == EF_STRIKE
                    && effect.target_character == Some(target_id)
                    && effect.x == x
                    && effect.y == y
                    && effect.strength == strength)
                    .then_some(effect_id)
            })
            .unwrap_or_else(|| {
                let effect_id = self.next_effect_id();
                let mut effect = Effect::new(
                    EF_STRIKE,
                    effect_id as i32,
                    self.tick.0 as i32,
                    self.tick.0.saturating_add(2) as i32,
                );
                effect.strength = strength;
                effect.light = 50;
                effect.x = x;
                effect.y = y;
                effect.target_character = Some(target_id);
                self.effects.insert(effect_id, effect);
                effect_id
            });

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.stop_tick = self.tick.0.saturating_add(2) as i32;
        }
        effect_id
    }

    fn create_pulse_effect(&mut self, x: u16, y: u16, strength: i32) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_PULSE,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(6) as i32,
        );
        effect.strength = strength;
        effect.x = i32::from(x);
        effect.y = i32::from(y);
        self.effects.insert(effect_id, effect);
        self.set_effect_on_map(effect_id, i32::from(x), i32::from(y));
        effect_id
    }

    fn create_pulseback_effect(
        &mut self,
        target_id: CharacterId,
        caster_x: u16,
        caster_y: u16,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_PULSEBACK,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(7) as i32,
        );
        effect.target_character = Some(target_id);
        effect.x = i32::from(caster_x);
        effect.y = i32::from(caster_y);
        effect.light = 20;
        effect.strength = strength;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    pub fn create_explosion_effect(
        &mut self,
        x: i32,
        y: i32,
        max_age: u32,
        base_sprite: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_EXPLODE,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(u64::from(max_age)) as i32,
        );
        effect.strength = max_age as i32;
        effect.light = 200;
        effect.base_sprite = base_sprite;
        self.effects.insert(effect_id, effect);
        self.set_effect_on_map(effect_id, x, y);
        effect_id
    }

    pub fn create_mist_effect(&mut self, x: i32, y: i32) -> u32 {
        self.create_map_effect(
            EF_MIST,
            x,
            y,
            self.tick.0 as i32,
            self.tick.0 as i32 + 24,
            0,
            0,
        )
    }

    pub fn create_map_effect(
        &mut self,
        effect_type: i32,
        x: i32,
        y: i32,
        start_tick: i32,
        stop_tick: i32,
        light: i32,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(effect_type, effect_id as i32, start_tick, stop_tick);
        effect.light = light;
        effect.strength = strength;
        self.effects.insert(effect_id, effect);
        self.set_effect_on_map(effect_id, x, y);
        effect_id
    }

    pub fn create_bubble_effect(&mut self, x: i32, y: i32, y_offset: i32, duration: u32) -> u32 {
        self.create_map_effect(
            EF_BUBBLE,
            x,
            y,
            self.tick.0 as i32,
            self.tick.0.saturating_add(u64::from(duration)) as i32,
            0,
            y_offset,
        )
    }

    pub fn create_earthrain_effect(&mut self, x: i32, y: i32, strength: i32) -> u32 {
        self.create_area_map_effect(EF_EARTHRAIN, x, y, 10, strength)
    }

    pub fn create_earthmud_effect(&mut self, x: i32, y: i32, strength: i32) -> u32 {
        self.create_area_map_effect(EF_EARTHMUD, x, y, 0, strength)
    }

    fn create_area_map_effect(
        &mut self,
        effect_type: i32,
        x: i32,
        y: i32,
        light: i32,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            effect_type,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 60) as i32,
        );
        effect.light = light;
        effect.strength = strength;
        self.effects.insert(effect_id, effect);

        self.add_area_effect_map_tile(effect_id, x, y, effect_type);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let tx = x + dx;
                let ty = y + dy;
                if !self.map_tile_blocks_sight(tx, ty) {
                    self.add_area_effect_map_tile(effect_id, tx, ty, effect_type);
                }
            }
        }
        effect_id
    }

    fn add_area_effect_map_tile(
        &mut self,
        effect_id: u32,
        x: i32,
        y: i32,
        effect_type: i32,
    ) -> bool {
        let (Ok(x_usize), Ok(y_usize)) = (usize::try_from(x), usize::try_from(y)) else {
            return false;
        };
        if self.map.tile(x_usize, y_usize).is_some_and(|tile| {
            tile.effects.iter().any(|&slot| {
                slot != 0
                    && self
                        .effects
                        .get(&u32::from(slot))
                        .is_some_and(|effect| effect.effect_type == effect_type)
            })
        }) {
            return false;
        }
        self.set_effect_on_map(effect_id, x, y)
    }

    fn map_tile_blocks_sight(&self, x: i32, y: i32) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return true;
        };
        self.map.tile(x, y).is_none_or(|tile| {
            tile.flags
                .intersects(MapFlags::SIGHTBLOCK | MapFlags::TSIGHTBLOCK)
        })
    }

    fn create_show_effect(
        &mut self,
        effect_type: i32,
        target_id: CharacterId,
        start_tick: u32,
        stop_tick: u32,
        light: i32,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            effect_type,
            effect_id as i32,
            start_tick as i32,
            stop_tick as i32,
        );
        effect.target_character = Some(target_id);
        effect.strength = strength;
        effect.light = light;
        if let Some(target) = self.characters.get(&target_id) {
            effect.x = i32::from(target.x);
            effect.y = i32::from(target.y);
        }
        self.effects.insert(effect_id, effect);
        effect_id
    }

    fn remove_show_effect_type(&mut self, target_id: CharacterId, effect_type: i32) {
        self.effects.retain(|_, effect| {
            !(effect.effect_type == effect_type && effect.target_character == Some(target_id))
        });
    }

    pub fn tick_effects(&mut self) {
        let mut state = self.tick.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.tick_effects_with_random(|limit| {
            if limit <= 0 {
                return 0;
            }
            state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (state % limit as u64) as i32
        });
    }

    pub fn tick_effects_with_random(&mut self, mut random_below: impl FnMut(i32) -> i32) {
        let effect_ids: Vec<u32> = self.effects.keys().copied().collect();
        for effect_id in effect_ids {
            match self
                .effects
                .get(&effect_id)
                .map(|effect| effect.effect_type)
            {
                Some(EF_FIREBALL) => self.tick_fireball_effect(effect_id),
                Some(EF_BALL) => self.tick_ball_effect(effect_id),
                Some(EF_STRIKE | EF_PULSE) => self.tick_strike_effect(effect_id),
                Some(EF_BURN) => self.tick_burn_effect(effect_id),
                Some(EF_EARTHRAIN) => self.tick_earthrain_effect(effect_id, &mut random_below),
                Some(_) => self.tick_expiring_effect(effect_id),
                _ => {}
            }
        }
    }

    fn tick_earthrain_effect(&mut self, effect_id: u32, random_below: &mut impl FnMut(i32) -> i32) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };

        let mut targets = Vec::new();
        for index in &effect.fields {
            if *index < 0 {
                continue;
            }
            let index = *index as usize;
            let x = index % self.map.width();
            let y = index / self.map.width();
            let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
            }) else {
                continue;
            };
            let Some(target) = self.characters.get(&target_id) else {
                continue;
            };
            if !target.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            let reduction =
                (effect.strength - character_value(target, CharacterValue::Demon)).max(0);
            let damage = reduction * 150;
            if damage == 0 || random_below(10) != 0 {
                continue;
            }
            targets.push((target_id, damage));
        }

        for (target_id, damage) in targets {
            if let Some(target) = self.characters.get_mut(&target_id) {
                target.hp = target.hp.saturating_sub(damage);
                target.flags.insert(CharacterFlags::UPDATE);
            }
        }

        self.tick_expiring_effect(effect_id);
    }

    fn tick_expiring_effect(&mut self, effect_id: u32) {
        if self
            .effects
            .get(&effect_id)
            .is_some_and(|effect| self.tick.0 >= effect.stop_tick as u64)
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
        }
    }

    fn tick_burn_effect(&mut self, effect_id: u32) {
        if self
            .effects
            .get(&effect_id)
            .is_some_and(|effect| self.tick.0 >= effect.stop_tick as u64)
        {
            self.effects.remove(&effect_id);
        }
    }

    fn tick_strike_effect(&mut self, effect_id: u32) {
        if self
            .effects
            .get(&effect_id)
            .is_some_and(|effect| self.tick.0 >= effect.stop_tick as u64)
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
        }
    }

    fn tick_ball_effect(&mut self, effect_id: u32) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };

        if effect.caster.is_some_and(|caster_id| {
            !self
                .characters
                .get(&caster_id)
                .is_some_and(|caster| caster.flags.contains(CharacterFlags::USED))
        }) || self.tick.0 >= effect.stop_tick as u64
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
            return;
        }

        let old_x = effect.x / 1024;
        let old_y = effect.y / 1024;
        let raw_dx = effect.to_x - effect.from_x;
        let raw_dy = effect.to_y - effect.from_y;
        if raw_dx == 0 && raw_dy == 0 {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
            return;
        }

        let (step_x, step_y) = if raw_dx.abs() > raw_dy.abs() {
            (raw_dx * 128 / raw_dx.abs(), raw_dy * 128 / raw_dx.abs())
        } else {
            (raw_dx * 128 / raw_dy.abs(), raw_dy * 128 / raw_dy.abs())
        };
        let x = effect.x + step_x;
        let y = effect.y + step_y;
        let tile_x = x / 1024;
        let tile_y = y / 1024;

        if self.fire_map_blocked(tile_x, tile_y)
            && !effect
                .caster
                .and_then(|caster_id| self.characters.get(&caster_id))
                .is_some_and(|caster| {
                    (i32::from(caster.x), i32::from(caster.y)) == (tile_x, tile_y)
                })
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
            return;
        }

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.x = x;
            effect.y = y;
            effect.last_x = old_x;
            effect.last_y = old_y;
        }
        if old_x != tile_x || old_y != tile_y {
            self.remove_effect_from_map(effect_id);
            self.set_effect_on_map(effect_id, tile_x, tile_y);
        }
        self.apply_ball_strikes(effect_id, tile_x, tile_y);
    }

    fn apply_ball_strikes(&mut self, effect_id: u32, x: i32, y: i32) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };
        let Some(caster_id) = effect.caster else {
            return;
        };
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return;
        };

        let mut targets = Vec::new();
        let min_x = (x - 5).max(1);
        let min_y = (y - 5).max(1);
        let max_x = (x + 5).min(self.map.width().saturating_sub(2) as i32);
        let max_y = (y + 5).min(self.map.height().saturating_sub(2) as i32);
        for target_y in min_y..max_y {
            for target_x in min_x..max_x {
                let (Ok(target_x_usize), Ok(target_y_usize)) =
                    (usize::try_from(target_x), usize::try_from(target_y))
                else {
                    continue;
                };
                let Some(target_id) =
                    self.map
                        .tile(target_x_usize, target_y_usize)
                        .and_then(|tile| {
                            (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                        })
                else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack(&caster, target, &self.map) {
                    continue;
                }
                let (Ok(ball_x), Ok(ball_y)) = (usize::try_from(x), usize::try_from(y)) else {
                    continue;
                };
                if !self
                    .map
                    .can_see(ball_x, ball_y, target_x_usize, target_y_usize, 5)
                {
                    continue;
                }
                if self.tick.0 & 3 == 0 {
                    let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
                    let damage = strike_damage(
                        effect.strength,
                        character_value(target, CharacterValue::Immunity),
                        character_value(target, CharacterValue::Tactics),
                        has_tactics,
                    ) * ball_target_damage_multiplier(effect.number_of_enemies)
                        / (25 * TICKS_PER_SECOND as i32 * 2);
                    targets.push((target_id, damage));
                } else {
                    targets.push((target_id, 0));
                }
            }
        }

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.number_of_enemies = targets.len() as i32;
        }
        for (target_id, damage) in targets {
            self.create_or_refresh_strike_effect(target_id, x, y, effect.strength);
            if damage == 0 {
                continue;
            }
            if let Some(target) = self.characters.get_mut(&target_id) {
                target.hp = target.hp.saturating_sub(damage);
                target.flags.insert(CharacterFlags::UPDATE);
            }
        }
    }

    fn tick_fireball_effect(&mut self, effect_id: u32) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };

        if effect.caster.is_some_and(|caster_id| {
            !self
                .characters
                .get(&caster_id)
                .is_some_and(|caster| caster.flags.contains(CharacterFlags::USED))
        }) || self.tick.0 >= effect.stop_tick as u64
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
            return;
        }

        self.remove_effect_from_map(effect_id);

        let raw_dx = effect.to_x - effect.from_x;
        let raw_dy = effect.to_y - effect.from_y;
        if raw_dx == 0 && raw_dy == 0 {
            self.explode_fireball_effect(effect_id, effect.x / 1024, effect.y / 1024);
            return;
        }

        let (step_x, step_y) = if raw_dx.abs() > raw_dy.abs() {
            (raw_dx * 512 / raw_dx.abs(), raw_dy * 512 / raw_dx.abs())
        } else {
            (raw_dx * 512 / raw_dy.abs(), raw_dy * 512 / raw_dy.abs())
        };

        let mut x = effect.x;
        let mut y = effect.y;
        let mut last_x = effect.last_x;
        let mut last_y = effect.last_y;
        for _ in 0..2 {
            last_x = x / 1024;
            last_y = y / 1024;
            x += step_x;
            y += step_y;

            let tile_x = x / 1024;
            let tile_y = y / 1024;
            if self.fire_map_blocked(tile_x, tile_y)
                && !self.fire_tile_contains_caster(effect.caster, tile_x, tile_y)
            {
                if let Some(effect) = self.effects.get_mut(&effect_id) {
                    effect.x = x;
                    effect.y = y;
                    effect.last_x = last_x;
                    effect.last_y = last_y;
                }
                self.explode_fireball_effect(effect_id, tile_x, tile_y);
                return;
            }
        }

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.x = x;
            effect.y = y;
            effect.last_x = last_x;
            effect.last_y = last_y;
        }
        self.set_effect_on_map(effect_id, x / 1024, y / 1024);
    }

    fn fire_map_blocked(&self, x: i32, y: i32) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return true;
        };
        let Some(tile) = self.map.tile(x, y) else {
            return true;
        };
        tile.flags.contains(MapFlags::TMOVEBLOCK)
            || (!tile.flags.contains(MapFlags::FIRETHRU)
                && tile.flags.contains(MapFlags::MOVEBLOCK))
    }

    fn fire_tile_contains_caster(&self, caster_id: Option<CharacterId>, x: i32, y: i32) -> bool {
        let Some(caster_id) = caster_id else {
            return false;
        };
        let Some(caster) = self.characters.get(&caster_id) else {
            return false;
        };
        (i32::from(caster.x), i32::from(caster.y)) == (x, y)
            || (i32::from(caster.tox), i32::from(caster.toy)) == (x, y)
    }

    fn set_effect_on_map(&mut self, effect_id: u32, x: i32, y: i32) -> bool {
        if effect_id == 0 || effect_id > u32::from(u16::MAX) {
            return false;
        }
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return false;
        };
        if !self.map.legacy_inner_bounds(x, y) {
            return false;
        }
        let Some(effect) = self.effects.get_mut(&effect_id) else {
            return false;
        };
        let light = effect.light;
        if effect.fields.len() >= MAX_FIELD {
            return false;
        }
        let Some(tile) = self.map.tile_mut(x, y) else {
            return false;
        };
        let Some(slot) = tile.effects.iter_mut().find(|slot| **slot == 0) else {
            return false;
        };
        *slot = effect_id as u16;
        if let Some(index) = self.map.legacy_index(x, y) {
            effect.fields.push(index as i32);
        }
        add_effect_light(&mut self.map, x, y, light as i16);
        self.mark_light_area(x, y, light as i16);
        true
    }

    fn remove_effect_from_map(&mut self, effect_id: u32) {
        let Some(effect) = self.effects.get_mut(&effect_id) else {
            return;
        };
        let light = effect.light;
        let fields = std::mem::take(&mut effect.fields);
        for index in fields {
            if index < 0 {
                continue;
            }
            let index = index as usize;
            let x = index % self.map.width();
            let y = index / self.map.width();
            if let Some(tile) = self.map.tile_mut(x, y) {
                for slot in &mut tile.effects {
                    if *slot == effect_id as u16 {
                        *slot = 0;
                        break;
                    }
                }
            }
            remove_effect_light(&mut self.map, x, y, light as i16);
            self.mark_light_area(x, y, light as i16);
        }
    }

    fn explode_fireball_effect(&mut self, effect_id: u32, x: i32, y: i32) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };
        self.remove_effect_from_map(effect_id);
        self.effects.remove(&effect_id);

        let Some(caster_id) = effect.caster else {
            return;
        };
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return;
        };

        let mut targets = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                let target_x = x + dx;
                let target_y = y + dy;
                let (Ok(target_x_usize), Ok(target_y_usize)) =
                    (usize::try_from(target_x), usize::try_from(target_y))
                else {
                    continue;
                };
                if dx != 0 || dy != 0 {
                    let (Ok(last_x), Ok(last_y)) = (
                        usize::try_from(effect.last_x),
                        usize::try_from(effect.last_y),
                    ) else {
                        continue;
                    };
                    if !self
                        .map
                        .can_see(last_x, last_y, target_x_usize, target_y_usize, 5)
                    {
                        continue;
                    }
                }
                let Some(target_id) =
                    self.map
                        .tile(target_x_usize, target_y_usize)
                        .and_then(|tile| {
                            (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                        })
                else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack(&caster, target, &self.map) {
                    return;
                }
                let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
                let damage = fireball_damage(
                    effect.strength,
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    has_tactics,
                );
                targets.push((target_id, damage));
            }
        }

        for (target_id, damage) in targets {
            if let Some(target) = self.characters.get_mut(&target_id) {
                target.hp = target.hp.saturating_sub(damage);
                target.flags.insert(CharacterFlags::UPDATE);
            }
        }
    }

    pub fn drain_look_map_requests(&mut self) -> Vec<LookMapRequest> {
        self.pending_look_maps.drain(..).collect()
    }

    pub fn process_due_timers(&mut self, area_id: u16) -> Vec<ItemDriverOutcome> {
        let mut outcomes = Vec::new();
        for event in self.timers.tick(self.tick.0) {
            match event.name.as_str() {
                ITEM_DRIVER_TIMER => {
                    let [driver, item_id, character_id, timer_call, _] = event.payload.0;
                    if driver <= 0 || item_id <= 0 || character_id < 0 {
                        continue;
                    }
                    let request = ItemDriverRequest::Driver {
                        driver: driver as u16,
                        item_id: ItemId(item_id as u32),
                        character_id: CharacterId(character_id as u32),
                        spec: 0,
                    };
                    outcomes.push(self.execute_item_driver_timer_request(
                        request,
                        area_id,
                        &ItemDriverContext {
                            timer_call: timer_call != 0,
                            ..ItemDriverContext::default()
                        },
                    ));
                }
                REMOVE_SPELL_TIMER => {
                    let [character_id, item_id, slot, character_serial, item_serial] =
                        event.payload.0;
                    if character_id <= 0 || item_id <= 0 || slot < 0 {
                        continue;
                    }
                    self.remove_spell_from_timer(
                        CharacterId(character_id as u32),
                        ItemId(item_id as u32),
                        slot as usize,
                        character_serial as u32,
                        item_serial as u32,
                    );
                }
                POISON_CALLBACK_TIMER => {
                    let [character_id, item_id, slot, character_serial, item_serial] =
                        event.payload.0;
                    if character_id <= 0 || item_id <= 0 || slot < 0 {
                        continue;
                    }
                    self.poison_callback_from_timer(
                        CharacterId(character_id as u32),
                        ItemId(item_id as u32),
                        slot as usize,
                        character_serial as u32,
                        item_serial as u32,
                    );
                }
                _ => {}
            }
        }
        outcomes
    }

    pub fn schedule_existing_light_timers(&mut self) -> usize {
        let item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter_map(|(&item_id, item)| match item.driver {
                IDR_NIGHTLIGHT => Some(item_id),
                IDR_TORCH if item.driver_data.first().copied().unwrap_or(0) != 0 => Some(item_id),
                IDR_FLAMETHROW => Some(item_id),
                _ => None,
            })
            .collect();

        item_ids
            .into_iter()
            .filter(|&item_id| {
                let character_id = self
                    .items
                    .get(&item_id)
                    .and_then(|item| item.carried_by)
                    .unwrap_or(CharacterId(0));
                self.schedule_item_driver_timer(item_id, character_id, 1)
            })
            .count()
    }

    pub fn advance_character_action(&mut self, character_id: CharacterId) -> Option<bool> {
        self.characters
            .get_mut(&character_id)
            .map(advance_action_step)
    }

    pub fn reset_character_action(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        reset_action_after_act(character);
        true
    }

    pub fn refresh_character_light_after_value_change(
        &mut self,
        character_id: CharacterId,
        old_light: i16,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let new_light = character_light_value(character);
        if old_light == new_light {
            return false;
        }

        let mut before = character.clone();
        if let Some(values) = before.values.get_mut(0) {
            if let Some(light) = values.get_mut(CharacterValue::Light as usize) {
                *light = old_light;
            }
        }
        remove_character_light(&mut self.map, &before);
        add_character_light(&mut self.map, character);
        character.flags.insert(CharacterFlags::UPDATE);
        let after = character.clone();
        self.mark_character_light_area(&before);
        self.mark_character_light_area(&after);
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

    pub fn complete_attack_with_rolls(
        &mut self,
        attacker_id: CharacterId,
        defender_id: CharacterId,
        d100_roll: i32,
        d6_roll: i32,
    ) -> bool {
        if attacker_id == defender_id {
            return false;
        }
        let Some(mut defender) = self.characters.remove(&defender_id) else {
            return false;
        };
        let ok = self
            .characters
            .get_mut(&attacker_id)
            .and_then(|attacker| act_attack(attacker, &mut defender, &self.map, d100_roll, d6_roll))
            .is_some();
        self.characters.insert(defender_id, defender);
        ok
    }

    pub fn use_item_request(
        &mut self,
        request: ItemUseRequest,
        account_depot_available: bool,
    ) -> Result<UseItemOutcome, UseItemError> {
        let Some(character) = self.characters.get_mut(&request.character_id) else {
            return Err(UseItemError::IllegalCharacter);
        };
        let Some(item) = self.items.get(&request.item_id) else {
            return Err(UseItemError::IllegalItem);
        };
        use_item(character, item, request, account_depot_available)
    }

    pub fn execute_item_driver_request(
        &mut self,
        request: ItemDriverRequest,
        area_id: u16,
    ) -> ItemDriverOutcome {
        self.execute_item_driver_request_with_context(
            request,
            area_id,
            &ItemDriverContext::default(),
        )
    }

    pub fn execute_item_driver_request_with_context(
        &mut self,
        request: ItemDriverRequest,
        area_id: u16,
        context: &ItemDriverContext,
    ) -> ItemDriverOutcome {
        let (character_id, item_id) = match request {
            ItemDriverRequest::Driver {
                character_id,
                item_id,
                ..
            }
            | ItemDriverRequest::AccountDepot {
                character_id,
                item_id,
            } => (character_id, item_id),
        };
        let character_tile_flags = self
            .characters
            .get(&character_id)
            .and_then(|character| {
                self.map
                    .tile(usize::from(character.x), usize::from(character.y))
            })
            .map(|tile| tile.flags)
            .unwrap_or_else(MapFlags::empty);
        let in_arena = character_tile_flags.contains(MapFlags::ARENA);
        let cursor_context = self
            .characters
            .get(&character_id)
            .and_then(|character| character.cursor_item)
            .and_then(|cursor_item_id| self.items.get(&cursor_item_id))
            .map(|item| {
                (
                    item.driver,
                    item.sprite,
                    item.driver_data.first().copied().unwrap_or(0),
                )
            });
        let Some(character) = self.characters.get_mut(&character_id) else {
            return ItemDriverOutcome::Noop;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return ItemDriverOutcome::Noop;
        };
        let mut effective_context = context.clone();
        if let Some((cursor_driver, cursor_sprite, cursor_drdata0)) = cursor_context {
            effective_context.cursor_driver =
                effective_context.cursor_driver.or(Some(cursor_driver));
            effective_context.cursor_sprite =
                effective_context.cursor_sprite.or(Some(cursor_sprite));
            effective_context.cursor_drdata0 =
                effective_context.cursor_drdata0.or(Some(cursor_drdata0));
        }
        effective_context.character_underwater |=
            character_tile_flags.contains(MapFlags::UNDERWATER);
        effective_context.daylight = effective_context
            .daylight
            .max(self.date.daylight.clamp(0, 255) as u8);
        let before = item.clone();
        let outcome = execute_item_driver_with_context(
            character,
            item,
            request,
            area_id,
            in_arena,
            &effective_context,
        );
        if item_light_may_have_changed(&outcome) {
            self.refresh_item_light_after_mutation(&before, item_id);
        }
        self.apply_item_driver_outcome(outcome, area_id)
    }

    fn execute_item_driver_timer_request(
        &mut self,
        request: ItemDriverRequest,
        area_id: u16,
        context: &ItemDriverContext,
    ) -> ItemDriverOutcome {
        let ItemDriverRequest::Driver {
            driver,
            item_id,
            character_id,
            spec,
        } = request
        else {
            return self.execute_item_driver_request_with_context(request, area_id, context);
        };

        if character_id.0 != 0 {
            return self.execute_item_driver_request_with_context(request, area_id, context);
        }

        let Some(item) = self.items.get_mut(&item_id) else {
            return ItemDriverOutcome::Noop;
        };
        let mut timer_character = timer_callback_character();
        let before = item.clone();
        let outcome = execute_item_driver_with_context(
            &mut timer_character,
            item,
            ItemDriverRequest::Driver {
                driver,
                item_id,
                character_id,
                spec,
            },
            area_id,
            false,
            context,
        );
        if item_light_may_have_changed(&outcome) {
            self.refresh_item_light_after_mutation(&before, item_id);
        }
        self.apply_item_driver_outcome(outcome, area_id)
    }

    fn refresh_item_light_after_mutation(&mut self, before: &Item, item_id: ItemId) {
        remove_item_light(&mut self.map, before);
        self.mark_item_light_area(before);
        if let Some(after) = self.items.get(&item_id) {
            let after = after.clone();
            add_item_light(&mut self.map, &after);
            self.mark_item_light_area(&after);
        }
    }

    fn apply_item_driver_outcome(
        &mut self,
        outcome: ItemDriverOutcome,
        current_area_id: u16,
    ) -> ItemDriverOutcome {
        match outcome {
            ItemDriverOutcome::Teleport {
                character_id,
                x,
                y,
                area_id,
                ..
            } => {
                if area_id != 0 && area_id != current_area_id {
                    return outcome;
                }
                if self.teleport_character(character_id, x, y, true) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TeleportDoor {
                character_id, x, y, ..
            } => {
                if self.teleport_character(character_id, x, y, false) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::Recall {
                item_id,
                character_id,
                x,
                y,
                area_id,
            } => {
                if area_id != current_area_id {
                    return outcome;
                }
                if !self.teleport_character(character_id, x, y, false) {
                    return ItemDriverOutcome::Noop;
                }
                if let (Some(character), Some(item)) = (
                    self.characters.get_mut(&character_id),
                    self.items.get_mut(&item_id),
                ) {
                    consume_item(character, item);
                }
                outcome
            }
            ItemDriverOutcome::CityRecall {
                item_id,
                character_id,
                x,
                y,
                area_id,
            } => {
                self.consume_city_recall_scroll(character_id, item_id);
                if area_id != current_area_id {
                    return outcome;
                }
                if self.teleport_character(character_id, x, y, false) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::DoorToggle {
                item_id,
                character_id,
            } => {
                if self.toggle_door(item_id, character_id) == DoorToggleResult::Toggled {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::KeyedDoorToggle {
                item_id,
                character_id,
                ..
            } => {
                if self.toggle_door(item_id, character_id) == DoorToggleResult::Toggled {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::DoubleDoorToggle {
                item_id,
                character_id,
            } => {
                if self.toggle_double_door(item_id, character_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TriggerMapItem {
                x,
                y,
                target_character_id,
                delay_ticks,
                ..
            } => {
                if self.schedule_map_item_driver_timer(
                    usize::from(x),
                    usize::from(y),
                    target_character_id,
                    delay_ticks,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StepTrapDiscoverTarget { item_id } => {
                if self.discover_steptrap_target(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BallTrapProjectile {
                start_x,
                start_y,
                target_x,
                target_y,
                power,
                ..
            } => {
                self.create_ball_trap_effect(start_x, start_y, target_x, target_y, power);
                outcome
            }
            ItemDriverOutcome::SpikeTrapTriggered {
                item_id,
                character_id,
                damage,
                reset_after_ticks,
            } => {
                if let Some(character) = self.characters.get_mut(&character_id) {
                    character.hp = character.hp.saturating_sub(damage);
                    character.flags.insert(CharacterFlags::UPDATE);
                }
                self.schedule_item_driver_timer(item_id, CharacterId(0), reset_after_ticks);
                outcome
            }
            ItemDriverOutcome::SpikeTrapReset { .. } => outcome,
            ItemDriverOutcome::Extinguish {
                item_id,
                character_id,
                ..
            } => {
                let extinguished = self.remove_character_burn_effect(character_id);
                ItemDriverOutcome::Extinguish {
                    item_id,
                    character_id,
                    extinguished,
                }
            }
            ItemDriverOutcome::FlameThrowerPulse {
                item_id,
                direction,
                schedule_after_ticks,
                ..
            } => {
                self.mark_flamethrower_targets_for_burn(item_id, direction);
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::FlameThrowerExtinguished {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::LightChanged {
                item_id,
                character_id,
                schedule_after_ticks,
            } => {
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, character_id, after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::TorchExtinguishedUnderwater {
                item_id,
                character_id,
                schedule_after_ticks,
            } => {
                self.schedule_item_driver_timer(item_id, character_id, schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::TorchExpired { item_id, .. } => {
                if self.destroy_item(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::DecayItemToggled {
                item_id,
                character_id,
                schedule_after_ticks,
                ..
            } => {
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, character_id, after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::DecayItemExpired { item_id, .. } => {
                if self.destroy_item(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::LabExitAnimating {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::LabExitExpired { item_id } => {
                if self.destroy_item(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::LabExitUse { .. } | ItemDriverOutcome::LabExitWrongOwner { .. } => {
                outcome
            }
            ItemDriverOutcome::BeyondPotion {
                item_id,
                character_id,
                duration_minutes,
                modifier_index,
                modifier_value,
                beyond_max_mod,
            } => {
                if self.install_beyond_potion_spell(
                    character_id,
                    item_id,
                    duration_minutes,
                    modifier_index,
                    modifier_value,
                    beyond_max_mod,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::BlockedByRequirements {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::SpecialPotionAntidote {
                item_id,
                character_id,
                kind,
                ..
            } => {
                let poison_removed = if kind <= 3 {
                    self.remove_poison(character_id, u16::from(kind))
                } else {
                    self.remove_all_poison(character_id)
                };
                self.destroy_item(item_id);
                ItemDriverOutcome::SpecialPotionAntidote {
                    item_id,
                    character_id,
                    kind,
                    poison_removed,
                }
            }
            ItemDriverOutcome::SpecialPotionInfravision {
                item_id,
                character_id,
                ..
            } => {
                let installed = self.install_infravision_spell(character_id);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::SpecialPotionInfravision {
                    item_id,
                    character_id,
                    installed,
                }
            }
            ItemDriverOutcome::SpecialPotionSecurity { .. }
            | ItemDriverOutcome::SpecialPotionProfessionReset { .. } => outcome,
            ItemDriverOutcome::SpecialShrine { .. } => outcome,
            ItemDriverOutcome::TorchExtractOrb { .. } => outcome,
            ItemDriverOutcome::NomadStack { .. } => outcome,
            ItemDriverOutcome::EnchantCursorItem {
                item_id,
                character_id,
                cursor_item_id,
                modifier,
                amount,
            } => {
                if self.apply_enchant_cursor_item(
                    item_id,
                    character_id,
                    cursor_item_id,
                    modifier,
                    amount,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::BlockedByRequirements {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::AntiEnchantCursorItem {
                item_id,
                character_id,
                cursor_item_id,
                modifier,
                amount,
                extract_orb: _,
            } => {
                if self.apply_anti_enchant_cursor_item(
                    item_id,
                    character_id,
                    cursor_item_id,
                    modifier,
                    amount,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::BlockedByRequirements {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::ShrikeAmuletAssemble {
                item_id,
                character_id,
                cursor_item_id,
                combined_bits,
            } => {
                if self.apply_shrike_amulet_assemble(
                    item_id,
                    character_id,
                    cursor_item_id,
                    combined_bits,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::ShrikeAmuletDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::MineGatewayKeyAssemble {
                item_id,
                character_id,
                cursor_item_id,
                combined_bits,
            } => {
                if self.apply_mine_gateway_key_assemble(
                    item_id,
                    character_id,
                    cursor_item_id,
                    combined_bits,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::MineGatewayKeyDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::PalaceKeyCombine {
                item_id,
                character_id,
                cursor_item_id,
                result_sprite,
                final_key,
            } => {
                if self.apply_palace_key_combine(
                    item_id,
                    character_id,
                    cursor_item_id,
                    result_sprite,
                    final_key,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::PalaceKeyDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            _ => outcome,
        }
    }

    fn apply_enchant_cursor_item(
        &mut self,
        orb_item_id: ItemId,
        character_id: CharacterId,
        target_item_id: ItemId,
        modifier: i16,
        amount: i16,
    ) -> bool {
        if amount <= 0 || !self.character_holds_cursor_item(character_id, target_item_id) {
            return false;
        }

        let Some(target) = self.items.get(&target_item_id) else {
            return false;
        };
        if !target.flags.intersects(ItemFlags::WEAR)
            || target.flags.contains(ItemFlags::NOENHANCE)
            || target.flags.contains(ItemFlags::WNLHAND)
        {
            return false;
        }

        let current = current_modifier_value(target, modifier).unwrap_or_default();
        let new_value = current.saturating_add(amount);
        if new_value > 20 {
            return false;
        }
        if current == 0 && counted_enhancement_modifiers(target) >= 3 {
            return false;
        }
        let Some(slot) = modifier_slot_for_write(target, modifier) else {
            return false;
        };

        if !self.destroy_item(orb_item_id) {
            return false;
        }
        let Some(target) = self.items.get_mut(&target_item_id) else {
            return false;
        };
        target.modifier_index[slot] = modifier;
        target.modifier_value[slot] = new_value;
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.flags.insert(CharacterFlags::ITEMS);
        }
        true
    }

    fn apply_anti_enchant_cursor_item(
        &mut self,
        anti_orb_item_id: ItemId,
        character_id: CharacterId,
        target_item_id: ItemId,
        modifier: i16,
        amount: i16,
    ) -> bool {
        if amount <= 0 || !self.character_holds_cursor_item(character_id, target_item_id) {
            return false;
        }
        if matches!(modifier, x if x == CharacterValue::Armor as i16 || x == CharacterValue::Weapon as i16)
        {
            return false;
        }

        let Some(target) = self.items.get(&target_item_id) else {
            return false;
        };
        if !target.flags.intersects(ItemFlags::WEAR) || target.flags.contains(ItemFlags::NOENHANCE)
        {
            return false;
        }
        let Some(slot) = modifier_slot_with_positive_value(target, modifier) else {
            return false;
        };

        if !self.destroy_item(anti_orb_item_id) {
            return false;
        }
        let Some(target) = self.items.get_mut(&target_item_id) else {
            return false;
        };
        let new_value = target.modifier_value[slot] - amount;
        if new_value <= 0 {
            target.modifier_index[slot] = 0;
            target.modifier_value[slot] = 0;
        } else {
            target.modifier_value[slot] = new_value;
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.flags.insert(CharacterFlags::ITEMS);
        }
        true
    }

    fn apply_shrike_amulet_assemble(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
    ) -> bool {
        if !self.character_holds_cursor_item(character_id, cursor_item_id) {
            return false;
        }
        if !self.items.contains_key(&cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.carried_by != Some(character_id) {
            return false;
        }
        item.driver_data.resize(1, 0);
        item.driver_data[0] = combined_bits;
        item.sprite = 51617 + i32::from(combined_bits);
        match combined_bits {
            3 => {
                item.name = "Crystal on Chain".to_string();
                item.description = "A light blue crystal on a silver chain.".to_string();
            }
            5 => {
                item.name = "Crystal on Charm".to_string();
                item.description = "A light blue crystal on a silver crescent charm.".to_string();
            }
            6 => {
                item.name = "Charm on Chain".to_string();
                item.description = "A silver crescent charm on a silver chain.".to_string();
            }
            7 => {
                item.name = "Talisman".to_string();
                item.description = "A silver talisman.".to_string();
            }
            _ => {}
        }
        self.destroy_item(cursor_item_id)
    }

    fn apply_mine_gateway_key_assemble(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
    ) -> bool {
        const IID_MINEGATEWAY: u32 = (0x01 << 24) | 0x000098;

        if !self.character_holds_cursor_item(character_id, cursor_item_id) {
            return false;
        }
        if !self.items.contains_key(&cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(1, 0);
        item.driver_data[0] = combined_bits;
        item.description = "A partially assembled key.".to_string();
        item.sprite = match combined_bits {
            1 => 52201,
            2 => 52202,
            3 => 52205,
            4 => 52203,
            5 => 52206,
            6 => 52209,
            7 => 52213,
            8 => 52204,
            9 => 52210,
            10 => 52207,
            11 => 52212,
            12 => 52208,
            13 => 52214,
            14 => 52211,
            15 => {
                item.flags.remove(ItemFlags::USE);
                item.template_id = IID_MINEGATEWAY;
                item.name = "Mine gateway key".to_string();
                item.description = "A fully assembled key.".to_string();
                52200
            }
            _ => item.sprite,
        };
        self.destroy_item(cursor_item_id)
    }

    fn apply_palace_key_combine(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_sprite: i32,
        final_key: bool,
    ) -> bool {
        if !self.character_holds_cursor_item(character_id, cursor_item_id) {
            return false;
        }
        if !self.items.contains_key(&cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.carried_by != Some(character_id) {
            return false;
        }

        item.sprite = result_sprite;
        if final_key {
            item.template_id = crate::item_driver::IID_AREA11_PALACEKEY;
            item.driver = 0;
            item.flags.remove(ItemFlags::USE);
            item.name = "Palace Key".to_string();
            item.description = "The key to the ice palace.".to_string();
        }
        self.destroy_item(cursor_item_id)
    }

    pub fn apply_torch_extract_orb(
        &mut self,
        torch_item_id: ItemId,
        character_id: CharacterId,
        modifier_slot: usize,
        mut orb: Item,
    ) -> bool {
        let Some(torch) = self.items.get(&torch_item_id) else {
            return false;
        };
        if torch.carried_by != Some(character_id)
            || modifier_slot >= torch.modifier_value.len()
            || torch.modifier_value[modifier_slot] <= 0
        {
            return false;
        }

        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        match give_item_to_character(
            character,
            &mut orb,
            GiveItemFlags::LOG.union(GiveItemFlags::ALLOW_DROP),
        ) {
            GiveItemResult::Ok => {}
            GiveItemResult::Dropped => {
                if !self.map.drop_item_extended(
                    &mut orb,
                    usize::from(character.x),
                    usize::from(character.y),
                    1,
                ) {
                    return false;
                }
            }
            GiveItemResult::Money => {}
            GiveItemResult::Full | GiveItemResult::Failed => return false,
        }

        let Some(torch) = self.items.get_mut(&torch_item_id) else {
            return false;
        };
        torch.modifier_value[modifier_slot] -= 1;
        self.add_item(orb);
        true
    }

    fn character_holds_cursor_item(&self, character_id: CharacterId, item_id: ItemId) -> bool {
        self.characters
            .get(&character_id)
            .is_some_and(|character| character.cursor_item == Some(item_id))
    }

    fn schedule_item_driver_timer(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        after_ticks: u64,
    ) -> bool {
        self.schedule_item_driver_timer_with_context(item_id, character_id, after_ticks, true)
    }

    fn schedule_item_driver_timer_with_context(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        after_ticks: u64,
        timer_call: bool,
    ) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.driver == 0 {
            return false;
        }
        self.timers.set_timer(
            self.tick.0.saturating_add(after_ticks),
            ITEM_DRIVER_TIMER,
            TimerPayload([
                i32::from(item.driver),
                item_id.0 as i32,
                character_id.0 as i32,
                if timer_call { 1 } else { 0 },
                0,
            ]),
        )
    }

    fn schedule_map_item_driver_timer(
        &mut self,
        x: usize,
        y: usize,
        character_id: CharacterId,
        after_ticks: u64,
    ) -> bool {
        let Some(target_item_id) = self
            .map
            .tile(x, y)
            .and_then(|tile| (tile.item != 0).then_some(ItemId(u32::from(tile.item))))
        else {
            return false;
        };
        self.schedule_item_driver_timer_with_context(
            target_item_id,
            character_id,
            after_ticks,
            character_id.0 == 0,
        )
    }

    fn discover_steptrap_target(&mut self, item_id: ItemId) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.driver != IDR_STEPTRAP || item.driver_data.first().copied().unwrap_or(0) != 0 {
            return false;
        }

        let origin_x = usize::from(item.x);
        let origin_y = usize::from(item.y);
        let target = [1_u8, 3, 5, 7].into_iter().find_map(|dir| {
            let direction = Direction::try_from(dir).ok()?;
            let (dx, dy) = direction.delta();
            [1_i16, 2].into_iter().find_map(|distance| {
                let x = offset_coordinate(origin_x, dx * distance)?;
                let y = offset_coordinate(origin_y, dy * distance)?;
                if !self.map.legacy_inner_bounds(x, y) {
                    return None;
                }
                let target_item_id = self.map.tile(x, y)?.item;
                let target_item = self.items.get(&ItemId(u32::from(target_item_id)))?;
                (target_item.driver != 0 && target_item.driver != IDR_STEPTRAP).then_some((x, y))
            })
        });

        let Some((x, y)) = target else {
            return false;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(2, 0);
        item.driver_data[0] = x as u8;
        item.driver_data[1] = y as u8;
        true
    }

    fn mark_flamethrower_targets_for_burn(&mut self, item_id: ItemId, direction: u8) {
        let Some(item) = self.items.get(&item_id) else {
            return;
        };
        let Some(direction) = Direction::try_from(direction).ok() else {
            return;
        };
        let (dx, dy) = direction.delta();
        let origin_x = usize::from(item.x);
        let origin_y = usize::from(item.y);

        for distance in [1_i16, 2] {
            let Some(x) = offset_coordinate(origin_x, dx * distance) else {
                continue;
            };
            let Some(y) = offset_coordinate(origin_y, dy * distance) else {
                continue;
            };
            if !self.map.legacy_inner_bounds(x, y) {
                continue;
            }
            let Some(character_id) = self.map.tile(x, y).and_then(|tile| {
                (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
            }) else {
                continue;
            };
            self.burn_character(character_id);
        }
    }

    pub fn burn_character(&mut self, character_id: CharacterId) -> bool {
        if self.effects.values().any(|effect| {
            effect.effect_type == EF_BURN && effect.target_character == Some(character_id)
        }) {
            return false;
        }

        let effect_id = self.next_effect_id();
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let mut effect = Effect::new(
            EF_BURN,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 60) as i32,
        );
        effect.light = 250;
        effect.strength = 1;
        effect.target_character = Some(character_id);
        effect.x = i32::from(character.x);
        effect.y = i32::from(character.y);
        character.hp = character.hp.saturating_sub(20 * POWERSCALE);
        character.flags.insert(CharacterFlags::UPDATE);
        self.effects.insert(effect_id, effect);
        true
    }

    pub fn remove_character_burn_effect(&mut self, character_id: CharacterId) -> bool {
        let Some(effect_id) = self.effects.iter().find_map(|(&effect_id, effect)| {
            (effect.effect_type == EF_BURN && effect.target_character == Some(character_id))
                .then_some(effect_id)
        }) else {
            return false;
        };
        self.remove_effect_from_map(effect_id);
        self.effects.remove(&effect_id);
        true
    }

    fn destroy_item(&mut self, item_id: ItemId) -> bool {
        let Some(mut item) = self.items.remove(&item_id) else {
            return false;
        };

        if let Some(character_id) = item.carried_by {
            if let Some(character) = self.characters.get_mut(&character_id) {
                if character.cursor_item == Some(item_id) {
                    character.cursor_item = None;
                }
                for slot in &mut character.inventory {
                    if *slot == Some(item_id) {
                        *slot = None;
                    }
                }
                character.flags.insert(CharacterFlags::ITEMS);
            }
        }

        if item.x != 0 {
            self.map.remove_item_map(&mut item);
        }
        true
    }

    fn consume_city_recall_scroll(&mut self, character_id: CharacterId, item_id: ItemId) {
        let Some(item) = self.items.get_mut(&item_id) else {
            return;
        };
        item.driver_data.resize(2, 0);
        if item.driver_data[1] > 1 {
            item.driver_data[1] -= 1;
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.flags.insert(CharacterFlags::ITEMS);
            }
            return;
        }

        if let (Some(character), Some(item)) = (
            self.characters.get_mut(&character_id),
            self.items.get_mut(&item_id),
        ) {
            consume_item(character, item);
        }
    }

    fn toggle_door(&mut self, item_id: ItemId, character_id: CharacterId) -> DoorToggleResult {
        let Some(item) = self.items.get(&item_id) else {
            return DoorToggleResult::Failed;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let is_open = item.driver_data.first().copied().unwrap_or_default() != 0;

        if x == 0 || y == 0 {
            return DoorToggleResult::Failed;
        }
        let Some(tile) = self.map.tile(x, y) else {
            return DoorToggleResult::Failed;
        };
        if tile.item != item_id.0 {
            return DoorToggleResult::Failed;
        }
        if is_open
            && tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        {
            if character_id.0 == 0 {
                let should_retry = self.items.get_mut(&item_id).is_some_and(|item| {
                    item.driver_data.resize(40, 0);
                    item.driver_data[39] = item.driver_data[39].saturating_add(1);
                    item.driver_data[5] == 0
                });
                if should_retry {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
                }
            }
            return DoorToggleResult::Blocked;
        }

        let mut schedule_auto_close = false;
        let extended_door = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return DoorToggleResult::Failed;
            };
            item.driver_data.resize(40, 0);
            let Some(tile) = self.map.tile_mut(x, y) else {
                return DoorToggleResult::Failed;
            };

            if is_open {
                let restored = door_stored_flags(item);
                item.flags.insert(restored);
                apply_door_tile_flags(tile, item.flags);
                item.driver_data[0] = 0;
                item.sprite -= 1;
            } else {
                let stored = item.flags
                    & (ItemFlags::MOVEBLOCK
                        | ItemFlags::SIGHTBLOCK
                        | ItemFlags::DOOR
                        | ItemFlags::SOUNDBLOCK);
                store_door_flags(item, stored);
                item.flags.remove(
                    ItemFlags::MOVEBLOCK
                        | ItemFlags::SIGHTBLOCK
                        | ItemFlags::DOOR
                        | ItemFlags::SOUNDBLOCK,
                );
                tile.flags.remove(
                    MapFlags::TMOVEBLOCK
                        | MapFlags::TSIGHTBLOCK
                        | MapFlags::DOOR
                        | MapFlags::TSOUNDBLOCK,
                );
                item.driver_data[0] = 1;
                item.sprite += 1;
                item.driver_data[39] = item.driver_data[39].saturating_add(1);
                schedule_auto_close = item.driver_data[5] == 0;
            }

            item.driver_data[7] != 0
        };

        if schedule_auto_close {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
        }

        if extended_door {
            self.shift_extended_door_foregrounds(x, y, if is_open { -1 } else { 1 });
        }

        DoorToggleResult::Toggled
    }

    fn shift_extended_door_foregrounds(&mut self, x: usize, y: usize, delta: i32) {
        for (tile_x, tile_y) in [
            (x.saturating_add(1), y),
            (x.saturating_sub(1), y),
            (x, y.saturating_add(1)),
            (x, y.saturating_sub(1)),
        ] {
            let Some(tile) = self.map.tile_mut(tile_x, tile_y) else {
                continue;
            };
            if tile.foreground_sprite == 0 {
                continue;
            }
            if delta.is_negative() {
                tile.foreground_sprite =
                    tile.foreground_sprite.saturating_sub(delta.unsigned_abs());
            } else {
                tile.foreground_sprite = tile.foreground_sprite.saturating_add(delta as u32);
            }
        }
    }

    fn toggle_double_door(&mut self, item_id: ItemId, character_id: CharacterId) -> bool {
        let mut toggled = self.toggle_door(item_id, character_id) == DoorToggleResult::Toggled;
        let Some((x, y, open_state)) = self.items.get(&item_id).map(|item| {
            (
                usize::from(item.x),
                usize::from(item.y),
                door_open_state(item),
            )
        }) else {
            return toggled;
        };
        if x == 0 || y == 0 {
            return toggled;
        }

        for (adjacent_x, adjacent_y) in [
            (x, y.saturating_add(1)),
            (x, y.saturating_sub(1)),
            (x.saturating_add(1), y),
            (x.saturating_sub(1), y),
        ] {
            let Some(adjacent_item_id) = self
                .map
                .tile(adjacent_x, adjacent_y)
                .and_then(|tile| (tile.item != 0).then_some(ItemId(tile.item)))
            else {
                continue;
            };
            let Some(adjacent_item) = self.items.get(&adjacent_item_id) else {
                continue;
            };
            if door_open_state(adjacent_item) != open_state {
                toggled |=
                    self.toggle_door(adjacent_item_id, character_id) == DoorToggleResult::Toggled;
            }
        }

        toggled
    }

    fn teleport_character(
        &mut self,
        character_id: CharacterId,
        x: u16,
        y: u16,
        extended: bool,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let old_x = usize::from(character.x);
        let old_y = usize::from(character.y);
        let before = character.clone();
        remove_character_light(&mut self.map, character);
        self.map.remove_char(character);
        character.action = 0;
        character.step = 0;
        character.duration = 0;
        let placed = if extended {
            self.map
                .drop_char_extended(character, usize::from(x), usize::from(y), 6)
        } else {
            self.map
                .drop_char(character, usize::from(x), usize::from(y))
        };
        if !placed {
            let _ = self.map.drop_char(character, old_x, old_y);
            add_character_light(&mut self.map, character);
            let after = character.clone();
            self.mark_character_light_area(&before);
            self.mark_character_light_area(&after);
            return false;
        }
        add_character_light(&mut self.map, character);
        let after = character.clone();
        self.mark_character_light_area(&before);
        self.mark_character_light_area(&after);
        true
    }

    pub fn apply_player_action_setup(&mut self, player: &mut PlayerRuntime, area_id: u16) -> bool {
        let Some(character_id) = player.character_id else {
            return false;
        };

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

    fn setup_fireball_character(
        &mut self,
        caster_id: CharacterId,
        target_id: CharacterId,
        target_serial: u32,
    ) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        if !target.flags.contains(CharacterFlags::USED) {
            return false;
        }
        if target_serial != 0 && target.id.0 != target_serial {
            return false;
        }

        let (target_x, target_y) = predicted_fireball_target(&caster, &target);
        let current_tick = self.tick.0 as u32;
        self.characters.get_mut(&caster_id).is_some_and(|caster| {
            do_fireball(caster, &self.items, target_x, target_y, current_tick).is_ok()
        })
    }

    fn setup_ball_character(
        &mut self,
        caster_id: CharacterId,
        target_id: CharacterId,
        target_serial: u32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        if !target.flags.contains(CharacterFlags::USED) {
            return false;
        }
        if target_serial != 0 && target.id.0 != target_serial {
            return false;
        }

        let current_tick = self.tick.0 as u32;
        self.characters.get_mut(&caster_id).is_some_and(|caster| {
            do_ball(
                caster,
                &self.items,
                usize::from(target.x),
                usize::from(target.y),
                current_tick,
            )
            .is_ok()
        })
    }

    fn setup_bless_spell(&mut self, caster_id: CharacterId, target_id: CharacterId) -> bool {
        if caster_id == target_id {
            let Some(target) = self.characters.get(&target_id).cloned() else {
                return false;
            };
            let current_tick = self.tick.0 as u32;
            return self.characters.get_mut(&caster_id).is_some_and(|caster| {
                do_bless(caster, &target, &self.items, current_tick, None).is_ok()
            });
        }

        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(caster) = self.characters.get(&caster_id) else {
            return false;
        };
        let Some(direction) = offset_to_direction(
            usize::from(caster.x),
            usize::from(caster.y),
            usize::from(target.x),
            usize::from(target.y),
        ) else {
            return false;
        };
        let current_tick = self.tick.0 as u32;

        self.characters.get_mut(&caster_id).is_some_and(|caster| {
            do_bless(
                caster,
                &target,
                &self.items,
                current_tick,
                Some(direction as u8),
            )
            .is_ok()
        })
    }

    fn setup_heal_spell(&mut self, caster_id: CharacterId, target_id: CharacterId) -> bool {
        if caster_id == target_id {
            let Some(target) = self.characters.get(&target_id).cloned() else {
                return false;
            };
            return self
                .characters
                .get_mut(&caster_id)
                .is_some_and(|caster| do_heal(caster, &target, None).is_ok());
        }

        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(caster) = self.characters.get(&caster_id) else {
            return false;
        };
        let Some(direction) = offset_to_direction(
            usize::from(caster.x),
            usize::from(caster.y),
            usize::from(target.x),
            usize::from(target.y),
        ) else {
            return false;
        };

        self.characters
            .get_mut(&caster_id)
            .is_some_and(|caster| do_heal(caster, &target, Some(direction as u8)).is_ok())
    }

    fn setup_give(
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

    fn transfer_cursor_item(&mut self, giver_id: CharacterId, receiver_id: CharacterId) -> bool {
        if giver_id == receiver_id {
            return false;
        }
        let Some(giver) = self.characters.get(&giver_id) else {
            return false;
        };
        let Some(receiver) = self.characters.get(&receiver_id) else {
            return false;
        };
        if receiver
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

        let Some(receiver) = self.characters.get_mut(&receiver_id) else {
            return false;
        };
        if receiver.cursor_item.is_none() {
            receiver.cursor_item = Some(item_id);
        } else if receiver.flags.contains(CharacterFlags::PLAYER) {
            let Some(slot) = receiver
                .inventory
                .iter_mut()
                .skip(INVENTORY_START_INVENTORY)
                .find(|slot| slot.is_none())
            else {
                return false;
            };
            *slot = Some(item_id);
        } else {
            return false;
        }
        receiver.flags.insert(CharacterFlags::ITEMS);

        let Some(giver) = self.characters.get_mut(&giver_id) else {
            return false;
        };
        if giver.cursor_item != Some(item_id) {
            return false;
        }
        giver.cursor_item = None;
        giver.flags.insert(CharacterFlags::ITEMS);

        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.carried_by = Some(receiver_id);
        true
    }

    fn setup_player_move(
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

    fn setup_walk_toward(
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
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        do_walk(character, &mut self.map, direction as u8, area_id).is_ok()
    }

    fn setup_walk_toward_use_item(
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

    fn set_player_idle(&mut self, player: &mut PlayerRuntime, character_id: CharacterId) -> bool {
        player.action.action = PlayerActionCode::Idle;
        self.characters
            .get_mut(&character_id)
            .is_some_and(|character| do_idle(character, 4).is_ok())
    }

    pub fn tick_basic_actions(&mut self) -> Vec<WorldActionCompletion> {
        let character_ids: Vec<CharacterId> = self.characters.keys().copied().collect();
        let mut completed = Vec::new();

        for character_id in character_ids {
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
                        self.complete_attack_with_rolls(
                            character_id,
                            defender_id,
                            d100_roll,
                            d6_roll,
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
                action::PULSE => self.complete_pulse(character_id),
                action::FIREBALL1 => self.complete_fireball(character_id),
                action::FIREBALL2 => true,
                action::BALL1 => self.complete_ball(character_id),
                action::BALL2 => true,
                action::EARTHRAIN => self.complete_earthrain(character_id),
                action::EARTHMUD => self.complete_earthmud(character_id),
                action::FIRERING => self.complete_firering(character_id),
                action::FREEZE => self.complete_freeze(character_id),
                action::FLASH => self.complete_flash(character_id),
                action::WARCRY => self.complete_warcry(character_id),
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

impl World {
    fn complete_bless(&mut self, caster_id: CharacterId, target_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }
        if caster.act1 != target_id.0 as i32 {
            return false;
        }
        let strength = character_value(&caster, CharacterValue::Bless);
        if strength <= 0 {
            return false;
        }
        let duration = spell_duration_ticks(&caster, BLESS_DURATION);
        self.install_bless_spell(target_id, strength, duration)
    }

    fn complete_flash(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }
        let duration = spell_duration_ticks(&caster, FLASH_DURATION);
        if !self.install_speed_spell(caster_id, IDR_FLASH, "Flash", 100, duration) {
            return false;
        }
        self.create_show_effect(
            EF_FLASH,
            caster_id,
            self.tick.0 as u32,
            self.tick.0.saturating_add(duration.max(0) as u64) as u32,
            50,
            spell_power(
                character_value(&caster, CharacterValue::Flash),
                character_value(&caster, CharacterValue::Tactics),
            ),
        );
        true
    }

    fn complete_fireball(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        self.create_fireball_effect(&caster);
        if let Some(caster) = self.characters.get_mut(&caster_id) {
            caster.action = action::FIREBALL2;
            caster.step = 0;
        }
        true
    }

    fn complete_ball(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        self.create_ball_effect(&caster);
        if let Some(caster) = self.characters.get_mut(&caster_id) {
            caster.action = action::BALL2;
            caster.step = 0;
        }
        true
    }

    fn complete_earthrain(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.act1 <= 0 {
            return false;
        }
        self.create_earthrain_effect(
            caster.act1 % MAX_MAP as i32,
            caster.act1 / MAX_MAP as i32,
            caster.act2,
        ) != 0
    }

    fn complete_earthmud(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.act1 <= 0 {
            return false;
        }
        self.create_earthmud_effect(
            caster.act1 % MAX_MAP as i32,
            caster.act1 / MAX_MAP as i32,
            caster.act2,
        ) != 0
    }

    fn complete_firering(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let power = spell_power(
            character_value(&caster, CharacterValue::Fireball),
            character_value(&caster, CharacterValue::Tactics),
        );
        if !self.install_firering_spell(caster_id) {
            return false;
        }
        self.create_show_effect(
            EF_FIRERING,
            caster_id,
            self.tick.0 as u32,
            self.tick.0.saturating_add(7) as u32,
            50,
            20,
        );

        let caster_x = usize::from(caster.x);
        let caster_y = usize::from(caster.y);
        let min_x = caster_x.saturating_sub(1).max(1);
        let max_x = caster_x
            .saturating_add(1)
            .min(self.map.width().saturating_sub(2));
        let min_y = caster_y.saturating_sub(1).max(1);
        let max_y = caster_y
            .saturating_add(1)
            .min(self.map.height().saturating_sub(2));
        let mut targets = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                    (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                }) else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack(&caster, target, &self.map) {
                    continue;
                }
                let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
                let damage = fireball_damage(
                    power,
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    has_tactics,
                );
                targets.push((target_id, damage));
            }
        }

        for (target_id, damage) in targets {
            if let Some(target) = self.characters.get_mut(&target_id) {
                target.hp = target.hp.saturating_sub(damage);
                target.flags.insert(CharacterFlags::UPDATE);
            }
        }

        true
    }

    fn complete_magicshield(&mut self, character_id: CharacterId) -> bool {
        if !self
            .characters
            .get_mut(&character_id)
            .is_some_and(act_magicshield)
        {
            return false;
        }
        self.create_show_effect(
            EF_MAGICSHIELD,
            character_id,
            self.tick.0 as u32,
            self.tick.0.saturating_add(3) as u32,
            25,
            0,
        );
        true
    }

    fn complete_pulse(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let caster_x = usize::from(caster.x);
        let caster_y = usize::from(caster.y);
        let min_x = caster_x.saturating_sub(2).max(1);
        let max_x = caster_x
            .saturating_add(2)
            .min(self.map.width().saturating_sub(2));
        let min_y = caster_y.saturating_sub(2).max(1);
        let max_y = caster_y
            .saturating_add(2)
            .min(self.map.height().saturating_sub(2));
        let mut targets = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                    (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                }) else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack(&caster, target, &self.map) {
                    continue;
                }
                if !self.map.can_see(caster_x, caster_y, x, y, DIST_MAX) {
                    continue;
                }
                let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
                let damage = pulse_damage(
                    character_value(&caster, CharacterValue::Pulse),
                    caster.act1,
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    has_tactics,
                );
                let had = target.hp.saturating_add(target.lifeshield);
                let total = character_value(target, CharacterValue::Hp) * POWERSCALE
                    + character_value(target, CharacterValue::MagicShield) * POWERSCALE
                    + 1;
                if had.saturating_mul(100) / total <= 75 && damage >= had {
                    targets.push((target_id, damage, had));
                }
            }
        }

        for (target_id, damage, had) in targets {
            self.create_pulseback_effect(target_id, caster.x, caster.y, caster.act1);
            if let Some(caster) = self.characters.get_mut(&caster_id) {
                let max_mana = character_value(caster, CharacterValue::Mana) * POWERSCALE;
                caster.mana = max_mana.min(caster.mana.saturating_add(damage.min(had)));
                caster.flags.insert(CharacterFlags::UPDATE);
            }
            if let Some(target) = self.characters.get_mut(&target_id) {
                target.hp = target.hp.saturating_sub(damage);
                target.flags.insert(CharacterFlags::UPDATE);
            }
        }

        self.create_pulse_effect(
            caster.x,
            caster.y,
            character_value(&caster, CharacterValue::Pulse),
        );
        true
    }

    fn complete_freeze(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let caster_x = usize::from(caster.x);
        let caster_y = usize::from(caster.y);
        let min_x = caster_x.saturating_sub(3).max(1);
        let max_x = caster_x
            .saturating_add(3)
            .min(self.map.width().saturating_sub(2));
        let min_y = caster_y.saturating_sub(3).max(1);
        let max_y = caster_y
            .saturating_add(3)
            .min(self.map.height().saturating_sub(2));
        let mut targets = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                    (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                }) else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack(&caster, target, &self.map)
                    || !self.map.can_see(caster_x, caster_y, x, y, DIST_MAX)
                {
                    continue;
                }
                let modifier = freeze_speed_modifier(
                    spell_power(
                        character_value(&caster, CharacterValue::Freeze),
                        character_value(&caster, CharacterValue::Tactics),
                    ),
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    character_value_present(target, CharacterValue::Tactics) != 0,
                    caster.flags.contains(CharacterFlags::IDEMON),
                    character_value_present(&caster, CharacterValue::Demon),
                    character_value(target, CharacterValue::Cold),
                );
                if modifier < 0 {
                    targets.push((target_id, modifier));
                }
            }
        }

        let duration = spell_duration_ticks(&caster, FREEZE_DURATION);
        let mut installed = false;
        for (target_id, modifier) in targets {
            installed |=
                self.install_speed_spell(target_id, IDR_FREEZE, "Freeze", modifier, duration);
        }
        installed
    }

    fn complete_warcry(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let caster_x = usize::from(caster.x);
        let caster_y = usize::from(caster.y);
        let min_x = caster_x.saturating_sub(10).max(1);
        let max_x = caster_x
            .saturating_add(10)
            .min(self.map.width().saturating_sub(2));
        let min_y = caster_y.saturating_sub(10).max(1);
        let max_y = caster_y
            .saturating_add(10)
            .min(self.map.height().saturating_sub(2));
        let sectors = SoundSectors::build(&self.map);
        let power = spell_power(
            character_value(&caster, CharacterValue::Warcry),
            character_value(&caster, CharacterValue::Tactics),
        );
        let duration = spell_duration_ticks(&caster, WARCRY_DURATION);
        let mut targets = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                    (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                }) else {
                    continue;
                };
                if target_id == caster_id
                    || !sectors.sector_hear(&self.map, caster_x, caster_y, x, y)
                {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack(&caster, target, &self.map) {
                    continue;
                }

                let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
                let modifier = warcry_speed_modifier(
                    power,
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    has_tactics,
                );
                if modifier >= 0 {
                    continue;
                }
                let damage = warcry_damage(
                    power,
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    has_tactics,
                );
                targets.push((target_id, modifier, damage));
            }
        }

        let mut affected = false;
        for (target_id, modifier, damage) in targets {
            if !self.install_speed_spell(target_id, IDR_WARCRY, "Warcry", modifier, duration) {
                continue;
            }
            affected = true;
            if damage > 0 {
                if let Some(target) = self.characters.get_mut(&target_id) {
                    target.hp = target.hp.saturating_sub(damage);
                    target.flags.insert(CharacterFlags::UPDATE);
                }
            }
        }

        if character_value_present(&caster, CharacterValue::MagicShield) == 0 {
            if let Some(caster) = self.characters.get_mut(&caster_id) {
                let max_lifeshield = if character_value(caster, CharacterValue::MagicShield) != 0 {
                    character_value(caster, CharacterValue::MagicShield)
                } else {
                    character_value(caster, CharacterValue::Warcry)
                } * crate::entity::POWERSCALE;
                let gain =
                    character_value(caster, CharacterValue::Warcry) * crate::entity::POWERSCALE / 2;
                caster.lifeshield = max_lifeshield.min(caster.lifeshield + gain);
                caster.flags.insert(CharacterFlags::UPDATE);
            }
        }

        affected
    }

    fn install_bless_spell(
        &mut self,
        target_id: CharacterId,
        strength: i32,
        duration: i32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, IDR_BLESS, self.tick.0 as u32) else {
            return false;
        };
        let old_item_id = target.inventory.get(slot).copied().flatten();
        if let Some(item_id) = old_item_id {
            self.items.remove(&item_id);
            self.remove_show_effect_type(target_id, EF_BLESS);
        }

        let item_id = self.next_runtime_item_id();
        let mut driver_data = Vec::with_capacity(12);
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration.max(0) as u32);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());
        driver_data.extend_from_slice(&strength.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Bless".to_string(),
            description: "A Spell of Bless.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [
                CharacterValue::Intelligence as i16,
                CharacterValue::Wisdom as i16,
                CharacterValue::Agility as i16,
                CharacterValue::Strength as i16,
                0,
            ],
            modifier_value: [
                (strength / 4) as i16,
                (strength / 4) as i16,
                (strength / 4) as i16,
                (strength / 4) as i16,
                0,
            ],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_BLESS,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                return false;
            }
            target.inventory[slot] = Some(item_id);
            let character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
            self.create_show_effect(
                EF_BLESS,
                target_id,
                start_tick,
                expire_tick,
                0,
                strength / 4,
            );
            true
        } else {
            false
        }
    }

    fn install_beyond_potion_spell(
        &mut self,
        character_id: CharacterId,
        potion_item_id: ItemId,
        duration_minutes: u8,
        modifier_index: [i16; MAX_MODIFIERS],
        modifier_value: [i16; MAX_MODIFIERS],
        beyond_max_mod: bool,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&character, &self.items, IDR_POTION_SP, self.tick.0 as u32)
        else {
            return false;
        };
        if !self
            .items
            .get(&potion_item_id)
            .is_some_and(|item| item.carried_by == Some(character_id))
        {
            return false;
        }

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let duration_ticks = u32::from(duration_minutes) * 60 * TICKS_PER_SECOND as u32;
        let expire_tick = start_tick.wrapping_add(duration_ticks);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let mut flags = ItemFlags::USED;
        if beyond_max_mod {
            flags.insert(ItemFlags::BEYONDMAXMOD);
        }
        let item = Item {
            id: item_id,
            name: "Potion Spell".to_string(),
            description: "A potion spell.".to_string(),
            flags,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index,
            modifier_value,
            x: 0,
            y: 0,
            carried_by: Some(character_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_POTION_SP,
            driver_data,
            serial: item_id.0,
        };

        if !self.destroy_item(potion_item_id) {
            return false;
        }
        self.items.insert(item_id, item);
        if let Some(character) = self.characters.get_mut(&character_id) {
            if character.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            character.inventory[slot] = Some(item_id);
            let character_serial = character.id.0;
            character
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            self.schedule_spell_remove_timer(
                character_id,
                item_id,
                slot,
                character_serial,
                item_id.0,
            );
            self.create_show_effect(
                EF_POTION,
                character_id,
                start_tick,
                expire_tick,
                0,
                i32::from(modifier_value[0]),
            );
            true
        } else {
            self.items.remove(&item_id);
            false
        }
    }

    fn install_speed_spell(
        &mut self,
        target_id: CharacterId,
        driver: u16,
        name: &str,
        speed_modifier: i32,
        duration: i32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration.max(0) as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: name.to_string(),
            description: format!("A Spell of {name}."),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [CharacterValue::Speed as i16, 0, 0, 0, 0],
            modifier_value: [speed_modifier as i16, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            let character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
            match driver {
                IDR_FREEZE => {
                    self.create_show_effect(EF_FREEZE, target_id, start_tick, expire_tick, 0, 0);
                }
                IDR_WARCRY => {
                    self.create_show_effect(EF_WARCRY, target_id, start_tick, expire_tick, 0, 0);
                }
                _ => {}
            }
            true
        } else {
            self.items.remove(&item_id);
            false
        }
    }

    fn install_firering_spell(&mut self, target_id: CharacterId) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, IDR_FIRERING, self.tick.0 as u32)
        else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(crate::tick::TICKS_PER_SECOND as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Firering".to_string(),
            description: "A Spell of Firering.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0, 0, 0, 0, 0],
            modifier_value: [0, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_FIRERING,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            let character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
            true
        } else {
            self.items.remove(&item_id);
            false
        }
    }

    fn install_infravision_spell(&mut self, target_id: CharacterId) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, IDR_INFRARED, self.tick.0 as u32)
        else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add((TICKS_PER_SECOND * 60 * 10) as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Infravision".to_string(),
            description: "A Spell of Infravision.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0, 0, 0, 0, 0],
            modifier_value: [0, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_INFRARED,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            let character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
            true
        } else {
            self.items.remove(&item_id);
            false
        }
    }

    pub fn poison_character(
        &mut self,
        character_id: CharacterId,
        power: u16,
        poison_type: u16,
    ) -> bool {
        if poison_type > 3 {
            return false;
        }
        let driver = IDR_POISON0 + poison_type;
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&character, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(POISON_DURATION as u32);
        let mut driver_data = Vec::with_capacity(12);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());
        driver_data.extend_from_slice(&power.to_le_bytes());
        driver_data.extend_from_slice(&9_u16.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Poison".to_string(),
            description: "A Spell of Poison.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [CharacterValue::Hp as i16, 0, 0, 0, 0],
            modifier_value: [-1, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(character_id),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        if let Some(character) = self.characters.get_mut(&character_id) {
            if character.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            character.inventory[slot] = Some(item_id);
            let character_serial = character.id.0;
            character
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            self.schedule_spell_remove_timer(
                character_id,
                item_id,
                slot,
                character_serial,
                item_id.0,
            );
            self.schedule_poison_callback_timer(
                self.tick.0 + crate::tick::TICKS_PER_SECOND,
                character_id,
                item_id,
                slot,
                character_serial,
                item_id.0,
            );
            true
        } else {
            self.items.remove(&item_id);
            false
        }
    }

    pub fn remove_poison(&mut self, character_id: CharacterId, poison_type: u16) -> bool {
        if poison_type > 3 {
            return false;
        }
        self.remove_poison_by_driver(character_id, IDR_POISON0 + poison_type)
    }

    pub fn remove_all_poison(&mut self, character_id: CharacterId) -> bool {
        let mut removed = false;
        for driver in IDR_POISON0..=IDR_POISON3 {
            removed |= self.remove_poison_by_driver(character_id, driver);
        }
        removed
    }

    fn remove_poison_by_driver(&mut self, character_id: CharacterId, driver: u16) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let slots: Vec<(usize, ItemId)> = character
            .inventory
            .iter()
            .copied()
            .enumerate()
            .skip(crate::spell::SPELL_SLOT_START)
            .take(crate::spell::SPELL_SLOT_END - crate::spell::SPELL_SLOT_START)
            .filter_map(|(slot, item_id)| {
                let item_id = item_id?;
                self.items
                    .get(&item_id)
                    .is_some_and(|item| item.driver == driver)
                    .then_some((slot, item_id))
            })
            .collect();
        if slots.is_empty() {
            return false;
        }
        let character = self
            .characters
            .get_mut(&character_id)
            .expect("checked above");
        for (slot, item_id) in slots {
            character.inventory[slot] = None;
            self.items.remove(&item_id);
        }
        character
            .flags
            .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        true
    }

    pub fn schedule_existing_spell_timers(&mut self) -> usize {
        let mut spells = Vec::new();
        for (&character_id, character) in &self.characters {
            for (slot, item_id) in character.inventory.iter().copied().enumerate() {
                let Some(item_id) = item_id else {
                    continue;
                };
                let Some(item) = self.items.get(&item_id) else {
                    continue;
                };
                if !is_timed_spell_driver(item.driver) {
                    continue;
                }
                let Some(due) = read_spell_expire_tick(&item.driver_data) else {
                    continue;
                };
                spells.push((
                    character_id,
                    item_id,
                    slot,
                    character.id.0,
                    item.serial,
                    due as u64,
                    item.driver,
                    item.driver_data
                        .get(4..8)
                        .and_then(|bytes| Some(u32::from_le_bytes(bytes.try_into().ok()?))),
                    item.modifier_value[0],
                ));
            }
        }

        spells
            .into_iter()
            .filter(
                |&(
                    character_id,
                    item_id,
                    slot,
                    character_serial,
                    item_serial,
                    due,
                    driver,
                    start_tick,
                    modifier_value,
                )| {
                    let scheduled = self.set_spell_remove_timer(
                        due,
                        character_id,
                        item_id,
                        slot,
                        character_serial,
                        item_serial,
                    );
                    if scheduled {
                        if let Some(start_tick) = start_tick {
                            let stop_tick = due as u32;
                            match driver {
                                IDR_BLESS => {
                                    self.create_show_effect(
                                        EF_BLESS,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        i32::from(modifier_value),
                                    );
                                }
                                IDR_FREEZE => {
                                    self.create_show_effect(
                                        EF_FREEZE,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        0,
                                    );
                                }
                                IDR_WARCRY => {
                                    self.create_show_effect(
                                        EF_WARCRY,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        0,
                                    );
                                }
                                IDR_POTION_SP => {
                                    self.create_show_effect(
                                        EF_POTION,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        i32::from(modifier_value),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    scheduled
                },
            )
            .count()
    }

    fn schedule_spell_remove_timer(
        &mut self,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let Some(due) = read_spell_expire_tick(&item.driver_data) else {
            return false;
        };
        if !is_timed_spell_driver(item.driver) {
            return false;
        }
        self.set_spell_remove_timer(
            due as u64,
            character_id,
            item_id,
            slot,
            character_serial,
            item_serial,
        )
    }

    fn set_spell_remove_timer(
        &mut self,
        due: u64,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Ok(slot) = i32::try_from(slot) else {
            return false;
        };
        self.timers.set_timer(
            due,
            REMOVE_SPELL_TIMER,
            TimerPayload([
                character_id.0 as i32,
                item_id.0 as i32,
                slot,
                character_serial as i32,
                item_serial as i32,
            ]),
        )
    }

    fn schedule_poison_callback_timer(
        &mut self,
        due: u64,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Ok(slot) = i32::try_from(slot) else {
            return false;
        };
        self.timers.set_timer(
            due,
            POISON_CALLBACK_TIMER,
            TimerPayload([
                character_id.0 as i32,
                item_id.0 as i32,
                slot,
                character_serial as i32,
                item_serial as i32,
            ]),
        )
    }

    fn poison_callback_from_timer(
        &mut self,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if !character.flags.contains(CharacterFlags::USED) || character.id.0 != character_serial {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.serial != item_serial || !matches!(item.driver, IDR_POISON0..=IDR_POISON3) {
            return false;
        }
        if character.inventory.get(slot).copied().flatten() != Some(item_id) {
            return false;
        }
        let Some(mut power) = read_poison_power(&item.driver_data) else {
            return false;
        };
        let Some(mut tick) = read_poison_tick(&item.driver_data) else {
            return false;
        };
        power = power.clamp(1, 20);

        if tick == 0 {
            item.modifier_value[0] = item.modifier_value[0].saturating_sub(1).max(-1000);
            character.flags.insert(CharacterFlags::UPDATE);
        }
        character.hp = character.hp.saturating_sub(crate::entity::POWERSCALE / 3);
        character.flags.insert(CharacterFlags::UPDATE);

        tick = if tick == 0 { 9 } else { tick - 1 };
        write_poison_tick(&mut item.driver_data, tick);
        let due = self.tick.0 + (crate::tick::TICKS_PER_SECOND * 2 / u64::from(power));
        self.schedule_poison_callback_timer(
            due,
            character_id,
            item_id,
            slot,
            character_serial,
            item_serial,
        );
        true
    }

    fn remove_spell_from_timer(
        &mut self,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if !character.flags.contains(CharacterFlags::USED) || character.id.0 != character_serial {
            return false;
        }
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.serial != item_serial {
            return false;
        }
        if character.inventory.get(slot).copied().flatten() != Some(item_id) {
            return false;
        }

        character.inventory[slot] = None;
        character
            .flags
            .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        self.items.remove(&item_id);
        true
    }

    fn next_runtime_item_id(&self) -> ItemId {
        let next = self
            .items
            .keys()
            .map(|item_id| item_id.0)
            .max()
            .unwrap_or_default()
            .saturating_add(1)
            .max(1);
        ItemId(next)
    }

    fn complete_heal(&mut self, caster_id: CharacterId, target_id: CharacterId) -> bool {
        if caster_id == target_id {
            let Some(caster) = self.characters.get(&caster_id).cloned() else {
                return false;
            };
            if !self
                .characters
                .get_mut(&target_id)
                .is_some_and(|target| act_heal(&caster, target))
            {
                return false;
            }
            self.create_show_effect(
                EF_HEAL,
                target_id,
                self.tick.0 as u32,
                self.tick.0.saturating_add(8) as u32,
                0,
                0,
            );
            return true;
        }

        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if !self
            .characters
            .get_mut(&target_id)
            .is_some_and(|target| act_heal(&caster, target))
        {
            return false;
        }
        self.create_show_effect(
            EF_HEAL,
            target_id,
            self.tick.0 as u32,
            self.tick.0.saturating_add(8) as u32,
            0,
            0,
        );
        true
    }
}

fn read_poison_power(driver_data: &[u8]) -> Option<u16> {
    let bytes = driver_data.get(8..10)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

fn read_poison_tick(driver_data: &[u8]) -> Option<u16> {
    let bytes = driver_data.get(10..12)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

fn write_poison_tick(driver_data: &mut Vec<u8>, tick: u16) {
    driver_data.resize(12, 0);
    driver_data[10..12].copy_from_slice(&tick.to_le_bytes());
}

fn valid_map_coords(x: i32, y: i32) -> Option<(usize, usize)> {
    let x = usize::try_from(x).ok()?;
    let y = usize::try_from(y).ok()?;
    Some((x, y))
}

fn can_receive_given_item(character: &Character) -> bool {
    if character.cursor_item.is_none() {
        return true;
    }
    character.flags.contains(CharacterFlags::PLAYER)
        && character
            .inventory
            .iter()
            .skip(INVENTORY_START_INVENTORY)
            .any(|slot| slot.is_none())
}

fn character_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

fn character_value_present(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

fn spell_duration_ticks(character: &Character, base_duration: i32) -> i32 {
    if character_value_present(character, CharacterValue::Duration) != 0 {
        base_duration + base_duration * character_value(character, CharacterValue::Duration) / 35
    } else if character.flags.contains(CharacterFlags::ARCH) {
        base_duration + base_duration * character.level as i32 / 35 / 2
    } else {
        base_duration
    }
}

fn predicted_fireball_target(caster: &Character, target: &Character) -> (usize, usize) {
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

fn ball_target_damage_multiplier(enemy_count: i32) -> i32 {
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

fn adjacent_direction(from_x: u16, from_y: u16, to_x: usize, to_y: usize) -> Option<Direction> {
    match (
        to_x as i32 - i32::from(from_x),
        to_y as i32 - i32::from(from_y),
    ) {
        (1, 0) => Some(Direction::Right),
        (0, 1) => Some(Direction::Down),
        (-1, 0) => Some(Direction::Left),
        (0, -1) => Some(Direction::Up),
        _ => None,
    }
}

fn adjacent_use_direction(
    from_x: u16,
    from_y: u16,
    to_x: usize,
    to_y: usize,
    front_wall: bool,
) -> Option<Direction> {
    match (
        to_x as i32 - i32::from(from_x),
        to_y as i32 - i32::from(from_y),
    ) {
        (1, 0) if !front_wall => Some(Direction::Right),
        (0, 1) if !front_wall => Some(Direction::Down),
        (-1, 0) => Some(Direction::Left),
        (0, -1) => Some(Direction::Up),
        _ => None,
    }
}

fn item_in_facing_direction(character: &Character, map: &MapGrid) -> Option<(ItemId, Direction)> {
    let direction = Direction::try_from(character.dir).ok()?;
    let (dx, dy) = direction.delta();
    let x = offset_coordinate(usize::from(character.x), dx)?;
    let y = offset_coordinate(usize::from(character.y), dy)?;
    let item_id = map.tile(x, y)?.item;
    (item_id != 0).then_some((ItemId(item_id), direction))
}

fn offset_to_direction(
    from_x: usize,
    from_y: usize,
    to_x: usize,
    to_y: usize,
) -> Option<Direction> {
    let mut dx = to_x as i32 - from_x as i32;
    let mut dy = to_y as i32 - from_y as i32;

    if dx.abs() / 2 > dy.abs() {
        dy = 0;
    }
    if dy.abs() / 2 > dx.abs() {
        dx = 0;
    }

    match (dx.signum(), dy.signum()) {
        (1, 1) => Some(Direction::RightDown),
        (1, -1) => Some(Direction::RightUp),
        (1, 0) => Some(Direction::Right),
        (-1, 1) => Some(Direction::LeftDown),
        (-1, -1) => Some(Direction::LeftUp),
        (-1, 0) => Some(Direction::Left),
        (0, 1) => Some(Direction::Down),
        (0, -1) => Some(Direction::Up),
        _ => None,
    }
}

fn offset_coordinate(value: usize, offset: i16) -> Option<usize> {
    if offset.is_negative() {
        value.checked_sub(offset.unsigned_abs() as usize)
    } else {
        value.checked_add(offset as usize)
    }
}

fn diagonal_slide_alternates(direction: u8) -> Option<(Direction, Direction)> {
    match Direction::try_from(direction).ok()? {
        Direction::LeftUp => Some((Direction::Left, Direction::Up)),
        Direction::RightUp => Some((Direction::Right, Direction::Up)),
        Direction::LeftDown => Some((Direction::Left, Direction::Down)),
        Direction::RightDown => Some((Direction::Right, Direction::Down)),
        _ => None,
    }
}

fn door_stored_flags(item: &Item) -> ItemFlags {
    let mut bytes = [0; 8];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        *byte = item
            .driver_data
            .get(30 + offset)
            .copied()
            .unwrap_or_default();
    }
    ItemFlags::from_bits_retain(u64::from_le_bytes(bytes))
}

fn door_open_state(item: &Item) -> bool {
    item.driver_data.first().copied().unwrap_or_default() != 0
}

fn current_modifier_value(item: &Item, modifier: i16) -> Option<i16> {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .find_map(|(index, value)| (*index == modifier).then_some(*value))
}

fn modifier_slot_for_write(item: &Item, modifier: i16) -> Option<usize> {
    item.modifier_index
        .iter()
        .position(|index| *index == modifier)
        .or_else(|| item.modifier_value.iter().position(|value| *value == 0))
}

fn modifier_slot_with_positive_value(item: &Item, modifier: i16) -> Option<usize> {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .position(|(index, value)| *index == modifier && *value > 0)
}

fn counted_enhancement_modifiers(item: &Item) -> usize {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .filter(|(index, value)| {
            **value > 0
                && **index >= 0
                && !matches!(
                    **index,
                    x if x == CharacterValue::Weapon as i16
                        || x == CharacterValue::Armor as i16
                        || x == CharacterValue::Demon as i16
                        || x == CharacterValue::Light as i16
                )
        })
        .count()
}

fn store_door_flags(item: &mut Item, flags: ItemFlags) {
    item.driver_data.resize(40, 0);
    item.driver_data[30..38].copy_from_slice(&flags.bits().to_le_bytes());
}

fn apply_door_tile_flags(tile: &mut crate::map::MapTile, item_flags: ItemFlags) {
    if item_flags.contains(ItemFlags::MOVEBLOCK) {
        tile.flags.insert(MapFlags::TMOVEBLOCK);
    }
    if item_flags.contains(ItemFlags::SIGHTBLOCK) {
        tile.flags.insert(MapFlags::TSIGHTBLOCK);
    }
    if item_flags.contains(ItemFlags::SOUNDBLOCK) {
        tile.flags.insert(MapFlags::TSOUNDBLOCK);
    }
    if item_flags.contains(ItemFlags::DOOR) {
        tile.flags.insert(MapFlags::DOOR);
    }
}

fn timer_callback_character() -> Character {
    Character {
        id: CharacterId(0),
        name: String::new(),
        description: String::new(),
        flags: CharacterFlags::empty(),
        sprite: 0,
        speed_mode: SpeedMode::Normal,
        x: 0,
        y: 0,
        rest_area: 0,
        rest_x: 0,
        rest_y: 0,
        tox: 0,
        toy: 0,
        dir: 0,
        action: 0,
        duration: 0,
        step: 0,
        act1: 0,
        act2: 0,
        hp: 0,
        mana: 0,
        endurance: 0,
        lifeshield: 0,
        level: 0,
        exp: 0,
        exp_used: 0,
        gold: 0,
        creation_time: 0,
        saves: 0,
        deaths: 0,
        cursor_item: None,
        current_container: None,
        values: Character::empty_values(),
        professions: Character::empty_professions(),
        inventory: Character::empty_inventory(),
        driver_state: None,
        driver_messages: Vec::new(),
    }
}

impl Default for Tick {
    fn default() -> Self {
        Self(0)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        direction::Direction,
        entity::{CharacterFlags, CharacterValue, ItemFlags, SpeedMode, MAX_MODIFIERS, POWERSCALE},
        item_driver::{
            UseItemOutcome, IDR_ANTIENCHANTITEM, IDR_BALLTRAP, IDR_DOOR, IDR_ENCHANTITEM,
            IDR_FLAMETHROW, IDR_NIGHTLIGHT, IDR_PALACEKEY, IDR_SPECIAL_POTION, IDR_SPIKETRAP,
            IDR_STEPTRAP, IDR_TORCH, IDR_USETRAP,
        },
        legacy::action,
        map::MapFlags,
        player::{PlayerActionCode, PlayerRuntime, QueuedAction},
        spell::{IDR_INFRARED, IDR_POISON2},
        tick::TICKS_PER_SECOND,
    };

    use super::*;

    #[test]
    fn world_advances_and_resets_character_action_steps() {
        let mut world = World::default();
        let mut character = character(1);
        character.duration = 2;
        character.action = action::WALK;
        world.add_character(character);

        assert_eq!(world.advance_character_action(CharacterId(1)), Some(false));
        assert_eq!(world.advance_character_action(CharacterId(1)), Some(true));
        assert!(world.reset_character_action(CharacterId(1)));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, 0);
        assert_eq!(character.duration, 0);
        assert_eq!(character.step, 0);
    }

    #[test]
    fn sound_area_specials_match_legacy_distance_and_pan() {
        let mut world = World {
            map: MapGrid::new(40, 40),
            ..World::default()
        };
        let mut nearby = character(1);
        nearby.flags.insert(CharacterFlags::PLAYER);
        nearby.x = 13;
        nearby.y = 14;
        let mut outside = character(2);
        outside.flags.insert(CharacterFlags::PLAYER);
        outside.x = 31;
        outside.y = 10;
        let mut npc = character(3);
        npc.x = 12;
        npc.y = 10;

        world.add_character(nearby);
        world.add_character(outside);
        world.add_character(npc);

        let specials = world.sound_area_specials(10, 10, 7);

        assert_eq!(specials.len(), 1);
        assert_eq!(specials[0].character_id, CharacterId(1));
        assert_eq!(specials[0].special.special_type, 7);
        assert_eq!(specials[0].special.opt1, -250);
        assert_eq!(specials[0].special.opt2, 300);
    }

    #[test]
    fn sound_area_talk_type_is_sound_sector_gated() {
        let mut world = World {
            map: MapGrid::new(12, 12),
            ..World::default()
        };
        for y in 0..12 {
            world.map.set_flags(6, y, MapFlags::SOUNDBLOCK);
        }
        let mut listener = character(1);
        listener.flags.insert(CharacterFlags::PLAYER);
        listener.x = 8;
        listener.y = 4;
        world.add_character(listener);

        assert!(world
            .sound_area_specials(4, 4, u32::from(LOG_TALK))
            .is_empty());
        assert_eq!(world.sound_area_specials(4, 4, 7).len(), 1);
    }

    #[test]
    fn world_groundlight_marks_dirty_sector_when_light_changes() {
        let mut world = World {
            tick: Tick(17),
            map: MapGrid::new(24, 24),
            ..World::default()
        };
        world.map.tile_mut(10, 10).unwrap().ground_sprite = 14361;

        assert!(world.compute_groundlight_at(10, 10));
        assert_eq!(world.map.tile(10, 10).unwrap().light, 64);
        assert_eq!(world.skip_x_sector(10, 10, 17), 0);
        assert!(!world.compute_groundlight_at(40, 40));
    }

    #[test]
    fn world_shadow_marks_dirty_sector_only_on_daylight_change() {
        let mut world = World {
            tick: Tick(23),
            map: MapGrid::new(24, 24),
            ..World::default()
        };

        assert!(world.compute_shadow_at(10, 10));
        assert_eq!(world.map.tile(10, 10).unwrap().daylight, 63);
        assert_eq!(world.skip_x_sector(10, 10, 23), 0);
        assert!(!world.compute_shadow_at(10, 10));
    }

    #[test]
    fn world_dlight_wrappers_mark_changed_indoor_tiles_dirty() {
        let mut world = World {
            tick: Tick(31),
            map: MapGrid::new(32, 32),
            ..World::default()
        };
        world.map.set_flags(10, 10, MapFlags::INDOORS);

        assert!(world.compute_dlight_at(10, 10));
        assert_eq!(world.map.tile(10, 10).unwrap().daylight, 63);
        assert_eq!(world.skip_x_sector(10, 10, 31), 0);

        let mut world = World {
            tick: Tick(37),
            map: MapGrid::new(32, 32),
            ..World::default()
        };
        for y in 8..=10 {
            for x in 8..=10 {
                world.map.set_flags(x, y, MapFlags::INDOORS);
            }
        }

        assert!(world.reset_dlight_around(10, 10));
        assert!(world.map.tile(10, 10).unwrap().daylight > 0);
        assert_eq!(world.skip_x_sector(10, 10, 37), 0);
    }

    #[test]
    fn world_spawns_and_removes_character_on_map() {
        let mut world = World::default();

        assert!(world.spawn_character(character(1), 10, 10));
        assert_eq!(world.map.tile(10, 10).unwrap().character, 1);
        assert!(!world.spawn_character(character(1), 11, 10));

        let removed = world.remove_character(CharacterId(1)).unwrap();
        assert_eq!(removed.id, CharacterId(1));
        assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
    }

    #[test]
    fn world_reschedules_light_timer_after_lighting_torch() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        let mut torch = item(7, ItemFlags::USED | ItemFlags::USE);
        torch.carried_by = Some(CharacterId(1));
        torch.driver = IDR_TORCH;
        torch.driver_data = vec![0, 0, 10, 20];
        world.add_character(character);
        world.add_item(torch);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_TORCH,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::LightChanged { .. }));
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn world_updates_map_light_when_timer_driven_map_item_changes() {
        let mut world = World::default();
        world.date.daylight = 40;
        let mut nightlight = item(7, ItemFlags::USED);
        nightlight.driver = IDR_NIGHTLIGHT;
        nightlight.driver_data = vec![0, 12];
        nightlight.x = 10;
        nightlight.y = 10;
        world.map.tile_mut(10, 10).unwrap().item = 7;
        world.add_item(nightlight);
        assert_eq!(world.map.tile(10, 10).unwrap().light, 0);

        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
        world.advance();
        let outcomes = world.process_due_timers(1);

        assert!(matches!(
            outcomes[0],
            ItemDriverOutcome::LightChanged { .. }
        ));
        assert_eq!(world.map.tile(10, 10).unwrap().light, 12);
    }

    #[test]
    fn world_updates_map_light_when_lit_item_is_taken_and_dropped() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.dir = Direction::Right as u8;
        character.act1 = 7;
        let mut light_item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        light_item.x = 11;
        light_item.y = 10;
        light_item.modifier_index[0] = CharacterValue::Light as i16;
        light_item.modifier_value[0] = 16;
        world.map.tile_mut(11, 10).unwrap().item = 7;
        world.add_character(character);
        world.add_item(light_item);
        assert_eq!(world.map.tile(11, 10).unwrap().light, 16);

        assert!(world.complete_take(CharacterId(1), ItemId(7), true));
        assert_eq!(world.map.tile(11, 10).unwrap().light, 0);

        assert!(world.complete_drop(CharacterId(1), ItemId(7)));
        assert_eq!(world.map.tile(11, 10).unwrap().light, 16);
    }

    #[test]
    fn world_marks_dirty_sectors_for_lit_item_changes() {
        let mut world = World::default();
        let mut light_item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        light_item.x = 11;
        light_item.y = 10;
        light_item.modifier_index[0] = CharacterValue::Light as i16;
        light_item.modifier_value[0] = 16;
        world.map.tile_mut(11, 10).unwrap().item = 7;

        assert!(world.skip_x_sector(11, 10, 1) > 0);
        world.add_item(light_item);

        assert_eq!(world.skip_x_sector(11, 10, 1), 0);
        assert_eq!(world.skip_x_sector(12, 10, 1), 0);
        assert!(world.skip_x_sector(40, 40, 1) > 0);
    }

    #[test]
    fn world_updates_map_light_when_character_spawns_walks_and_leaves() {
        let mut world = World::default();
        let mut character = character(1);
        character.values[0][CharacterValue::Light as usize] = 16;

        assert!(world.spawn_character(character, 10, 10));
        assert_eq!(world.map.tile(10, 10).unwrap().light, 16);

        let character = world.characters.get_mut(&CharacterId(1)).unwrap();
        character.tox = 12;
        character.toy = 10;
        assert!(world.complete_walk(CharacterId(1)));
        assert_eq!(world.map.tile(10, 10).unwrap().light, 3);
        assert_eq!(world.map.tile(12, 10).unwrap().light, 16);
        assert!(world
            .map
            .tile(12, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));

        assert!(world.remove_character(CharacterId(1)).is_some());
        assert_eq!(world.map.tile(12, 10).unwrap().light, 0);
    }

    #[test]
    fn world_refreshes_character_light_after_value_change_without_stale_light() {
        let mut world = World::default();
        let mut character = character(1);
        character.values[0][CharacterValue::Light as usize] = 16;
        assert!(world.spawn_character(character, 10, 10));

        let old_light = world.characters[&CharacterId(1)].values[0][CharacterValue::Light as usize];
        world.characters.get_mut(&CharacterId(1)).unwrap().values[0]
            [CharacterValue::Light as usize] = 25;

        assert!(world.refresh_character_light_after_value_change(CharacterId(1), old_light));
        assert_eq!(world.map.tile(10, 10).unwrap().light, 25);
        assert!(world.characters[&CharacterId(1)]
            .flags
            .contains(CharacterFlags::UPDATE));

        let old_light = world.characters[&CharacterId(1)].values[0][CharacterValue::Light as usize];
        world.characters.get_mut(&CharacterId(1)).unwrap().values[0]
            [CharacterValue::Light as usize] = 0;

        assert!(world.refresh_character_light_after_value_change(CharacterId(1), old_light));
        assert_eq!(world.map.tile(10, 10).unwrap().light, 0);
    }

    #[test]
    fn world_marks_dirty_sectors_for_character_light_movement() {
        let mut world = World::default();
        let mut character = character(1);
        character.values[0][CharacterValue::Light as usize] = 16;

        assert!(world.spawn_character(character, 10, 10));
        assert_eq!(world.skip_x_sector(10, 10, 1), 0);

        let character = world.characters.get_mut(&CharacterId(1)).unwrap();
        character.tox = 12;
        character.toy = 10;
        assert!(world.complete_walk(CharacterId(1)));

        assert_eq!(world.skip_x_sector(10, 10, 1), 0);
        assert_eq!(world.skip_x_sector(12, 10, 1), 0);
    }

    #[test]
    fn world_updates_map_light_when_effect_enters_and_leaves_tile() {
        let mut world = World::default();
        let mut effect = Effect::new(EF_BALL, 42, 0, 10);
        effect.light = 30;
        world.effects.insert(42, effect);

        assert!(world.set_effect_on_map(42, 10, 10));
        assert_eq!(world.map.tile(10, 10).unwrap().light, 30);

        world.remove_effect_from_map(42);
        assert_eq!(world.map.tile(10, 10).unwrap().light, 0);
    }

    #[test]
    fn world_marks_dirty_sectors_for_effect_light_changes() {
        let mut world = World::default();
        let mut effect = Effect::new(EF_BALL, 42, 0, 10);
        effect.light = 30;
        world.effects.insert(42, effect);

        assert!(world.set_effect_on_map(42, 10, 10));
        assert_eq!(world.skip_x_sector(10, 10, 1), 0);
        assert_eq!(world.skip_x_sector(11, 10, 1), 0);

        world.remove_effect_from_map(42);
        assert_eq!(world.skip_x_sector(10, 10, 1), 0);
    }

    #[test]
    fn world_create_explosion_effect_matches_legacy_shape_and_expires() {
        let mut world = World::default();

        let effect_id = world.create_explosion_effect(10, 10, 8, 50450);

        let effect = world.effects.get(&effect_id).unwrap();
        assert_eq!(effect.effect_type, EF_EXPLODE);
        assert_eq!(effect.strength, 8);
        assert_eq!(effect.light, 200);
        assert_eq!(effect.base_sprite, 50450);
        assert_eq!(effect.stop_tick, 8);
        assert_eq!(world.map.tile(10, 10).unwrap().effects[0], effect_id as u16);
        assert_eq!(world.map.tile(10, 10).unwrap().light, 200);

        for _ in 0..8 {
            world.advance();
        }
        world.tick_effects();

        assert!(!world.effects.contains_key(&effect_id));
        assert_eq!(world.map.tile(10, 10).unwrap().effects[0], 0);
        assert_eq!(world.map.tile(10, 10).unwrap().light, 0);
    }

    #[test]
    fn world_create_mist_effect_uses_legacy_duration_without_light() {
        let mut world = World::default();
        world.tick.0 = 5;

        let effect_id = world.create_mist_effect(10, 10);

        let effect = world.effects.get(&effect_id).unwrap();
        assert_eq!(effect.effect_type, EF_MIST);
        assert_eq!(effect.start_tick, 5);
        assert_eq!(effect.stop_tick, 29);
        assert_eq!(effect.light, 0);
        assert_eq!(world.map.tile(10, 10).unwrap().effects[0], effect_id as u16);
        assert_eq!(world.map.tile(10, 10).unwrap().light, 0);
    }

    #[test]
    fn world_create_earthrain_places_3x3_except_sight_blocked_tiles() {
        let mut world = World::default();
        world.map.set_flags(11, 10, MapFlags::SIGHTBLOCK);
        world.map.set_flags(9, 9, MapFlags::TSIGHTBLOCK);

        let effect_id = world.create_earthrain_effect(10, 10, 7);

        let effect = world.effects.get(&effect_id).unwrap();
        assert_eq!(effect.effect_type, EF_EARTHRAIN);
        assert_eq!(effect.light, 10);
        assert_eq!(effect.strength, 7);
        assert_eq!(effect.stop_tick, TICKS_PER_SECOND as i32 * 60);
        assert_eq!(effect.fields.len(), 7);
        assert_eq!(world.map.tile(10, 10).unwrap().effects[0], effect_id as u16);
        assert_eq!(world.map.tile(11, 10).unwrap().effects[0], 0);
        assert_eq!(world.map.tile(9, 9).unwrap().effects[0], 0);
        assert!(world.map.tile(10, 10).unwrap().light >= 10);
    }

    #[test]
    fn earthrain_tick_damages_players_using_legacy_demon_reduction() {
        let mut world = World::default();
        let effect_id = world.create_earthrain_effect(10, 10, 7);
        let mut target = character(1);
        target.flags |= CharacterFlags::PLAYER;
        target.hp = 10_000;
        target.values[0][CharacterValue::Demon as usize] = 2;
        assert!(world.spawn_character(target, 10, 10));

        world.tick_effects_with_random(|_| 0);

        let target = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(target.hp, 10_000 - (7 - 2) * 150);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
        assert!(world.effects.contains_key(&effect_id));
    }

    #[test]
    fn earthrain_tick_skips_non_players_roll_misses_and_full_demon_reduction() {
        let mut world = World::default();
        world.create_earthrain_effect(10, 10, 4);
        let mut non_player = character(1);
        non_player.hp = 10_000;
        assert!(world.spawn_character(non_player, 10, 10));
        let mut demon_player = character(2);
        demon_player.flags |= CharacterFlags::PLAYER;
        demon_player.hp = 10_000;
        demon_player.values[0][CharacterValue::Demon as usize] = 4;
        assert!(world.spawn_character(demon_player, 11, 10));
        let mut missed_player = character(3);
        missed_player.flags |= CharacterFlags::PLAYER;
        missed_player.hp = 10_000;
        assert!(world.spawn_character(missed_player, 10, 11));

        world.tick_effects_with_random(|_| 1);

        assert_eq!(world.characters.get(&CharacterId(1)).unwrap().hp, 10_000);
        assert_eq!(world.characters.get(&CharacterId(2)).unwrap().hp, 10_000);
        assert_eq!(world.characters.get(&CharacterId(3)).unwrap().hp, 10_000);
    }

    #[test]
    fn world_create_earthmud_avoids_duplicate_effect_type_on_tile() {
        let mut world = World::default();

        let first_id = world.create_earthmud_effect(10, 10, 4);
        let second_id = world.create_earthmud_effect(11, 10, 9);

        assert_eq!(world.map.tile(10, 10).unwrap().effects[0], first_id as u16);
        assert!(world.map.tile(10, 10).unwrap().effects[1..]
            .iter()
            .all(|&slot| slot != second_id as u16));
        assert_eq!(world.effects[&first_id].fields.len(), 9);
        assert!(world.effects[&second_id].fields.len() < 9);
    }

    #[test]
    fn world_create_bubble_effect_stores_legacy_y_offset_as_strength() {
        let mut world = World::default();
        world.tick.0 = 100;

        let effect_id = world.create_bubble_effect(10, 10, -14, 12);

        let effect = world.effects.get(&effect_id).unwrap();
        assert_eq!(effect.effect_type, EF_BUBBLE);
        assert_eq!(effect.strength, -14);
        assert_eq!(effect.start_tick, 100);
        assert_eq!(effect.stop_tick, 112);
        assert_eq!(world.map.tile(10, 10).unwrap().effects[0], effect_id as u16);
    }

    #[test]
    fn world_schedules_existing_timer_driven_light_items() {
        let mut world = World::default();
        let mut nightlight = item(7, ItemFlags::USED);
        nightlight.driver = IDR_NIGHTLIGHT;
        nightlight.driver_data = vec![0, 9];
        let mut burning_torch = item(8, ItemFlags::USED | ItemFlags::NODECAY);
        burning_torch.driver = IDR_TORCH;
        burning_torch.driver_data = vec![1, 0, 10, 20];
        let mut unlit_torch = item(9, ItemFlags::USED);
        unlit_torch.driver = IDR_TORCH;
        unlit_torch.driver_data = vec![0, 0, 10, 20];
        world.add_item(nightlight);
        world.add_item(burning_torch);
        world.add_item(unlit_torch);

        assert_eq!(world.schedule_existing_light_timers(), 2);
        assert_eq!(world.timers.used_timers(), 2);
    }

    #[test]
    fn world_processes_zero_character_nightlight_timer_callback() {
        let mut world = World::default();
        world.date.daylight = 40;
        let mut nightlight = item(7, ItemFlags::USED);
        nightlight.driver = IDR_NIGHTLIGHT;
        nightlight.driver_data = vec![0, 9];
        world.add_item(nightlight);
        assert_eq!(world.schedule_existing_light_timers(), 1);

        world.advance();
        let outcomes = world.process_due_timers(1);

        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            outcomes[0],
            ItemDriverOutcome::LightChanged {
                character_id: CharacterId(0),
                ..
            }
        ));
        let nightlight = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(nightlight.driver_data[0], 1);
        assert_eq!(nightlight.modifier_value[0], 9);
        assert_eq!(nightlight.sprite, 1);
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn world_usetrap_schedules_target_item_driver_timer() {
        let mut world = World::default();
        world.add_character(character(1));
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
        trap.driver = IDR_USETRAP;
        trap.driver_data = vec![20, 30];
        let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
        door.driver = IDR_DOOR;
        door.x = 20;
        door.y = 30;
        world.add_item(trap);
        world.add_item(door);
        world.map.tile_mut(20, 30).unwrap().item = 8;

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_USETRAP,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::TriggerMapItem { .. }));
        assert_eq!(world.timers.used_timers(), 1);
        for _ in 0..(TICKS_PER_SECOND / 2) {
            world.advance();
        }
        let outcomes = world.process_due_timers(1);
        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            outcomes[0],
            ItemDriverOutcome::DoorToggle {
                item_id: ItemId(8),
                character_id: CharacterId(1)
            }
        ));
    }

    #[test]
    fn world_steptrap_timer_discovers_nearby_non_steptrap_target() {
        let mut world = World::default();
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
        trap.driver = IDR_STEPTRAP;
        trap.x = 10;
        trap.y = 10;
        trap.driver_data = vec![0, 0];
        let mut target = item(8, ItemFlags::USED | ItemFlags::USE);
        target.driver = IDR_DOOR;
        target.x = 11;
        target.y = 10;
        world.add_item(trap);
        world.add_item(target);
        world.map.tile_mut(10, 10).unwrap().item = 7;
        world.map.tile_mut(11, 10).unwrap().item = 8;
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

        world.advance();
        let outcomes = world.process_due_timers(1);

        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::StepTrapDiscoverTarget { item_id: ItemId(7) }]
        );
        let trap = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(&trap.driver_data[..2], &[11, 10]);
    }

    #[test]
    fn world_balltrap_creates_retained_ball_effect() {
        let mut world = World::default();
        let mut trigger = character(1);
        trigger.flags.remove(CharacterFlags::PLAYER);
        world.add_character(trigger);
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
        trap.driver = IDR_BALLTRAP;
        trap.x = 10;
        trap.y = 20;
        trap.driver_data = vec![130, 125, 42];
        world.add_item(trap);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_BALLTRAP,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::BallTrapProjectile {
                start_x: 11,
                start_y: 19,
                target_x: 12,
                target_y: 17,
                power: 42,
                ..
            }
        ));
        assert_eq!(world.effects.len(), 1);
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_BALL);
        assert_eq!(effect.strength, 42);
        assert_eq!(effect.light, 80);
        assert_eq!((effect.from_x, effect.from_y), (11, 19));
        assert_eq!((effect.to_x, effect.to_y), (12, 17));
        assert_eq!((effect.x, effect.y), (11 * 1024 + 512, 19 * 1024 + 512));
        assert_eq!(effect.caster, None);
        assert_eq!(effect.stop_tick, (TICKS_PER_SECOND * 5) as i32);
    }

    #[test]
    fn world_spiketrap_damages_and_resets_on_timer() {
        let mut world = World::default();
        let mut character = character(1);
        character.hp = 10_000;
        world.add_character(character);
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
        trap.driver = IDR_SPIKETRAP;
        trap.driver_data = vec![0, 4];
        world.add_item(trap);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_SPIKETRAP,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::SpikeTrapTriggered { .. }
        ));
        assert_eq!(world.characters.get(&CharacterId(1)).unwrap().hp, 6_000);
        assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 1);
        for _ in 0..TICKS_PER_SECOND {
            world.advance();
        }
        let outcomes = world.process_due_timers(1);
        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::SpikeTrapReset { item_id: ItemId(7) }]
        );
        assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 0);
    }

    #[test]
    fn world_flamethrower_timer_burns_forward_characters_and_reschedules() {
        let mut world = World::default();
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
        trap.driver = IDR_FLAMETHROW;
        trap.x = 10;
        trap.y = 10;
        trap.driver_data = vec![1, 3, 0, 0];
        let mut first = character(1);
        first.x = 10;
        first.y = 11;
        let mut second = character(2);
        second.x = 10;
        second.y = 12;
        world.add_item(trap);
        world.add_character(first);
        world.add_character(second);
        world.map.tile_mut(10, 10).unwrap().item = 7;
        world.map.tile_mut(10, 11).unwrap().character = 1;
        world.map.tile_mut(10, 12).unwrap().character = 2;
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

        world.advance();
        let outcomes = world.process_due_timers(1);

        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            outcomes[0],
            ItemDriverOutcome::FlameThrowerPulse { .. }
        ));
        assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 0);
        assert!(world
            .characters
            .get(&CharacterId(1))
            .unwrap()
            .flags
            .contains(CharacterFlags::UPDATE));
        assert!(world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .flags
            .contains(CharacterFlags::UPDATE));
        assert_eq!(world.effects.len(), 2);
        assert!(world.effects.values().any(|effect| {
            effect.effect_type == EF_BURN && effect.target_character == Some(CharacterId(1))
        }));
        assert!(world.effects.values().any(|effect| {
            effect.effect_type == EF_BURN && effect.target_character == Some(CharacterId(2))
        }));
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn burn_character_suppresses_duplicates_and_expires() {
        let mut world = World::default();
        let mut character = character(1);
        character.hp = 50 * POWERSCALE;
        world.add_character(character);

        assert!(world.burn_character(CharacterId(1)));
        assert!(!world.burn_character(CharacterId(1)));
        assert_eq!(world.effects.len(), 1);
        assert_eq!(
            world.characters.get(&CharacterId(1)).unwrap().hp,
            30 * POWERSCALE
        );

        world.tick = Tick(TICKS_PER_SECOND * 60);
        world.tick_effects();

        assert!(world.effects.is_empty());
    }

    #[test]
    fn extinguish_driver_removes_burn_effect() {
        let mut world = World::default();
        let mut character = character(1);
        character.hp = 50 * POWERSCALE;
        world.add_character(character);
        let mut water = item(7, ItemFlags::USED | ItemFlags::USE);
        water.driver = crate::item_driver::IDR_EXTINGUISH;
        world.add_item(water);
        assert!(world.burn_character(CharacterId(1)));

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_EXTINGUISH,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            2,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::Extinguish {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                extinguished: true,
            }
        );
        assert!(world.effects.is_empty());
    }

    #[test]
    fn extinguish_driver_reports_refreshing_when_not_burning() {
        let mut world = World::default();
        world.add_character(character(1));
        let mut water = item(7, ItemFlags::USED | ItemFlags::USE);
        water.driver = crate::item_driver::IDR_EXTINGUISH;
        world.add_item(water);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_EXTINGUISH,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            2,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::Extinguish {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                extinguished: false,
            }
        );
    }

    #[test]
    fn world_timer_callback_expires_and_destroys_burned_out_torch() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        let mut torch = item(7, ItemFlags::USED | ItemFlags::USE);
        torch.carried_by = Some(CharacterId(1));
        torch.driver = IDR_TORCH;
        torch.driver_data = vec![0, 1, 1, 20];
        world.add_character(character);
        world.add_item(torch);

        world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_TORCH,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );
        world.tick.0 = 30 * crate::tick::TICKS_PER_SECOND;
        let outcomes = world.process_due_timers(1);

        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            outcomes[0],
            ItemDriverOutcome::TorchExpired { item_name: _, .. }
        ));
        assert!(!world.items.contains_key(&ItemId(7)));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[30], None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
    }

    #[test]
    fn world_palace_key_final_combine_consumes_cursor_and_creates_final_key() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        character.cursor_item = Some(ItemId(8));
        let mut carried = item(7, ItemFlags::USED | ItemFlags::USE);
        carried.carried_by = Some(CharacterId(1));
        carried.driver = IDR_PALACEKEY;
        carried.template_id = crate::item_driver::IID_AREA11_PALACEKEYPART;
        carried.sprite = 51015;
        let mut cursor = item(8, ItemFlags::USED | ItemFlags::USE);
        cursor.carried_by = Some(CharacterId(1));
        cursor.driver = IDR_PALACEKEY;
        cursor.template_id = crate::item_driver::IID_AREA11_PALACEKEYPART;
        cursor.sprite = 51039;
        world.add_character(character);
        world.add_item(carried);
        world.add_item(cursor);

        let outcome = world.execute_item_driver_request_with_context(
            ItemDriverRequest::Driver {
                driver: IDR_PALACEKEY,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            11,
            &ItemDriverContext {
                cursor_template_id: Some(crate::item_driver::IID_AREA11_PALACEKEYPART),
                cursor_sprite: Some(51039),
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::PalaceKeyCombine {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(8),
                result_sprite: 51014,
                final_key: true,
            }
        );
        assert!(!world.items.contains_key(&ItemId(8)));
        let carried = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(carried.sprite, 51014);
        assert_eq!(
            carried.template_id,
            crate::item_driver::IID_AREA11_PALACEKEY
        );
        assert_eq!(carried.driver, 0);
        assert!(!carried.flags.contains(ItemFlags::USE));
        assert_eq!(carried.name, "Palace Key");
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.cursor_item, None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
    }

    #[test]
    fn world_blocks_lighting_torch_underwater() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.inventory[30] = Some(ItemId(7));
        let mut torch = item(7, ItemFlags::USED | ItemFlags::USE);
        torch.carried_by = Some(CharacterId(1));
        torch.driver = IDR_TORCH;
        torch.driver_data = vec![0, 0, 10, 20];
        world.add_character(character);
        world.add_item(torch);
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::UNDERWATER);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_TORCH,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
        let torch = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(torch.driver_data[0], 0);
        assert_eq!(torch.modifier_value[0], 0);
        assert!(!torch.flags.contains(ItemFlags::NODECAY));
    }

    #[test]
    fn world_timer_extinguishes_burning_torch_underwater() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.inventory[30] = Some(ItemId(7));
        let mut torch = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::NODECAY);
        torch.carried_by = Some(CharacterId(1));
        torch.driver = IDR_TORCH;
        torch.driver_data = vec![1, 0, 10, 20];
        torch.modifier_value[0] = 20;
        torch.sprite = -1;
        world.add_character(character);
        world.add_item(torch);
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::UNDERWATER);

        let outcome = world.execute_item_driver_request_with_context(
            ItemDriverRequest::Driver {
                driver: IDR_TORCH,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::TorchExtinguishedUnderwater {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                schedule_after_ticks: 30 * crate::tick::TICKS_PER_SECOND,
            }
        );
        let torch = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(torch.driver_data[0], 0);
        assert_eq!(torch.modifier_value[0], 0);
        assert_eq!(torch.sprite, 0);
        assert!(!torch.flags.contains(ItemFlags::NODECAY));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn world_applies_torch_orb_extraction_to_inventory() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        let mut torch = item(7, ItemFlags::USED | ItemFlags::USE);
        torch.carried_by = Some(CharacterId(1));
        torch.driver = IDR_TORCH;
        torch.modifier_index[1] = CharacterValue::Speed as i16;
        torch.modifier_value[1] = 2;
        let mut orb = item(8, ItemFlags::USED | ItemFlags::USE);
        orb.name = "Orb of Speed".to_string();
        orb.carried_by = Some(CharacterId(1));
        orb.driver_data = vec![CharacterValue::Speed as u8, 1];
        world.add_character(character);
        world.add_item(torch);

        assert!(world.apply_torch_extract_orb(ItemId(7), CharacterId(1), 1, orb));

        let torch = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(torch.modifier_value[1], 1);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[31], Some(ItemId(8)));
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert_eq!(
            world.items.get(&ItemId(8)).unwrap().carried_by,
            Some(CharacterId(1))
        );
    }

    #[test]
    fn world_enchants_cursor_equipment_and_consumes_orb() {
        let mut world = World::default();
        let mut character = character(1);
        character.cursor_item = Some(ItemId(8));
        character.inventory[30] = Some(ItemId(7));
        let mut orb = item(7, ItemFlags::USED | ItemFlags::USE);
        orb.carried_by = Some(CharacterId(1));
        orb.driver = IDR_ENCHANTITEM;
        orb.driver_data = vec![CharacterValue::Sword as u8, 3];
        let equipment = item(8, ItemFlags::USED | ItemFlags::WNNECK);
        world.add_character(character);
        world.add_item(orb);
        world.add_item(equipment);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_ENCHANTITEM,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::EnchantCursorItem { .. }
        ));
        assert!(!world.items.contains_key(&ItemId(7)));
        let equipment = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(equipment.modifier_index[0], CharacterValue::Sword as i16);
        assert_eq!(equipment.modifier_value[0], 3);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[30], None);
        assert_eq!(character.cursor_item, Some(ItemId(8)));
        assert!(character.flags.contains(CharacterFlags::ITEMS));
    }

    #[test]
    fn world_blocks_enchant_beyond_legacy_limits_without_consuming_orb() {
        let mut world = World::default();
        let mut character = character(1);
        character.cursor_item = Some(ItemId(8));
        character.inventory[30] = Some(ItemId(7));
        let mut orb = item(7, ItemFlags::USED | ItemFlags::USE);
        orb.carried_by = Some(CharacterId(1));
        orb.driver = IDR_ENCHANTITEM;
        orb.driver_data = vec![CharacterValue::Sword as u8, 2];
        let mut equipment = item(8, ItemFlags::USED | ItemFlags::WNNECK);
        equipment.modifier_index[0] = CharacterValue::Sword as i16;
        equipment.modifier_value[0] = 19;
        world.add_character(character);
        world.add_item(orb);
        world.add_item(equipment);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_ENCHANTITEM,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
        assert!(world.items.contains_key(&ItemId(7)));
        assert_eq!(world.items.get(&ItemId(8)).unwrap().modifier_value[0], 19);
    }

    #[test]
    fn world_anti_enchant_reduces_or_removes_cursor_equipment_modifier() {
        let mut world = World::default();
        let mut character = character(1);
        character.cursor_item = Some(ItemId(8));
        character.inventory[30] = Some(ItemId(7));
        character.inventory[31] = Some(ItemId(9));
        let mut anti_orb = item(7, ItemFlags::USED | ItemFlags::USE);
        anti_orb.carried_by = Some(CharacterId(1));
        anti_orb.driver = IDR_ANTIENCHANTITEM;
        anti_orb.driver_data = vec![CharacterValue::Sword as u8, 2];
        let mut second_anti_orb = item(9, ItemFlags::USED | ItemFlags::USE);
        second_anti_orb.carried_by = Some(CharacterId(1));
        second_anti_orb.driver = IDR_ANTIENCHANTITEM;
        second_anti_orb.driver_data = vec![CharacterValue::Sword as u8, 3];
        let mut equipment = item(8, ItemFlags::USED | ItemFlags::WNNECK);
        equipment.modifier_index[0] = CharacterValue::Sword as i16;
        equipment.modifier_value[0] = 5;
        world.add_character(character);
        world.add_item(anti_orb);
        world.add_item(second_anti_orb);
        world.add_item(equipment);

        let request = |item_id| ItemDriverRequest::Driver {
            driver: IDR_ANTIENCHANTITEM,
            item_id: ItemId(item_id),
            character_id: CharacterId(1),
            spec: 0,
        };
        assert!(matches!(
            world.execute_item_driver_request(request(7), 1),
            ItemDriverOutcome::AntiEnchantCursorItem { .. }
        ));
        assert_eq!(world.items.get(&ItemId(8)).unwrap().modifier_value[0], 3);
        assert!(!world.items.contains_key(&ItemId(7)));

        assert!(matches!(
            world.execute_item_driver_request(request(9), 1),
            ItemDriverOutcome::AntiEnchantCursorItem { .. }
        ));
        let equipment = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(equipment.modifier_index[0], 0);
        assert_eq!(equipment.modifier_value[0], 0);
        assert!(!world.items.contains_key(&ItemId(9)));
    }

    #[test]
    fn world_completes_walk_against_map_storage() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.tox = 11;
        character.toy = 10;
        world.map.tile_mut(10, 10).unwrap().character = 1;
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world
            .map
            .tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_character(character);

        assert!(world.complete_walk(CharacterId(1)));
        assert_eq!(world.characters.get(&CharacterId(1)).unwrap().x, 11);
        assert_eq!(world.map.tile(11, 10).unwrap().character, 1);
    }

    #[test]
    fn world_completes_take_and_drop_against_item_storage() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.dir = Direction::Right as u8;
        character.act1 = 7;
        let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        assert!(world.map.set_item_map(&mut item, 11, 10));
        world.add_character(character);
        world.add_item(item);

        assert!(world.complete_take(CharacterId(1), ItemId(7), true));
        assert_eq!(
            world.characters.get(&CharacterId(1)).unwrap().cursor_item,
            Some(ItemId(7))
        );

        world.characters.get_mut(&CharacterId(1)).unwrap().act1 = 7;
        assert!(world.complete_drop(CharacterId(1), ItemId(7)));
        assert_eq!(
            world.characters.get(&CharacterId(1)).unwrap().cursor_item,
            None
        );
        assert_eq!(world.map.tile(11, 10).unwrap().item, 7);
    }

    #[test]
    fn world_applies_player_walkdir_setup_or_falls_back_to_idle() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::WalkDir,
            arg1: Direction::Right as i32,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::WALK);
        assert_eq!((character.tox, character.toy), (11, 10));

        world.map.set_flags(12, 10, MapFlags::MOVEBLOCK);
        world.characters.get_mut(&CharacterId(1)).unwrap().x = 11;
        world.characters.get_mut(&CharacterId(1)).unwrap().y = 10;
        world.characters.get_mut(&CharacterId(1)).unwrap().tox = 0;
        world.characters.get_mut(&CharacterId(1)).unwrap().toy = 0;
        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::IDLE);
        assert_eq!(player.action.action, PlayerActionCode::Idle);
    }

    #[test]
    fn world_applies_player_walkdir_diagonal_wall_slide() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        world.map.set_flags(11, 10, MapFlags::MOVEBLOCK);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::WalkDir,
            arg1: Direction::RightUp as i32,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::WALK);
        assert_eq!((character.tox, character.toy), (10, 9));
        assert_eq!(character.dir, Direction::Up as u8);
    }

    #[test]
    fn world_applies_player_move_setup_with_pathfinder() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Move,
            arg1: 13,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::WALK);
        assert_eq!((character.tox, character.toy), (11, 10));
        assert_eq!(player.action.action, PlayerActionCode::Move);
    }

    #[test]
    fn world_applies_player_drop_setup_from_cursor_item() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.dir = Direction::Right as u8;
        character.cursor_item = Some(ItemId(7));
        world.add_character(character);
        world.add_item(item(7, ItemFlags::USED | ItemFlags::TAKE));
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Drop,
            arg1: 11,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::DROP);
        assert_eq!(character.act1, 7);
    }

    #[test]
    fn world_applies_player_take_setup_from_adjacent_map_item() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        assert!(world.map.set_item_map(&mut item, 11, 10));
        world.add_item(item);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Take,
            arg1: 11,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::TAKE);
        assert_eq!(character.act1, 7);
        assert_eq!(character.dir, Direction::Right as u8);
    }

    #[test]
    fn world_applies_player_take_setup_by_walking_toward_distant_item() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        assert!(world.map.set_item_map(&mut item, 13, 10));
        world.add_item(item);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Take,
            arg1: 13,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::WALK);
        assert_eq!((character.tox, character.toy), (11, 10));
        assert_eq!(player.action.action, PlayerActionCode::Take);
    }

    #[test]
    fn world_applies_player_drop_setup_by_walking_toward_distant_target() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.cursor_item = Some(ItemId(7));
        world.add_character(character);
        world.add_item(item(7, ItemFlags::USED | ItemFlags::TAKE));
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Drop,
            arg1: 13,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::WALK);
        assert_eq!((character.tox, character.toy), (11, 10));
        assert_eq!(player.action.action, PlayerActionCode::Drop);
    }

    #[test]
    fn world_applies_player_give_setup_to_adjacent_character() {
        let mut world = World::default();
        let mut giver = character(1);
        giver.x = 10;
        giver.y = 10;
        giver.cursor_item = Some(ItemId(7));
        let mut receiver = character(2);
        receiver.flags.insert(CharacterFlags::PLAYER);
        receiver.x = 11;
        receiver.y = 10;
        world.add_character(giver);
        world.add_character(receiver);
        let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        item.carried_by = Some(CharacterId(1));
        world.add_item(item);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Give,
            arg1: 2,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let giver = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(giver.action, action::GIVE);
        assert_eq!(giver.act1, 2);
        assert_eq!(giver.dir, Direction::Right as u8);
    }

    #[test]
    fn world_applies_player_give_setup_by_walking_toward_recipient() {
        let mut world = World::default();
        let mut giver = character(1);
        giver.x = 10;
        giver.y = 10;
        giver.cursor_item = Some(ItemId(7));
        let mut receiver = character(2);
        receiver.flags.insert(CharacterFlags::PLAYER);
        receiver.x = 13;
        receiver.y = 10;
        world.add_character(giver);
        world.add_character(receiver);
        let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        item.carried_by = Some(CharacterId(1));
        world.add_item(item);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Give,
            arg1: 2,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let giver = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(giver.action, action::WALK);
        assert_eq!((giver.tox, giver.toy), (11, 10));
        assert_eq!(player.action.action, PlayerActionCode::Give);
    }

    #[test]
    fn world_completes_give_to_player_inventory_or_cursor() {
        let mut world = World::default();
        let mut giver = character(1);
        giver.x = 10;
        giver.y = 10;
        giver.dir = Direction::Right as u8;
        giver.action = action::GIVE;
        giver.duration = 1;
        giver.act1 = 2;
        giver.cursor_item = Some(ItemId(7));
        let mut receiver = character(2);
        receiver.flags.insert(CharacterFlags::PLAYER);
        receiver.x = 11;
        receiver.y = 10;
        world.map.tile_mut(11, 10).unwrap().character = 2;
        world.add_character(giver);
        world.add_character(receiver);
        let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        item.carried_by = Some(CharacterId(1));
        world.add_item(item);

        let completed = world.tick_basic_actions();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].action_id, action::GIVE);
        assert!(completed[0].ok);
        assert_eq!(
            world.characters.get(&CharacterId(1)).unwrap().cursor_item,
            None
        );
        assert_eq!(
            world.characters.get(&CharacterId(2)).unwrap().cursor_item,
            Some(ItemId(7))
        );
        assert_eq!(
            world.items.get(&ItemId(7)).unwrap().carried_by,
            Some(CharacterId(2))
        );
    }

    #[test]
    fn world_applies_player_use_setup_from_adjacent_map_item() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        assert!(world.map.set_item_map(&mut item, 11, 10));
        world.add_item(item);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Use,
            arg1: 11,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::USE);
        assert_eq!(character.act1, 7);
        assert_eq!(character.dir, Direction::Right as u8);
    }

    #[test]
    fn world_applies_player_use_setup_by_walking_toward_frontwall_item() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::FRONTWALL);
        assert!(world.map.set_item_map(&mut item, 13, 10));
        world.add_item(item);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Use,
            arg1: 13,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::WALK);
        assert_eq!((character.tox, character.toy), (11, 10));
        assert_eq!(player.action.action, PlayerActionCode::Use);
    }

    #[test]
    fn world_completes_use_as_pending_item_driver_request() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.dir = Direction::Right as u8;
        character.action = action::USE;
        character.duration = 1;
        character.act1 = 7;
        character.act2 = 42;
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        assert!(world.map.set_item_map(&mut item, 11, 10));
        world.add_item(item);

        let completed = world.tick_basic_actions();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].character_id, CharacterId(1));
        assert_eq!(completed[0].action_id, action::USE);
        assert!(completed[0].ok);
        assert_eq!(completed[0].item_use.unwrap().item_id, ItemId(7));
        assert_eq!(completed[0].item_use.unwrap().spec, 42);
    }

    #[test]
    fn world_applies_player_kill_setup_to_adjacent_character() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker.x = 10;
        attacker.y = 10;
        let mut defender = character(2);
        defender.x = 11;
        defender.y = 10;
        world.map.tile_mut(11, 10).unwrap().character = 2;
        world.add_character(attacker);
        world.add_character(defender);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Kill,
            arg1: 2,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let attacker = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(attacker.action, action::ATTACK1);
        assert_eq!(attacker.act1, 2);
        assert_eq!(attacker.dir, Direction::Right as u8);
    }

    #[test]
    fn world_applies_player_kill_setup_by_walking_toward_target() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker.x = 10;
        attacker.y = 10;
        world.map.tile_mut(10, 10).unwrap().character = 1;
        let mut defender = character(2);
        defender.x = 13;
        defender.y = 10;
        world.map.tile_mut(13, 10).unwrap().character = 2;
        world.add_character(attacker);
        world.add_character(defender);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Kill,
            arg1: 2,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let attacker = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(attacker.action, action::WALK);
        assert_eq!((attacker.tox, attacker.toy), (11, 10));
        assert_eq!(player.action.action, PlayerActionCode::Kill);
    }

    #[test]
    fn world_completes_attack_action_with_damage() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker.x = 10;
        attacker.y = 10;
        attacker.dir = Direction::Right as u8;
        attacker.action = action::ATTACK1;
        attacker.duration = 1;
        attacker.act1 = 2;
        attacker.values[0][CharacterValue::Attack as usize] = 10;
        attacker.values[0][CharacterValue::Weapon as usize] = 10;
        let mut defender = character(2);
        defender.x = 11;
        defender.y = 10;
        defender.dir = Direction::Left as u8;
        defender.hp = 10_000;
        defender.values[0][CharacterValue::Parry as usize] = 10;
        world.map.tile_mut(11, 10).unwrap().character = 2;
        world.add_character(attacker);
        world.add_character(defender);

        let completed = world.tick_basic_actions();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].action_id, action::ATTACK1);
        assert!(completed[0].ok);
        assert!(world.characters.get(&CharacterId(2)).unwrap().hp < 10_000);
    }

    #[test]
    fn world_applies_completed_item_use_request_to_container_state() {
        let mut world = World::default();
        world.add_character(character(1));
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        item.content_id = 22;
        world.add_item(item);

        let outcome = world
            .use_item_request(
                ItemUseRequest {
                    character_id: CharacterId(1),
                    item_id: ItemId(7),
                    spec: 0,
                },
                false,
            )
            .unwrap();

        assert_eq!(
            outcome,
            UseItemOutcome::OpenContainer { item_id: ItemId(7) }
        );
        assert_eq!(
            world
                .characters
                .get(&CharacterId(1))
                .unwrap()
                .current_container,
            Some(ItemId(7))
        );
    }

    #[test]
    fn world_executes_same_area_teleport_driver_outcome() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.action = action::USE;
        character.duration = 3;
        world.map.tile_mut(10, 10).unwrap().character = 1;
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        item.driver = crate::item_driver::IDR_TELEPORT;
        item.driver_data = vec![30, 0, 40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        world.add_item(item);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_TELEPORT,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::Teleport { .. }));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (30, 40));
        assert_eq!(character.action, 0);
        assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
        assert_eq!(world.map.tile(30, 40).unwrap().character, 1);
    }

    #[test]
    fn world_executes_teleport_door_to_exact_opposite_side() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 9;
        character.y = 10;
        world.map.tile_mut(9, 10).unwrap().character = 1;
        world
            .map
            .tile_mut(9, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        item.driver = crate::item_driver::IDR_TELE_DOOR;
        item.x = 10;
        item.y = 10;
        world.add_item(item);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_TELE_DOOR,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::TeleportDoor { .. }));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (11, 10));
        assert_eq!(world.map.tile(9, 10).unwrap().character, 0);
        assert_eq!(world.map.tile(11, 10).unwrap().character, 1);
    }

    #[test]
    fn world_executes_same_area_recall_and_consumes_scroll() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.rest_area = 1;
        character.rest_x = 30;
        character.rest_y = 40;
        character.cursor_item = Some(ItemId(7));
        world.map.tile_mut(10, 10).unwrap().character = 1;
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        item.driver = crate::item_driver::IDR_RECALL;
        item.carried_by = Some(CharacterId(1));
        item.driver_data = vec![10];
        world.add_item(item);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_RECALL,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::Recall { .. }));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (30, 40));
        assert_eq!(character.cursor_item, None);
        assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
        assert_eq!(world.map.tile(30, 40).unwrap().character, 1);
        assert!(!world
            .items
            .get(&ItemId(7))
            .unwrap()
            .flags
            .contains(ItemFlags::USED));
    }

    #[test]
    fn world_executes_same_area_city_recall_and_decrements_stack() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.inventory[30] = Some(ItemId(7));
        world.map.tile_mut(10, 10).unwrap().character = 1;
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        item.driver = crate::item_driver::IDR_CITY_RECALL;
        item.carried_by = Some(CharacterId(1));
        item.driver_data = vec![0, 3];
        world.add_item(item);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_CITY_RECALL,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::CityRecall { .. }));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (126, 179));
        assert_eq!(character.inventory[30], Some(ItemId(7)));
        assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[1], 2);
        assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
        assert_eq!(world.map.tile(126, 179).unwrap().character, 1);
    }

    #[test]
    fn world_consumes_final_city_recall_before_cross_area_handoff() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.cursor_item = Some(ItemId(7));
        world.map.tile_mut(10, 10).unwrap().character = 1;
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        item.driver = crate::item_driver::IDR_CITY_RECALL;
        item.carried_by = Some(CharacterId(1));
        item.driver_data = vec![1, 1];
        world.add_item(item);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_CITY_RECALL,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::CityRecall {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 167,
                y: 188,
                area_id: 3,
            }
        );
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (10, 10));
        assert_eq!(character.cursor_item, None);
        assert!(!world
            .items
            .get(&ItemId(7))
            .unwrap()
            .flags
            .contains(ItemFlags::USED));
    }

    #[test]
    fn world_executes_door_driver_open_and_close() {
        let mut world = World::default();
        world.add_character(character(1));
        let mut door = item(
            7,
            ItemFlags::USED
                | ItemFlags::USE
                | ItemFlags::MOVEBLOCK
                | ItemFlags::SIGHTBLOCK
                | ItemFlags::SOUNDBLOCK
                | ItemFlags::DOOR,
        );
        door.driver = crate::item_driver::IDR_DOOR;
        door.sprite = 100;
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        let request = ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        let outcome = world.execute_item_driver_request(request, 1);
        assert_eq!(
            outcome,
            ItemDriverOutcome::DoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
        let door = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(door.driver_data[0], 1);
        assert_eq!(door.sprite, 101);
        assert!(!door.flags.intersects(
            ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK | ItemFlags::SOUNDBLOCK | ItemFlags::DOOR
        ));
        let tile = world.map.tile(10, 10).unwrap();
        assert!(!tile.flags.intersects(
            MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::TSOUNDBLOCK | MapFlags::DOOR
        ));

        let outcome = world.execute_item_driver_request(request, 1);
        assert!(matches!(outcome, ItemDriverOutcome::DoorToggle { .. }));
        let door = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(door.driver_data[0], 0);
        assert_eq!(door.sprite, 100);
        assert!(door.flags.contains(ItemFlags::MOVEBLOCK));
        assert!(door.flags.contains(ItemFlags::SIGHTBLOCK));
        assert!(door.flags.contains(ItemFlags::SOUNDBLOCK));
        assert!(door.flags.contains(ItemFlags::DOOR));
        let tile = world.map.tile(10, 10).unwrap();
        assert!(tile.flags.contains(MapFlags::TMOVEBLOCK));
        assert!(tile.flags.contains(MapFlags::TSIGHTBLOCK));
        assert!(tile.flags.contains(MapFlags::TSOUNDBLOCK));
        assert!(tile.flags.contains(MapFlags::DOOR));
    }

    #[test]
    fn world_does_not_close_open_door_when_blocked() {
        let mut world = World::default();
        world.add_character(character(1));
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
        door.driver = crate::item_driver::IDR_DOOR;
        door.sprite = 101;
        door.driver_data = vec![1];
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_item(door);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_DOOR,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert_eq!(outcome, ItemDriverOutcome::Noop);
        let door = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(door.driver_data[0], 1);
        assert_eq!(door.sprite, 101);
    }

    #[test]
    fn world_auto_closes_opened_door_from_timer() {
        let mut world = World::default();
        world.add_character(character(1));
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
        door.driver = crate::item_driver::IDR_DOOR;
        door.sprite = 100;
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_DOOR,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::DoorToggle { .. }));
        assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[39], 1);
        assert_eq!(world.timers.used_timers(), 1);

        for _ in 0..(TICKS_PER_SECOND * 10) {
            world.advance();
        }
        let outcomes = world.process_due_timers(1);

        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::DoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(0),
            }]
        );
        let door = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(door.driver_data[0], 0);
        assert_eq!(door.driver_data[39], 0);
        assert_eq!(door.sprite, 100);
    }

    #[test]
    fn world_respects_no_auto_close_door_flag() {
        let mut world = World::default();
        world.add_character(character(1));
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
        door.driver = crate::item_driver::IDR_DOOR;
        door.driver_data.resize(6, 0);
        door.driver_data[5] = 1;
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_DOOR,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::DoorToggle { .. }));
        let door = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(door.driver_data[0], 1);
        assert_eq!(door.driver_data[39], 1);
        assert_eq!(world.timers.used_timers(), 0);
    }

    #[test]
    fn world_retries_blocked_door_timer_close() {
        let mut world = World::default();
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
        door.driver = crate::item_driver::IDR_DOOR;
        door.sprite = 101;
        door.driver_data.resize(40, 0);
        door.driver_data[0] = 1;
        door.driver_data[39] = 1;
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_item(door);
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

        world.advance();
        assert_eq!(world.process_due_timers(1), vec![ItemDriverOutcome::Noop]);
        let door = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(door.driver_data[0], 1);
        assert_eq!(door.driver_data[39], 1);
        assert_eq!(world.timers.used_timers(), 1);

        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .remove(MapFlags::TMOVEBLOCK);
        for _ in 0..(TICKS_PER_SECOND * 5) {
            world.advance();
        }
        let outcomes = world.process_due_timers(1);

        assert!(matches!(
            outcomes.as_slice(),
            [ItemDriverOutcome::DoorToggle { .. }]
        ));
        let door = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(door.driver_data[0], 0);
        assert_eq!(door.driver_data[39], 0);
        assert_eq!(door.sprite, 100);
    }

    #[test]
    fn world_shifts_extended_door_foreground_sprites() {
        let mut world = World::default();
        world.add_character(character(1));
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
        door.driver = crate::item_driver::IDR_DOOR;
        door.sprite = 100;
        door.driver_data.resize(8, 0);
        door.driver_data[7] = 1;
        assert!(world.map.set_item_map(&mut door, 10, 10));
        for (x, y, sprite) in [(11, 10, 20), (9, 10, 21), (10, 11, 22), (10, 9, 23)] {
            world.map.tile_mut(x, y).unwrap().foreground_sprite = sprite;
        }
        world.add_item(door);

        let request = ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert!(matches!(
            world.execute_item_driver_request(request, 1),
            ItemDriverOutcome::DoorToggle { .. }
        ));
        assert_eq!(world.map.tile(11, 10).unwrap().foreground_sprite, 21);
        assert_eq!(world.map.tile(9, 10).unwrap().foreground_sprite, 22);
        assert_eq!(world.map.tile(10, 11).unwrap().foreground_sprite, 23);
        assert_eq!(world.map.tile(10, 9).unwrap().foreground_sprite, 24);

        assert!(matches!(
            world.execute_item_driver_request(request, 1),
            ItemDriverOutcome::DoorToggle { .. }
        ));
        assert_eq!(world.map.tile(11, 10).unwrap().foreground_sprite, 20);
        assert_eq!(world.map.tile(9, 10).unwrap().foreground_sprite, 21);
        assert_eq!(world.map.tile(10, 11).unwrap().foreground_sprite, 22);
        assert_eq!(world.map.tile(10, 9).unwrap().foreground_sprite, 23);
    }

    #[test]
    fn world_executes_double_door_and_syncs_adjacent_state() {
        let mut world = World::default();
        world.add_character(character(1));
        let closed_flags = ItemFlags::USED
            | ItemFlags::USE
            | ItemFlags::MOVEBLOCK
            | ItemFlags::SIGHTBLOCK
            | ItemFlags::SOUNDBLOCK
            | ItemFlags::DOOR;
        let mut primary = item(7, closed_flags);
        primary.driver = crate::item_driver::IDR_DOUBLE_DOOR;
        primary.sprite = 100;
        assert!(world.map.set_item_map(&mut primary, 10, 10));
        world.add_item(primary);

        let mut adjacent = item(8, closed_flags);
        adjacent.driver = crate::item_driver::IDR_DOOR;
        adjacent.sprite = 200;
        assert!(world.map.set_item_map(&mut adjacent, 10, 11));
        world.add_item(adjacent);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_DOUBLE_DOOR,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::DoubleDoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
        let primary = world.items.get(&ItemId(7)).unwrap();
        let adjacent = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(primary.driver_data[0], 1);
        assert_eq!(adjacent.driver_data[0], 1);
        assert_eq!(primary.sprite, 101);
        assert_eq!(adjacent.sprite, 201);
        assert!(!world
            .map
            .tile(10, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));
        assert!(!world
            .map
            .tile(10, 11)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));
    }

    #[test]
    fn world_applies_shrike_amulet_assembly() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        character.cursor_item = Some(ItemId(8));
        world.add_character(character);

        let mut base = item(7, ItemFlags::USED | ItemFlags::USE);
        base.driver = crate::item_driver::IDR_SHRIKEAMULET;
        base.carried_by = Some(CharacterId(1));
        base.driver_data = vec![1];
        world.add_item(base);
        let mut cursor = item(8, ItemFlags::USED | ItemFlags::USE);
        cursor.driver = crate::item_driver::IDR_SHRIKEAMULET;
        cursor.carried_by = Some(CharacterId(1));
        cursor.driver_data = vec![2];
        world.add_item(cursor);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_SHRIKEAMULET,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::ShrikeAmuletAssemble { .. }
        ));
        let base = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(base.driver_data[0], 3);
        assert_eq!(base.sprite, 51620);
        assert_eq!(base.name, "Crystal on Chain");
        assert_eq!(base.description, "A light blue crystal on a silver chain.");
        assert!(!world.items.contains_key(&ItemId(8)));
        assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
    }

    #[test]
    fn world_applies_mine_gateway_key_final_assembly() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        character.cursor_item = Some(ItemId(8));
        world.add_character(character);

        let mut base = item(7, ItemFlags::USED | ItemFlags::USE);
        base.driver = crate::item_driver::IDR_MINEGATEWAYKEY;
        base.carried_by = Some(CharacterId(1));
        base.driver_data = vec![7];
        world.add_item(base);
        let mut cursor = item(8, ItemFlags::USED | ItemFlags::USE);
        cursor.driver = crate::item_driver::IDR_MINEGATEWAYKEY;
        cursor.carried_by = Some(CharacterId(1));
        cursor.driver_data = vec![8];
        world.add_item(cursor);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_MINEGATEWAYKEY,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::MineGatewayKeyAssemble { .. }
        ));
        let base = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(base.driver_data[0], 15);
        assert_eq!(base.sprite, 52200);
        assert_eq!(base.template_id, 0x01000098);
        assert_eq!(base.name, "Mine gateway key");
        assert_eq!(base.description, "A fully assembled key.");
        assert!(!base.flags.contains(ItemFlags::USE));
        assert!(!world.items.contains_key(&ItemId(8)));
        assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
    }

    #[test]
    fn world_applies_player_teleport_as_facing_item_use() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.dir = Direction::Right as u8;
        world.add_character(character);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        assert!(world.map.set_item_map(&mut item, 11, 10));
        world.add_item(item);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Teleport,
            arg1: 5,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::USE);
        assert_eq!((character.act1, character.act2), (7, 6));
        assert_eq!(character.dir, Direction::Right as u8);
    }

    #[test]
    fn world_applies_player_look_map_as_immediate_request() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::LookMap,
            arg1: 13,
            arg2: 9,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.dir, Direction::RightUp as u8);
        assert_eq!(character.action, action::IDLE);
        let requests = world.drain_look_map_requests();
        assert_eq!(
            requests,
            vec![LookMapRequest {
                character_id: CharacterId(1),
                x: 13,
                y: 9,
                character_level: 1,
                visible: true,
            }]
        );
    }

    #[test]
    fn world_ticks_basic_action_completion_and_resets_state() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.tox = 11;
        character.toy = 10;
        character.action = action::WALK;
        character.duration = 2;
        world.map.tile_mut(10, 10).unwrap().character = 1;
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world
            .map
            .tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_character(character);

        assert!(world.tick_basic_actions().is_empty());
        let completed = world.tick_basic_actions();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].character_id, CharacterId(1));
        assert_eq!(completed[0].action_id, action::WALK);
        assert!(completed[0].ok);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (11, 10));
        assert_eq!(character.action, 0);
        assert_eq!(character.duration, 0);
        assert_eq!(character.step, 0);
    }

    #[test]
    fn player_magicshield_spell_sets_up_and_completes_lifeshield_gain() {
        let mut world = World::default();
        let mut character = character(1);
        character.mana = 10 * POWERSCALE;
        character.values[0][CharacterValue::MagicShield as usize] = 8;
        character.values[0][CharacterValue::Speed as usize] = 24;
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::MagicShield,
            arg1: 0,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::MAGICSHIELD);
        assert_eq!(character.act1, 8 * POWERSCALE);
        assert_eq!(character.mana, 6 * POWERSCALE);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        let completed = world.tick_basic_actions();

        assert_eq!(completed.len(), 1);
        assert!(completed[0].ok);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.lifeshield, 8 * POWERSCALE);
        assert_eq!(character.action, 0);
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_MAGICSHIELD);
        assert_eq!(effect.target_character, Some(CharacterId(1)));
        assert_eq!(effect.stop_tick, 3);
        assert_eq!(effect.light, 25);
    }

    #[test]
    fn player_heal_spell_restores_target_hp_on_completion() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.x = 10;
        caster.y = 10;
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Heal as usize] = 10;
        caster.values[0][CharacterValue::Speed as usize] = 24;
        let mut target = character(2);
        target.x = 11;
        target.y = 10;
        target.hp = 5 * POWERSCALE;
        target.values[0][CharacterValue::Hp as usize] = 10;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 11, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Heal,
            arg1: 2,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::HEAL1);
        assert_eq!(caster.dir, Direction::Right as u8);
        assert_eq!(caster.act2, 5 * POWERSCALE);
        assert_eq!(caster.mana, 15 * POWERSCALE / 2);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        let completed = world.tick_basic_actions();

        assert_eq!(completed.len(), 1);
        assert!(completed[0].ok);
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 10 * POWERSCALE);
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_HEAL);
        assert_eq!(effect.target_character, Some(CharacterId(2)));
        assert_eq!(effect.stop_tick, 8);
    }

    #[test]
    fn player_bless_spell_installs_carried_spell_item_on_completion() {
        let mut world = World::default();
        world.tick = Tick(100);
        let mut character = character(1);
        character.flags.insert(CharacterFlags::PLAYER);
        character.mana = 10 * POWERSCALE;
        character.values[0][CharacterValue::Bless as usize] = 40;
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Bless,
            arg1: 1,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::BLESS_SELF);
        assert_eq!(character.act1, 1);
        assert_eq!(character.mana, 8 * POWERSCALE);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        let completed = world.tick_basic_actions();

        assert_eq!(completed.len(), 1);
        assert!(completed[0].ok);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        let spell_id = character.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.name, "Bless");
        assert_eq!(spell.driver, IDR_BLESS);
        assert_eq!(spell.carried_by, Some(CharacterId(1)));
        assert_eq!(spell.modifier_index[..4], [4, 3, 5, 6]);
        assert_eq!(spell.modifier_value[..4], [10, 10, 10, 10]);
        assert_eq!(
            u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
            2_980
        );
        assert_eq!(
            u32::from_le_bytes(spell.driver_data[4..8].try_into().unwrap()),
            100
        );
        assert_eq!(
            i32::from_le_bytes(spell.driver_data[8..12].try_into().unwrap()),
            40
        );
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_BLESS);
        assert_eq!(effect.target_character, Some(CharacterId(1)));
        assert_eq!(effect.start_tick, 100);
        assert_eq!(effect.stop_tick, 2_980);
        assert_eq!(effect.strength, 10);
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn player_flash_spell_installs_timed_speed_spell_on_self() {
        let mut world = World::default();
        world.tick = Tick(200);
        let mut character = character(1);
        character.flags.insert(CharacterFlags::PLAYER);
        character.mana = 10 * POWERSCALE;
        character.values[0][CharacterValue::Flash as usize] = 40;
        world.add_character(character);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Flash,
            arg1: 0,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.action, action::FLASH);
        assert_eq!(character.mana, 7 * POWERSCALE);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);

        let character = world.characters.get(&CharacterId(1)).unwrap();
        let spell_id = character.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_FLASH);
        assert_eq!(spell.modifier_index[0], CharacterValue::Speed as i16);
        assert_eq!(spell.modifier_value[0], 100);
        assert_eq!(spell.carried_by, Some(CharacterId(1)));
        assert_eq!(
            u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
            248
        );
        assert_eq!(
            u32::from_le_bytes(spell.driver_data[4..8].try_into().unwrap()),
            200
        );
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_FLASH);
        assert_eq!(effect.target_character, Some(CharacterId(1)));
        assert_eq!(effect.start_tick, 200);
        assert_eq!(effect.stop_tick, 248);
        assert_eq!(effect.light, 50);
        assert_eq!(effect.strength, 40);
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn targeted_fireball_sets_up_projectile_action() {
        let mut world = World::default();
        world.tick = Tick(240);
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        caster.values[0][CharacterValue::Tactics as usize] = 24;
        world.spawn_character(caster, 10, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Fireball,
            arg1: 15,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::FIREBALL1);
        assert_eq!(caster.act1, 15);
        assert_eq!(caster.act2, 10);
        assert_eq!(caster.dir, Direction::Right as u8);
        assert_eq!(caster.mana, 7 * POWERSCALE);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);
        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::FIREBALL2);
        assert_eq!(caster.step, 0);
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_FIREBALL);
        assert_eq!(effect.serial, 1);
        assert_eq!(effect.start_tick, 240);
        assert_eq!(effect.stop_tick, 240 + TICKS_PER_SECOND as i32);
        assert_eq!(effect.strength, 53);
        assert_eq!(effect.light, 200);
        assert_eq!(effect.caster, Some(CharacterId(1)));
        assert_eq!(effect.caster_serial, 1);
        assert_eq!((effect.from_x, effect.from_y), (10, 10));
        assert_eq!((effect.to_x, effect.to_y), (15, 10));
        assert_eq!((effect.x, effect.y), (10 * 1024 + 512, 10 * 1024 + 512));
    }

    #[test]
    fn fireball_effect_moves_one_tile_per_tick_and_marks_map_slot() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.x = 10;
        caster.y = 10;
        caster.act1 = 13;
        caster.act2 = 10;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        world.spawn_character(caster, 10, 10);
        let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
        let effect_id = world.create_fireball_effect(&caster);

        world.tick_effects();

        let effect = world.effects.get(&effect_id).unwrap();
        assert_eq!((effect.x, effect.y), (11 * 1024 + 512, 10 * 1024 + 512));
        assert_eq!((effect.last_x, effect.last_y), (11, 10));
        assert_eq!(effect.fields, vec![11 + 10 * world.map.width() as i32]);
        assert_eq!(world.map.tile(11, 10).unwrap().effects[0], effect_id as u16);
    }

    #[test]
    fn fireball_effect_explodes_on_character_block_and_applies_area_damage() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.x = 10;
        caster.y = 10;
        caster.act1 = 15;
        caster.act2 = 10;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        caster.values[0][CharacterValue::Tactics as usize] = 24;
        let mut target = character(2);
        target.flags.insert(CharacterFlags::ALIVE);
        target.hp = 30 * POWERSCALE;
        target.values[0][CharacterValue::Immunity as usize] = 20;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
        let effect_id = world.create_fireball_effect(&caster);

        world.tick_effects();
        world.tick_effects();

        assert!(!world.effects.contains_key(&effect_id));
        assert_eq!(world.map.tile(11, 10).unwrap().effects, [0; 4]);
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 14_100);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
    }

    #[test]
    fn targeted_ball_sets_up_projectile_action() {
        let mut world = World::default();
        world.tick = Tick(300);
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Flash as usize] = 50;
        caster.values[0][CharacterValue::Tactics as usize] = 24;
        world.spawn_character(caster, 10, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Ball,
            arg1: 15,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::BALL1);
        assert_eq!((caster.act1, caster.act2), (15, 10));
        assert_eq!(caster.dir, Direction::Right as u8);
        assert_eq!(caster.mana, 7 * POWERSCALE);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);
        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::BALL2);
        assert_eq!(caster.step, 0);
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_BALL);
        assert_eq!(effect.stop_tick, 300 + TICKS_PER_SECOND as i32 * 5);
        assert_eq!(effect.strength, 53);
        assert_eq!(effect.light, 80);
        assert_eq!((effect.from_x, effect.from_y), (10, 10));
        assert_eq!((effect.to_x, effect.to_y), (15, 10));
    }

    #[test]
    fn earthrain_action_completion_creates_legacy_area_effect() {
        let mut world = World::default();
        world.tick = Tick(400);
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.hp = 10 * POWERSCALE;

        crate::do_action::do_earthrain(&mut caster, 12, 10, 7).unwrap();
        caster.duration = 1;
        world.spawn_character(caster, 10, 10);

        let completion = world.tick_basic_actions().pop().unwrap();
        assert!(completion.ok);

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::IDLE);
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_EARTHRAIN);
        assert_eq!(effect.strength, 7);
        assert_eq!(effect.light, 10);
        assert_eq!(effect.stop_tick, 400 + TICKS_PER_SECOND as i32 * 60);
        assert_eq!(
            world.map.tile(12, 10).unwrap().effects[0],
            effect.serial as u16
        );
    }

    #[test]
    fn earthmud_action_completion_creates_legacy_area_effect() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.hp = 10 * POWERSCALE;

        crate::do_action::do_earthmud(&mut caster, 12, 10, 4).unwrap();
        caster.duration = 1;
        world.spawn_character(caster, 10, 10);

        let completion = world.tick_basic_actions().pop().unwrap();
        assert!(completion.ok);

        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_EARTHMUD);
        assert_eq!(effect.strength, 4);
        assert_eq!(effect.light, 0);
        assert_eq!(
            world.map.tile(12, 10).unwrap().effects[0],
            effect.serial as u16
        );
    }

    #[test]
    fn ball_effect_moves_slowly_and_strikes_nearby_targets() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.x = 10;
        caster.y = 10;
        caster.act1 = 15;
        caster.act2 = 10;
        caster.values[0][CharacterValue::Flash as usize] = 50;
        caster.values[0][CharacterValue::Tactics as usize] = 24;
        let mut target = character(2);
        target.flags.insert(CharacterFlags::ALIVE);
        target.hp = 30 * POWERSCALE;
        target.values[0][CharacterValue::Immunity as usize] = 20;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
        let effect_id = world.create_ball_effect(&caster);

        world.tick_effects();

        let effect = world.effects.get(&effect_id).unwrap();
        assert_eq!((effect.x, effect.y), (10 * 1024 + 640, 10 * 1024 + 512));
        assert_eq!(effect.number_of_enemies, 1);
        let strike = world
            .effects
            .values()
            .find(|effect| effect.effect_type == EF_STRIKE)
            .unwrap();
        assert_eq!(strike.light, 50);
        assert_eq!(strike.strength, 53);
        assert_eq!(strike.target_character, Some(CharacterId(2)));
        assert_eq!((strike.x, strike.y), (10, 10));
        assert_eq!(strike.stop_tick, 2);
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 28_675);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
    }

    #[test]
    fn strike_effect_refreshes_matching_target_and_expires_after_two_ticks() {
        let mut world = World::default();

        let effect_id = world.create_or_refresh_strike_effect(CharacterId(2), 10, 11, 53);
        assert_eq!(world.effects.len(), 1);
        assert_eq!(world.effects.get(&effect_id).unwrap().stop_tick, 2);

        world.tick = Tick(1);
        let refreshed_id = world.create_or_refresh_strike_effect(CharacterId(2), 10, 11, 53);
        assert_eq!(refreshed_id, effect_id);
        assert_eq!(world.effects.len(), 1);
        assert_eq!(world.effects.get(&effect_id).unwrap().stop_tick, 3);

        world.tick = Tick(2);
        world.tick_effects();
        assert!(world.effects.contains_key(&effect_id));

        world.tick = Tick(3);
        world.tick_effects();
        assert!(!world.effects.contains_key(&effect_id));
    }

    #[test]
    fn character_fireball_targets_stationary_character_position() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        let target = character(2);
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 15, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::FireballCharacter,
            arg1: 2,
            arg2: 2,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::FIREBALL1);
        assert_eq!((caster.act1, caster.act2), (15, 10));
        assert_eq!(caster.dir, Direction::Right as u8);
        assert_eq!(caster.mana, 7 * POWERSCALE);
    }

    #[test]
    fn character_fireball_predicts_moving_target_like_c_fireball_driver() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        let mut target = character(2);
        target.action = action::WALK;
        target.dir = Direction::Right as u8;
        target.duration = 8;
        target.step = 1;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 18, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::FireballCharacter,
            arg1: 2,
            arg2: 2,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::FIREBALL1);
        assert_eq!((caster.act1, caster.act2), (20, 10));
        assert_eq!(caster.dir, Direction::Right as u8);
    }

    #[test]
    fn character_fireball_rejects_stale_serial_guard() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        let target = character(2);
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 15, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::FireballCharacter,
            arg1: 2,
            arg2: 99,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::IDLE);
        assert_eq!(caster.mana, 10 * POWERSCALE);
    }

    #[test]
    fn self_targeted_fireball_sets_up_firering_and_damages_adjacent_targets() {
        let mut world = World::default();
        world.tick = Tick(250);
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        caster.values[0][CharacterValue::Tactics as usize] = 24;
        let mut target = character(2);
        target.flags.insert(CharacterFlags::ALIVE);
        target.hp = 30 * POWERSCALE;
        target.values[0][CharacterValue::Immunity as usize] = 20;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 11, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Fireball,
            arg1: 10,
            arg2: 10,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::FIRERING);
        assert_eq!(caster.mana, 7 * POWERSCALE);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        let spell_id = caster.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_FIRERING);
        assert_eq!(spell.carried_by, Some(CharacterId(1)));
        assert_eq!(spell.modifier_index, [0, 0, 0, 0, 0]);
        assert_eq!(
            u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
            274
        );
        assert_eq!(
            u32::from_le_bytes(spell.driver_data[4..8].try_into().unwrap()),
            250
        );
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 14_100);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_FIRERING);
        assert_eq!(effect.target_character, Some(CharacterId(1)));
        assert_eq!(effect.stop_tick, 257);
        assert_eq!(effect.light, 50);
        assert_eq!(effect.strength, 20);
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn player_freeze_spell_installs_negative_speed_spell_on_nearby_target() {
        let mut world = World::default();
        world.tick = Tick(300);
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Freeze as usize] = 50;
        let mut target = character(2);
        target.flags.insert(CharacterFlags::PLAYER);
        target.values[0][CharacterValue::Immunity as usize] = 30;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Freeze,
            arg1: 0,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::FREEZE);
        assert_eq!(caster.mana, 8 * POWERSCALE);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);

        let target = world.characters.get(&CharacterId(2)).unwrap();
        let spell_id = target.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_FREEZE);
        assert_eq!(spell.modifier_index[0], CharacterValue::Speed as i16);
        assert_eq!(spell.modifier_value[0], -420);
        assert_eq!(spell.carried_by, Some(CharacterId(2)));
        assert_eq!(
            u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
            396
        );
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_FREEZE);
        assert_eq!(effect.target_character, Some(CharacterId(2)));
        assert_eq!(effect.start_tick, 300);
        assert_eq!(effect.stop_tick, 396);
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn player_warcry_sets_up_and_debuffs_sound_reachable_targets() {
        let mut world = World::default();
        world.tick = Tick(400);
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.endurance = 30 * POWERSCALE;
        caster.values[0][CharacterValue::Warcry as usize] = 60;
        let mut target = character(2);
        target.flags.insert(CharacterFlags::ALIVE);
        target.hp = 20 * POWERSCALE;
        target.values[0][CharacterValue::Immunity as usize] = 20;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 13, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Warcry,
            arg1: 0,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::WARCRY);
        assert_eq!(caster.endurance, 10 * POWERSCALE);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.lifeshield, 30 * POWERSCALE);
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 16_400);
        let spell_id = target.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_WARCRY);
        assert_eq!(spell.modifier_index[0], CharacterValue::Speed as i16);
        assert_eq!(spell.modifier_value[0], -340);
        assert_eq!(spell.carried_by, Some(CharacterId(2)));
        assert_eq!(
            u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
            496
        );
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_WARCRY);
        assert_eq!(effect.target_character, Some(CharacterId(2)));
        assert_eq!(effect.start_tick, 400);
        assert_eq!(effect.stop_tick, 496);
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn player_warcry_does_not_pass_soundblocking_tiles() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.endurance = 30 * POWERSCALE;
        caster.values[0][CharacterValue::Warcry as usize] = 60;
        let mut target = character(2);
        target.flags.insert(CharacterFlags::ALIVE);
        target.values[0][CharacterValue::Immunity as usize] = 20;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 13, 10);
        for y in 0..world.map.height() {
            world.map.set_flags(11, y, MapFlags::SOUNDBLOCK);
        }
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Warcry,
            arg1: 0,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(!world.tick_basic_actions()[0].ok);

        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert!(target.inventory[12..30].iter().all(Option::is_none));
    }

    #[test]
    fn beyond_potion_installs_timed_potion_spell_and_consumes_potion() {
        let mut world = World::default();
        world.tick = Tick(1_200);
        let mut character = character(1);
        character
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::WARRIOR);
        character.level = 20;
        character.inventory[30] = Some(ItemId(7));
        world.add_character(character);

        let mut potion = item(
            7,
            ItemFlags::USED | ItemFlags::USE | ItemFlags::BEYONDMAXMOD,
        );
        potion.driver = crate::item_driver::IDR_BEYONDPOTION;
        potion.carried_by = Some(CharacterId(1));
        potion.driver_data = vec![3];
        potion.modifier_index = [
            CharacterValue::Strength as i16,
            CharacterValue::Agility as i16,
            0,
            0,
            0,
        ];
        potion.modifier_value = [5, 6, 0, 0, 0];
        world.add_item(potion);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_BEYONDPOTION,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::BeyondPotion { .. }));
        assert!(!world.items.contains_key(&ItemId(7)));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[30], None);
        let spell_id = character.inventory[29].unwrap();
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert!(character.flags.contains(CharacterFlags::UPDATE));
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_POTION_SP);
        assert_eq!(spell.carried_by, Some(CharacterId(1)));
        assert_eq!(spell.modifier_index[0], CharacterValue::Strength as i16);
        assert_eq!(spell.modifier_value[0], 5);
        assert!(spell.flags.contains(ItemFlags::BEYONDMAXMOD));
        assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(5_520));
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_POTION);
        assert_eq!(effect.target_character, Some(CharacterId(1)));
        assert_eq!(effect.start_tick, 1_200);
        assert_eq!(effect.stop_tick, 5_520);
        assert_eq!(effect.strength, 5);
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn beyond_potion_blocks_while_another_potion_spell_is_active() {
        let mut world = World::default();
        let mut character = character(1);
        character.flags.insert(CharacterFlags::PLAYER);
        character.inventory[12] = Some(ItemId(8));
        character.inventory[30] = Some(ItemId(7));
        world.add_character(character);

        let mut active = item(8, ItemFlags::USED);
        active.driver = IDR_POTION_SP;
        active.carried_by = Some(CharacterId(1));
        active.driver_data = 10_000_u32.to_le_bytes().to_vec();
        world.add_item(active);
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE);
        potion.driver = crate::item_driver::IDR_BEYONDPOTION;
        potion.carried_by = Some(CharacterId(1));
        potion.driver_data = vec![3];
        world.add_item(potion);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_BEYONDPOTION,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::BlockedByRequirements { .. }
        ));
        assert!(world.items.contains_key(&ItemId(7)));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[12], Some(ItemId(8)));
        assert_eq!(character.inventory[30], Some(ItemId(7)));
    }

    #[test]
    fn world_spell_timer_removes_carried_spell_at_expiry() {
        let mut world = World::default();
        world.tick = Tick(100);
        let mut character = character(1);
        character.flags.insert(CharacterFlags::PLAYER);
        character.mana = 10 * POWERSCALE;
        character.values[0][CharacterValue::Bless as usize] = 40;
        world.add_character(character);

        assert!(world.setup_bless_spell(CharacterId(1), CharacterId(1)));
        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);
        let spell_id = world.characters.get(&CharacterId(1)).unwrap().inventory[29].unwrap();

        world.tick = Tick(2_979);
        assert!(world.process_due_timers(1).is_empty());
        assert!(world.items.contains_key(&spell_id));
        world.tick = Tick(2_980);
        assert!(world.process_due_timers(1).is_empty());

        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[29], None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert!(character.flags.contains(CharacterFlags::UPDATE));
        assert!(!world.items.contains_key(&spell_id));
    }

    #[test]
    fn world_spell_timer_serial_guard_preserves_refreshed_spell() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[12] = Some(ItemId(7));
        let mut stale_spell = item(7, ItemFlags::USED);
        stale_spell.driver = IDR_BLESS;
        stale_spell.carried_by = Some(CharacterId(1));
        stale_spell.serial = 7;
        stale_spell.driver_data = 10_u32.to_le_bytes().to_vec();
        world.add_character(character);
        world.add_item(stale_spell);

        assert_eq!(world.schedule_existing_spell_timers(), 1);
        world.items.get_mut(&ItemId(7)).unwrap().serial = 8;
        world.tick = Tick(10);
        assert!(world.process_due_timers(1).is_empty());

        assert_eq!(
            world.characters.get(&CharacterId(1)).unwrap().inventory[12],
            Some(ItemId(7))
        );
        assert!(world.items.contains_key(&ItemId(7)));
    }

    #[test]
    fn scheduling_existing_bless_spell_restores_show_effect() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[12] = Some(ItemId(7));
        world.add_character(character);

        let mut spell = item(7, ItemFlags::USED);
        spell.driver = IDR_BLESS;
        spell.carried_by = Some(CharacterId(1));
        spell.modifier_value[0] = 15;
        spell.driver_data = Vec::new();
        spell.driver_data.extend_from_slice(&500_u32.to_le_bytes());
        spell.driver_data.extend_from_slice(&100_u32.to_le_bytes());
        world.add_item(spell);

        assert_eq!(world.schedule_existing_spell_timers(), 1);

        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_BLESS);
        assert_eq!(effect.target_character, Some(CharacterId(1)));
        assert_eq!(effect.start_tick, 100);
        assert_eq!(effect.stop_tick, 500);
        assert_eq!(effect.strength, 15);
    }

    #[test]
    fn player_bless_spell_replaces_near_expired_spell_in_same_slot() {
        let mut world = World::default();
        world.tick = Tick(1_000);
        let mut character = character(1);
        character.flags.insert(CharacterFlags::PLAYER);
        character.mana = 10 * POWERSCALE;
        character.values[0][CharacterValue::Bless as usize] = 80;
        character.inventory[18] = Some(ItemId(7));
        let mut old_spell = item(7, ItemFlags::USED);
        old_spell.driver = IDR_BLESS;
        old_spell.carried_by = Some(CharacterId(1));
        old_spell.driver_data = vec![0; 12];
        old_spell.driver_data[0..4].copy_from_slice(&1_100_u32.to_le_bytes());
        world.add_character(character);
        world.add_item(old_spell);

        assert!(world.setup_bless_spell(CharacterId(1), CharacterId(1)));
        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);

        let character = world.characters.get(&CharacterId(1)).unwrap();
        let new_spell_id = character.inventory[18].unwrap();
        assert_ne!(new_spell_id, ItemId(7));
        assert!(!world.items.contains_key(&ItemId(7)));
        assert_eq!(
            world.items.get(&new_spell_id).unwrap().modifier_value[0],
            20
        );
    }

    #[test]
    fn poison_character_installs_legacy_timed_poison_spell() {
        let mut world = World::default();
        world.tick = Tick(500);
        let mut character = character(1);
        character.hp = 10 * POWERSCALE;
        world.add_character(character);

        assert!(world.poison_character(CharacterId(1), 7, 2));

        let character = world.characters.get(&CharacterId(1)).unwrap();
        let spell_id = character.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.name, "Poison");
        assert_eq!(spell.driver, IDR_POISON2);
        assert_eq!(spell.carried_by, Some(CharacterId(1)));
        assert_eq!(spell.modifier_index[0], CharacterValue::Hp as i16);
        assert_eq!(spell.modifier_value[0], -1);
        assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(173_300));
        assert_eq!(read_poison_power(&spell.driver_data), Some(7));
        assert_eq!(read_poison_tick(&spell.driver_data), Some(9));
        assert_eq!(world.timers.used_timers(), 2);
    }

    #[test]
    fn poison_callback_damages_and_reschedules_while_spell_is_carried() {
        let mut world = World::default();
        world.tick = Tick(1_000);
        let mut character = character(1);
        character.hp = 10 * POWERSCALE;
        world.add_character(character);
        assert!(world.poison_character(CharacterId(1), 4, 0));
        let spell_id = world.characters[&CharacterId(1)].inventory[29].unwrap();

        world.tick = Tick(1_000 + TICKS_PER_SECOND);
        world.process_due_timers(1);

        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.hp, 10 * POWERSCALE - POWERSCALE / 3);
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(read_poison_tick(&spell.driver_data), Some(8));
        assert_eq!(spell.modifier_value[0], -1);
        assert_eq!(world.timers.used_timers(), 2);
    }

    #[test]
    fn poison_callback_weakens_hp_modifier_every_tenth_tick() {
        let mut world = World::default();
        world.tick = Tick(2_000);
        let mut character = character(1);
        character.hp = 10 * POWERSCALE;
        world.add_character(character);
        assert!(world.poison_character(CharacterId(1), 20, 3));
        let spell_id = world.characters[&CharacterId(1)].inventory[29].unwrap();
        write_poison_tick(&mut world.items.get_mut(&spell_id).unwrap().driver_data, 0);

        world.tick = Tick(2_000 + TICKS_PER_SECOND);
        world.process_due_timers(1);

        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_POISON3);
        assert_eq!(spell.modifier_value[0], -2);
        assert_eq!(read_poison_tick(&spell.driver_data), Some(9));
    }

    #[test]
    fn remove_poison_helpers_clear_spell_slots() {
        let mut world = World::default();
        world.add_character(character(1));
        assert!(world.poison_character(CharacterId(1), 5, 1));
        let spell_id = world.characters[&CharacterId(1)].inventory[29].unwrap();

        assert!(world.remove_poison(CharacterId(1), 1));

        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[29], None);
        assert!(!world.items.contains_key(&spell_id));
        assert!(character.flags.contains(CharacterFlags::ITEMS));
    }

    #[test]
    fn special_potion_antidote_clears_matching_poison_and_consumes_item() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(10));
        world.add_character(character);
        assert!(world.poison_character(CharacterId(1), 5, 2));
        let poison_id = world.characters[&CharacterId(1)].inventory[29].unwrap();
        let mut potion = item(10, ItemFlags::USED);
        potion.carried_by = Some(CharacterId(1));
        potion.driver = IDR_SPECIAL_POTION;
        potion.driver_data = vec![2];
        world.items.insert(ItemId(10), potion);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_SPECIAL_POTION,
                item_id: ItemId(10),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::SpecialPotionAntidote {
                kind: 2,
                poison_removed: true,
                ..
            }
        ));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[29], None);
        assert_eq!(character.inventory[30], None);
        assert!(!world.items.contains_key(&poison_id));
        assert!(!world.items.contains_key(&ItemId(10)));
    }

    #[test]
    fn special_potion_infravision_installs_timed_spell_and_consumes_item() {
        let mut world = World::default();
        world.tick = Tick(42);
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(10));
        world.add_character(character);
        let mut potion = item(10, ItemFlags::USED);
        potion.carried_by = Some(CharacterId(1));
        potion.driver = IDR_SPECIAL_POTION;
        potion.driver_data = vec![6];
        world.items.insert(ItemId(10), potion);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_SPECIAL_POTION,
                item_id: ItemId(10),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::SpecialPotionInfravision {
                installed: true,
                ..
            }
        ));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        let spell_id = character.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.name, "Infravision");
        assert_eq!(spell.driver, IDR_INFRARED);
        assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(14_442));
        assert_eq!(character.inventory[30], None);
        assert!(!world.items.contains_key(&ItemId(10)));
    }

    #[test]
    fn player_pulse_damages_low_health_target_and_creates_visible_effects() {
        let mut world = World::default();
        world.tick = Tick(500);
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.mana = 100 * POWERSCALE;
        caster.values[0][CharacterValue::Mana as usize] = 100;
        caster.values[0][CharacterValue::Pulse as usize] = 200;
        let mut target = character(2);
        target.flags.insert(CharacterFlags::ALIVE);
        target.hp = 10 * POWERSCALE;
        target.values[0][CharacterValue::Hp as usize] = 100;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::Pulse,
            arg1: 0,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));
        let mana_after_setup = world.characters.get(&CharacterId(1)).unwrap().mana;
        assert!(mana_after_setup < 100 * POWERSCALE);

        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert!(caster.mana > mana_after_setup);
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert!(target.hp <= 0);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
        assert!(world
            .effects
            .values()
            .any(|effect| effect.effect_type == EF_PULSE && effect.x == 10 && effect.y == 10));
        assert!(world.effects.values().any(|effect| {
            effect.effect_type == EF_PULSEBACK
                && effect.target_character == Some(CharacterId(2))
                && effect.x == 10
                && effect.y == 10
        }));
    }

    fn character(id: u32) -> Character {
        Character {
            id: CharacterId(id),
            name: "Character".into(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
            speed_mode: SpeedMode::Normal,
            x: 0,
            y: 0,
            rest_area: 0,
            rest_x: 0,
            rest_y: 0,
            tox: 0,
            toy: 0,
            dir: 0,
            action: 0,
            duration: 0,
            step: 0,
            act1: 0,
            act2: 0,
            hp: 0,
            mana: 0,
            endurance: 0,
            lifeshield: 0,
            level: 1,
            exp: 0,
            exp_used: 0,
            gold: 0,
            creation_time: 0,
            saves: 0,
            deaths: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
            driver_state: None,
            driver_messages: Vec::new(),
        }
    }

    fn item(id: u32, flags: ItemFlags) -> Item {
        Item {
            id: ItemId(id),
            name: "Item".into(),
            description: String::new(),
            flags,
            sprite: 0,
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
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: 0,
        }
    }
}
