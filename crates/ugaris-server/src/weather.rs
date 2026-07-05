use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WeatherState {
    pub(crate) current_weather: i32,
    pub(crate) weather_intensity: usize,
    pub(crate) weather_effects: u32,
    pub(crate) is_transitioning: bool,
    pub(crate) transition_start: u64,
    pub(crate) transition_duration: u64,
    pub(crate) prev_weather: i32,
    pub(crate) weather_change_time: u64,
    pub(crate) affected_areas: Vec<u16>,
    /// C `world_weather.seasonal_influence` (`weather.h`'s `struct
    /// weather_data`): the last `get_current_season()` result the
    /// autonomous cycle (`update_weather_tick`) observed, used to detect
    /// season changes. Seeded from the live game date at server startup
    /// (`main`'s pre-loop init, matching C `init_weather`'s `world_weather.
    /// seasonal_influence = get_current_season();`) rather than defaulting
    /// to `0`/spring, so the very first tick doesn't see a spurious season
    /// change.
    pub(crate) seasonal_influence: i32,
    /// C `apply_elemental_debuffs`'s `static int last_notify[TOTAL_MAXCHARS]`
    /// (`weather.c:636`): the tick each character last received an
    /// elemental-debuff flavor message, gating repeats to once per 10
    /// seconds. Entries are removed on disconnect (`ServerRuntime::
    /// disconnect`) since a fresh `PlayerRuntime`/character reuses the
    /// slot with no memory of the old cooldown in C either (a
    /// process-lifetime static array reset only on server restart, but
    /// per-character-id here since `CharacterId`s aren't reused within a
    /// server's lifetime the way C's fixed `TOTAL_MAXCHARS` slots are).
    pub(crate) elemental_debuff_last_notify: HashMap<CharacterId, u64>,
}

pub(crate) const WEATHER_EFFECT_SLOW: u32 = 0x01;

pub(crate) const WEATHER_EFFECT_BLIND: u32 = 0x02;

pub(crate) const WEATHER_EFFECT_DAMAGE: u32 = 0x04;

pub(crate) const WEATHER_EFFECT_SLIP: u32 = 0x08;

/// `mod_weather.h`'s `MOD_WEATHER_EFFECT_SKILL`: set whenever the current
/// weather/intensity cell has a nonzero `skill_mods` entry
/// (`weather.c:148-192`'s `weather_effects` table). The per-skill
/// modifier application itself (`modify_skill_value`) isn't wired yet -
/// see `PORTING_TODO.md`.
pub(crate) const WEATHER_EFFECT_SKILL: u32 = 0x10;

/// `weather.h`'s `WEATHER_EFFECT_LIGHTNING` (`0x100`): a server-internal
/// flag (never sent to the client - the client-visible bits are `0x01`-
/// `0x20` above) set whenever the current weather/intensity cell has a
/// nonzero `lightning_chance` (only `MOD_WEATHER_STORM` does).
pub(crate) const WEATHER_EFFECT_LIGHTNING: u32 = 0x100;

/// `weather.h`'s `WEATHER_EFFECT_ELEMENTAL` (`0x400`): a server-internal
/// flag set whenever the current weather/intensity cell has a nonzero
/// `elemental_debuff_type` (rain/storm = wet, snow = cold, sandstorm =
/// scorched). Gates `apply_elemental_debuffs`'s periodic flavor-text
/// notification - see [`elemental_debuff_type`]/[`elemental_debuff_message`].
pub(crate) const WEATHER_EFFECT_ELEMENTAL: u32 = 0x400;

/// `weather.h`'s `enum elemental_debuff_type` values, transcribed
/// digit-for-digit.
pub(crate) const DEBUFF_NONE: i32 = 0;
pub(crate) const DEBUFF_WET: i32 = 1;
pub(crate) const DEBUFF_COLD: i32 = 2;
pub(crate) const DEBUFF_SCORCHED: i32 = 3;

pub(crate) const WEATHER_INTENSITY_NAMES: [&str; 4] = ["None", "Light", "Moderate", "Heavy"];

/// C `weather.h`'s `WEATHER_TRANSITION_TIME` (`TICKS * 60 * 2`): the
/// autonomous cycle's transition length for a normal periodic weather
/// change (`update_weather`'s main branch uses this value directly; the
/// season-change branch uses `WEATHER_TRANSITION_TIME * 2`). Distinct from
/// the `/setweather`/`/clearweather` admin commands, which C hardcodes to
/// a flat one-minute `TICKS * 60` transition (`command.c`'s `cmd_setweather`/
/// `cmd_clearweather`) - already matched by `apply_weather_admin_command`
/// below.
pub(crate) const WEATHER_TRANSITION_TIME: u64 = TICKS_PER_SECOND * 60 * 2;
/// C `weather.h`'s `WEATHER_DURATION_MIN` (`TICKS * 60 * 30`).
pub(crate) const WEATHER_DURATION_MIN: u64 = TICKS_PER_SECOND * 60 * 30;
/// C `weather.h`'s `WEATHER_DURATION_MAX` (`TICKS * 60 * 60`).
pub(crate) const WEATHER_DURATION_MAX: u64 = TICKS_PER_SECOND * 60 * 60;

pub(crate) const SEASON_SPRING: i32 = 0;
pub(crate) const SEASON_SUMMER: i32 = 1;
pub(crate) const SEASON_AUTUMN: i32 = 2;
pub(crate) const SEASON_WINTER: i32 = 3;

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            current_weather: 0,
            weather_intensity: 0,
            weather_effects: 0,
            is_transitioning: false,
            transition_start: 0,
            transition_duration: 0,
            prev_weather: 0,
            weather_change_time: 0,
            affected_areas: Vec::new(),
            seasonal_influence: SEASON_SPRING,
            elemental_debuff_last_notify: HashMap::new(),
        }
    }
}

