use super::*;
use crate::character_driver::{
    apply_labgnome_create_message, LabGnomeDriverData, CDR_LABGNOMEDRIVER, NTID_LABGNOMETORCH,
    NT_CREATE, NT_NPC,
};
use crate::item_driver::IDR_LABTORCH;

fn gnome_npc(id: u32) -> Character {
    let mut gnome = character(id);
    gnome.name = "Gnome Worker".into();
    gnome.driver = CDR_LABGNOMEDRIVER;
    gnome
}

fn gnome_state(world: &World, id: CharacterId) -> LabGnomeDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::LabGnome(data)) => data,
        _ => panic!("expected lab-gnome driver state"),
    }
}

fn unlit_torch(id: u32, x: u16, y: u16) -> Item {
    let mut torch = item(id, ItemFlags::USE);
    torch.driver = IDR_LABTORCH;
    torch.driver_data = vec![0, 0];
    torch.x = x;
    torch.y = y;
    torch
}

#[test]
fn apply_labgnome_create_message_parses_args_and_master_gets_immortal() {
    let mut character = character(1);
    apply_labgnome_create_message(&mut character, Some("aggressive=1;helper=0;master=1;"));

    let Some(CharacterDriverState::LabGnome(data)) = character.driver_state else {
        panic!("expected lab-gnome driver state");
    };
    assert!(data.master);
    assert!(data.aggressive);
    assert!(!data.helper);
    assert!(character.flags.contains(CharacterFlags::IMMORTAL));
    assert_eq!(
        character
            .driver_messages
            .iter()
            .filter(|message| message.message_type == NT_CREATE)
            .count(),
        1
    );
}

#[test]
fn apply_labgnome_create_message_plain_guard_is_not_immortal() {
    let mut character = character(1);
    apply_labgnome_create_message(&mut character, Some("aggressive=0;helper=1;"));

    assert!(!character.flags.contains(CharacterFlags::IMMORTAL));
    let Some(CharacterDriverState::LabGnome(data)) = character.driver_state else {
        panic!("expected lab-gnome driver state");
    };
    assert!(!data.master);
    assert!(!data.fighter);
}

#[test]
fn nt_create_scans_nearby_unlit_torches_farthest_first() {
    let mut world = World::default();
    let mut gnome = gnome_npc(1);
    gnome.driver_state = Some(CharacterDriverState::LabGnome(LabGnomeDriverData::default()));
    assert!(world.spawn_character(gnome, 20, 20));
    if let Some(gnome) = world.characters.get_mut(&CharacterId(1)) {
        gnome.push_driver_message(NT_CREATE, 0, 0, 0);
    }
    // Near torch (distance 2) and far torch (distance 10), both within the
    // 15-tile scan radius.
    world.add_item(unlit_torch(9, 22, 20));
    world.add_item(unlit_torch(10, 30, 20));

    world.process_labgnome_actions(1);

    let state = gnome_state(&world, CharacterId(1));
    assert_eq!(state.torches, vec![ItemId(10), ItemId(9)]);
}

#[test]
fn plain_guard_walks_to_and_lights_an_adjacent_unlit_torch() {
    let mut world = World::default();
    let mut gnome = gnome_npc(1);
    gnome.driver_state = Some(CharacterDriverState::LabGnome(LabGnomeDriverData {
        torches: vec![ItemId(9)],
        ..Default::default()
    }));
    assert!(world.spawn_character(gnome, 20, 21));
    let torch = unlit_torch(9, 20, 20);
    world.map.tile_mut(20, 20).unwrap().item = 9;
    world.add_item(torch);

    let acted = world.process_labgnome_actions(1);

    assert_eq!(acted, 1);
    let gnome = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(gnome.action, action::USE);
    assert_eq!(
        gnome_state(&world, CharacterId(1)).usetarget,
        Some(ItemId(9))
    );
}

#[test]
fn fighter_gnome_never_scans_for_torches() {
    let mut world = World::default();
    let mut gnome = gnome_npc(1);
    gnome.driver_state = Some(CharacterDriverState::LabGnome(LabGnomeDriverData {
        fighter: true,
        ..Default::default()
    }));
    assert!(world.spawn_character(gnome, 20, 20));
    if let Some(gnome) = world.characters.get_mut(&CharacterId(1)) {
        gnome.push_driver_message(NT_CREATE, 0, 0, 0);
    }
    world.add_item(unlit_torch(9, 21, 20));

    world.process_labgnome_actions(1);

    assert!(gnome_state(&world, CharacterId(1)).torches.is_empty());
}

