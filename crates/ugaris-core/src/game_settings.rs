use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{entity::POWERSCALE, legacy::SAY_DIST, tick::TICKS_PER_SECOND};

pub const SP_LOTS_CONST: i32 = 5000;
pub const SP_MANY_CONST: i32 = 1000;
pub const SP_SOME_CONST: i32 = 250;
pub const SP_FEW_CONST: i32 = 50;
pub const SP_RARE_CONST: i32 = 10;
pub const SP_ULTRA_CONST: i32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSettings {
    pub item_decay_time: i32,
    pub player_body_decay_time: i32,
    pub npc_body_decay_time: i32,
    pub npc_body_decay_time_area32: i32,
    pub npc_respawn_timer: i32,
    pub lagout_time: i32,
    pub regen_time: i32,
    pub sewer_item_respawn_time: i32,
    pub exp_modifier: f64,
    pub hardcore_military_exp_bonus: f64,
    pub hardcore_exp_bonus: f64,
    pub hardcore_kill_exp_bonus: f64,
    pub holler_dist: i32,
    pub shout_dist: i32,
    pub say_dist: i32,
    pub emote_dist: i32,
    pub quietsay_dist: i32,
    pub whisper_dist: i32,
    pub holler_cost: i32,
    pub shout_cost: i32,
    pub special_item_drop_multiplier: f64,
    pub sp_lots: i32,
    pub sp_many: i32,
    pub sp_some: i32,
    pub sp_few: i32,
    pub sp_rare: i32,
    pub sp_ultra: i32,
    pub jail_x: i32,
    pub jail_y: i32,
    pub jail_area: i32,
    pub aston_x: i32,
    pub aston_y: i32,
    pub aston_area: i32,
    pub base_orb_respawn_time_days: i32,
    pub base_anti_orb_respawn_time_days: i32,
    pub max_jewel_count: i32,
    pub tunnel_exp_base_value_divider: f64,
    pub tunnel_mill_exp_base_value: i32,
    pub rare_golem_chance: i32,
    pub max_silver_golem_type: i32,
    pub normal_drop_chance: i32,
    pub rare_drop_chance: i32,
    pub rare_drop_multiplier: f32,
    pub base_drop_multiplier: i32,
    pub level_divisor: i32,
    pub golem_inventory_slot: i32,
    pub rare_golem_level_boost: i32,
    pub rare_golem_hp_multiplier: i32,
    pub mining_silver_chance_base: i32,
    pub mining_gold_chance_base: i32,
    pub mining_golem_chance_base: i32,
    pub mining_orb_chance_base: i32,
    pub mining_cavein_chance_base: i32,
    pub mining_artifact_chance_base: i32,
    pub mining_silver_gold_multiplier: f64,
    pub mining_cavein_multiplier: f64,
    pub mining_golem_event_multiplier: f64,
    pub mining_artifact_multiplier: f64,
    pub dungeon_time: i32,
    pub branfo_exp_base: i32,
    pub bran_exp_base: i32,
    pub demon_lord_door_after_solve_access_time: i32,
    pub solve_max_divisor: i32,
    pub drop_prob_low_level: i32,
    pub drop_prob_mid_level: i32,
    pub drop_prob_high_level: i32,
    pub demon_power_deduction: i32,
    pub max_visible_pents: i32,
    pub demon_inventory_slot: i32,
    pub pentagram_value_multiplier: i32,
    pub pentagram_worth_divisor: i32,
    pub lucky_pentagram_chance: i32,
    pub power_increment: i32,
    pub max_power_level: i32,
    pub max_training_power: i32,
    pub random_spawn_chance: i32,
    pub activation_spawn_count: i32,
    pub spawn_count_level_threshold: i32,
    pub max_spawn_low_level: i32,
    pub max_spawn_high_level: i32,
    pub item_drop_equipment_threshold: i32,
    pub item_drop_bronze_threshold: i32,
    pub item_drop_silver_threshold: i32,
    pub equipment_type_count: i32,
    pub tester_movement_range: i32,
    pub exp_level_divisor_primary: i32,
    pub exp_solve_multiplier: f64,
    pub exp_level_divisor_secondary: i32,
    pub exp_clan_reflection_multiplier: f64,
    pub max_clan_bonus_percent: i32,
    pub tester_heal_threshold: f64,
    /// C `modify_movement_speed`'s resolved outdoor weather multiplier
    /// (`module/weather/weather.c:477-493`), refreshed every tick by
    /// `ugaris-server`'s weather module from its own `WeatherEffectData`
    /// table (`ugaris-core` has no visibility into the weather table
    /// itself - see `do_action::speed_ticks_with_weather_movement` and
    /// `do_walk`, which apply this value and the indoor-tile override).
    /// 100 = no weather effect (C's `MOD_WEATHER_EFFECT_SLOW` flag unset).
    pub weather_movement_percent: i32,
    /// C `src/system/loot/loot.c`'s `static struct LootModifier modifiers[]`
    /// registry: named runtime-settable scalars (default 1.0) that loot
    /// table groups opt into via a `"modifiers": ["event_drop_rate"]` JSON
    /// list. Recurring events (`src/module/events/recurring/*`) call
    /// `loot_set_modifier`/`loot_get_modifier` to scale drop rates for the
    /// duration of the event; the JSON loot-table roll engine itself
    /// (`Death-mode loot tables` porting task) is the eventual consumer.
    #[serde(default)]
    pub loot_modifiers: HashMap<String, f64>,
}

