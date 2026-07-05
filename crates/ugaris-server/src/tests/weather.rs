use super::*;
use ugaris_protocol::mod_weather::{
    MOD_WEATHER_EFFECT_INDOOR, SV_VIS_WEATHER, SV_WEATHER_PACKET_SIZE,
};
use ugaris_protocol::packet::SV_MOD2;

#[test]
fn weather_command_reports_default_clear_area_weather() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Weather"), 1, 10, 10);
    character.x = 10;
    character.y = 10;
    world.add_character(character);

    let result = apply_weather_command(
        &world,
        character_id,
        1,
        &WeatherState::default(),
        "/weather",
    )
    .expect("weather command should be recognized");

    assert_eq!(
        result.messages,
        vec!["Current weather in this area: Clear skies"]
    );
    assert!(
        apply_weather_command(&world, character_id, 1, &WeatherState::default(), "/weath",)
            .is_none()
    );
}

#[test]
fn calculate_weather_effects_matches_c_table_at_every_boundary() {
    // `weather.c:148-192`'s table, transcribed digit-for-digit into
    // `WEATHER_EFFECTS`. Previously this Rust port hardcoded a simplified
    // per-type-only bitmask that (among other discrepancies) never set
    // `WEATHER_EFFECT_SLOW` for Fog even though C's own table has
    // `move_mod` below 100 at every Fog intensity - this test locks in the
    // fix.
    assert_eq!(calculate_weather_effects(0, 1), 0); // Clear: never any effect.

    // Light rain: slow+blind+slip, no skill mods ({0} in the C table).
    assert_eq!(
        calculate_weather_effects(1, 1),
        WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SLIP
    );
    // Moderate rain gains a skill mod (V_PERCEPT = -5).
    assert_eq!(
        calculate_weather_effects(1, 2),
        WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SLIP | WEATHER_EFFECT_SKILL
    );

    // Heavy storm: slow+blind+slip+skill, no damage.
    assert_eq!(
        calculate_weather_effects(2, 3),
        WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SLIP | WEATHER_EFFECT_SKILL
    );

    // Light sandstorm already has damage=1 (unlike moderate/heavy-only
    // damage in the old simplified port).
    assert_eq!(
        calculate_weather_effects(4, 1),
        WEATHER_EFFECT_SLOW
            | WEATHER_EFFECT_BLIND
            | WEATHER_EFFECT_DAMAGE
            | WEATHER_EFFECT_SLIP
            | WEATHER_EFFECT_SKILL
    );

    // Light fog: C's table has move_mod=95 (<100), so SLOW must be set -
    // the bug this test guards against.
    assert_eq!(
        calculate_weather_effects(5, 1),
        WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SKILL
    );
    // Fog never has slip or damage at any intensity.
    assert_eq!(
        calculate_weather_effects(5, 3),
        WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SKILL
    );

    // Invalid inputs are always a no-op (matches C's is_valid_weather_type/
    // is_valid_weather_intensity guard clauses).
    assert_eq!(calculate_weather_effects(6, 1), 0);
    assert_eq!(calculate_weather_effects(1, 0), 0);
    assert_eq!(calculate_weather_effects(1, 4), 0);
}

#[test]
fn weather_damage_amount_matches_sandstorm_table_and_is_zero_elsewhere() {
    assert_eq!(weather_damage_amount(4, 1), 1);
    assert_eq!(weather_damage_amount(4, 2), 2);
    assert_eq!(weather_damage_amount(4, 3), 3);
    assert_eq!(weather_damage_amount(1, 3), 0); // Rain never damages.
    assert_eq!(weather_damage_amount(0, 1), 0); // Clear never damages.
    assert_eq!(weather_damage_amount(1, 0), 0); // Invalid intensity.
}

#[test]
fn is_weather_allowed_in_area_treats_desert_as_clear_only() {
    // `weather.h`'s `WEATHER_DESERT` macro is `WEATHER_ALLOW_CLEAR` only
    // ("Only clear until sandstorm is ready"), so areas 19/20 behave like
    // the no-weather list for allowed-type purposes even though they
    // still technically "have weather".
    assert!(is_weather_allowed_in_area(0, 19));
    assert!(!is_weather_allowed_in_area(1, 19));
    assert!(!is_weather_allowed_in_area(3, 20));
    // A normal outdoor area allows Clear/Rain/Storm/Snow but not
    // Sandstorm/Fog (globally disabled).
    assert!(is_weather_allowed_in_area(3, 1));
    assert!(!is_weather_allowed_in_area(4, 1));
    assert!(!is_weather_allowed_in_area(5, 1));
    // A no-weather (underground) area only ever allows Clear.
    assert!(is_weather_allowed_in_area(0, 12));
    assert!(!is_weather_allowed_in_area(1, 12));
}