#[test]
fn torch_extinguished_by_a_tracked_torch_makes_the_extinguisher_the_victim() {
    let mut world = World::default();
    let mut gnome = gnome_npc(1);
    gnome.driver_state = Some(CharacterDriverState::LabGnome(LabGnomeDriverData {
        torches: vec![ItemId(9)],
        ..Default::default()
    }));
    assert!(world.spawn_character(gnome, 20, 20));
    let mut attacker = character(7);
    attacker
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::ALIVE);
    attacker.group = 99;
    // Within `char_see_char_nolos`'s unconditional `dist < 3` visibility
    // branch, so light level doesn't matter for this test.
    assert!(world.spawn_character(attacker, 21, 21));
    if let Some(gnome) = world.characters.get_mut(&CharacterId(1)) {
        gnome.push_driver_message(NT_NPC, NTID_LABGNOMETORCH, 9, 7);
    }

    world.process_labgnome_actions(1);

    assert_eq!(
        gnome_state(&world, CharacterId(1)).victim,
        Some(CharacterId(7))
    );
}

#[test]
fn torch_message_for_an_untracked_torch_is_ignored() {
    let mut world = World::default();
    let mut gnome = gnome_npc(1);
    gnome.driver_state = Some(CharacterDriverState::LabGnome(LabGnomeDriverData {
        torches: vec![ItemId(9)],
        ..Default::default()
    }));
    assert!(world.spawn_character(gnome, 20, 20));
    let mut attacker = character(7);
    attacker
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::ALIVE);
    attacker.group = 99;
    assert!(world.spawn_character(attacker, 21, 21));
    if let Some(gnome) = world.characters.get_mut(&CharacterId(1)) {
        // Item 42 is not in this gnome's tracked torch list.
        gnome.push_driver_message(NT_NPC, NTID_LABGNOMETORCH, 42, 7);
    }

    world.process_labgnome_actions(1);

    assert_eq!(gnome_state(&world, CharacterId(1)).victim, None);
}

#[test]
fn labgnome_death_driver_speaks_when_text_flag_is_set() {
    let mut world = World::default();
    let mut gnome = gnome_npc(1);
    gnome.driver_state = Some(CharacterDriverState::LabGnome(LabGnomeDriverData {
        text: true,
        ..Default::default()
    }));
    assert!(world.spawn_character(gnome, 20, 20));
    let mut killer = character(7);
    killer.name = "Hero".into();
    assert!(world.spawn_character(killer, 21, 20));

    world.apply_labgnome_death_driver(CharacterId(1), CharacterId(7));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hero me killed")));
}

#[test]
fn deathfibrin_strike_hurts_master_and_restores_immortality() {
    use crate::item_driver::{ItemDriverRequest, IDR_DEATHFIBRIN};

    let mut world = World::default();
    let mut master = gnome_npc(9);
    master.name = "Immortal Master".into();
    master.driver_state = Some(CharacterDriverState::LabGnome(LabGnomeDriverData {
        master: true,
        ..Default::default()
    }));
    master
        .flags
        .insert(CharacterFlags::IMMORTAL | CharacterFlags::ALIVE);
    master.hp = 20 * crate::entity::POWERSCALE;
    master.values[1][CharacterValue::Hp as usize] = 20;
    assert!(world.spawn_character(master, 30, 30));

    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(player, 31, 30));

    let mut staff = item(7, ItemFlags::USE | ItemFlags::TAKE);
    staff.driver = IDR_DEATHFIBRIN;
    staff.sprite = 10418;
    staff.carried_by = Some(CharacterId(1));
    world.add_item(staff);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_DEATHFIBRIN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
    );

    assert!(matches!(
        outcome,
        crate::item_driver::ItemDriverOutcome::DeathfibrinStrike {
            vanished: false,
            ..
        }
    ));
    let master = world.characters.get(&CharacterId(9)).unwrap();
    assert!(master.hp < 20 * crate::entity::POWERSCALE);
    assert!(master.flags.contains(CharacterFlags::IMMORTAL));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Oh no! Deathfibrin hurts.")));
}

#[test]
fn labgnome_death_driver_silent_without_text_flag() {
    let mut world = World::default();
    let mut gnome = gnome_npc(1);
    gnome.driver_state = Some(CharacterDriverState::LabGnome(LabGnomeDriverData::default()));
    assert!(world.spawn_character(gnome, 20, 20));
    let mut killer = character(7);
    killer.name = "Hero".into();
    assert!(world.spawn_character(killer, 21, 20));

    world.apply_labgnome_death_driver(CharacterId(1), CharacterId(7));

    let texts = world.drain_pending_area_texts();
    assert!(texts.is_empty());
}
