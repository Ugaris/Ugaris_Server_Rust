use super::*;
use crate::character_driver::{CharacterDriverMessage, VampireDriverData, CDR_VAMPIRE, NT_CHAR};
use crate::item_driver::IID_AREA2_SUN123;

fn vampire_npc(id: u32) -> Character {
    let mut vampire = character(id);
    vampire.name = "Vampire Lord".into();
    vampire.driver = CDR_VAMPIRE;
    vampire
}

fn vampire_state(world: &World, id: CharacterId) -> VampireDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Vampire(data)) => data,
        _ => panic!("expected vampire driver state"),
    }
}

#[test]
fn vampire_seeds_killed_tick_on_first_ever_process() {
    // C `NT_CREATE`: `dat->killed = ticker;` - see module doc comment for
    // why this port seeds it on first tick instead.
    let mut world = World::default();
    world.tick.0 = 777;
    let vampire = vampire_npc(1);
    assert!(world.spawn_character(vampire, 10, 10));

    world.process_vampire_actions(1);

    assert_eq!(vampire_state(&world, CharacterId(1)).killed, 777);
}

#[test]
fn vampire_becomes_vulnerable_after_seeing_the_assembled_sun_amulet() {
    // C `area2.c:592-614`.
    let mut world = World::default();
    let mut vampire = vampire_npc(1);
    vampire.flags |= CharacterFlags::NODEATH;
    assert!(world.spawn_character(vampire, 10, 10));

    let mut amulet = item(9, ItemFlags::USED);
    amulet.template_id = IID_AREA2_SUN123;
    amulet.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(9), amulet);
    let mut wearer = character(2);
    wearer.flags |= CharacterFlags::PLAYER;
    wearer.inventory[worn_slot::NECK] = Some(ItemId(9));
    assert!(world.spawn_character(wearer, 11, 10));

    world.tick.0 = 1000;
    if let Some(vampire) = world.characters.get_mut(&CharacterId(1)) {
        vampire.driver_messages.push(CharacterDriverMessage {
            message_type: NT_CHAR,
            dat1: 2,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }

    world.process_vampire_actions(1);

    assert_eq!(vampire_state(&world, CharacterId(1)).amulet, 1000);
    let vampire = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!vampire.flags.contains(CharacterFlags::NODEATH));
}

#[test]
fn vampire_is_immortal_again_once_amulet_sighting_expires() {
    let mut world = World::default();
    let mut vampire = vampire_npc(1);
    vampire.driver_state = Some(CharacterDriverState::Vampire(VampireDriverData {
        killed: 0,
        amulet: 0,
        ..Default::default()
    }));
    world.tick.0 = TICKS_PER_SECOND * 6; // more than 5 seconds since amulet=0.
    assert!(world.spawn_character(vampire, 10, 10));

    world.process_vampire_actions(1);

    let vampire = world.characters.get(&CharacterId(1)).unwrap();
    assert!(vampire.flags.contains(CharacterFlags::NODEATH));
}

#[test]
fn vampire_fake_deaths_via_mist_teleport_when_critical_and_not_recently_shown_the_amulet() {
    // C `area2.c:610-622`: `CF_NODEATH` is only *cleared* while the sun
    // amulet was seen in the last 5 seconds (see the two tests above); the
    // fake-death mist-teleport escape fires when `CF_NODEATH` is set
    // (i.e. the amulet has *not* been seen recently) and HP is critical -
    // the vampire's normal, "still immortal" state.
    let mut world = World::default();
    let mut vampire = vampire_npc(1);
    vampire.rest_x = 50;
    vampire.rest_y = 60;
    vampire.hp = POWERSCALE / 4; // below POWERSCALE/2.
    vampire.driver_state = Some(CharacterDriverState::Vampire(VampireDriverData {
        killed: 0,
        amulet: 0, // long ago -> not vulnerable, CF_NODEATH gets (re)set.
        ..Default::default()
    }));
    world.tick.0 = TICKS_PER_SECOND * 100;
    assert!(world.spawn_character(vampire, 10, 10));

    world.process_vampire_actions(1);

    let vampire = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(vampire.hp, POWERSCALE);
    assert_eq!((vampire.x, vampire.y), (50, 60));
    assert_eq!(
        vampire_state(&world, CharacterId(1)).killed,
        TICKS_PER_SECOND * 100
    );
}

#[test]
fn vampire_roams_to_fixed_crypt_tile_after_return_home_window() {
    // C `area2.c:633-643`.
    let mut world = World::default();
    let mut vampire = vampire_npc(1);
    vampire.hp = 100 * POWERSCALE; // well above the near-death threshold.
    vampire.driver_state = Some(CharacterDriverState::Vampire(VampireDriverData {
        killed: 0,
        amulet: 0,
        ..Default::default()
    }));
    world.tick.0 = TICKS_PER_SECOND * 121; // past the 120s return-home window.
    assert!(world.spawn_character(vampire, 10, 10));

    let acted = world.process_vampire_actions(1);

    // `secure_move_driver` moves the vampire toward the fixed roam tile
    // (either by starting a walk or, for a long-distance jump beyond the
    // pathfinder's search budget, teleporting directly there - both are
    // "some action happened" per its own contract, matching every other
    // NPC that reuses it).
    assert_eq!(acted, 1);
    let vampire = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((vampire.x, vampire.y), VAMPIRE_ROAM_TILE_FOR_TEST);
}

const VAMPIRE_ROAM_TILE_FOR_TEST: (u16, u16) = (232, 123);
