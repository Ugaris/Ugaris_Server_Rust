// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
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

    // Light rain: slow+blind+slip+elemental (wet), no skill mods ({0} in
    // the C table).
    assert_eq!(
        calculate_weather_effects(1, 1),
        WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SLIP | WEATHER_EFFECT_ELEMENTAL
    );
    // Moderate rain gains a skill mod (V_PERCEPT = -5).
    assert_eq!(
        calculate_weather_effects(1, 2),
        WEATHER_EFFECT_SLOW
            | WEATHER_EFFECT_BLIND
            | WEATHER_EFFECT_SLIP
            | WEATHER_EFFECT_SKILL
            | WEATHER_EFFECT_ELEMENTAL
    );

    // Heavy storm: slow+blind+slip+skill+elemental (wet), no damage, but a
    // nonzero `lightning_chance` (30%) so `WEATHER_EFFECT_LIGHTNING` is set
    // too - the only weather/intensity cell where that bit ever appears.
    assert_eq!(
        calculate_weather_effects(2, 3),
        WEATHER_EFFECT_SLOW
            | WEATHER_EFFECT_BLIND
            | WEATHER_EFFECT_SLIP
            | WEATHER_EFFECT_SKILL
            | WEATHER_EFFECT_LIGHTNING
            | WEATHER_EFFECT_ELEMENTAL
    );
    // Light/moderate storm also carry the lightning bit (5%/15% chance).
    assert_eq!(
        calculate_weather_effects(2, 1),
        WEATHER_EFFECT_SLOW
            | WEATHER_EFFECT_BLIND
            | WEATHER_EFFECT_SLIP
            | WEATHER_EFFECT_SKILL
            | WEATHER_EFFECT_LIGHTNING
            | WEATHER_EFFECT_ELEMENTAL
    );

    // Snow carries the cold elemental debuff bit too.
    assert_eq!(
        calculate_weather_effects(3, 1),
        WEATHER_EFFECT_SLOW
            | WEATHER_EFFECT_BLIND
            | WEATHER_EFFECT_SLIP
            | WEATHER_EFFECT_SKILL
            | WEATHER_EFFECT_ELEMENTAL
    );

    // Light sandstorm already has damage=1 (unlike moderate/heavy-only
    // damage in the old simplified port), plus the scorched elemental bit.
    assert_eq!(
        calculate_weather_effects(4, 1),
        WEATHER_EFFECT_SLOW
            | WEATHER_EFFECT_BLIND
            | WEATHER_EFFECT_DAMAGE
            | WEATHER_EFFECT_SLIP
            | WEATHER_EFFECT_SKILL
            | WEATHER_EFFECT_ELEMENTAL
    );

    // Light fog: C's table has move_mod=95 (<100), so SLOW must be set -
    // the bug this test guards against. Fog never carries an elemental
    // debuff (DEBUFF_NONE at every intensity).
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
fn weather_damage_message_only_sandstorm_is_reachable_today() {
    // `MOD_WEATHER_SANDSTORM` is the only weather type with a nonzero
    // `damage` in the current `WEATHER_EFFECTS` table, so it's the only
    // branch `main.rs`'s tick loop can actually reach - but the switch
    // itself is transcribed for all three cases per C source.
    assert_eq!(
        weather_damage_message(2), // MOD_WEATHER_STORM
        Some("Lightning strikes nearby!")
    );
    assert_eq!(
        weather_damage_message(3), // MOD_WEATHER_SNOW
        Some("The freezing cold bites into you!")
    );
    assert_eq!(
        weather_damage_message(4), // MOD_WEATHER_SANDSTORM
        Some("The stinging sand hurts you!")
    );
    assert_eq!(weather_damage_message(0), None); // Clear.
    assert_eq!(weather_damage_message(1), None); // Rain.
    assert_eq!(weather_damage_message(5), None); // Fog.
}