impl Default for GameSettings {
    fn default() -> Self {
        let ticks = TICKS_PER_SECOND as i32;
        Self {
            item_decay_time: 5 * 60 * ticks,
            player_body_decay_time: 30 * 60 * ticks,
            npc_body_decay_time: 2 * 60 * ticks,
            npc_body_decay_time_area32: 15 * 60 * ticks,
            npc_respawn_timer: 2 * 60 * ticks,
            lagout_time: 5 * 60 * ticks,
            regen_time: 4 * ticks,
            sewer_item_respawn_time: 60 * 60 * 24,
            exp_modifier: 1.0,
            hardcore_military_exp_bonus: 1.10,
            hardcore_exp_bonus: 1.0,
            hardcore_kill_exp_bonus: 1.30,
            holler_dist: SAY_DIST as i32 * 3,
            shout_dist: SAY_DIST as i32 * 2,
            say_dist: SAY_DIST as i32,
            emote_dist: SAY_DIST as i32 / 2,
            quietsay_dist: SAY_DIST as i32 / 3,
            whisper_dist: SAY_DIST as i32 / 4,
            holler_cost: 12 * POWERSCALE,
            shout_cost: 6 * POWERSCALE,
            special_item_drop_multiplier: 1.0,
            sp_lots: SP_LOTS_CONST,
            sp_many: SP_MANY_CONST,
            sp_some: SP_SOME_CONST,
            sp_few: SP_FEW_CONST,
            sp_rare: SP_RARE_CONST,
            sp_ultra: SP_ULTRA_CONST,
            jail_x: 186,
            jail_y: 234,
            jail_area: 3,
            aston_x: 133,
            aston_y: 203,
            aston_area: 3,
            base_orb_respawn_time_days: 30,
            base_anti_orb_respawn_time_days: 30,
            max_jewel_count: 2,
            tunnel_exp_base_value_divider: 5.0,
            tunnel_mill_exp_base_value: 100,
            rare_golem_chance: 25,
            max_silver_golem_type: 8,
            normal_drop_chance: 20,
            rare_drop_chance: 100,
            rare_drop_multiplier: 1.2,
            base_drop_multiplier: 8,
            level_divisor: 10,
            golem_inventory_slot: 30,
            rare_golem_level_boost: 2,
            rare_golem_hp_multiplier: 2,
            mining_silver_chance_base: 6667,
            mining_gold_chance_base: 2000,
            mining_golem_chance_base: 10000,
            mining_orb_chance_base: 5,
            mining_cavein_chance_base: 2000,
            mining_artifact_chance_base: 200,
            mining_silver_gold_multiplier: 1.0,
            mining_cavein_multiplier: 1.0,
            mining_golem_event_multiplier: 1.0,
            mining_artifact_multiplier: 1.0,
            dungeon_time: ticks * 60 * 60,
            branfo_exp_base: 10000,
            bran_exp_base: 15000,
            demon_lord_door_after_solve_access_time: 120,
            solve_max_divisor: 4,
            drop_prob_low_level: 1700,
            drop_prob_mid_level: 800,
            drop_prob_high_level: 532,
            demon_power_deduction: 750,
            max_visible_pents: 10,
            demon_inventory_slot: 30,
            pentagram_value_multiplier: 50,
            pentagram_worth_divisor: 6,
            lucky_pentagram_chance: 50,
            power_increment: 20,
            max_power_level: 2000,
            max_training_power: 1600,
            random_spawn_chance: 15,
            activation_spawn_count: 3,
            spawn_count_level_threshold: 16,
            max_spawn_low_level: 3,
            max_spawn_high_level: 2,
            item_drop_equipment_threshold: 100,
            item_drop_bronze_threshold: 900,
            item_drop_silver_threshold: 990,
            equipment_type_count: 12,
            tester_movement_range: 10,
            exp_level_divisor_primary: 3,
            exp_solve_multiplier: 0.66,
            exp_level_divisor_secondary: 6,
            exp_clan_reflection_multiplier: 0.70,
            max_clan_bonus_percent: 20,
            tester_heal_threshold: 0.5,
            weather_movement_percent: 100,
            loot_modifiers: HashMap::new(),
        }
    }
}

