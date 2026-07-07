use super::*;
use crate::character_driver::{RobberDriverData, CDR_ROBBER, NT_GOTHIT};

fn robber_npc(id: u32) -> Character {
    let mut robber = character(id);
    robber.name = "Robber".into();
    robber.driver = CDR_ROBBER;
    robber.driver_state = Some(CharacterDriverState::Robber(RobberDriverData::default()));
    robber
}

fn robber_state(world: &World, id: CharacterId) -> RobberDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Robber(data)) => data,
        _ => panic!("expected robber driver state"),
    }
}

fn torch_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
            torch:
              name="Torch"
              sprite=50023
              flag=IF_TAKE
              flag=IF_WNLHAND
              flag=IF_USE
              driver=12
              arg="00007878"
            ;
            "#,
        )
        .unwrap();
    loader
}

#[test]
fn robber_walks_toward_guard_post_when_not_there() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(robber_npc(1), 20, 242));

    let acted = world.process_robber_actions(&mut loader, 1);

    assert_eq!(acted, 1);
    let robber = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(robber.action, action::WALK);
    assert_eq!(robber_state(&world, CharacterId(1)).state, 0);
}

#[test]
fn robber_waits_at_post_before_clock_gate() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(robber_npc(1), 30, 242));
    world.date.hour = 20;
    world.date.minute = 0;

    let acted = world.process_robber_actions(&mut loader, 1);

    assert_eq!(acted, 1);
    assert_eq!(robber_state(&world, CharacterId(1)).state, 0);
    let robber = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(robber.action, action::IDLE);
}

#[test]
fn robber_leaves_post_once_clock_gate_passes() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(robber_npc(1), 30, 242));
    world.date.hour = 23;
    world.date.minute = 46;

    world.process_robber_actions(&mut loader, 1);

    assert_eq!(robber_state(&world, CharacterId(1)).state, 1);
}

#[test]
fn robber_uses_ladder_at_state_two() {
    let mut world = World::default();
    let mut loader = torch_loader();
    let mut robber = robber_npc(1);
    robber.driver_state = Some(CharacterDriverState::Robber(RobberDriverData {
        state: 2,
        ..RobberDriverData::default()
    }));
    assert!(world.spawn_character(robber, 31, 238));

    let mut ladder = item(9, ItemFlags::USE);
    ladder.x = 31;
    ladder.y = 237;
    world.items.insert(ItemId(9), ladder);
    world.map.tile_mut(31, 237).unwrap().item = 9;

    let acted = world.process_robber_actions(&mut loader, 1);

    assert_eq!(acted, 1);
    let robber = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(robber.action, action::USE);
    assert_eq!(robber_state(&world, CharacterId(1)).state, 2);
}

#[test]
fn robber_resets_to_state_zero_when_ladder_missing() {
    let mut world = World::default();
    let mut loader = torch_loader();
    let mut robber = robber_npc(1);
    robber.driver_state = Some(CharacterDriverState::Robber(RobberDriverData {
        state: 2,
        ..RobberDriverData::default()
    }));
    assert!(world.spawn_character(robber, 31, 238));
    // no item placed at (31, 237): C `charlog(cn, "my ladder is gone!")`
    // + `dat->state = 0`.

    world.process_robber_actions(&mut loader, 1);

    assert_eq!(robber_state(&world, CharacterId(1)).state, 0);
}

#[test]
fn robber_equips_a_torch_when_missing() {
    let mut world = World::default();
    let mut loader = torch_loader();
    assert!(world.spawn_character(robber_npc(1), 30, 242));
    world.date.hour = 12;

    world.process_robber_actions(&mut loader, 1);

    let robber = world.characters.get(&CharacterId(1)).unwrap();
    let torch_id = robber.inventory[8].expect("torch equipped in WN_LHAND");
    let torch = world.items.get(&torch_id).expect("torch item exists");
    assert_eq!(torch.driver, 12);
    assert_eq!(torch.carried_by, Some(CharacterId(1)));
}

#[test]
fn robber_relights_an_unlit_torch() {
    let mut world = World::default();
    let mut loader = torch_loader();
    let mut robber = robber_npc(1);
    let mut torch = item(50, ItemFlags::TAKE | ItemFlags::USE);
    torch.driver = 12;
    torch.driver_data = vec![0, 0, 120, 120];
    torch.carried_by = Some(CharacterId(1));
    robber.inventory[8] = Some(ItemId(50));
    world.items.insert(ItemId(50), torch);
    assert!(world.spawn_character(robber, 30, 242));
    world.date.hour = 12;

    world.process_robber_actions(&mut loader, 1);

    let relit = world.items.get(&ItemId(50)).unwrap();
    assert_ne!(relit.driver_data[0], 0);
}

#[test]
fn robber_tracks_victim_from_gothit_message_and_attacks_when_adjacent() {
    let mut world = World::default();
    let mut loader = torch_loader();
    let mut robber = robber_npc(1);
    robber.group = 0;
    assert!(world.spawn_character(robber, 10, 10));
    let mut attacker = character(2);
    attacker.group = 1;
    attacker.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(robber) = world.characters.get_mut(&CharacterId(1)) {
        robber.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    let acted = world.process_robber_actions(&mut loader, 1);

    assert_eq!(acted, 1);
    assert_eq!(
        robber_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
    let robber = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(robber.action, action::ATTACK1);
}

#[test]
fn robber_does_not_track_victim_from_same_group_gothit() {
    // C `if (ch[cn].group == ch[co].group) break;` - both default to
    // group 0, so this self-defense branch stays inert (see module doc
    // comment on `world::npc::area1::robber`).
    let mut world = World::default();
    let mut loader = torch_loader();
    let robber = robber_npc(1);
    assert!(world.spawn_character(robber, 10, 10));
    let attacker = character(2);
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(robber) = world.characters.get_mut(&CharacterId(1)) {
        robber.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    world.process_robber_actions(&mut loader, 1);

    assert_eq!(robber_state(&world, CharacterId(1)).victim, None);
}
