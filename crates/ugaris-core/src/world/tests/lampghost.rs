use super::*;
use crate::character_driver::{
    CharacterDriverMessage, LampghostDriverData, CDR_LAMPGHOST, NT_CHAR,
};

fn lampghost_npc(id: u32) -> Character {
    let mut lampghost = character(id);
    lampghost.name = "Lamp Ghost".into();
    lampghost.driver = CDR_LAMPGHOST;
    lampghost
}

fn lampghost_state(world: &World, id: CharacterId) -> LampghostDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lampghost(data)) => data,
        _ => panic!("expected lampghost driver state"),
    }
}

/// A registered (`drdata[6] != 0`), currently-lit (`drdata[0] != 0`)
/// palace lamp - the only kind `lampghost_driver`'s target scan considers.
fn lit_registered_lamp(id: u32, x: u16, y: u16) -> Item {
    let mut lamp = item(id, ItemFlags::USE | ItemFlags::USED);
    lamp.driver = IDR_ONOFFLIGHT;
    lamp.driver_data = vec![1, 14, 0, 0, 0, 0, 1];
    lamp.x = x;
    lamp.y = y;
    lamp
}

#[test]
fn lampghost_uses_adjacent_lit_registered_lamp() {
    let mut world = World::default();
    assert!(world.spawn_character(lampghost_npc(1), 10, 11));
    let lamp = lit_registered_lamp(9, 10, 10);
    world.map.tile_mut(10, 10).unwrap().item = 9;
    world.add_item(lamp);

    let acted = world.process_lampghost_actions(1);

    assert_eq!(acted, 1);
    let lampghost = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lampghost.action, action::USE);
    assert_eq!(
        lampghost_state(&world, CharacterId(1)).claimed_lamp,
        Some(ItemId(9))
    );
    assert_eq!(
        world.area3_lamp_claims.get(&ItemId(9)),
        Some(&(CharacterId(1), map_dist(10, 11, 10, 10)))
    );
}

#[test]
fn lampghost_walks_toward_a_distant_lit_registered_lamp() {
    let mut world = World::default();
    assert!(world.spawn_character(lampghost_npc(1), 10, 10));
    let lamp = lit_registered_lamp(9, 30, 30);
    world.map.tile_mut(30, 30).unwrap().item = 9;
    world.add_item(lamp);

    let acted = world.process_lampghost_actions(1);

    assert_eq!(acted, 1);
    let lampghost = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lampghost.action, action::WALK);
    assert_eq!(
        lampghost_state(&world, CharacterId(1)).claimed_lamp,
        Some(ItemId(9))
    );
    assert!(world.area3_lamp_claims.contains_key(&ItemId(9)));
}

#[test]
fn lampghost_ignores_unlit_and_unregistered_lamps() {
    let mut world = World::default();
    assert!(world.spawn_character(lampghost_npc(1), 10, 11));

    // Registered but currently off.
    let mut off_lamp = item(9, ItemFlags::USE | ItemFlags::USED);
    off_lamp.driver = IDR_ONOFFLIGHT;
    off_lamp.driver_data = vec![0, 14, 0, 0, 0, 0, 1];
    off_lamp.x = 10;
    off_lamp.y = 10;
    world.map.tile_mut(10, 10).unwrap().item = 9;
    world.add_item(off_lamp);

    // Lit but never registered (`add_lamp`'s equivalent `drdata[6]` unset).
    let mut unregistered_lamp = item(8, ItemFlags::USE | ItemFlags::USED);
    unregistered_lamp.driver = IDR_ONOFFLIGHT;
    unregistered_lamp.driver_data = vec![1, 14];
    unregistered_lamp.x = 10;
    unregistered_lamp.y = 12;
    world.map.tile_mut(10, 12).unwrap().item = 8;
    world.add_item(unregistered_lamp);

    let acted = world.process_lampghost_actions(1);

    assert_eq!(acted, 1);
    let lampghost = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lampghost.action, action::IDLE);
    assert_eq!(lampghost_state(&world, CharacterId(1)).claimed_lamp, None);
    assert!(world.area3_lamp_claims.is_empty());
}

/// C `area3.c:2699`: `if (lamp[n].cn && cost >= lamp[n].cost) continue;` -
/// a closer lampghost can steal a farther competitor's claim.
#[test]
fn lampghost_steals_a_farther_competitors_claim_when_closer() {
    let mut world = World::default();
    assert!(world.spawn_character(lampghost_npc(1), 10, 10));
    let lamp = lit_registered_lamp(9, 12, 10);
    world.map.tile_mut(12, 10).unwrap().item = 9;
    world.add_item(lamp);
    // A competitor far away already claims it at a much higher cost.
    world
        .area3_lamp_claims
        .insert(ItemId(9), (CharacterId(99), 999));

    world.process_lampghost_actions(1);

    assert_eq!(
        world.area3_lamp_claims.get(&ItemId(9)).map(|(cn, _)| *cn),
        Some(CharacterId(1))
    );
}