macro_rules! accessors {
    ($get:ident, $set:ident, $field:ident, $ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.$field
        }

        pub fn $set(&mut self, value: $ty) {
            self.$field = value;
        }
    };
}

impl GameSettings {
    accessors!(
        get_rare_golem_chance,
        set_rare_golem_chance,
        rare_golem_chance,
        i32
    );
    accessors!(
        get_max_silver_golem_type,
        set_max_silver_golem_type,
        max_silver_golem_type,
        i32
    );
    accessors!(
        get_normal_drop_chance,
        set_normal_drop_chance,
        normal_drop_chance,
        i32
    );
    accessors!(
        get_rare_drop_chance,
        set_rare_drop_chance,
        rare_drop_chance,
        i32
    );
    accessors!(
        get_rare_drop_multiplier,
        set_rare_drop_multiplier,
        rare_drop_multiplier,
        f32
    );
    accessors!(
        get_base_drop_multiplier,
        set_base_drop_multiplier,
        base_drop_multiplier,
        i32
    );
    accessors!(get_level_divisor, set_level_divisor, level_divisor, i32);
    accessors!(
        get_golem_inventory_slot,
        set_golem_inventory_slot,
        golem_inventory_slot,
        i32
    );
    accessors!(
        get_rare_golem_level_boost,
        set_rare_golem_level_boost,
        rare_golem_level_boost,
        i32
    );
    accessors!(
        get_rare_golem_hp_multiplier,
        set_rare_golem_hp_multiplier,
        rare_golem_hp_multiplier,
        i32
    );
    accessors!(get_dungeon_time, set_dungeon_time, dungeon_time, i32);
    accessors!(
        get_branfo_exp_base,
        set_branfo_exp_base,
        branfo_exp_base,
        i32
    );
    accessors!(get_bran_exp_base, set_bran_exp_base, bran_exp_base, i32);
    accessors!(
        get_demon_lord_door_after_solve_access_time,
        set_demon_lord_door_after_solve_access_time,
        demon_lord_door_after_solve_access_time,
        i32
    );
    accessors!(
        get_solve_max_divisor,
        set_solve_max_divisor,
        solve_max_divisor,
        i32
    );
    accessors!(
        get_drop_prob_low_level,
        set_drop_prob_low_level,
        drop_prob_low_level,
        i32
    );
    accessors!(
        get_drop_prob_mid_level,
        set_drop_prob_mid_level,
        drop_prob_mid_level,
        i32
    );
    accessors!(
        get_drop_prob_high_level,
        set_drop_prob_high_level,
        drop_prob_high_level,
        i32
    );
    accessors!(
        get_demon_power_deduction,
        set_demon_power_deduction,
        demon_power_deduction,
        i32
    );
    accessors!(
        get_max_visible_pents,
        set_max_visible_pents,
        max_visible_pents,
        i32
    );
    accessors!(
        get_demon_inventory_slot,
        set_demon_inventory_slot,
        demon_inventory_slot,
        i32
    );
    accessors!(
        get_pentagram_value_multiplier,
        set_pentagram_value_multiplier,
        pentagram_value_multiplier,
        i32
    );
    accessors!(
        get_pentagram_worth_divisor,
        set_pentagram_worth_divisor,
        pentagram_worth_divisor,
        i32
    );
    accessors!(
        get_lucky_pentagram_chance,
        set_lucky_pentagram_chance,
        lucky_pentagram_chance,
        i32
    );
    accessors!(
        get_power_increment,
        set_power_increment,
        power_increment,
        i32
    );
    accessors!(
        get_max_power_level,
        set_max_power_level,
        max_power_level,
        i32
    );
    accessors!(
        get_max_training_power,
        set_max_training_power,
        max_training_power,
        i32
    );
    accessors!(
        get_random_spawn_chance,
        set_random_spawn_chance,
        random_spawn_chance,
        i32
    );
    accessors!(
        get_activation_spawn_count,
        set_activation_spawn_count,
        activation_spawn_count,
        i32
    );
    accessors!(
        get_spawn_count_level_threshold,
        set_spawn_count_level_threshold,
        spawn_count_level_threshold,
        i32
    );
    accessors!(
        get_max_spawn_low_level,
        set_max_spawn_low_level,
        max_spawn_low_level,
        i32
    );
    accessors!(
        get_max_spawn_high_level,
        set_max_spawn_high_level,
        max_spawn_high_level,
        i32
    );
    accessors!(
        get_item_drop_equipment_threshold,
        set_item_drop_equipment_threshold,
        item_drop_equipment_threshold,
        i32
    );
    accessors!(
        get_item_drop_bronze_threshold,
        set_item_drop_bronze_threshold,
        item_drop_bronze_threshold,
        i32
    );
    accessors!(
        get_item_drop_silver_threshold,
        set_item_drop_silver_threshold,
        item_drop_silver_threshold,
        i32
    );
    accessors!(
        get_equipment_type_count,
        set_equipment_type_count,
        equipment_type_count,
        i32
    );
    accessors!(
        get_tester_movement_range,
        set_tester_movement_range,
        tester_movement_range,
        i32
    );
    accessors!(
        get_exp_level_divisor_primary,
        set_exp_level_divisor_primary,
        exp_level_divisor_primary,
        i32
    );
    accessors!(
        get_exp_solve_multiplier,
        set_exp_solve_multiplier,
        exp_solve_multiplier,
        f64
    );
    accessors!(
        get_exp_level_divisor_secondary,
        set_exp_level_divisor_secondary,
        exp_level_divisor_secondary,
        i32
    );
    accessors!(
        get_exp_clan_reflection_multiplier,
        set_exp_clan_reflection_multiplier,
        exp_clan_reflection_multiplier,
        f64
    );
    accessors!(
        get_max_clan_bonus_percent,
        set_max_clan_bonus_percent,
        max_clan_bonus_percent,
        i32
    );
    accessors!(
        get_tester_heal_threshold,
        set_tester_heal_threshold,
        tester_heal_threshold,
        f64
    );
    accessors!(
        get_special_item_drop_multiplier,
        set_special_item_drop_multiplier,
        special_item_drop_multiplier,
        f64
    );