pub(crate) fn weather_name(weather_type: i32) -> &'static str {
    match weather_type {
        0 => "Clear",
        1 => "Rain",
        2 => "Storm",
        3 => "Snow",
        4 => "Sandstorm",
        5 => "Fog",
        _ => "Unknown",
    }
}

pub(crate) fn weather_description(weather_type: i32, intensity: usize) -> &'static str {
    if weather_type == 0 || intensity == 0 {
        return "Clear skies";
    }
    match (weather_type, intensity.min(3)) {
        (1, 1) => "Light rain",
        (1, 2) => "Moderate rain",
        (1, _) => "Heavy rain",
        (2, 1) => "Light storm",
        (2, 2) => "Moderate storm",
        (2, _) => "Heavy storm",
        (3, 1) => "Light snow",
        (3, 2) => "Moderate snow",
        (3, _) => "Heavy snow",
        (4, 1) => "Light sandstorm",
        (4, 2) => "Moderate sandstorm",
        (4, _) => "Heavy sandstorm",
        (5, 1) => "Light fog",
        (5, 2) => "Moderate fog",
        (5, _) => "Heavy fog",
        _ => "Unknown weather",
    }
}

pub(crate) fn is_valid_weather_type(weather_type: i64) -> bool {
    (0..=5).contains(&weather_type)
}

pub(crate) fn is_valid_weather_intensity(intensity: i64) -> bool {
    (1..=3).contains(&intensity)
}

pub(crate) fn weather_type_list_messages() -> Vec<String> {
    vec![
        "Invalid weather type. Valid types are:".to_string(),
        "0 = Clear".to_string(),
        "1 = Rain".to_string(),
        "2 = Storm".to_string(),
        "3 = Snow".to_string(),
        "4 = Sandstorm".to_string(),
        "5 = Fog".to_string(),
    ]
}

/// C `weather.c:148-192`'s `static const struct weather_effect_data
/// weather_effects[MOD_WEATHER_MAX][4]` (one row per weather type, index 0
/// of each row is the C table's unused "no intensity" placeholder - real
/// intensities are 1=Light/2=Moderate/3=Heavy, gated by
/// `is_valid_weather_intensity`). Only the fields consumed by any Rust
/// code path are ported (`move_mod`/`vis_mod`/`damage`/`slip_chance`/
/// whether any `skill_mods` entry is nonzero/`lightning_chance`);
/// `attack_speed_mod`/`spell_power_mod`/`elemental_debuff_type` and the
/// per-skill `V_PERCEPT`/`V_STEALTH` values are C table columns not yet
/// consumed by any Rust code path (`modify_attack_speed`/
/// `modify_spell_power`/`modify_skill_value` are unported - see
/// `PORTING_TODO.md`; `elemental_debuff_type` is consumed by
/// `apply_elemental_debuffs`'s periodic-notification slice, see
/// [`elemental_debuff_type`]).
#[derive(Debug, Clone, Copy)]
struct WeatherEffectData {
    move_mod: i32,
    vis_mod: i32,
    damage: i32,
    slip_chance: i32,
    has_skill_mod: bool,
    lightning_chance: i32,
    elemental_debuff_type: i32,
}

const NO_EFFECT: WeatherEffectData = WeatherEffectData {
    move_mod: 100,
    vis_mod: 100,
    damage: 0,
    slip_chance: 0,
    has_skill_mod: false,
    lightning_chance: 0,
    elemental_debuff_type: DEBUFF_NONE,
};

/// `[weather_type][intensity]`, transcribed digit-for-digit from
/// `weather.c:148-192`.
const WEATHER_EFFECTS: [[WeatherEffectData; 4]; 6] = [
    // MOD_WEATHER_CLEAR
    [NO_EFFECT, NO_EFFECT, NO_EFFECT, NO_EFFECT],
    // MOD_WEATHER_RAIN - no lightning
    [
        NO_EFFECT,
        WeatherEffectData {
            move_mod: 95,
            vis_mod: 90,
            damage: 0,
            slip_chance: 15,
            has_skill_mod: false,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_WET,
        },
        WeatherEffectData {
            move_mod: 90,
            vis_mod: 80,
            damage: 0,
            slip_chance: 25,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_WET,
        },
        WeatherEffectData {
            move_mod: 80,
            vis_mod: 70,
            damage: 0,
            slip_chance: 35,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_WET,
        },
    ],
    // MOD_WEATHER_STORM - the only weather type with lightning strikes
    [
        NO_EFFECT,
        WeatherEffectData {
            move_mod: 90,
            vis_mod: 80,
            damage: 0,
            slip_chance: 20,
            has_skill_mod: true,
            lightning_chance: 5,
            elemental_debuff_type: DEBUFF_WET,
        },
        WeatherEffectData {
            move_mod: 80,
            vis_mod: 60,
            damage: 0,
            slip_chance: 30,
            has_skill_mod: true,
            lightning_chance: 15,
            elemental_debuff_type: DEBUFF_WET,
        },
        WeatherEffectData {
            move_mod: 70,
            vis_mod: 40,
            damage: 0,
            slip_chance: 40,
            has_skill_mod: true,
            lightning_chance: 30,
            elemental_debuff_type: DEBUFF_WET,
        },
    ],
    // MOD_WEATHER_SNOW - no lightning
    [
        NO_EFFECT,
        WeatherEffectData {
            move_mod: 90,
            vis_mod: 85,
            damage: 0,
            slip_chance: 20,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_COLD,
        },
        WeatherEffectData {
            move_mod: 80,
            vis_mod: 70,
            damage: 0,
            slip_chance: 30,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_COLD,
        },
        WeatherEffectData {
            move_mod: 70,
            vis_mod: 55,
            damage: 0,
            slip_chance: 40,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_COLD,
        },
    ],
    // MOD_WEATHER_SANDSTORM - no lightning
    [
        NO_EFFECT,
        WeatherEffectData {
            move_mod: 85,
            vis_mod: 75,
            damage: 1,
            slip_chance: 15,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_SCORCHED,
        },
        WeatherEffectData {
            move_mod: 70,
            vis_mod: 50,
            damage: 2,
            slip_chance: 25,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_SCORCHED,
        },
        WeatherEffectData {
            move_mod: 55,
            vis_mod: 25,
            damage: 3,
            slip_chance: 35,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_SCORCHED,
        },
    ],
    // MOD_WEATHER_FOG - no lightning, no elemental debuff
    [
        NO_EFFECT,
        WeatherEffectData {
            move_mod: 95,
            vis_mod: 70,
            damage: 0,
            slip_chance: 0,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_NONE,
        },
        WeatherEffectData {
            move_mod: 90,
            vis_mod: 50,
            damage: 0,
            slip_chance: 0,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_NONE,
        },
        WeatherEffectData {
            move_mod: 85,
            vis_mod: 30,
            damage: 0,
            slip_chance: 0,
            has_skill_mod: true,
            lightning_chance: 0,
            elemental_debuff_type: DEBUFF_NONE,
        },
    ],
];

