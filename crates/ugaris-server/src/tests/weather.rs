use super::*;

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