/// C `area3.c:2699`: a claim at an equal-or-lower cost survives - the
/// contender must be strictly cheaper to steal it.
#[test]
fn lampghost_does_not_steal_an_equally_cheap_claim() {
    let mut world = World::default();
    assert!(world.spawn_character(lampghost_npc(1), 10, 10));
    let lamp = lit_registered_lamp(9, 12, 10);
    world.map.tile_mut(12, 10).unwrap().item = 9;
    world.add_item(lamp);
    let cost = map_dist(10, 10, 12, 10);
    world
        .area3_lamp_claims
        .insert(ItemId(9), (CharacterId(99), cost));

    world.process_lampghost_actions(1);

    assert_eq!(
        world.area3_lamp_claims.get(&ItemId(9)),
        Some(&(CharacterId(99), cost))
    );
    assert_eq!(lampghost_state(&world, CharacterId(1)).claimed_lamp, None);
}

/// C `area3.c:2676-2679`: `if (!it[in].drdata[0]) { lamp[dat->ln].cn = 0;
/// dat->ln = 0; }` - a claimed lamp that went dark is dropped.
#[test]
fn lampghost_drops_its_claim_once_the_lamp_goes_dark() {
    let mut world = World::default();
    let mut lampghost = lampghost_npc(1);
    lampghost.driver_state = Some(CharacterDriverState::Lampghost(LampghostDriverData {
        claimed_lamp: Some(ItemId(9)),
        ..Default::default()
    }));
    assert!(world.spawn_character(lampghost, 10, 11));
    let mut lamp = item(9, ItemFlags::USE | ItemFlags::USED);
    lamp.driver = IDR_ONOFFLIGHT;
    lamp.driver_data = vec![0, 14, 0, 0, 0, 0, 1];
    lamp.x = 10;
    lamp.y = 10;
    world.map.tile_mut(10, 10).unwrap().item = 9;
    world.add_item(lamp);
    world
        .area3_lamp_claims
        .insert(ItemId(9), (CharacterId(1), 0));

    world.process_lampghost_actions(1);

    assert_eq!(lampghost_state(&world, CharacterId(1)).claimed_lamp, None);
    assert!(!world.area3_lamp_claims.contains_key(&ItemId(9)));
}

/// C `area3.c:2680-2682`: `if (lamp[dat->ln].cn != cn) { dat->ln = 0; }` -
/// a claim taken over by somebody else is dropped, not fought over.
#[test]
fn lampghost_drops_its_claim_once_somebody_else_took_it() {
    let mut world = World::default();
    let mut lampghost = lampghost_npc(1);
    lampghost.driver_state = Some(CharacterDriverState::Lampghost(LampghostDriverData {
        claimed_lamp: Some(ItemId(9)),
        ..Default::default()
    }));
    assert!(world.spawn_character(lampghost, 10, 11));
    let lamp = lit_registered_lamp(9, 10, 10);
    world.map.tile_mut(10, 10).unwrap().item = 9;
    world.add_item(lamp);
    // Somebody else now holds the registry claim on this exact lamp.
    world
        .area3_lamp_claims
        .insert(ItemId(9), (CharacterId(2), 0));

    world.process_lampghost_actions(1);

    // No other lamp exists to re-claim, so it goes back to idling.
    assert_eq!(lampghost_state(&world, CharacterId(1)).claimed_lamp, None);
    assert_eq!(
        world.area3_lamp_claims.get(&ItemId(9)),
        Some(&(CharacterId(2), 0))
    );
}

#[test]
fn lampghost_tracks_victim_from_char_sighting_and_attacks_when_adjacent() {
    // C `standard_message_driver(cn, msg, 1, 0)` with `aggressive=1`: any
    // valid enemy seen via `NT_CHAR` becomes the tracked victim.
    let mut world = World::default();
    let mut lampghost = lampghost_npc(1);
    lampghost.group = 0;
    assert!(world.spawn_character(lampghost, 10, 10));
    let mut enemy = character(2);
    enemy.group = 1;
    enemy.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(enemy, 11, 10));
    if let Some(lampghost) = world.characters.get_mut(&CharacterId(1)) {
        lampghost.driver_messages.push(CharacterDriverMessage {
            message_type: NT_CHAR,
            dat1: 2,
            dat2: 0,
            dat3: 0,
            text: None,
        });
    }

    world.process_lampghost_actions(1);

    assert_eq!(
        lampghost_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
    let lampghost = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lampghost.action, action::ATTACK1);
}

#[test]
fn lampghost_drains_incoming_messages_every_tick() {
    let mut world = World::default();
    assert!(world.spawn_character(lampghost_npc(1), 10, 10));
    if let Some(lampghost) = world.characters.get_mut(&CharacterId(1)) {
        lampghost.push_driver_message(NT_CHAR, 0, 0, 0);
    }

    world.process_lampghost_actions(1);

    assert!(world.characters[&CharacterId(1)].driver_messages.is_empty());
}

#[test]
fn lampghost_returns_to_post_when_no_lamp_and_no_victim() {
    let mut world = World::default();
    let mut lampghost = lampghost_npc(1);
    lampghost.rest_x = 10;
    lampghost.rest_y = 10;
    assert!(world.spawn_character(lampghost, 12, 10));

    let acted = world.process_lampghost_actions(1);

    assert_eq!(acted, 1);
    let lampghost = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lampghost.action, action::WALK);
}