fn weather_effect_data(weather_type: i32, intensity: usize) -> WeatherEffectData {
    WEATHER_EFFECTS[weather_type as usize][intensity]
}

pub(crate) fn calculate_weather_effects(weather_type: i32, intensity: usize) -> u32 {
    if !is_valid_weather_type(i64::from(weather_type))
        || !is_valid_weather_intensity(intensity as i64)
    {
        return 0;
    }
    let data = weather_effect_data(weather_type, intensity);
    let mut effects = 0;
    if data.move_mod < 100 {
        effects |= WEATHER_EFFECT_SLOW;
    }
    if data.vis_mod < 100 {
        effects |= WEATHER_EFFECT_BLIND;
    }
    if data.damage > 0 {
        effects |= WEATHER_EFFECT_DAMAGE;
    }
    if data.slip_chance > 0 {
        effects |= WEATHER_EFFECT_SLIP;
    }
    if data.has_skill_mod {
        effects |= WEATHER_EFFECT_SKILL;
    }
    if data.lightning_chance > 0 {
        effects |= WEATHER_EFFECT_LIGHTNING;
    }
    if data.elemental_debuff_type != DEBUFF_NONE {
        effects |= WEATHER_EFFECT_ELEMENTAL;
    }
    effects
}

/// C `handle_weather_damage` (`weather.c:435-471`)'s damage lookup:
/// `weather_effects[world_weather.current_weather][world_weather.
/// weather_intensity].damage`.
pub(crate) fn weather_damage_amount(weather_type: i32, intensity: usize) -> i32 {
    if !is_valid_weather_type(i64::from(weather_type))
        || !is_valid_weather_intensity(intensity as i64)
    {
        return 0;
    }
    weather_effect_data(weather_type, intensity).damage
}

/// C `handle_weather_damage` (`weather.c:461-469`)'s per-weather-type
/// message shown whenever damage was actually dealt this tick. Given the
/// current `WEATHER_EFFECTS` table only `MOD_WEATHER_SANDSTORM` has a
/// nonzero `damage` (`MOD_WEATHER_STORM`/`MOD_WEATHER_SNOW` are always
/// `0`), the storm/snow branches are unreachable today, but transcribed
/// digit-for-digit anyway since they're a real (data-dependent, not
/// permanently dead) C `switch`, not the unreachable-by-construction
/// `LOG_INFO`/`dat1=0` pattern documented on the lightning nearby-players
/// broadcast below.
pub(crate) fn weather_damage_message(weather_type: i32) -> Option<&'static str> {
    match weather_type {
        2 => Some("Lightning strikes nearby!"), // MOD_WEATHER_STORM
        3 => Some("The freezing cold bites into you!"), // MOD_WEATHER_SNOW
        4 => Some("The stinging sand hurts you!"), // MOD_WEATHER_SANDSTORM
        _ => None,
    }
}

/// C `handle_lightning_strike` (`weather.c:551-554`)'s `lightning_chance`
/// lookup: `weather_effects[world_weather.current_weather][world_weather.
/// weather_intensity].lightning_chance`.
pub(crate) fn lightning_strike_chance(weather_type: i32, intensity: usize) -> i32 {
    if !is_valid_weather_type(i64::from(weather_type))
        || !is_valid_weather_intensity(intensity as i64)
    {
        return 0;
    }
    weather_effect_data(weather_type, intensity).lightning_chance
}

/// C `handle_lightning_strike` (`weather.c:562-573`)'s intensity-scaled
/// damage roll: `10+RANDOM(10)` / `20+RANDOM(20)` / `40+RANDOM(40)` for
/// Light/Moderate/Heavy, `15` flat for any other (unreachable in practice
/// since `lightning_chance` is only nonzero for those three intensities)
/// value - transcribed digit-for-digit including the C `switch`'s
/// `default` branch.
pub(crate) fn lightning_strike_damage_amount(
    intensity: usize,
    mut random_below: impl FnMut(i32) -> i32,
) -> i32 {
    match intensity {
        1 => 10 + random_below(10), // MOD_INTENSITY_LIGHT
        2 => 20 + random_below(20), // MOD_INTENSITY_MODERATE
        3 => 40 + random_below(40), // MOD_INTENSITY_HEAVY
        _ => 15,
    }
}

/// C `apply_elemental_debuffs` (`weather.c:614-655`)'s table lookup:
/// `weather_effects[world_weather.current_weather][world_weather.
/// weather_intensity].elemental_debuff_type`.
pub(crate) fn elemental_debuff_type(weather_type: i32, intensity: usize) -> i32 {
    if !is_valid_weather_type(i64::from(weather_type))
        || !is_valid_weather_intensity(intensity as i64)
    {
        return DEBUFF_NONE;
    }
    weather_effect_data(weather_type, intensity).elemental_debuff_type
}