#[test]
fn lightning_strike_chance_matches_storm_table_and_is_zero_elsewhere() {
    assert_eq!(lightning_strike_chance(2, 1), 5); // Light storm.
    assert_eq!(lightning_strike_chance(2, 2), 15); // Moderate storm.
    assert_eq!(lightning_strike_chance(2, 3), 30); // Heavy storm.
    assert_eq!(lightning_strike_chance(1, 3), 0); // Rain never has lightning.
    assert_eq!(lightning_strike_chance(4, 3), 0); // Sandstorm never has lightning.
    assert_eq!(lightning_strike_chance(2, 0), 0); // Invalid intensity.
}

#[test]
fn lightning_strike_damage_amount_matches_c_switch_at_every_intensity() {
    // `RANDOM(n)` here always returns 0, isolating the base value at each
    // intensity boundary.
    assert_eq!(lightning_strike_damage_amount(1, |_| 0), 10); // Light: 10-20.
    assert_eq!(lightning_strike_damage_amount(1, |_| 9), 19);
    assert_eq!(lightning_strike_damage_amount(2, |_| 0), 20); // Moderate: 20-40.
    assert_eq!(lightning_strike_damage_amount(2, |_| 19), 39);
    assert_eq!(lightning_strike_damage_amount(3, |_| 0), 40); // Heavy: 40-80.
    assert_eq!(lightning_strike_damage_amount(3, |_| 39), 79);
    // Unreachable-in-practice `default` branch (intensity 0/None).
    assert_eq!(lightning_strike_damage_amount(0, |_| 0), 15);
}

#[test]
fn elemental_debuff_type_matches_c_table_at_every_boundary() {
    assert_eq!(elemental_debuff_type(0, 1), DEBUFF_NONE); // Clear.
    assert_eq!(elemental_debuff_type(1, 1), DEBUFF_WET); // Rain, every intensity.
    assert_eq!(elemental_debuff_type(1, 3), DEBUFF_WET);
    assert_eq!(elemental_debuff_type(2, 1), DEBUFF_WET); // Storm, every intensity.
    assert_eq!(elemental_debuff_type(2, 3), DEBUFF_WET);
    assert_eq!(elemental_debuff_type(3, 1), DEBUFF_COLD); // Snow, every intensity.
    assert_eq!(elemental_debuff_type(3, 3), DEBUFF_COLD);
    assert_eq!(elemental_debuff_type(4, 1), DEBUFF_SCORCHED); // Sandstorm.
    assert_eq!(elemental_debuff_type(4, 3), DEBUFF_SCORCHED);
    assert_eq!(elemental_debuff_type(5, 1), DEBUFF_NONE); // Fog: no debuff.
    assert_eq!(elemental_debuff_type(5, 3), DEBUFF_NONE);
    // Invalid inputs are always a no-op.
    assert_eq!(elemental_debuff_type(6, 1), DEBUFF_NONE);
    assert_eq!(elemental_debuff_type(1, 0), DEBUFF_NONE);
    assert_eq!(elemental_debuff_type(1, 4), DEBUFF_NONE);
}

#[test]
fn elemental_debuff_message_matches_c_switch_letter_for_letter() {
    assert_eq!(
        elemental_debuff_message(DEBUFF_WET),
        Some("You are getting soaked by the rain.")
    );
    assert_eq!(
        elemental_debuff_message(DEBUFF_COLD),
        Some("The cold is seeping into your bones.")
    );
    assert_eq!(
        elemental_debuff_message(DEBUFF_SCORCHED),
        Some("The scorching heat is draining your energy.")
    );
    assert_eq!(elemental_debuff_message(DEBUFF_NONE), None);
}

#[test]
fn should_notify_elemental_debuff_gates_to_once_per_ten_seconds() {
    let ten_seconds = TICKS_PER_SECOND * 10;
    // Never notified before (last_notify defaults to 0): fires once ticker
    // reaches the threshold, not before.
    assert!(!should_notify_elemental_debuff(0, ten_seconds - 1));
    assert!(should_notify_elemental_debuff(0, ten_seconds));
    // Just notified: doesn't fire again until another full 10 seconds pass.
    assert!(!should_notify_elemental_debuff(100, 100 + ten_seconds - 1));
    assert!(should_notify_elemental_debuff(100, 100 + ten_seconds));
}

