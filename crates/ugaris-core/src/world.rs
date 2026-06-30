use std::collections::HashMap;

use crate::{
    area_sound::AreaSoundSpecial,
    attack::{attack_skill, reduce_hurt_by_armor, spell_average},
    character_driver::{
        add_simple_baddy_enemy, add_simple_baddy_enemy_unchecked, process_simple_baddy_messages,
        CharacterDriverState, SimpleBaddyEnemy, SimpleBaddyMessageOutcome, CDR_SIMPLEBADDY,
        NT_DIDHIT, NT_GOTHIT, NT_SEEHIT,
    },
    direction::Direction,
    do_action::{
        act_attack, act_drop, act_heal, act_magicshield, act_take, act_use, act_walk,
        advance_action_step, can_attack, can_attack_in_area, can_attack_in_area_with_clan_policy,
        do_attack, do_ball, do_bless, do_drop, do_earthmud, do_fireball, do_flash, do_freeze,
        do_heal, do_idle, do_magicshield, do_pulse, do_take, do_use, do_walk, do_warcry,
        endurance_cost, reset_action_after_act, speed_ticks, speed_ticks_inverse, turn,
        ClanAttackPolicy, ItemUseRequest, DUR_MISC_ACTION,
    },
    drvlib::{char_dist, map_dist, step_char_dist, tile_char_dist},
    effect::Effect,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, SpeedMode,
        CHARACTER_VALUE_COUNT, INVENTORY_SIZE, MAX_MODIFIERS, POWERSCALE,
    },
    game_time::GameDate,
    ids::{CharacterId, ItemId},
    item_driver::{
        execute_item_driver_with_context, reset_flask_empty_state, use_item,
        EdemonGateSpawnContext, FdemonGateSpawnContext, ItemDriverContext, ItemDriverOutcome,
        ItemDriverRequest, UseItemError, UseItemOutcome, IDR_BONEWALL, IDR_CALIGAR,
        IDR_CALIGARFLAME, IDR_DUNGEONDOOR, IDR_EDEMONBLOCK, IDR_EDEMONDOOR, IDR_EDEMONGATE,
        IDR_EDEMONLIGHT, IDR_EDEMONLOADER, IDR_EDEMONTUBE, IDR_FDEMONFARM, IDR_FDEMONGATE,
        IDR_FDEMONLIGHT, IDR_FDEMONLOADER, IDR_FLAMETHROW, IDR_LAB3_PLANT, IDR_NIGHTLIGHT,
        IDR_ONOFFLIGHT, IDR_POTION, IDR_RANDOMSHRINE, IDR_STEPTRAP, IDR_TORCH,
        IID_AREA14_SHRINEKEY,
    },
    item_ops::{consume_item, give_item_to_character, GiveItemFlags, GiveItemResult},
    legacy::{action, worn_slot, DIST_MAX, INVENTORY_START_INVENTORY, MAX_FIELD, MAX_MAP},
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
    see::{char_see_char, char_see_item},
    spell::{
        add_same_spell_slot, fireball_damage, freeze_speed_modifier, is_timed_spell_driver,
        may_add_spell, pulse_damage, pulse_spend, read_spell_expire_tick, spell_power,
        strike_damage, warcry_damage, warcry_speed_modifier, BLESS_COST, BLESS_DURATION, EF_BALL,
        EF_BLESS, EF_BUBBLE, EF_BURN, EF_CURSE, EF_EARTHMUD, EF_EARTHRAIN, EF_EDEMONBALL,
        EF_EXPLODE, EF_FIREBALL, EF_FIRERING, EF_FLASH, EF_FREEZE, EF_HEAL, EF_MAGICSHIELD,
        EF_MIST, EF_POTION, EF_PULSE, EF_PULSEBACK, EF_STRIKE, EF_WARCRY, FIREBALL_COST,
        FLASH_COST, FLASH_DURATION, FREEZE_COST, FREEZE_DURATION, IDR_ARMOR, IDR_BLESS, IDR_CURSE,
        IDR_FIRERING, IDR_FLASH, IDR_FREEZE, IDR_HP, IDR_INFRARED, IDR_MANA, IDR_NONOMAGIC,
        IDR_OXYGEN, IDR_POISON0, IDR_POISON3, IDR_POTION_SP, IDR_UWTALK, IDR_WARCRY, IDR_WEAPON,
        POISON_DURATION, SPELL_SLOT_END, SPELL_SLOT_START, WARCRY_DURATION,
    },
    tick::TICKS_PER_SECOND,
    Tick,
};

const IID_REFLECT_FIREBALL: u32 = (0x01 << 24) | 0x00004E;
const IID_AREA6_GREENCRYSTAL: u32 = (0x01 << 24) | 0x000048;
const LEGACY_EQUIPMENT_SLOTS: std::ops::Range<usize> = 0..12;
const IID_HARDKILL: u32 = (0x01 << 24) | 0x00005D;
const EDEMON_GATE_MODE0_POSITIONS: [(u16, u16); 7] = [
    (62, 157),
    (62, 164),
    (62, 174),
    (62, 184),
    (62, 191),
    (56, 174),
    (67, 174),
];
const EDEMON_GATE_MODE1_SLOT_BASE: usize = 404;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSoundSpecial {
    pub character_id: CharacterId,
    pub special: AreaSoundSpecial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSystemText {
    pub character_id: CharacterId,
    pub message: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StafferSpecDoorResult {
    Toggled,
    Locked,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaligarWeightDoorResult {
    Moved,
    Locked,
    Busy,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
enum FightDriverTaskKind {
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
struct FightDriverTask {
    kind: FightDriverTaskKind,
    value: i32,
}

struct RuntimePlayerAttackPolicy<'a> {
    attacker_runtime: &'a PlayerRuntime,
}

impl ClanAttackPolicy for RuntimePlayerAttackPolicy<'_> {
    fn has_pk_hate(&self, _attacker: &Character, defender: &Character) -> bool {
        self.attacker_runtime.has_pk_hate_for(defender.id.0)
    }
}

#[cfg_attr(not(test), allow(dead_code))]
const FIGHT_DRIVER_LOW_PRIO: i32 = 1;
#[cfg_attr(not(test), allow(dead_code))]
const FIGHT_DRIVER_MED_PRIO: i32 = 500;
#[cfg_attr(not(test), allow(dead_code))]
const FIGHT_DRIVER_HIGH_PRIO: i32 = 750;

#[cfg_attr(not(test), allow(dead_code))]
fn order_fight_driver_tasks(
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
fn fight_driver_attackback_may_run(tasks: &[FightDriverTask], index: usize) -> bool {
    tasks
        .get(index + 1)
        .is_some_and(|task| task.kind == FightDriverTaskKind::Attack)
}

fn simple_baddy_enemy_hurtme(enemy: &SimpleBaddyEnemy) -> bool {
    enemy.priority == 1
}

fn item_light_may_have_changed(outcome: &ItemDriverOutcome) -> bool {
    matches!(
        outcome,
        ItemDriverOutcome::LightChanged { .. }
            | ItemDriverOutcome::OnOffLightChanged { .. }
            | ItemDriverOutcome::FlameThrowerPulse { .. }
            | ItemDriverOutcome::FlameThrowerExtinguished { .. }
            | ItemDriverOutcome::DecayItemToggled { .. }
    )
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Area3PalaceLampState {
    pub switched_on_count: i32,
    pub switched_off_count: i32,
    pub keep_open_until_tick: u64,
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
    pub show_attack_debug: bool,
    pub timers: TimerQueue,
    pub scheduler: TaskScheduler,
    pub map: MapGrid,
    pub dirty_sectors: DirtySectors,
    pub characters: HashMap<CharacterId, Character>,
    pub items: HashMap<ItemId, Item>,
    pub effects: HashMap<u32, Effect>,
    pub area3_palace_lamps: Area3PalaceLampState,
    pending_look_maps: Vec<LookMapRequest>,
    pending_sound_specials: Vec<WorldSoundSpecial>,
    pending_system_texts: Vec<WorldSystemText>,
    pending_hurt_events: Vec<LegacyHurtEvent>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TileSpecialOutcome {
    pub damage: i32,
    pub bubble_effect_id: Option<u32>,
    pub sound_type: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LegacyHurtOutcome {
    pub damage_after_armor: i32,
    pub shield_absorbed: i32,
    pub hp_damage: i32,
    pub killed: bool,
    pub nodeath_saved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyHurtEvent {
    pub target_id: CharacterId,
    pub cause_id: CharacterId,
    pub outcome: LegacyHurtOutcome,
}

impl World {
    pub fn advance(&mut self) {
        self.tick.0 += 1;
    }

    pub fn apply_legacy_hurt(
        &mut self,
        target_id: CharacterId,
        cause_id: Option<CharacterId>,
        damage: i32,
        armor_divisor: i32,
        armor_percent: i32,
        shield_percent: i32,
    ) -> Option<LegacyHurtOutcome> {
        let cause_id = cause_id.filter(|id| id.0 != 0 && self.characters.contains_key(id));
        let mut outcome = LegacyHurtOutcome::default();
        let show_attack_debug = self.show_attack_debug;
        let cause_position = cause_id.and_then(|id| {
            self.characters
                .get(&id)
                .map(|character| (character.x, character.y))
        });
        let cause_name = cause_id
            .and_then(|id| self.characters.get(&id))
            .map(|character| character.name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let cause_hardkill_weapon = cause_id.and_then(|id| {
            let item_id = self
                .characters
                .get(&id)?
                .inventory
                .get(worn_slot::RIGHT_HAND)?
                .as_ref()?;
            self.items.get(item_id).map(|item| {
                (
                    item.template_id,
                    item.driver_data.get(37).copied().unwrap_or_default(),
                )
            })
        });
        let has_magicshield_effect = self.effects.values().any(|effect| {
            effect.effect_type == EF_MAGICSHIELD && effect.target_character == Some(target_id)
        });
        let mut create_magicshield_effect = false;

        let mut show_attack_messages = Vec::new();
        let (target_x, target_y, target_was_player, target_was_male) = {
            let target = self.characters.get_mut(&target_id)?;
            if target.flags.contains(CharacterFlags::DEAD) {
                return None;
            }
            let target_was_player = target.flags.contains(CharacterFlags::PLAYER);
            let target_was_male = target.flags.contains(CharacterFlags::MALE);

            let damage = damage.max(0);
            let armor_value = character_value(target, CharacterValue::Armor);
            let mut damage_after_armor =
                reduce_hurt_by_armor(damage, armor_value, armor_divisor, armor_percent);
            outcome.damage_after_armor = damage_after_armor;

            if show_attack_debug && damage != 0 {
                show_attack_messages.push(format!(
                    "hurt by {}, dam={:.2}, armor={:.2} armorper={} shieldper={}",
                    cause_name,
                    damage as f64 / f64::from(POWERSCALE),
                    armor_value as f64 / 20.0 / f64::from(armor_divisor.max(1)),
                    armor_percent,
                    shield_percent
                ));
                if damage_after_armor != 0 {
                    show_attack_messages.push(format!(
                        "dam after armor: {:.2}",
                        damage_after_armor as f64 / f64::from(POWERSCALE)
                    ));
                }
            }

            if target.flags.contains(CharacterFlags::FDEMON)
                && !cause_position.is_some_and(|(x, y)| is_back_attack_against_target(target, x, y))
            {
                damage_after_armor /= 100;
            }

            if target.flags.contains(CharacterFlags::HARDKILL)
                && !cause_hardkill_weapon.is_some_and(|(template_id, level)| {
                    template_id == IID_HARDKILL && u32::from(level) >= target.level
                })
            {
                damage_after_armor = 0;
            }

            if !target.flags.contains(CharacterFlags::IMMORTAL) {
                let mut hp_damage = damage_after_armor;
                if hp_damage != 0 && target.lifeshield != 0 && shield_percent > 0 {
                    let shield_absorbed = (hp_damage * shield_percent / 100).min(target.lifeshield);
                    target.lifeshield -= shield_absorbed;
                    hp_damage -= shield_absorbed;
                    outcome.shield_absorbed = shield_absorbed;
                    create_magicshield_effect = shield_absorbed > 0
                        && character_value_present(target, CharacterValue::MagicShield) != 0
                        && !has_magicshield_effect;
                }

                target.hp -= hp_damage;
                target.flags.insert(CharacterFlags::UPDATE);
                outcome.hp_damage = hp_damage;

                if target.hp < POWERSCALE / 2 {
                    if target.flags.contains(CharacterFlags::NODEATH) {
                        target.hp = 1;
                        outcome.nodeath_saved = true;
                    } else {
                        target.flags.insert(CharacterFlags::DEAD);
                        target.flags.remove(CharacterFlags::ALIVE);
                        target.deaths = target.deaths.saturating_add(1);
                        outcome.killed = true;
                    }
                }
            }

            target.regen_ticker = self.tick.0.min(u64::from(u32::MAX)) as u32;

            target.push_driver_message(
                NT_GOTHIT,
                cause_id.map(|id| id.0 as i32).unwrap_or_default(),
                outcome.hp_damage,
                0,
            );
            (target.x, target.y, target_was_player, target_was_male)
        };

        self.pending_system_texts
            .extend(
                show_attack_messages
                    .into_iter()
                    .map(|message| WorldSystemText {
                        character_id: target_id,
                        message,
                    }),
            );

        if target_was_player && outcome.hp_damage >= POWERSCALE {
            self.queue_sound_area(
                usize::from(target_x),
                usize::from(target_y),
                if target_was_male { 9 } else { 32 },
            );
        }
        if target_was_player && (outcome.killed || outcome.nodeath_saved) {
            self.queue_sound_area(
                usize::from(target_x),
                usize::from(target_y),
                if target_was_male { 4 } else { 33 },
            );
        }

        if create_magicshield_effect {
            self.create_show_effect(
                EF_MAGICSHIELD,
                target_id,
                self.tick.0 as u32,
                self.tick.0 as u32 + 3,
                16,
                0,
            );
        }

        if let Some(cause_id) = cause_id {
            if let Some(cause) = self.characters.get_mut(&cause_id) {
                cause.push_driver_message(NT_DIDHIT, target_id.0 as i32, outcome.hp_damage, 0);
            }
            self.pending_hurt_events.push(LegacyHurtEvent {
                target_id,
                cause_id,
                outcome,
            });
        }

        if outcome.killed {
            if let Some(cause_id) = cause_id {
                self.apply_simple_baddy_death_driver(target_id, cause_id);
            }
        }

        for (id, character) in self.characters.iter_mut() {
            if *id == target_id || Some(*id) == cause_id {
                continue;
            }
            if map_dist(character.x, character.y, target_x, target_y) <= 16 {
                character.push_driver_message(
                    NT_SEEHIT,
                    cause_id.map(|id| id.0 as i32).unwrap_or_default(),
                    target_id.0 as i32,
                    0,
                );
            }
        }

        Some(outcome)
    }

    pub fn drain_legacy_hurt_events(&mut self) -> Vec<LegacyHurtEvent> {
        self.pending_hurt_events.drain(..).collect()
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

    pub fn apply_edemon_gate_spawn_result(
        &mut self,
        item_id: ItemId,
        slot: usize,
        character_id: CharacterId,
        serial: u32,
    ) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        let mode = item.driver_data.first().copied().unwrap_or_default();
        let offset = edemon_gate_slot_offset(mode, slot);
        item.driver_data.resize(offset + 4, 0);
        let character_id = character_id.0 as u16;
        let serial = serial as u16;
        item.driver_data[offset..offset + 2].copy_from_slice(&character_id.to_le_bytes());
        item.driver_data[offset + 2..offset + 4].copy_from_slice(&serial.to_le_bytes());
        true
    }

    pub fn apply_chestspawn_spawn_result(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        _serial: u32,
    ) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(8, 0);
        if item.driver_data[1] != 0 {
            return false;
        }
        item.sprite += 1;
        item.driver_data[1] = 1;
        item.driver_data[2..4].copy_from_slice(&(character_id.0 as u16).to_le_bytes());
        item.driver_data[6..8].copy_from_slice(&0_u16.to_le_bytes());
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        self.mark_dirty_sector(x, y);
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
        true
    }

    fn chestspawn_spawn_alive(&self, character_id: CharacterId) -> bool {
        character_id.0 != 0
            && self
                .characters
                .get(&character_id)
                .is_some_and(|character| !character.flags.contains(CharacterFlags::DEAD))
    }

    fn reset_chestspawn_item(&mut self, item_id: ItemId) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(8, 0);
        if item.driver_data[1] == 0 {
            return false;
        }
        item.sprite -= 1;
        item.driver_data[1] = 0;
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        self.mark_dirty_sector(x, y);
        true
    }

    pub fn apply_fdemon_gate_spawn_result(
        &mut self,
        item_id: ItemId,
        slot: usize,
        character_id: CharacterId,
        _serial: u32,
    ) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if slot >= 3 {
            return false;
        }
        let offset = fdemon_gate_slot_offset(slot);
        item.driver_data.resize(offset + 4, 0);
        let character_id = character_id.0 as u16;
        let serial = 0_u16;
        item.driver_data[offset..offset + 2].copy_from_slice(&character_id.to_le_bytes());
        item.driver_data[offset + 2..offset + 4].copy_from_slice(&serial.to_le_bytes());
        true
    }

    fn edemon_gate_spawn_context(&self, item_id: ItemId) -> Option<EdemonGateSpawnContext> {
        let item = self.items.get(&item_id)?;
        match item.driver_data.first().copied().unwrap_or_default() {
            0 => EDEMON_GATE_MODE0_POSITIONS
                .iter()
                .copied()
                .enumerate()
                .find_map(|(slot, (x, y))| {
                    self.edemon_gate_slot_is_stale(item, 0, slot)
                        .then_some(EdemonGateSpawnContext { slot, x, y })
                }),
            1 => {
                let mut positions = self
                    .items
                    .values()
                    .filter(|candidate| {
                        candidate.driver == IDR_EDEMONLIGHT
                            && candidate.driver_data.first() == Some(&4)
                    })
                    .map(|candidate| (candidate.id, candidate.x, candidate.y))
                    .collect::<Vec<_>>();
                positions.sort_by_key(|(id, _, _)| id.0);
                positions
                    .into_iter()
                    .take(100)
                    .enumerate()
                    .find_map(|(slot, (_, x, y))| {
                        self.edemon_gate_slot_is_stale(item, 1, slot)
                            .then_some(EdemonGateSpawnContext { slot, x, y })
                    })
            }
            _ => None,
        }
    }

    fn edemon_gate_slot_is_stale(&self, item: &Item, mode: u8, slot: usize) -> bool {
        let offset = edemon_gate_slot_offset(mode, slot);
        let Some(bytes) = item.driver_data.get(offset..offset + 4) else {
            return true;
        };
        let character_id = u16::from_le_bytes([bytes[0], bytes[1]]);
        let serial = u16::from_le_bytes([bytes[2], bytes[3]]);
        if character_id == 0 {
            return true;
        }
        self.characters
            .get(&CharacterId(u32::from(character_id)))
            .is_none_or(|character| {
                !character.flags.contains(CharacterFlags::USED)
                    || character.flags.contains(CharacterFlags::DEAD)
                    || character.serial as u16 != serial
            })
    }

    fn fdemon_gate_spawn_context(&self, item_id: ItemId) -> Option<FdemonGateSpawnContext> {
        let item = self.items.get(&item_id)?;
        (0..3).find_map(|slot| {
            self.fdemon_gate_slot_is_stale(item, slot)
                .then_some(FdemonGateSpawnContext {
                    slot,
                    x: item.x,
                    y: item.y,
                })
        })
    }

    fn fdemon_gate_slot_is_stale(&self, item: &Item, slot: usize) -> bool {
        let offset = fdemon_gate_slot_offset(slot);
        let Some(bytes) = item.driver_data.get(offset..offset + 4) else {
            return true;
        };
        let character_id = u16::from_le_bytes([bytes[0], bytes[1]]);
        let _serial = u16::from_le_bytes([bytes[2], bytes[3]]);
        if character_id == 0 {
            return true;
        }
        self.characters
            .get(&CharacterId(u32::from(character_id)))
            .is_none_or(|character| {
                !character.flags.contains(CharacterFlags::USED)
                    || character.flags.contains(CharacterFlags::DEAD)
            })
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

    pub fn queue_sound_area(&mut self, x: usize, y: usize, sound_type: u32) {
        let specials = self.sound_area_specials(x, y, sound_type);
        self.pending_sound_specials.extend(specials);
    }

    pub fn drain_pending_sound_specials(&mut self) -> Vec<WorldSoundSpecial> {
        self.pending_sound_specials.drain(..).collect()
    }

    pub fn drain_pending_system_texts(&mut self) -> Vec<WorldSystemText> {
        self.pending_system_texts.drain(..).collect()
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

    fn move_edemon_block(&mut self, item_id: ItemId, target_x: u16, target_y: u16) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let from_x = usize::from(item.x);
        let from_y = usize::from(item.y);
        let to_x = usize::from(target_x);
        let to_y = usize::from(target_y);
        let Some(target) = self.map.tile(to_x, to_y) else {
            return false;
        };
        if target
            .flags
            .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            || target.item != 0
            || !(12150..=12158).contains(&(target.ground_sprite & 0xffff))
        {
            return false;
        }

        if let Some(source) = self.map.tile_mut(from_x, from_y) {
            if source.item == item_id.0 {
                source.item = 0;
                source.flags.remove(MapFlags::TMOVEBLOCK);
                self.mark_dirty_sector(from_x, from_y);
            }
        }
        if let Some(target) = self.map.tile_mut(to_x, to_y) {
            target.item = item_id.0;
            target.flags.insert(MapFlags::TMOVEBLOCK);
            self.mark_dirty_sector(to_x, to_y);
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.x = target_x;
            item.y = target_y;
        }
        true
    }

    fn pulse_edemon_tube(&mut self, item_id: ItemId, target_x: u16, target_y: u16) {
        if target_x == 0 || target_y == 0 {
            return;
        }
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let item_x = i32::from(item.x);
        let item_y = i32::from(item.y);
        let targets: Vec<_> = self
            .characters
            .values()
            .filter(|character| {
                character
                    .flags
                    .contains(CharacterFlags::USED | CharacterFlags::PLAYER)
                    && (i32::from(character.x) - item_x).abs() <= 10
                    && (i32::from(character.y) - item_y).abs() <= 10
                    && char_see_item(character, &item, &self.map, self.date.daylight)
            })
            .map(|character| character.id)
            .collect();

        for character_id in targets {
            if self.teleport_character(character_id, target_x, target_y, false) {
                self.pending_system_texts.push(WorldSystemText {
                    character_id,
                    message: "The strange tube teleported you.".to_string(),
                });
            }
        }
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

    fn create_reflected_fireball_effect(
        &mut self,
        reflector: &Character,
        caster: &Character,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_FIREBALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND) as i32,
        );
        effect.strength = strength;
        effect.light = 200;
        effect.from_x = i32::from(reflector.x);
        effect.from_y = i32::from(reflector.y);
        effect.to_x = i32::from(caster.x);
        effect.to_y = i32::from(caster.y);
        effect.caster = Some(reflector.id);
        effect.caster_serial = reflector.id.0 as i32;
        effect.x = i32::from(reflector.x) * 1024 + 512;
        effect.y = i32::from(reflector.y) * 1024 + 512;
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

    fn create_fireball_machine_effect(
        &mut self,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        power: u8,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_FIREBALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND) as i32,
        );
        effect.strength = i32::from(power);
        effect.light = 200;
        effect.from_x = i32::from(start_x);
        effect.from_y = i32::from(start_y);
        effect.to_x = i32::from(target_x);
        effect.to_y = i32::from(target_y);
        effect.x = i32::from(start_x) * 1024 + 512;
        effect.y = i32::from(start_y) * 1024 + 512;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    fn create_edemonball_effect(
        &mut self,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        strength: i32,
        base_sprite: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_EDEMONBALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 4) as i32,
        );
        effect.strength = strength;
        effect.from_x = i32::from(start_x);
        effect.from_y = i32::from(start_y);
        effect.to_x = i32::from(target_x);
        effect.to_y = i32::from(target_y);
        effect.x = i32::from(start_x) * 1024 + 512;
        effect.y = i32::from(start_y) * 1024 + 512;
        effect.base_sprite = base_sprite;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    fn create_caligar_gun_effects(&mut self, item_id: ItemId, direction: u8) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let item_x = i32::from(item.x);
        let item_y = i32::from(item.y);
        let shots: &[(i32, i32, i32, i32)] = match direction {
            1 => &[(1, 0, 10, 0)],
            2 => &[(0, 1, 0, 10)],
            3 => &[(-1, 0, -10, 0)],
            4 => &[(0, -1, 0, -10)],
            5 => &[
                (0, 1, 0, 10),
                (1, 0, 10, 0),
                (0, -1, 0, -10),
                (-1, 0, -10, 0),
            ],
            _ => return false,
        };
        for (start_dx, start_dy, target_dx, target_dy) in shots {
            self.create_edemonball_effect(
                clamp_world_coordinate(item_x + start_dx),
                clamp_world_coordinate(item_y + start_dy),
                clamp_world_coordinate(item_x + target_dx),
                clamp_world_coordinate(item_y + target_dy),
                50,
                1,
            );
        }
        true
    }

    fn find_edemonball_target_shot(
        &self,
        item_id: ItemId,
        strength: i32,
        base_sprite: i32,
    ) -> Option<ItemDriverOutcome> {
        let item = self.items.get(&item_id)?;
        let item_x = i32::from(item.x);
        let item_y = i32::from(item.y);

        let mut candidates: Vec<_> = self
            .characters
            .values()
            .filter(|character| {
                (i32::from(character.x) - item_x).abs() <= 10
                    && (i32::from(character.y) - item_y).abs() <= 10
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);

        for character in candidates {
            let (ox, oy) = if (i32::from(character.x) - item_x).abs()
                > (i32::from(character.y) - item_y).abs()
            {
                ((i32::from(character.x) - item_x).signum(), 0)
            } else {
                (0, (i32::from(character.y) - item_y).signum())
            };
            let (target_x, target_y) = self.predict_edemonball_target(item, character);
            let start_x = item_x + ox;
            let start_y = item_y + oy;
            if self.edemonball_can_hit(item_id, character.id, start_x, start_y, target_x, target_y)
            {
                return Some(ItemDriverOutcome::EdemonBallProjectile {
                    item_id,
                    character_id: CharacterId(0),
                    start_x: start_x.clamp(0, i32::from(u16::MAX)) as u16,
                    start_y: start_y.clamp(0, i32::from(u16::MAX)) as u16,
                    target_x: target_x.clamp(0, i32::from(u16::MAX)) as u16,
                    target_y: target_y.clamp(0, i32::from(u16::MAX)) as u16,
                    strength,
                    base_sprite,
                    schedule_after_ticks: TICKS_PER_SECOND * 8,
                });
            }
        }

        None
    }

    fn predict_edemonball_target(&self, item: &Item, character: &Character) -> (i32, i32) {
        if character.action != action::WALK {
            return (i32::from(character.x), i32::from(character.y));
        }

        let Ok(direction) = Direction::try_from(character.dir) else {
            return (i32::from(character.x), i32::from(character.y));
        };
        let (dx, dy) = direction.delta();
        let dist = map_dist(item.x, item.y, character.x, character.y);
        let mut eta = dist * 3 / 2;
        eta -= character.duration - character.step;
        if eta <= 0 {
            return (i32::from(character.tox), i32::from(character.toy));
        }

        for step in 1..10 {
            eta -= character.duration;
            if eta <= 0 {
                return (
                    i32::from(character.x) + i32::from(dx) * step,
                    i32::from(character.y) + i32::from(dy) * step,
                );
            }
        }

        (i32::from(character.x), i32::from(character.y))
    }

    fn edemonball_can_hit(
        &self,
        item_id: ItemId,
        target_id: CharacterId,
        from_x: i32,
        from_y: i32,
        target_x: i32,
        target_y: i32,
    ) -> bool {
        let mut x = from_x * 1024 + 512;
        let mut y = from_y * 1024 + 512;
        let mut dx = target_x - from_x;
        let mut dy = target_y - from_y;

        if dx.abs() < 2 && dy.abs() < 2 {
            return false;
        }

        if dx.abs() > dy.abs() {
            dy = dy * 256 / dx.abs();
            dx = dx * 256 / dx.abs();
        } else {
            dx = dx * 256 / dy.abs();
            dy = dy * 256 / dy.abs();
        }

        for _ in 0..48 {
            x += dx;
            y += dy;
            let tile_x = x / 1024;
            let tile_y = y / 1024;
            if tile_x == target_x && tile_y == target_y {
                return true;
            }

            let (Ok(tile_x_usize), Ok(tile_y_usize)) =
                (usize::try_from(tile_x), usize::try_from(tile_y))
            else {
                return false;
            };
            let Some(tile) = self.map.tile(tile_x_usize, tile_y_usize) else {
                return false;
            };
            let item_blocks = tile.item != 0
                && tile.item != item_id.0
                && tile.flags.contains(MapFlags::TMOVEBLOCK);
            let map_blocks = !tile.flags.contains(MapFlags::FIRETHRU)
                && tile.flags.contains(MapFlags::MOVEBLOCK);
            let blocked = tile.character != 0 || item_blocks || map_blocks;
            if blocked {
                return tile.character == target_id.0 as u16;
            }
        }

        true
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
        self.tick_effects_with_attack_policy(|_caster_id, caster, target, map| {
            can_attack(caster, target, map)
        });
    }

    pub fn tick_effects_with_attack_policy(
        &mut self,
        mut can_effect_attack: impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
        let mut state = self.tick.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.tick_effects_with_random_and_attack_policy(
            |limit| {
                if limit <= 0 {
                    return 0;
                }
                state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                (state % limit as u64) as i32
            },
            &mut can_effect_attack,
        );
    }

    pub fn tick_effects_with_random(&mut self, mut random_below: impl FnMut(i32) -> i32) {
        self.tick_effects_with_random_and_attack_policy(
            &mut random_below,
            |_, caster, target, map| can_attack(caster, target, map),
        );
    }

    pub fn tick_effects_with_random_and_attack_policy(
        &mut self,
        mut random_below: impl FnMut(i32) -> i32,
        mut can_effect_attack: impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
        let effect_ids: Vec<u32> = self.effects.keys().copied().collect();
        for effect_id in effect_ids {
            match self
                .effects
                .get(&effect_id)
                .map(|effect| effect.effect_type)
            {
                Some(EF_FIREBALL) => self.tick_fireball_effect(effect_id, &mut can_effect_attack),
                Some(EF_BALL) => self.tick_ball_effect(effect_id, &mut can_effect_attack),
                Some(EF_EDEMONBALL) => self.tick_edemonball_effect(effect_id),
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
            let armor_percent = 50 - reduction.min(50);
            targets.push((target_id, damage, armor_percent));
        }

        for (target_id, damage, armor_percent) in targets {
            self.apply_legacy_hurt(
                target_id,
                None,
                damage,
                8,
                armor_percent,
                armor_percent + 25,
            );
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
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };
        let Some(target_id) = effect.target_character else {
            self.effects.remove(&effect_id);
            return;
        };
        if self.tick.0 >= effect.stop_tick as u64
            || !self
                .characters
                .get(&target_id)
                .is_some_and(|character| character.flags.contains(CharacterFlags::USED))
        {
            self.effects.remove(&effect_id);
            return;
        }

        if effect.strength != 0 {
            self.apply_legacy_hurt(
                target_id,
                None,
                POWERSCALE / 6 + effect.strength,
                30,
                50,
                75,
            );
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

    fn tick_ball_effect(
        &mut self,
        effect_id: u32,
        can_effect_attack: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
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
        self.apply_ball_strikes(effect_id, tile_x, tile_y, can_effect_attack);
    }

    fn apply_ball_strikes(
        &mut self,
        effect_id: u32,
        x: i32,
        y: i32,
        can_effect_attack: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
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
                if !can_effect_attack(caster_id, &caster, target, &self.map) {
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
        if !targets.is_empty() && self.tick.0 & 7 == 0 {
            self.queue_sound_area(usize::from(caster.x), usize::from(caster.y), 30);
        }
        for (target_id, damage) in targets {
            self.create_or_refresh_strike_effect(target_id, x, y, effect.strength);
            if damage == 0 {
                continue;
            }
            self.apply_legacy_hurt(target_id, Some(caster_id), damage, 100, 30, 85);
        }
    }

    fn tick_fireball_effect(
        &mut self,
        effect_id: u32,
        can_effect_attack: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
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
            self.explode_fireball_effect(
                effect_id,
                effect.x / 1024,
                effect.y / 1024,
                can_effect_attack,
            );
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
                self.explode_fireball_effect(effect_id, tile_x, tile_y, can_effect_attack);
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

    fn tick_edemonball_effect(&mut self, effect_id: u32) {
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
            self.explode_edemonball_effect(effect_id, effect.x / 1024, effect.y / 1024);
            return;
        }

        let (step_x, step_y) = if raw_dx.abs() > raw_dy.abs() {
            (raw_dx * 256 / raw_dx.abs(), raw_dy * 256 / raw_dx.abs())
        } else {
            (raw_dx * 256 / raw_dy.abs(), raw_dy * 256 / raw_dy.abs())
        };

        let last_x = effect.x / 1024;
        let last_y = effect.y / 1024;
        let x = effect.x + step_x;
        let y = effect.y + step_y;
        let tile_x = x / 1024;
        let tile_y = y / 1024;

        if self.edemonball_map_blocked(tile_x, tile_y)
            && !self.fire_tile_contains_caster(effect.caster, tile_x, tile_y)
        {
            if let Some(effect) = self.effects.get_mut(&effect_id) {
                effect.x = x;
                effect.y = y;
                effect.last_x = last_x;
                effect.last_y = last_y;
            }
            let has_character = self
                .map
                .tile(
                    usize::try_from(tile_x).unwrap_or_default(),
                    usize::try_from(tile_y).unwrap_or_default(),
                )
                .is_some_and(|tile| tile.character != 0);
            let (explode_x, explode_y) = if has_character {
                (tile_x, tile_y)
            } else {
                (last_x, last_y)
            };
            self.explode_edemonball_effect(effect_id, explode_x, explode_y);
            return;
        }

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.x = x;
            effect.y = y;
            effect.last_x = last_x;
            effect.last_y = last_y;
        }
        self.set_effect_on_map(effect_id, tile_x, tile_y);
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

    fn edemonball_map_blocked(&self, x: i32, y: i32) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return true;
        };
        let Some(tile) = self.map.tile(x, y) else {
            return true;
        };
        tile.character != 0
            || (tile.item != 0 && tile.flags.contains(MapFlags::TMOVEBLOCK))
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

    fn explode_fireball_effect(
        &mut self,
        effect_id: u32,
        x: i32,
        y: i32,
        can_effect_attack: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
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
                if !can_effect_attack(caster_id, &caster, target, &self.map) {
                    return;
                }
                let target = target.clone();
                if self.reflect_fireball_from_target(&target, &caster, effect.strength) {
                    return;
                }
                if target.flags.contains(CharacterFlags::EDEMON) {
                    self.create_reflected_fireball_effect(&target, &caster, effect.strength - 1);
                }
                let has_tactics = character_value_present(&target, CharacterValue::Tactics) != 0;
                let damage = fireball_damage(
                    effect.strength,
                    character_value(&target, CharacterValue::Immunity),
                    character_value(&target, CharacterValue::Tactics),
                    has_tactics,
                );
                targets.push((target_id, damage));
            }
        }

        for (target_id, damage) in targets {
            self.apply_legacy_hurt(target_id, Some(caster_id), damage, 10, 50, 70);
        }

        self.create_explosion_effect(x, y, 8, 50050);
        self.queue_sound_area(x as usize, y as usize, 6);
    }

    fn explode_edemonball_effect(&mut self, effect_id: u32, x: i32, y: i32) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };
        self.remove_effect_from_map(effect_id);
        self.effects.remove(&effect_id);

        if x < 1 || x >= self.map.width().saturating_sub(1) as i32 {
            return;
        }
        if y < 1 || y >= self.map.height().saturating_sub(1) as i32 {
            return;
        }

        if let Some(target_id) = self.map.tile(x as usize, y as usize).and_then(|tile| {
            (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
        }) {
            let may_damage = self.characters.get(&target_id).is_some_and(|target| {
                if effect.base_sprite == 2
                    && target
                        .flags
                        .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
                {
                    return false;
                }
                match effect.caster {
                    Some(caster_id) => self
                        .characters
                        .get(&caster_id)
                        .is_some_and(|caster| can_attack(caster, target, &self.map)),
                    None => true,
                }
            });
            if may_damage {
                let strength = if effect.base_sprite == 0 {
                    self.absorb_edemonball_with_green_crystal(target_id, effect.strength)
                } else {
                    effect.strength
                };
                let damage = strength.saturating_mul(POWERSCALE);
                self.apply_legacy_hurt(target_id, effect.caster, damage, 6, 75, 50);
            }
        }

        self.create_explosion_effect(x, y, 8, 50450 + effect.base_sprite);
    }

    fn absorb_edemonball_with_green_crystal(
        &mut self,
        target_id: CharacterId,
        mut strength: i32,
    ) -> i32 {
        let Some(target) = self.characters.get(&target_id) else {
            return strength;
        };
        let mut candidates = Vec::with_capacity(INVENTORY_SIZE - 29);
        if let Some(item_id) = target.cursor_item {
            candidates.push(item_id);
        }
        candidates.extend(target.inventory[30..].iter().filter_map(|&item_id| item_id));

        for item_id in candidates {
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            if item.template_id != IID_AREA6_GREENCRYSTAL {
                continue;
            }
            let crystal_power = item.driver_data.first().copied().unwrap_or_default() as i32;
            if strength > crystal_power {
                strength -= crystal_power;
                self.destroy_item(item_id);
                continue;
            }

            let mut sprite_changed = false;
            if let Some(item) = self.items.get_mut(&item_id) {
                item.driver_data.resize(1, 0);
                item.driver_data[0] = (crystal_power - strength).clamp(0, u8::MAX as i32) as u8;
                let sprite = 50318 + 5 - (i32::from(item.driver_data[0]) / 42);
                if item.sprite != sprite {
                    item.sprite = sprite;
                    sprite_changed = true;
                }
            }
            if sprite_changed {
                if let Some(target) = self.characters.get_mut(&target_id) {
                    target.flags.insert(CharacterFlags::ITEMS);
                }
            }
            return 0;
        }

        strength
    }

    fn reflect_fireball_from_target(
        &mut self,
        target: &Character,
        caster: &Character,
        strength: i32,
    ) -> bool {
        let Some((slot, item_id, charges)) = LEGACY_EQUIPMENT_SLOTS.clone().find_map(|slot| {
            let item_id = *target.inventory.get(slot)?.as_ref()?;
            let item = self.items.get(&item_id)?;
            (item.template_id == IID_REFLECT_FIREBALL)
                .then(|| (slot, item_id, read_u32_le_prefix(&item.driver_data)))
        }) else {
            return false;
        };

        let used_charges = strength.max(0) as u32;
        if charges <= used_charges {
            if let Some(target) = self.characters.get_mut(&target.id) {
                if target.inventory.get(slot) == Some(&Some(item_id)) {
                    target.inventory[slot] = None;
                }
            }
            self.items.remove(&item_id);
        } else if let Some(item) = self.items.get_mut(&item_id) {
            let remaining = charges - used_charges;
            write_u32_le_prefix(&mut item.driver_data, remaining);
            item.description = format!("{remaining} units left.");
            item.flags.insert(ItemFlags::FORCEUPDATE);
        }

        self.create_reflected_fireball_effect(target, caster, strength - 1);
        true
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
                IDR_ONOFFLIGHT if item.driver_data.first().copied().unwrap_or(0) != 0 => {
                    Some(item_id)
                }
                IDR_TORCH if item.driver_data.first().copied().unwrap_or(0) != 0 => Some(item_id),
                IDR_FLAMETHROW | IDR_CALIGARFLAME | IDR_EDEMONLIGHT | IDR_EDEMONLOADER
                | IDR_EDEMONBLOCK | IDR_EDEMONTUBE | IDR_FDEMONLIGHT | IDR_FDEMONLOADER
                | IDR_FDEMONGATE | IDR_FDEMONFARM => Some(item_id),
                IDR_CALIGAR if matches!(item.driver_data.first().copied(), Some(2 | 4)) => {
                    Some(item_id)
                }
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

    fn schedule_registered_area3_lamp_extinguish(&mut self) -> usize {
        let mut item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter_map(|(&item_id, item)| {
                (item.driver == IDR_ONOFFLIGHT
                    && item.driver_data.get(6).copied().unwrap_or_default() != 0)
                    .then_some(item_id)
            })
            .collect();
        item_ids.sort_by_key(|item_id| item_id.0);

        item_ids
            .into_iter()
            .enumerate()
            .filter(|(index, item_id)| {
                self.schedule_item_driver_timer(*item_id, CharacterId(0), (*index as u64) + 1)
            })
            .count()
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
                if cadence % 12 == 0 {
                    self.queue_sound_area(x, y, 44);
                }
                return TileSpecialOutcome {
                    damage: 0,
                    bubble_effect_id: Some(bubble_effect_id),
                    sound_type: (cadence % 12 == 0).then_some(44),
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
        self.queue_sound_area(x, y, 66);
        TileSpecialOutcome {
            damage,
            bubble_effect_id: None,
            sound_type: Some(66),
        }
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
        self.complete_attack_with_rolls_and_clash_roll(
            attacker_id,
            defender_id,
            d100_roll,
            d6_roll,
            d100_roll.rem_euclid(2),
        )
    }

    pub fn complete_attack_with_rolls_and_clash_roll(
        &mut self,
        attacker_id: CharacterId,
        defender_id: CharacterId,
        d100_roll: i32,
        d6_roll: i32,
        clash_roll: i32,
    ) -> bool {
        if attacker_id == defender_id {
            return false;
        }
        let Some((attacker_x, attacker_y, attacker_rhand)) =
            self.characters.get(&attacker_id).map(|attacker| {
                (
                    usize::from(attacker.x),
                    usize::from(attacker.y),
                    attacker.inventory[worn_slot::RIGHT_HAND].is_some(),
                )
            })
        else {
            return false;
        };
        let Some(mut defender) = self.characters.remove(&defender_id) else {
            return false;
        };
        let defender_rhand = defender.inventory[worn_slot::RIGHT_HAND].is_some();
        let resolution = self.characters.get_mut(&attacker_id).and_then(|attacker| {
            act_attack(attacker, &mut defender, &self.map, d100_roll, d6_roll)
        });
        self.characters.insert(defender_id, defender);
        let Some(resolution) = resolution else {
            return false;
        };
        if self.show_attack_debug {
            if let Some(defender) = self.characters.get(&defender_id) {
                self.pending_system_texts.push(WorldSystemText {
                    character_id: attacker_id,
                    message: format!(
                        "attack {}, diff={} ({} {}), chan={}, percent={}, dam={}",
                        defender.name,
                        resolution.attack_skill - resolution.parry_skill,
                        resolution.attack_skill,
                        resolution.parry_skill,
                        resolution.hit_chance,
                        resolution.armor_percent,
                        resolution.raw_damage * resolution.armor_divisor / POWERSCALE,
                    ),
                });
            }
        }
        let sound_type = if resolution.hit {
            7
        } else if !attacker_rhand || !defender_rhand {
            8
        } else if clash_roll.rem_euclid(2) == 0 {
            34
        } else {
            35
        };
        self.queue_sound_area(attacker_x, attacker_y, sound_type);
        if resolution.hit {
            self.apply_legacy_hurt(
                defender_id,
                Some(attacker_id),
                resolution.raw_damage,
                resolution.armor_divisor,
                resolution.armor_percent,
                resolution.shield_percent,
            );
        }
        true
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

    fn dungeon_door_context(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> (bool, bool, u16) {
        let Some(item) = self.items.get(&item_id) else {
            return (false, false, 0);
        };
        let key1 = read_u32_le_at(&item.driver_data, 0);
        let key2 = read_u32_le_at(&item.driver_data, 4);
        let has_key1 = key1 == 0 || self.character_has_template_id(character_id, key1);
        let has_key2 = key2 == 0 || self.character_has_template_id(character_id, key2);

        let catacomb = ((usize::from(item.x).saturating_sub(2)) / 81)
            + ((usize::from(item.y).saturating_sub(2)) / 81) * 3;
        let xf = (catacomb % 3) * 81 + 2;
        let yf = (catacomb / 3) * 81 + 2;
        let mut defenders = 0u16;
        for x in xf..xf + 80 {
            for y in yf..yf + 80 {
                let Some(tile) = self.map.tile(x, y) else {
                    continue;
                };
                if tile.character == 0 {
                    continue;
                }
                if let Some(character) =
                    self.characters.get(&CharacterId(u32::from(tile.character)))
                {
                    if !character.flags.contains(CharacterFlags::PLAYER) {
                        defenders = defenders.saturating_add(1);
                    }
                }
            }
        }

        (has_key1, has_key2, defenders)
    }

    fn character_has_template_id(&self, character_id: CharacterId, template_id: u32) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        character
            .cursor_item
            .into_iter()
            .chain(character.inventory.iter().flatten().copied())
            .any(|item_id| {
                self.items
                    .get(&item_id)
                    .is_some_and(|item| item.template_id == template_id)
            })
    }

    fn has_matching_random_shrine_key(
        &self,
        character_id: CharacterId,
        shrine_item_id: ItemId,
    ) -> bool {
        let Some(shrine) = self.items.get(&shrine_item_id) else {
            return false;
        };
        let required_level = shrine.driver_data.get(1).copied().unwrap_or(0);
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };

        character
            .inventory
            .iter()
            .flatten()
            .copied()
            .chain(character.cursor_item)
            .any(|item_id| {
                self.items.get(&item_id).is_some_and(|item| {
                    item.template_id == IID_AREA14_SHRINEKEY
                        && item.driver_data.first().copied().unwrap_or(0) == required_level
                })
            })
    }

    pub fn execute_item_driver_request_with_context(
        &mut self,
        request: ItemDriverRequest,
        area_id: u16,
        context: &ItemDriverContext,
    ) -> ItemDriverOutcome {
        let (driver, character_id, item_id) = match request {
            ItemDriverRequest::Driver {
                driver,
                character_id,
                item_id,
                ..
            } => (Some(driver), character_id, item_id),
            ItemDriverRequest::AccountDepot {
                character_id,
                item_id,
            } => (None, character_id, item_id),
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
                    item.template_id,
                    item.driver,
                    item.sprite,
                    item.driver_data.first().copied().unwrap_or(0),
                )
            });
        let fdemon_loader_power = (driver == Some(IDR_FDEMONLIGHT)
            && context.fdemon_loader_power.is_none())
        .then(|| fdemon_loader_power_for_light(&self.items, item_id))
        .flatten();
        let edemon_section_power = (matches!(
            driver,
            Some(IDR_EDEMONLIGHT | IDR_EDEMONDOOR | IDR_EDEMONTUBE)
        ) && context.edemon_section_power.is_none())
        .then(|| edemon_section_power_for_light(&self.items, item_id))
        .flatten();
        let edemon_tube_target = (driver == Some(IDR_EDEMONTUBE)
            && context.edemon_tube_target.is_none())
        .then(|| edemon_tube_target(&self.items, &self.map, item_id))
        .flatten();
        let edemon_gate_spawn = (driver == Some(IDR_EDEMONGATE)
            && context.edemon_gate_spawn.is_none())
        .then(|| self.edemon_gate_spawn_context(item_id))
        .flatten();
        let fdemon_gate_spawn = (driver == Some(IDR_FDEMONGATE)
            && context.fdemon_gate_spawn.is_none())
        .then(|| self.fdemon_gate_spawn_context(item_id))
        .flatten();
        let dungeon_door_context = (driver == Some(IDR_DUNGEONDOOR))
            .then(|| self.dungeon_door_context(character_id, item_id));
        let random_shrine_key_context = (driver == Some(IDR_RANDOMSHRINE)
            && !context.has_matching_random_shrine_key)
            .then(|| self.has_matching_random_shrine_key(character_id, item_id))
            .unwrap_or(false);
        let Some(character) = self.characters.get_mut(&character_id) else {
            return ItemDriverOutcome::Noop;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return ItemDriverOutcome::Noop;
        };
        let mut effective_context = context.clone();
        effective_context.current_tick = self.tick.0 as u32;
        if effective_context.fdemon_loader_power.is_none() {
            effective_context.fdemon_loader_power = fdemon_loader_power;
        }
        if effective_context.edemon_section_power.is_none() {
            effective_context.edemon_section_power = edemon_section_power;
        }
        if effective_context.edemon_tube_target.is_none() {
            effective_context.edemon_tube_target = edemon_tube_target;
        }
        if effective_context.edemon_gate_spawn.is_none() {
            effective_context.edemon_gate_spawn = edemon_gate_spawn;
        }
        if effective_context.fdemon_gate_spawn.is_none() {
            effective_context.fdemon_gate_spawn = fdemon_gate_spawn;
        }
        if let Some((has_key1, has_key2, defender_count)) = dungeon_door_context {
            effective_context.has_dungeon_door_key1 |= has_key1;
            effective_context.has_dungeon_door_key2 |= has_key2;
            effective_context.dungeon_defender_count = effective_context
                .dungeon_defender_count
                .or(Some(defender_count));
        }
        effective_context.has_matching_random_shrine_key |= random_shrine_key_context;
        if let Some((cursor_template_id, cursor_driver, cursor_sprite, cursor_drdata0)) =
            cursor_context
        {
            effective_context.cursor_template_id = effective_context
                .cursor_template_id
                .or(Some(cursor_template_id));
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

    pub fn process_simple_baddy_message_actions(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> Vec<ItemDriverOutcome> {
        let tick = self.tick.0;
        self.process_simple_baddy_message_actions_with_random(character_id, area_id, |limit| {
            if limit <= 0 {
                0
            } else {
                ((tick + u64::from(character_id.0)) % limit as u64) as i32
            }
        })
    }

    pub fn process_simple_baddy_message_actions_with_random(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        mut random_below: impl FnMut(i32) -> i32,
    ) -> Vec<ItemDriverOutcome> {
        let carried_items: Vec<Item> = self.items.values().cloned().collect();
        let Some(character) = self.characters.get_mut(&character_id) else {
            return Vec::new();
        };
        if character.action != action::IDLE || character.flags.contains(CharacterFlags::DEAD) {
            return Vec::new();
        }
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
                    } else {
                        self.clear_simple_baddy_bless_friend(character_id);
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
                        let tracking = self.simple_baddy_enemy_tracking(character_id, target_id);
                        if let Some(character) = self.characters.get_mut(&character_id) {
                            let _ = add_simple_baddy_enemy(character, &caller, target_id, tick);
                            Self::apply_simple_baddy_enemy_tracking(character, target_id, tracking);
                        }
                        self.sort_simple_baddy_enemies_like_c(character_id);
                    }
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
                SimpleBaddyMessageOutcome::NoteHit => {
                    if let Some(CharacterDriverState::SimpleBaddy(data)) = self
                        .characters
                        .get_mut(&character_id)
                        .and_then(|character| character.driver_state.as_mut())
                    {
                        data.last_hit = self.tick.0 as i32;
                    }
                }
            }
        }
        applied
    }

    pub fn process_simple_baddy_attack_action(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> bool {
        self.process_simple_baddy_attack_action_with_random(character_id, area_id, |_| 1)
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
        if attacker.driver != CDR_SIMPLEBADDY
            || attacker.action != 0
            || attacker.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }

        let enemies = self.refresh_simple_baddy_enemy_tracking(&attacker);
        if enemies.is_empty() {
            return false;
        }
        let mut visible_enemies: Vec<_> = enemies
            .iter()
            .filter(|enemy| enemy.visible)
            .copied()
            .collect();
        visible_enemies.sort_by(|left, right| {
            self.simple_baddy_visible_enemy_score(&attacker, right)
                .cmp(&self.simple_baddy_visible_enemy_score(&attacker, left))
        });

        for enemy in visible_enemies {
            let previous_lastfight = self
                .simple_baddy_lastfight(character_id)
                .unwrap_or_default();
            let Some(target) = self.characters.get(&enemy.target_id).cloned() else {
                continue;
            };
            if !can_attack_in_area(&attacker, &target, &self.map, area_id) {
                continue;
            }
            if self.setup_simple_baddy_weighted_fight_task(
                character_id,
                &target,
                area_id,
                &mut random,
            ) {
                self.queue_simple_baddy_attack_sound(character_id, previous_lastfight);
                return true;
            }
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
                return true;
            }
            self.remove_simple_baddy_enemy(character_id, enemy.target_id);
        }

        false
    }

    fn simple_baddy_visible_enemy_score(
        &self,
        attacker: &Character,
        enemy: &SimpleBaddyEnemy,
    ) -> i32 {
        let Some(target) = self.characters.get(&enemy.target_id) else {
            return i32::MIN;
        };
        let mut score = (999 - char_dist(attacker, target)) * 10;
        if simple_baddy_enemy_hurtme(enemy) {
            score += 100_000;
        }
        if character_is_facing(attacker, target) {
            score += 5;
        }
        score
    }

    fn setup_simple_baddy_attack_move(
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

    fn setup_adjacent_use_toward_target(
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
        do_use(character, &self.map, item, direction as u8, 0).is_ok()
    }

    fn setup_simple_baddy_attack_driver(
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

    fn simple_baddy_lastfight(&self, character_id: CharacterId) -> Option<i32> {
        let character = self.characters.get(&character_id)?;
        let CharacterDriverState::SimpleBaddy(data) = character.driver_state.as_ref()?;
        Some(data.lastfight)
    }

    fn queue_simple_baddy_attack_sound(
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

    fn setup_simple_baddy_earthmud_attack(
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
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_earthmud(character, target_x, target_y, strength).is_err() {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    fn simple_baddy_earthmud_value(&self, target: &Character) -> i32 {
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

    fn simple_baddy_can_place_earthmud(&self, x: usize, y: usize) -> bool {
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

    fn simple_baddy_can_heal_self(&self, character: &Character) -> bool {
        character_value(character, CharacterValue::Heal) > 1
            && character.mana >= POWERSCALE * 2
            && character.hp < character_value(character, CharacterValue::Hp) * POWERSCALE / 2
    }

    fn setup_simple_baddy_heal_action(&mut self, character_id: CharacterId) -> bool {
        let Some(target) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_heal_self(&target) {
            return false;
        }
        self.setup_simple_baddy_spell_action(character_id, |character, _items, _tick| {
            do_heal(character, &target, None)
        })
    }

    fn simple_baddy_can_magicshield_self(&self, character: &Character) -> bool {
        character_value(character, CharacterValue::MagicShield) > 1
            && character.mana >= POWERSCALE * 2
            && character.lifeshield
                < character_value(character, CharacterValue::MagicShield) * POWERSCALE / 2
    }

    fn setup_simple_baddy_magicshield_action(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_magicshield_self(&character) {
            return false;
        }
        self.setup_simple_baddy_spell_action(character_id, |character, _items, _tick| {
            do_magicshield(character)
        })
    }

    fn simple_baddy_can_bless_self(&self, character: &Character) -> bool {
        character_value(character, CharacterValue::Bless) > 1
            && character.mana >= BLESS_COST
            && may_add_spell(character, &self.items, IDR_BLESS, self.tick.0 as u32).is_some()
    }

    fn setup_simple_baddy_self_bless_action(&mut self, character_id: CharacterId) -> bool {
        let Some(target) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_bless_self(&target) {
            return false;
        }
        self.setup_simple_baddy_spell_action(character_id, |character, items, tick| {
            do_bless(character, &target, items, tick, None)
        })
    }

    fn simple_baddy_needs_regeneration(&self, character: &Character) -> bool {
        character.mana < character_value(character, CharacterValue::Mana) * POWERSCALE
            || character.hp < character_value(character, CharacterValue::Hp) * POWERSCALE
    }

    fn setup_simple_baddy_regenerate_action(&mut self, character_id: CharacterId) -> bool {
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

    fn simple_baddy_regenerate_task_value(&self, character: &Character) -> i32 {
        let base = character_value(character, CharacterValue::Fireball)
            .max(character_value(character, CharacterValue::Flash))
            .max(character_value(character, CharacterValue::Freeze))
            .max(character_value(character, CharacterValue::Attack))
            * 2;
        let last_hit = match character.driver_state.as_ref() {
            Some(CharacterDriverState::SimpleBaddy(data)) => data.last_hit,
            _ => 0,
        };
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

    fn simple_baddy_freeze_modifier(&self, attacker: &Character, target: &Character) -> i32 {
        freeze_speed_modifier(
            spell_power(
                character_value(attacker, CharacterValue::Freeze),
                character_value(attacker, CharacterValue::Tactics),
            ),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            character_value_present(target, CharacterValue::Tactics) != 0,
            attacker.flags.contains(CharacterFlags::IDEMON),
            character_value(attacker, CharacterValue::Demon),
            character_value(target, CharacterValue::Cold),
        )
    }

    fn setup_simple_baddy_freeze_attack(
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
        self.setup_simple_baddy_spell_action(character_id, |character, _items, _tick| {
            do_freeze(character)
        })
    }

    fn setup_simple_baddy_ball_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let target_x = usize::from(target.x).saturating_sub(1)
            + usize::try_from(random(3).min(2)).unwrap_or(0);
        let target_y = usize::from(target.y).saturating_sub(1)
            + usize::try_from(random(3).min(2)).unwrap_or(0);
        self.setup_simple_baddy_spell_action(character_id, |character, items, tick| {
            do_ball(character, items, target_x, target_y, tick)
        })
    }

    fn simple_baddy_calc_ball_steps(
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

    fn ball_path_blocked_for_caster(&self, x: i32, y: i32, caster_id: CharacterId) -> bool {
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

    fn setup_simple_baddy_flash_attack(
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
        self.setup_simple_baddy_spell_action(character_id, |character, items, tick| {
            do_flash(character, items, tick)
        })
    }

    fn simple_baddy_can_warcry(&self, attacker: &Character, target: &Character) -> bool {
        if character_value(attacker, CharacterValue::Warcry) <= 1
            || attacker.endurance
                < character_value(attacker, CharacterValue::Warcry) * POWERSCALE / 3
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

    fn setup_simple_baddy_warcry_attack(
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
        self.setup_simple_baddy_spell_action(character_id, |character, items, _tick| {
            do_warcry(character, items)
        })
    }

    #[allow(dead_code)]
    fn setup_simple_baddy_distance_attack(
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
                character_value(&attacker, CharacterValue::Demon),
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
    fn setup_simple_baddy_fireball_distance_attack(
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

    fn setup_simple_baddy_distance_driver(
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

    fn setup_simple_baddy_attack_back_move(
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

    fn simple_baddy_flee_eval_path(&self, x: usize, y: usize, direction: Direction) -> i32 {
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

    fn setup_simple_baddy_pulse_attack(&mut self, character_id: CharacterId) -> bool {
        if self.simple_baddy_pulse_value(character_id) == 0 {
            return false;
        }

        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_pulse(character).is_err() {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    fn setup_simple_baddy_weighted_fight_task(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let mut tasks = self.simple_baddy_fight_tasks(character_id, target, area_id, false);
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
                FightDriverTaskKind::Fireball | FightDriverTaskKind::FireRing => {
                    self.setup_simple_baddy_fireball_attack(character_id, target, area_id)
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
                FightDriverTaskKind::Flee | FightDriverTaskKind::EarthRain => false,
            };
            if ret {
                return true;
            }
        }

        false
    }

    fn simple_baddy_fight_tasks(
        &self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
        nomove: bool,
    ) -> Vec<FightDriverTask> {
        let Some(attacker) = self.characters.get(&character_id) else {
            return Vec::new();
        };
        let mut tasks = Vec::new();
        let character_distance = char_dist(attacker, target);
        let tile_distance = tile_char_dist(attacker, target);
        let current_tick = self.tick.0 as u32;
        let target_has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;

        if character_value(attacker, CharacterValue::Freeze) > 1
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
        if self.simple_baddy_can_heal_self(attacker) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Heal,
                value: FIGHT_DRIVER_HIGH_PRIO + character_value(attacker, CharacterValue::Heal),
            });
        }
        if self.simple_baddy_can_magicshield_self(attacker) {
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
        if self.simple_baddy_can_bless_self(attacker) {
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
        if character_value(attacker, CharacterValue::Fireball) > 1
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
            if character_distance > 10 && character_distance < 30 && ball_reaches_target {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Ball,
                    value: FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Flash),
                });
            }
            if tile_distance < 4
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
        if self.simple_baddy_can_warcry(attacker, target) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Warcry,
                value: FIGHT_DRIVER_HIGH_PRIO
                    + character_value(attacker, CharacterValue::Warcry) / 2,
            });
        }
        if !nomove || character_distance == 2 {
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
        let distance3 = self.simple_baddy_distance3_task_value(attacker, target);
        if distance3 > 0 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Distance3,
                value: distance3,
            });
        }
        let distance7 = self.simple_baddy_distance7_task_value(attacker, target);
        if distance7 > 0 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Distance7,
                value: distance7,
            });
        }
        let pulse = self.simple_baddy_pulse_value(character_id);
        if pulse > 0 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Pulse,
                value: FIGHT_DRIVER_HIGH_PRIO + pulse,
            });
        }
        if self.simple_baddy_attackback_value(character_id, target) > 0 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::AttackBack,
                value: FIGHT_DRIVER_HIGH_PRIO,
            });
        }

        tasks
    }

    fn simple_baddy_pulse_value(&self, character_id: CharacterId) -> i32 {
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

    fn simple_baddy_pulse_field_value(
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

    fn setup_simple_baddy_fireball_attack(
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

        let Some(attacker_mut) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_fireball(
            attacker_mut,
            &self.items,
            target_x,
            target_y,
            self.tick.0 as u32,
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

    fn setup_simple_baddy_fireball_lane_move(
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

    fn simple_baddy_fireball_lane_task(
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

    fn setup_simple_baddy_lane_walk(
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

    fn fireball_line_hits_target(
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

    fn fireball_block_hits_recorded_enemy(
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

    fn simple_baddy_recorded_enemy_ids(&self, character_id: CharacterId) -> Vec<CharacterId> {
        self.characters
            .get(&character_id)
            .and_then(|character| match character.driver_state.as_ref()? {
                CharacterDriverState::SimpleBaddy(data) => Some(
                    data.enemies
                        .iter()
                        .map(|enemy| enemy.target_id)
                        .collect::<Vec<_>>(),
                ),
            })
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    fn setup_simple_baddy_spell_attack(
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
                character_value(&attacker, CharacterValue::Demon),
                character_value(target, CharacterValue::Cold),
            );
            if modifier < -10 {
                return self
                    .setup_simple_baddy_spell_action(character_id, |character, _items, _tick| {
                        do_freeze(character)
                    });
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
                return self
                    .setup_simple_baddy_spell_action(character_id, |character, items, tick| {
                        do_ball(character, items, target_x, target_y, tick)
                    });
            }

            if tile_distance < 4
                && may_add_spell(&attacker, &self.items, IDR_FLASH, current_tick).is_some()
            {
                return self
                    .setup_simple_baddy_spell_action(character_id, |character, items, tick| {
                        do_flash(character, items, tick)
                    });
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
                return self
                    .setup_simple_baddy_spell_action(character_id, |character, items, _tick| {
                        do_warcry(character, items)
                    });
            }
        }

        false
    }

    fn setup_simple_baddy_spell_action(
        &mut self,
        character_id: CharacterId,
        action: impl FnOnce(
            &mut Character,
            &HashMap<ItemId, Item>,
            u32,
        ) -> Result<(), crate::do_action::DoError>,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if action(character, &self.items, self.tick.0 as u32).is_err() {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    fn simple_baddy_distance3_task_value(&self, attacker: &Character, target: &Character) -> i32 {
        let current_tick = self.tick.0 as u32;
        let mut value = 0;
        if attacker.mana > POWERSCALE * 3
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
        if attacker.mana > POWERSCALE * 3
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

    fn simple_baddy_distance7_task_value(&self, attacker: &Character, target: &Character) -> i32 {
        if attacker.mana <= FIREBALL_COST
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

    fn simple_baddy_attackback_value(&self, character_id: CharacterId, target: &Character) -> i32 {
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

    fn simple_baddy_enemy_tracking(
        &self,
        character_id: CharacterId,
        target_id: CharacterId,
    ) -> Option<(bool, u16, u16)> {
        let character = self.characters.get(&character_id)?;
        let target = self.characters.get(&target_id)?;
        let visible = char_see_char(character, target, &self.map, self.date.daylight);
        Some((visible, target.x, target.y))
    }

    fn apply_simple_baddy_enemy_tracking(
        character: &mut Character,
        target_id: CharacterId,
        tracking: Option<(bool, u16, u16)>,
    ) {
        let Some((visible, last_x, last_y)) = tracking else {
            return;
        };
        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() else {
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

    fn refresh_simple_baddy_enemy_tracking(
        &mut self,
        attacker: &Character,
    ) -> Vec<SimpleBaddyEnemy> {
        let enemies = match attacker.driver_state.as_ref() {
            Some(CharacterDriverState::SimpleBaddy(data)) => data.enemies.clone(),
            _ => return Vec::new(),
        };
        let mut updated = Vec::new();
        for mut enemy in enemies {
            let Some(target) = self.characters.get(&enemy.target_id).cloned() else {
                continue;
            };
            if target.flags.contains(CharacterFlags::DEAD)
                || !can_attack(&attacker, &target, &self.map)
                || self.simple_baddy_enemy_past_stop_dist(&attacker, &target)
            {
                continue;
            }
            enemy.visible = char_see_char(attacker, &target, &self.map, self.date.daylight);
            if enemy.visible {
                enemy.last_x = target.x;
                enemy.last_y = target.y;
            }
            updated.push(enemy);
        }

        if let Some(character) = self.characters.get_mut(&attacker.id) {
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
                data.enemies = updated.clone();
            }
        }
        self.sort_simple_baddy_enemies_like_c(attacker.id);
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get(&attacker.id)
            .and_then(|character| character.driver_state.as_ref())
        {
            return data.enemies.clone();
        }
        updated
    }

    fn sort_simple_baddy_enemies_like_c(&mut self, character_id: CharacterId) {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return;
        };
        let mut enemies = match attacker.driver_state.as_ref() {
            Some(CharacterDriverState::SimpleBaddy(data)) => data.enemies.clone(),
            _ => return,
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
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.enemies = enemies;
        }
    }

    fn simple_baddy_enemy_past_stop_dist(&self, character: &Character, target: &Character) -> bool {
        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_ref() else {
            return false;
        };
        data.stopdist != 0 && self.simple_baddy_target_home_dist(character, target) > data.stopdist
    }

    pub fn set_simple_baddy_home(&mut self, character_id: CharacterId, x: u16, y: u16) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() else {
            return false;
        };
        data.home_x = x;
        data.home_y = y;
        true
    }

    fn remove_simple_baddy_enemy(&mut self, character_id: CharacterId, target_id: CharacterId) {
        if let Some(character) = self.characters.get_mut(&character_id) {
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
                data.enemies.retain(|enemy| enemy.target_id != target_id);
            }
        }
    }

    pub fn process_simple_baddy_attack_actions(&mut self, area_id: u16) -> usize {
        self.process_simple_baddy_attack_actions_with_random(area_id, |_| 1)
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
                (character.driver == CDR_SIMPLEBADDY
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

    pub fn process_simple_baddy_noncombat_action(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> bool {
        self.process_simple_baddy_noncombat_action_with_random_and_context(
            character_id,
            area_id,
            0,
            0,
            |_| 0,
        )
    }

    pub fn process_simple_baddy_noncombat_action_with_random(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        mut random_below: impl FnMut(i32) -> i32,
    ) -> bool {
        self.process_simple_baddy_noncombat_action_with_random_and_context(
            character_id,
            area_id,
            0,
            0,
            &mut random_below,
        )
    }

    pub fn process_simple_baddy_noncombat_action_with_context(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        ret: i32,
        last_action: u16,
    ) -> bool {
        self.process_simple_baddy_noncombat_action_with_random_and_context(
            character_id,
            area_id,
            ret,
            last_action,
            |_| 0,
        )
    }

    pub fn process_simple_baddy_noncombat_action_with_random_and_context(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        ret: i32,
        last_action: u16,
        mut random_below: impl FnMut(i32) -> i32,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_ref() else {
            return false;
        };
        if character.driver != CDR_SIMPLEBADDY
            || character.action != 0
            || character.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }

        let current_tick = self.tick.0 as i32;
        if current_tick - data.creation_time < TICKS_PER_SECOND as i32 {
            return self
                .characters
                .get_mut(&character_id)
                .is_some_and(|character| {
                    do_idle(character, (TICKS_PER_SECOND / 4) as i32).is_ok()
                });
        }

        if data.scavenger != 0 {
            let Some((target_x, target_y)) = character
                .rest_x
                .ne(&0)
                .then_some((character.rest_x, character.rest_y))
            else {
                return self.idle_simple_baddy(character_id);
            };
            let scavenger_distance = data.scavenger.max(0) as u16;
            if character.x.abs_diff(target_x) >= scavenger_distance
                || character.y.abs_diff(target_y) >= scavenger_distance
            {
                let min_dist = if data.notsecure != 0 {
                    data.mindist.max(0) as usize
                } else {
                    0
                };
                if self.setup_walk_toward(
                    character_id,
                    usize::from(target_x),
                    usize::from(target_y),
                    min_dist,
                    area_id,
                    false,
                ) || self.setup_walk_toward(
                    character_id,
                    usize::from(target_x),
                    usize::from(target_y),
                    min_dist,
                    area_id,
                    true,
                ) {
                    return true;
                }
            }
            if self.regenerate_simple_baddy(character_id) {
                return true;
            }
            if self.spell_self_simple_baddy(character_id) {
                return true;
            }
            if self.setup_pending_simple_baddy_friend_bless(character_id) {
                return true;
            }
            if random_below(2) == 0 {
                return self.idle_simple_baddy(character_id);
            }

            let direction = if data.dir != 0 {
                data.dir
            } else {
                random_below(8).clamp(0, 7) + 1
            };
            let Some(direction) = u8::try_from(direction)
                .ok()
                .and_then(|direction| Direction::try_from(direction).ok())
            else {
                self.clear_simple_baddy_scavenger_direction(character_id);
                return self.idle_simple_baddy(character_id);
            };
            let (dx, dy) = direction.delta();
            let next_x = i32::from(character.x) + i32::from(dx);
            let next_y = i32::from(character.y) + i32::from(dy);
            if (next_x - i32::from(target_x)).abs() < i32::from(scavenger_distance)
                && (next_y - i32::from(target_y)).abs() < i32::from(scavenger_distance)
                && self.setup_walk_direction(character_id, direction, area_id)
            {
                let _ = self.set_simple_baddy_home(character_id, character.x, character.y);
                if let Some(CharacterDriverState::SimpleBaddy(data)) = self
                    .characters
                    .get_mut(&character_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    data.dir = direction as i32;
                }
                return true;
            }

            self.clear_simple_baddy_scavenger_direction(character_id);
            self.drink_special_poison_simple_baddy(character_id);
            return self.regenerate_simple_baddy(character_id)
                || self.spell_self_simple_baddy(character_id)
                || self.setup_pending_simple_baddy_friend_bless(character_id)
                || self.idle_simple_baddy(character_id);
        }

        let target = if data.dayx != 0 {
            if self.date.hour > 19 || self.date.hour < 6 {
                Some((data.nightx, data.nighty, data.nightdir))
            } else {
                Some((data.dayx, data.dayy, data.daydir))
            }
        } else if character.rest_x != 0 {
            Some((i32::from(character.rest_x), i32::from(character.rest_y), 0))
        } else {
            None
        };

        let Some((target_x, target_y, target_dir)) = target.filter(|(x, y, _)| *x > 0 && *y > 0)
        else {
            self.drink_special_poison_simple_baddy(character_id);
            return self.regenerate_simple_baddy(character_id)
                || self.spell_self_simple_baddy(character_id)
                || self.setup_pending_simple_baddy_friend_bless(character_id)
                || self.idle_simple_baddy(character_id);
        };
        let target_x = target_x as u16;
        let target_y = target_y as u16;
        if character.x == target_x && character.y == target_y {
            if let Some(CharacterDriverState::SimpleBaddy(data)) = self
                .characters
                .get_mut(&character_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                data.home_x = target_x;
                data.home_y = target_y;
            }
            if target_dir != 0 {
                if let Some(character) = self.characters.get_mut(&character_id) {
                    let _ = turn(character, target_dir as u8);
                }
            }
            return self.regenerate_simple_baddy(character_id)
                || self.spell_self_simple_baddy(character_id)
                || self.setup_pending_simple_baddy_friend_bless(character_id)
                || self.idle_simple_baddy(character_id);
        }

        if data.teleport != 0 && self.teleport_character(character_id, target_x, target_y, false) {
            let _ = self.set_simple_baddy_home(character_id, target_x, target_y);
            return true;
        }

        if data.notsecure == 0
            && current_tick - data.lastfight > (TICKS_PER_SECOND * 10) as i32
            && self.secure_move_driver(
                character_id,
                target_x,
                target_y,
                target_dir as u8,
                ret,
                last_action,
                area_id,
            )
        {
            return true;
        }

        let min_dist = if data.notsecure != 0 {
            data.mindist.max(0) as usize
        } else {
            0
        };
        let (walk_x, walk_y) = if data.notsecure != 0 && character.rest_x != 0 {
            (character.rest_x, character.rest_y)
        } else {
            (target_x, target_y)
        };
        if self.setup_walk_toward(
            character_id,
            usize::from(walk_x),
            usize::from(walk_y),
            min_dist,
            area_id,
            false,
        ) || self.setup_walk_toward(
            character_id,
            usize::from(walk_x),
            usize::from(walk_y),
            min_dist,
            area_id,
            true,
        ) {
            return true;
        }

        let _ = self.set_simple_baddy_home(character_id, character.x, character.y);
        self.drink_special_poison_simple_baddy(character_id);
        self.regenerate_simple_baddy(character_id)
            || self.spell_self_simple_baddy(character_id)
            || self.setup_pending_simple_baddy_friend_bless(character_id)
            || self.idle_simple_baddy(character_id)
    }

    pub fn process_simple_baddy_noncombat_actions(&mut self, area_id: u16) -> usize {
        self.process_simple_baddy_noncombat_actions_with_completions(area_id, &[])
    }

    pub fn process_simple_baddy_noncombat_actions_with_completions(
        &mut self,
        area_id: u16,
        completions: &[WorldActionCompletion],
    ) -> usize {
        let character_ids: Vec<_> = self
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                (character.driver == CDR_SIMPLEBADDY
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
                let (ret, last_action) = completions
                    .iter()
                    .rev()
                    .find(|completion| completion.character_id == character_id)
                    .map(|completion| (completion.legacy_return_code, completion.action_id))
                    .unwrap_or((0, 0));
                self.process_simple_baddy_noncombat_action_with_context(
                    character_id,
                    area_id,
                    ret,
                    last_action,
                )
            })
            .count()
    }

    fn idle_simple_baddy(&mut self, character_id: CharacterId) -> bool {
        self.characters
            .get_mut(&character_id)
            .is_some_and(|character| do_idle(character, TICKS_PER_SECOND as i32).is_ok())
    }

    fn regenerate_simple_baddy(&mut self, character_id: CharacterId) -> bool {
        self.characters
            .get_mut(&character_id)
            .is_some_and(|character| {
                let max_mana = character_value(character, CharacterValue::Mana) * POWERSCALE;
                let max_hp = character_value(character, CharacterValue::Hp) * POWERSCALE;
                if character.mana < max_mana || character.hp < max_hp {
                    do_idle(character, TICKS_PER_SECOND as i32).is_ok()
                } else {
                    false
                }
            })
    }

    fn spell_self_simple_baddy(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let current_tick = self.tick.0 as u32;

        if character_value(&character, CharacterValue::Bless) > 0
            && character.mana >= BLESS_COST
            && may_add_spell(&character, &self.items, IDR_BLESS, current_tick).is_some()
        {
            return self
                .characters
                .get_mut(&character_id)
                .is_some_and(|caster| {
                    do_bless(caster, &character, &self.items, current_tick, None).is_ok()
                });
        }

        if character_value(&character, CharacterValue::MagicShield) * POWERSCALE
            > character.lifeshield
            && character.mana >= POWERSCALE * 3
        {
            return self
                .characters
                .get_mut(&character_id)
                .is_some_and(|character| do_magicshield(character).is_ok());
        }

        if character_value(&character, CharacterValue::Heal) > 0
            && character.hp < character_value(&character, CharacterValue::Hp) * POWERSCALE / 2
            && character.mana >= POWERSCALE * 3
        {
            return self
                .characters
                .get_mut(&character_id)
                .is_some_and(|caster| do_heal(caster, &character, None).is_ok());
        }

        false
    }

    fn remember_simple_baddy_bless_friend(
        &mut self,
        character_id: CharacterId,
        target_id: CharacterId,
    ) {
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.pending_bless_friend = Some(target_id);
        }
    }

    fn clear_simple_baddy_bless_friend(&mut self, character_id: CharacterId) {
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.pending_bless_friend = None;
        }
    }

    fn setup_pending_simple_baddy_friend_bless(&mut self, character_id: CharacterId) -> bool {
        let target_id = self
            .characters
            .get(&character_id)
            .and_then(|character| character.driver_state.as_ref())
            .and_then(|state| match state {
                CharacterDriverState::SimpleBaddy(data) => data.pending_bless_friend,
            });
        let Some(target_id) = target_id else {
            return false;
        };

        self.clear_simple_baddy_bless_friend(character_id);
        self.simple_baddy_can_bless_friend(character_id, target_id)
            && self.setup_bless_spell(character_id, target_id)
    }

    fn drink_special_poison_simple_baddy(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_ref() else {
            return;
        };
        if data.drinkspecial == 0 {
            return;
        }
        let has_poison0 = character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
            .iter()
            .flatten()
            .any(|item_id| {
                self.items
                    .get(item_id)
                    .is_some_and(|item| item.driver == IDR_POISON0)
            });
        if has_poison0 {
            self.remove_all_poison(character_id);
        }
    }

    fn clear_simple_baddy_scavenger_direction(&mut self, character_id: CharacterId) {
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.dir = 0;
        }
    }

    pub fn apply_simple_baddy_death_driver(
        &mut self,
        dead_id: CharacterId,
        killer_id: CharacterId,
    ) -> Vec<u32> {
        let Some(dead) = self.characters.get(&dead_id).cloned() else {
            return Vec::new();
        };
        let Some(killer) = self.characters.get(&killer_id).cloned() else {
            return Vec::new();
        };
        if dead.driver != CDR_SIMPLEBADDY
            || !dead.flags.contains(CharacterFlags::EDEMON)
            || !char_see_char(&dead, &killer, &self.map, self.date.daylight)
        {
            return Vec::new();
        }

        let strength = character_value_present(&dead, CharacterValue::Demon);
        let mut effects = Vec::new();
        if strength > 5 {
            effects.push(self.create_earthmud_effect(
                i32::from(killer.x),
                i32::from(killer.y),
                strength,
            ));
        }
        effects.push(self.create_earthrain_effect(
            i32::from(killer.x),
            i32::from(killer.y),
            strength,
        ));
        effects
    }

    fn simple_baddy_can_poison_hit(
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

    fn simple_baddy_can_bless_friend(
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
            && may_add_spell(target, &self.items, IDR_BLESS, self.tick.0 as u32).is_some()
    }

    fn simple_baddy_can_add_standard_enemy(
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
            && self.simple_baddy_enemy_within_start_limits(character, target)
    }

    fn simple_baddy_enemy_within_start_limits(
        &self,
        character: &Character,
        target: &Character,
    ) -> bool {
        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_ref() else {
            return false;
        };
        if data.startdist != 0
            && self.simple_baddy_target_home_dist(character, target) > data.startdist
        {
            return false;
        }
        if data.chardist != 0 && char_dist(character, target) > data.chardist {
            return false;
        }
        true
    }

    fn simple_baddy_target_home_dist(&self, character: &Character, target: &Character) -> i32 {
        let (home_x, home_y) = match character.driver_state.as_ref() {
            Some(CharacterDriverState::SimpleBaddy(data)) if data.home_x != 0 => {
                (data.home_x, data.home_y)
            }
            _ if character.rest_x != 0 => (character.rest_x, character.rest_y),
            _ => (character.x, character.y),
        };
        map_dist(home_x, home_y, target.x, target.y)
    }

    fn simple_baddy_seen_hit_enemy(
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

        let mut effective_context = context.clone();
        if matches!(driver, IDR_EDEMONLIGHT | IDR_EDEMONDOOR | IDR_EDEMONTUBE)
            && effective_context.edemon_section_power.is_none()
        {
            effective_context.edemon_section_power =
                edemon_section_power_for_light(&self.items, item_id);
        }
        if driver == IDR_EDEMONTUBE && effective_context.edemon_tube_target.is_none() {
            effective_context.edemon_tube_target =
                edemon_tube_target(&self.items, &self.map, item_id);
        }
        if driver == IDR_EDEMONGATE && effective_context.edemon_gate_spawn.is_none() {
            effective_context.edemon_gate_spawn = self.edemon_gate_spawn_context(item_id);
        }
        if driver == IDR_FDEMONGATE && effective_context.fdemon_gate_spawn.is_none() {
            effective_context.fdemon_gate_spawn = self.fdemon_gate_spawn_context(item_id);
        }
        if driver == IDR_FDEMONLIGHT && effective_context.fdemon_loader_power.is_none() {
            effective_context.fdemon_loader_power =
                fdemon_loader_power_for_light(&self.items, item_id);
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
            &effective_context,
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
            ItemDriverOutcome::PentBossDoor {
                item_id,
                character_id,
                x,
                y,
            } => {
                if self.teleport_character(character_id, x, y, false) {
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        character.dir = match character.dir {
                            1 => 5,
                            5 => 1,
                            7 => 3,
                            3 => 7,
                            other => other,
                        };
                    }
                    outcome
                } else {
                    ItemDriverOutcome::PentBossDoorBusy {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::DungeonDoorSolved { character_id, .. } => {
                if [(245, 250), (240, 250), (235, 250), (230, 250)]
                    .into_iter()
                    .any(|(x, y)| self.teleport_character(character_id, x, y, false))
                {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::ClanSpawnExit {
                item_id,
                character_id,
                area_id,
                x,
                y,
            } => {
                if area_id != current_area_id {
                    return outcome;
                }
                if self.teleport_character(character_id, x, y, false) {
                    outcome
                } else {
                    ItemDriverOutcome::ClanSpawnExitBusy {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::StafferAnimationBook { character_id, .. } => {
                if self.teleport_character(character_id, 25, 114, true) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::ForestSpadeCollapse {
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
            ItemDriverOutcome::PickDoorToggle {
                item_id,
                character_id,
            } => {
                if self.toggle_pick_door(item_id, character_id) == DoorToggleResult::Toggled {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StafferSpecDoorToggle {
                item_id,
                character_id,
                kind,
            } => match self.toggle_staffer_spec_door(item_id, character_id, kind) {
                StafferSpecDoorResult::Toggled => outcome,
                StafferSpecDoorResult::Locked => ItemDriverOutcome::StafferSpecDoorLocked {
                    item_id,
                    character_id,
                },
                StafferSpecDoorResult::Blocked | StafferSpecDoorResult::Failed => {
                    ItemDriverOutcome::Noop
                }
            },
            ItemDriverOutcome::EdemonDoorToggle {
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
            ItemDriverOutcome::EdemonBlockMove {
                item_id,
                target_x,
                target_y,
                schedule_after_ticks,
                ..
            } => {
                let moved = self.move_edemon_block(item_id, target_x, target_y);
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                if moved {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::EdemonBlockBlocked { .. } => outcome,
            ItemDriverOutcome::EdemonTubePulse {
                item_id,
                x,
                y,
                schedule_after_ticks,
                ..
            } => {
                self.pulse_edemon_tube(item_id, x, y);
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::EdemonGateSpawn {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::ChestSpawn { .. } => outcome,
            ItemDriverOutcome::ChestSpawnCheck {
                item_id,
                spawned_character_id,
                schedule_after_ticks,
                ..
            } => {
                if self.chestspawn_spawn_alive(spawned_character_id) {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                } else {
                    self.reset_chestspawn_item(item_id);
                }
                outcome
            }
            ItemDriverOutcome::FdemonGateSpawn {
                item_id,
                schedule_after_ticks,
                ..
            } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::FreakDoorUse {
                item_id,
                character_id,
                link_group,
                one_way,
                recursion_guard,
                cached_partner_id,
                no_target,
            } => {
                if recursion_guard {
                    return ItemDriverOutcome::Noop;
                }
                if self.use_freak_door(
                    item_id,
                    character_id,
                    link_group,
                    one_way,
                    cached_partner_id,
                    no_target,
                ) {
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
            ItemDriverOutcome::TrapdoorOpen {
                item_id,
                character_id,
                target_x,
                target_y,
                schedule_after_ticks,
            } => {
                if self.open_trapdoor(
                    item_id,
                    character_id,
                    target_x,
                    target_y,
                    schedule_after_ticks,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TrapdoorBlocked {
                item_id,
                cursor_item_id,
                ..
            } => {
                if self.block_trapdoor(item_id, cursor_item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TrapdoorClose { item_id } => {
                if self.close_trapdoor(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::TrapdoorBusy { .. }
            | ItemDriverOutcome::TrapdoorNeedsStick { .. } => outcome,
            ItemDriverOutcome::GasTrapPulse {
                item_id,
                character_id,
                power,
                schedule_initial_trigger,
                schedule_animation,
            } => {
                if schedule_initial_trigger {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), 1);
                }
                if schedule_animation {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), 3);
                }
                if let Some(animation) = self
                    .items
                    .get(&item_id)
                    .and_then(|item| item.driver_data.get(1).copied())
                {
                    self.apply_gastrap_foreground(item_id, animation);
                }
                if character_id.0 != 0
                    && self
                        .characters
                        .get(&character_id)
                        .is_some_and(|character| character.flags.contains(CharacterFlags::PLAYER))
                {
                    self.apply_legacy_hurt(
                        character_id,
                        None,
                        i32::from(power) * POWERSCALE,
                        1,
                        50,
                        33,
                    );
                }
                outcome
            }
            ItemDriverOutcome::BoneBridgePlace {
                item_id,
                character_id,
                cursor_item_id,
            } => {
                if self.place_bone_bridge(item_id, character_id, cursor_item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BoneBridgeTimerTick { item_id } => {
                if self.tick_bone_bridge(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BoneWallTick { item_id, .. } => {
                if self.tick_bone_wall(item_id) {
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
            ItemDriverOutcome::FireballMachineProjectile {
                item_id,
                start_x,
                start_y,
                target_x,
                target_y,
                power,
                schedule_after_ticks,
                ..
            } => {
                self.create_fireball_machine_effect(start_x, start_y, target_x, target_y, power);
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::EdemonBallProjectile {
                item_id,
                start_x: _,
                start_y: _,
                target_x: _,
                target_y: _,
                strength,
                base_sprite,
                schedule_after_ticks: _,
                ..
            } => {
                let applied = if let Some(targeted) =
                    self.find_edemonball_target_shot(item_id, strength, base_sprite)
                {
                    if let Some(item) = self.items.get_mut(&item_id) {
                        if item.driver_data.len() > 3 {
                            item.driver_data[3] = item.driver_data[3].saturating_sub(1);
                        }
                    }
                    targeted
                } else {
                    outcome
                };
                let ItemDriverOutcome::EdemonBallProjectile {
                    start_x,
                    start_y,
                    target_x,
                    target_y,
                    strength,
                    base_sprite,
                    schedule_after_ticks,
                    ..
                } = applied
                else {
                    return ItemDriverOutcome::Noop;
                };
                self.create_edemonball_effect(
                    start_x,
                    start_y,
                    target_x,
                    target_y,
                    strength,
                    base_sprite,
                );
                self.schedule_item_driver_timer(
                    item_id,
                    CharacterId(0),
                    u64::from(schedule_after_ticks),
                );
                applied
            }
            ItemDriverOutcome::CaligarGunProjectile {
                item_id,
                direction,
                schedule_after_ticks,
                ..
            } => {
                if self.create_caligar_gun_effects(item_id, direction) {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::SpikeTrapTriggered {
                item_id,
                character_id,
                damage,
                reset_after_ticks,
            } => {
                self.apply_legacy_hurt(character_id, None, damage, 1, 75, 75);
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
                self.schedule_item_driver_timer(
                    item_id,
                    CharacterId(0),
                    u64::from(schedule_after_ticks),
                );
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
            ItemDriverOutcome::BurndownIgnite { item_id, .. } => {
                if self.ignite_burndown_barrel(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BurndownTimerTick { item_id } => {
                if self.tick_burndown_barrel(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::BurndownTouch { .. }
            | ItemDriverOutcome::BurndownTooHot { .. }
            | ItemDriverOutcome::BurndownAlreadyBurned { .. } => outcome,
            ItemDriverOutcome::FdemonLoaderChanged {
                item_id,
                character_id,
                consumed_cursor_item_id,
                ground_overlay_sprite,
                sound_type,
                schedule_after_ticks,
            } => {
                if let Some(cursor_item_id) = consumed_cursor_item_id {
                    self.destroy_item(cursor_item_id);
                }
                let item_pos = self
                    .items
                    .get(&item_id)
                    .map(|item| (usize::from(item.x), usize::from(item.y)));
                if let Some((x, y)) = item_pos {
                    if let Some(tile) = self.map.tile_mut(x, y) {
                        let new_ground_sprite =
                            (tile.ground_sprite & 0xffff) | (ground_overlay_sprite << 16);
                        if tile.ground_sprite != new_ground_sprite {
                            tile.ground_sprite = new_ground_sprite;
                            self.mark_dirty_sector(x, y);
                        }
                    }
                    if let Some(sound_type) = sound_type {
                        self.queue_sound_area(x, y, sound_type);
                    }
                }
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, character_id, after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::FdemonLoaderBlocked { .. } => outcome,
            ItemDriverOutcome::EdemonLoaderChanged {
                item_id,
                character_id,
                consumed_cursor_item_id,
                ground_overlay_sprite,
                sound_type,
                schedule_after_ticks,
            } => {
                if let Some(cursor_item_id) = consumed_cursor_item_id {
                    self.destroy_item(cursor_item_id);
                }
                let item_pos = self
                    .items
                    .get(&item_id)
                    .map(|item| (usize::from(item.x), usize::from(item.y)));
                if let Some((x, y)) = item_pos {
                    if let Some(tile) = self.map.tile_mut(x, y) {
                        let new_ground_sprite =
                            (tile.ground_sprite & 0xffff) | (ground_overlay_sprite << 16);
                        if tile.ground_sprite != new_ground_sprite {
                            tile.ground_sprite = new_ground_sprite;
                            self.mark_dirty_sector(x, y);
                        }
                    }
                    if let Some(sound_type) = sound_type {
                        self.queue_sound_area(x, y, sound_type);
                    }
                }
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, character_id, after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::EdemonLoaderBlocked { .. } => outcome,
            ItemDriverOutcome::FdemonFarmChanged {
                item_id,
                foreground_sprite,
                schedule_after_ticks,
                ..
            } => {
                self.apply_fdemon_farm_foreground(item_id, foreground_sprite);
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::FdemonFarmHarvest {
                item_id,
                foreground_sprite,
                ..
            } => {
                self.apply_fdemon_farm_foreground(item_id, foreground_sprite);
                outcome
            }
            ItemDriverOutcome::FdemonFarmCursorOccupied { .. }
            | ItemDriverOutcome::FdemonFarmNotReady { .. }
            | ItemDriverOutcome::FdemonFarmBug { .. } => outcome,
            ItemDriverOutcome::FdemonBloodDestroyedFlask { flask_item_id, .. } => {
                self.destroy_item(flask_item_id);
                outcome
            }
            ItemDriverOutcome::FdemonBloodFilled {
                item_id,
                container_item_id,
                amount,
                ..
            } => {
                if let Some(container) = self.items.get_mut(&container_item_id) {
                    container.driver_data.resize(1, 0);
                    container.driver_data[0] = amount;
                    container.sprite += 1;
                    container.description =
                        format!("A container holding {} parts golem blood.", amount);
                }
                self.destroy_item(item_id);
                outcome
            }
            ItemDriverOutcome::FdemonBloodBlocked { .. } => outcome,
            ItemDriverOutcome::FdemonLavaActivated {
                item_id,
                container_item_id,
                amount,
                schedule_after_ticks,
                ..
            } => {
                if let Some(container) = self.items.get_mut(&container_item_id) {
                    container.driver_data.resize(1, 0);
                    container.driver_data[0] = amount;
                    container.sprite -= 1;
                    container.description =
                        format!("A container holding {} parts golem blood.", amount);
                }
                self.apply_fdemon_lava_tile(item_id, 120);
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::FdemonLavaPulse {
                item_id,
                stage,
                damage,
                armor_percent,
                schedule_after_ticks,
                ..
            } => {
                if let Some(target_id) = self.apply_fdemon_lava_tile(item_id, stage) {
                    if damage > 0 {
                        self.apply_legacy_hurt(target_id, None, damage, 1, 0, armor_percent);
                    }
                }
                if let Some(after_ticks) = schedule_after_ticks {
                    self.schedule_item_driver_timer(item_id, CharacterId(0), after_ticks);
                }
                outcome
            }
            ItemDriverOutcome::FdemonLavaBlocked { .. } => outcome,
            ItemDriverOutcome::FdemonWaypoint {
                item_id,
                spotted_enemy,
                target_character_id,
                target_serial,
                schedule_after_ticks,
                ..
            } => {
                self.apply_fdemon_waypoint(
                    item_id,
                    spotted_enemy,
                    target_character_id,
                    target_serial,
                );
                self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
                outcome
            }
            ItemDriverOutcome::EdemonSwitchStuck { .. } => outcome,
            ItemDriverOutcome::OnOffLightChanged {
                item_id,
                character_id,
                now_on,
                ..
            } => {
                let mut remaining_off = None;
                let mut gates_opened = false;
                if now_on {
                    self.area3_palace_lamps.switched_on_count += 1;
                    let remaining = self.area3_palace_lamps.switched_off_count
                        - self.area3_palace_lamps.switched_on_count;
                    remaining_off = Some(remaining);
                    if remaining == 0 {
                        gates_opened = true;
                        self.area3_palace_lamps.keep_open_until_tick =
                            self.tick.0 + (TICKS_PER_SECOND as u64 * 60 * 3);
                        self.schedule_registered_area3_lamp_extinguish();
                    }
                } else {
                    self.area3_palace_lamps.switched_off_count += 1;
                }
                ItemDriverOutcome::OnOffLightChanged {
                    item_id,
                    character_id,
                    now_on,
                    remaining_off,
                    gates_opened,
                }
            }
            ItemDriverOutcome::PalaceGateTick { item_id, .. } => {
                self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
                let Some((opened, closed, blocked)) = self.tick_area3_palace_gate(item_id) else {
                    return ItemDriverOutcome::Noop;
                };
                ItemDriverOutcome::PalaceGateTick {
                    item_id,
                    opened,
                    closed,
                    blocked,
                }
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
            ItemDriverOutcome::ClanJewelRescheduled {
                item_id,
                schedule_after_ticks,
            } => {
                self.schedule_item_driver_timer(
                    item_id,
                    CharacterId(0),
                    u64::from(schedule_after_ticks),
                );
                outcome
            }
            ItemDriverOutcome::ClanJewelExpired { item_id, .. } => {
                if self.destroy_item(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::ArkhataStopwatch {
                item_id,
                character_id: _,
                schedule_after_ticks,
            } => {
                self.schedule_item_driver_timer(
                    item_id,
                    CharacterId(0),
                    u64::from(schedule_after_ticks),
                );
                outcome
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
            ItemDriverOutcome::StafferMineDig {
                item_id,
                character_id: _,
            } => {
                if self.apply_staffer_mine_dig(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StafferMineTimer { item_id } => {
                if self.apply_staffer_mine_timer(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StafferBlockMove {
                item_id,
                character_id,
            } => {
                if self.apply_staffer_block_move(item_id, character_id) {
                    outcome
                } else {
                    ItemDriverOutcome::StafferBlockBlocked {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::StafferBlockTimer { item_id } => {
                if self.apply_staffer_block_timer(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::CaligarWeightMove {
                item_id,
                character_id,
            } => {
                if self.apply_caligar_weight_move(item_id, character_id) {
                    outcome
                } else {
                    ItemDriverOutcome::CaligarWeightBlocked {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::CaligarWeightTimer { item_id } => {
                if self.apply_caligar_weight_timer(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::CaligarWeightDoor {
                item_id,
                character_id,
            } => match self.apply_caligar_weight_door(item_id, character_id) {
                CaligarWeightDoorResult::Moved => outcome,
                CaligarWeightDoorResult::Locked => ItemDriverOutcome::CaligarWeightDoorLocked {
                    item_id,
                    character_id,
                },
                CaligarWeightDoorResult::Busy => ItemDriverOutcome::CaligarWeightDoorBusy {
                    item_id,
                    character_id,
                },
                CaligarWeightDoorResult::Noop => ItemDriverOutcome::Noop,
            },
            ItemDriverOutcome::CaligarSkellyDoor { .. } => outcome,
            ItemDriverOutcome::SkelRaiseTimer { item_id } => {
                if self.apply_skelraise_timer(item_id) {
                    outcome
                } else {
                    ItemDriverOutcome::Noop
                }
            }
            ItemDriverOutcome::StafferMineExhausted { .. }
            | ItemDriverOutcome::StafferBlockBlocked { .. }
            | ItemDriverOutcome::CaligarWeightBlocked { .. }
            | ItemDriverOutcome::CaligarWeightDoorLocked { .. }
            | ItemDriverOutcome::CaligarWeightDoorBusy { .. }
            | ItemDriverOutcome::CaligarSkellyDoorLocked { .. }
            | ItemDriverOutcome::CaligarSkellyDoorBusy { .. } => outcome,
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
                    true,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::BlockedByRequirements {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::AlchemyFlaskPotion {
                item_id,
                character_id,
                duration_minutes,
                modifier_index,
                modifier_value,
            } => {
                if self.install_beyond_potion_spell(
                    character_id,
                    item_id,
                    duration_minutes,
                    modifier_index,
                    modifier_value,
                    false,
                    false,
                ) {
                    if let Some(item) = self.items.get_mut(&item_id) {
                        reset_flask_empty_state(item);
                    }
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
            ItemDriverOutcome::OxygenPotion {
                item_id,
                character_id,
                ..
            } => {
                let installed = self.install_oxygen_spell(character_id);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::OxygenPotion {
                    item_id,
                    character_id,
                    installed,
                }
            }
            ItemDriverOutcome::Lab3YellowBerry {
                item_id,
                character_id,
                duration_ticks,
                ..
            } => {
                self.remove_driver_spells(character_id, IDR_OXYGEN);
                let installed = self.install_oxygen_spell_for_ticks(character_id, duration_ticks);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::Lab3YellowBerry {
                    item_id,
                    character_id,
                    duration_ticks,
                    installed,
                }
            }
            ItemDriverOutcome::Lab3WhiteBerry {
                item_id,
                character_id,
                light_power,
                ..
            } => {
                let (installed, started_emit) =
                    self.apply_lab3_whiteberry(character_id, light_power);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::Lab3WhiteBerry {
                    item_id,
                    character_id,
                    light_power,
                    started_emit,
                    installed,
                }
            }
            ItemDriverOutcome::Lab3WhiteBerryLightTick { item_id, .. } => {
                let destroyed = self.decay_lab3_whiteberry_light(item_id);
                ItemDriverOutcome::Lab3WhiteBerryLightTick { item_id, destroyed }
            }
            ItemDriverOutcome::Lab3BrownBerry {
                item_id,
                character_id,
                duration_ticks,
                ..
            } => {
                let installed = self.install_underwater_talk_spell(character_id, duration_ticks);
                if installed {
                    self.destroy_item(item_id);
                }
                ItemDriverOutcome::Lab3BrownBerry {
                    item_id,
                    character_id,
                    duration_ticks,
                    installed,
                }
            }
            ItemDriverOutcome::SpecialPotionSecurity { .. }
            | ItemDriverOutcome::SpecialPotionProfessionReset { .. }
            | ItemDriverOutcome::SpecialPotionBug { .. } => outcome,
            ItemDriverOutcome::SpecialShrine { .. } => outcome,
            ItemDriverOutcome::TorchExtractOrb { .. } => outcome,
            ItemDriverOutcome::NomadStack { .. }
            | ItemDriverOutcome::TransportOpen { .. }
            | ItemDriverOutcome::TransportTravel { .. }
            | ItemDriverOutcome::TransportInvalid { .. }
            | ItemDriverOutcome::ArenaToplist { .. } => outcome,
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
            ItemDriverOutcome::ArkhataKeyAssemble {
                item_id,
                character_id,
                cursor_item_id,
                result_template_id,
                result_sprite,
                final_key,
            } => {
                if self.apply_arkhata_key_assemble(
                    item_id,
                    character_id,
                    cursor_item_id,
                    result_template_id,
                    result_sprite,
                    final_key,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::ArkhataKeyDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            ItemDriverOutcome::CaligarKeyAssemble {
                item_id,
                character_id,
                cursor_item_id,
                result_sprite,
                final_key,
            } => {
                if self.apply_caligar_key_assemble(
                    item_id,
                    character_id,
                    cursor_item_id,
                    result_sprite,
                    final_key,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::CaligarKeyDoesNotFit {
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
            ItemDriverOutcome::LizardFlowerMixed {
                item_id,
                character_id,
                cursor_item_id,
                combined_bits,
                ..
            } => {
                if self.apply_lizard_flower_mixed(
                    item_id,
                    character_id,
                    cursor_item_id,
                    combined_bits,
                ) {
                    outcome
                } else {
                    ItemDriverOutcome::LizardFlowerDoesNotFit {
                        item_id,
                        character_id,
                    }
                }
            }
            _ => outcome,
        }
    }

    fn apply_fdemon_farm_foreground(&mut self, item_id: ItemId, foreground_sprite: u32) {
        let item_pos = self
            .items
            .get(&item_id)
            .map(|item| (usize::from(item.x), usize::from(item.y)));
        if let Some((x, y)) = item_pos {
            if let Some(tile) = self.map.tile_mut(x, y) {
                let new_foreground_sprite =
                    (tile.foreground_sprite & 0xffff) | (foreground_sprite << 16);
                if tile.foreground_sprite != new_foreground_sprite {
                    tile.foreground_sprite = new_foreground_sprite;
                    self.mark_dirty_sector(x, y);
                }
            }
        }
    }

    fn apply_fdemon_lava_tile(&mut self, item_id: ItemId, stage: u8) -> Option<CharacterId> {
        let item_pos = self
            .items
            .get(&item_id)
            .map(|item| (usize::from(item.x), usize::from(item.y)))?;
        let (x, y) = item_pos;
        let mut target_id = None;
        let mut changed = false;
        if let Some(tile) = self.map.tile_mut(x, y) {
            if tile.character != 0 {
                target_id = Some(CharacterId(u32::from(tile.character)));
            }
            if stage == 0 {
                let flags = tile.flags | MapFlags::MOVEBLOCK | MapFlags::FIRETHRU;
                if tile.flags != flags {
                    tile.flags = flags;
                    changed = true;
                }
                let foreground = tile.foreground_sprite & 0xffff;
                if tile.foreground_sprite != foreground {
                    tile.foreground_sprite = foreground;
                    changed = true;
                }
            } else if stage < 20 {
                let foreground = (tile.foreground_sprite & 0xffff) | (1024 << 16);
                if tile.foreground_sprite != foreground {
                    tile.foreground_sprite = foreground;
                    changed = true;
                }
            } else if stage < 115 {
                let foreground = tile.foreground_sprite & 0xffff;
                if tile.foreground_sprite != foreground {
                    tile.foreground_sprite = foreground;
                    changed = true;
                }
            } else {
                if tile.flags.contains(MapFlags::MOVEBLOCK) {
                    tile.flags.remove(MapFlags::MOVEBLOCK);
                    changed = true;
                }
                let foreground = (tile.foreground_sprite & 0xffff) | (1034 << 16);
                if tile.foreground_sprite != foreground {
                    tile.foreground_sprite = foreground;
                    changed = true;
                }
            }
        }
        if changed {
            self.mark_dirty_sector(x, y);
        }
        target_id
    }

    fn apply_fdemon_waypoint(
        &mut self,
        item_id: ItemId,
        spotted_enemy: bool,
        target_character_id: Option<CharacterId>,
        target_serial: Option<u32>,
    ) {
        let Some((x, y)) = self.items.get_mut(&item_id).map(|item| {
            item.driver_data.resize(12, 0);
            item.driver_data[0] = u8::from(spotted_enemy);
            item.sprite = if spotted_enemy { 14200 } else { 14202 };
            let target_id = target_character_id.map_or(0, |id| id.0);
            item.driver_data[4..8].copy_from_slice(&target_id.to_le_bytes());
            item.driver_data[8..12].copy_from_slice(&target_serial.unwrap_or(0).to_le_bytes());
            (usize::from(item.x), usize::from(item.y))
        }) else {
            return;
        };
        self.mark_dirty_sector(x, y);
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

    fn apply_arkhata_key_assemble(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_template_id: u32,
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
        item.template_id = result_template_id;
        if final_key {
            item.name = "Knoger Key 1".to_string();
            item.description =
                "A finished key. Should open something now. A door, perhaps.".to_string();
        }
        self.destroy_item(cursor_item_id)
    }

    fn apply_caligar_key_assemble(
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

        if final_key {
            return true;
        }

        item.sprite = result_sprite;
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

    fn apply_lizard_flower_mixed(
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
        if combined_bits == 7 {
            item.sprite = 11188;
            item.driver = crate::item_driver::IDR_OXYPOTION;
            item.name = "Scuba Potion".to_string();
            item.description = "A bubbly fluid in a nice bottle.".to_string();
        } else {
            item.sprite = 11189;
            item.description = "A partially finished scuba potion.".to_string();
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

    fn open_trapdoor(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        target_x: u16,
        target_y: u16,
        schedule_after_ticks: u64,
    ) -> bool {
        let Some((x, y)) = self
            .items
            .get(&item_id)
            .map(|item| (usize::from(item.x), usize::from(item.y)))
        else {
            return false;
        };
        if !self.teleport_character_exact(
            character_id,
            usize::from(target_x),
            usize::from(target_y),
        ) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(1, 0);
        item.driver_data[0] = 1;
        item.sprite += 1;
        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.flags.insert(MapFlags::TMOVEBLOCK);
        }
        self.mark_dirty_sector(x, y);
        self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
        self.pending_system_texts.push(WorldSystemText {
            character_id,
            message: "A trapdoor opens under your feet, but you manage to jump back in time."
                .to_string(),
        });
        true
    }

    fn block_trapdoor(&mut self, item_id: ItemId, cursor_item_id: ItemId) -> bool {
        let Some((x, y)) = self.items.get(&item_id).map(|item| (item.x, item.y)) else {
            return false;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(1, 0);
        item.driver_data[0] = 2;
        item.sprite += 2;
        self.mark_dirty_sector(usize::from(x), usize::from(y));
        self.destroy_item(cursor_item_id)
    }

    fn close_trapdoor(&mut self, item_id: ItemId) -> bool {
        let Some((x, y)) = self.items.get(&item_id).map(|item| (item.x, item.y)) else {
            return false;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.driver_data.first().copied().unwrap_or_default() != 1 {
            return false;
        }
        item.driver_data[0] = 0;
        item.sprite -= 1;
        if let Some(tile) = self.map.tile_mut(usize::from(x), usize::from(y)) {
            tile.flags.remove(MapFlags::TMOVEBLOCK);
        }
        self.mark_dirty_sector(usize::from(x), usize::from(y));
        true
    }

    fn apply_gastrap_foreground(&mut self, item_id: ItemId, animation: u8) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let origin_x = usize::from(item.x);
        let origin_y = usize::from(item.y);
        let Some((x, y, base_sprite)) = [(0_i16, 0_i16), (1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .filter_map(|(dx, dy)| {
                let x = offset_coordinate(origin_x, dx)?;
                let y = offset_coordinate(origin_y, dy)?;
                let sprite = self.map.tile(x, y)?.foreground_sprite;
                let base = match sprite {
                    15291..=15299 => 15291,
                    15300..=15308 => 15300,
                    15309..=15317 => 15309,
                    15318..=15326 => 15318,
                    _ => return None,
                };
                Some((x, y, base))
            })
            .next()
        else {
            return false;
        };
        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.foreground_sprite = base_sprite + u32::from(animation);
            self.mark_dirty_sector(x, y);
            true
        } else {
            false
        }
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
        self.effects.insert(effect_id, effect);
        self.apply_legacy_hurt(character_id, None, 20 * POWERSCALE, 1, 50, 75);
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

    pub fn destroy_item(&mut self, item_id: ItemId) -> bool {
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

    pub fn apply_skelraise_raise(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        raised_id: CharacterId,
        raised_serial: u32,
    ) -> bool {
        if !self.characters.contains_key(&character_id) || !self.characters.contains_key(&raised_id)
        {
            return false;
        }
        let (x, y) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.driver_data.resize(12, 0);
            item.driver_data[2] = 1;
            item.driver_data[4..8].copy_from_slice(&raised_id.0.to_le_bytes());
            item.driver_data[8..12].copy_from_slice(&raised_serial.to_le_bytes());
            item.sprite += 1;
            (usize::from(item.x), usize::from(item.y))
        };
        self.destroy_item(cursor_item_id);
        self.mark_dirty_sector(x, y);
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
        true
    }

    fn apply_skelraise_timer(&mut self, item_id: ItemId) -> bool {
        let (raised_id, raised_serial, active, x, y) = {
            let Some(item) = self.items.get(&item_id) else {
                return false;
            };
            let active = item.driver_data.get(2).copied().unwrap_or_default() != 0;
            let raised_id = if item.driver_data.len() >= 8 {
                CharacterId(u32::from_le_bytes([
                    item.driver_data[4],
                    item.driver_data[5],
                    item.driver_data[6],
                    item.driver_data[7],
                ]))
            } else {
                CharacterId(0)
            };
            let raised_serial = if item.driver_data.len() >= 12 {
                u32::from_le_bytes([
                    item.driver_data[8],
                    item.driver_data[9],
                    item.driver_data[10],
                    item.driver_data[11],
                ])
            } else {
                0
            };
            (
                raised_id,
                raised_serial,
                active,
                usize::from(item.x),
                usize::from(item.y),
            )
        };
        if !active {
            return true;
        }
        let still_alive = raised_id.0 != 0
            && self.characters.get(&raised_id).is_some_and(|character| {
                (raised_serial == 0 || character.id.0 == raised_id.0) && !character.flags.is_empty()
            });
        if still_alive {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
            return true;
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(12, 0);
            item.driver_data[2] = 0;
            item.sprite -= 1;
        }
        self.mark_dirty_sector(x, y);
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

    fn place_bone_bridge(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        if character.cursor_item != Some(cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let dx = i32::from(item.x)
            .saturating_sub(i32::from(character.x))
            .signum();
        let dy = i32::from(item.y)
            .saturating_sub(i32::from(character.y))
            .signum();
        let target_x = i32::from(item.x) + dx;
        let target_y = i32::from(item.y) + dy;
        if target_x < 2
            || target_y < 2
            || target_x >= MAX_MAP as i32 - 2
            || target_y >= MAX_MAP as i32 - 2
        {
            return false;
        }
        let target_x = target_x as usize;
        let target_y = target_y as usize;
        let Some(tile) = self.map.tile(target_x, target_y) else {
            return false;
        };
        if tile.item != 0 || !tile.flags.contains(MapFlags::MOVEBLOCK) {
            return false;
        }
        let Some(cursor) = self.items.get(&cursor_item_id) else {
            return false;
        };
        if cursor.carried_by != Some(character_id) {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(target_x, target_y) {
            tile.item = cursor_item_id.0;
            tile.flags.remove(MapFlags::MOVEBLOCK);
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.cursor_item = None;
            character.flags.insert(CharacterFlags::ITEMS);
        }
        if let Some(cursor) = self.items.get_mut(&cursor_item_id) {
            cursor.carried_by = None;
            cursor.contained_in = None;
            cursor.x = target_x as u16;
            cursor.y = target_y as u16;
            cursor.flags.remove(ItemFlags::TAKE);
            cursor.driver_data.resize(2, 0);
            cursor.driver_data[1] = 1;
            cursor.sprite = if dx == 0 { 13045 } else { 13035 };
        }
        self.mark_dirty_sector(target_x, target_y);
        self.schedule_item_driver_timer(cursor_item_id, CharacterId(0), TICKS_PER_SECOND * 60);
        true
    }

    fn tick_bone_bridge(&mut self, item_id: ItemId) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.driver_data.get(1).copied().unwrap_or_default() == 0 || item.carried_by.is_some() {
            return false;
        }
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let Some(tile) = self.map.tile(x, y) else {
            return false;
        };
        if tile.item != item_id.0 {
            return false;
        }
        if tile.flags.contains(MapFlags::TMOVEBLOCK) {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND);
            return true;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.flags.insert(MapFlags::MOVEBLOCK);
        }
        let remove = if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(2, 0);
            item.driver_data[1] = item.driver_data[1].saturating_add(1);
            item.sprite += 1;
            item.driver_data[1] > 9
        } else {
            return false;
        };
        self.mark_dirty_sector(x, y);
        if remove {
            self.destroy_item(item_id)
        } else {
            self.schedule_item_driver_timer(item_id, CharacterId(0), 3)
        }
    }

    fn tick_bone_wall(&mut self, item_id: ItemId) -> bool {
        let (x, y, state) = {
            let Some(item) = self.items.get(&item_id) else {
                return false;
            };
            (
                usize::from(item.x),
                usize::from(item.y),
                item.driver_data.first().copied().unwrap_or_default(),
            )
        };
        if !self.map.legacy_inner_bounds(x, y) {
            return false;
        }

        if state == 0 {
            for (nx, ny) in [
                (x.saturating_add(1), y),
                (x.saturating_sub(1), y),
                (x, y.saturating_add(1)),
                (x, y.saturating_sub(1)),
            ] {
                let Some(tile) = self.map.tile(nx, ny) else {
                    continue;
                };
                let neighbor_id = ItemId(tile.item);
                if neighbor_id.0 == 0 {
                    continue;
                }
                let Some(neighbor) = self.items.get(&neighbor_id) else {
                    continue;
                };
                if neighbor.driver == IDR_BONEWALL
                    && neighbor.driver_data.first().copied().unwrap_or_default() == 0
                {
                    self.schedule_item_driver_timer_with_context(
                        neighbor_id,
                        CharacterId(0),
                        4,
                        false,
                    );
                }
            }
        }

        if state < 5 {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.driver_data.resize(1, 0);
            item.driver_data[0] = state.saturating_add(1);
            item.sprite = item.sprite.saturating_add(1);
            self.mark_dirty_sector(x, y);
            self.schedule_item_driver_timer(item_id, CharacterId(0), 2);
            return true;
        }

        if state == 5 {
            if let Some(tile) = self.map.tile_mut(x, y) {
                if tile.item == item_id.0 {
                    tile.item = 0;
                }
                tile.flags
                    .remove(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
            }
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.flags.remove(ItemFlags::USE);
            item.flags.insert(ItemFlags::VOID);
            item.driver_data.resize(1, 0);
            item.driver_data[0] = 6;
            self.mark_dirty_sector(x, y);
            self.schedule_item_driver_timer(
                item_id,
                CharacterId(0),
                TICKS_PER_SECOND.saturating_mul(60),
            );
            return true;
        }

        if state == 6 {
            let blocked = self
                .map
                .tile(x, y)
                .is_some_and(|tile| tile.item != 0 || tile.flags.contains(MapFlags::TMOVEBLOCK));
            if blocked {
                self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND);
                return true;
            }

            if let Some(tile) = self.map.tile_mut(x, y) {
                tile.item = item_id.0;
                tile.flags
                    .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
            }
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.sprite = item.sprite.saturating_sub(5);
            item.driver_data.resize(1, 0);
            item.driver_data[0] = 0;
            item.flags.insert(ItemFlags::USE);
            item.flags.remove(ItemFlags::VOID);
            self.mark_dirty_sector(x, y);
            return true;
        }

        false
    }

    fn apply_staffer_mine_dig(&mut self, item_id: ItemId) -> bool {
        let (x, y, stage) = {
            let Some(item) = self.items.get(&item_id) else {
                return false;
            };
            (
                usize::from(item.x),
                usize::from(item.y),
                item.driver_data.get(3).copied().unwrap_or_default(),
            )
        };

        if stage == 3 {
            let before = self.items.get(&item_id).cloned();
            if let Some(tile) = self.map.tile_mut(x, y) {
                tile.flags.remove(MapFlags::TSIGHTBLOCK);
            }
            if let Some(item) = self.items.get_mut(&item_id) {
                item.flags.remove(ItemFlags::SIGHTBLOCK);
            }
            if let Some(before) = before.as_ref() {
                self.refresh_item_light_after_mutation(before, item_id);
            }
        }

        if stage == 8 {
            if let Some(tile) = self.map.tile_mut(x, y) {
                if tile.item == item_id.0 {
                    tile.item = 0;
                }
                tile.flags.remove(MapFlags::TMOVEBLOCK);
            }
            if let Some(item) = self.items.get_mut(&item_id) {
                item.flags.remove(ItemFlags::USE);
                item.flags.insert(ItemFlags::VOID);
            }
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 60 * 5);
        }

        self.mark_dirty_sector(x, y);
        true
    }

    fn apply_staffer_mine_timer(&mut self, item_id: ItemId) -> bool {
        let (x, y, stage, initialized) = {
            let Some(item) = self.items.get(&item_id) else {
                return false;
            };
            (
                usize::from(item.x),
                usize::from(item.y),
                item.driver_data.get(3).copied().unwrap_or_default(),
                item.driver_data.get(4).copied().unwrap_or_default() != 0,
            )
        };

        if !initialized {
            if let Some(item) = self.items.get_mut(&item_id) {
                item.driver_data.resize(5, 0);
                item.driver_data[4] = 1;
                item.sprite = match (u32::from(item.x) + u32::from(item.y)) % 3 {
                    0 => 15070,
                    1 => 15078,
                    _ => 15086,
                };
            }
        }

        if stage != 8 {
            return true;
        }

        let blocked = self
            .map
            .tile(x, y)
            .is_none_or(|tile| tile.flags.contains(MapFlags::TMOVEBLOCK) || tile.item != 0);
        if blocked {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND);
            return true;
        }

        let before = self.items.get(&item_id).cloned();
        if let Some(item) = self.items.get_mut(&item_id) {
            item.sprite -= 8;
            item.driver_data.resize(4, 0);
            item.driver_data[3] = 0;
            item.flags.insert(ItemFlags::USE | ItemFlags::SIGHTBLOCK);
            item.flags.remove(ItemFlags::VOID);
        }
        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.item = item_id.0;
            tile.flags
                .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
        }
        if let Some(before) = before.as_ref() {
            self.refresh_item_light_after_mutation(before, item_id);
        }
        self.mark_dirty_sector(x, y);
        true
    }

    fn apply_staffer_block_move(&mut self, item_id: ItemId, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Ok(direction) = Direction::try_from(character.dir) else {
            return false;
        };
        let (dx, dy) = direction.delta();
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let target_x_i = i32::from(item.x) + i32::from(dx);
        let target_y_i = i32::from(item.y) + i32::from(dy);
        if target_x_i < 0 || target_y_i < 0 {
            return false;
        }
        let target_x = target_x_i as usize;
        let target_y = target_y_i as usize;
        let Some(target) = self.map.tile(target_x, target_y) else {
            return false;
        };
        let gsprite = target.ground_sprite;
        let wrong_sprite =
            (gsprite < 20291 || gsprite > 20299) && gsprite != 13154 && gsprite > 13156;
        if target
            .flags
            .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            || target.item != 0
            || wrong_sprite
        {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.flags.remove(MapFlags::TMOVEBLOCK);
            if tile.item == item_id.0 {
                tile.item = 0;
            }
        }
        if let Some(tile) = self.map.tile_mut(target_x, target_y) {
            tile.flags.insert(MapFlags::TMOVEBLOCK);
            tile.item = item_id.0;
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(12, 0);
            if u16::from_le_bytes([item.driver_data[8], item.driver_data[9]]) == 0 {
                item.driver_data[8..10].copy_from_slice(&item.x.to_le_bytes());
                item.driver_data[10..12].copy_from_slice(&item.y.to_le_bytes());
            }
            item.x = target_x as u16;
            item.y = target_y as u16;
            item.driver_data[4..8].copy_from_slice(&(self.tick.0 as u32).to_le_bytes());
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.action = 0;
            character.step = 0;
            character.duration = 0;
        }
        self.mark_dirty_sector(x, y);
        self.mark_dirty_sector(target_x, target_y);
        true
    }

    fn apply_staffer_block_timer(&mut self, item_id: ItemId) -> bool {
        let (x, y, home_x, home_y, last_touch) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.driver_data.resize(12, 0);
            if u16::from_le_bytes([item.driver_data[8], item.driver_data[9]]) == 0 {
                item.driver_data[8..10].copy_from_slice(&item.x.to_le_bytes());
                item.driver_data[10..12].copy_from_slice(&item.y.to_le_bytes());
            }
            (
                usize::from(item.x),
                usize::from(item.y),
                usize::from(u16::from_le_bytes([
                    item.driver_data[8],
                    item.driver_data[9],
                ])),
                usize::from(u16::from_le_bytes([
                    item.driver_data[10],
                    item.driver_data[11],
                ])),
                u32::from_le_bytes([
                    item.driver_data[4],
                    item.driver_data[5],
                    item.driver_data[6],
                    item.driver_data[7],
                ]) as u64,
            )
        };

        if self.tick.0.saturating_sub(last_touch) > TICKS_PER_SECOND * 60 * 2
            && (home_x != x || home_y != y)
        {
            let home_free = self.map.tile(home_x, home_y).is_some_and(|tile| {
                !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                    && tile.item == 0
            });
            if home_free {
                if let Some(tile) = self.map.tile_mut(x, y) {
                    tile.flags.remove(MapFlags::TMOVEBLOCK);
                    if tile.item == item_id.0 {
                        tile.item = 0;
                    }
                }
                if let Some(tile) = self.map.tile_mut(home_x, home_y) {
                    tile.flags.insert(MapFlags::TMOVEBLOCK);
                    tile.item = item_id.0;
                }
                if let Some(item) = self.items.get_mut(&item_id) {
                    item.x = home_x as u16;
                    item.y = home_y as u16;
                }
                self.mark_dirty_sector(x, y);
                self.mark_dirty_sector(home_x, home_y);
            }
        }
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        true
    }

    fn apply_caligar_weight_move(&mut self, item_id: ItemId, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Ok(direction) = Direction::try_from(character.dir) else {
            return false;
        };
        let (dx, dy) = direction.delta();
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let target_x_i = i32::from(item.x) + i32::from(dx);
        let target_y_i = i32::from(item.y) + i32::from(dy);
        if target_x_i < 0 || target_y_i < 0 {
            return false;
        }
        let target_x = target_x_i as usize;
        let target_y = target_y_i as usize;
        let Some(target) = self.map.tile(target_x, target_y) else {
            return false;
        };
        let gsprite = target.ground_sprite;
        let valid_floor = (20797..=20823).contains(&gsprite)
            || gsprite == 59683
            || (20291..=20299).contains(&gsprite);
        if !valid_floor
            || target
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            || target.item != 0
        {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.flags.remove(MapFlags::TMOVEBLOCK);
            if tile.item == item_id.0 {
                tile.item = 0;
            }
        }
        if let Some(tile) = self.map.tile_mut(target_x, target_y) {
            tile.flags.insert(MapFlags::TMOVEBLOCK);
            tile.item = item_id.0;
        }
        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(12, 0);
            if u16::from_le_bytes([item.driver_data[8], item.driver_data[9]]) == 0 {
                item.driver_data[8..10].copy_from_slice(&item.x.to_le_bytes());
                item.driver_data[10..12].copy_from_slice(&item.y.to_le_bytes());
            }
            item.x = target_x as u16;
            item.y = target_y as u16;
            item.driver_data[4..8].copy_from_slice(&(self.tick.0 as u32).to_le_bytes());
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.action = 0;
            character.step = 0;
            character.duration = 0;
        }
        self.mark_dirty_sector(x, y);
        self.mark_dirty_sector(target_x, target_y);
        true
    }

    fn apply_caligar_weight_timer(&mut self, item_id: ItemId) -> bool {
        let (x, y, home_x, home_y, last_touch) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            item.driver_data.resize(12, 0);
            if u16::from_le_bytes([item.driver_data[8], item.driver_data[9]]) == 0 {
                item.driver_data[8..10].copy_from_slice(&item.x.to_le_bytes());
                item.driver_data[10..12].copy_from_slice(&item.y.to_le_bytes());
            }
            (
                usize::from(item.x),
                usize::from(item.y),
                usize::from(u16::from_le_bytes([
                    item.driver_data[8],
                    item.driver_data[9],
                ])),
                usize::from(u16::from_le_bytes([
                    item.driver_data[10],
                    item.driver_data[11],
                ])),
                u32::from_le_bytes([
                    item.driver_data[4],
                    item.driver_data[5],
                    item.driver_data[6],
                    item.driver_data[7],
                ]) as u64,
            )
        };

        if self.tick.0.saturating_sub(last_touch) > TICKS_PER_SECOND * 60 * 5
            && (home_x != x || home_y != y)
        {
            let home_free = self.map.tile(home_x, home_y).is_some_and(|tile| {
                !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                    && tile.item == 0
            });
            if home_free {
                if let Some(tile) = self.map.tile_mut(x, y) {
                    tile.flags.remove(MapFlags::TMOVEBLOCK);
                    if tile.item == item_id.0 {
                        tile.item = 0;
                    }
                }
                if let Some(tile) = self.map.tile_mut(home_x, home_y) {
                    tile.flags.insert(MapFlags::TMOVEBLOCK);
                    tile.item = item_id.0;
                }
                if let Some(item) = self.items.get_mut(&item_id) {
                    item.x = home_x as u16;
                    item.y = home_y as u16;
                }
                self.mark_dirty_sector(x, y);
                self.mark_dirty_sector(home_x, home_y);
            }
        }
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        true
    }

    fn apply_caligar_weight_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> CaligarWeightDoorResult {
        let Some(item) = self.items.get(&item_id) else {
            return CaligarWeightDoorResult::Noop;
        };
        let Some(character) = self.characters.get(&character_id) else {
            return CaligarWeightDoorResult::Noop;
        };
        let dx = i32::from(character.x) - i32::from(item.x);
        let dy = i32::from(character.y) - i32::from(item.y);
        if dx != 0 && dy != 0 {
            return CaligarWeightDoorResult::Noop;
        }

        if dy > 0 {
            let has_lock_weight = |world: &World, x: usize, y: usize| {
                let item_id = world.map.tile(x, y).map(|tile| tile.item).unwrap_or(0);
                item_id != 0
                    && world
                        .items
                        .get(&ItemId(item_id))
                        .is_some_and(|item| item.driver == IDR_CALIGAR)
            };
            if !has_lock_weight(self, 210, 184) || !has_lock_weight(self, 213, 176) {
                if let Some(character) = self.characters.get_mut(&character_id) {
                    character.action = 0;
                    character.step = 0;
                    character.duration = 0;
                }
                return CaligarWeightDoorResult::Locked;
            }
        }

        let target_x = i32::from(item.x) - dx;
        let target_y = i32::from(item.y) - dy;
        if target_x < 1
            || target_y < 1
            || target_x as usize > self.map.width().saturating_sub(2)
            || target_y as usize > self.map.height().saturating_sub(2)
        {
            return CaligarWeightDoorResult::Noop;
        }

        if !self.teleport_character_exact(character_id, target_x as usize, target_y as usize) {
            return CaligarWeightDoorResult::Busy;
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.dir = match character.dir {
                value if value == Direction::Right as u8 => Direction::Left as u8,
                value if value == Direction::Left as u8 => Direction::Right as u8,
                value if value == Direction::Up as u8 => Direction::Down as u8,
                value if value == Direction::Down as u8 => Direction::Up as u8,
                value => value,
            };
            character.action = 0;
            character.step = 0;
            character.duration = 0;
        }
        CaligarWeightDoorResult::Moved
    }

    pub fn apply_caligar_skelly_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        door_index: u8,
    ) -> ItemDriverOutcome {
        let Some(item) = self.items.get(&item_id) else {
            return ItemDriverOutcome::Noop;
        };
        let Some(character) = self.characters.get(&character_id) else {
            return ItemDriverOutcome::Noop;
        };
        let dx = i32::from(character.x) - i32::from(item.x);
        let dy = i32::from(character.y) - i32::from(item.y);
        if dx != 0 && dy != 0 {
            return ItemDriverOutcome::Noop;
        }

        let target_x = i32::from(item.x) - dx;
        let target_y = i32::from(item.y) - dy;
        if target_x < 1
            || target_y < 1
            || target_x as usize > self.map.width().saturating_sub(2)
            || target_y as usize > self.map.height().saturating_sub(2)
        {
            return ItemDriverOutcome::Noop;
        }

        if !self.teleport_character_exact(character_id, target_x as usize, target_y as usize) {
            return ItemDriverOutcome::CaligarSkellyDoorBusy {
                item_id,
                character_id,
            };
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.dir = match character.dir {
                value if value == Direction::Right as u8 => Direction::Left as u8,
                value if value == Direction::Left as u8 => Direction::Right as u8,
                value if value == Direction::Up as u8 => Direction::Down as u8,
                value if value == Direction::Down as u8 => Direction::Up as u8,
                value => value,
            };
            character.action = 0;
            character.step = 0;
            character.duration = 0;
        }

        ItemDriverOutcome::CaligarSkellyDoor {
            item_id,
            character_id,
            door_index,
        }
    }

    fn teleport_character_exact(&mut self, character_id: CharacterId, x: usize, y: usize) -> bool {
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
        if !self.map.set_char(character, x, y) {
            let _ = self.map.set_char(character, old_x, old_y);
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

        self.queue_sound_area(x, y, if character_id.0 == 0 { 2 } else { 3 });

        DoorToggleResult::Toggled
    }

    fn toggle_pick_door(&mut self, item_id: ItemId, character_id: CharacterId) -> DoorToggleResult {
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

        if character_id.0 == 0 && !is_open {
            return DoorToggleResult::Blocked;
        }

        if is_open && self.pick_door_close_blocked(x, y) {
            if character_id.0 == 0 {
                self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND);
            }
            return DoorToggleResult::Blocked;
        }

        {
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
            }
        }

        if !is_open {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 20);
        }

        DoorToggleResult::Toggled
    }

    fn pick_door_close_blocked(&self, x: usize, y: usize) -> bool {
        if self.map.tile(x, y).is_some_and(|tile| {
            tile.flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        }) {
            return true;
        }

        [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .any(|(dx, dy)| {
                let Some(nx) = x.checked_add_signed(dx) else {
                    return false;
                };
                let Some(ny) = y.checked_add_signed(dy) else {
                    return false;
                };
                self.map
                    .tile(nx, ny)
                    .is_some_and(|tile| tile.character != 0)
            })
    }

    fn ignite_burndown_barrel(&mut self, item_id: ItemId) -> bool {
        let Some(before) = self.items.get(&item_id).cloned() else {
            return false;
        };
        let x = usize::from(before.x);
        let y = usize::from(before.y);
        if self.map.tile(x, y).is_none() {
            return false;
        }

        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(1, 0);
            item.driver_data[0] = 20;
            item.sprite = 51077;
            item.modifier_index[0] = CharacterValue::Light as i16;
            item.modifier_value[0] = 200;
        } else {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.foreground_sprite = 1024 << 16;
            self.mark_dirty_sector(x, y);
        }
        self.refresh_item_light_after_mutation(&before, item_id);
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        true
    }

    fn tick_burndown_barrel(&mut self, item_id: ItemId) -> bool {
        let Some(before) = self.items.get(&item_id).cloned() else {
            return false;
        };
        let x = usize::from(before.x);
        let y = usize::from(before.y);
        let Some(state) = before.driver_data.first().copied() else {
            return false;
        };
        if state == 0 {
            return false;
        }

        let mut schedule_next = false;
        let mut light_changed = false;
        if let Some(item) = self.items.get_mut(&item_id) {
            item.driver_data.resize(1, 0);
            item.driver_data[0] = item.driver_data[0].saturating_sub(1);
            let new_state = item.driver_data[0];
            if new_state > 15 {
                item.sprite += 1;
                schedule_next = true;
            } else if new_state == 15 {
                item.modifier_index[0] = CharacterValue::Light as i16;
                item.modifier_value[0] = 0;
                light_changed = true;
                schedule_next = true;
            } else if new_state == 0 {
                item.sprite = 21115;
            } else {
                schedule_next = true;
            }
        } else {
            return false;
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            if state == 16 {
                tile.foreground_sprite = 0;
            }
            self.mark_dirty_sector(x, y);
        }
        if light_changed {
            self.refresh_item_light_after_mutation(&before, item_id);
        }
        if schedule_next {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        }
        true
    }

    fn toggle_staffer_spec_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    ) -> StafferSpecDoorResult {
        let Some(item) = self.items.get(&item_id) else {
            return StafferSpecDoorResult::Failed;
        };
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let is_open = item.driver_data.get(1).copied().unwrap_or_default() != 0;

        if x == 0 || y == 0 {
            return StafferSpecDoorResult::Failed;
        }
        let Some(tile) = self.map.tile(x, y) else {
            return StafferSpecDoorResult::Failed;
        };
        if tile.item != item_id.0 {
            return StafferSpecDoorResult::Failed;
        }

        if character_id.0 == 0 {
            let mut should_continue = true;
            if let Some(item) = self.items.get_mut(&item_id) {
                item.driver_data.resize(40, 0);
                if item.driver_data[39] != 0 {
                    item.driver_data[39] = item.driver_data[39].saturating_sub(1);
                }
                should_continue = item.driver_data[1] != 0 && item.driver_data[39] == 0;
            }
            if !should_continue {
                return StafferSpecDoorResult::Blocked;
            }
        }

        if is_open
            && tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        {
            if character_id.0 == 0 {
                if let Some(item) = self.items.get_mut(&item_id) {
                    item.driver_data.resize(40, 0);
                    item.driver_data[39] = item.driver_data[39].saturating_add(1);
                }
                self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
            }
            return StafferSpecDoorResult::Blocked;
        }

        if character_id.0 != 0 {
            let marker = match kind {
                4 => Some((51, 234)),
                5 => Some((59, 240)),
                _ => None,
            };
            if marker
                .and_then(|(mx, my)| self.map.tile(mx, my))
                .is_some_and(|tile| tile.item == 0)
            {
                return StafferSpecDoorResult::Locked;
            }
        }

        let mut schedule_auto_close = false;
        {
            let Some(item) = self.items.get_mut(&item_id) else {
                return StafferSpecDoorResult::Failed;
            };
            item.driver_data.resize(40, 0);
            let Some(tile) = self.map.tile_mut(x, y) else {
                return StafferSpecDoorResult::Failed;
            };

            if is_open {
                let restored = door_stored_flags(item);
                item.flags.insert(restored);
                apply_door_tile_flags(tile, item.flags);
                item.driver_data[1] = 0;
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
                item.driver_data[1] = 1;
                item.sprite += 1;
                item.driver_data[39] = item.driver_data[39].saturating_add(1);
                schedule_auto_close = item.driver_data[5] == 0;
            }
        }

        if schedule_auto_close {
            self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
        }
        self.queue_sound_area(x, y, if character_id.0 == 0 { 2 } else { 3 });
        self.mark_dirty_sector(x, y);

        StafferSpecDoorResult::Toggled
    }

    fn tick_area3_palace_gate(&mut self, item_id: ItemId) -> Option<(bool, bool, bool)> {
        let item = self.items.get(&item_id)?;
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let is_open = item.driver_data.first().copied().unwrap_or_default() != 0;
        let keep_open = self.area3_palace_lamps.keep_open_until_tick > self.tick.0;
        let tile = self.map.tile(x, y)?;
        if tile.item != item_id.0 {
            return None;
        }

        if keep_open {
            if is_open {
                return Some((false, false, false));
            }
            let item = self.items.get_mut(&item_id)?;
            item.driver_data.resize(40, 0);
            let tile = self.map.tile_mut(x, y)?;
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
            self.mark_dirty_sector(x, y);
            return Some((true, false, false));
        }

        if !is_open {
            return Some((false, false, false));
        }
        if tile
            .flags
            .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        {
            return Some((false, false, true));
        }

        let item = self.items.get_mut(&item_id)?;
        item.driver_data.resize(40, 0);
        let tile = self.map.tile_mut(x, y)?;
        let restored = door_stored_flags(item);
        item.flags.insert(restored);
        apply_door_tile_flags(tile, item.flags);
        item.driver_data[0] = 0;
        item.sprite -= 1;
        self.mark_dirty_sector(x, y);
        Some((false, true, false))
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

    fn use_freak_door(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        link_group: u8,
        one_way: bool,
        cached_partner_id: Option<ItemId>,
        no_target: bool,
    ) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let item_x = item.x;
        let item_y = item.y;
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let character_x = character.x;
        let character_y = character.y;

        let effective_group = if one_way { 0 } else { link_group };
        let partner_id = if effective_group == 0 {
            item_id
        } else if let Some(partner_id) = cached_partner_id.filter(|id| self.items.contains_key(id))
        {
            partner_id
        } else {
            let Some(found_id) = self.items.iter().find_map(|(candidate_id, candidate)| {
                (candidate_id != &item_id
                    && candidate.driver == crate::item_driver::IDR_FREAKDOOR
                    && candidate.driver_data.get(15).copied().unwrap_or_default() == 0
                    && candidate.driver_data.get(8).copied().unwrap_or_default() == effective_group)
                    .then_some(*candidate_id)
            }) else {
                return false;
            };
            if let Some(item) = self.items.get_mut(&item_id) {
                write_driver_data_u32(item, 10, found_id.0);
            }
            found_id
        };

        if item_x != character_x || item_y != character_y {
            let toggled = self.toggle_door(item_id, character_id) == DoorToggleResult::Toggled;
            let opened = self.items.get(&item_id).is_some_and(door_open_state);
            let partner_closed = self
                .items
                .get(&partner_id)
                .is_some_and(|partner| !door_open_state(partner));
            if partner_id != item_id && opened && partner_closed {
                self.toggle_door(partner_id, character_id);
            }
            return toggled;
        }

        if partner_id == item_id || no_target {
            return false;
        }
        if self
            .items
            .get(&partner_id)
            .is_some_and(|partner| !door_open_state(partner))
        {
            self.toggle_door(partner_id, character_id);
        }

        let Some(partner) = self.items.get(&partner_id) else {
            return false;
        };
        let (target_x, target_y) = (partner.x, partner.y);
        let (dx, dy) = self
            .characters
            .get(&character_id)
            .map(|character| {
                (
                    i32::from(character.tox) - i32::from(character.x),
                    i32::from(character.toy) - i32::from(character.y),
                )
            })
            .unwrap_or((0, 0));

        if let Some(partner) = self.items.get_mut(&partner_id) {
            partner.driver_data.resize(10, 0);
            partner.driver_data[9] = 1;
        }
        let teleported = self.teleport_character(character_id, target_x, target_y, false);
        if let Some(partner) = self.items.get_mut(&partner_id) {
            partner.driver_data.resize(10, 0);
            partner.driver_data[9] = 0;
        }
        if teleported && (dx != 0 || dy != 0) {
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.tox = (i32::from(character.x) + dx).clamp(0, u16::MAX as i32) as u16;
                character.toy = (i32::from(character.y) + dy).clamp(0, u16::MAX as i32) as u16;
            }
        }
        teleported
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

    pub fn teleport_character_same_area(
        &mut self,
        character_id: CharacterId,
        x: u16,
        y: u16,
        extended: bool,
    ) -> bool {
        self.teleport_character(character_id, x, y, extended)
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

    fn player_can_attack_target(
        &self,
        player: &mut PlayerRuntime,
        attacker_id: CharacterId,
        target_id: CharacterId,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&attacker_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        let attack_policy = RuntimePlayerAttackPolicy {
            attacker_runtime: player,
        };
        let can_attack = can_attack_in_area_with_clan_policy(
            attacker,
            target,
            &self.map,
            area_id,
            &attack_policy,
        );
        if !can_attack {
            self.remove_stale_pvp_hate_if_legacy_check_fails(
                player,
                attacker_id,
                target_id,
                area_id,
            );
        }
        can_attack
    }

    fn remove_stale_pvp_hate_if_legacy_check_fails(
        &self,
        player: &mut PlayerRuntime,
        attacker_id: CharacterId,
        target_id: CharacterId,
        area_id: u16,
    ) {
        if area_id == 1 {
            return;
        }
        let Some(attacker) = self.characters.get(&attacker_id) else {
            return;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return;
        };
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
        self.walk_or_use_driver(character_id, direction, area_id)
    }

    fn walk_or_use_driver(
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

    pub fn secure_move_driver(
        &mut self,
        character_id: CharacterId,
        target_x: u16,
        target_y: u16,
        direction: u8,
        ret: i32,
        last_action: u16,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };

        if character.x != target_x || character.y != target_y {
            if (last_action != action::USE || ret != 2)
                && self.setup_walk_toward(
                    character_id,
                    usize::from(target_x),
                    usize::from(target_y),
                    0,
                    area_id,
                    false,
                )
            {
                return true;
            }
            return self.teleport_character(character_id, target_x, target_y, false);
        }

        if character.dir != direction {
            if let Some(character) = self.characters.get_mut(&character_id) {
                let _ = turn(character, direction);
            }
        }
        false
    }

    fn setup_walk_direction(
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
        let installed = self.install_bless_spell(target_id, strength, duration);
        if installed {
            self.queue_sound_area(usize::from(caster.x), usize::from(caster.y), 29);
        }
        installed
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
        self.queue_sound_area(usize::from(caster.x), usize::from(caster.y), 5);
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

    fn complete_firering(
        &mut self,
        caster_id: CharacterId,
        can_attack_target: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) -> bool {
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
            20,
            50,
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
                if !can_attack_target(caster_id, &caster, target, &self.map) {
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
            self.create_show_effect(
                EF_BURN,
                target_id,
                self.tick.0 as u32,
                self.tick.0.saturating_add(8) as u32,
                20,
                0,
            );
            self.apply_legacy_hurt(target_id, Some(caster_id), damage, 10, 30, 85);
        }

        self.queue_sound_area(caster_x, caster_y, 5);

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

    fn complete_pulse(
        &mut self,
        caster_id: CharacterId,
        can_attack_target: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) -> bool {
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
                if !can_attack_target(caster_id, &caster, target, &self.map) {
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
            self.apply_legacy_hurt(target_id, Some(caster_id), damage, 1, 0, 100);
        }

        self.create_pulse_effect(
            caster.x,
            caster.y,
            character_value(&caster, CharacterValue::Pulse),
        );
        true
    }

    fn complete_freeze(
        &mut self,
        caster_id: CharacterId,
        can_attack_target: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) -> bool {
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
                if !can_attack_target(caster_id, &caster, target, &self.map)
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
        for (target_id, modifier) in targets {
            self.install_speed_spell(target_id, IDR_FREEZE, "Freeze", modifier, duration);
            let Some(target) = self.characters.get(&target_id) else {
                continue;
            };
            let curse_strength = character_value_present(&caster, CharacterValue::Demon)
                - character_value(target, CharacterValue::Cold);
            if caster.flags.contains(CharacterFlags::IDEMON) && curse_strength > 0 {
                if self.install_curse_spell(target_id, curse_strength, curse_strength * 50) {
                    self.pending_system_texts.push(WorldSystemText {
                        character_id: target_id,
                        message: format!(
                            "You have been frozen by {}. You feel like you'll never thaw again.",
                            caster.name
                        ),
                    });
                }
            }
        }
        self.queue_sound_area(caster_x, caster_y, 31);
        true
    }

    fn complete_warcry(
        &mut self,
        caster_id: CharacterId,
        can_attack_target: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) -> bool {
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
                if !can_attack_target(caster_id, &caster, target, &self.map) {
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

        for (target_id, modifier, damage) in targets {
            if !self.install_speed_spell(target_id, IDR_WARCRY, "Warcry", modifier, duration) {
                continue;
            }
            if damage > 0 {
                self.apply_legacy_hurt(target_id, Some(caster_id), damage, 1, 0, 0);
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

        true
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
            if let (Some(target), Some(item)) = (
                self.characters.get_mut(&target_id),
                self.items.get(&item_id),
            ) {
                apply_item_modifier_deltas(target, item, -1);
            }
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
            if let Some(item) = self.items.get(&item_id) {
                apply_item_modifier_deltas(target, item, 1);
            }
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

    pub fn install_bonus_spell(
        &mut self,
        target_id: CharacterId,
        driver: u16,
        strength: i32,
        duration: i32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };
        let Some((name, modifier_index)) = bonus_spell_shape(driver) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration.max(0) as u32);
        let mut driver_data = Vec::with_capacity(4);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());

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
            modifier_index: [modifier_index as i16, 0, 0, 0, 0],
            modifier_value: [
                strength.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                0,
                0,
                0,
                0,
            ],
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
            if let Some(item) = self.items.get(&item_id) {
                apply_item_modifier_deltas(target, item, 1);
            }
            self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
            true
        } else {
            self.items.remove(&item_id);
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
        consume_source_item: bool,
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

        if consume_source_item && !self.destroy_item(potion_item_id) {
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
            if let Some(item) = self.items.get(&item_id) {
                apply_item_modifier_deltas(character, item, 1);
            }
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
            if let Some(item) = self.items.get(&item_id) {
                apply_item_modifier_deltas(target, item, 1);
            }
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

    fn install_curse_spell(
        &mut self,
        target_id: CharacterId,
        strength: i32,
        max_strength: i32,
    ) -> bool {
        if strength <= 0 || max_strength <= 0 {
            return false;
        }
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = add_same_spell_slot(&target, &self.items, IDR_CURSE) else {
            return false;
        };

        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add((30 * 60 * TICKS_PER_SECOND) as u32);
        if let Some(item_id) = target.inventory.get(slot).copied().flatten() {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            let current_strength = -i32::from(item.modifier_value[0]);
            if current_strength >= max_strength {
                return false;
            }
            let added_strength = strength.min(max_strength - current_strength);
            for value in &mut item.modifier_value[..4] {
                *value = (i32::from(*value) - added_strength)
                    .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            let mut missing_effect = None;
            if let Some(effect) = self.effects.values_mut().find(|effect| {
                effect.effect_type == EF_CURSE && effect.target_character == Some(target_id)
            }) {
                effect.strength += added_strength;
            } else {
                missing_effect = Some((
                    read_spell_start_tick(&item.driver_data).unwrap_or(start_tick),
                    read_spell_expire_tick(&item.driver_data).unwrap_or(expire_tick),
                    -i32::from(item.modifier_value[0]),
                ));
            }
            if let Some((effect_start, effect_stop, effect_strength)) = missing_effect {
                self.create_show_effect(
                    EF_CURSE,
                    target_id,
                    effect_start,
                    effect_stop,
                    0,
                    effect_strength,
                );
            }
            if let Some(target) = self.characters.get_mut(&target_id) {
                for value in [
                    CharacterValue::Intelligence,
                    CharacterValue::Wisdom,
                    CharacterValue::Agility,
                    CharacterValue::Strength,
                ] {
                    add_character_value_delta(target, value, -added_strength);
                }
                target
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            return true;
        }

        let item_id = self.next_runtime_item_id();
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());
        let item = Item {
            id: item_id,
            name: "Curse".to_string(),
            description: "A Spell of Curse.".to_string(),
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
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                0,
            ],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_CURSE,
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
            if let Some(item) = self.items.get(&item_id) {
                apply_item_modifier_deltas(target, item, 1);
            }
            self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
            self.create_show_effect(EF_CURSE, target_id, start_tick, expire_tick, 0, strength);
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
            refresh_driver_spell_flags(target, &self.items);
            self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
            true
        } else {
            self.items.remove(&item_id);
            false
        }
    }

    fn install_infravision_spell(&mut self, target_id: CharacterId) -> bool {
        self.install_timed_identity_spell(
            target_id,
            IDR_INFRARED,
            TICKS_PER_SECOND * 60 * 10,
            "Infravision",
            "A Spell of Infravision.",
        )
    }

    fn install_oxygen_spell(&mut self, target_id: CharacterId) -> bool {
        self.install_oxygen_spell_for_ticks(target_id, TICKS_PER_SECOND * 60)
    }

    fn install_oxygen_spell_for_ticks(
        &mut self,
        target_id: CharacterId,
        duration_ticks: u64,
    ) -> bool {
        self.install_timed_identity_spell(
            target_id,
            IDR_OXYGEN,
            duration_ticks,
            "Oxygen",
            "A Spell of Oxygen.",
        )
    }

    fn apply_lab3_whiteberry(&mut self, target_id: CharacterId, light_power: i16) -> (bool, bool) {
        if light_power <= 0 {
            return (false, false);
        }

        let existing_light_id = self.characters.get(&target_id).and_then(|character| {
            character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
                .iter()
                .filter_map(|slot| *slot)
                .find(|item_id| {
                    self.items.get(item_id).is_some_and(|item| {
                        item.driver == IDR_LAB3_PLANT && item.driver_data.first() == Some(&10)
                    })
                })
        });

        if let Some(item_id) = existing_light_id {
            let Some(old_item_light) = self.items.get(&item_id).map(|item| item.modifier_value[0])
            else {
                return (false, false);
            };
            let new_item_light = old_item_light.saturating_add(light_power).min(255);
            if let Some(item) = self.items.get_mut(&item_id) {
                item.modifier_value[0] = new_item_light;
            }
            let old_character_light = self
                .characters
                .get(&target_id)
                .map(character_light_value)
                .unwrap_or_default();
            if let Some(character) = self.characters.get_mut(&target_id) {
                if let Some(light) = character
                    .values
                    .get_mut(0)
                    .and_then(|values| values.get_mut(CharacterValue::Light as usize))
                {
                    *light = light.saturating_add(new_item_light - old_item_light);
                }
                character
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            self.refresh_character_light_after_value_change(target_id, old_character_light);
            return (true, false);
        }

        let Some(slot) = self.characters.get(&target_id).and_then(|character| {
            character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
                .iter()
                .rposition(|slot| slot.is_none())
                .map(|offset| SPELL_SLOT_START + offset)
        }) else {
            return (false, false);
        };

        let item_light = light_power.saturating_mul(4).saturating_div(3).min(255);
        if item_light <= 0 {
            return (false, false);
        }

        let item_id = self.next_runtime_item_id();
        let item = Item {
            id: item_id,
            name: "Whiteberry Light".to_string(),
            description: "A whiteberry light spell.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [CharacterValue::Light as i16, 0, 0, 0, 0],
            modifier_value: [item_light, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_LAB3_PLANT,
            driver_data: vec![10, 0, 0, item_light as u8],
            serial: item_id.0,
        };

        let old_character_light = self
            .characters
            .get(&target_id)
            .map(character_light_value)
            .unwrap_or_default();
        self.items.insert(item_id, item);
        if let Some(character) = self.characters.get_mut(&target_id) {
            if character.inventory.len() <= slot {
                self.items.remove(&item_id);
                return (false, false);
            }
            character.inventory[slot] = Some(item_id);
            if let Some(light) = character
                .values
                .get_mut(0)
                .and_then(|values| values.get_mut(CharacterValue::Light as usize))
            {
                *light = light.saturating_add(item_light);
            }
            character
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return (false, false);
        }
        self.refresh_character_light_after_value_change(target_id, old_character_light);
        self.schedule_item_driver_timer_with_context(
            item_id,
            CharacterId(0),
            20 * TICKS_PER_SECOND,
            true,
        );
        (true, true)
    }

    fn decay_lab3_whiteberry_light(&mut self, item_id: ItemId) -> bool {
        let Some((target_id, old_item_light)) = self.items.get(&item_id).and_then(|item| {
            (item.driver == IDR_LAB3_PLANT && item.driver_data.first() == Some(&10))
                .then_some((item.carried_by?, item.modifier_value[0]))
        }) else {
            return false;
        };
        let old_character_light = self
            .characters
            .get(&target_id)
            .map(character_light_value)
            .unwrap_or_default();
        let new_item_light = 3 * old_item_light / 4;

        if new_item_light < 8 {
            if let Some(character) = self.characters.get_mut(&target_id) {
                if let Some(light) = character
                    .values
                    .get_mut(0)
                    .and_then(|values| values.get_mut(CharacterValue::Light as usize))
                {
                    *light = light.saturating_sub(old_item_light);
                }
                character
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            self.destroy_item(item_id);
            self.refresh_character_light_after_value_change(target_id, old_character_light);
            return true;
        }

        if let Some(item) = self.items.get_mut(&item_id) {
            item.modifier_value[0] = new_item_light;
            if item.driver_data.len() < 4 {
                item.driver_data.resize(4, 0);
            }
            item.driver_data[3] = new_item_light as u8;
        }
        if let Some(character) = self.characters.get_mut(&target_id) {
            if let Some(light) = character
                .values
                .get_mut(0)
                .and_then(|values| values.get_mut(CharacterValue::Light as usize))
            {
                *light = light.saturating_add(new_item_light - old_item_light);
            }
            character.flags.insert(CharacterFlags::UPDATE);
        }
        self.refresh_character_light_after_value_change(target_id, old_character_light);
        self.schedule_item_driver_timer_with_context(
            item_id,
            CharacterId(0),
            20 * TICKS_PER_SECOND,
            true,
        );
        false
    }

    fn install_underwater_talk_spell(
        &mut self,
        target_id: CharacterId,
        duration_ticks: u64,
    ) -> bool {
        self.install_timed_identity_spell(
            target_id,
            IDR_UWTALK,
            duration_ticks,
            "Underwater Talk",
            "A Spell of Underwater Talk.",
        )
    }

    fn install_timed_identity_spell(
        &mut self,
        target_id: CharacterId,
        driver: u16,
        duration_ticks: u64,
        name: &str,
        description: &str,
    ) -> bool {
        if duration_ticks == 0 {
            return false;
        }
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration_ticks as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: name.to_string(),
            description: description.to_string(),
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
            refresh_driver_spell_flags(target, &self.items);
            self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
            true
        } else {
            self.items.remove(&item_id);
            false
        }
    }

    fn remove_driver_spells(&mut self, target_id: CharacterId, driver: u16) {
        let item_ids: Vec<ItemId> = self
            .characters
            .get(&target_id)
            .map(|character| {
                character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
                    .iter()
                    .filter_map(|slot| *slot)
                    .filter(|item_id| {
                        self.items
                            .get(item_id)
                            .is_some_and(|item| item.driver == driver)
                    })
                    .collect()
            })
            .unwrap_or_default();

        for item_id in item_ids {
            self.destroy_item(item_id);
        }
        if let Some(character) = self.characters.get_mut(&target_id) {
            refresh_driver_spell_flags(character, &self.items);
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
            if let Some(item) = self.items.get(&item_id) {
                apply_item_modifier_deltas(character, item, 1);
            }
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
        let character_ids: Vec<_> = self.characters.keys().copied().collect();
        for character_id in character_ids {
            if let Some(character) = self.characters.get_mut(&character_id) {
                refresh_driver_spell_flags(character, &self.items);
            }
        }

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
                                IDR_CURSE => {
                                    self.create_show_effect(
                                        EF_CURSE,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        -i32::from(modifier_value),
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
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        if !character.flags.contains(CharacterFlags::USED) || character.id.0 != character_serial {
            return false;
        }
        let Some(item) = self.items.get(&item_id) else {
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
            if let Some(item) = self.items.get_mut(&item_id) {
                item.modifier_value[0] = item.modifier_value[0].saturating_sub(1).max(-1000);
            }
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.flags.insert(CharacterFlags::UPDATE);
            }
        }

        self.apply_legacy_hurt(character_id, None, crate::entity::POWERSCALE / 3, 1, 0, 50);

        tick = if tick == 0 { 9 } else { tick - 1 };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
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
        let item_for_modifier_removal = item.clone();
        let spell_driver = item.driver;
        if character.inventory.get(slot).copied().flatten() != Some(item_id) {
            return false;
        }

        let old_speed = character_value(character, CharacterValue::Speed);
        let old_duration = character.duration;
        character.inventory[slot] = None;
        character
            .flags
            .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        self.items.remove(&item_id);
        apply_item_modifier_deltas(character, &item_for_modifier_removal, -1);
        refresh_driver_spell_flags(character, &self.items);
        if spell_driver == IDR_FREEZE && old_duration != 0 {
            let real_duration = speed_ticks_inverse(old_speed, character.speed_mode, old_duration);
            let new_duration = speed_ticks(
                character_value(character, CharacterValue::Speed),
                character.speed_mode,
                real_duration,
            );
            character.duration = new_duration;
            character.step = character.step * new_duration / old_duration;
        }
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

fn fdemon_loader_power_for_light(
    items: &HashMap<ItemId, Item>,
    light_item_id: ItemId,
) -> Option<u16> {
    let light = items.get(&light_item_id)?;
    let mut max_power = 0u16;
    let mut found = false;

    for loader_nr in 1..=3u8 {
        let nearest = items
            .values()
            .filter(|item| {
                item.driver == IDR_FDEMONLOADER && item.driver_data.first() == Some(&loader_nr)
            })
            .min_by_key(|item| {
                i32::from(light.x).abs_diff(i32::from(item.x))
                    + i32::from(light.y).abs_diff(i32::from(item.y))
            });

        if let Some(loader) = nearest {
            found = true;
            if let Some(bytes) = loader.driver_data.get(1..3) {
                if let Ok(bytes) = <[u8; 2]>::try_from(bytes) {
                    max_power = max_power.max(u16::from_le_bytes(bytes));
                }
            }
        }
    }

    found.then_some(max_power)
}

fn edemon_section_power_for_light(
    items: &HashMap<ItemId, Item>,
    light_item_id: ItemId,
) -> Option<u8> {
    let light = items.get(&light_item_id)?;
    let section = light.driver_data.first().copied().unwrap_or_default();
    let mut max_power = 0u8;
    let mut found = false;

    for loader in items.values().filter(|item| {
        item.driver == IDR_EDEMONLOADER && item.driver_data.first() == Some(&section)
    }) {
        found = true;
        max_power = max_power.max(loader.driver_data.get(1).copied().unwrap_or_default());
    }

    found.then_some(max_power)
}

fn edemon_tube_target(
    items: &HashMap<ItemId, Item>,
    map: &MapGrid,
    tube_item_id: ItemId,
) -> Option<(u16, u16)> {
    let tube = items.get(&tube_item_id)?;
    let section = tube.driver_data.first().copied().unwrap_or_default();

    for loader in items.values().filter(|item| {
        item.driver == IDR_EDEMONLOADER && item.driver_data.first() == Some(&section)
    }) {
        let x = usize::from(loader.x);
        let y = usize::from(loader.y);
        if y < usize::from(u16::MAX) {
            if let Some(tile) = map.tile(x, y + 1) {
                if !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                {
                    return Some((loader.x, loader.y.saturating_add(1)));
                }
            }
        }
        if y > 0 {
            if let Some(tile) = map.tile(x, y - 1) {
                if !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
                {
                    return Some((loader.x, loader.y.saturating_sub(1)));
                }
            }
        }
    }

    None
}

fn edemon_gate_slot_offset(mode: u8, slot: usize) -> usize {
    match mode {
        0 => 4 + slot * 4,
        1 => EDEMON_GATE_MODE1_SLOT_BASE + slot * 4,
        _ => 4 + slot * 4,
    }
}

fn fdemon_gate_slot_offset(slot: usize) -> usize {
    4 + slot * 4
}

fn read_spell_start_tick(driver_data: &[u8]) -> Option<u32> {
    let bytes = driver_data.get(4..8)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
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

fn bonus_spell_shape(driver: u16) -> Option<(&'static str, CharacterValue)> {
    Some(match driver {
        IDR_ARMOR => ("Armor", CharacterValue::Armor),
        IDR_WEAPON => ("Weapon", CharacterValue::Weapon),
        IDR_MANA => ("Mana", CharacterValue::Mana),
        IDR_HP => ("HP", CharacterValue::Hp),
        _ => return None,
    })
}

fn add_character_value_delta(character: &mut Character, value: CharacterValue, delta: i32) {
    if let Some(slot) = character
        .values
        .get_mut(0)
        .and_then(|values| values.get_mut(value as usize))
    {
        *slot = (i32::from(*slot) + delta).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
}

fn apply_item_modifier_deltas(character: &mut Character, item: &Item, sign: i32) {
    for (&modifier_index, &modifier_value) in
        item.modifier_index.iter().zip(item.modifier_value.iter())
    {
        if modifier_value == 0 || modifier_index < 0 {
            continue;
        }
        let Ok(value_index) = usize::try_from(modifier_index) else {
            continue;
        };
        if value_index >= CHARACTER_VALUE_COUNT {
            continue;
        }
        let Some(value) = character_value_from_index(value_index) else {
            continue;
        };
        add_character_value_delta(character, value, i32::from(modifier_value) * sign);
    }
}

fn refresh_driver_spell_flags(character: &mut Character, items: &HashMap<ItemId, Item>) {
    let mut has_infravision_spell = false;
    let mut has_nonomagic_spell = false;
    let mut has_oxygen_spell = false;

    for item_id in character.inventory.iter().take(30).flatten() {
        let Some(item) = items.get(item_id) else {
            continue;
        };
        match item.driver {
            IDR_INFRARED => has_infravision_spell = true,
            IDR_NONOMAGIC => has_nonomagic_spell = true,
            IDR_OXYGEN => has_oxygen_spell = true,
            _ => {}
        }
    }

    let old_flags = character.flags;
    character
        .flags
        .set(CharacterFlags::INFRAVISION, has_infravision_spell);
    character
        .flags
        .set(CharacterFlags::NONOMAGIC, has_nonomagic_spell);
    character
        .flags
        .set(CharacterFlags::OXYGEN, has_oxygen_spell);
    if character.flags != old_flags {
        character.flags.insert(CharacterFlags::UPDATE);
    }
}

fn character_value_from_index(index: usize) -> Option<CharacterValue> {
    Some(match index {
        0 => CharacterValue::Hp,
        1 => CharacterValue::Endurance,
        2 => CharacterValue::Mana,
        3 => CharacterValue::Wisdom,
        4 => CharacterValue::Intelligence,
        5 => CharacterValue::Agility,
        6 => CharacterValue::Strength,
        7 => CharacterValue::Armor,
        8 => CharacterValue::Weapon,
        9 => CharacterValue::Light,
        10 => CharacterValue::Speed,
        11 => CharacterValue::Pulse,
        12 => CharacterValue::Dagger,
        13 => CharacterValue::Hand,
        14 => CharacterValue::Staff,
        15 => CharacterValue::Sword,
        16 => CharacterValue::TwoHand,
        17 => CharacterValue::ArmorSkill,
        18 => CharacterValue::Attack,
        19 => CharacterValue::Parry,
        20 => CharacterValue::Warcry,
        21 => CharacterValue::Tactics,
        22 => CharacterValue::Surround,
        23 => CharacterValue::BodyControl,
        24 => CharacterValue::SpeedSkill,
        25 => CharacterValue::Barter,
        26 => CharacterValue::Percept,
        27 => CharacterValue::Stealth,
        28 => CharacterValue::Bless,
        29 => CharacterValue::Heal,
        30 => CharacterValue::Freeze,
        31 => CharacterValue::MagicShield,
        32 => CharacterValue::Flash,
        33 => CharacterValue::Fireball,
        34 => CharacterValue::Empty,
        35 => CharacterValue::Regenerate,
        36 => CharacterValue::Meditate,
        37 => CharacterValue::Immunity,
        38 => CharacterValue::Demon,
        39 => CharacterValue::Duration,
        40 => CharacterValue::Rage,
        41 => CharacterValue::Cold,
        42 => CharacterValue::Profession,
        _ => return None,
    })
}

fn character_value_present(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

fn simple_baddy_earth_task_value(good_fields: i32) -> i32 {
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

fn simple_baddy_attack_task_value(character: &Character, items: &HashMap<ItemId, Item>) -> i32 {
    let attack = simple_baddy_attack_skill(character, items);
    if character_value_present(character, CharacterValue::Attack) != 0 {
        FIGHT_DRIVER_MED_PRIO + attack * 2 / 7 + 10
    } else {
        FIGHT_DRIVER_LOW_PRIO + attack / 3
    }
}

fn simple_baddy_attack_skill(character: &Character, items: &HashMap<ItemId, Item>) -> i32 {
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

fn simple_baddy_fight_skill(character: &Character, items: &HashMap<ItemId, Item>) -> i32 {
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

fn spell_duration_ticks(character: &Character, base_duration: i32) -> i32 {
    if character_value_present(character, CharacterValue::Duration) != 0 {
        base_duration + base_duration * character_value(character, CharacterValue::Duration) / 35
    } else if character.flags.contains(CharacterFlags::ARCH) {
        base_duration + base_duration * character.level as i32 / 35 / 2
    } else {
        base_duration
    }
}

fn is_back_attack_against_target(target: &Character, attacker_x: u16, attacker_y: u16) -> bool {
    let target_x = i32::from(target.x);
    let target_y = i32::from(target.y);
    let attacker_x = i32::from(attacker_x);
    let attacker_y = i32::from(attacker_y);

    match Direction::try_from(target.dir).ok() {
        Some(Direction::Left) => target_x + 1 == attacker_x && target_y == attacker_y,
        Some(Direction::Right) => target_x - 1 == attacker_x && target_y == attacker_y,
        Some(Direction::Down) => target_x == attacker_x && target_y - 1 == attacker_y,
        Some(Direction::Up) => target_x == attacker_x && target_y + 1 == attacker_y,
        _ => false,
    }
}

fn character_is_facing(character: &Character, other: &Character) -> bool {
    let Ok(direction) = Direction::try_from(character.dir) else {
        return false;
    };
    let (dx, dy) = direction.delta();
    let delta_x = i32::from(other.x) - i32::from(character.x);
    let delta_y = i32::from(other.y) - i32::from(character.y);
    match (dx, dy) {
        (1, 0) => delta_x > 0 && delta_y.abs() <= delta_x,
        (-1, 0) => delta_x < 0 && delta_y.abs() <= -delta_x,
        (0, 1) => delta_y > 0 && delta_x.abs() <= delta_y,
        (0, -1) => delta_y < 0 && delta_x.abs() <= -delta_y,
        (1, 1) => delta_x > 0 && delta_y > 0,
        (-1, 1) => delta_x < 0 && delta_y > 0,
        (1, -1) => delta_x > 0 && delta_y < 0,
        (-1, -1) => delta_x < 0 && delta_y < 0,
        _ => false,
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

fn simple_baddy_earth_spell_target(target: &Character) -> (usize, usize) {
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

fn clamp_world_coordinate(value: i32) -> u16 {
    value.clamp(0, (MAX_MAP - 1) as i32) as u16
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

fn write_driver_data_u32(item: &mut Item, offset: usize, value: u32) {
    item.driver_data.resize(offset + 4, 0);
    item.driver_data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
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

fn read_u32_le_prefix(bytes: &[u8]) -> u32 {
    read_u32_le_at(bytes, 0)
}

fn read_u32_le_at(bytes: &[u8], offset: usize) -> u32 {
    let mut raw = [0; 4];
    if offset < bytes.len() {
        let len = (bytes.len() - offset).min(raw.len());
        raw[..len].copy_from_slice(&bytes[offset..offset + len]);
    }
    u32::from_le_bytes(raw)
}

fn write_u32_le_prefix(bytes: &mut Vec<u8>, value: u32) {
    if bytes.len() < 4 {
        bytes.resize(4, 0);
    }
    bytes[..4].copy_from_slice(&value.to_le_bytes());
}

fn timer_callback_character() -> Character {
    Character {
        id: CharacterId(0),
        serial: 0,
        name: String::new(),
        description: String::new(),
        flags: CharacterFlags::empty(),
        sprite: 0,
        c1: 0,
        c2: 0,
        c3: 0,
        driver: 0,
        group: 0,
        clan: 0,
        clan_rank: 0,
        clan_serial: 0,
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
        regen_ticker: 0,
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
        character_driver::{
            CharacterDriverState, SimpleBaddyDriverData, SimpleBaddyEnemy, NTID_GLADIATOR, NT_CHAR,
            NT_DIDHIT, NT_GOTHIT, NT_NPC, NT_SEEHIT,
        },
        direction::Direction,
        entity::{CharacterFlags, CharacterValue, ItemFlags, SpeedMode, MAX_MODIFIERS, POWERSCALE},
        item_driver::{
            UseItemOutcome, IDR_ANTIENCHANTITEM, IDR_BALLTRAP, IDR_BONEBRIDGE, IDR_CALIGAR,
            IDR_CALIGARFLAME, IDR_CHESTSPAWN, IDR_DOOR, IDR_EDEMONBALL, IDR_EDEMONLIGHT,
            IDR_ENCHANTITEM, IDR_FDEMONBLOOD, IDR_FDEMONLAVA, IDR_FIREBALL, IDR_FLAMETHROW,
            IDR_FLASK, IDR_LAB3_PLANT, IDR_LIZARDFLOWER, IDR_NIGHTLIGHT, IDR_ONOFFLIGHT,
            IDR_OXYPOTION, IDR_PALACEGATE, IDR_PALACEKEY, IDR_POTION, IDR_SPECIAL_POTION,
            IDR_SPIKETRAP, IDR_STAFFER2, IDR_STEPTRAP, IDR_TORCH, IDR_USETRAP, IID_AREA18_BONE,
        },
        legacy::action,
        map::{MapFlags, MapGrid},
        player::{PlayerActionCode, PlayerRuntime, QueuedAction},
        spell::{
            IDR_INFRARED, IDR_NONOMAGIC, IDR_OXYGEN, IDR_POISON0, IDR_POISON1, IDR_POISON2,
            IDR_UWTALK,
        },
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
    fn char_swap_exchanges_idle_character_with_visible_playerlike_target() {
        let mut world = World::default();
        let mut actor = character(1);
        actor.dir = Direction::Right as u8;
        let mut target = character(2);
        target.flags |= CharacterFlags::PLAYERLIKE;
        assert!(world.spawn_character(actor, 10, 10));
        assert!(world.spawn_character(target, 11, 10));

        assert!(world.char_swap(CharacterId(1)));

        assert_eq!(
            (
                world.characters[&CharacterId(1)].x,
                world.characters[&CharacterId(1)].y
            ),
            (11, 10)
        );
        assert_eq!(
            (
                world.characters[&CharacterId(2)].x,
                world.characters[&CharacterId(2)].y
            ),
            (10, 10)
        );
        assert_eq!(world.map.tile(11, 10).unwrap().character, 1);
        assert_eq!(world.map.tile(10, 10).unwrap().character, 2);
    }

    #[test]
    fn chestspawn_spawn_result_marks_active_and_schedules_poll() {
        let mut world = World::default();
        let mut spawner = item(8, ItemFlags::USE);
        spawner.driver = IDR_CHESTSPAWN;
        spawner.sprite = 1234;
        spawner.x = 10;
        spawner.y = 10;
        spawner.driver_data = vec![0, 0, 0, 0, 0, 0, 0, 0];
        world.items.insert(spawner.id, spawner);

        assert!(world.apply_chestspawn_spawn_result(ItemId(8), CharacterId(44), 0));
        let spawner = &world.items[&ItemId(8)];
        assert_eq!(spawner.sprite, 1235);
        assert_eq!(spawner.driver_data[1], 1);
        assert_eq!(&spawner.driver_data[2..4], &44_u16.to_le_bytes());
        assert_eq!(world.process_due_timers(2), Vec::<ItemDriverOutcome>::new());
        world.tick.0 = TICKS_PER_SECOND * 10;
        let outcomes = world.process_due_timers(2);
        assert_eq!(outcomes.len(), 1);
    }

    #[test]
    fn chestspawn_timer_resets_when_spawn_is_gone() {
        let mut world = World::default();
        let mut spawner = item(8, ItemFlags::USE);
        spawner.driver = IDR_CHESTSPAWN;
        spawner.sprite = 1235;
        spawner.x = 10;
        spawner.y = 10;
        spawner.driver_data = vec![0, 1, 44, 0, 0, 0, 0, 0];
        world.items.insert(spawner.id, spawner);
        assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));
        world.tick.0 = 1;

        let outcomes = world.process_due_timers(2);

        assert_eq!(outcomes.len(), 1);
        let spawner = &world.items[&ItemId(8)];
        assert_eq!(spawner.sprite, 1234);
        assert_eq!(spawner.driver_data[1], 0);
    }

    #[test]
    fn char_swap_rejects_invisible_targets() {
        let mut world = World::default();
        let mut actor = character(1);
        actor.dir = Direction::Right as u8;
        let mut target = character(2);
        target.flags |= CharacterFlags::PLAYER | CharacterFlags::INVISIBLE;
        assert!(world.spawn_character(actor, 10, 10));
        assert!(world.spawn_character(target, 11, 10));

        assert!(!world.char_swap(CharacterId(1)));

        assert_eq!(
            (
                world.characters[&CharacterId(1)].x,
                world.characters[&CharacterId(1)].y
            ),
            (10, 10)
        );
        assert_eq!(
            (
                world.characters[&CharacterId(2)].x,
                world.characters[&CharacterId(2)].y
            ),
            (11, 10)
        );
    }

    #[test]
    fn walk_swap_or_use_falls_back_to_use_after_blocked_walk_and_no_swap() {
        let mut world = World::default();
        assert!(world.spawn_character(character(1), 10, 10));
        let mut lever = item(1, ItemFlags::USE | ItemFlags::MOVEBLOCK);
        assert!(world.map.set_item_map(&mut lever, 11, 10));
        world.add_item(lever);

        assert!(world.walk_swap_or_use_driver(CharacterId(1), Direction::Right, 1));

        let actor = &world.characters[&CharacterId(1)];
        assert_eq!(actor.action, action::USE);
        assert_eq!(actor.act1, 1);
    }

    #[test]
    fn legacy_hurt_applies_armor_lifeshield_and_hit_notifications() {
        let mut world = World::default();
        world.tick = Tick(1234);
        let mut target = character(1);
        target.hp = 5 * POWERSCALE;
        target.lifeshield = POWERSCALE;
        target.values[0][CharacterValue::Armor as usize] = 20;
        assert!(world.spawn_character(target, 10, 10));
        assert!(world.spawn_character(character(2), 11, 10));
        assert!(world.spawn_character(character(3), 12, 10));

        let outcome = world
            .apply_legacy_hurt(
                CharacterId(1),
                Some(CharacterId(2)),
                5 * POWERSCALE,
                5,
                90,
                75,
            )
            .unwrap();

        assert_eq!(outcome.damage_after_armor, 4_800);
        assert_eq!(outcome.shield_absorbed, POWERSCALE);
        assert_eq!(outcome.hp_damage, 3_800);
        let target = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(target.hp, 1_200);
        assert_eq!(target.lifeshield, 0);
        assert_eq!(target.regen_ticker, 1234);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
        assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
        assert_eq!(target.driver_messages[0].dat1, 2);
        assert_eq!(
            world.characters[&CharacterId(2)].driver_messages[0].message_type,
            NT_DIDHIT
        );
        assert_eq!(
            world.characters[&CharacterId(3)].driver_messages[0].message_type,
            NT_SEEHIT
        );
    }

    #[test]
    fn legacy_hurt_queues_showattack_debug_text_when_enabled() {
        let mut world = World::default();
        world.show_attack_debug = true;
        let mut target = character(1);
        target.hp = 5 * POWERSCALE;
        target.values[0][CharacterValue::Armor as usize] = 20;
        assert!(world.spawn_character(target, 10, 10));
        assert!(world.spawn_character(character(2), 11, 10));

        world.apply_legacy_hurt(
            CharacterId(1),
            Some(CharacterId(2)),
            5 * POWERSCALE,
            5,
            90,
            75,
        );

        assert_eq!(
            world.drain_pending_system_texts(),
            vec![
                WorldSystemText {
                    character_id: CharacterId(1),
                    message: "hurt by Character, dam=5.00, armor=0.20 armorper=90 shieldper=75"
                        .to_string(),
                },
                WorldSystemText {
                    character_id: CharacterId(1),
                    message: "dam after armor: 4.80".to_string(),
                },
            ]
        );
    }

    #[test]
    fn legacy_hurt_queues_player_ouch_and_death_sounds() {
        let mut world = World::default();
        let mut male = character(1);
        male.flags |= CharacterFlags::PLAYER | CharacterFlags::MALE;
        male.hp = 5 * POWERSCALE;
        assert!(world.spawn_character(male, 10, 10));

        let outcome = world
            .apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 0)
            .unwrap();

        assert_eq!(outcome.hp_damage, POWERSCALE);
        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].character_id, CharacterId(1));
        assert_eq!(sounds[0].special.special_type, 9);

        let mut female = character(2);
        female.flags |= CharacterFlags::PLAYER | CharacterFlags::FEMALE;
        female.hp = POWERSCALE;
        assert!(world.spawn_character(female, 11, 10));

        let outcome = world
            .apply_legacy_hurt(CharacterId(2), None, POWERSCALE, 1, 0, 0)
            .unwrap();

        assert!(outcome.killed);
        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 4);
        assert_eq!(sounds[0].special.special_type, 32);
        assert_eq!(sounds[1].special.special_type, 32);
        assert_eq!(sounds[2].special.special_type, 33);
        assert_eq!(sounds[3].special.special_type, 33);
    }

    #[test]
    fn legacy_hurt_nodeath_player_still_queues_death_sound() {
        let mut world = World::default();
        let mut target = character(1);
        target.flags |= CharacterFlags::PLAYER | CharacterFlags::MALE | CharacterFlags::NODEATH;
        target.hp = 700;
        assert!(world.spawn_character(target, 10, 10));

        let outcome = world
            .apply_legacy_hurt(CharacterId(1), None, 800, 1, 0, 0)
            .unwrap();

        assert!(outcome.nodeath_saved);
        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].special.special_type, 4);
    }

    #[test]
    fn legacy_hurt_creates_magicshield_visual_on_shield_absorption() {
        let mut world = World::default();
        world.tick = Tick(77);
        let mut target = character(1);
        target.hp = 5 * POWERSCALE;
        target.lifeshield = POWERSCALE;
        target.values[1][CharacterValue::MagicShield as usize] = 10;
        assert!(world.spawn_character(target, 10, 10));

        let outcome = world
            .apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 100)
            .unwrap();

        assert_eq!(outcome.shield_absorbed, POWERSCALE);
        let effect = world
            .effects
            .values()
            .find(|effect| effect.effect_type == EF_MAGICSHIELD)
            .unwrap();
        assert_eq!(effect.target_character, Some(CharacterId(1)));
        assert_eq!(effect.start_tick, 77);
        assert_eq!(effect.stop_tick, 80);
        assert_eq!(effect.light, 16);
        assert_eq!(effect.strength, 0);
    }

    #[test]
    fn legacy_hurt_does_not_duplicate_active_magicshield_visual() {
        let mut world = World::default();
        let mut target = character(1);
        target.hp = 5 * POWERSCALE;
        target.lifeshield = 2 * POWERSCALE;
        target.values[1][CharacterValue::MagicShield as usize] = 10;
        assert!(world.spawn_character(target, 10, 10));
        world.create_show_effect(EF_MAGICSHIELD, CharacterId(1), 1, 4, 16, 0);

        world.apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 100);

        assert_eq!(
            world
                .effects
                .values()
                .filter(|effect| effect.effect_type == EF_MAGICSHIELD)
                .count(),
            1
        );
    }

    #[test]
    fn legacy_hurt_ports_immortal_and_nodeath_guards() {
        let mut world = World::default();
        let mut immortal = character(1);
        immortal.flags |= CharacterFlags::IMMORTAL;
        immortal.hp = POWERSCALE;
        immortal.lifeshield = POWERSCALE;
        assert!(world.spawn_character(immortal, 10, 10));

        let outcome = world
            .apply_legacy_hurt(CharacterId(1), None, 5 * POWERSCALE, 1, 0, 100)
            .unwrap();

        let immortal = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(outcome.hp_damage, 0);
        assert_eq!(immortal.hp, POWERSCALE);
        assert_eq!(immortal.lifeshield, POWERSCALE);

        let mut nodeath = character(2);
        nodeath.flags |= CharacterFlags::NODEATH;
        nodeath.hp = 700;
        assert!(world.spawn_character(nodeath, 11, 10));

        let outcome = world
            .apply_legacy_hurt(CharacterId(2), None, POWERSCALE, 1, 0, 0)
            .unwrap();

        let nodeath = world.characters.get(&CharacterId(2)).unwrap();
        assert!(outcome.nodeath_saved);
        assert_eq!(nodeath.hp, 1);
        assert!(!nodeath.flags.contains(CharacterFlags::DEAD));
    }

    #[test]
    fn legacy_hurt_ports_fdemon_back_attack_gate() {
        let mut world = World::default();
        let mut target = character(1);
        target.flags |= CharacterFlags::FDEMON;
        target.dir = Direction::Right as u8;
        target.hp = 20 * POWERSCALE;
        assert!(world.spawn_character(target, 10, 10));
        assert!(world.spawn_character(character(2), 11, 10));

        let outcome = world
            .apply_legacy_hurt(
                CharacterId(1),
                Some(CharacterId(2)),
                10 * POWERSCALE,
                1,
                0,
                0,
            )
            .unwrap();

        assert_eq!(outcome.damage_after_armor, 10 * POWERSCALE);
        assert_eq!(outcome.hp_damage, 100);
        assert_eq!(world.characters[&CharacterId(1)].hp, 19_900);

        world.remove_character(CharacterId(2));
        assert!(world.spawn_character(character(2), 9, 10));

        let outcome = world
            .apply_legacy_hurt(
                CharacterId(1),
                Some(CharacterId(2)),
                10 * POWERSCALE,
                1,
                0,
                0,
            )
            .unwrap();

        assert_eq!(outcome.damage_after_armor, 10 * POWERSCALE);
        assert_eq!(outcome.hp_damage, 10 * POWERSCALE);
        assert_eq!(world.characters[&CharacterId(1)].hp, 9_900);
    }

    #[test]
    fn legacy_hurt_ports_hardkill_weapon_gate() {
        let mut world = World::default();
        let mut target = character(1);
        target.flags |= CharacterFlags::HARDKILL;
        target.hp = 10 * POWERSCALE;
        target.level = 8;
        assert!(world.spawn_character(target, 10, 10));
        assert!(world.spawn_character(character(2), 11, 10));

        let outcome = world
            .apply_legacy_hurt(
                CharacterId(1),
                Some(CharacterId(2)),
                5 * POWERSCALE,
                1,
                0,
                0,
            )
            .unwrap();

        assert_eq!(outcome.damage_after_armor, 5 * POWERSCALE);
        assert_eq!(outcome.hp_damage, 0);
        assert_eq!(world.characters[&CharacterId(1)].hp, 10 * POWERSCALE);

        let mut weak_weapon = item(7, ItemFlags::USED | ItemFlags::SWORD);
        weak_weapon.template_id = IID_HARDKILL;
        weak_weapon.driver_data.resize(38, 0);
        weak_weapon.driver_data[37] = 7;
        world.items.insert(ItemId(7), weak_weapon);
        world.characters.get_mut(&CharacterId(2)).unwrap().inventory[worn_slot::RIGHT_HAND] =
            Some(ItemId(7));

        let outcome = world
            .apply_legacy_hurt(
                CharacterId(1),
                Some(CharacterId(2)),
                5 * POWERSCALE,
                1,
                0,
                0,
            )
            .unwrap();

        assert_eq!(outcome.hp_damage, 0);
        assert_eq!(world.characters[&CharacterId(1)].hp, 10 * POWERSCALE);

        world.items.get_mut(&ItemId(7)).unwrap().driver_data[37] = 8;

        let outcome = world
            .apply_legacy_hurt(
                CharacterId(1),
                Some(CharacterId(2)),
                5 * POWERSCALE,
                1,
                0,
                0,
            )
            .unwrap();

        assert_eq!(outcome.damage_after_armor, 5 * POWERSCALE);
        assert_eq!(outcome.hp_damage, 5 * POWERSCALE);
        assert_eq!(world.characters[&CharacterId(1)].hp, 5 * POWERSCALE);
    }

    #[test]
    fn simple_baddy_message_actions_use_inventory_hp_potion() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.hp = 40 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 100;
        npc.values[1][CharacterValue::Hp as usize] = 100;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            drink_inventory_potions: 1,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_GOTHIT, 0, 0, 0);
        npc.inventory[30] = Some(ItemId(7));
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE);
        potion.carried_by = Some(CharacterId(1));
        potion.driver = IDR_POTION;
        potion.driver_data = vec![0, 20, 0, 0];
        world.add_character(npc);
        world.items.insert(ItemId(7), potion);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::PotionDrunk {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                hp_added: 20 * POWERSCALE,
                mana_added: 0,
                endurance_added: 0,
            }]
        );
        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.hp, 60 * POWERSCALE);
        assert_eq!(npc.inventory[30], None);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.last_hit, world.tick.0 as i32);
        assert!(npc.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_message_actions_wait_until_current_action_completes() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.action = action::WALK;
        npc.hp = 40 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 100;
        npc.values[1][CharacterValue::Hp as usize] = 100;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            drink_inventory_potions: 1,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_GOTHIT, 0, 0, 0);
        npc.inventory[30] = Some(ItemId(7));
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE);
        potion.carried_by = Some(CharacterId(1));
        potion.driver = IDR_POTION;
        potion.driver_data = vec![0, 20, 0, 0];
        world.add_character(npc);
        world.items.insert(ItemId(7), potion);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert!(outcomes.is_empty());
        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.hp, 40 * POWERSCALE);
        assert_eq!(npc.inventory[30], Some(ItemId(7)));
        assert_eq!(npc.driver_messages.len(), 1);
    }

    #[test]
    fn simple_baddy_message_actions_skip_when_drink_inventory_potions_disabled() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.hp = 40 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 100;
        npc.values[1][CharacterValue::Hp as usize] = 100;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        npc.push_driver_message(NT_GOTHIT, 0, 0, 0);
        npc.inventory[30] = Some(ItemId(7));
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE);
        potion.carried_by = Some(CharacterId(1));
        potion.driver = IDR_POTION;
        potion.driver_data = vec![0, 20, 0, 0];
        world.add_character(npc);
        world.items.insert(ItemId(7), potion);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert!(outcomes.is_empty());
        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.hp, 40 * POWERSCALE);
        assert_eq!(npc.inventory[30], Some(ItemId(7)));
        assert!(npc.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_message_actions_remember_helper_bless_for_noncombat_flow() {
        let mut world = World::default();
        world.tick = Tick(1_000);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.group = 7;
        npc.mana = 10 * POWERSCALE;
        npc.values[0][CharacterValue::Bless as usize] = 40;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helper: 1,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_CHAR, 2, 0, 0);
        let mut existing_bless = item(20, ItemFlags::empty());
        existing_bless.driver = IDR_BLESS;
        npc.inventory[SPELL_SLOT_START] = Some(existing_bless.id);
        let mut friend = character(2);
        friend.group = 7;
        world.items.insert(existing_bless.id, existing_bless);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(friend, 12, 10);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, 0);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.pending_bless_friend, Some(CharacterId(2)));
        assert!(npc.driver_messages.is_empty());

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::BLESS1);
        assert_eq!(npc.act1, 2);
        assert_eq!(npc.mana, 8 * POWERSCALE);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.pending_bless_friend, None);
    }

    #[test]
    fn simple_baddy_message_actions_reject_helper_bless_for_other_group() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.group = 7;
        npc.mana = 10 * POWERSCALE;
        npc.values[0][CharacterValue::Bless as usize] = 40;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helper: 1,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_CHAR, 2, 0, 0);
        let mut other = character(2);
        other.group = 8;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(other, 12, 10);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, 0);
        assert_eq!(npc.mana, 10 * POWERSCALE);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.pending_bless_friend, None);
        assert!(npc.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_message_actions_poison_successful_hit() {
        let mut world = World::default();
        world.tick = Tick(1_000);
        let mut npc = character(1);
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            poison_power: 6,
            poison_type: 2,
            poison_chance: 50,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_DIDHIT, 2, 10, 0);
        let mut target = character(2);
        target.values[1][CharacterValue::Hp as usize] = 100;
        target.hp = 100 * POWERSCALE;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        let outcomes =
            world.process_simple_baddy_message_actions_with_random(CharacterId(1), 1, |_| 49);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let target = world.characters.get(&CharacterId(2)).unwrap();
        let poison_id = target.inventory[29].expect("poison spell item");
        let poison = world.items.get(&poison_id).unwrap();
        assert_eq!(poison.driver, IDR_POISON0 + 2);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
        assert!(world.characters[&CharacterId(1)].driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_message_actions_poison_respects_chance_and_attack_policy() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            poison_power: 6,
            poison_type: 2,
            poison_chance: 50,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_DIDHIT, 2, 10, 0);
        let mut target = character(2);
        target.flags.insert(CharacterFlags::NOATTACK);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        let outcomes =
            world.process_simple_baddy_message_actions_with_random(CharacterId(1), 1, |_| 0);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        assert!(world.characters[&CharacterId(2)].inventory[29].is_none());

        world
            .characters
            .get_mut(&CharacterId(2))
            .unwrap()
            .flags
            .remove(CharacterFlags::NOATTACK);
        world
            .characters
            .get_mut(&CharacterId(1))
            .unwrap()
            .push_driver_message(NT_DIDHIT, 2, 10, 0);

        let outcomes =
            world.process_simple_baddy_message_actions_with_random(CharacterId(1), 1, |_| 50);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        assert!(world.characters[&CharacterId(2)].inventory[29].is_none());
    }

    #[test]
    fn simple_baddy_message_actions_add_npc_alert_enemy_for_same_group_caller() {
        let mut world = World::default();
        world.tick = Tick(123);
        let mut npc = character(1);
        npc.group = 7;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helpid: NTID_GLADIATOR,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_NPC, NTID_GLADIATOR, 2, 99);
        let mut caller = character(2);
        caller.group = 7;
        world.add_character(npc);
        world.add_character(caller);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert_eq!(
            data.enemies,
            vec![SimpleBaddyEnemy {
                target_id: CharacterId(99),
                priority: 1,
                last_seen_tick: 123,
                visible: false,
                last_x: 0,
                last_y: 0,
            }]
        );
    }

    #[test]
    fn simple_baddy_message_actions_add_aggressive_seen_character_enemy() {
        let mut world = World::default();
        world.tick = Tick(321);
        let mut npc = character(1);
        npc.group = 7;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            aggressive: 1,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_CHAR, 2, 0, 0);
        let mut target = character(2);
        target.group = 8;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert_eq!(
            data.enemies,
            vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 0,
                last_seen_tick: 321,
                visible: true,
                last_x: 11,
                last_y: 10,
            }]
        );
    }

    #[test]
    fn simple_baddy_enemy_memory_sorts_and_caps_like_c_table() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: (2..14)
                .map(|id| SimpleBaddyEnemy {
                    target_id: CharacterId(id),
                    priority: if id == 12 { 1 } else { 0 },
                    last_seen_tick: id as i32,
                    visible: id != 13,
                    last_x: 10 + id as u16,
                    last_y: 10,
                })
                .collect(),
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);
        for id in 2..14 {
            world.spawn_character(character(id), 10 + id as usize, 10);
        }

        world.sort_simple_baddy_enemies_like_c(CharacterId(1));

        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.enemies.len(), 10);
        assert_eq!(data.enemies[0].target_id, CharacterId(12));
        assert_eq!(data.enemies[1].target_id, CharacterId(2));
        assert!(!data
            .enemies
            .iter()
            .any(|enemy| enemy.target_id == CharacterId(13)));
    }

    #[test]
    fn simple_baddy_message_actions_rejects_enemy_outside_start_or_char_distance() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.group = 7;
        npc.rest_x = 10;
        npc.rest_y = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            aggressive: 1,
            startdist: 6,
            chardist: 4,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_CHAR, 2, 0, 0);
        let mut target = character(2);
        target.group = 8;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 14, 10);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert!(data.enemies.is_empty());
    }

    #[test]
    fn simple_baddy_message_actions_use_explicit_fight_driver_home_for_start_distance() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.group = 7;
        npc.rest_x = 10;
        npc.rest_y = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            aggressive: 1,
            startdist: 6,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_CHAR, 2, 0, 0);
        let mut target = character(2);
        target.group = 8;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 14, 10);
        world.map.tile_mut(14, 10).unwrap().light = 255;
        assert!(world.set_simple_baddy_home(CharacterId(1), 14, 10));

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.home_x, 14);
        assert_eq!(data.home_y, 10);
        assert_eq!(data.enemies.len(), 1);
        assert_eq!(data.enemies[0].target_id, CharacterId(2));
    }

    #[test]
    fn simple_baddy_message_actions_rejects_non_hurt_enemy_in_neutral_zone() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.group = 7;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            aggressive: 1,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_CHAR, 2, 0, 0);
        let mut target = character(2);
        target.group = 8;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);
        world
            .map
            .tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::NEUTRAL);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert!(data.enemies.is_empty());
    }

    #[test]
    fn simple_baddy_message_actions_keeps_hurt_enemy_in_neutral_zone() {
        let mut world = World::default();
        world.tick = Tick(324);
        let mut npc = character(1);
        npc.group = 7;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        npc.push_driver_message(NT_GOTHIT, 2, 10, 0);
        let mut attacker = character(2);
        attacker.group = 8;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(attacker, 11, 10);
        world
            .map
            .tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::NEUTRAL);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert_eq!(
            data.enemies,
            vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 324,
                visible: true,
                last_x: 11,
                last_y: 10,
            }]
        );
    }

    #[test]
    fn simple_baddy_message_actions_add_defensive_gothit_enemy_without_sight() {
        let mut world = World::default();
        world.tick = Tick(322);
        let mut npc = character(1);
        npc.group = 7;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        npc.push_driver_message(NT_GOTHIT, 2, 10, 0);
        let mut attacker = character(2);
        attacker.group = 8;
        world.add_character(npc);
        world.add_character(attacker);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.last_hit, 322);
        assert_eq!(
            data.enemies,
            vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 322,
                visible: true,
                last_x: 0,
                last_y: 0,
            }]
        );
    }

    #[test]
    fn simple_baddy_message_actions_helper_seen_hit_adds_enemy_for_friend() {
        let mut world = World::default();
        world.tick = Tick(323);
        let mut npc = character(1);
        npc.group = 7;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helper: 1,
            ..SimpleBaddyDriverData::default()
        }));
        npc.push_driver_message(NT_SEEHIT, 2, 3, 0);
        let mut attacker = character(2);
        attacker.group = 8;
        let mut victim = character(3);
        victim.group = 7;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(attacker, 11, 10);
        world.spawn_character(victim, 12, 10);

        let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert_eq!(
            data.enemies,
            vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 323,
                visible: true,
                last_x: 11,
                last_y: 10,
            }]
        );
    }

    #[test]
    fn fight_driver_task_order_sorts_by_descending_legacy_value() {
        let all_legacy_task_kinds = [
            FightDriverTaskKind::Freeze,
            FightDriverTaskKind::Fireball,
            FightDriverTaskKind::Ball,
            FightDriverTaskKind::Flash,
            FightDriverTaskKind::Warcry,
            FightDriverTaskKind::Attack,
            FightDriverTaskKind::MoveRight,
            FightDriverTaskKind::MoveLeft,
            FightDriverTaskKind::MoveUp,
            FightDriverTaskKind::MoveDown,
            FightDriverTaskKind::Regenerate,
            FightDriverTaskKind::Distance3,
            FightDriverTaskKind::Distance7,
            FightDriverTaskKind::Bless,
            FightDriverTaskKind::EarthRain,
            FightDriverTaskKind::EarthMud,
            FightDriverTaskKind::Heal,
            FightDriverTaskKind::MagicShield,
            FightDriverTaskKind::Pulse,
            FightDriverTaskKind::AttackBack,
            FightDriverTaskKind::Flee,
            FightDriverTaskKind::FireRing,
        ];
        assert_eq!(all_legacy_task_kinds.len(), 22);

        let mut tasks = [
            FightDriverTask {
                kind: FightDriverTaskKind::Attack,
                value: FIGHT_DRIVER_LOW_PRIO + 20,
            },
            FightDriverTask {
                kind: FightDriverTaskKind::Fireball,
                value: FIGHT_DRIVER_MED_PRIO + 5,
            },
            FightDriverTask {
                kind: FightDriverTaskKind::Heal,
                value: FIGHT_DRIVER_HIGH_PRIO + 1,
            },
        ];

        order_fight_driver_tasks(&mut tasks, -10, |_| {
            unreachable!("no silliness at level -10")
        });

        assert_eq!(
            tasks.iter().map(|task| task.kind).collect::<Vec<_>>(),
            vec![
                FightDriverTaskKind::Heal,
                FightDriverTaskKind::Fireball,
                FightDriverTaskKind::Attack,
            ]
        );
    }

    #[test]
    fn fight_driver_task_order_adds_c_silliness_rolls_before_sorting() {
        let mut tasks = [
            FightDriverTask {
                kind: FightDriverTaskKind::Attack,
                value: 100,
            },
            FightDriverTask {
                kind: FightDriverTaskKind::Flash,
                value: 103,
            },
        ];
        let mut rolls = [4, 0].into_iter();

        order_fight_driver_tasks(&mut tasks, 0, |below| {
            assert_eq!(below, 5);
            rolls.next().unwrap()
        });

        assert_eq!(tasks[0].kind, FightDriverTaskKind::Attack);
        assert_eq!(tasks[0].value, 104);
        assert_eq!(tasks[1].kind, FightDriverTaskKind::Flash);
        assert_eq!(tasks[1].value, 103);
    }

    #[test]
    fn fight_driver_attackback_requires_attack_as_next_task_like_c() {
        let tasks = [
            FightDriverTask {
                kind: FightDriverTaskKind::AttackBack,
                value: FIGHT_DRIVER_HIGH_PRIO,
            },
            FightDriverTask {
                kind: FightDriverTaskKind::Fireball,
                value: FIGHT_DRIVER_MED_PRIO,
            },
            FightDriverTask {
                kind: FightDriverTaskKind::Attack,
                value: FIGHT_DRIVER_LOW_PRIO,
            },
        ];

        assert!(!fight_driver_attackback_may_run(&tasks, 0));
        assert!(!fight_driver_attackback_may_run(&tasks, 2));

        let tasks = [
            FightDriverTask {
                kind: FightDriverTaskKind::AttackBack,
                value: FIGHT_DRIVER_HIGH_PRIO,
            },
            FightDriverTask {
                kind: FightDriverTaskKind::Attack,
                value: FIGHT_DRIVER_MED_PRIO,
            },
        ];

        assert!(fight_driver_attackback_may_run(&tasks, 0));
    }

    #[test]
    fn simple_baddy_fight_tasks_skip_regeneration_in_area_33_like_c() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.hp = POWERSCALE;
        npc.mana = POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 10;
        npc.values[0][CharacterValue::Mana as usize] = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        let area_one_tasks = world.simple_baddy_fight_tasks(
            CharacterId(1),
            world.characters.get(&CharacterId(2)).unwrap(),
            1,
            false,
        );
        let area_thirty_three_tasks = world.simple_baddy_fight_tasks(
            CharacterId(1),
            world.characters.get(&CharacterId(2)).unwrap(),
            33,
            false,
        );

        assert!(area_one_tasks
            .iter()
            .any(|task| task.kind == FightDriverTaskKind::Regenerate));
        assert!(!area_thirty_three_tasks
            .iter()
            .any(|task| task.kind == FightDriverTaskKind::Regenerate));
    }

    #[test]
    fn simple_baddy_fight_tasks_honor_legacy_nomove_attack_gate() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 13, 10);

        let moving_tasks = world.simple_baddy_fight_tasks(
            CharacterId(1),
            world.characters.get(&CharacterId(2)).unwrap(),
            1,
            false,
        );
        let no_move_tasks = world.simple_baddy_fight_tasks(
            CharacterId(1),
            world.characters.get(&CharacterId(2)).unwrap(),
            1,
            true,
        );

        assert!(moving_tasks
            .iter()
            .any(|task| task.kind == FightDriverTaskKind::Attack));
        assert!(!no_move_tasks
            .iter()
            .any(|task| task.kind == FightDriverTaskKind::Attack));
    }

    #[test]
    fn simple_baddy_fight_tasks_allow_nomove_attack_at_distance_two_like_c() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        let tasks = world.simple_baddy_fight_tasks(
            CharacterId(1),
            world.characters.get(&CharacterId(2)).unwrap(),
            1,
            true,
        );

        assert!(tasks
            .iter()
            .any(|task| task.kind == FightDriverTaskKind::Attack));
    }

    #[test]
    fn simple_baddy_attack_action_self_heals_before_offense_when_badly_hurt() {
        let mut world = World::default();
        world.tick = Tick(450);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.regen_ticker = 450;
        npc.hp = 40 * POWERSCALE;
        npc.mana = 10 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 100;
        npc.values[0][CharacterValue::Mana as usize] = 10;
        npc.values[0][CharacterValue::Heal as usize] = 20;
        npc.values[0][CharacterValue::Fireball as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);
        world.map.tile_mut(15, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::HEAL_SELF);
        assert!(npc.mana < 10 * POWERSCALE);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 450);
    }

    #[test]
    fn simple_baddy_visible_attack_queues_legacy_start_combat_sound_after_delay() {
        let mut world = World::default();
        world.tick = Tick(TICKS_PER_SECOND * 11);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            lastfight: 0,
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 11,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.flags.insert(CharacterFlags::PLAYER);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].character_id, CharacterId(2));
        assert_eq!(sounds[0].special.special_type, 1);

        let mut world = World::default();
        world.tick = Tick(TICKS_PER_SECOND * 11);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            lastfight: (TICKS_PER_SECOND * 11 - 1) as i32,
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 11,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.flags.insert(CharacterFlags::PLAYER);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        assert!(world.drain_pending_sound_specials().is_empty());
    }

    #[test]
    fn simple_baddy_attack_action_restores_magicshield_before_melee() {
        let mut world = World::default();
        world.tick = Tick(451);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = 10 * POWERSCALE;
        npc.lifeshield = 0;
        npc.values[0][CharacterValue::MagicShield as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 11,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::MAGICSHIELD);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 451);
    }

    #[test]
    fn simple_baddy_attack_action_self_blesses_when_unblessed() {
        let mut world = World::default();
        world.tick = Tick(452);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = BLESS_COST;
        npc.values[0][CharacterValue::Bless as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 11,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::BLESS_SELF);
        assert_eq!(npc.mana, 0);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 452);
    }

    #[test]
    fn simple_baddy_attack_action_idles_to_regenerate_during_fight() {
        let mut world = World::default();
        world.tick = Tick(453);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.hp = 90 * POWERSCALE;
        npc.mana = 100 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 100;
        npc.values[0][CharacterValue::Mana as usize] = 100;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 11,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::IDLE);
        assert_eq!(npc.duration, (TICKS_PER_SECOND / 2) as i32);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 453);
    }

    #[test]
    fn simple_baddy_attack_action_earth_demon_casts_useful_earthmud() {
        let mut world = World::default();
        world.tick = Tick(454);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.flags.insert(CharacterFlags::EDEMON);
        npc.hp = 100 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 100;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.values[1][CharacterValue::Demon as usize] = 30;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.action = action::WALK;
        target.tox = 16;
        target.toy = 10;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);
        world.map.tile_mut(15, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::EARTHMUD);
        assert_eq!(npc.act1, 17 + 10 * MAX_MAP as i32);
        assert_eq!(npc.act2, 30);
        assert_eq!(npc.hp, 100 * POWERSCALE - 3000);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 454);
    }

    #[test]
    fn simple_baddy_attack_action_skips_earthmud_without_useful_tiles() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.flags.insert(CharacterFlags::EDEMON);
        npc.hp = 100 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 100;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.values[1][CharacterValue::Demon as usize] = 30;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);
        for (x, y) in [(15, 10), (16, 10), (14, 10), (15, 11), (15, 9)] {
            world.map.set_flags(x, y, MapFlags::SIGHTBLOCK);
        }

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_ne!(npc.action, action::EARTHMUD);
    }

    #[test]
    fn simple_baddy_fight_tasks_keep_c_commented_earthrain_disabled() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.flags.insert(CharacterFlags::EDEMON);
        npc.hp = 100 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 100;
        npc.values[1][CharacterValue::Demon as usize] = 30;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);
        world.map.tile_mut(15, 10).unwrap().light = 255;

        let tasks = world.simple_baddy_fight_tasks(
            CharacterId(1),
            world.characters.get(&CharacterId(2)).unwrap(),
            1,
            false,
        );

        assert!(!tasks
            .iter()
            .any(|task| task.kind == FightDriverTaskKind::EarthRain));
        assert!(tasks
            .iter()
            .any(|task| task.kind == FightDriverTaskKind::EarthMud));
    }

    #[test]
    fn simple_baddy_attack_action_uses_firering_against_adjacent_recorded_enemy() {
        let mut world = World::default();
        world.tick = Tick(455);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FIREBALL_COST;
        npc.values[0][CharacterValue::Fireball as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 11,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::FIRERING);
        assert_eq!(npc.mana, 0);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 455);
    }

    #[test]
    fn simple_baddy_attack_action_uses_fireball_against_visible_recorded_enemy() {
        let mut world = World::default();
        world.tick = Tick(455);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FIREBALL_COST;
        npc.values[0][CharacterValue::Fireball as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);
        world.map.tile_mut(15, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::FIREBALL1);
        assert_eq!(npc.act1, 15);
        assert_eq!(npc.act2, 10);
        assert_eq!(npc.dir, Direction::Right as u8);
        assert_eq!(npc.mana, 0);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 455);
    }

    #[test]
    fn simple_baddy_fireball_repositions_for_blocked_line_of_fire() {
        let mut world = World::default();
        world.tick = Tick(467);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FIREBALL_COST;
        npc.values[0][CharacterValue::Fireball as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 14,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 14, 10);
        world
            .map
            .tile_mut(12, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
        world.map.tile_mut(14, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!(npc.tox, 10);
        assert_eq!(npc.toy, 11);
        assert_eq!(npc.mana, FIREBALL_COST);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 467);
    }

    #[test]
    fn simple_baddy_fireball_does_not_cast_through_blocked_line_without_lane() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FIREBALL_COST;
        npc.values[0][CharacterValue::Fireball as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 14,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 14, 10);
        for (x, y) in [(12, 10), (10, 9), (10, 11), (11, 10), (9, 10)] {
            world
                .map
                .tile_mut(x, y)
                .unwrap()
                .flags
                .insert(MapFlags::MOVEBLOCK);
        }
        world.map.tile_mut(14, 10).unwrap().light = 255;

        let target = world.characters[&CharacterId(2)].clone();
        assert!(!world.setup_simple_baddy_fireball_attack(CharacterId(1), &target, 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, 0);
        assert_eq!(npc.mana, FIREBALL_COST);
    }

    #[test]
    fn simple_baddy_fireball_line_accepts_recorded_enemy_blast() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![
                SimpleBaddyEnemy {
                    target_id: CharacterId(2),
                    priority: 1,
                    last_seen_tick: 123,
                    visible: true,
                    last_x: 15,
                    last_y: 10,
                },
                SimpleBaddyEnemy {
                    target_id: CharacterId(3),
                    priority: 1,
                    last_seen_tick: 123,
                    visible: true,
                    last_x: 12,
                    last_y: 11,
                },
            ],
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);
        world.spawn_character(character(2), 15, 10);
        world.spawn_character(character(3), 12, 11);
        world
            .map
            .tile_mut(12, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);

        assert!(world.fireball_line_hits_target(CharacterId(1), CharacterId(2), 10, 10, 15, 10));
    }

    #[test]
    fn simple_baddy_fireball_line_rejects_friendly_blast() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);
        world.spawn_character(character(2), 15, 10);
        world.spawn_character(character(3), 12, 11);
        world
            .map
            .tile_mut(12, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);

        assert!(!world.fireball_line_hits_target(CharacterId(1), CharacterId(2), 10, 10, 15, 10));
    }

    #[test]
    fn simple_baddy_attack_action_uses_freeze_against_close_recorded_enemy() {
        let mut world = World::default();
        world.tick = Tick(458);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FREEZE_COST;
        npc.values[0][CharacterValue::Freeze as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 12,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 12, 10);
        world.map.tile_mut(12, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::FREEZE);
        assert_eq!(npc.mana, 0);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 458);
    }

    #[test]
    fn simple_baddy_attack_action_uses_flash_against_close_recorded_enemy() {
        let mut world = World::default();
        world.tick = Tick(459);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FLASH_COST;
        npc.values[0][CharacterValue::Flash as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 12,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 12, 10);
        world.map.tile_mut(12, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::FLASH);
        assert_eq!(npc.mana, 0);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 459);
    }

    #[test]
    fn simple_baddy_attack_action_applies_legacy_task_silliness_rolls() {
        let mut world = World::default();
        world.tick = Tick(459);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FLASH_COST;
        npc.values[0][CharacterValue::Attack as usize] = 100;
        npc.values[1][CharacterValue::Attack as usize] = 100;
        npc.values[0][CharacterValue::Flash as usize] = 26;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 11,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);
        world.map.tile_mut(11, 10).unwrap().light = 255;
        let mut rolls = [0, 4].into_iter();

        assert!(
            world.process_simple_baddy_attack_action_with_random(CharacterId(1), 1, |below| {
                assert_eq!(below, 5);
                rolls.next().unwrap_or(0)
            })
        );

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::ATTACK1);
        assert_eq!(npc.mana, FLASH_COST);
    }

    #[test]
    fn simple_baddy_attack_task_uses_c_attack_skill_with_weapon_skill() {
        let mut character = character(1);
        character.level = 20;
        character.values[0][CharacterValue::Attack as usize] = 30;
        character.values[1][CharacterValue::Attack as usize] = 30;
        character.values[0][CharacterValue::Tactics as usize] = 12;
        character.values[0][CharacterValue::Hand as usize] = 5;
        character.values[0][CharacterValue::Sword as usize] = 40;
        character.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(7));
        let weapon = item(7, ItemFlags::SWORD);
        let items = HashMap::from([(weapon.id, weapon)]);

        assert_eq!(simple_baddy_attack_skill(&character, &items), 104);
        assert_eq!(simple_baddy_attack_task_value(&character, &items), 539);
    }

    #[test]
    fn simple_baddy_attack_task_falls_back_to_hand_without_weapon() {
        let mut character = character(1);
        character.level = 20;
        character.values[0][CharacterValue::Hand as usize] = 9;
        character.values[0][CharacterValue::Bless as usize] = 8;
        character.values[0][CharacterValue::Heal as usize] = 8;
        character.values[0][CharacterValue::Freeze as usize] = 8;
        character.values[0][CharacterValue::MagicShield as usize] = 8;
        character.values[0][CharacterValue::Flash as usize] = 8;
        character.values[0][CharacterValue::Fireball as usize] = 8;
        character.values[0][CharacterValue::Pulse as usize] = 8;

        let items = HashMap::new();

        assert_eq!(simple_baddy_attack_skill(&character, &items), 3);
        assert_eq!(simple_baddy_attack_task_value(&character, &items), 2);
    }

    #[test]
    fn simple_baddy_attack_action_uses_warcry_when_close_and_unshielded() {
        let mut world = World::default();
        world.tick = Tick(460);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.endurance = 10 * POWERSCALE;
        npc.values[0][CharacterValue::Warcry as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 12,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 12, 10);
        world.map.tile_mut(12, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WARCRY);
        assert_eq!(npc.endurance, 10 * POWERSCALE - 20 * POWERSCALE / 3);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 460);
    }

    #[test]
    fn simple_baddy_warcry_task_does_not_precheck_modifier_like_c() {
        let mut world = World::default();
        world.tick = Tick(460);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.endurance = 10 * POWERSCALE;
        npc.lifeshield = 10 * POWERSCALE;
        npc.values[0][CharacterValue::Warcry as usize] = 2;
        npc.values[0][CharacterValue::MagicShield as usize] = 10;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.values[1][CharacterValue::MagicShield as usize] = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 12,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.values[0][CharacterValue::Immunity as usize] = 100;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 12, 10);
        world.map.tile_mut(12, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WARCRY);
        assert_eq!(npc.endurance, 10 * POWERSCALE - 2 * POWERSCALE / 3);
    }

    #[test]
    fn simple_baddy_attack_action_uses_ball_against_distant_recorded_enemy() {
        let mut world = World::default();
        world.tick = Tick(461);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FLASH_COST;
        npc.values[0][CharacterValue::Flash as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 16,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 16, 10);
        world.map.tile_mut(16, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::BALL1);
        assert_eq!(npc.act1, 16);
        assert_eq!(npc.act2, 10);
        assert_eq!(npc.mana, 0);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 461);
    }

    #[test]
    fn simple_baddy_ball_task_requires_unblocked_legacy_intercept_steps() {
        let mut world = World::default();
        world.tick = Tick(461);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FLASH_COST;
        npc.values[0][CharacterValue::Flash as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 16,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 16, 10);
        world.map.tile_mut(16, 10).unwrap().light = 255;
        world
            .map
            .tile_mut(12, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_ne!(npc.action, action::BALL1);
    }

    #[test]
    fn simple_baddy_ball_attack_uses_legacy_random_target_offset() {
        let mut world = World::default();
        world.tick = Tick(461);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FLASH_COST;
        npc.values[0][CharacterValue::Flash as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 16,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 16, 10);
        world.map.tile_mut(16, 10).unwrap().light = 255;
        let mut rolls = [0, 0, 0, 2].into_iter();

        assert!(
            world.process_simple_baddy_attack_action_with_random(CharacterId(1), 1, |_| {
                rolls.next().unwrap()
            })
        );

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::BALL1);
        assert_eq!(npc.act1, 15);
        assert_eq!(npc.act2, 11);
    }

    #[test]
    fn simple_baddy_attack_batch_threads_runtime_random() {
        let mut world = World::default();
        world.tick = Tick(461);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FLASH_COST;
        npc.values[0][CharacterValue::Flash as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 16,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 16, 10);
        world.map.tile_mut(16, 10).unwrap().light = 255;
        let mut rolls = [0, 0, 0, 2].into_iter();

        assert_eq!(
            world.process_simple_baddy_attack_actions_with_random(1, |_| rolls.next().unwrap()),
            1
        );

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::BALL1);
        assert_eq!(npc.act1, 15);
        assert_eq!(npc.act2, 11);
    }

    #[test]
    fn simple_baddy_attack_action_uses_pulse_when_nearby_enemy_is_finishable() {
        let mut world = World::default();
        world.tick = Tick(462);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = POWERSCALE + 1;
        npc.values[0][CharacterValue::Mana as usize] = 1;
        npc.values[0][CharacterValue::Pulse as usize] = 2_000;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 12,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.flags.insert(CharacterFlags::ALIVE);
        target.hp = POWERSCALE + 100;
        target.lifeshield = 0;
        target.values[0][CharacterValue::Hp as usize] = 100;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 12, 10);
        world.map.tile_mut(12, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::PULSE);
        assert_eq!(npc.mana, 1);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 462);
    }

    #[test]
    fn simple_baddy_attack_action_does_not_pulse_healthy_targets() {
        let mut world = World::default();
        world.tick = Tick(463);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = 100 * POWERSCALE;
        npc.values[0][CharacterValue::Mana as usize] = 100;
        npc.values[0][CharacterValue::Pulse as usize] = 200;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 11,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.hp = 100 * POWERSCALE;
        target.values[0][CharacterValue::Hp as usize] = 100;
        target.values[0][CharacterValue::Attack as usize] = 1;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::ATTACK1);
        assert_eq!(npc.mana, 100 * POWERSCALE);
    }

    #[test]
    fn simple_baddy_attack_action_idles_when_already_at_flash_spacing_distance() {
        let mut world = World::default();
        world.tick = Tick(464);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = 4 * POWERSCALE;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.values[1][CharacterValue::Flash as usize] = 20;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 13,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut active_flash = item(50, ItemFlags::empty());
        active_flash.driver = IDR_FLASH;
        world.items.insert(active_flash.id, active_flash);
        npc.inventory[12] = Some(ItemId(50));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 13, 10);
        world.map.tile_mut(13, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::IDLE);
        assert_eq!(npc.duration, (TICKS_PER_SECOND / 4) as i32);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 464);
    }

    #[test]
    fn simple_baddy_attack_action_does_not_distance_idle_without_active_flash_spell_slot() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = 4 * POWERSCALE;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.values[1][CharacterValue::Flash as usize] = 20;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 13,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 13, 10);
        world.map.tile_mut(13, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!(npc.tox, 11);
        assert_eq!(npc.toy, 10);
    }

    #[test]
    fn simple_baddy_fireball_spacing_moves_toward_distance_seven() {
        let mut world = World::default();
        world.tick = Tick(466);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FIREBALL_COST + 1;
        npc.values[0][CharacterValue::Fireball as usize] = 1;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.values[1][CharacterValue::Fireball as usize] = 20;
        npc.values[1][CharacterValue::Flash as usize] = 5;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 20, 10);

        let target = world.characters.get(&CharacterId(2)).cloned().unwrap();
        assert!(world.setup_simple_baddy_fireball_distance_attack(CharacterId(1), &target, 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!(npc.tox, 11);
        assert_eq!(npc.toy, 10);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 466);
    }

    #[test]
    fn simple_baddy_fireball_spacing_requires_fireball_above_flash() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FIREBALL_COST + 1;
        npc.values[0][CharacterValue::Fireball as usize] = 1;
        npc.values[1][CharacterValue::Fireball as usize] = 5;
        npc.values[1][CharacterValue::Flash as usize] = 5;
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 20, 10);

        let target = world.characters.get(&CharacterId(2)).cloned().unwrap();
        assert!(!world.setup_simple_baddy_fireball_distance_attack(CharacterId(1), &target, 1));
        assert_eq!(world.characters[&CharacterId(1)].action, 0);
    }

    #[test]
    fn simple_baddy_distance_driver_uses_best_partial_when_exact_spacing_blocked() {
        let mut world = World::default();
        world.tick = Tick(467);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = FIREBALL_COST + 1;
        npc.values[0][CharacterValue::Fireball as usize] = 1;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.values[1][CharacterValue::Fireball as usize] = 20;
        npc.values[1][CharacterValue::Flash as usize] = 5;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 20, 10);
        for y in 1..MAX_MAP - 1 {
            world.map.set_flags(13, y, MapFlags::MOVEBLOCK);
        }

        let target = world.characters.get(&CharacterId(2)).cloned().unwrap();
        assert!(world.setup_simple_baddy_fireball_distance_attack(CharacterId(1), &target, 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!(npc.tox, 11);
        assert_eq!(npc.toy, 10);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 467);
    }

    #[test]
    fn simple_baddy_attack_action_attacks_visible_adjacent_recorded_enemy() {
        let mut world = World::default();
        world.tick = Tick(456);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 11,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.values[0][CharacterValue::Attack as usize] = 1;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 11, 10);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::ATTACK1);
        assert_eq!(npc.act1, 2);
        assert_eq!(npc.dir, Direction::Right as u8);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 456);
    }

    #[test]
    fn simple_baddy_attack_action_attacks_moving_target_destination() {
        let mut world = World::default();
        world.tick = Tick(457);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 12,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.tox = 11;
        target.toy = 10;
        target.values[0][CharacterValue::Attack as usize] = 1;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 12, 10);
        world.map.tile_mut(12, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::ATTACK1);
        assert_eq!(npc.act1, 2);
        assert_eq!(npc.dir, Direction::Right as u8);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 457);
    }

    #[test]
    fn simple_baddy_attack_action_walks_toward_visible_non_adjacent_enemies() {
        let mut world = World::default();
        world.tick = Tick(458);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);
        world.map.tile_mut(15, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!(npc.tox, 11);
        assert_eq!(npc.toy, 10);
        assert_eq!(npc.dir, Direction::Right as u8);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 458);
    }

    #[test]
    fn simple_baddy_attack_action_prefers_c_visible_enemy_score_over_priority() {
        let mut world = World::default();
        world.tick = Tick(459);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![
                SimpleBaddyEnemy {
                    target_id: CharacterId(2),
                    priority: 99,
                    last_seen_tick: 999,
                    visible: true,
                    last_x: 14,
                    last_y: 10,
                },
                SimpleBaddyEnemy {
                    target_id: CharacterId(3),
                    priority: 0,
                    last_seen_tick: 1,
                    visible: true,
                    last_x: 11,
                    last_y: 10,
                },
            ],
            ..SimpleBaddyDriverData::default()
        }));
        let mut far_target = character(2);
        far_target.values[0][CharacterValue::Attack as usize] = 1;
        let mut close_target = character(3);
        close_target.values[0][CharacterValue::Attack as usize] = 1;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(far_target, 14, 10);
        world.spawn_character(close_target, 11, 10);
        world.map.tile_mut(14, 10).unwrap().light = 255;
        world.map.tile_mut(11, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::ATTACK1);
        assert_eq!(npc.act1, 3);
        assert_eq!(npc.dir, Direction::Right as u8);
    }

    #[test]
    fn simple_baddy_attack_action_prefers_hurt_visible_enemy_before_distance_like_c() {
        let mut world = World::default();
        world.tick = Tick(459);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![
                SimpleBaddyEnemy {
                    target_id: CharacterId(2),
                    priority: 1,
                    last_seen_tick: 999,
                    visible: true,
                    last_x: 14,
                    last_y: 10,
                },
                SimpleBaddyEnemy {
                    target_id: CharacterId(3),
                    priority: 0,
                    last_seen_tick: 1,
                    visible: true,
                    last_x: 10,
                    last_y: 11,
                },
            ],
            ..SimpleBaddyDriverData::default()
        }));
        let mut hurt_target = character(2);
        hurt_target.values[0][CharacterValue::Attack as usize] = 1;
        let mut seen_target = character(3);
        seen_target.values[0][CharacterValue::Attack as usize] = 1;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(hurt_target, 14, 10);
        world.spawn_character(seen_target, 10, 11);
        world.map.tile_mut(14, 10).unwrap().light = 255;
        world.map.tile_mut(10, 11).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!(npc.tox, 11);
        assert_eq!(npc.toy, 10);
        assert_eq!(npc.dir, Direction::Right as u8);
    }

    #[test]
    fn simple_baddy_attack_action_moves_to_target_back_when_front_is_occupied() {
        let mut world = World::default();
        world.tick = Tick(458);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 10,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.dir = Direction::Right as u8;
        let front_blocker = character(3);
        world.spawn_character(npc, 9, 9);
        world.spawn_character(target, 10, 10);
        world.spawn_character(front_blocker, 11, 10);
        world.map.tile_mut(10, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!(npc.tox, 9);
        assert_eq!(npc.toy, 10);
        assert_eq!(npc.dir, Direction::Down as u8);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 458);
    }

    #[test]
    fn simple_baddy_attack_action_skips_back_move_when_back_tile_is_blocked() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 10,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.dir = Direction::Right as u8;
        let front_blocker = character(3);
        world.spawn_character(npc, 9, 9);
        world.spawn_character(target, 10, 10);
        world.spawn_character(front_blocker, 11, 10);
        world
            .map
            .tile_mut(9, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
        world.map.tile_mut(10, 10).unwrap().light = 255;

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_ne!((npc.tox, npc.toy), (9, 10));
    }

    #[test]
    fn simple_baddy_attack_back_move_rejects_front_position_like_c() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.group = 7;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        let mut target = character(2);
        target.dir = Direction::Right as u8;
        world.spawn_character(npc, 11, 10);
        world.spawn_character(target.clone(), 10, 10);
        target.x = 10;
        target.y = 10;

        assert!(!world.setup_simple_baddy_attack_back_move(CharacterId(1), &target, 1));
    }

    #[test]
    fn simple_baddy_attack_back_move_rejects_same_group_side_occupant_like_c() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.group = 7;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        let mut target = character(2);
        target.dir = Direction::Right as u8;
        let front_blocker = character(3);
        let mut side_ally = character(4);
        side_ally.group = 7;
        world.spawn_character(npc, 9, 9);
        world.spawn_character(target.clone(), 10, 10);
        world.spawn_character(front_blocker, 11, 10);
        world.spawn_character(side_ally, 10, 11);
        target.x = 10;
        target.y = 10;

        assert!(!world.setup_simple_baddy_attack_back_move(CharacterId(1), &target, 1));
    }

    #[test]
    fn simple_baddy_flee_action_moves_away_from_visible_enemy() {
        let mut world = World::default();
        world.tick = Tick(459);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.endurance = 5 * POWERSCALE;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 13,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 13, 10);
        world.map.tile_mut(13, 10).unwrap().light = 255;

        assert!(world.setup_simple_baddy_flee_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert!(npc.tox < 10);
        assert_eq!(npc.dir, Direction::Left as u8);
        assert_eq!(npc.speed_mode, SpeedMode::Fast);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 459);
    }

    #[test]
    fn simple_baddy_flee_action_uses_stealth_when_enemy_is_distant() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 20,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 20, 10);
        world.map.tile_mut(20, 10).unwrap().light = 255;

        assert!(world.setup_simple_baddy_flee_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.speed_mode, SpeedMode::Stealth);
        assert!(npc.tox < 10);
    }

    #[test]
    fn simple_baddy_flee_action_scores_blocked_escape_path() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.endurance = 5 * POWERSCALE;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 13,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 13, 10);
        world.map.tile_mut(13, 10).unwrap().light = 255;
        world
            .map
            .tile_mut(8, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);

        assert!(world.setup_simple_baddy_flee_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert!(npc.tox < 10);
    }

    #[test]
    fn simple_baddy_attack_action_removes_visible_enemy_past_stop_distance() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 10;
        npc.rest_y = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            stopdist: 6,
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 14,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 14, 10);
        world.map.tile_mut(14, 10).unwrap().light = 255;

        assert!(!world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert!(data.enemies.is_empty());
    }

    #[test]
    fn simple_baddy_attack_action_uses_best_partial_path_when_target_unreachable() {
        let mut world = World::default();
        world.tick = Tick(460);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);
        world.map.tile_mut(15, 10).unwrap().light = 255;
        for y in 1..MAX_MAP - 1 {
            world
                .map
                .tile_mut(12, y)
                .unwrap()
                .flags
                .insert(MapFlags::MOVEBLOCK);
        }

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!((npc.tox, npc.toy), (11, 10));
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 460);
    }

    #[test]
    fn simple_baddy_attack_action_uses_adjacent_blocker_when_path_fails() {
        let mut world = World::default();
        world.tick = Tick(461);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 13,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        let mut blocker = item(10, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
        blocker.x = 11;
        blocker.y = 10;

        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 13, 10);
        world.map.tile_mut(13, 10).unwrap().light = 255;
        world.items.insert(blocker.id, blocker);
        let tile = world.map.tile_mut(11, 10).unwrap();
        tile.item = 10;
        tile.flags.insert(MapFlags::TMOVEBLOCK);
        for y in 1..MAX_MAP - 1 {
            world
                .map
                .tile_mut(12, y)
                .unwrap()
                .flags
                .insert(MapFlags::MOVEBLOCK);
        }

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::USE);
        assert_eq!(npc.dir, Direction::Right as u8);
        assert_eq!(npc.act1, 10);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.lastfight, 461);
    }

    #[test]
    fn simple_baddy_attack_action_idles_when_unreachable_path_does_not_improve() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);
        world.map.tile_mut(15, 10).unwrap().light = 255;
        for y in 1..MAX_MAP - 1 {
            world
                .map
                .tile_mut(11, y)
                .unwrap()
                .flags
                .insert(MapFlags::MOVEBLOCK);
        }

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::IDLE);
        assert_eq!(npc.duration, (TICKS_PER_SECOND / 4) as i32);
    }

    #[test]
    fn distance_driver_prefers_moving_target_position_like_c() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.values[0][CharacterValue::Speed as usize] = 50;
        let mut target = character(2);
        target.tox = 10;
        target.toy = 14;
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 16, 10);
        world.map.tile_mut(16, 10).unwrap().light = 255;

        assert!(world.distance_driver(CharacterId(1), CharacterId(2), 1, 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!((npc.tox, npc.toy), (10, 11));
        assert_eq!(npc.dir, Direction::Down as u8);
    }

    #[test]
    fn distance_driver_returns_false_when_already_at_requested_distance() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.values[0][CharacterValue::Speed as usize] = 50;
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 18, 10);
        world.map.tile_mut(18, 10).unwrap().light = 255;

        assert!(!world.distance_driver(CharacterId(1), CharacterId(2), 8, 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, 0);
    }

    #[test]
    fn distance_driver_uses_best_partial_path_when_exact_distance_unreachable() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.values[0][CharacterValue::Speed as usize] = 50;
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);
        world.map.tile_mut(15, 10).unwrap().light = 255;
        for y in 1..MAX_MAP - 1 {
            world
                .map
                .tile_mut(12, y)
                .unwrap()
                .flags
                .insert(MapFlags::MOVEBLOCK);
        }

        assert!(world.distance_driver(CharacterId(1), CharacterId(2), 1, 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!((npc.tox, npc.toy), (11, 10));
    }

    #[test]
    fn simple_baddy_attack_action_uses_explicit_fight_driver_home_for_stop_distance() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 10;
        npc.rest_y = 10;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            stopdist: 6,
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 14,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let target = character(2);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 14, 10);
        world.map.tile_mut(14, 10).unwrap().light = 255;
        assert!(world.set_simple_baddy_home(CharacterId(1), 14, 10));

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.home_x, 14);
        assert_eq!(data.home_y, 10);
        assert_eq!(data.enemies.len(), 1);
    }

    #[test]
    fn simple_baddy_attack_action_follows_invisible_enemy_last_position() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.values[0][CharacterValue::Attack as usize] = 20;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: false,
                last_x: 15,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.flags.insert(CharacterFlags::INVISIBLE);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);

        assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!(npc.tox, 11);
        assert_eq!(npc.toy, 10);
        assert_eq!(npc.dir, Direction::Right as u8);
    }

    #[test]
    fn simple_baddy_attack_action_drops_invisible_enemy_at_last_position() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: false,
                last_x: 10,
                last_y: 10,
            }],
            ..SimpleBaddyDriverData::default()
        }));
        let mut target = character(2);
        target.flags.insert(CharacterFlags::INVISIBLE);
        world.spawn_character(npc, 10, 10);
        world.spawn_character(target, 15, 10);

        assert!(!world.process_simple_baddy_attack_action(CharacterId(1), 1));

        let Some(CharacterDriverState::SimpleBaddy(data)) =
            world.characters[&CharacterId(1)].driver_state.as_ref()
        else {
            panic!("simple baddy state missing");
        };
        assert!(data.enemies.is_empty());
    }

    #[test]
    fn simple_baddy_noncombat_action_idles_shortly_after_creation() {
        let mut world = World::default();
        world.tick = Tick(3);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            creation_time: 0,
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::IDLE);
        assert_eq!(npc.duration, (TICKS_PER_SECOND / 4) as i32);
    }

    #[test]
    fn simple_baddy_noncombat_action_teleports_to_night_post_and_sets_home() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        world.date.hour = 21;
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            dayx: 20,
            dayy: 10,
            nightx: 15,
            nighty: 10,
            teleport: 1,
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((npc.x, npc.y), (15, 10));
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!((data.home_x, data.home_y), (15, 10));
    }

    #[test]
    fn simple_baddy_noncombat_action_turns_to_day_post_direction() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        world.date.hour = 12;
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.dir = Direction::Left as u8;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            dayx: 10,
            dayy: 10,
            daydir: Direction::Down as i32,
            nightx: 15,
            nighty: 10,
            nightdir: Direction::Up as i32,
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.dir, Direction::Down as u8);
        assert_eq!(npc.action, action::IDLE);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!((data.home_x, data.home_y), (10, 10));
    }

    #[test]
    fn simple_baddy_noncombat_action_walks_back_to_rest_home() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 15;
        npc.rest_y = 10;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!((npc.tox, npc.toy), (11, 10));
        assert_eq!(npc.dir, Direction::Right as u8);
    }

    #[test]
    fn simple_baddy_notsecure_day_post_walks_to_rest_home_like_c() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        world.date.hour = 12;
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 15;
        npc.rest_y = 10;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            dayx: 30,
            dayy: 10,
            nightx: 35,
            nighty: 10,
            notsecure: 1,
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!((npc.tox, npc.toy), (11, 10));
        assert_eq!(npc.dir, Direction::Right as u8);
    }

    #[test]
    fn secure_move_driver_turns_at_target_without_claiming_action() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.dir = Direction::Left as u8;
        world.spawn_character(npc, 10, 10);

        assert!(!world.secure_move_driver(CharacterId(1), 10, 10, Direction::Down as u8, 0, 0, 1,));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.dir, Direction::Down as u8);
        assert_eq!(npc.action, 0);
    }

    #[test]
    fn secure_move_driver_skips_move_after_blocked_use_and_teleports() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.values[0][CharacterValue::Speed as usize] = 50;
        world.spawn_character(npc, 10, 10);
        world
            .map
            .tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);

        assert!(world.secure_move_driver(
            CharacterId(1),
            12,
            10,
            Direction::Right as u8,
            2,
            action::USE,
            1,
        ));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((npc.x, npc.y), (12, 10));
        assert_eq!(npc.action, 0);
    }

    #[test]
    fn simple_baddy_noncombat_threads_failed_use_into_secure_move() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 20) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 12;
        npc.rest_y = 10;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        world.spawn_character(npc, 10, 10);

        let completions = [WorldActionCompletion {
            character_id: CharacterId(1),
            action_id: action::USE,
            action_item_id: None,
            ok: false,
            legacy_return_code: 2,
            item_use: None,
            old_x: 10,
            old_y: 10,
            new_x: 10,
            new_y: 10,
        }];

        assert_eq!(
            world.process_simple_baddy_noncombat_actions_with_completions(1, &completions),
            1
        );

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((npc.x, npc.y), (12, 10));
        assert_eq!(npc.action, 0);
    }

    #[test]
    fn simple_baddy_noncombat_failed_use_without_retry_code_still_walks() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 20) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 12;
        npc.rest_y = 10;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        world.spawn_character(npc, 10, 10);

        let completions = [WorldActionCompletion {
            character_id: CharacterId(1),
            action_id: action::USE,
            action_item_id: None,
            ok: false,
            legacy_return_code: 0,
            item_use: None,
            old_x: 10,
            old_y: 10,
            new_x: 10,
            new_y: 10,
        }];

        assert_eq!(
            world.process_simple_baddy_noncombat_actions_with_completions(1, &completions),
            1
        );

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!((npc.tox, npc.toy), (11, 10));
    }

    #[test]
    fn secure_move_driver_walks_before_teleport_when_not_blocked_use() {
        let mut world = World::default();
        let mut npc = character(1);
        npc.values[0][CharacterValue::Speed as usize] = 50;
        world.spawn_character(npc, 10, 10);

        assert!(world.secure_move_driver(CharacterId(1), 12, 10, Direction::Right as u8, 0, 0, 1,));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!((npc.tox, npc.toy), (11, 10));
    }

    #[test]
    fn simple_baddy_scavenger_idles_on_legacy_random_gate() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 10;
        npc.rest_y = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            scavenger: 4,
            dir: 0,
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| 0));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::IDLE);
        assert_eq!(npc.duration, TICKS_PER_SECOND as i32);
    }

    #[test]
    fn simple_baddy_scavenger_randomly_walks_inside_home_bounds() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 10;
        npc.rest_y = 10;
        npc.values[0][CharacterValue::Speed as usize] = 50;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            scavenger: 4,
            dir: 0,
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);
        let mut rolls = [1, 0].into_iter();

        assert!(
            world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| {
                rolls.next().unwrap_or(0)
            })
        );

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::WALK);
        assert_eq!((npc.tox, npc.toy), (11, 10));
        assert_eq!(npc.dir, Direction::Right as u8);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.dir, Direction::Right as i32);
        assert_eq!((data.home_x, data.home_y), (10, 10));
    }

    #[test]
    fn simple_baddy_scavenger_regenerates_before_random_wander() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 10;
        npc.rest_y = 10;
        npc.hp = 9 * POWERSCALE;
        npc.mana = 10 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 10;
        npc.values[0][CharacterValue::Mana as usize] = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            scavenger: 4,
            dir: 0,
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);

        assert!(
            world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| {
                panic!("regenerate_driver should run before RANDOM wander gates")
            })
        );

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::IDLE);
        assert_eq!(npc.duration, TICKS_PER_SECOND as i32);
    }

    #[test]
    fn simple_baddy_scavenger_regenerates_before_drinkspecial_poison() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 10;
        npc.rest_y = 10;
        npc.hp = 9 * POWERSCALE;
        npc.mana = 10 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 10;
        npc.values[0][CharacterValue::Mana as usize] = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            scavenger: 4,
            drinkspecial: 1,
            ..SimpleBaddyDriverData::default()
        }));
        let mut poison0 = item(10, ItemFlags::empty());
        poison0.driver = IDR_POISON0;
        npc.inventory[SPELL_SLOT_START] = Some(poison0.id);
        world.items.insert(poison0.id, poison0);
        world.spawn_character(npc, 10, 10);

        assert!(
            world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| {
                panic!("regenerate_driver should run before drinkspecial and RANDOM wander gates")
            })
        );

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::IDLE);
        assert_eq!(npc.inventory[SPELL_SLOT_START], Some(ItemId(10)));
        assert!(world.items.contains_key(&ItemId(10)));
    }

    #[test]
    fn simple_baddy_noncombat_self_blesses_before_idle() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = 10 * POWERSCALE;
        npc.values[0][CharacterValue::Bless as usize] = 20;
        npc.values[0][CharacterValue::MagicShield as usize] = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::BLESS_SELF);
        assert_eq!(npc.act1, 1);
        assert_eq!(npc.mana, 8 * POWERSCALE);
    }

    #[test]
    fn simple_baddy_noncombat_self_magicshields_when_bless_unavailable() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = 10 * POWERSCALE;
        npc.values[0][CharacterValue::MagicShield as usize] = 8;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::MAGICSHIELD);
        assert_eq!(npc.act1, 8 * POWERSCALE);
        assert_eq!(npc.mana, 6 * POWERSCALE);
    }

    #[test]
    fn simple_baddy_noncombat_regenerates_before_self_spells() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.mana = 10 * POWERSCALE;
        npc.hp = 9 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 10;
        npc.values[0][CharacterValue::Bless as usize] = 20;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::IDLE);
        assert_eq!(npc.duration, TICKS_PER_SECOND as i32);
        assert_eq!(npc.mana, 10 * POWERSCALE);
    }

    #[test]
    fn simple_baddy_drinkspecial_removes_poison_when_poison0_is_active() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.hp = 10 * POWERSCALE;
        npc.mana = 10 * POWERSCALE;
        npc.values[0][CharacterValue::Hp as usize] = 10;
        npc.values[0][CharacterValue::Mana as usize] = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            drinkspecial: 1,
            ..SimpleBaddyDriverData::default()
        }));
        let mut poison0 = item(10, ItemFlags::empty());
        poison0.driver = IDR_POISON0;
        let mut poison1 = item(11, ItemFlags::empty());
        poison1.driver = IDR_POISON1;
        npc.inventory[SPELL_SLOT_START] = Some(poison0.id);
        npc.inventory[SPELL_SLOT_START + 1] = Some(poison1.id);
        world.items.insert(poison0.id, poison0);
        world.items.insert(poison1.id, poison1);
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert!(npc.inventory[SPELL_SLOT_START].is_none());
        assert!(npc.inventory[SPELL_SLOT_START + 1].is_none());
        assert!(!world.items.contains_key(&ItemId(10)));
        assert!(!world.items.contains_key(&ItemId(11)));
        assert!(npc
            .flags
            .contains(CharacterFlags::ITEMS | CharacterFlags::UPDATE));
        assert_eq!(npc.action, action::IDLE);
    }

    #[test]
    fn simple_baddy_drinkspecial_requires_poison0_trigger() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            drinkspecial: 1,
            ..SimpleBaddyDriverData::default()
        }));
        let mut poison1 = item(11, ItemFlags::empty());
        poison1.driver = IDR_POISON1;
        npc.inventory[SPELL_SLOT_START] = Some(poison1.id);
        world.items.insert(poison1.id, poison1);
        world.spawn_character(npc, 10, 10);

        assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.inventory[SPELL_SLOT_START], Some(ItemId(11)));
        assert!(world.items.contains_key(&ItemId(11)));
    }

    #[test]
    fn simple_baddy_scavenger_clears_direction_when_walk_fails() {
        let mut world = World::default();
        world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
        let mut npc = character(1);
        npc.driver = CDR_SIMPLEBADDY;
        npc.rest_x = 10;
        npc.rest_y = 10;
        npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            scavenger: 4,
            dir: Direction::Right as i32,
            ..SimpleBaddyDriverData::default()
        }));
        world.spawn_character(npc, 10, 10);
        world
            .map
            .tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);

        assert!(world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| 1));

        let npc = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(npc.action, action::IDLE);
        let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.dir, 0);
    }

    #[test]
    fn simple_baddy_death_driver_creates_earth_demon_effects_at_killer() {
        let mut world = World::default();
        let mut dead = character(1);
        dead.driver = CDR_SIMPLEBADDY;
        dead.flags.insert(CharacterFlags::EDEMON);
        dead.flags.insert(CharacterFlags::GOD);
        dead.values[1][CharacterValue::Demon as usize] = 6;
        let killer = character(2);
        assert!(world.spawn_character(dead, 10, 10));
        assert!(world.spawn_character(killer, 12, 10));
        world.map.tile_mut(12, 10).unwrap().light = 255;

        let effect_ids = world.apply_simple_baddy_death_driver(CharacterId(1), CharacterId(2));

        assert_eq!(effect_ids.len(), 2);
        let mud = world.effects.get(&effect_ids[0]).unwrap();
        assert_eq!(mud.effect_type, EF_EARTHMUD);
        assert_eq!(mud.strength, 6);
        let rain = world.effects.get(&effect_ids[1]).unwrap();
        assert_eq!(rain.effect_type, EF_EARTHRAIN);
        assert_eq!(rain.strength, 6);
        let killer_tile = world.map.tile(12, 10).unwrap();
        assert!(killer_tile.effects.contains(&(effect_ids[0] as u16)));
        assert!(killer_tile.effects.contains(&(effect_ids[1] as u16)));
    }

    #[test]
    fn simple_baddy_death_driver_respects_earth_demon_gates() {
        let mut world = World::default();
        let mut dead = character(1);
        dead.driver = CDR_SIMPLEBADDY;
        dead.flags.insert(CharacterFlags::EDEMON);
        dead.flags.insert(CharacterFlags::GOD);
        dead.values[1][CharacterValue::Demon as usize] = 5;
        let killer = character(2);
        assert!(world.spawn_character(dead, 10, 10));
        assert!(world.spawn_character(killer, 12, 10));
        world.map.tile_mut(12, 10).unwrap().light = 255;

        let effect_ids = world.apply_simple_baddy_death_driver(CharacterId(1), CharacterId(2));

        assert_eq!(effect_ids.len(), 1);
        assert_eq!(world.effects[&effect_ids[0]].effect_type, EF_EARTHRAIN);

        world
            .map
            .tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::SIGHTBLOCK);
        let effect_ids = world.apply_simple_baddy_death_driver(CharacterId(1), CharacterId(2));

        assert!(effect_ids.is_empty());
    }

    #[test]
    fn legacy_hurt_invokes_simple_baddy_death_driver_for_earth_demons() {
        let mut world = World::default();
        let mut dead = character(1);
        dead.driver = CDR_SIMPLEBADDY;
        dead.flags.insert(CharacterFlags::EDEMON);
        dead.flags.insert(CharacterFlags::GOD);
        dead.values[1][CharacterValue::Demon as usize] = 6;
        dead.hp = POWERSCALE;
        let killer = character(2);
        assert!(world.spawn_character(dead, 10, 10));
        assert!(world.spawn_character(killer, 12, 10));
        world.map.tile_mut(12, 10).unwrap().light = 255;

        let outcome = world
            .apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), POWERSCALE, 1, 0, 0)
            .unwrap();

        assert!(outcome.killed);
        let dead = world.characters.get(&CharacterId(1)).unwrap();
        assert!(dead.flags.contains(CharacterFlags::DEAD));
        assert!(world
            .effects
            .values()
            .any(|effect| effect.effect_type == EF_EARTHRAIN && effect.strength == 6));
        assert!(world
            .effects
            .values()
            .any(|effect| effect.effect_type == EF_EARTHMUD && effect.strength == 6));
    }

    #[test]
    fn attack_driver_direct_attacks_adjacent_target() {
        let mut world = World::default();
        let attacker = character(1);
        let target = character(2);
        assert!(world.spawn_character(attacker, 10, 10));
        assert!(world.spawn_character(target, 11, 10));

        assert!(world.attack_driver_direct(CharacterId(1), CharacterId(2), 1));

        let attacker = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(attacker.action, action::ATTACK1);
        assert_eq!(attacker.dir, Direction::Right as u8);
        assert_eq!(attacker.act1, 2);
    }

    #[test]
    fn attack_driver_direct_attacks_moving_target_tile() {
        let mut world = World::default();
        let attacker = character(1);
        let mut target = character(2);
        target.tox = 11;
        target.toy = 10;
        assert!(world.spawn_character(attacker, 10, 10));
        assert!(world.spawn_character(target, 12, 10));
        world.map.tile_mut(12, 10).unwrap().light = 255;

        assert!(world.attack_driver_direct(CharacterId(1), CharacterId(2), 1));

        let attacker = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(attacker.action, action::ATTACK1);
        assert_eq!(attacker.dir, Direction::Right as u8);
        assert_eq!(attacker.act1, 2);
    }

    #[test]
    fn attack_driver_direct_walks_one_step_on_complete_path() {
        let mut world = World::default();
        let attacker = character(1);
        let target = character(2);
        assert!(world.spawn_character(attacker, 10, 10));
        assert!(world.spawn_character(target, 13, 10));
        world.map.tile_mut(13, 10).unwrap().light = 255;

        assert!(world.attack_driver_direct(CharacterId(1), CharacterId(2), 1));

        let attacker = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(attacker.action, action::WALK);
        assert_eq!(attacker.dir, Direction::Right as u8);
        assert_eq!((attacker.tox, attacker.toy), (11, 10));
    }

    #[test]
    fn attack_driver_direct_does_not_idle_or_best_partial_when_no_path_exists() {
        let mut world = World::default();
        let attacker = character(1);
        let target = character(2);
        assert!(world.spawn_character(attacker, 10, 10));
        assert!(world.spawn_character(target, 13, 10));
        world.map.tile_mut(13, 10).unwrap().light = 255;
        for (x, y) in [(11, 10), (9, 10), (10, 11), (10, 9)] {
            world
                .map
                .tile_mut(x, y)
                .unwrap()
                .flags
                .insert(MapFlags::MOVEBLOCK);
        }

        assert!(!world.attack_driver_direct(CharacterId(1), CharacterId(2), 1));

        let attacker = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(attacker.action, 0);
        assert_eq!((attacker.tox, attacker.toy), (0, 0));
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
    fn queued_sound_area_drains_legacy_player_special_targets() {
        let mut world = World {
            map: MapGrid::new(24, 24),
            ..World::default()
        };
        let mut listener = character(1);
        listener.flags.insert(CharacterFlags::PLAYER);
        listener.x = 12;
        listener.y = 10;
        world.add_character(listener);

        world.queue_sound_area(10, 10, 5);

        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].character_id, CharacterId(1));
        assert_eq!(sounds[0].special.special_type, 5);
        assert_eq!(sounds[0].special.opt1, -40);
        assert_eq!(sounds[0].special.opt2, 200);
        assert!(world.drain_pending_sound_specials().is_empty());
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
    fn world_schedules_existing_onofflight_and_preserves_first_timer_state() {
        let mut world = World::default();
        let mut light = item(7, ItemFlags::USED | ItemFlags::USE);
        light.driver = IDR_ONOFFLIGHT;
        light.driver_data = vec![1, 14];
        light.modifier_index[0] = CharacterValue::Light as i16;
        light.modifier_value[0] = 14;
        light.sprite = 101;
        light.x = 10;
        light.y = 10;
        world.map.tile_mut(10, 10).unwrap().item = 7;
        world.add_item(light);
        assert_eq!(world.map.tile(10, 10).unwrap().light, 14);

        assert_eq!(world.schedule_existing_light_timers(), 1);
        world.advance();
        let outcomes = world.process_due_timers(3);

        assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
        let light = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(light.driver_data[6], 1);
        assert_eq!(light.driver_data[0], 1);
        assert_eq!(light.modifier_value[0], 14);
        assert_eq!(light.sprite, 101);
        assert_eq!(world.map.tile(10, 10).unwrap().light, 14);
    }

    #[test]
    fn world_tracks_area3_onofflight_counts_and_gate_window() {
        let mut world = World::default();
        world.tick = Tick(100);
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        let mut light = item(7, ItemFlags::USED | ItemFlags::USE);
        light.driver = IDR_ONOFFLIGHT;
        light.driver_data = vec![1, 14, 0, 0, 0, 0, 1];
        light.modifier_index[0] = CharacterValue::Light as i16;
        light.modifier_value[0] = 14;
        light.sprite = 101;
        light.x = 10;
        light.y = 10;
        world.map.tile_mut(10, 10).unwrap().item = 7;
        world.add_character(character);
        world.add_item(light);

        let request = ItemDriverRequest::Driver {
            driver: IDR_ONOFFLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        let off = world.execute_item_driver_request(request, 3);
        assert_eq!(
            off,
            ItemDriverOutcome::OnOffLightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                now_on: false,
                remaining_off: None,
                gates_opened: false,
            }
        );
        assert_eq!(world.area3_palace_lamps.switched_off_count, 1);
        assert_eq!(world.map.tile(10, 10).unwrap().light, 0);

        let on = world.execute_item_driver_request(request, 3);
        assert_eq!(
            on,
            ItemDriverOutcome::OnOffLightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                now_on: true,
                remaining_off: Some(0),
                gates_opened: true,
            }
        );
        assert_eq!(world.area3_palace_lamps.switched_on_count, 1);
        assert_eq!(
            world.area3_palace_lamps.keep_open_until_tick,
            100 + TICKS_PER_SECOND as u64 * 60 * 3
        );
        assert_eq!(world.timers.used_timers(), 1);
        assert_eq!(world.map.tile(10, 10).unwrap().light, 14);
    }

    #[test]
    fn world_schedules_registered_area3_lamps_for_extinguish_when_gates_open() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        world.add_character(character);

        for id in [7, 9] {
            let mut light = item(id, ItemFlags::USED | ItemFlags::USE);
            light.driver = IDR_ONOFFLIGHT;
            light.driver_data = vec![0, 10, 0, 0, 0, 0, 1];
            light.x = 10 + id as u16;
            light.y = 10;
            world.add_item(light);
        }
        world.area3_palace_lamps.switched_off_count = 1;

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_ONOFFLIGHT,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            3,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::OnOffLightChanged {
                now_on: true,
                gates_opened: true,
                ..
            }
        ));
        assert_eq!(world.timers.used_timers(), 2);
    }

    #[test]
    fn world_area3_palace_gate_opens_and_closes_from_keepopen_window() {
        let mut world = World::default();
        world.tick = Tick(100);
        world.area3_palace_lamps.keep_open_until_tick = 200;
        let mut gate = item(
            7,
            ItemFlags::USED | ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK | ItemFlags::DOOR,
        );
        gate.driver = IDR_PALACEGATE;
        gate.driver_data = vec![0];
        gate.sprite = 500;
        gate.x = 10;
        gate.y = 10;
        world.map.tile_mut(10, 10).unwrap().item = 7;
        world.map.tile_mut(10, 10).unwrap().flags =
            MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR;
        world.add_item(gate);

        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
        world.advance();
        let open_outcome = world.process_due_timers(3).remove(0);

        assert_eq!(
            open_outcome,
            ItemDriverOutcome::PalaceGateTick {
                item_id: ItemId(7),
                opened: true,
                closed: false,
                blocked: false,
            }
        );
        let gate = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(gate.driver_data[0], 1);
        assert_eq!(gate.sprite, 501);
        assert!(!gate
            .flags
            .intersects(ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK | ItemFlags::DOOR));
        assert!(!world
            .map
            .tile(10, 10)
            .unwrap()
            .flags
            .intersects(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR));

        world.tick = Tick(250);
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
        world.advance();
        let close_outcome = world.process_due_timers(3).remove(0);

        assert_eq!(
            close_outcome,
            ItemDriverOutcome::PalaceGateTick {
                item_id: ItemId(7),
                opened: false,
                closed: true,
                blocked: false,
            }
        );
        let gate = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(gate.driver_data[0], 0);
        assert_eq!(gate.sprite, 500);
        assert!(gate.flags.contains(ItemFlags::MOVEBLOCK));
        assert!(world
            .map
            .tile(10, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));
    }

    #[test]
    fn world_area3_palace_gate_refuses_to_close_when_blocked() {
        let mut world = World::default();
        world.tick = Tick(250);
        let mut gate = item(7, ItemFlags::USED);
        gate.driver = IDR_PALACEGATE;
        gate.driver_data = vec![1];
        gate.driver_data.resize(40, 0);
        gate.driver_data[30..38].copy_from_slice(&ItemFlags::MOVEBLOCK.bits().to_le_bytes());
        gate.sprite = 501;
        gate.x = 10;
        gate.y = 10;
        world.map.tile_mut(10, 10).unwrap().item = 7;
        world.map.tile_mut(10, 10).unwrap().flags = MapFlags::MOVEBLOCK;
        world.add_item(gate);

        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
        world.advance();
        let outcome = world.process_due_timers(3).remove(0);

        assert_eq!(
            outcome,
            ItemDriverOutcome::PalaceGateTick {
                item_id: ItemId(7),
                opened: false,
                closed: false,
                blocked: true,
            }
        );
        let gate = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(gate.driver_data[0], 1);
        assert_eq!(gate.sprite, 501);
        assert_eq!(world.timers.used_timers(), 1);
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

        assert_ne!(
            world
                .effects
                .get(&effect_id)
                .map(|effect| effect.effect_type),
            Some(EF_FIREBALL)
        );
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
        let mut edemon_light = item(10, ItemFlags::USED);
        edemon_light.driver = IDR_EDEMONLIGHT;
        let mut edemon_tube = item(14, ItemFlags::USED);
        edemon_tube.driver = IDR_EDEMONTUBE;
        let mut edemon_loader = item(13, ItemFlags::USED);
        edemon_loader.driver = IDR_EDEMONLOADER;
        let mut fdemon_loader = item(11, ItemFlags::USED);
        fdemon_loader.driver = IDR_FDEMONLOADER;
        let mut fdemon_farm = item(12, ItemFlags::USED);
        fdemon_farm.driver = IDR_FDEMONFARM;
        world.add_item(nightlight);
        world.add_item(burning_torch);
        world.add_item(unlit_torch);
        world.add_item(edemon_light);
        world.add_item(edemon_tube);
        world.add_item(edemon_loader);
        world.add_item(fdemon_loader);
        world.add_item(fdemon_farm);

        assert_eq!(world.schedule_existing_light_timers(), 7);
        assert_eq!(world.timers.used_timers(), 7);
    }

    #[test]
    fn world_edemon_tube_discovers_loader_target_on_timer() {
        let mut world = World::default();
        world.add_character(character(0));
        let mut tube = item(7, ItemFlags::USED | ItemFlags::USE);
        tube.driver = IDR_EDEMONTUBE;
        tube.driver_data = vec![4, 0, 0, 0, 0, 0];
        world.add_item(tube);
        let mut loader = item(8, ItemFlags::USED | ItemFlags::USE);
        loader.driver = IDR_EDEMONLOADER;
        loader.driver_data = vec![4, 42, 0];
        loader.x = 20;
        loader.y = 20;
        world.add_item(loader);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_EDEMONTUBE,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            6,
        );

        assert!(matches!(outcome, ItemDriverOutcome::LightChanged { .. }));
        let tube = &world.items[&ItemId(7)];
        assert_eq!(tube.sprite, 14138);
        assert_eq!(tube.modifier_value[0], 200);
        assert_eq!(
            u16::from_le_bytes([tube.driver_data[2], tube.driver_data[3]]),
            20
        );
        assert_eq!(
            u16::from_le_bytes([tube.driver_data[4], tube.driver_data[5]]),
            21
        );
    }

    #[test]
    fn world_edemon_tube_overload_teleports_visible_nearby_players() {
        let mut world = World::default();
        world.add_character(character(0));
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        assert!(world.spawn_character(player, 12, 10));

        let mut tube = item(7, ItemFlags::USED | ItemFlags::USE);
        tube.driver = IDR_EDEMONTUBE;
        tube.driver_data = vec![4, 0, 0, 0, 0, 0];
        assert!(world.map.set_item_map(&mut tube, 10, 10));
        world.add_item(tube);

        let mut loader = item(8, ItemFlags::USED | ItemFlags::USE);
        loader.driver = IDR_EDEMONLOADER;
        loader.driver_data = vec![4, 251, 0];
        loader.x = 20;
        loader.y = 20;
        world.add_item(loader);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_EDEMONTUBE,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            6,
        );

        assert!(matches!(outcome, ItemDriverOutcome::EdemonTubePulse { .. }));
        let player = &world.characters[&CharacterId(1)];
        assert_eq!((player.x, player.y), (20, 21));
        assert_eq!(
            world.drain_pending_system_texts(),
            vec![WorldSystemText {
                character_id: CharacterId(1),
                message: "The strange tube teleported you.".to_string(),
            }]
        );
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn world_edemon_gate_timer_finds_stale_slot_and_reschedules() {
        let mut world = World::default();
        let mut gate = item(7, ItemFlags::USED);
        gate.driver = IDR_EDEMONGATE;
        gate.driver_data = vec![0];
        world.add_item(gate);
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
        world.advance();

        let outcomes = world.process_due_timers(6);

        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            outcomes[0],
            ItemDriverOutcome::EdemonGateSpawn {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                template: "edemon2s",
                slot: 0,
                x: 62,
                y: 157,
                schedule_after_ticks: TICKS_PER_SECOND * 10,
            }
        );
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn world_applies_fdemon_loader_cursor_ground_sound_and_timer() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.cursor_item = Some(ItemId(9));
        assert!(world.spawn_character(player, 11, 10));
        let mut loader = item(7, ItemFlags::USED | ItemFlags::USE);
        loader.driver = IDR_FDEMONLOADER;
        assert!(world.map.set_item_map(&mut loader, 10, 10));
        world.map.tile_mut(10, 10).unwrap().ground_sprite = 123;
        world.add_item(loader);
        let mut crystal = item(9, ItemFlags::USED);
        crystal.template_id = 0x0100004A;
        crystal.driver_data = vec![12];
        crystal.carried_by = Some(CharacterId(1));
        world.add_item(crystal);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONLOADER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            8,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::FdemonLoaderChanged { .. }
        ));
        assert!(!world.items.contains_key(&ItemId(9)));
        assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
        assert_eq!(world.items[&ItemId(7)].sprite, 59036);
        assert_eq!(
            world.map.tile(10, 10).unwrap().ground_sprite,
            (59021 << 16) | 123
        );
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            41
        );
        assert_eq!(world.timers.used_timers(), 0);

        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
        world.advance();
        let outcomes = world.process_due_timers(8);

        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            outcomes[0],
            ItemDriverOutcome::FdemonLoaderChanged { .. }
        ));
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn world_applies_edemon_loader_and_powers_matching_section_light() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.cursor_item = Some(ItemId(9));
        assert!(world.spawn_character(player, 11, 10));
        let mut loader = item(7, ItemFlags::USED | ItemFlags::USE);
        loader.driver = IDR_EDEMONLOADER;
        loader.driver_data = vec![2, 0, 0];
        assert!(world.map.set_item_map(&mut loader, 10, 10));
        world.map.tile_mut(10, 10).unwrap().ground_sprite = 123;
        world.add_item(loader);
        let mut crystal = item(9, ItemFlags::USED);
        crystal.template_id = 0x01000049;
        crystal.driver_data = vec![86];
        crystal.carried_by = Some(CharacterId(1));
        world.add_item(crystal);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_EDEMONLOADER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            6,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::EdemonLoaderChanged { .. }
        ));
        assert!(!world.items.contains_key(&ItemId(9)));
        assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
        assert_eq!(world.items[&ItemId(7)].driver_data, vec![2, 86, 7]);
        assert_eq!(world.items[&ItemId(7)].sprite, 14260);
        assert_eq!(
            world.map.tile(10, 10).unwrap().ground_sprite,
            (14240 << 16) | 123
        );
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            41
        );

        let mut light = item(10, ItemFlags::USED);
        light.driver = IDR_EDEMONLIGHT;
        light.driver_data = vec![2];
        assert!(world.map.set_item_map(&mut light, 12, 10));
        world.add_item(light);
        assert!(world.schedule_item_driver_timer(ItemId(10), CharacterId(0), 1));
        world.advance();
        let light_outcomes = world.process_due_timers(6);

        assert!(light_outcomes
            .iter()
            .any(|outcome| matches!(outcome, ItemDriverOutcome::LightChanged { .. })));
        assert_eq!(world.items[&ItemId(10)].sprite, 14191);
        assert_eq!(world.items[&ItemId(10)].modifier_value[0], 200);
    }

    #[test]
    fn world_moves_edemon_block_on_character_use_and_blocks_bad_target() {
        let mut world = World::default();
        let mut player = character(1);
        player.dir = Direction::Right as u8;
        assert!(world.spawn_character(player, 9, 10));
        let mut block = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
        block.driver = IDR_EDEMONBLOCK;
        assert!(world.map.set_item_map(&mut block, 10, 10));
        world.map.tile_mut(11, 10).unwrap().ground_sprite = 12150;
        world.add_item(block);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_EDEMONBLOCK,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            6,
        );

        assert!(matches!(outcome, ItemDriverOutcome::EdemonBlockMove { .. }));
        assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
        assert!(!world
            .map
            .tile(10, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));
        assert_eq!(world.map.tile(11, 10).unwrap().item, 7);
        assert!(world
            .map
            .tile(11, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));
        assert_eq!(
            (world.items[&ItemId(7)].x, world.items[&ItemId(7)].y),
            (11, 10)
        );

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_EDEMONBLOCK,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            6,
        );
        assert!(matches!(outcome, ItemDriverOutcome::Noop));
        assert_eq!(
            (world.items[&ItemId(7)].x, world.items[&ItemId(7)].y),
            (11, 10)
        );
    }

    #[test]
    fn world_edemon_block_timer_returns_to_origin_and_is_scheduled_on_startup() {
        let mut world = World::default();
        world.add_character(character(0));
        let mut block = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
        block.driver = IDR_EDEMONBLOCK;
        block.driver_data = vec![0, 0, 0, 0, 10, 0, 10, 0];
        assert!(world.map.set_item_map(&mut block, 12, 10));
        world.map.tile_mut(10, 10).unwrap().ground_sprite = 12158;
        world.add_item(block);

        assert_eq!(world.schedule_existing_light_timers(), 1);
        world.tick.0 = TICKS_PER_SECOND * 60 * 15 + 3;
        let outcome = world.execute_item_driver_request_with_context(
            ItemDriverRequest::Driver {
                driver: IDR_EDEMONBLOCK,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            6,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert!(matches!(outcome, ItemDriverOutcome::EdemonBlockMove { .. }));
        assert_eq!(world.map.tile(12, 10).unwrap().item, 0);
        assert_eq!(world.map.tile(10, 10).unwrap().item, 7);
        assert_eq!(
            (world.items[&ItemId(7)].x, world.items[&ItemId(7)].y),
            (10, 10)
        );
        assert_eq!(world.timers.used_timers(), 2);
    }

    #[test]
    fn world_applies_fdemon_waypoint_marker_and_timer() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        world.add_character(player);
        let mut waypoint = item(7, ItemFlags::USED | ItemFlags::USE);
        waypoint.driver = crate::item_driver::IDR_FDEMONWAYPOINT;
        assert!(world.map.set_item_map(&mut waypoint, 10, 10));
        world.add_item(waypoint);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_FDEMONWAYPOINT,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            8,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::FdemonWaypoint {
                spotted_enemy: true,
                target_character_id: Some(CharacterId(1)),
                target_serial: Some(1),
                ..
            }
        ));
        let waypoint = &world.items[&ItemId(7)];
        assert_eq!(waypoint.driver_data[0], 1);
        assert_eq!(
            u32::from_le_bytes(waypoint.driver_data[4..8].try_into().unwrap()),
            1
        );
        assert_eq!(
            u32::from_le_bytes(waypoint.driver_data[8..12].try_into().unwrap()),
            1
        );
        assert_eq!(waypoint.sprite, 14200);
        assert_eq!(world.timers.used_timers(), 1);

        let mut demon = character(2);
        demon.flags.insert(CharacterFlags::FDEMON);
        world.add_character(demon);
        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_FDEMONWAYPOINT,
                item_id: ItemId(7),
                character_id: CharacterId(2),
                spec: 0,
            },
            8,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::FdemonWaypoint {
                spotted_enemy: false,
                ..
            }
        ));
        let waypoint = &world.items[&ItemId(7)];
        assert_eq!(waypoint.driver_data[0], 0);
        assert_eq!(
            u32::from_le_bytes(waypoint.driver_data[4..8].try_into().unwrap()),
            0
        );
        assert_eq!(
            u32::from_le_bytes(waypoint.driver_data[8..12].try_into().unwrap()),
            0
        );
        assert_eq!(waypoint.sprite, 14202);
    }

    #[test]
    fn world_applies_fdemon_farm_foreground_and_timer() {
        let mut world = World::default();
        world.add_character(character(0));
        let mut farm = item(7, ItemFlags::USED | ItemFlags::USE);
        farm.driver = IDR_FDEMONFARM;
        farm.driver_data = vec![5, 24, 24];
        assert!(world.map.set_item_map(&mut farm, 10, 10));
        world.map.tile_mut(10, 10).unwrap().foreground_sprite = 123;
        world.add_item(farm);

        let outcome = world.execute_item_driver_request_with_context(
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONFARM,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            8,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::FdemonFarmChanged { .. }
        ));
        assert_eq!(
            world.map.tile(10, 10).unwrap().foreground_sprite,
            (59040 << 16) | 123
        );
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn world_applies_fdemon_blood_fill_and_flask_destruction() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.cursor_item = Some(ItemId(9));
        world.add_character(player);
        let mut blood = item(7, ItemFlags::USED | ItemFlags::USE);
        blood.driver = IDR_FDEMONBLOOD;
        assert!(world.map.set_item_map(&mut blood, 10, 10));
        world.add_item(blood);
        let mut container = item(9, ItemFlags::USED);
        container.template_id = 0x0100004B;
        container.driver_data = vec![2];
        container.sprite = 100;
        container.carried_by = Some(CharacterId(1));
        world.add_item(container);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONBLOOD,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            8,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::FdemonBloodFilled { amount: 3, .. }
        ));
        assert!(!world.items.contains_key(&ItemId(7)));
        assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
        let container = &world.items[&ItemId(9)];
        assert_eq!(container.driver_data[0], 3);
        assert_eq!(container.sprite, 101);
        assert_eq!(
            container.description,
            "A container holding 3 parts golem blood."
        );

        let mut blood = item(11, ItemFlags::USED | ItemFlags::USE);
        blood.driver = IDR_FDEMONBLOOD;
        world.add_item(blood);
        let mut flask = item(12, ItemFlags::USED);
        flask.driver = IDR_FLASK;
        flask.carried_by = Some(CharacterId(1));
        world.add_item(flask);
        world
            .characters
            .get_mut(&CharacterId(1))
            .unwrap()
            .cursor_item = Some(ItemId(12));

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONBLOOD,
                item_id: ItemId(11),
                character_id: CharacterId(1),
                spec: 0,
            },
            8,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::FdemonBloodDestroyedFlask { .. }
        ));
        assert!(!world.items.contains_key(&ItemId(12)));
        assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
        assert_eq!(world.items[&ItemId(11)].sprite, 14348);
    }

    #[test]
    fn world_applies_fdemon_lava_activation_and_timer_damage() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.cursor_item = Some(ItemId(9));
        world.add_character(player);
        let mut lava = item(7, ItemFlags::USED | ItemFlags::USE);
        lava.driver = IDR_FDEMONLAVA;
        assert!(world.map.set_item_map(&mut lava, 10, 10));
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
        world.add_item(lava);
        let mut container = item(9, ItemFlags::USED);
        container.template_id = (0x01 << 24) | 0x00004B;
        container.driver_data = vec![2];
        container.sprite = 100;
        container.carried_by = Some(CharacterId(1));
        world.add_item(container);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONLAVA,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            8,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::FdemonLavaActivated { amount: 1, .. }
        ));
        let lava_tile = world.map.tile(10, 10).unwrap();
        assert!(!lava_tile.flags.contains(MapFlags::MOVEBLOCK));
        assert_eq!(lava_tile.foreground_sprite, 1034 << 16);
        assert_eq!(world.items[&ItemId(7)].driver_data[0], 120);
        assert_eq!(world.items[&ItemId(7)].sprite, 14366);
        assert_eq!(world.items[&ItemId(9)].driver_data[0], 1);
        assert_eq!(world.items[&ItemId(9)].sprite, 99);
        assert_eq!(world.timers.used_timers(), 1);

        world.add_character(character(0));
        world.items.get_mut(&ItemId(7)).unwrap().driver_data[0] = 20;
        let mut target = character(2);
        target.x = 10;
        target.y = 10;
        target.hp = 20 * POWERSCALE;
        world.add_character(target);
        world.map.tile_mut(10, 10).unwrap().character = 2;

        let outcome = world.execute_item_driver_request_with_context(
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONLAVA,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            8,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::FdemonLavaPulse {
                stage: 19,
                damage,
                armor_percent: 50,
                ..
            } if damage == 10 * POWERSCALE
        ));
        assert_eq!(world.items[&ItemId(7)].sprite, 14364);
        assert_eq!(
            world.map.tile(10, 10).unwrap().foreground_sprite,
            1024 << 16
        );
        assert_eq!(world.characters[&CharacterId(2)].hp, 10 * POWERSCALE);
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
    fn world_fireball_machine_timer_creates_retained_projectile_and_reschedules() {
        let mut world = World::default();
        let mut machine = item(7, ItemFlags::USED | ItemFlags::USE);
        machine.driver = IDR_FIREBALL;
        machine.x = 10;
        machine.y = 20;
        machine.driver_data = vec![130, 125, 42, 9];
        world.add_item(machine);
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

        world.advance();
        let outcomes = world.process_due_timers(1);

        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            outcomes[0],
            ItemDriverOutcome::FireballMachineProjectile {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                start_x: 11,
                start_y: 19,
                target_x: 12,
                target_y: 17,
                power: 42,
                schedule_after_ticks: Some(9),
            }
        );
        assert_eq!(world.effects.len(), 1);
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_FIREBALL);
        assert_eq!(effect.strength, 42);
        assert_eq!(effect.light, 200);
        assert_eq!((effect.from_x, effect.from_y), (11, 19));
        assert_eq!((effect.to_x, effect.to_y), (12, 17));
        assert_eq!((effect.x, effect.y), (11 * 1024 + 512, 19 * 1024 + 512));
        assert_eq!(effect.caster, None);
        assert_eq!(effect.stop_tick, 1 + TICKS_PER_SECOND as i32);

        for _ in 0..8 {
            world.advance();
        }
        assert!(world.process_due_timers(1).is_empty());
        world.advance();
        assert_eq!(world.process_due_timers(1).len(), 1);
    }

    #[test]
    fn world_edemonball_timer_creates_retained_projectile_effect() {
        let mut world = World::default();
        let mut cannon = item(7, ItemFlags::USED | ItemFlags::USE);
        cannon.driver = IDR_EDEMONBALL;
        cannon.x = 10;
        cannon.y = 20;
        cannon.driver_data = vec![1, 2, 42, 0];
        world.add_item(cannon);
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

        world.advance();
        let outcomes = world.process_due_timers(6);

        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            &outcomes[0],
            &ItemDriverOutcome::EdemonBallProjectile {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                start_x: 10,
                start_y: 21,
                target_x: 10,
                target_y: 30,
                strength: 42,
                base_sprite: 2,
                schedule_after_ticks: TICKS_PER_SECOND * 16,
            }
        );
        let cannon = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(cannon.driver_data[3], 1);
        assert_eq!(world.effects.len(), 1);
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_EDEMONBALL);
        assert_eq!(effect.strength, 42);
        assert_eq!(effect.base_sprite, 2);
        assert_eq!((effect.from_x, effect.from_y), (10, 21));
        assert_eq!((effect.to_x, effect.to_y), (10, 30));
        assert_eq!((effect.x, effect.y), (10 * 1024 + 512, 21 * 1024 + 512));
        assert_eq!(effect.stop_tick, 1 + (TICKS_PER_SECOND * 4) as i32);
    }

    #[test]
    fn caligar_gun_timer_creates_fixed_edemonball_projectiles() {
        let mut world = World::default();
        let mut gun = item(7, ItemFlags::USED | ItemFlags::USE);
        gun.driver = IDR_CALIGAR;
        gun.x = 10;
        gun.y = 20;
        gun.driver_data = vec![9];
        world.add_item(gun);
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

        world.advance();
        let outcomes = world.process_due_timers(36);

        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::CaligarGunProjectile {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                direction: 5,
                schedule_after_ticks: 12,
            }]
        );
        assert_eq!(world.effects.len(), 4);
        let mut shots: Vec<_> = world
            .effects
            .values()
            .map(|effect| {
                (
                    effect.effect_type,
                    effect.from_x,
                    effect.from_y,
                    effect.to_x,
                    effect.to_y,
                    effect.strength,
                    effect.base_sprite,
                )
            })
            .collect();
        shots.sort();
        assert_eq!(
            shots,
            vec![
                (EF_EDEMONBALL, 9, 20, 0, 20, 50, 1),
                (EF_EDEMONBALL, 10, 19, 10, 10, 50, 1),
                (EF_EDEMONBALL, 10, 21, 10, 30, 50, 1),
                (EF_EDEMONBALL, 11, 20, 20, 20, 50, 1),
            ]
        );
        for _ in 0..12 {
            world.advance();
        }
        assert_eq!(world.process_due_timers(36).len(), 1);
    }

    #[test]
    fn edemonball_timer_aims_at_nearby_character_before_fallback_rotation() {
        let mut world = World::default();
        let mut cannon = item(7, ItemFlags::USED | ItemFlags::USE);
        cannon.driver = IDR_EDEMONBALL;
        cannon.x = 10;
        cannon.y = 20;
        cannon.driver_data = vec![1, 2, 42, 0];
        world.add_item(cannon);
        assert!(world.spawn_character(character(1), 10, 25));
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

        world.advance();
        let outcomes = world.process_due_timers(6);

        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            &outcomes[0],
            &ItemDriverOutcome::EdemonBallProjectile {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                start_x: 10,
                start_y: 21,
                target_x: 10,
                target_y: 25,
                strength: 42,
                base_sprite: 2,
                schedule_after_ticks: TICKS_PER_SECOND * 8,
            }
        );
        let cannon = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(cannon.driver_data[3], 0);
        let effect = world.effects.values().next().unwrap();
        assert_eq!((effect.from_x, effect.from_y), (10, 21));
        assert_eq!((effect.to_x, effect.to_y), (10, 25));
    }

    #[test]
    fn edemonball_timer_predicts_walking_character_target_tile() {
        let mut world = World::default();
        let mut cannon = item(7, ItemFlags::USED | ItemFlags::USE);
        cannon.driver = IDR_EDEMONBALL;
        cannon.x = 10;
        cannon.y = 20;
        cannon.driver_data = vec![1, 2, 42, 0];
        world.add_item(cannon);
        let mut target = character(1);
        target.action = action::WALK;
        target.dir = Direction::Down as u8;
        target.duration = 10;
        target.step = 5;
        target.tox = 10;
        target.toy = 26;
        assert!(world.spawn_character(target, 10, 25));
        assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

        world.advance();
        let outcomes = world.process_due_timers(6);

        assert!(matches!(
            outcomes.first(),
            Some(ItemDriverOutcome::EdemonBallProjectile {
                start_x: 10,
                start_y: 21,
                target_x: 10,
                target_y: 26,
                schedule_after_ticks,
                ..
            }) if *schedule_after_ticks == TICKS_PER_SECOND * 8
        ));
        let effect = world.effects.values().next().unwrap();
        assert_eq!((effect.to_x, effect.to_y), (10, 26));
    }

    #[test]
    fn edemonball_effect_moves_by_legacy_quarter_tile_steps() {
        let mut world = World::default();
        let effect_id = world.create_edemonball_effect(10, 10, 10, 20, 7, 1);

        world.tick_effects();

        let effect = world.effects.get(&effect_id).unwrap();
        assert_eq!(effect.effect_type, EF_EDEMONBALL);
        assert_eq!((effect.x, effect.y), (10 * 1024 + 512, 10 * 1024 + 768));
        assert_eq!((effect.last_x, effect.last_y), (10, 10));
        assert!(world
            .map
            .tile(10, 10)
            .unwrap()
            .effects
            .contains(&(effect_id as u16)));
    }

    #[test]
    fn edemonball_effect_explodes_on_character_and_applies_direct_damage() {
        let mut world = World::default();
        let mut target = character(1);
        target.hp = 10_000;
        assert!(world.spawn_character(target, 10, 12));
        let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 3, 0);

        for _ in 0..6 {
            world.tick_effects();
        }

        assert!(!world
            .effects
            .values()
            .any(|effect| effect.effect_type == EF_EDEMONBALL));
        let target = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(target.hp, 7_000);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
        assert!(world.effects.values().any(|effect| {
            effect.effect_type == EF_EXPLODE
                && effect.base_sprite == 50450
                && effect
                    .fields
                    .iter()
                    .any(|&field| field == world.map.legacy_index(10, 12).unwrap() as i32)
        }));
    }

    #[test]
    fn edemonball_impact_uses_legacy_hurt_reduction() {
        let mut world = World::default();
        let mut target = character(1);
        target.hp = 10_000;
        target.lifeshield = POWERSCALE;
        target.values[0][CharacterValue::Armor as usize] = 60;
        assert!(world.spawn_character(target, 10, 12));
        let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 3, 1);

        for _ in 0..6 {
            world.tick_effects();
        }

        let target = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(target.hp, 8_500);
        assert_eq!(target.lifeshield, 0);
        assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
        assert_eq!(target.driver_messages[0].dat1, 0);
        assert_eq!(target.driver_messages[0].dat2, 1_500);
    }

    #[test]
    fn edemonball_green_base_is_absorbed_by_green_crystal() {
        let mut world = World::default();
        let mut target = character(1);
        target.hp = 10_000;
        target.inventory[30] = Some(ItemId(77));
        assert!(world.spawn_character(target, 10, 12));
        let mut crystal = item(77, ItemFlags::USED);
        crystal.carried_by = Some(CharacterId(1));
        crystal.template_id = IID_AREA6_GREENCRYSTAL;
        crystal.driver_data = vec![100];
        crystal.sprite = 50318;
        world.items.insert(ItemId(77), crystal);
        let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 30, 0);

        for _ in 0..6 {
            world.tick_effects();
        }

        let target = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(target.hp, 10_000);
        assert!(target.flags.contains(CharacterFlags::ITEMS));
        let crystal = world.items.get(&ItemId(77)).unwrap();
        assert_eq!(crystal.driver_data[0], 70);
        assert_eq!(crystal.sprite, 50322);
    }

    #[test]
    fn edemonball_green_crystals_are_destroyed_until_damage_remaining() {
        let mut world = World::default();
        let mut target = character(1);
        target.hp = 10_000;
        target.cursor_item = Some(ItemId(77));
        target.inventory[30] = Some(ItemId(78));
        assert!(world.spawn_character(target, 10, 12));
        for (id, power) in [(77, 20), (78, 40)] {
            let mut crystal = item(id, ItemFlags::USED);
            crystal.carried_by = Some(CharacterId(1));
            crystal.template_id = IID_AREA6_GREENCRYSTAL;
            crystal.driver_data = vec![power];
            world.items.insert(ItemId(id), crystal);
        }
        let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 70, 0);

        for _ in 0..6 {
            world.tick_effects();
        }

        let target = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(target.hp, 0);
        assert_eq!(target.cursor_item, None);
        assert_eq!(target.inventory[30], None);
        assert!(!world.items.contains_key(&ItemId(77)));
        assert!(!world.items.contains_key(&ItemId(78)));
    }

    #[test]
    fn edemonball_non_green_base_ignores_green_crystal() {
        let mut world = World::default();
        let mut target = character(1);
        target.hp = 10_000;
        target.inventory[30] = Some(ItemId(77));
        assert!(world.spawn_character(target, 10, 12));
        let mut crystal = item(77, ItemFlags::USED);
        crystal.carried_by = Some(CharacterId(1));
        crystal.template_id = IID_AREA6_GREENCRYSTAL;
        crystal.driver_data = vec![100];
        world.items.insert(ItemId(77), crystal);
        let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 3, 1);

        for _ in 0..6 {
            world.tick_effects();
        }

        let target = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(target.hp, 7_000);
        assert_eq!(world.items.get(&ItemId(77)).unwrap().driver_data[0], 100);
    }

    #[test]
    fn edemonball_effect_explodes_on_wall_at_previous_tile() {
        let mut world = World::default();
        world
            .map
            .tile_mut(10, 11)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
        let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 3, 2);

        world.tick_effects();
        world.tick_effects();

        assert!(!world
            .effects
            .values()
            .any(|effect| effect.effect_type == EF_EDEMONBALL));
        assert!(world.effects.values().any(|effect| {
            effect.effect_type == EF_EXPLODE
                && effect.base_sprite == 50452
                && effect
                    .fields
                    .iter()
                    .any(|&field| field == world.map.legacy_index(10, 10).unwrap() as i32)
        }));
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
    fn world_spiketrap_damage_uses_legacy_hurt_reduction() {
        let mut world = World::default();
        let mut character = character(1);
        character.hp = 10_000;
        character.lifeshield = 1_000;
        character.values[0][CharacterValue::Armor as usize] = 20;
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
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.hp, 8_000);
        assert_eq!(character.lifeshield, 0);
        assert_eq!(character.driver_messages[0].message_type, NT_GOTHIT);
        assert_eq!(character.driver_messages[0].dat2, 2_000);
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
    fn schedule_existing_light_timers_includes_caligar_flames() {
        let mut world = World::default();
        let mut flame = item(7, ItemFlags::USED | ItemFlags::USE);
        flame.driver = IDR_CALIGARFLAME;
        flame.driver_data = vec![1, 3, 0, 0];
        world.add_item(flame);

        assert_eq!(world.schedule_existing_light_timers(), 1);

        world.advance();
        let outcomes = world.process_due_timers(36);

        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            outcomes[0],
            ItemDriverOutcome::FlameThrowerPulse {
                item_id: ItemId(7),
                direction: 3,
                ..
            }
        ));
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
    fn burn_character_damage_uses_legacy_hurt_reduction() {
        let mut world = World::default();
        let mut character = character(1);
        character.hp = 50 * POWERSCALE;
        character.lifeshield = 5 * POWERSCALE;
        character.values[0][CharacterValue::Armor as usize] = 100;
        world.add_character(character);

        assert!(world.burn_character(CharacterId(1)));

        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.hp, 40 * POWERSCALE);
        assert_eq!(character.lifeshield, 0);
        assert_eq!(character.driver_messages[0].message_type, NT_GOTHIT);
        assert_eq!(character.driver_messages[0].dat2, 10 * POWERSCALE);
    }

    #[test]
    fn burn_effect_tick_applies_recurring_legacy_hurt_damage() {
        let mut world = World::default();
        let mut character = character(1);
        character.hp = 50 * POWERSCALE;
        character.lifeshield = POWERSCALE;
        world.add_character(character);

        assert!(world.burn_character(CharacterId(1)));
        let hp_after_initial_burn = world.characters[&CharacterId(1)].hp;
        let shield_after_initial_burn = world.characters[&CharacterId(1)].lifeshield;

        world.tick_effects();

        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.hp, hp_after_initial_burn - 167);
        assert_eq!(character.lifeshield, shield_after_initial_burn);
        assert_eq!(
            character.driver_messages.last().unwrap().message_type,
            NT_GOTHIT
        );
        assert_eq!(character.driver_messages.last().unwrap().dat2, 167);
        assert_eq!(world.effects.len(), 1);
    }

    #[test]
    fn burn_effect_tick_removes_stale_attached_effect() {
        let mut world = World::default();
        assert!(!world.burn_character(CharacterId(1)));

        let effect_id = world.next_effect_id();
        let mut effect = Effect::new(EF_BURN, effect_id as i32, 0, TICKS_PER_SECOND as i32 * 60);
        effect.target_character = Some(CharacterId(99));
        effect.strength = 1;
        world.effects.insert(effect_id, effect);

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
    fn world_places_and_ages_area18_bone_bridge_segments() {
        let mut world = World::default();
        let mut character = character(1);
        character.cursor_item = Some(ItemId(8));
        assert!(world.spawn_character(character, 10, 10));

        let mut bridge_base = item(7, ItemFlags::USED | ItemFlags::USE);
        bridge_base.driver = IDR_BONEBRIDGE;
        bridge_base.x = 11;
        bridge_base.y = 10;
        world.map.tile_mut(11, 10).unwrap().item = 7;
        world.map.set_flags(12, 10, MapFlags::MOVEBLOCK);

        let mut bone = item(8, ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE);
        bone.driver = IDR_BONEBRIDGE;
        bone.template_id = IID_AREA18_BONE;
        bone.carried_by = Some(CharacterId(1));
        bone.driver_data = vec![5];
        world.add_item(bridge_base);
        world.add_item(bone);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_BONEBRIDGE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            18,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::BoneBridgePlace {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(8),
            }
        );
        let tile = world.map.tile(12, 10).unwrap();
        assert_eq!(tile.item, 8);
        assert!(!tile.flags.contains(MapFlags::MOVEBLOCK));
        let bone = world.items.get(&ItemId(8)).unwrap();
        assert_eq!((bone.x, bone.y), (12, 10));
        assert_eq!(bone.carried_by, None);
        assert!(!bone.flags.contains(ItemFlags::TAKE));
        assert_eq!(bone.driver_data[1], 1);
        assert_eq!(bone.sprite, 13035);
        assert_eq!(
            world.characters.get(&CharacterId(1)).unwrap().cursor_item,
            None
        );
        assert_eq!(world.timers.used_timers(), 1);

        world.add_character(timer_callback_character());
        let outcome = world.execute_item_driver_request_with_context(
            ItemDriverRequest::Driver {
                driver: IDR_BONEBRIDGE,
                item_id: ItemId(8),
                character_id: CharacterId(0),
                spec: 0,
            },
            18,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::BoneBridgeTimerTick { item_id: ItemId(8) }
        );
        assert!(world
            .map
            .tile(12, 10)
            .unwrap()
            .flags
            .contains(MapFlags::MOVEBLOCK));
        let bone = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(bone.driver_data[1], 2);
        assert_eq!(bone.sprite, 13036);
        assert_eq!(world.timers.used_timers(), 2);
    }

    #[test]
    fn world_retries_or_removes_area18_bone_bridge_timer_cleanup_like_c() {
        let mut world = World::default();
        world.add_character(timer_callback_character());
        let mut bone = item(8, ItemFlags::USED | ItemFlags::USE);
        bone.driver = IDR_BONEBRIDGE;
        bone.driver_data = vec![5, 1];
        bone.x = 12;
        bone.y = 10;
        world.map.tile_mut(12, 10).unwrap().item = 8;
        world.map.tile_mut(12, 10).unwrap().flags = MapFlags::TMOVEBLOCK;
        world.add_item(bone);

        let request = ItemDriverRequest::Driver {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        };
        let context = ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        };
        assert_eq!(
            world.execute_item_driver_request_with_context(request, 18, &context),
            ItemDriverOutcome::BoneBridgeTimerTick { item_id: ItemId(8) }
        );
        assert_eq!(world.items.get(&ItemId(8)).unwrap().driver_data[1], 1);
        assert_eq!(world.timers.used_timers(), 1);

        world.map.tile_mut(12, 10).unwrap().flags = MapFlags::empty();
        world.items.get_mut(&ItemId(8)).unwrap().driver_data[1] = 9;
        assert_eq!(
            world.execute_item_driver_request_with_context(request, 18, &context),
            ItemDriverOutcome::BoneBridgeTimerTick { item_id: ItemId(8) }
        );
        assert!(!world.items.contains_key(&ItemId(8)));
        let tile = world.map.tile(12, 10).unwrap();
        assert_eq!(tile.item, 0);
        assert!(tile.flags.contains(MapFlags::MOVEBLOCK));
    }

    #[test]
    fn world_opens_and_restores_area18_bone_walls_like_c() {
        let mut world = World::default();
        assert!(world.spawn_character(character(1), 10, 10));
        world.add_character(timer_callback_character());

        for (id, x) in [(7_u32, 11_usize), (8, 12)] {
            let mut wall = item(
                id,
                ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK,
            );
            wall.driver = IDR_BONEWALL;
            wall.x = x as u16;
            wall.y = 10;
            wall.sprite = 14000;
            world.map.tile_mut(x, 10).unwrap().item = id;
            world
                .map
                .tile_mut(x, 10)
                .unwrap()
                .flags
                .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
            world.add_item(wall);
        }

        let request = ItemDriverRequest::Driver {
            driver: IDR_BONEWALL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        assert_eq!(
            world.execute_item_driver_request(request, 18),
            ItemDriverOutcome::BoneWallTick {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 1);
        assert_eq!(world.items.get(&ItemId(7)).unwrap().sprite, 14001);
        assert_eq!(world.timers.used_timers(), 2);

        world.items.get_mut(&ItemId(7)).unwrap().driver_data[0] = 5;
        let timer_request = ItemDriverRequest::Driver {
            driver: IDR_BONEWALL,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };
        let context = ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        };
        assert_eq!(
            world.execute_item_driver_request_with_context(timer_request, 18, &context),
            ItemDriverOutcome::BoneWallTick {
                item_id: ItemId(7),
                character_id: CharacterId(0),
            }
        );
        let tile = world.map.tile(11, 10).unwrap();
        assert_eq!(tile.item, 0);
        assert!(!tile.flags.contains(MapFlags::TMOVEBLOCK));
        assert!(!tile.flags.contains(MapFlags::TSIGHTBLOCK));
        let wall = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(wall.driver_data[0], 6);
        assert!(wall.flags.contains(ItemFlags::VOID));
        assert!(!wall.flags.contains(ItemFlags::USE));

        world.map.tile_mut(11, 10).unwrap().item = 99;
        assert_eq!(
            world.execute_item_driver_request_with_context(timer_request, 18, &context),
            ItemDriverOutcome::BoneWallTick {
                item_id: ItemId(7),
                character_id: CharacterId(0),
            }
        );
        assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 6);

        world.map.tile_mut(11, 10).unwrap().item = 0;
        assert_eq!(
            world.execute_item_driver_request_with_context(timer_request, 18, &context),
            ItemDriverOutcome::BoneWallTick {
                item_id: ItemId(7),
                character_id: CharacterId(0),
            }
        );
        let tile = world.map.tile(11, 10).unwrap();
        assert_eq!(tile.item, 7);
        assert!(tile.flags.contains(MapFlags::TMOVEBLOCK));
        assert!(tile.flags.contains(MapFlags::TSIGHTBLOCK));
        let wall = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(wall.driver_data[0], 0);
        assert_eq!(wall.sprite, 13996);
        assert!(wall.flags.contains(ItemFlags::USE));
        assert!(!wall.flags.contains(ItemFlags::VOID));
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
    fn world_blocks_player_kill_setup_against_area_one_player() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker.flags.insert(CharacterFlags::PLAYER);
        attacker.x = 10;
        attacker.y = 10;
        let mut defender = character(2);
        defender.flags.insert(CharacterFlags::PLAYER);
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
        assert_eq!(attacker.action, action::IDLE);
        assert_eq!(player.action.action, PlayerActionCode::Idle);
    }

    #[test]
    fn world_blocks_player_kill_setup_without_pk_hate_entry() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        attacker.x = 10;
        attacker.y = 10;
        let mut defender = character(2);
        defender
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
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

        assert!(world.apply_player_action_setup(&mut player, 2));
        let attacker = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(attacker.action, action::IDLE);
        assert_eq!(player.action.action, PlayerActionCode::Idle);
    }

    #[test]
    fn world_removes_stale_pk_hate_when_pvp_level_check_fails() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        attacker.level = 10;
        attacker.x = 10;
        attacker.y = 10;
        let mut defender = character(2);
        defender
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        defender.level = 14;
        defender.x = 11;
        defender.y = 10;
        world.map.tile_mut(11, 10).unwrap().character = 2;
        world.add_character(attacker);
        world.add_character(defender);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        assert!(player.add_pk_hate(2));
        player.action = QueuedAction {
            action: PlayerActionCode::Kill,
            arg1: 2,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 2));

        assert!(!player.has_pk_hate_for(2));
        assert_eq!(player.action.action, PlayerActionCode::Idle);
    }

    #[test]
    fn world_keeps_pk_hate_when_area_one_blocks_pvp() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        attacker.level = 10;
        attacker.x = 10;
        attacker.y = 10;
        let mut defender = character(2);
        defender
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        defender.level = 10;
        defender.x = 11;
        defender.y = 10;
        world.map.tile_mut(11, 10).unwrap().character = 2;
        world.add_character(attacker);
        world.add_character(defender);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        assert!(player.add_pk_hate(2));
        player.action = QueuedAction {
            action: PlayerActionCode::Kill,
            arg1: 2,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 1));

        assert!(player.has_pk_hate_for(2));
        assert_eq!(player.action.action, PlayerActionCode::Idle);
    }

    #[test]
    fn world_allows_player_kill_setup_with_pk_hate_entry() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        attacker.x = 10;
        attacker.y = 10;
        let mut defender = character(2);
        defender
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        defender.x = 11;
        defender.y = 10;
        world.map.tile_mut(11, 10).unwrap().character = 2;
        world.add_character(attacker);
        world.add_character(defender);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        assert!(player.add_pk_hate(2));
        player.action = QueuedAction {
            action: PlayerActionCode::Kill,
            arg1: 2,
            arg2: 0,
        };

        assert!(world.apply_player_action_setup(&mut player, 2));
        let attacker = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(attacker.action, action::ATTACK1);
        assert_eq!(attacker.act1, 2);
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
        attacker.flags.insert(CharacterFlags::PLAYER);
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
        let defender = world.characters.get(&CharacterId(2)).unwrap();
        assert!(defender.hp < 10_000);
        assert_eq!(defender.driver_messages[0].message_type, NT_GOTHIT);
        assert_eq!(defender.driver_messages[0].dat1, 1);
        assert_eq!(
            world.characters[&CharacterId(1)].driver_messages[0].message_type,
            NT_DIDHIT
        );
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            7
        );
    }

    #[test]
    fn completed_attack_queues_legacy_unarmed_miss_sound() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker.flags.insert(CharacterFlags::PLAYER);
        attacker.x = 10;
        attacker.y = 10;
        attacker.dir = Direction::Right as u8;
        attacker.action = action::ATTACK1;
        attacker.duration = 1;
        attacker.act1 = 2;
        attacker.values[0][CharacterValue::Attack as usize] = 10;
        let mut defender = character(2);
        defender.x = 11;
        defender.y = 10;
        defender.values[0][CharacterValue::Parry as usize] = 10;
        world.spawn_character(attacker, 10, 10);
        world.spawn_character(defender, 11, 10);

        assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 100, 1));

        assert_eq!(world.characters[&CharacterId(2)].hp, 0);
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            8
        );
    }

    #[test]
    fn completed_attack_queues_legacy_weapon_clash_miss_sound() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker.flags.insert(CharacterFlags::PLAYER);
        attacker.x = 10;
        attacker.y = 10;
        attacker.dir = Direction::Right as u8;
        attacker.act1 = 2;
        attacker.values[0][CharacterValue::Attack as usize] = 10;
        let mut defender = character(2);
        defender.x = 11;
        defender.y = 10;
        defender.values[0][CharacterValue::Parry as usize] = 10;
        let mut attacker_weapon = item(10, ItemFlags::USED | ItemFlags::WNRHAND);
        attacker_weapon.carried_by = Some(CharacterId(1));
        let mut defender_weapon = item(11, ItemFlags::USED | ItemFlags::WNRHAND);
        defender_weapon.carried_by = Some(CharacterId(2));
        attacker.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(10));
        defender.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(11));
        world.spawn_character(attacker, 10, 10);
        world.spawn_character(defender, 11, 10);
        world.add_item(attacker_weapon);
        world.add_item(defender_weapon);

        assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 100, 1));
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            34
        );
        assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 99, 1));
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            35
        );
    }

    #[test]
    fn completed_attack_queues_legacy_showattack_pre_hurt_line() {
        let mut world = World::default();
        world.show_attack_debug = true;
        let mut attacker = character(1);
        attacker.x = 10;
        attacker.y = 10;
        attacker.dir = Direction::Right as u8;
        attacker.act1 = 2;
        attacker.values[0][CharacterValue::Attack as usize] = 10;
        attacker.values[0][CharacterValue::Weapon as usize] = 10;
        let mut defender = character(2);
        defender.name = "Target".to_string();
        defender.x = 11;
        defender.y = 10;
        defender.dir = Direction::Left as u8;
        defender.values[0][CharacterValue::Parry as usize] = 10;
        world.spawn_character(attacker, 10, 10);
        world.spawn_character(defender, 11, 10);

        assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 49, 6));

        let texts = world.drain_pending_system_texts();
        assert_eq!(texts[0].character_id, CharacterId(1));
        assert_eq!(
            texts[0].message,
            "attack Target, diff=0 (10 10), chan=50, percent=90, dam=16"
        );
    }

    #[test]
    fn completed_attack_weapon_clash_sound_uses_independent_legacy_roll() {
        let mut world = World::default();
        let mut attacker = character(1);
        attacker.flags.insert(CharacterFlags::PLAYER);
        attacker.x = 10;
        attacker.y = 10;
        attacker.dir = Direction::Right as u8;
        attacker.act1 = 2;
        attacker.values[0][CharacterValue::Attack as usize] = 10;
        let mut defender = character(2);
        defender.x = 11;
        defender.y = 10;
        defender.values[0][CharacterValue::Parry as usize] = 10;
        let mut attacker_weapon = item(10, ItemFlags::USED | ItemFlags::WNRHAND);
        attacker_weapon.carried_by = Some(CharacterId(1));
        let mut defender_weapon = item(11, ItemFlags::USED | ItemFlags::WNRHAND);
        defender_weapon.carried_by = Some(CharacterId(2));
        attacker.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(10));
        defender.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(11));
        world.spawn_character(attacker, 10, 10);
        world.spawn_character(defender, 11, 10);
        world.add_item(attacker_weapon);
        world.add_item(defender_weapon);

        assert!(world.complete_attack_with_rolls_and_clash_roll(
            CharacterId(1),
            CharacterId(2),
            100,
            1,
            1,
        ));
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            35
        );
        assert!(world.complete_attack_with_rolls_and_clash_roll(
            CharacterId(1),
            CharacterId(2),
            99,
            1,
            0,
        ));
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            34
        );
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
    fn world_applies_pent_boss_door_teleport_and_reverses_cardinal_facing() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 11;
        character.y = 10;
        character.dir = Direction::Right as u8;
        world.map.tile_mut(11, 10).unwrap().character = 1;
        world
            .map
            .tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        world.add_character(character);

        let outcome = world.apply_item_driver_outcome(
            ItemDriverOutcome::PentBossDoor {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 9,
                y: 10,
            },
            4,
        );

        assert!(matches!(outcome, ItemDriverOutcome::PentBossDoor { .. }));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (9, 10));
        assert_eq!(character.dir, Direction::Left as u8);
        assert_eq!(world.map.tile(11, 10).unwrap().character, 0);
        assert_eq!(world.map.tile(9, 10).unwrap().character, 1);
    }

    #[test]
    fn world_executes_freakdoor_partner_teleport_and_caches_partner() {
        let mut world = World::default();
        let mut character = character(1);
        character.x = 10;
        character.y = 10;
        character.tox = 11;
        character.toy = 10;
        world.map.tile_mut(10, 10).unwrap().character = 1;
        world.add_character(character);

        let door_flags = ItemFlags::USED
            | ItemFlags::USE
            | ItemFlags::DOOR
            | ItemFlags::MOVEBLOCK
            | ItemFlags::SIGHTBLOCK;
        let mut first = item(7, door_flags);
        first.driver = crate::item_driver::IDR_FREAKDOOR;
        first.x = 10;
        first.y = 10;
        first.driver_data = vec![0; 16];
        first.driver_data[8] = 3;
        world.map.tile_mut(10, 10).unwrap().item = 7;
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR);
        world.add_item(first);

        let mut second = item(8, door_flags);
        second.driver = crate::item_driver::IDR_FREAKDOOR;
        second.x = 20;
        second.y = 20;
        second.driver_data = vec![0; 16];
        second.driver_data[8] = 3;
        world.map.tile_mut(20, 20).unwrap().item = 8;
        world
            .map
            .tile_mut(20, 20)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR);
        world.add_item(second);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_FREAKDOOR,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(matches!(outcome, ItemDriverOutcome::FreakDoorUse { .. }));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (20, 20));
        assert_eq!((character.tox, character.toy), (21, 20));
        assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
        assert_eq!(world.map.tile(20, 20).unwrap().character, 1);
        let first = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(read_u32_le_at(&first.driver_data, 10), 8);
        let second = world.items.get(&ItemId(8)).unwrap();
        assert!(door_open_state(second));
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
        let mut actor = character(1);
        actor.flags.insert(CharacterFlags::PLAYER);
        actor.x = 10;
        actor.y = 10;
        world.add_character(actor);
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
        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].special.special_type, 3);

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
        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].special.special_type, 3);
    }

    #[test]
    fn world_executes_area17_pick_door_with_legacy_timer() {
        let mut world = World::default();
        let mut actor = character(1);
        actor.flags.insert(CharacterFlags::PLAYER);
        actor.x = 8;
        actor.y = 8;
        world.add_character(actor);
        let mut door = item(
            7,
            ItemFlags::USED
                | ItemFlags::USE
                | ItemFlags::MOVEBLOCK
                | ItemFlags::SIGHTBLOCK
                | ItemFlags::SOUNDBLOCK
                | ItemFlags::DOOR,
        );
        door.driver = crate::item_driver::IDR_PICKDOOR;
        door.sprite = 100;
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        let request = ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_PICKDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            world.execute_item_driver_request(request, 17),
            ItemDriverOutcome::PickDoorLocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        assert_eq!(
            world.execute_item_driver_request_with_context(
                request,
                17,
                &ItemDriverContext {
                    has_area17_lockpick: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::PickDoorToggle {
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
        assert_eq!(world.timers.used_timers(), 1);

        world.tick = Tick(TICKS_PER_SECOND * 20);
        let outcomes = world.process_due_timers(17);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            outcomes[0],
            ItemDriverOutcome::PickDoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(0),
            }
        );
        let door = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(door.driver_data[0], 0);
        assert_eq!(door.sprite, 100);
        assert!(door.flags.contains(ItemFlags::MOVEBLOCK));
        assert!(door.flags.contains(ItemFlags::SIGHTBLOCK));
        assert!(door.flags.contains(ItemFlags::SOUNDBLOCK));
        assert!(door.flags.contains(ItemFlags::DOOR));
    }

    #[test]
    fn world_executes_area17_burndown_barrel_ignite_and_timer() {
        let mut world = World::default();
        let mut actor = character(1);
        actor.x = 8;
        actor.y = 8;
        world.add_character(actor);
        let mut barrel = item(7, ItemFlags::USED | ItemFlags::USE);
        barrel.driver = crate::item_driver::IDR_BURNDOWN;
        barrel.sprite = 51076;
        barrel.driver_data = vec![0];
        assert!(world.map.set_item_map(&mut barrel, 10, 10));
        world.add_item(barrel);

        let request = ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_BURNDOWN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            world.execute_item_driver_request_with_context(
                request,
                17,
                &ItemDriverContext {
                    cursor_driver: Some(crate::item_driver::IDR_TORCH),
                    cursor_drdata0: Some(1),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::BurndownIgnite {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
        let barrel = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(barrel.driver_data[0], 20);
        assert_eq!(barrel.sprite, 51077);
        assert_eq!(barrel.modifier_index[0], CharacterValue::Light as i16);
        assert_eq!(barrel.modifier_value[0], 200);
        assert_eq!(
            world.map.tile(10, 10).unwrap().foreground_sprite,
            1024 << 16
        );
        assert_eq!(world.timers.used_timers(), 1);

        world.tick = Tick(TICKS_PER_SECOND * 5);
        let outcomes = world.process_due_timers(17);
        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::BurndownTimerTick { item_id: ItemId(7) }]
        );
        let barrel = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(barrel.driver_data[0], 19);
        assert_eq!(barrel.sprite, 51078);
        assert_eq!(world.timers.used_timers(), 1);

        world.items.get_mut(&ItemId(7)).unwrap().driver_data[0] = 16;
        world.tick = Tick(TICKS_PER_SECOND * 10);
        let outcomes = world.process_due_timers(17);
        assert_eq!(outcomes.len(), 1);
        let barrel = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(barrel.driver_data[0], 15);
        assert_eq!(barrel.modifier_value[0], 0);
        assert_eq!(world.map.tile(10, 10).unwrap().foreground_sprite, 0);
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
        let mut actor = character(1);
        actor.flags.insert(CharacterFlags::PLAYER);
        actor.x = 10;
        actor.y = 10;
        world.add_character(actor);
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
        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].special.special_type, 3);
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
        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].special.special_type, 2);
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
    fn world_applies_arkhata_key_final_assembly() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        character.cursor_item = Some(ItemId(8));
        world.add_character(character);

        let mut base = item(7, ItemFlags::USED | ItemFlags::USE);
        base.driver = crate::item_driver::IDR_ARKHATA;
        base.carried_by = Some(CharacterId(1));
        base.template_id = 0x0100_00CD;
        base.driver_data = vec![2];
        world.add_item(base);
        let mut cursor = item(8, ItemFlags::USED | ItemFlags::USE);
        cursor.driver = crate::item_driver::IDR_ARKHATA;
        cursor.carried_by = Some(CharacterId(1));
        cursor.template_id = 0x0100_00CC;
        world.add_item(cursor);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_ARKHATA,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            37,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::ArkhataKeyAssemble {
                final_key: true,
                ..
            }
        ));
        let base = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(base.sprite, 13413);
        assert_eq!(base.template_id, 0x3B00_0089);
        assert_eq!(base.name, "Knoger Key 1");
        assert_eq!(
            base.description,
            "A finished key. Should open something now. A door, perhaps."
        );
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
            character.values[0][CharacterValue::Intelligence as usize],
            10
        );
        assert_eq!(character.values[0][CharacterValue::Wisdom as usize], 10);
        assert_eq!(character.values[0][CharacterValue::Agility as usize], 10);
        assert_eq!(character.values[0][CharacterValue::Strength as usize], 10);
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
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            29
        );

        world.tick = Tick(2_980);
        world.process_due_timers(1);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert!(character.inventory[29].is_none());
        assert_eq!(
            character.values[0][CharacterValue::Intelligence as usize],
            0
        );
        assert_eq!(character.values[0][CharacterValue::Wisdom as usize], 0);
        assert_eq!(character.values[0][CharacterValue::Agility as usize], 0);
        assert_eq!(character.values[0][CharacterValue::Strength as usize], 0);
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
        assert_eq!(character.values[0][CharacterValue::Speed as usize], 100);
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

        assert_ne!(
            world
                .effects
                .get(&effect_id)
                .map(|effect| effect.effect_type),
            Some(EF_FIREBALL)
        );
        assert_eq!(world.map.tile(11, 10).unwrap().effects, [0; 4]);
        assert!(world
            .effects
            .values()
            .any(|effect| effect.effect_type == EF_EXPLODE
                && effect.base_sprite == 50050
                && effect.strength == 8));
        assert_eq!(
            world.drain_pending_sound_specials()[0].special.special_type,
            6
        );
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 14_100);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
    }

    #[test]
    fn fireball_effect_respects_runtime_attack_policy() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.x = 10;
        caster.y = 10;
        caster.act1 = 15;
        caster.act2 = 10;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        let mut target = character(2);
        target
            .flags
            .insert(CharacterFlags::ALIVE | CharacterFlags::PLAYER);
        target.hp = 30 * POWERSCALE;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
        world.create_fireball_effect(&caster);

        world.tick_effects_with_attack_policy(|_, _, _, _| false);
        world.tick_effects_with_attack_policy(|_, _, _, _| false);

        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 30 * POWERSCALE);
        assert!(!target.flags.contains(CharacterFlags::UPDATE));
    }

    #[test]
    fn fireball_impact_uses_legacy_hurt_reduction() {
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
        target.lifeshield = POWERSCALE;
        target.values[0][CharacterValue::Armor as usize] = 20;
        target.values[0][CharacterValue::Immunity as usize] = 20;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
        world.create_fireball_effect(&caster);

        world.tick_effects();
        world.tick_effects();

        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 15_200);
        assert_eq!(target.lifeshield, 0);
        assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
        assert_eq!(target.driver_messages[0].dat1, 1);
        assert_eq!(target.driver_messages[0].dat2, 14_800);
        assert_eq!(
            world.characters[&CharacterId(1)].driver_messages[0].message_type,
            NT_DIDHIT
        );
    }

    #[test]
    fn fireball_hit_earth_demon_shoots_weaker_fireball_back() {
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
        target
            .flags
            .insert(CharacterFlags::ALIVE | CharacterFlags::EDEMON);
        target.hp = 30 * POWERSCALE;
        target.values[0][CharacterValue::Immunity as usize] = 20;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
        world.create_fireball_effect(&caster);

        world.tick_effects();
        world.tick_effects();

        let shootback = world
            .effects
            .values()
            .find(|effect| effect.effect_type == EF_FIREBALL)
            .unwrap();
        assert_eq!(shootback.strength, 52);
        assert_eq!(shootback.caster, Some(CharacterId(2)));
        assert_eq!((shootback.from_x, shootback.from_y), (12, 10));
        assert_eq!((shootback.to_x, shootback.to_y), (10, 10));
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 14_100);
    }

    #[test]
    fn fireball_reflect_item_reduces_charges_and_shoots_back() {
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
        target.inventory[0] = Some(ItemId(70));
        let mut reflector = item(70, ItemFlags::USED);
        reflector.template_id = IID_REFLECT_FIREBALL;
        reflector.carried_by = Some(CharacterId(2));
        reflector.driver_data = 100_u32.to_le_bytes().to_vec();
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        world.items.insert(ItemId(70), reflector);
        let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
        world.create_fireball_effect(&caster);

        world.tick_effects();
        world.tick_effects();

        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 30 * POWERSCALE);
        assert_eq!(target.inventory[0], Some(ItemId(70)));
        let reflector = world.items.get(&ItemId(70)).unwrap();
        assert_eq!(read_u32_le_prefix(&reflector.driver_data), 47);
        assert_eq!(reflector.description, "47 units left.");
        let reflected = world.effects.values().next().unwrap();
        assert_eq!(reflected.effect_type, EF_FIREBALL);
        assert_eq!(reflected.strength, 52);
        assert_eq!(reflected.caster, Some(CharacterId(2)));
        assert_eq!((reflected.from_x, reflected.from_y), (12, 10));
        assert_eq!((reflected.to_x, reflected.to_y), (10, 10));
    }

    #[test]
    fn fireball_reflect_item_is_destroyed_when_charges_are_used_up() {
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
        target.inventory[0] = Some(ItemId(71));
        let mut reflector = item(71, ItemFlags::USED);
        reflector.template_id = IID_REFLECT_FIREBALL;
        reflector.carried_by = Some(CharacterId(2));
        reflector.driver_data = 10_u32.to_le_bytes().to_vec();
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        world.items.insert(ItemId(71), reflector);
        let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
        world.create_fireball_effect(&caster);

        world.tick_effects();
        world.tick_effects();

        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 30 * POWERSCALE);
        assert_eq!(target.inventory[0], None);
        assert!(!world.items.contains_key(&ItemId(71)));
        let reflected = world.effects.values().next().unwrap();
        assert_eq!(reflected.effect_type, EF_FIREBALL);
        assert_eq!(reflected.strength, 52);
        assert_eq!(reflected.caster, Some(CharacterId(2)));
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
        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].character_id, CharacterId(1));
        assert_eq!(sounds[0].special.special_type, 30);
    }

    #[test]
    fn ball_effect_respects_runtime_attack_policy() {
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
        target
            .flags
            .insert(CharacterFlags::ALIVE | CharacterFlags::PLAYER);
        target.hp = 30 * POWERSCALE;
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 12, 10);
        let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
        let effect_id = world.create_ball_effect(&caster);

        world.tick_effects_with_attack_policy(|_, _, _, _| false);

        let effect = world.effects.get(&effect_id).unwrap();
        assert_eq!(effect.number_of_enemies, 0);
        assert!(!world
            .effects
            .values()
            .any(|effect| effect.effect_type == EF_STRIKE));
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 30 * POWERSCALE);
        assert!(!target.flags.contains(CharacterFlags::UPDATE));
    }

    #[test]
    fn ball_strike_sound_keeps_legacy_eighth_tick_cadence() {
        let mut world = World {
            tick: Tick(1),
            ..World::default()
        };
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
        world.create_ball_effect(&caster);

        world.tick_effects();

        assert!(world.drain_pending_sound_specials().is_empty());
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
    fn character_fireball_blocks_player_target_without_pk_hate_entry() {
        let mut world = World::default();
        let mut caster = character(1);
        caster
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        let mut target = character(2);
        target
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 15, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::FireballCharacter,
            arg1: 2,
            arg2: 2,
        };

        assert!(world.apply_player_action_setup(&mut player, 2));

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::IDLE);
        assert_eq!(caster.mana, 10 * POWERSCALE);
        assert_eq!(player.action.action, PlayerActionCode::Idle);
    }

    #[test]
    fn character_fireball_allows_player_target_with_pk_hate_entry() {
        let mut world = World::default();
        let mut caster = character(1);
        caster
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Fireball as usize] = 50;
        let mut target = character(2);
        target
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 15, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        assert!(player.add_pk_hate(2));
        player.action = QueuedAction {
            action: PlayerActionCode::FireballCharacter,
            arg1: 2,
            arg2: 2,
        };

        assert!(world.apply_player_action_setup(&mut player, 2));

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::FIREBALL1);
        assert_eq!((caster.act1, caster.act2), (15, 10));
    }

    #[test]
    fn character_ball_blocks_player_target_without_pk_hate_entry() {
        let mut world = World::default();
        let mut caster = character(1);
        caster
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Flash as usize] = 50;
        let mut target = character(2);
        target
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        world.spawn_character(caster, 10, 10);
        world.spawn_character(target, 15, 10);
        let mut player = PlayerRuntime::connected(1, 0);
        player.character_id = Some(CharacterId(1));
        player.action = QueuedAction {
            action: PlayerActionCode::BallCharacter,
            arg1: 2,
            arg2: 2,
        };

        assert!(world.apply_player_action_setup(&mut player, 2));

        let caster = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(caster.action, action::IDLE);
        assert_eq!(caster.mana, 10 * POWERSCALE);
        assert_eq!(player.action.action, PlayerActionCode::Idle);
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
        target.lifeshield = POWERSCALE;
        target.values[0][CharacterValue::Armor as usize] = 20;
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
        assert_eq!(target.hp, 15_200);
        assert_eq!(target.lifeshield, 0);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
        assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
        assert_eq!(target.driver_messages[0].dat1, 1);
        assert_eq!(target.driver_messages[0].dat2, 14_800);
        assert_eq!(
            world.characters[&CharacterId(1)].driver_messages[0].message_type,
            NT_DIDHIT
        );
        let firering_effect = world
            .effects
            .values()
            .find(|effect| effect.effect_type == EF_FIRERING)
            .unwrap();
        assert_eq!(firering_effect.target_character, Some(CharacterId(1)));
        assert_eq!(firering_effect.stop_tick, 257);
        assert_eq!(firering_effect.light, 20);
        assert_eq!(firering_effect.strength, 50);
        let burn_effect = world
            .effects
            .values()
            .find(|effect| effect.effect_type == EF_BURN)
            .unwrap();
        assert_eq!(burn_effect.target_character, Some(CharacterId(2)));
        assert_eq!(burn_effect.stop_tick, 258);
        assert_eq!(burn_effect.light, 20);
        assert_eq!(burn_effect.strength, 0);
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
        assert_eq!(target.values[0][CharacterValue::Speed as usize], -420);
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
        let sounds = world.drain_pending_sound_specials();
        assert!(sounds.iter().any(|sound| sound.special.special_type == 31));
    }

    #[test]
    fn ice_demon_freeze_installs_legacy_curse_spell() {
        let mut world = World::default();
        world.tick = Tick(300);
        let mut caster = character(1);
        caster.name = "Ice Demon".into();
        caster
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::IDEMON);
        caster.mana = 10 * POWERSCALE;
        caster.values[0][CharacterValue::Freeze as usize] = 50;
        caster.values[1][CharacterValue::Demon as usize] = 10;
        let mut target = character(2);
        target.flags.insert(CharacterFlags::ALIVE);
        target.values[0][CharacterValue::Cold as usize] = 3;
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
        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        assert!(world.tick_basic_actions()[0].ok);

        let target = world.characters.get(&CharacterId(2)).unwrap();
        let curse_id = target.inventory[28].unwrap();
        let curse = world.items.get(&curse_id).unwrap();
        assert_eq!(curse.driver, IDR_CURSE);
        assert_eq!(curse.modifier_index[0], CharacterValue::Intelligence as i16);
        assert_eq!(curse.modifier_index[1], CharacterValue::Wisdom as i16);
        assert_eq!(curse.modifier_index[2], CharacterValue::Agility as i16);
        assert_eq!(curse.modifier_index[3], CharacterValue::Strength as i16);
        assert_eq!(curse.modifier_value[..4], [-7, -7, -7, -7]);
        assert_eq!(curse.carried_by, Some(CharacterId(2)));
        assert_eq!(target.values[0][CharacterValue::Intelligence as usize], -7);
        assert_eq!(target.values[0][CharacterValue::Wisdom as usize], -7);
        assert_eq!(target.values[0][CharacterValue::Agility as usize], -7);
        assert_eq!(target.values[0][CharacterValue::Strength as usize], -7);
        assert_eq!(
            u32::from_le_bytes(curse.driver_data[0..4].try_into().unwrap()),
            43_500
        );
        assert_eq!(
            u32::from_le_bytes(curse.driver_data[4..8].try_into().unwrap()),
            300
        );
        let curse_effect = world
            .effects
            .values()
            .find(|effect| effect.effect_type == EF_CURSE)
            .unwrap();
        assert_eq!(curse_effect.target_character, Some(CharacterId(2)));
        assert_eq!(curse_effect.start_tick, 300);
        assert_eq!(curse_effect.stop_tick, 43_500);
        assert_eq!(curse_effect.strength, 7);
        assert_eq!(world.timers.used_timers(), 2);
        assert_eq!(
            world.drain_pending_system_texts(),
            vec![WorldSystemText {
                character_id: CharacterId(2),
                message:
                    "You have been frozen by Ice Demon. You feel like you'll never thaw again."
                        .into(),
            }]
        );
    }

    #[test]
    fn curse_spell_stack_uses_existing_slot_and_caps_strength() {
        let mut world = World::default();
        world.tick = Tick(100);
        let mut target = character(2);
        target.flags.insert(CharacterFlags::ALIVE);
        world.spawn_character(target, 10, 10);

        assert!(world.install_curse_spell(CharacterId(2), 7, 10));
        assert!(world.install_curse_spell(CharacterId(2), 7, 10));

        let target = world.characters.get(&CharacterId(2)).unwrap();
        let curse_id = target.inventory[29].unwrap();
        let curse = world.items.get(&curse_id).unwrap();
        assert_eq!(curse.driver, IDR_CURSE);
        assert_eq!(curse.modifier_value[..4], [-10, -10, -10, -10]);
        assert!(target.inventory[28].is_none());
        assert_eq!(target.values[0][CharacterValue::Intelligence as usize], -10);
        assert_eq!(target.values[0][CharacterValue::Wisdom as usize], -10);
        assert_eq!(target.values[0][CharacterValue::Agility as usize], -10);
        assert_eq!(target.values[0][CharacterValue::Strength as usize], -10);
        let effects: Vec<_> = world
            .effects
            .values()
            .filter(|effect| effect.effect_type == EF_CURSE)
            .collect();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].strength, 10);
        assert_eq!(world.timers.used_timers(), 1);
    }

    #[test]
    fn freeze_completion_succeeds_and_sounds_without_targets() {
        let mut world = World::default();
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.action = action::FREEZE;
        caster.duration = 1;
        caster.values[0][CharacterValue::Freeze as usize] = 50;
        world.spawn_character(caster, 10, 10);

        let completed = world.tick_basic_actions();

        assert_eq!(completed.len(), 1);
        assert!(completed[0].ok);
        let sounds = world.drain_pending_sound_specials();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].character_id, CharacterId(1));
        assert_eq!(sounds[0].special.special_type, 31);
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
        target.lifeshield = POWERSCALE;
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
        assert_eq!(target.lifeshield, POWERSCALE);
        assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
        assert_eq!(target.driver_messages[0].dat1, 1);
        assert_eq!(target.driver_messages[0].dat2, 3_600);
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
        assert!(world.drain_pending_sound_specials().is_empty());
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
        assert!(world.tick_basic_actions()[0].ok);

        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert!(target.inventory[12..30].iter().all(Option::is_none));
        assert!(world.drain_pending_sound_specials().is_empty());
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
    fn finished_alchemy_flask_installs_timed_potion_spell_and_resets_flask() {
        let mut world = World::default();
        world.tick = Tick(2_000);
        let mut character = character(1);
        character
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::ARCH);
        character.inventory[30] = Some(ItemId(7));
        world.add_character(character);

        let mut flask = item(7, ItemFlags::USED | ItemFlags::USE);
        flask.driver = crate::item_driver::IDR_FLASK;
        flask.carried_by = Some(CharacterId(1));
        flask.name = "Magical Potion".to_string();
        flask.description = "A flask containing a magical liquid.".to_string();
        flask.sprite = 50214;
        flask.value = 999;
        flask.needs_class = 8;
        flask.driver_data = vec![2, 3, 1, 10];
        flask.modifier_index = [CharacterValue::Agility as i16, 0, 0, 0, 0];
        flask.modifier_value = [4, 0, 0, 0, 0];
        world.add_item(flask);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_FLASK,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
        );

        assert!(
            matches!(outcome, ItemDriverOutcome::AlchemyFlaskPotion { .. }),
            "unexpected outcome: {outcome:?}"
        );
        let reset_flask = world.items.get(&ItemId(7)).unwrap();
        assert_eq!(reset_flask.name, "Empty Potion");
        assert_eq!(reset_flask.description, "A flask made of glass.");
        assert_eq!(reset_flask.sprite, 10294);
        assert_eq!(reset_flask.driver_data, vec![2]);
        assert_eq!(reset_flask.modifier_index, [0; MAX_MODIFIERS]);
        assert_eq!(reset_flask.modifier_value, [0; MAX_MODIFIERS]);
        assert_eq!(reset_flask.value, 10);
        assert_eq!(reset_flask.needs_class, 0);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[30], Some(ItemId(7)));
        let spell_id = character.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_POTION_SP);
        assert_eq!(spell.modifier_index[0], CharacterValue::Agility as i16);
        assert_eq!(spell.modifier_value[0], 4);
        assert!(!spell.flags.contains(ItemFlags::BEYONDMAXMOD));
        assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(16_400));
        let effect = world.effects.values().next().unwrap();
        assert_eq!(effect.effect_type, EF_POTION);
        assert_eq!(effect.strength, 4);
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
    fn freeze_spell_timer_restores_speed_and_rescales_current_action() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[12] = Some(ItemId(7));
        character.values[0][CharacterValue::Speed as usize] = -420;
        character.duration = 50;
        character.step = 25;
        let mut spell = item(7, ItemFlags::USED);
        spell.driver = IDR_FREEZE;
        spell.carried_by = Some(CharacterId(1));
        spell.modifier_index[0] = CharacterValue::Speed as i16;
        spell.modifier_value[0] = -420;
        spell.driver_data = 110_u32.to_le_bytes().to_vec();
        world.add_character(character);
        world.add_item(spell);

        assert_eq!(world.schedule_existing_spell_timers(), 1);
        world.tick = Tick(110);
        assert!(world.process_due_timers(1).is_empty());

        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[12], None);
        assert_eq!(character.values[0][CharacterValue::Speed as usize], 0);
        assert_eq!(character.duration, 13);
        assert_eq!(character.step, 6);
        assert!(!world.items.contains_key(&ItemId(7)));
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
    fn poison_callback_uses_legacy_hurt_shield_reduction() {
        let mut world = World::default();
        world.tick = Tick(1_000);
        let mut character = character(1);
        character.hp = 10 * POWERSCALE;
        character.lifeshield = POWERSCALE;
        world.add_character(character);
        assert!(world.poison_character(CharacterId(1), 4, 0));

        world.tick = Tick(1_000 + TICKS_PER_SECOND);
        world.process_due_timers(1);

        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.hp, 10 * POWERSCALE - 167);
        assert_eq!(character.lifeshield, POWERSCALE - 166);
        assert!(character.flags.contains(CharacterFlags::UPDATE));
        assert_eq!(character.driver_messages.len(), 1);
        assert_eq!(character.driver_messages[0].dat2, 167);
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
        assert!(character.flags.contains(CharacterFlags::INFRAVISION));
        assert!(!world.items.contains_key(&ItemId(10)));

        world.tick = Tick(14_442);
        world.process_due_timers(1);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert!(!character.flags.contains(CharacterFlags::INFRAVISION));
        assert!(character.flags.contains(CharacterFlags::UPDATE));
    }

    #[test]
    fn oxy_potion_installs_one_minute_oxygen_spell_and_consumes_item() {
        let mut world = World::default();
        world.tick = Tick(77);
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(10));
        world.add_character(character);
        let mut potion = item(10, ItemFlags::USED);
        potion.carried_by = Some(CharacterId(1));
        potion.driver = IDR_OXYPOTION;
        world.items.insert(ItemId(10), potion);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_OXYPOTION,
                item_id: ItemId(10),
                character_id: CharacterId(1),
                spec: 0,
            },
            31,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::OxygenPotion {
                installed: true,
                ..
            }
        ));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        let spell_id = character.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.name, "Oxygen");
        assert_eq!(spell.driver, IDR_OXYGEN);
        assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(1_517));
        assert_eq!(character.inventory[30], None);
        assert!(character.flags.contains(CharacterFlags::OXYGEN));
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert!(character.flags.contains(CharacterFlags::UPDATE));
        assert!(!world.items.contains_key(&ItemId(10)));

        world.tick = Tick(1_517);
        world.process_due_timers(31);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert!(!character.flags.contains(CharacterFlags::OXYGEN));
        assert!(character.flags.contains(CharacterFlags::UPDATE));
    }

    #[test]
    fn lab3_yellow_berry_replaces_existing_oxygen_with_fresh_duration() {
        let mut world = World::default();
        world.tick = Tick(100);
        let mut character = character(1);
        character.inventory[12] = Some(ItemId(12));
        character.inventory[30] = Some(ItemId(10));
        world.add_character(character);

        let mut old_oxygen = item(12, ItemFlags::USED);
        old_oxygen.carried_by = Some(CharacterId(1));
        old_oxygen.driver = IDR_OXYGEN;
        old_oxygen.driver_data = 10_000u32.to_le_bytes().to_vec();
        world.items.insert(ItemId(12), old_oxygen);

        let mut berry = item(10, ItemFlags::USED | ItemFlags::USE);
        berry.carried_by = Some(CharacterId(1));
        berry.driver = IDR_LAB3_PLANT;
        berry.driver_data = vec![5, 2, 3];
        world.items.insert(ItemId(10), berry);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_LAB3_PLANT,
                item_id: ItemId(10),
                character_id: CharacterId(1),
                spec: 0,
            },
            22,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::Lab3YellowBerry {
                duration_ticks,
                installed: true,
                ..
            } if duration_ticks == 24 * TICKS_PER_SECOND
        ));
        assert!(!world.items.contains_key(&ItemId(12)));
        assert!(!world.items.contains_key(&ItemId(10)));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[12], None);
        assert_eq!(character.inventory[30], None);
        let spell_id = character.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_OXYGEN);
        assert_eq!(
            read_spell_expire_tick(&spell.driver_data),
            Some(100 + (24 * TICKS_PER_SECOND) as u32)
        );
        assert!(character.flags.contains(CharacterFlags::OXYGEN));
    }

    #[test]
    fn lab3_brown_berry_installs_timed_underwater_talk_spell() {
        let mut world = World::default();
        world.tick = Tick(200);
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(10));
        world.add_character(character);

        let mut berry = item(10, ItemFlags::USED | ItemFlags::USE);
        berry.carried_by = Some(CharacterId(1));
        berry.driver = IDR_LAB3_PLANT;
        berry.driver_data = vec![11];
        world.items.insert(ItemId(10), berry);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_LAB3_PLANT,
                item_id: ItemId(10),
                character_id: CharacterId(1),
                spec: 0,
            },
            22,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::Lab3BrownBerry {
                duration_ticks,
                installed: true,
                ..
            } if duration_ticks == 10 * TICKS_PER_SECOND
        ));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        let spell_id = character.inventory[29].unwrap();
        let spell = world.items.get(&spell_id).unwrap();
        assert_eq!(spell.driver, IDR_UWTALK);
        assert_eq!(
            read_spell_expire_tick(&spell.driver_data),
            Some(200 + (10 * TICKS_PER_SECOND) as u32)
        );
        assert_eq!(character.inventory[30], None);

        world.tick = Tick(200 + 10 * TICKS_PER_SECOND);
        world.process_due_timers(22);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[29], None);
    }

    #[test]
    fn lab3_white_berry_creates_and_refreshes_decaying_light_item() {
        let mut world = World::default();
        world.tick = Tick(300);
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(10));
        world.add_character(character);

        let mut berry = item(10, ItemFlags::USED | ItemFlags::USE);
        berry.carried_by = Some(CharacterId(1));
        berry.driver = IDR_LAB3_PLANT;
        berry.driver_data = vec![6, 2, 1];
        world.items.insert(ItemId(10), berry);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_LAB3_PLANT,
                item_id: ItemId(10),
                character_id: CharacterId(1),
                spec: 0,
            },
            22,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::Lab3WhiteBerry {
                light_power: 60,
                started_emit: true,
                installed: true,
                ..
            }
        ));
        assert!(!world.items.contains_key(&ItemId(10)));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        let light_id = character.inventory[29].unwrap();
        assert_eq!(character.values[0][CharacterValue::Light as usize], 80);
        let light = world.items.get(&light_id).unwrap();
        assert_eq!(light.driver, IDR_LAB3_PLANT);
        assert_eq!(light.driver_data.first(), Some(&10));
        assert_eq!(light.modifier_index[0], CharacterValue::Light as i16);
        assert_eq!(light.modifier_value[0], 80);

        let mut second = item(20, ItemFlags::USED | ItemFlags::USE);
        second.carried_by = Some(CharacterId(1));
        second.driver = IDR_LAB3_PLANT;
        second.driver_data = vec![6, 1, 0];
        world.items.insert(ItemId(20), second);
        if let Some(character) = world.characters.get_mut(&CharacterId(1)) {
            character.inventory[30] = Some(ItemId(20));
        }

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_LAB3_PLANT,
                item_id: ItemId(20),
                character_id: CharacterId(1),
                spec: 0,
            },
            22,
        );

        match outcome {
            ItemDriverOutcome::Lab3WhiteBerry {
                light_power: 10,
                started_emit: false,
                installed: true,
                ..
            } => {}
            other => panic!("unexpected whiteberry refresh outcome: {other:?}"),
        }
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[29], Some(light_id));
        assert_eq!(character.values[0][CharacterValue::Light as usize], 90);
        assert_eq!(world.items.get(&light_id).unwrap().modifier_value[0], 90);
    }

    #[test]
    fn lab3_whiteberry_light_timer_decays_and_destroys_low_light() {
        let mut world = World::default();
        world.tick = Tick(10);
        let mut character = character(1);
        character.inventory[12] = Some(ItemId(12));
        character.values[0][CharacterValue::Light as usize] = 12;
        world.add_character(character);
        let mut light = item(12, ItemFlags::USED);
        light.carried_by = Some(CharacterId(1));
        light.driver = IDR_LAB3_PLANT;
        light.driver_data = vec![10, 0, 0, 12];
        light.modifier_index[0] = CharacterValue::Light as i16;
        light.modifier_value[0] = 12;
        world.items.insert(ItemId(12), light);

        assert!(world.schedule_item_driver_timer_with_context(
            ItemId(12),
            CharacterId(0),
            20 * TICKS_PER_SECOND,
            true,
        ));
        world.tick = Tick(10 + 20 * TICKS_PER_SECOND);
        let outcomes = world.process_due_timers(22);
        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::Lab3WhiteBerryLightTick {
                item_id: ItemId(12),
                destroyed: false,
            }]
        );
        assert_eq!(world.items.get(&ItemId(12)).unwrap().modifier_value[0], 9);
        assert_eq!(
            world.characters.get(&CharacterId(1)).unwrap().values[0]
                [CharacterValue::Light as usize],
            9
        );

        world.tick = Tick(10 + 40 * TICKS_PER_SECOND);
        let outcomes = world.process_due_timers(22);
        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::Lab3WhiteBerryLightTick {
                item_id: ItemId(12),
                destroyed: true,
            }]
        );
        assert!(!world.items.contains_key(&ItemId(12)));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[12], None);
        assert_eq!(character.values[0][CharacterValue::Light as usize], 0);
    }

    #[test]
    fn existing_driver_spell_items_refresh_legacy_character_flags() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[12] = Some(ItemId(12));
        character.inventory[13] = Some(ItemId(13));
        character.inventory[30] = Some(ItemId(30));
        world.add_character(character);

        let mut nonomagic = item(12, ItemFlags::USED);
        nonomagic.carried_by = Some(CharacterId(1));
        nonomagic.driver = IDR_NONOMAGIC;
        nonomagic.driver_data = 100u32.to_le_bytes().to_vec();
        world.items.insert(ItemId(12), nonomagic);

        let mut oxygen = item(13, ItemFlags::USED);
        oxygen.carried_by = Some(CharacterId(1));
        oxygen.driver = IDR_OXYGEN;
        oxygen.driver_data = 200u32.to_le_bytes().to_vec();
        world.items.insert(ItemId(13), oxygen);

        let mut ignored_infravision = item(30, ItemFlags::USED);
        ignored_infravision.carried_by = Some(CharacterId(1));
        ignored_infravision.driver = IDR_INFRARED;
        ignored_infravision.driver_data = 300u32.to_le_bytes().to_vec();
        world.items.insert(ItemId(30), ignored_infravision);

        assert_eq!(world.schedule_existing_spell_timers(), 3);
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert!(character.flags.contains(CharacterFlags::NONOMAGIC));
        assert!(character.flags.contains(CharacterFlags::OXYGEN));
        assert!(!character.flags.contains(CharacterFlags::INFRAVISION));
    }

    #[test]
    fn lizard_flower_mixer_updates_carried_flower_and_consumes_cursor() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(10));
        character.cursor_item = Some(ItemId(11));
        world.add_character(character);

        let mut carried = item(10, ItemFlags::USED | ItemFlags::USE);
        carried.carried_by = Some(CharacterId(1));
        carried.driver = IDR_LIZARDFLOWER;
        carried.driver_data = vec![1];
        carried.sprite = 11190;
        world.items.insert(ItemId(10), carried);

        let mut cursor = item(11, ItemFlags::USED | ItemFlags::USE);
        cursor.carried_by = Some(CharacterId(1));
        cursor.driver = IDR_LIZARDFLOWER;
        cursor.driver_data = vec![6];
        cursor.sprite = 11191;
        world.items.insert(ItemId(11), cursor);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_LIZARDFLOWER,
                item_id: ItemId(10),
                character_id: CharacterId(1),
                spec: 0,
            },
            31,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::LizardFlowerMixed {
                combined_bits: 7,
                complete: true,
                bottle_message: true,
                ..
            }
        ));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.cursor_item, None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert!(!world.items.contains_key(&ItemId(11)));
        let item = world.items.get(&ItemId(10)).unwrap();
        assert_eq!(item.driver_data[0], 7);
        assert_eq!(item.sprite, 11188);
        assert_eq!(item.driver, IDR_OXYPOTION);
        assert_eq!(item.name, "Scuba Potion");
        assert_eq!(item.description, "A bubbly fluid in a nice bottle.");
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
        target.lifeshield = POWERSCALE;
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
        assert_eq!(target.lifeshield, 0);
        assert!(target.hp <= 0);
        assert!(target.flags.contains(CharacterFlags::UPDATE));
        assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
        assert_eq!(
            world.characters[&CharacterId(1)].driver_messages[0].message_type,
            NT_DIDHIT
        );
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

    #[test]
    fn action_tick_attack_policy_can_block_area_spell_targets() {
        let mut world = World::default();
        world.tick = Tick(500);
        let mut caster = character(1);
        caster.flags.insert(CharacterFlags::PLAYER);
        caster.mana = 100 * POWERSCALE;
        caster.values[0][CharacterValue::Mana as usize] = 100;
        caster.values[0][CharacterValue::Pulse as usize] = 200;
        let mut target = character(2);
        target
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        target.hp = 10 * POWERSCALE;
        target.lifeshield = POWERSCALE;
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

        assert!(world.apply_player_action_setup(&mut player, 2));
        world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
        let completed =
            world.tick_basic_actions_with_attack_policy(|_caster_id, _caster, target, _map| {
                target.id != CharacterId(2)
            });

        assert!(completed[0].ok);
        let target = world.characters.get(&CharacterId(2)).unwrap();
        assert_eq!(target.hp, 10 * POWERSCALE);
        assert_eq!(target.lifeshield, POWERSCALE);
        assert!(target.driver_messages.is_empty());
        assert!(world
            .effects
            .values()
            .any(|effect| effect.effect_type == EF_PULSE && effect.x == 10 && effect.y == 10));
        assert!(!world
            .effects
            .values()
            .any(|effect| effect.effect_type == EF_PULSEBACK));
    }

    #[test]
    fn tile_special_check_drowns_player_without_oxygen_on_underwater_slowdeath() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.hp = 1_000;
        assert!(world.spawn_character(player, 10, 10));
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::SLOWDEATH | MapFlags::UNDERWATER);

        let outcome = world.tile_special_check(CharacterId(1));

        assert_eq!(outcome.damage, 50);
        assert_eq!(outcome.bubble_effect_id, None);
        assert_eq!(outcome.sound_type, None);
        let player = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(player.hp, 950);
        assert!(player.flags.contains(CharacterFlags::UPDATE));
    }

    #[test]
    fn tile_special_check_creates_legacy_bubble_cadence_for_oxygen_player() {
        let mut world = World::default();
        world.tick.0 = 40;
        let mut player = character(1);
        player
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::OXYGEN);
        player.hp = 1_000;
        assert!(world.spawn_character(player, 10, 10));
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::SLOWDEATH | MapFlags::UNDERWATER);

        let outcome = world.tile_special_check(CharacterId(1));

        let effect_id = outcome.bubble_effect_id.unwrap();
        assert_eq!(outcome.damage, 0);
        assert_eq!(outcome.sound_type, Some(44));
        let player = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(player.hp, 1_000);
        let effect = world.effects.get(&effect_id).unwrap();
        assert_eq!(effect.effect_type, EF_BUBBLE);
        assert_eq!(effect.strength, 45);
        assert_eq!(effect.stop_tick - effect.start_tick, 1);
        assert!(world
            .map
            .tile(10, 10)
            .unwrap()
            .effects
            .contains(&(effect_id as u16)));
    }

    #[test]
    fn tile_special_check_applies_non_underwater_slowdeath_damage() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.hp = 1_000;
        assert!(world.spawn_character(player, 10, 10));
        let tile = world.map.tile_mut(10, 10).unwrap();
        tile.flags.insert(MapFlags::SLOWDEATH);
        tile.ground_sprite = 59706;

        let outcome = world.tile_special_check(CharacterId(1));

        assert_eq!(outcome.damage, 250);
        assert_eq!(outcome.bubble_effect_id, None);
        assert_eq!(outcome.sound_type, Some(66));
        let player = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(player.hp, 750);
        assert!(player.flags.contains(CharacterFlags::UPDATE));
    }

    #[test]
    fn tick_basic_actions_runs_tile_specials_before_skipping_idle_players() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.hp = 1_000;
        assert!(world.spawn_character(player, 10, 10));
        world
            .map
            .tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::SLOWDEATH | MapFlags::UNDERWATER);

        assert!(world.tick_basic_actions().is_empty());

        let player = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(player.hp, 950);
    }

    #[test]
    fn staffer2_animation_book_teleports_without_granting_core_exp() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.level = 60;
        assert!(world.spawn_character(player, 10, 10));
        let mut book = item(8, ItemFlags::USED | ItemFlags::USE);
        book.driver = IDR_STAFFER2;
        book.driver_data = vec![6];
        world.add_item(book);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER2,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            29,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::StafferAnimationBook {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                exp_added: 177_168,
            }
        );
        let player = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((player.x, player.y), (25, 114));
        assert_eq!(player.exp, 0);
    }

    #[test]
    fn staffer2_mine_dig_clears_sightblock_then_opens_and_schedules_restore() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.endurance = 10 * POWERSCALE;
        world.add_character(player);
        let mut mine = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::SIGHTBLOCK);
        mine.driver = IDR_STAFFER2;
        mine.driver_data = vec![2, 0, 0, 2];
        mine.sprite = 15072;
        assert!(world.map.set_item_map(&mut mine, 10, 10));
        world.add_item(mine);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER2,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            29,
        );

        assert!(matches!(outcome, ItemDriverOutcome::StafferMineDig { .. }));
        assert!(!world
            .map
            .tile(10, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TSIGHTBLOCK));
        assert!(!world
            .items
            .get(&ItemId(8))
            .unwrap()
            .flags
            .contains(ItemFlags::SIGHTBLOCK));

        for _ in 0..5 {
            world.execute_item_driver_request(
                ItemDriverRequest::Driver {
                    driver: IDR_STAFFER2,
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                29,
            );
        }
        let tile = world.map.tile(10, 10).unwrap();
        assert_eq!(tile.item, 0);
        assert!(!tile.flags.contains(MapFlags::TMOVEBLOCK));
        let item = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(item.driver_data[3], 8);
        assert!(item.flags.contains(ItemFlags::VOID));
        assert!(!item.flags.contains(ItemFlags::USE));
    }

    #[test]
    fn staffer2_mine_timer_restores_opened_mine_wall() {
        let mut world = World::default();
        let mut mine = item(8, ItemFlags::USED | ItemFlags::VOID);
        mine.driver = IDR_STAFFER2;
        mine.driver_data = vec![2, 0, 0, 8, 1];
        mine.sprite = 15078;
        mine.x = 10;
        mine.y = 10;
        world.add_item(mine);

        assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));
        world.tick = Tick(1);
        let outcomes = world.process_due_timers(29);

        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::StafferMineTimer { item_id: ItemId(8) }]
        );
        let tile = world.map.tile(10, 10).unwrap();
        assert_eq!(tile.item, 8);
        assert!(tile
            .flags
            .contains(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK));
        let item = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(item.driver_data[3], 0);
        assert!(item.flags.contains(ItemFlags::USE | ItemFlags::SIGHTBLOCK));
        assert!(!item.flags.contains(ItemFlags::VOID));
        assert_eq!(item.sprite, 15070);
    }

    #[test]
    fn staffer2_block_move_pushes_block_and_timer_returns_home() {
        let mut world = World::default();
        world.tick = Tick(10);
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.dir = Direction::Right as u8;
        world.add_character(player);
        let mut block = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
        block.driver = IDR_STAFFER2;
        block.driver_data = vec![3];
        assert!(world.map.set_item_map(&mut block, 10, 10));
        world.add_item(block);
        world.map.tile_mut(11, 10).unwrap().ground_sprite = 20291;

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER2,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            29,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::StafferBlockMove { .. }
        ));
        assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
        assert_eq!(world.map.tile(11, 10).unwrap().item, 8);
        assert_eq!(world.items.get(&ItemId(8)).unwrap().x, 11);

        world.tick = Tick(10 + TICKS_PER_SECOND * 60 * 3);
        assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));
        world.advance();
        let outcomes = world.process_due_timers(29);
        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::StafferBlockTimer { item_id: ItemId(8) }]
        );
        assert_eq!(world.map.tile(11, 10).unwrap().item, 0);
        assert_eq!(world.map.tile(10, 10).unwrap().item, 8);
        assert_eq!(world.items.get(&ItemId(8)).unwrap().x, 10);
    }

    #[test]
    fn staffer2_block_move_reports_blocked_target() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.dir = Direction::Right as u8;
        world.add_character(player);
        let mut block = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
        block.driver = IDR_STAFFER2;
        block.driver_data = vec![3];
        assert!(world.map.set_item_map(&mut block, 10, 10));
        world.add_item(block);
        world.map.tile_mut(11, 10).unwrap().ground_sprite = 30000;

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER2,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            29,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::StafferBlockBlocked {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(world.map.tile(10, 10).unwrap().item, 8);
        assert_eq!(world.map.tile(11, 10).unwrap().item, 0);
    }

    #[test]
    fn caligar_weight_move_pushes_weight_and_timer_returns_home() {
        let mut world = World::default();
        world.tick = Tick(10);
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.dir = Direction::Right as u8;
        world.add_character(player);
        let mut weight = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
        weight.driver = IDR_CALIGAR;
        weight.driver_data = vec![2];
        assert!(world.map.set_item_map(&mut weight, 10, 10));
        world.add_item(weight);
        world.map.tile_mut(11, 10).unwrap().ground_sprite = 20797;

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_CALIGAR,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            36,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::CaligarWeightMove { .. }
        ));
        assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
        assert_eq!(world.map.tile(11, 10).unwrap().item, 8);
        let moved = world.items.get(&ItemId(8)).unwrap();
        assert_eq!((moved.x, moved.y), (11, 10));
        assert_eq!(
            u32::from_le_bytes(moved.driver_data[4..8].try_into().unwrap()),
            10
        );
        assert_eq!(
            u16::from_le_bytes(moved.driver_data[8..10].try_into().unwrap()),
            10
        );
        assert_eq!(
            u16::from_le_bytes(moved.driver_data[10..12].try_into().unwrap()),
            10
        );

        world.tick = Tick(10 + TICKS_PER_SECOND * 60 * 5 + 1);
        assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));
        world.advance();
        let outcomes = world.process_due_timers(36);
        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::CaligarWeightTimer { item_id: ItemId(8) }]
        );
        assert_eq!(world.map.tile(11, 10).unwrap().item, 0);
        assert_eq!(world.map.tile(10, 10).unwrap().item, 8);
        assert_eq!(world.items.get(&ItemId(8)).unwrap().x, 10);
    }

    #[test]
    fn caligar_weight_move_reports_blocked_or_bad_floor_target() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.dir = Direction::Right as u8;
        world.add_character(player);
        let mut weight = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
        weight.driver = IDR_CALIGAR;
        weight.driver_data = vec![4];
        assert!(world.map.set_item_map(&mut weight, 10, 10));
        world.add_item(weight);
        world.map.tile_mut(11, 10).unwrap().ground_sprite = 30000;

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_CALIGAR,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            36,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::CaligarWeightBlocked {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(world.map.tile(10, 10).unwrap().item, 8);
        assert_eq!(world.map.tile(11, 10).unwrap().item, 0);
    }

    #[test]
    fn caligar_weight_door_requires_lock_weights_from_south() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        assert!(world.spawn_character(player, 10, 11));
        let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
        door.driver = IDR_CALIGAR;
        door.driver_data = vec![3];
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_CALIGAR,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            36,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::CaligarWeightDoorLocked {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(world.characters.get(&CharacterId(1)).unwrap().x, 10);
        assert_eq!(world.characters.get(&CharacterId(1)).unwrap().y, 11);
    }

    #[test]
    fn caligar_weight_door_teleports_to_opposite_side_and_reverses_facing() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.dir = Direction::Down as u8;
        assert!(world.spawn_character(player, 10, 9));
        let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
        door.driver = IDR_CALIGAR;
        door.driver_data = vec![3];
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_CALIGAR,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            36,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::CaligarWeightDoor {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        let player = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((player.x, player.y), (10, 11));
        assert_eq!(player.dir, Direction::Up as u8);
        assert_eq!(world.map.tile(10, 9).unwrap().character, 0);
        assert_eq!(world.map.tile(10, 11).unwrap().character, 1);
    }

    #[test]
    fn caligar_weight_door_reports_busy_target() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        assert!(world.spawn_character(player, 10, 9));
        assert!(world.spawn_character(character(2), 10, 11));
        let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
        door.driver = IDR_CALIGAR;
        door.driver_data = vec![3];
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_CALIGAR,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            36,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::CaligarWeightDoorBusy {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(world.characters.get(&CharacterId(1)).unwrap().y, 9);
    }

    #[test]
    fn caligar_skelly_door_teleports_to_opposite_side_and_reverses_facing() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.dir = Direction::Right as u8;
        assert!(world.spawn_character(player, 9, 10));
        let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
        door.driver = IDR_CALIGAR;
        door.driver_data = vec![12, 2];
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        let outcome = world.apply_caligar_skelly_door(ItemId(8), CharacterId(1), 2);

        assert_eq!(
            outcome,
            ItemDriverOutcome::CaligarSkellyDoor {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                door_index: 2,
            }
        );
        let player = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((player.x, player.y), (11, 10));
        assert_eq!(player.dir, Direction::Left as u8);
        assert_eq!(world.map.tile(9, 10).unwrap().character, 0);
        assert_eq!(world.map.tile(11, 10).unwrap().character, 1);
    }

    #[test]
    fn caligar_skelly_door_reports_busy_target() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        assert!(world.spawn_character(player, 9, 10));
        assert!(world.spawn_character(character(2), 11, 10));
        let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
        door.driver = IDR_CALIGAR;
        door.driver_data = vec![12, 1];
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        assert_eq!(
            world.apply_caligar_skelly_door(ItemId(8), CharacterId(1), 1),
            ItemDriverOutcome::CaligarSkellyDoorBusy {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(world.characters.get(&CharacterId(1)).unwrap().x, 9);
    }

    #[test]
    fn schedule_existing_light_timers_includes_caligar_weights() {
        let mut world = World::default();
        let mut weight = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
        weight.driver = IDR_CALIGAR;
        weight.driver_data = vec![2];
        weight.x = 10;
        weight.y = 10;
        world.add_item(weight);

        assert_eq!(world.schedule_existing_light_timers(), 1);
        world.tick = Tick(1);
        let outcomes = world.process_due_timers(36);
        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::CaligarWeightTimer { item_id: ItemId(8) }]
        );
    }

    #[test]
    fn staffer2_spec_door_opens_schedules_and_timer_closes() {
        let mut world = World::default();
        world.map = MapGrid::new(300, 300);
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        world.add_character(player);
        let mut door = item(
            8,
            ItemFlags::USED
                | ItemFlags::USE
                | ItemFlags::MOVEBLOCK
                | ItemFlags::SIGHTBLOCK
                | ItemFlags::DOOR,
        );
        door.driver = IDR_STAFFER2;
        door.driver_data = vec![4, 0, 0, 0, 0, 0];
        door.sprite = 1200;
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);
        let mut marker = item(9, ItemFlags::USED);
        marker.sprite = 21203;
        assert!(world.map.set_item_map(&mut marker, 51, 234));
        world.add_item(marker);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER2,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            29,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::StafferSpecDoorToggle {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                kind: 4,
            }
        );
        let door = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(door.driver_data[1], 1);
        assert_eq!(door.driver_data[39], 1);
        assert_eq!(door.sprite, 1201);
        assert!(!door
            .flags
            .contains(ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK));
        assert!(!world
            .map
            .tile(10, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR));

        world.tick = Tick(TICKS_PER_SECOND * 10);
        let outcomes = world.process_due_timers(29);

        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::StafferSpecDoorToggle {
                item_id: ItemId(8),
                character_id: CharacterId(0),
                kind: 4,
            }]
        );
        let door = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(door.driver_data[1], 0);
        assert_eq!(door.driver_data[39], 0);
        assert_eq!(door.sprite, 1200);
        assert!(door
            .flags
            .contains(ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK));
        assert!(world
            .map
            .tile(10, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR));
    }

    #[test]
    fn staffer2_spec_door_reports_locked_without_marker_item() {
        let mut world = World::default();
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        world.add_character(player);
        let mut door = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
        door.driver = IDR_STAFFER2;
        door.driver_data = vec![5, 0, 0, 0, 0, 0];
        assert!(world.map.set_item_map(&mut door, 10, 10));
        world.add_item(door);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER2,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            29,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::StafferSpecDoorLocked {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(world.items.get(&ItemId(8)).unwrap().driver_data[1], 0);
    }

    #[test]
    fn world_applies_clanjewel_expiry_to_carried_inventory_item() {
        let mut world = World::default();
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(8));
        world.add_character(character);

        let mut jewel = item(8, ItemFlags::USED);
        jewel.name = "Clan Jewel".into();
        jewel.driver = crate::item_driver::IDR_CLANJEWEL;
        jewel.carried_by = Some(CharacterId(1));
        world.add_item(jewel);

        let outcome = world.apply_item_driver_outcome(
            ItemDriverOutcome::ClanJewelExpired {
                item_id: ItemId(8),
                character_id: Some(CharacterId(1)),
                item_name: crate::item_driver::outcome_item_name("Clan Jewel"),
            },
            30,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::ClanJewelExpired { .. }
        ));
        assert!(!world.items.contains_key(&ItemId(8)));
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!(character.inventory[30], None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
    }

    #[test]
    fn world_applies_clanspawn_exit_same_area_rest_teleport() {
        let mut world = World::default();
        let mut exit = item(8, ItemFlags::USED | ItemFlags::USE);
        exit.driver = crate::item_driver::IDR_CLANSPAWNEXIT;
        assert!(world.map.set_item_map(&mut exit, 10, 10));
        world.add_item(exit);
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.rest_area = 30;
        player.rest_x = 12;
        player.rest_y = 13;
        assert!(world.spawn_character(player, 10, 10));

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_CLANSPAWNEXIT,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            30,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::ClanSpawnExit {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                area_id: 30,
                x: 12,
                y: 13,
            }
        );
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (12, 13));
        assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
        assert_eq!(world.map.tile(12, 13).unwrap().character, 1);
    }

    #[test]
    fn world_applies_clanspawn_exit_busy_target_feedback_outcome() {
        let mut world = World::default();
        let mut exit = item(8, ItemFlags::USED | ItemFlags::USE);
        exit.driver = crate::item_driver::IDR_CLANSPAWNEXIT;
        assert!(world.map.set_item_map(&mut exit, 10, 10));
        world.add_item(exit);
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.rest_area = 30;
        player.rest_x = 12;
        player.rest_y = 13;
        assert!(world.spawn_character(player, 10, 10));
        let mut blocker_id = 2;
        for y in 12..=14 {
            for x in 11..=13 {
                assert!(world.spawn_character(character(blocker_id), x, y));
                blocker_id += 1;
            }
        }

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_CLANSPAWNEXIT,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            30,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::ClanSpawnExitBusy {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        let character = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((character.x, character.y), (10, 10));
    }

    #[test]
    fn world_applies_area14_trapdoor_stepback_open_and_timer_close() {
        let mut world = World::default();
        world.map = MapGrid::new(20, 20);
        let mut trapdoor = item(8, ItemFlags::USED | ItemFlags::USE);
        trapdoor.driver = crate::item_driver::IDR_TRAPDOOR;
        assert!(world.map.set_item_map(&mut trapdoor, 10, 10));
        world.add_item(trapdoor);

        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.dir = Direction::Right as u8;
        assert!(world.spawn_character(player, 10, 10));

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_TRAPDOOR,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            14,
        );

        assert!(matches!(outcome, ItemDriverOutcome::TrapdoorOpen { .. }));
        let player = world.characters.get(&CharacterId(1)).unwrap();
        assert_eq!((player.x, player.y), (9, 10));
        let trapdoor = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(trapdoor.driver_data[0], 1);
        assert_eq!(trapdoor.sprite, 1);
        assert!(world
            .map
            .tile(10, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));
        assert_eq!(world.timers.used_timers(), 1);
        assert_eq!(
            world.drain_pending_system_texts(),
            vec![WorldSystemText {
                character_id: CharacterId(1),
                message: "A trapdoor opens under your feet, but you manage to jump back in time."
                    .to_string(),
            }]
        );

        world.tick.0 = TICKS_PER_SECOND * 6;
        let outcomes = world.process_due_timers(14);
        assert!(matches!(
            outcomes.as_slice(),
            [ItemDriverOutcome::TrapdoorClose { item_id: ItemId(8) }]
        ));
        let trapdoor = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(trapdoor.driver_data[0], 0);
        assert_eq!(trapdoor.sprite, 0);
        assert!(!world
            .map
            .tile(10, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));
    }

    #[test]
    fn world_applies_area14_trapdoor_steelbar_block() {
        let mut world = World::default();
        world.map = MapGrid::new(20, 20);
        let mut trapdoor = item(8, ItemFlags::USED | ItemFlags::USE);
        trapdoor.driver = crate::item_driver::IDR_TRAPDOOR;
        assert!(world.map.set_item_map(&mut trapdoor, 10, 10));
        world.add_item(trapdoor);

        let mut player = character(1);
        player.cursor_item = Some(ItemId(9));
        assert!(world.spawn_character(player, 9, 10));
        let mut steelbar = item(9, ItemFlags::USED);
        steelbar.template_id = crate::item_driver::IID_AREA14_STEELBAR;
        steelbar.carried_by = Some(CharacterId(1));
        world.add_item(steelbar);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_TRAPDOOR,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            14,
        );

        assert!(matches!(outcome, ItemDriverOutcome::TrapdoorBlocked { .. }));
        let trapdoor = world.items.get(&ItemId(8)).unwrap();
        assert_eq!(trapdoor.driver_data[0], 2);
        assert_eq!(trapdoor.sprite, 2);
        assert!(!world.items.contains_key(&ItemId(9)));
        assert_eq!(
            world.characters.get(&CharacterId(1)).unwrap().cursor_item,
            None
        );
    }

    #[test]
    fn world_applies_area14_gastrap_damage_foreground_and_timers() {
        let mut world = World::default();
        world.map = MapGrid::new(20, 20);
        let mut trap = item(8, ItemFlags::USED | ItemFlags::USE);
        trap.driver = crate::item_driver::IDR_GASTRAP;
        trap.driver_data = vec![2, 0];
        assert!(world.map.set_item_map(&mut trap, 10, 10));
        world.add_item(trap);
        world.map.tile_mut(11, 10).unwrap().foreground_sprite = 15300;

        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.hp = 10 * POWERSCALE;
        assert!(world.spawn_character(player, 10, 10));

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_GASTRAP,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            14,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::GasTrapPulse {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                power: 2,
                schedule_initial_trigger: true,
                schedule_animation: true,
            }
        );
        assert_eq!(world.items.get(&ItemId(8)).unwrap().driver_data[1], 1);
        assert_eq!(world.map.tile(11, 10).unwrap().foreground_sprite, 15301);
        assert!(world.characters.get(&CharacterId(1)).unwrap().hp < 10 * POWERSCALE);
        assert_eq!(world.timers.used_timers(), 2);
    }

    #[test]
    fn world_applies_area14_gastrap_timer_animation_reset() {
        let mut world = World::default();
        world.map = MapGrid::new(20, 20);
        world.add_character(character(0));
        let mut trap = item(8, ItemFlags::USED | ItemFlags::USE);
        trap.driver = crate::item_driver::IDR_GASTRAP;
        trap.driver_data = vec![2, 8];
        assert!(world.map.set_item_map(&mut trap, 10, 10));
        world.add_item(trap);
        world.map.tile_mut(10, 9).unwrap().foreground_sprite = 15318;
        assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));

        world.tick = Tick(1);
        let outcomes = world.process_due_timers(14);

        assert_eq!(
            outcomes,
            vec![ItemDriverOutcome::GasTrapPulse {
                item_id: ItemId(8),
                character_id: CharacterId(0),
                power: 2,
                schedule_initial_trigger: false,
                schedule_animation: false,
            }]
        );
        assert_eq!(world.items.get(&ItemId(8)).unwrap().driver_data[1], 0);
        assert_eq!(world.map.tile(10, 9).unwrap().foreground_sprite, 15318);
        assert_eq!(world.timers.used_timers(), 0);
    }

    #[test]
    fn world_randomshrine_key_context_scans_inventory_and_cursor() {
        let mut world = World::default();
        world.map = MapGrid::new(20, 20);
        let mut shrine = item(8, ItemFlags::USED | ItemFlags::USE);
        shrine.driver = crate::item_driver::IDR_RANDOMSHRINE;
        shrine.driver_data = vec![53, 17];
        assert!(world.map.set_item_map(&mut shrine, 10, 10));
        world.add_item(shrine);

        let mut player = character(1);
        player.inventory[30] = Some(ItemId(9));
        assert!(world.spawn_character(player, 9, 10));
        let mut key = item(9, ItemFlags::USED);
        key.template_id = crate::item_driver::IID_AREA14_SHRINEKEY;
        key.driver_data = vec![17];
        key.carried_by = Some(CharacterId(1));
        world.add_item(key);

        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_RANDOMSHRINE,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            14,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::RandomShrineUse {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                shrine_type: 53,
                level: 17,
                kind: crate::item_driver::RandomShrineKind::Security,
            }
        );

        world.items.get_mut(&ItemId(9)).unwrap().driver_data[0] = 18;
        let outcome = world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: crate::item_driver::IDR_RANDOMSHRINE,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            14,
        );

        assert!(matches!(
            outcome,
            ItemDriverOutcome::RandomShrineNeedsKey { .. }
        ));
    }

    #[test]
    fn edemon_gate_spawn_slots_validate_character_serial_like_c() {
        let mut world = World::default();
        let mut gate = item(7, ItemFlags::USED);
        gate.driver = IDR_EDEMONGATE;
        gate.driver_data = vec![0];
        world.add_item(gate);

        assert!(world.apply_edemon_gate_spawn_result(ItemId(7), 0, CharacterId(2), 55));
        let mut demon = character(2);
        demon.serial = 55;
        assert!(world.spawn_character(demon, 62, 157));

        let context = world.edemon_gate_spawn_context(ItemId(7)).unwrap();
        assert_eq!(context.slot, 1);
        assert_eq!((context.x, context.y), (62, 164));

        world.characters.get_mut(&CharacterId(2)).unwrap().serial = 56;
        let context = world.edemon_gate_spawn_context(ItemId(7)).unwrap();
        assert_eq!(context.slot, 0);
        assert_eq!((context.x, context.y), (62, 157));
    }

    fn character(id: u32) -> Character {
        Character {
            id: CharacterId(id),
            serial: id,
            name: "Character".into(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
            c1: 0,
            c2: 0,
            c3: 0,
            driver: 0,
            group: 0,
            clan: 0,
            clan_rank: 0,
            clan_serial: 0,
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
            regen_ticker: 0,
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