/// C `apply_elemental_debuffs` (`weather.c:636-647`)'s per-debuff-type
/// flavor message, transcribed letter-for-letter.
///
/// Only the periodic notification is ported: C's own persistent
/// `elemental_debuff[cn]`/`elemental_debuff_expire[cn]` state and the
/// "debuff wearing off" message it would otherwise gate are genuinely
/// unreachable dead code in the real C server - `apply_elemental_debuffs`
/// is only ever called (`weather.c:1164-1166`) when `world_weather.
/// weather_effects & WEATHER_EFFECT_ELEMENTAL` is set, which is derived
/// from the exact same table lookup this function itself re-reads, so its
/// own `debuff_type == DEBUFF_NONE` branch (the wearing-off message) can
/// never execute; and the persistent debuff/expire state is only ever
/// *read* by `get_elemental_debuff`, whose only 4 callers
/// (`modify_attack_speed`/`modify_spell_power`/`modify_fire_resistance`/
/// `modify_cold_resistance`) are themselves confirmed dead code (verified:
/// no other `.c` file calls any of the four) - so the state has zero
/// observable effect anywhere. Only the `last_notify[cn]`/`TICKS*10` gate
/// on this message is real, live behavior.
pub(crate) fn elemental_debuff_message(debuff_type: i32) -> Option<&'static str> {
    match debuff_type {
        DEBUFF_WET => Some("You are getting soaked by the rain."),
        DEBUFF_COLD => Some("The cold is seeping into your bones."),
        DEBUFF_SCORCHED => Some("The scorching heat is draining your energy."),
        _ => None,
    }
}

/// C `apply_elemental_debuffs` (`weather.c:636-638`)'s `static int
/// last_notify[TOTAL_MAXCHARS]` gate: `ticker - last_notify[cn] >=
/// TICKS*10`. `last_notify` defaults to `0` for a character never seen
/// before, matching a fresh (or restarted) server's zero-initialized
/// static array.
pub(crate) fn should_notify_elemental_debuff(last_notify: u64, now: u64) -> bool {
    now.saturating_sub(last_notify) >= TICKS_PER_SECOND * 10
}

/// C `modify_movement_speed`'s table lookup (`module/weather/weather.c:
/// 477-493`): resolves `world_weather.weather_effects & MOD_WEATHER_EFFECT_SLOW`
/// and the `move_mod` cell into the single percentage `ugaris-core`'s
/// `do_action::speed_ticks_with_weather_movement`/`do_walk` apply (100 = no
/// change, matching C's early return when the flag is unset). The
/// indoor-tile override lives in `do_walk` itself, since C's own check uses
/// the *character's* position, not anything area/weather-global.
pub(crate) fn current_movement_percent(weather: &WeatherState) -> i32 {
    if weather.weather_effects & WEATHER_EFFECT_SLOW == 0 {
        return 100;
    }
    if !is_valid_weather_type(i64::from(weather.current_weather))
        || !is_valid_weather_intensity(weather.weather_intensity as i64)
    {
        return 100;
    }
    weather_effect_data(weather.current_weather, weather.weather_intensity).move_mod
}

/// C `weather.c:104-127`'s per-area config table's `has_weather = false`
/// entries: underground/indoor/arena areas where weather never applies at
/// all (`area_has_weather`).
const NO_WEATHER_AREAS: &[i64] = &[4, 8, 11, 12, 13, 14, 16, 17, 18, 22, 25, 32, 33, 34, 36, 37];

/// C `weather.c:104-127`'s desert entries (areas 19/20, `WEATHER_DESERT`).
/// `weather.h`'s `WEATHER_DESERT` macro is `WEATHER_ALLOW_CLEAR` only
/// ("Only clear until sandstorm is ready" - fog/sandstorm are globally
/// disabled pending further development, matching `WEATHER_ALL`/
/// `WEATHER_OUTDOOR_NORMAL` never including those two bits either), so
/// deserts currently behave like the no-weather areas for allowed-type
/// purposes even though `has_weather` is `true` for them.
const DESERT_AREAS: &[i64] = &[19, 20];

/// C `area_has_weather` (`weather.c:140-147`).
pub(crate) fn area_has_weather(area: i64) -> bool {
    !NO_WEATHER_AREAS.contains(&area)
}

pub(crate) fn is_weather_allowed_in_area(weather_type: i64, area: i64) -> bool {
    if !is_valid_weather_type(weather_type) || !(0..=255).contains(&area) {
        return false;
    }
    if NO_WEATHER_AREAS.contains(&area) || DESERT_AREAS.contains(&area) {
        return weather_type == 0;
    }
    matches!(weather_type, 0..=3)
}