#[test]
fn thunder_screen_flash_intensity_wraps_around_past_dist_twenty() {
    // C `broadcast_weather_thunder_effect`'s `(uint8_t)(200 - dist*10)`
    // cast happens *before* the `< 50` floor check, so far-away players
    // (beyond `dist=20`) see the subtraction go negative and wrap back up
    // into the 200s instead of being floored to 50 - replicating a real
    // C integer-truncation quirk, not a bug in this port.
    assert_eq!(thunder_screen_flash_intensity(0), 200); // Epicenter.
    assert_eq!(thunder_screen_flash_intensity(10), 100);
    assert_eq!(thunder_screen_flash_intensity(15), 50); // Exactly at floor.
    assert_eq!(thunder_screen_flash_intensity(16), 50); // Floored (40 < 50).
    assert_eq!(thunder_screen_flash_intensity(20), 50); // Floored (0 < 50).
    assert_eq!(thunder_screen_flash_intensity(21), 246); // Wraps: -10 as u8.
    assert_eq!(thunder_screen_flash_intensity(24), 216); // Wraps: -40 as u8.
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
    assert!(!runtime.tick_out.contains_key(&1));

    broadcast_weather_packet(&world, &mut runtime, 1); // area 1 = Cameron, has weather.
    let payloads = runtime.tick_out.get(&1).expect("packet queued");
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0][0], SV_MOD2);
    assert_eq!(payloads[0][3], 1);
}

#[test]
fn broadcast_weather_thunder_effect_sends_bolt_and_fading_flash_within_radius() {
    let mut world = World::default();
    // Struck character, at the epicenter itself.
    let struck_id = CharacterId(1);
    let mut struck = login_character(struck_id, &login_block("Struck"), 1, 50, 50);
    struck.x = 50;
    struck.y = 50;
    world.add_character(struck);
    // Within the radius-12 box but far enough to hit the flash-intensity
    // floor (dist = 10 + 10 = 20 <= radius on both axes).
    let nearby_id = CharacterId(2);
    let mut nearby = login_character(nearby_id, &login_block("Nearby"), 1, 60, 60);
    nearby.x = 60;
    nearby.y = 60;
    world.add_character(nearby);
    // Outside the radius-12 box entirely.
    let far_id = CharacterId(3);
    let mut far = login_character(far_id, &login_block("Far"), 1, 100, 100);
    far.x = 100;
    far.y = 100;
    world.add_character(far);

    let mut runtime = ServerRuntime::default();
    for (session_id, character_id) in [(1u64, struck_id), (2, nearby_id), (3, far_id)] {
        let (commands, _rx) = mpsc::channel(16);
        runtime.connect(session_id, commands, 0);
        if let Some(player) = runtime.players.get_mut(&session_id) {
            player.character_id = Some(character_id);
        }
    }

    broadcast_weather_thunder_effect(&world, &mut runtime, 50, 50, 12, 3);

    // The struck player and the in-range nearby player each get exactly
    // two SFX packets (bolt + screen flash); the far player gets none.
    let struck_payloads = runtime.tick_out.get(&1).expect("struck player queued SFX");
    assert_eq!(struck_payloads.len(), 2);
    assert_eq!(struck_payloads[0][0], SV_MOD2);
    assert_eq!(struck_payloads[0][2], ugaris_protocol::mod_sfx::SV_VIS_SFX);
    assert_eq!(
        struck_payloads[0][3],
        ugaris_protocol::mod_sfx::SFX_LIGHTNING_STRIKE
    );
    assert_eq!(struck_payloads[0][8], 255); // Heavy intensity bolt.
    assert_eq!(
        struck_payloads[1][3],
        ugaris_protocol::mod_sfx::SFX_SCREEN_FLASH
    );
    assert_eq!(struck_payloads[1][8], 200); // dist=0 -> no fade.

    let nearby_payloads = runtime.tick_out.get(&2).expect("nearby player queued SFX");
    assert_eq!(nearby_payloads.len(), 2);
    assert_eq!(nearby_payloads[1][8], 50); // dist=20 -> floored.

    assert!(!runtime.tick_out.contains_key(&3));
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