    /// C `loot_set_modifier` (`src/system/loot/loot.c:138-152`): registers or
    /// updates a named runtime scalar. C silently ignores an empty name and
    /// logs+ignores registration past a fixed pool size; the Rust
    /// `HashMap` has no such fixed-size limit so only the empty-name guard
    /// applies here.
    pub fn set_loot_modifier(&mut self, name: &str, value: f64) {
        if name.is_empty() {
            return;
        }
        self.loot_modifiers.insert(name.to_string(), value);
    }

    /// C `loot_get_modifier` (`src/system/loot/loot.c:153-158`): returns the
    /// registered scalar, or `1.0` (no-op multiplier) if unset or the name
    /// is empty.
    pub fn get_loot_modifier(&self, name: &str) -> f64 {
        if name.is_empty() {
            return 1.0;
        }
        self.loot_modifiers.get(name).copied().unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_c_initializers() {
        let settings = GameSettings::default();
        assert_eq!(settings.item_decay_time, 5 * 60 * 24);
        assert_eq!(settings.say_dist, 25);
        assert_eq!(settings.holler_cost, 12 * 1000);
        assert_eq!(settings.sp_lots, 5000);
        assert_eq!(settings.rare_golem_chance, 25);
        assert_eq!(settings.dungeon_time, 24 * 60 * 60);
        assert_eq!(settings.max_power_level, 2000);
        assert_eq!(settings.tester_heal_threshold, 0.5);
    }

    #[test]
    fn compatibility_accessors_read_and_write_settings() {
        let mut settings = GameSettings::default();

        assert_eq!(settings.get_rare_golem_chance(), 25);
        settings.set_rare_golem_chance(33);
        assert_eq!(settings.get_rare_golem_chance(), 33);

        assert_eq!(settings.get_exp_solve_multiplier(), 0.66);
        settings.set_exp_solve_multiplier(0.75);
        assert_eq!(settings.get_exp_solve_multiplier(), 0.75);

        assert_eq!(settings.get_special_item_drop_multiplier(), 1.0);
        settings.set_special_item_drop_multiplier(1.5);
        assert_eq!(settings.get_special_item_drop_multiplier(), 1.5);
    }

    #[test]
    fn loot_modifier_defaults_to_one_and_round_trips_like_c() {
        let mut settings = GameSettings::default();

        // C `loot_get_modifier` returns 1.0 for an unregistered name.
        assert_eq!(settings.get_loot_modifier("event_drop_rate"), 1.0);

        settings.set_loot_modifier("event_drop_rate", 2.0);
        assert_eq!(settings.get_loot_modifier("event_drop_rate"), 2.0);

        // Re-setting the same name updates in place (C `find_modifier` hit).
        settings.set_loot_modifier("event_drop_rate", 1.0);
        assert_eq!(settings.get_loot_modifier("event_drop_rate"), 1.0);

        // C `loot_set_modifier`/`loot_get_modifier` both silently ignore an
        // empty name.
        settings.set_loot_modifier("", 5.0);
        assert_eq!(settings.get_loot_modifier(""), 1.0);
    }
}