pub(crate) fn apply_weather_admin_command(
    world: &World,
    character_id: CharacterId,
    weather: &mut WeatherState,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    let lower = verb.to_ascii_lowercase();
    let recognized = matches!(
        lower.as_str(),
        "setweather" | "clearweather" | "setareaweather"
    );
    if !recognized {
        return None;
    }
    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    if !character.flags.contains(CharacterFlags::GOD) {
        return Some(KeyringCommandResult {
            messages: vec!["You need to be a god to use this command.".to_string()],
            ..Default::default()
        });
    }

    if lower == "clearweather" {
        weather.prev_weather = weather.current_weather;
        weather.current_weather = 0;
        weather.weather_intensity = 1;
        weather.weather_effects = 0;
        weather.is_transitioning = true;
        weather.transition_start = world.tick.0;
        weather.transition_duration = TICKS_PER_SECOND * 60;
        weather.affected_areas.clear();
        return Some(KeyringCommandResult {
            messages: vec!["Weather clearing globally.".to_string()],
            ..Default::default()
        });
    }

    if lower == "setweather" {
        let mut ptr = rest.trim_start();
        let weather_type = legacy_atoi_prefix(ptr);
        ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
        let intensity = legacy_atoi_prefix(ptr.trim_start());
        if !is_valid_weather_type(weather_type) {
            return Some(KeyringCommandResult {
                messages: weather_type_list_messages(),
                ..Default::default()
            });
        }
        if !is_valid_weather_intensity(intensity) {
            return Some(KeyringCommandResult {
                messages: vec![
                    "Invalid intensity. Must be between 1 (Light) and 3 (Heavy).".to_string(),
                ],
                ..Default::default()
            });
        }
        weather.prev_weather = weather.current_weather;
        let weather_type = weather_type as i32;
        weather.current_weather = weather_type;
        weather.weather_intensity = intensity as usize;
        weather.weather_effects = calculate_weather_effects(weather_type, intensity as usize);
        weather.is_transitioning = true;
        weather.transition_start = world.tick.0;
        weather.transition_duration = TICKS_PER_SECOND * 60;
        return Some(KeyringCommandResult {
            messages: vec![format!(
                "Weather changing to {}",
                weather_description(weather_type, intensity as usize)
            )],
            ..Default::default()
        });
    }

    let mut ptr = rest.trim_start();
    let area = legacy_atoi_prefix(ptr);
    ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
    let weather_type = legacy_atoi_prefix(ptr.trim_start());
    if !(0..=255).contains(&area) {
        return Some(KeyringCommandResult {
            messages: vec!["Invalid area ID. Must be between 0 and 255.".to_string()],
            ..Default::default()
        });
    }
    if !is_valid_weather_type(weather_type) {
        return Some(KeyringCommandResult {
            messages: weather_type_list_messages(),
            ..Default::default()
        });
    }
    if !is_weather_allowed_in_area(weather_type, area) {
        return Some(KeyringCommandResult {
            messages: vec![format!("This weather type is not allowed in area {area}.")],
            ..Default::default()
        });
    }
    let area = area as u16;
    if weather_type == 0 {
        weather.affected_areas.retain(|affected| *affected != area);
    } else if !weather.affected_areas.contains(&area) {
        weather.affected_areas.push(area);
    }
    Some(KeyringCommandResult {
        messages: vec![format!(
            "Set weather in area {area} to {}",
            weather_name(weather_type as i32)
        )],
        ..Default::default()
    })
}

pub(crate) fn apply_weather_command(
    world: &World,
    character_id: CharacterId,
    area_id: u16,
    weather: &WeatherState,
    command: &str,
) -> Option<KeyringCommandResult> {
    let (verb, _) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb.trim_start_matches('/').trim_start_matches('#');
    if !verb.eq_ignore_ascii_case("weather") {
        return None;
    }

    let Some(character) = world.characters.get(&character_id) else {
        return Some(KeyringCommandResult::default());
    };
    let mut messages = vec![format!(
        "Current weather in this area: {}",
        weather_description(weather.current_weather, weather.weather_intensity)
    )];

    if character.flags.contains(CharacterFlags::GOD) {
        let intensity = WEATHER_INTENSITY_NAMES
            .get(weather.weather_intensity)
            .copied()
            .unwrap_or("Unknown");
        messages.extend([
            "Global Weather Debug Info:".to_string(),
            format!(
                "- Current Weather: {}",
                weather_name(weather.current_weather)
            ),
            format!("- Intensity: {intensity}"),
            format!("- Effects: 0x{:x}", weather.weather_effects),
        ]);
        if weather.is_transitioning {
            let end = weather
                .transition_start
                .saturating_add(weather.transition_duration);
            let time_left = end.saturating_sub(world.tick.0) / TICKS_PER_SECOND;
            let progress = if weather.transition_duration == 0 {
                100.0
            } else {
                (world.tick.0.saturating_sub(weather.transition_start) as f64
                    / weather.transition_duration as f64)
                    .clamp(0.0, 1.0)
                    * 100.0
            };
            messages.push(format!("- Transitioning: Yes ({time_left} seconds left)"));
            messages.push(format!(
                "- Previous Weather: {}",
                weather_name(weather.prev_weather)
            ));
            messages.push(format!("- Progress: {progress:.1}%"));
        } else {
            messages.push("- Transitioning: No".to_string());
        }
        messages.push(format!(
            "- Next Change: {} seconds",
            weather.weather_change_time.saturating_sub(world.tick.0) / TICKS_PER_SECOND
        ));
        messages.push(format!(
            "- Affected Areas ({}):",
            weather.affected_areas.len()
        ));
        if !weather.affected_areas.is_empty() {
            let mut areas = weather
                .affected_areas
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            areas.push(' ');
            messages.push(format!("  {areas}"));
        }
    }

    let indoors = world
        .map
        .tile(usize::from(character.x), usize::from(character.y))
        .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS));
    if indoors {
        messages.push("You are indoors and protected from weather effects.".to_string());
    } else if weather.affected_areas.is_empty() || weather.affected_areas.contains(&area_id) {
        if weather.weather_effects & WEATHER_EFFECT_SLOW != 0 {
            messages.push("Movement is affected by the weather.".to_string());
        }
        if weather.weather_effects & WEATHER_EFFECT_BLIND != 0 {
            messages.push("Visibility is reduced by the weather.".to_string());
        }
        if weather.weather_effects & WEATHER_EFFECT_DAMAGE != 0 {
            messages.push("The weather is causing damage.".to_string());
        }
        if weather.weather_effects & WEATHER_EFFECT_SLIP != 0 {
            messages.push("The weather makes the ground slippery.".to_string());
        }
    }

    Some(KeyringCommandResult {
        messages,
        ..Default::default()
    })
}

