use super::*;
use crate::character_driver::{CDR_TEUFELDEMON, NT_CHAR};

const AREA_ID: u16 = 34;

fn teufeldemon_npc(id: u32) -> Character {
    let mut demon = character(id);
    demon.name = "Demon".into();
    demon.driver = CDR_TEUFELDEMON;
    // C `is_demon` sprite (`teufel.c:366-371`) - matches the zone data's
    // own `teufer1`/`teufer2`/`teufer3` sprites.
    demon.sprite = 27;
    demon.group = 3;
    demon.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    demon
}

fn player_char(id: u32, sprite: i32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = "Godmode".into();
    player.sprite = sprite;
    player
}

fn lit_world() -> World {
    let mut world = World::default();
    for x in 0..20 {
        for y in 0..20 {
            world.map.tile_mut(x, y).unwrap().light = 255;
        }
    }
    world
}

fn recorded_enemies(world: &World, demon_id: CharacterId) -> Vec<CharacterId> {
    world
        .characters
        .get(&demon_id)
        .and_then(|demon| demon.fight_driver.as_ref())
        .map(|data| data.enemies.iter().map(|enemy| enemy.target_id).collect())
        .unwrap_or_default()
}

#[test]
fn teufeldemon_attacks_disguise_less_player_seen_via_nt_char() {
    let mut world = lit_world();
    assert!(world.spawn_character(teufeldemon_npc(1), 10, 10));
    // sprite 0 == no demon-suit disguise.
    assert!(world.spawn_character(player_char(2, 0), 11, 10));

    if let Some(demon) = world.characters.get_mut(&CharacterId(1)) {
        demon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), AREA_ID);
    assert!(outcomes.is_empty());
    assert_eq!(
        recorded_enemies(&world, CharacterId(1)),
        vec![CharacterId(2)]
    );
}

#[test]
fn teufeldemon_ignores_demon_disguised_player() {
    let mut world = lit_world();
    assert!(world.spawn_character(teufeldemon_npc(1), 10, 10));
    // sprite 27 == earth-demon-suit, matches `is_demon`.
    assert!(world.spawn_character(player_char(2, 27), 11, 10));

    if let Some(demon) = world.characters.get_mut(&CharacterId(1)) {
        demon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_simple_baddy_message_actions(CharacterId(1), AREA_ID);
    assert!(recorded_enemies(&world, CharacterId(1)).is_empty());
}

#[test]
fn teufeldemon_ignores_non_player_sighting() {
    let mut world = lit_world();
    assert!(world.spawn_character(teufeldemon_npc(1), 10, 10));
    let mut other_npc = character(2);
    other_npc.sprite = 0;
    assert!(world.spawn_character(other_npc, 11, 10));

    if let Some(demon) = world.characters.get_mut(&CharacterId(1)) {
        demon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_simple_baddy_message_actions(CharacterId(1), AREA_ID);
    assert!(recorded_enemies(&world, CharacterId(1)).is_empty());
}

#[test]
fn teufeldemon_widened_gate_lets_recorded_enemy_be_attacked() {
    let mut world = lit_world();
    assert!(world.spawn_character(teufeldemon_npc(1), 10, 10));
    assert!(world.spawn_character(player_char(2, 0), 11, 10));

    if let Some(demon) = world.characters.get_mut(&CharacterId(1)) {
        demon.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_simple_baddy_message_actions(CharacterId(1), AREA_ID);
    assert_eq!(
        recorded_enemies(&world, CharacterId(1)),
        vec![CharacterId(2)]
    );

    // The `CDR_TEUFELDEMON` gate widening in `world::npc_fight` is what
    // lets the recorded enemy actually be attacked, same as
    // `CDR_PENTER`/`CDR_TWOROBBER`.
    assert!(world.process_simple_baddy_attack_action(CharacterId(1), AREA_ID));
    let demon = world.characters.get(&CharacterId(1)).unwrap();
    assert_ne!(demon.action, action::IDLE);
}