#[test]
fn area_has_weather_matches_c_table() {
    // Desert areas still "have weather" (`has_weather = true` in C's
    // config table) even though only Clear is currently allowed there.
    assert!(area_has_weather(19));
    assert!(area_has_weather(20));
    // Normal outdoor + unlisted-default areas have weather.
    assert!(area_has_weather(1));
    assert!(area_has_weather(200));
    // Underground/indoor/arena areas never have weather.
    assert!(!area_has_weather(12));
    assert!(!area_has_weather(33));
}

fn game_date_with_yday(yday: i64) -> GameDate {
    GameDate {
        yday,
        ..GameDate::default()
    }
}

#[test]
fn current_season_matches_c_yday_thresholds_and_equinox_overrides() {
    assert_eq!(current_season(&game_date_with_yday(0)), SEASON_WINTER);
    assert_eq!(current_season(&game_date_with_yday(89)), SEASON_WINTER);
    assert_eq!(current_season(&game_date_with_yday(90)), SEASON_SPRING);
    assert_eq!(current_season(&game_date_with_yday(179)), SEASON_SPRING);
    assert_eq!(current_season(&game_date_with_yday(180)), SEASON_SUMMER);
    assert_eq!(current_season(&game_date_with_yday(269)), SEASON_SUMMER);
    assert_eq!(current_season(&game_date_with_yday(270)), SEASON_AUTUMN);
    assert_eq!(current_season(&game_date_with_yday(359)), SEASON_AUTUMN);

    // An equinox/solstice flag overrides the plain `yday` range check even
    // outside its usual window (mirrors C's `||` short-circuit order).
    let mut date = game_date_with_yday(5);
    date.summer_solstice = true;
    assert_eq!(current_season(&date), SEASON_SUMMER);
}

#[test]
fn pick_seasonal_weather_walks_the_cumulative_distribution() {
    // Spring: [45, 35, 20, 0, 0, 0] -> Clear/Rain/Storm only.
    assert_eq!(pick_seasonal_weather(0, |_| 0), 0);
    assert_eq!(pick_seasonal_weather(0, |_| 44), 0);
    assert_eq!(pick_seasonal_weather(0, |_| 45), 1);
    assert_eq!(pick_seasonal_weather(0, |_| 79), 1);
    assert_eq!(pick_seasonal_weather(0, |_| 80), 2);
    assert_eq!(pick_seasonal_weather(0, |_| 99), 2);

    // Winter: [35, 5, 10, 50, 0, 0] -> Snow is the majority weight,
    // occupying the cumulative range [50, 100).
    assert_eq!(pick_seasonal_weather(3, |_| 50), 3);
    assert_eq!(pick_seasonal_weather(3, |_| 99), 3);
    assert_eq!(pick_seasonal_weather(3, |_| 0), 0);
}

#[test]
fn pick_intensity_matches_c_thresholds() {
    assert_eq!(pick_intensity(|_| 0), 1);
    assert_eq!(pick_intensity(|_| 49), 1);
    assert_eq!(pick_intensity(|_| 50), 2);
    assert_eq!(pick_intensity(|_| 79), 2);
    assert_eq!(pick_intensity(|_| 80), 3);
    assert_eq!(pick_intensity(|_| 99), 3);
}

#[test]
fn update_weather_tick_periodic_change_never_repeats_and_reschedules() {
    let mut weather = WeatherState::default();
    weather.weather_change_time = 0; // already due
    let date = game_date_with_yday(10);
    // Force every roll to `0`: picks Clear from the weighted table, which
    // equals `current_weather` (also Clear), so C's "don't pick the same
    // weather twice" modulo bump must kick in.
    let changed = update_weather_tick(&mut weather, &date, 100, |_| 0);

    assert!(changed);
    assert_ne!(weather.current_weather, 0);
    assert_eq!(weather.current_weather, 1); // (0 + 1) % 6
    assert!(weather.is_transitioning);
    assert_eq!(weather.transition_start, 100);
    assert_eq!(weather.transition_duration, WEATHER_TRANSITION_TIME);
    assert_eq!(weather.weather_intensity, 1);
    assert_eq!(weather.weather_change_time, 100 + WEATHER_DURATION_MIN);
}