// ============================================================================
// Autonomous weather cycle (`weather.c:937-1059`'s `update_weather`, minus
// the multi-server mirror storage sync (`tick_weather_storage`,
// `weather.c:797-935`) and DB persistence (`save_weather_state`/
// `load_weather_state`) - both N/A: this Rust process is always a single
// area's only server (no `areaM` mirror concept), and there is no "global
// blob storage" primitive in `ugaris-db` yet (same architectural gap noted
// for the Arena rankings task's toplist in `PORTING_TODO.md`). Every area
// this process runs always behaves like C's `WEATHER_MASTER_MIRROR`.
// ============================================================================

/// C `get_current_season` (`weather.c:258-274`), using the already-ported
/// `GameDate` fields instead of the global `yday`/`*_equinox`/`*_solstice`
/// C globals.
pub(crate) fn current_season(date: &GameDate) -> i32 {
    if date.spring_equinox || (date.yday >= 90 && date.yday < 180) {
        SEASON_SPRING
    } else if date.summer_solstice || (date.yday >= 180 && date.yday < 270) {
        SEASON_SUMMER
    } else if date.fall_equinox || (date.yday >= 270 && date.yday < 360) {
        SEASON_AUTUMN
    } else {
        SEASON_WINTER
    }
}

/// C `weather.c:70-77`'s `seasonal_weather_chance[MAX_SEASONS][MOD_WEATHER_MAX]`.
/// Indices 4 (Sandstorm)/5 (Fog) are `0` in every season, matching "Fog and
/// Sandstorm disabled pending further development".
const SEASONAL_WEATHER_CHANCE: [[i32; 6]; 4] = [
    [45, 35, 20, 0, 0, 0], // Spring
    [70, 15, 15, 0, 0, 0], // Summer
    [35, 40, 25, 0, 0, 0], // Autumn
    [35, 5, 10, 50, 0, 0], // Winter
];

/// C `update_weather`'s inlined weighted-random weather selection (used
/// identically by both the season-change branch, `weather.c:1005-1029`,
/// and the periodic-change branch, `weather.c:1038-1049`): roll
/// `RANDOM(total_chance)` and walk the cumulative distribution. Falls back
/// to `MOD_WEATHER_CLEAR` (`0`) if `total_chance` is `0` (never happens
/// with the real table above, but matches C's `new_weather = MOD_WEATHER_
/// CLEAR` initializer if the loop never breaks).
pub(crate) fn pick_seasonal_weather(
    season: usize,
    mut random_below: impl FnMut(i32) -> i32,
) -> i32 {
    let chances = SEASONAL_WEATHER_CHANCE[season];
    let total: i32 = chances.iter().sum();
    let roll = random_below(total);
    let mut cumulative = 0;
    for (weather_type, chance) in chances.iter().enumerate() {
        cumulative += chance;
        if roll < cumulative {
            return weather_type as i32;
        }
    }
    0
}

/// C `update_weather`'s inlined intensity roll (identical in both
/// branches, `weather.c:1013-1021`/`1051-1059`): 50% Light, 30% Moderate,
/// 20% Heavy.
pub(crate) fn pick_intensity(mut random_below: impl FnMut(i32) -> i32) -> usize {
    let roll = random_below(100);
    if roll < 50 {
        1
    } else if roll < 80 {
        2
    } else {
        3
    }
}

/// C `start_weather_transition` (`weather.c:762-774`). Recomputes
/// `weather_effects` from `new_weather` paired with the *current* (not yet
/// updated) `weather.weather_intensity`, exactly like C - both call sites
/// (`update_weather`'s two branches) set `weather_intensity` to its new
/// value only *after* calling this, so the effects bitmask legitimately
/// lags the intensity update by construction until the next transition;
/// this is a faithful port of that quirk, not a bug.
fn start_weather_transition(
    weather: &mut WeatherState,
    new_weather: i32,
    tick: u64,
    duration: u64,
) {
    weather.prev_weather = weather.current_weather;
    weather.current_weather = new_weather;
    weather.is_transitioning = true;
    weather.transition_start = tick;
    weather.transition_duration = duration;
    weather.weather_effects = calculate_weather_effects(new_weather, weather.weather_intensity);
}

/// C `update_weather_transition` (`weather.c:777-786`).
fn update_weather_transition_tick(weather: &mut WeatherState, tick: u64) {
    if !weather.is_transitioning {
        return;
    }
    if tick
        >= weather
            .transition_start
            .saturating_add(weather.transition_duration)
    {
        weather.is_transitioning = false;
        weather.prev_weather = weather.current_weather;
    }
}

/// C `update_weather` (`weather.c:937-1059`), minus the mirror/storage
/// sync described above. Returns whether the weather actually changed
/// this tick (either the season-change branch or the periodic-change
/// branch started a new transition), so the caller knows whether to
/// broadcast an `SV_MOD2`/`SV_VIS_WEATHER` packet (mirroring C's own
/// `broadcast_weather_change()` calls, which only happen on those same
/// two branches).
pub(crate) fn update_weather_tick(
    weather: &mut WeatherState,
    date: &GameDate,
    tick: u64,
    mut random_below: impl FnMut(i32) -> i32,
) -> bool {
    update_weather_transition_tick(weather, tick);

    let mut changed = false;
    let season = current_season(date);
    if season != weather.seasonal_influence {
        weather.seasonal_influence = season;
        if weather.current_weather != 0 && random_below(100) < 25 {
            let new_weather = pick_seasonal_weather(season as usize, &mut random_below);
            if new_weather != weather.current_weather {
                start_weather_transition(weather, new_weather, tick, WEATHER_TRANSITION_TIME * 2);
                weather.weather_intensity = pick_intensity(&mut random_below);
                changed = true;
            }
        }
    }

    if tick < weather.weather_change_time {
        return changed;
    }

    let mut new_weather = pick_seasonal_weather(season as usize, &mut random_below);
    if new_weather == weather.current_weather {
        new_weather = (new_weather + 1) % 6;
    }
    start_weather_transition(weather, new_weather, tick, WEATHER_TRANSITION_TIME);
    weather.weather_intensity = pick_intensity(&mut random_below);
    weather.weather_change_time = tick
        + WEATHER_DURATION_MIN
        + u64::from(
            random_below((WEATHER_DURATION_MAX - WEATHER_DURATION_MIN) as i32).max(0) as u32,
        );
    true
}

