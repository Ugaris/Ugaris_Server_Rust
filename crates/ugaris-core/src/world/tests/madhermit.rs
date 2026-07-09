use super::*;
use crate::character_driver::{FightDriverData, CDR_MADHERMIT, NT_CHAR};
use crate::item_driver::IDR_FLOWER;

const AC_USE: u16 = 7;

fn madhermit_npc(id: u32) -> Character {
    let mut hermit = character(id);
    hermit.name = "Mad Hermit".into();
    hermit.driver = CDR_MADHERMIT;
    hermit.driver_state = Some(CharacterDriverState::Madhermit(
        crate::world::npc::area19::MadhermitDriverData,
    ));
    hermit.fight_driver = Some(FightDriverData {
        start_dist: 30,
        char_dist: 0,
        stop_dist: 60,
        ..Default::default()
    });
    hermit
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

#[test]
fn madhermit_attacks_a_player_seen_picking_its_flowers() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(madhermit_npc(1), 10, 10));
    let mut thief = player(2, "Godmode");
    thief.action = AC_USE;
    thief.act1 = 99;
    assert!(world.spawn_character(thief, 11, 10));
    let mut flower = item(99, ItemFlags::empty());
    flower.driver = IDR_FLOWER;
    flower.carried_by = Some(CharacterId(2));
    world.add_item(flower);

    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_madhermit_actions(19);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Those flowers are mine!")));
    let hermit = world.characters.get(&CharacterId(1)).unwrap();
    let enemies = &hermit.fight_driver.as_ref().unwrap().enemies;
    assert!(enemies
        .iter()
        .any(|enemy| enemy.target_id == CharacterId(2)));
}

#[test]
fn madhermit_ignores_a_player_using_something_other_than_a_flower() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(madhermit_npc(1), 10, 10));
    let mut passerby = player(2, "Godmode");
    passerby.action = AC_USE;
    passerby.act1 = 99;
    assert!(world.spawn_character(passerby, 11, 10));
    let mut torch = item(99, ItemFlags::empty());
    torch.carried_by = Some(CharacterId(2));
    world.add_item(torch);

    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_madhermit_actions(19);

    assert!(world.drain_pending_area_texts().is_empty());
    let hermit = world.characters.get(&CharacterId(1)).unwrap();
    assert!(hermit.fight_driver.as_ref().unwrap().enemies.is_empty());
}