#[test]
fn update_weather_tick_is_a_noop_before_the_scheduled_change_time_and_same_season() {
    let mut weather = WeatherState::default();
    weather.weather_change_time = 1_000;
    weather.seasonal_influence = SEASON_WINTER;
    let date = game_date_with_yday(10); // winter, same as seasonal_influence

    let changed = update_weather_tick(&mut weather, &date, 100, |_| {
        panic!("no random roll should happen when nothing is due")
    });

    assert!(!changed);
    assert_eq!(
        weather,
        WeatherState {
            weather_change_time: 1_000,
            seasonal_influence: SEASON_WINTER,
            ..WeatherState::default()
        }
    );
}

#[test]
fn update_weather_tick_completes_a_pending_transition() {
    let mut weather = WeatherState::default();
    weather.weather_change_time = 1_000;
    weather.seasonal_influence = SEASON_WINTER;
    weather.is_transitioning = true;
    weather.transition_start = 0;
    weather.transition_duration = 50;
    weather.current_weather = 2;
    let date = game_date_with_yday(10);

    let changed = update_weather_tick(&mut weather, &date, 60, |_| {
        panic!("no random roll needed just to finish a transition")
    });

    assert!(!changed); // transition completing isn't itself a "weather changed" event.
    assert!(!weather.is_transitioning);
    assert_eq!(weather.prev_weather, 2);
}

#[test]
fn transition_progress_byte_matches_c_formula() {
    let mut weather = WeatherState::default();
    assert_eq!(transition_progress_byte(&weather, 0), 255); // not transitioning.

    weather.is_transitioning = true;
    weather.transition_start = 100;
    weather.transition_duration = 200;
    assert_eq!(transition_progress_byte(&weather, 100), 0);
    assert_eq!(transition_progress_byte(&weather, 200), 127); // halfway.
    assert_eq!(transition_progress_byte(&weather, 300), 255);
    assert_eq!(transition_progress_byte(&weather, 1_000), 255); // clamped.
}

#[test]
fn day_night_position_matches_c_formula_at_key_points() {
    let mut date = GameDate {
        sunrise: HOUR_LEN * 6,
        sunset: HOUR_LEN * 18,
        ..GameDate::default()
    };

    date.hour = 0;
    date.minute = 0;
    assert_eq!(day_night_position(&date), 0); // Midnight.

    date.hour = 6;
    assert_eq!(day_night_position(&date), 64); // Sunrise.

    date.hour = 12;
    assert_eq!(day_night_position(&date), 128); // Noon.

    date.hour = 18;
    assert_eq!(day_night_position(&date), 192); // Sunset.

    date.hour = 23;
    assert!(day_night_position(&date) > 192);
}

#[test]
fn weather_packet_bytes_matches_legacy_wire_layout() {
    let weather = WeatherState {
        current_weather: 2,
        weather_intensity: 3,
        weather_effects: WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND,
        is_transitioning: false,
        ..WeatherState::default()
    };
    let date = GameDate {
        sunrise: HOUR_LEN * 6,
        sunset: HOUR_LEN * 18,
        hour: 12,
        minute: 0,
        ..GameDate::default()
    };

    let bytes = weather_packet_bytes(&weather, &date, 0, false);
    assert_eq!(bytes.len(), SV_WEATHER_PACKET_SIZE);
    assert_eq!(bytes[0], SV_MOD2);
    assert_eq!(bytes[1], 6);
    assert_eq!(bytes[2], SV_VIS_WEATHER);
    assert_eq!(bytes[3], 2);
    assert_eq!(bytes[4], 3);
    assert_eq!(bytes[5], 255); // not transitioning.
    assert_eq!(bytes[6], 128); // noon.
    assert_eq!(bytes[7], (WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND) as u8);

    let indoor_bytes = weather_packet_bytes(&weather, &date, 0, true);
    assert_eq!(
        indoor_bytes[7],
        (WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND) as u8 | MOD_WEATHER_EFFECT_INDOOR
    );
}

#[test]
fn broadcast_weather_packet_skips_areas_without_weather() {
    let mut world = World::default();
    let character_id = CharacterId(1);
    world.add_character(login_character(
        character_id,
        &login_block("Weather"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
    }
    runtime.weather.current_weather = 1;
    runtime.weather.weather_intensity = 2;

    broadcast_weather_packet(&world, &mut runtime, 12); // area 12 = Mines, no weather.
    assert!(runtime.tick_out.get(&1).is_none());

    broadcast_weather_packet(&world, &mut runtime, 1); // area 1 = Cameron, has weather.
    let payloads = runtime.tick_out.get(&1).expect("packet queued");
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0][0], SV_MOD2);
    assert_eq!(payloads[0][3], 1);
}