// ============================================================================
// Client packet (`weather_client.c`'s `send_weather_update`/
// `broadcast_weather_packet`, `mod_send_weather`).
// ============================================================================

/// C `calculate_transition_progress` (`weather_client.c:57-64`).
pub(crate) fn transition_progress_byte(weather: &WeatherState, tick: u64) -> u8 {
    if !weather.is_transitioning {
        return 255;
    }
    let elapsed = tick.saturating_sub(weather.transition_start) as f32;
    let duration = weather.transition_duration.max(1) as f32;
    ((elapsed / duration).clamp(0.0, 1.0) * 255.0) as u8
}

/// C `calculate_day_night_position` (`weather_client.c:27-52`): `hour`/
/// `minute`/`sunrise`/`sunset` are all already in the compressed game-day
/// units `GameDate` uses (`HOUR_LEN`/`MIN_LEN`/`DAY_LEN`), so the same
/// integer-ratio formula applies unchanged.
pub(crate) fn day_night_position(date: &GameDate) -> u8 {
    let daylight_time = (date.sunset - date.sunrise).max(1);
    let current_time = date.hour * HOUR_LEN + date.minute * MIN_LEN;
    let position = if current_time < date.sunrise {
        (current_time * 64) / date.sunrise.max(1)
    } else if current_time < date.sunset {
        64 + ((current_time - date.sunrise) * 128) / daylight_time
    } else {
        let night_remaining = (DAY_LEN - date.sunset + date.sunrise).max(1);
        192 + ((current_time - date.sunset) * 64) / night_remaining
    };
    position.clamp(0, 255) as u8
}

/// C `mod_send_weather`'s payload (`weather_client.c:69-93`/`96-131`):
/// both `send_weather_update` and `broadcast_weather_packet` send
/// `world_weather.current_weather`/`weather_intensity` as-is (no
/// `is_weather_allowed_in_area` coercion at broadcast time - only the
/// admin commands' validation gates what `current_weather` can *become*).
/// `effects` is masked to the 5 client-visible bits and the caller adds
/// `MOD_WEATHER_EFFECT_INDOOR` per player.
pub(crate) fn weather_packet_bytes(
    weather: &WeatherState,
    date: &GameDate,
    tick: u64,
    indoor: bool,
) -> [u8; ugaris_protocol::mod_weather::SV_WEATHER_PACKET_SIZE] {
    let mut effects = (weather.weather_effects & 0x1F) as u8;
    if indoor {
        effects |= MOD_WEATHER_EFFECT_INDOOR;
    }
    sv_weather_packet(
        weather.current_weather as u8,
        weather.weather_intensity as u8,
        transition_progress_byte(weather, tick),
        day_night_position(date),
        effects,
    )
}

/// C `broadcast_weather_packet` (`weather_client.c:96-131`): sends every
/// connected player the current weather state, adding
/// `MOD_WEATHER_EFFECT_INDOOR` for players `is_player_indoors` reports
/// true for. Gated on `area_has_weather` exactly like C ("For no-weather
/// areas... don't broadcast weather changes - Players in these areas
/// already have CLEAR + INDOOR set on entry" via `init_player_weather_packet`
/// at login, see below).
pub(crate) fn broadcast_weather_packet(world: &World, runtime: &mut ServerRuntime, area_id: u16) {
    if !area_has_weather(i64::from(area_id)) {
        return;
    }
    let weather = runtime.weather.clone();
    let tick = world.tick.0;
    let targets: Vec<(u64, CharacterId)> = runtime
        .players
        .values()
        .filter_map(|player| {
            player
                .character_id
                .map(|character_id| (player.session_id, character_id))
        })
        .collect();
    for (session_id, character_id) in targets {
        let indoors = world
            .characters
            .get(&character_id)
            .and_then(|character| {
                world
                    .map
                    .tile(usize::from(character.x), usize::from(character.y))
            })
            .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS));
        let bytes = weather_packet_bytes(&weather, &world.date, tick, indoors);
        runtime.send_to_session(session_id, bytes::BytesMut::from(&bytes[..]));
    }
}

/// C `send_weather_lightning_strike` (`weather_client.c:267-287`)'s
/// intensity-by-weather-intensity table: brighter storms get a brighter
/// bolt SFX. The `default` branch (`200`) is unreachable in practice
/// (`lightning_chance` is only nonzero for intensities 1-3), ported for
/// parity anyway.
fn lightning_bolt_intensity(weather_intensity: usize) -> u8 {
    match weather_intensity {
        1 => 180, // MOD_INTENSITY_LIGHT
        2 => 220, // MOD_INTENSITY_MODERATE
        3 => 255, // MOD_INTENSITY_HEAVY
        _ => 200,
    }
}

/// C `broadcast_weather_thunder_effect` (`weather_client.c:313-337`)'s
/// per-recipient screen-flash intensity: `uint8_t flash_intensity =
/// (uint8_t)(200 - dist*10)`, floored at `50`. Note the C cast happens
/// *before* the floor check, so for `dist >= 21` the subtraction goes
/// negative and the `uint8_t` truncation wraps back up into the 200s
/// (e.g. `dist=21` -> `-10 as u8` -> `246`, which is `>= 50` so the floor
/// never fires) - a genuine C integer-wraparound quirk, not a "cap at
/// 50" for far-away players, replicated digit-for-digit here via the same
/// `as u8` truncation semantics.
pub(crate) fn thunder_screen_flash_intensity(dist: i32) -> u8 {
    let mut flash_intensity = (200 - dist * 10) as u8;
    if flash_intensity < 50 {
        flash_intensity = 50;
    }
    flash_intensity
}

