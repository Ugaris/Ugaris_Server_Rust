use super::*;
use crate::character_driver::{CharacterDriverMessage, CDR_VAMPIRE2, NT_GOTHIT};
use crate::item_driver::{IID_AREA2_DAGGERRIGHT, IID_AREA2_DAGGERWRONG};

fn vampire2_npc(id: u32) -> Character {
    let mut vampire2 = character(id);
    vampire2.name = "Strong Vampire".into();
    vampire2.driver = CDR_VAMPIRE2;
    vampire2.hp = 1000 * POWERSCALE;
    vampire2
}

#[test]
fn vampire2_is_killed_by_a_hit_from_the_right_dagger() {
    // C `area2.c:667-682`: `kill_char(cn, co)`, then the dagger and every
    // stray copy of both dagger IDs are destroyed.
    let mut world = World::default();
    let vampire2 = vampire2_npc(1);
    assert!(world.spawn_character(vampire2, 10, 10));

    let mut attacker = character(2);
    attacker.group = 1;
    attacker.flags |= CharacterFlags::PLAYER;
    let mut dagger = item(9, ItemFlags::USED);
    dagger.template_id = IID_AREA2_DAGGERRIGHT;
    dagger.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(9), dagger);
    attacker.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(9));
    assert!(world.spawn_character(attacker, 11, 10));

    if let Some(vampire2) = world.characters.get_mut(&CharacterId(1)) {
        vampire2.driver_messages.push(CharacterDriverMessage {
            message_type: NT_GOTHIT,
            dat1: 2,
            dat2: 5,
            dat3: 0,
            text: None,
        });
    }

    world.process_vampire2_actions(1);

    let vampire2 = world.characters.get(&CharacterId(1)).unwrap();
    assert!(vampire2.flags.contains(CharacterFlags::DEAD));
    assert!(!world.items.contains_key(&ItemId(9)));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().inventory[worn_slot::RIGHT_HAND],
        None
    );
}

#[test]
fn vampire2_survives_the_wrong_dagger_but_it_shatters() {
    // C `area2.c:684-694`.
    let mut world = World::default();
    let vampire2 = vampire2_npc(1);
    assert!(world.spawn_character(vampire2, 10, 10));

    let mut attacker = character(2);
    attacker.group = 1;
    attacker.flags |= CharacterFlags::PLAYER;
    let mut dagger = item(9, ItemFlags::USED);
    dagger.template_id = IID_AREA2_DAGGERWRONG;
    dagger.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(9), dagger);
    attacker.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(9));
    assert!(world.spawn_character(attacker, 11, 10));

    if let Some(vampire2) = world.characters.get_mut(&CharacterId(1)) {
        vampire2.driver_messages.push(CharacterDriverMessage {
            message_type: NT_GOTHIT,
            dat1: 2,
            dat2: 5,
            dat3: 0,
            text: None,
        });
    }

    world.process_vampire2_actions(1);

    let vampire2 = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!vampire2.flags.contains(CharacterFlags::DEAD));
    assert!(!world.items.contains_key(&ItemId(9)));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().inventory[worn_slot::RIGHT_HAND],
        None
    );
}

#[test]
fn vampire2_ordinary_hit_falls_through_to_the_generic_defend_branch() {
    let mut world = World::default();
    let vampire2 = vampire2_npc(1);
    assert!(world.spawn_character(vampire2, 10, 10));

    let mut attacker = character(2);
    attacker.group = 1;
    attacker.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(attacker, 11, 10));

    if let Some(vampire2) = world.characters.get_mut(&CharacterId(1)) {
        vampire2.driver_messages.push(CharacterDriverMessage {
            message_type: NT_GOTHIT,
            dat1: 2,
            dat2: 5,
            dat3: 0,
            text: None,
        });
    }

    world.process_vampire2_actions(1);

    let vampire2 = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!vampire2.flags.contains(CharacterFlags::DEAD));
    assert_eq!(vampire2.action, action::ATTACK1);
}
