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
}

pub(crate) const WEATHER_EFFECT_SLOW: u32 = 0x01;

pub(crate) const WEATHER_EFFECT_BLIND: u32 = 0x02;

pub(crate) const WEATHER_EFFECT_DAMAGE: u32 = 0x04;

pub(crate) const WEATHER_EFFECT_SLIP: u32 = 0x08;

pub(crate) const WEATHER_INTENSITY_NAMES: [&str; 4] = ["None", "Light", "Moderate", "Heavy"];

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

pub(crate) fn calculate_weather_effects(weather_type: i32, intensity: usize) -> u32 {
    if !is_valid_weather_type(i64::from(weather_type))
        || !is_valid_weather_intensity(intensity as i64)
    {
        return 0;
    }
    match weather_type {
        1 | 2 | 3 => WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SLIP,
        4 => {
            WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_DAMAGE | WEATHER_EFFECT_SLIP
        }
        5 => WEATHER_EFFECT_BLIND,
        _ => 0,
    }
}

pub(crate) fn is_weather_allowed_in_area(weather_type: i64, area: i64) -> bool {
    if !is_valid_weather_type(weather_type) || !(0..=255).contains(&area) {
        return false;
    }
    const NO_WEATHER_AREAS: &[i64] =
        &[4, 8, 11, 12, 13, 14, 16, 17, 18, 22, 25, 32, 33, 34, 36, 37];
    if NO_WEATHER_AREAS.contains(&area) {
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