/// C `broadcast_weather_thunder_effect` (`weather_client.c:313-337`):
/// sends every player within `radius` (independent x/y box, matching C's
/// `mod_broadcast_sfx`/this function's own inline loop - not a true
/// Chebyshev/circular distance) two SFX packets - a positional
/// lightning-bolt effect at `(x, y)` (same intensity for everyone in
/// range, from `send_weather_lightning_strike`, `duration=8`,
/// `SFX_COLOR_DEFAULT`) and a screen-wide flash that fades (with the
/// wraparound quirk above) by Manhattan distance from the strike. Called
/// from `main.rs`'s per-tick lightning-strike roll with the C call
/// site's hardcoded `radius = 12` (`weather.c:593`'s
/// `broadcast_weather_thunder_effect(x, y, 12)`).
pub(crate) fn broadcast_weather_thunder_effect(
    world: &World,
    runtime: &mut ServerRuntime,
    x: u16,
    y: u16,
    radius: i32,
    weather_intensity: usize,
) {
    let bolt_intensity = lightning_bolt_intensity(weather_intensity);
    let targets: Vec<(u64, i32, i32)> = runtime
        .players
        .values()
        .filter_map(|player| {
            player
                .character_id
                .map(|character_id| (player.session_id, character_id))
        })
        .filter_map(|(session_id, character_id)| {
            let character = world.characters.get(&character_id)?;
            let dx = i32::from(character.x) - i32::from(x);
            let dy = i32::from(character.y) - i32::from(y);
            (dx >= -radius && dx <= radius && dy >= -radius && dy <= radius)
                .then_some((session_id, dx, dy))
        })
        .collect();
    for (session_id, dx, dy) in targets {
        let bolt = sv_sfx_packet(
            SFX_LIGHTNING_STRIKE,
            x,
            y,
            bolt_intensity,
            8,
            SFX_COLOR_DEFAULT,
        );
        runtime.send_to_session(session_id, bytes::BytesMut::from(&bolt[..]));

        let dist = dx.abs() + dy.abs();
        let flash = sv_sfx_packet(
            SFX_SCREEN_FLASH,
            SFX_POS_SCREEN,
            SFX_POS_SCREEN,
            thunder_screen_flash_intensity(dist),
            0,
            SFX_COLOR_WHITE,
        );
        runtime.send_to_session(session_id, bytes::BytesMut::from(&flash[..]));
    }
}

/// C `get_area_weather` (`weather.c:328-345`): returns `world_weather.
/// current_weather` unless a specific set of `affected_areas` has been
/// pinned (`/weather affect <area>`) and this area isn't one of them, in
/// which case it's forced clear. An empty `affected_areas` list means
/// "global weather, applies to every area" (C's `num_affected_areas == 0`
/// branch).
fn area_weather_type(weather: &WeatherState, area_id: u16) -> i32 {
    if weather.affected_areas.is_empty() || weather.affected_areas.contains(&area_id) {
        weather.current_weather
    } else {
        0 // MOD_WEATHER_CLEAR
    }
}

/// C `init_player_weather` (`weather_client.c:155-169`), inlining the two
/// helpers it calls (`update_player_indoor_state`/`send_indoor_state`,
/// `play_weather_effects`/`send_weather_update`) down to the single packet
/// their combined effect always produces exactly once per login/area-change
/// (C's `reset_player_indoor_state` unconditionally clears the cached
/// indoor flag first, so `update_player_indoor_state`'s "only send on
/// change" guard either fires immediately when the fresh read is `true`,
/// or is skipped and `play_weather_effects` sends instead when it's
/// `false` - the two paths are mutually exclusive and exhaustive, so
/// exactly one packet is always produced):
/// - no-weather area (`!area_has_weather`): forced indoor Clear, matching
///   `send_indoor_state`'s `!area_has_weather(areaID)` branch
///   (`weather_client.c:1321-1325`) - `weather=CLEAR`, `intensity=0`,
///   `transition=255`, `day_night=0`, `effects=INDOOR` only.
/// - indoor tile in a weather-capable area: `send_indoor_state`'s `else`
///   branch (`weather_client.c:1326-1331`) - real area weather/intensity
///   with the `INDOOR` bit added, `transition=255`, `day_night=0` (both
///   hardcoded in that C call site, not computed).
/// - outdoors: `send_weather_update` (`weather_client.c:69-93`) - real
///   area weather (coerced to Clear if `!is_weather_allowed_in_area`),
///   real intensity/transition/day-night/effects, no `INDOOR` bit.
pub(crate) fn init_player_weather_packet(
    weather: &WeatherState,
    date: &GameDate,
    tick: u64,
    area_id: u16,
    indoor_tile: bool,
) -> [u8; ugaris_protocol::mod_weather::SV_WEATHER_PACKET_SIZE] {
    if !area_has_weather(i64::from(area_id)) {
        return sv_weather_packet(
            0, /* MOD_WEATHER_CLEAR */
            0,
            255,
            0,
            MOD_WEATHER_EFFECT_INDOOR,
        );
    }
    if indoor_tile {
        let area_weather = area_weather_type(weather, area_id) as u8;
        let effects = (weather.weather_effects & 0x1F) as u8 | MOD_WEATHER_EFFECT_INDOOR;
        return sv_weather_packet(
            area_weather,
            weather.weather_intensity as u8,
            255,
            0,
            effects,
        );
    }
    let mut area_weather = area_weather_type(weather, area_id);
    if !is_weather_allowed_in_area(i64::from(area_weather), i64::from(area_id)) {
        area_weather = 0; // MOD_WEATHER_CLEAR
    }
    sv_weather_packet(
        area_weather as u8,
        weather.weather_intensity as u8,
        transition_progress_byte(weather, tick),
        day_night_position(date),
        (weather.weather_effects & 0x1F) as u8,
    )
}