#[test]
fn init_player_weather_packet_no_weather_area_forces_indoor_clear() {
    // C `send_indoor_state`'s `!area_has_weather(areaID)` branch
    // (`weather_client.c:1321-1325`): area 12 (Mines) never has weather,
    // so login always sends forced Clear + Indoor regardless of the live
    // `WeatherState`, tile, or time of day.
    let weather = WeatherState {
        current_weather: 2,
        weather_intensity: 3,
        weather_effects: WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND,
        ..WeatherState::default()
    };
    let date = GameDate {
        hour: 12,
        ..GameDate::default()
    };

    let bytes = init_player_weather_packet(&weather, &date, 0, 12, false);
    assert_eq!(bytes[3], 0); // MOD_WEATHER_CLEAR
    assert_eq!(bytes[4], 0); // intensity forced to 0.
    assert_eq!(bytes[5], 255);
    assert_eq!(bytes[6], 0);
    assert_eq!(bytes[7], MOD_WEATHER_EFFECT_INDOOR);

    // Even an indoor tile in a no-weather area produces the same packet.
    let indoor_bytes = init_player_weather_packet(&weather, &date, 0, 12, true);
    assert_eq!(indoor_bytes, bytes);
}

#[test]
fn init_player_weather_packet_indoor_tile_keeps_real_weather_but_adds_indoor_flag() {
    // C `send_indoor_state`'s `else` branch (`weather_client.c:1326-1331`):
    // an indoor tile in a weather-capable area still reports the real area
    // weather/intensity/effects (so the UI/`/weather` command works), just
    // with the `INDOOR` bit set and `transition`/`day_night` hardcoded.
    let weather = WeatherState {
        current_weather: 1,
        weather_intensity: 2,
        weather_effects: WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND,
        ..WeatherState::default()
    };
    let date = GameDate {
        hour: 12,
        ..GameDate::default()
    };

    let bytes = init_player_weather_packet(&weather, &date, 0, 1, true);
    assert_eq!(bytes[3], 1);
    assert_eq!(bytes[4], 2);
    assert_eq!(bytes[5], 255);
    assert_eq!(bytes[6], 0);
    assert_eq!(
        bytes[7],
        (WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND) as u8 | MOD_WEATHER_EFFECT_INDOOR
    );
}

#[test]
fn init_player_weather_packet_outdoors_matches_send_weather_update() {
    // C `send_weather_update` (`weather_client.c:69-93`): outdoors gets the
    // real computed transition/day-night bytes and no `INDOOR` flag.
    let mut weather = WeatherState {
        current_weather: 2,
        weather_intensity: 3,
        weather_effects: WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND,
        ..WeatherState::default()
    };
    weather.transition_start = 0;
    weather.transition_duration = 0;
    let date = GameDate {
        sunrise: HOUR_LEN * 6,
        sunset: HOUR_LEN * 18,
        hour: 12,
        minute: 0,
        ..GameDate::default()
    };

    let bytes = init_player_weather_packet(&weather, &date, 0, 1, false);
    assert_eq!(bytes[3], 2);
    assert_eq!(bytes[4], 3);
    assert_eq!(bytes[5], 255); // not transitioning.
    assert_eq!(bytes[6], 128); // noon.
    assert_eq!(bytes[7], (WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND) as u8);
}

#[test]
fn init_player_weather_packet_outdoors_coerces_disallowed_weather_to_clear() {
    // C `send_weather_update`'s `!is_weather_allowed_in_area` coercion:
    // the weather byte is forced to Clear but intensity/effects are left
    // as-is (matching C's own behavior exactly, quirk and all).
    let weather = WeatherState {
        current_weather: 4, // MOD_WEATHER_SANDSTORM, not allowed in area 1.
        weather_intensity: 2,
        weather_effects: WEATHER_EFFECT_SLOW,
        ..WeatherState::default()
    };
    let date = GameDate::default();

    let bytes = init_player_weather_packet(&weather, &date, 0, 1, false);
    assert_eq!(bytes[3], 0); // coerced to Clear.
    assert_eq!(bytes[4], 2); // intensity left untouched.
    assert_eq!(bytes[7], WEATHER_EFFECT_SLOW as u8); // effects left untouched.
}
