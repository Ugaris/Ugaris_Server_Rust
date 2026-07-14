use super::*;
use crate::character_driver::{
    FightDriverData, CDR_GLADIATOR, NTID_GLADIATOR, NT_CHAR, NT_DEAD, NT_GOTHIT, NT_NPC,
};
use crate::world::npc::area37::gladiator::GladiatorDriverData;

/// Well under [`SELF_DESTRUCT_TICKS`] (`TICKS_PER_SECOND * 60 * 3`, 3
/// minutes) so ordinary message-processing tests don't accidentally
/// trigger the self-destruct branch.
const BASELINE_TICK: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 60 * 3` (`arkhata.c:1136`): the 3-minute self-destruct
/// timeout.
const SELF_DESTRUCT_TICKS: u64 = TICKS_PER_SECOND * 60 * 3;

fn gladiator_npc(id: u32) -> Character {
    let mut gladiator = character(id);
    gladiator.name = "John".into();
    gladiator.driver = CDR_GLADIATOR;
    // C `zones/37/Fighting_School.chr`'s `Gladiator_<n>` templates all
    // carry `group=1`.
    gladiator.group = 1;
    gladiator.rest_x = 14;
    gladiator.rest_y = 244;
    gladiator.driver_state = Some(CharacterDriverState::Gladiator(GladiatorDriverData {
        last_talk: 0,
    }));
    // C never calls `fight_driver_set_dist` for `CDR_GLADIATOR` - see
    // `ugaris-server::area37::spawn_gladiator_student`'s own comment for
    // why this still needs seeding in Rust.
    gladiator.fight_driver = Some(FightDriverData::default());
    gladiator
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

#[test]
fn self_destructs_after_three_minutes_of_silence_and_teleports_arena_players() {
    let mut world = World::default();
    assert!(world.spawn_character(gladiator_npc(1), 14, 244));
    assert!(world.spawn_character(player(2, "Godmode"), 15, 245));

    world.tick = Tick(SELF_DESTRUCT_TICKS + 1);
    world.process_gladiator_actions(1);

    assert!(!world.characters.contains_key(&CharacterId(1)));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((godmode.x, godmode.y), (15, 235));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("That's all folks!")));
}

#[test]
fn does_not_self_destruct_before_the_timeout() {
    let mut world = World::default();
    assert!(world.spawn_character(gladiator_npc(1), 14, 244));

    world.tick = Tick(SELF_DESTRUCT_TICKS - 1);
    world.process_gladiator_actions(1);

    assert!(world.characters.contains_key(&CharacterId(1)));
}

#[test]
fn nt_char_visible_player_is_added_as_enemy() {
    let mut world = World::default();
    world.map.tile_mut(15, 244).unwrap().light = 255;
    assert!(world.spawn_character(gladiator_npc(1), 14, 244));
    assert!(world.spawn_character(player(2, "Godmode"), 15, 244));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gladiator) = world.characters.get_mut(&CharacterId(1)) {
        gladiator.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_gladiator_actions(1);

    let gladiator = world.characters.get(&CharacterId(1)).unwrap();
    let enemies: Vec<_> = gladiator
        .fight_driver
        .as_ref()
        .map(|data| data.enemies.iter().map(|e| e.target_id).collect())
        .unwrap_or_default();
    assert!(enemies.contains(&CharacterId(2)));
}

#[test]
fn nt_gothit_from_a_different_group_adds_the_attacker_as_enemy() {
    let mut world = World::default();
    assert!(world.spawn_character(gladiator_npc(1), 14, 244));
    let mut attacker = player(2, "Godmode");
    attacker.group = 9;
    assert!(world.spawn_character(attacker, 15, 244));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gladiator) = world.characters.get_mut(&CharacterId(1)) {
        gladiator.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }
    world.process_gladiator_actions(1);

    let gladiator = world.characters.get(&CharacterId(1)).unwrap();
    let enemies: Vec<_> = gladiator
        .fight_driver
        .as_ref()
        .map(|data| data.enemies.iter().map(|e| e.target_id).collect())
        .unwrap_or_default();
    assert!(enemies.contains(&CharacterId(2)));
    assert!(gladiator.fight_driver.as_ref().unwrap().last_hit == BASELINE_TICK as i32);
}

#[test]
fn nt_gothit_from_the_same_group_is_ignored() {
    let mut world = World::default();
    assert!(world.spawn_character(gladiator_npc(1), 14, 244));
    let mut attacker = player(2, "Godmode");
    attacker.group = 1;
    assert!(world.spawn_character(attacker, 15, 244));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gladiator) = world.characters.get_mut(&CharacterId(1)) {
        gladiator.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }
    world.process_gladiator_actions(1);

    let gladiator = world.characters.get(&CharacterId(1)).unwrap();
    let enemies: Vec<_> = gladiator
        .fight_driver
        .as_ref()
        .map(|data| data.enemies.iter().map(|e| e.target_id).collect())
        .unwrap_or_default();
    assert!(!enemies.contains(&CharacterId(2)));
}

#[test]
fn killing_a_player_self_destructs() {
    let mut world = World::default();
    assert!(world.spawn_character(gladiator_npc(1), 14, 244));
    assert!(world.spawn_character(player(2, "Godmode"), 15, 244));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gladiator) = world.characters.get_mut(&CharacterId(1)) {
        // C `NT_DEAD`: `dat1`=victim, `dat2`=killer.
        gladiator.push_driver_message(NT_DEAD, 2, 1, 0);
    }
    world.process_gladiator_actions(1);

    assert!(!world.characters.contains_key(&CharacterId(1)));
}

#[test]
fn witnessing_an_unrelated_death_is_a_no_op() {
    let mut world = World::default();
    assert!(world.spawn_character(gladiator_npc(1), 14, 244));
    assert!(world.spawn_character(player(2, "Godmode"), 15, 244));

    world.tick = Tick(BASELINE_TICK);
    if let Some(gladiator) = world.characters.get_mut(&CharacterId(1)) {
        // Killed by someone else (dat2 != gladiator's own id).
        gladiator.push_driver_message(NT_DEAD, 2, 99, 0);
    }
    world.process_gladiator_actions(1);

    assert!(world.characters.contains_key(&CharacterId(1)));
}

#[test]
fn apply_gladiator_death_notifies_area_for_a_player_killer() {
    let mut world = World::default();
    assert!(world.spawn_character(gladiator_npc(1), 15, 232));
    let mut fiona = character(3);
    fiona.name = "Queen Fiona".into();
    assert!(world.spawn_character(fiona, 15, 232));
    assert!(world.spawn_character(player(2, "Godmode"), 15, 232));

    world.apply_gladiator_death(CharacterId(1), CharacterId(2));

    let fiona = world.characters.get(&CharacterId(3)).unwrap();
    assert!(fiona
        .driver_messages
        .iter()
        .any(|msg| msg.message_type == NT_NPC && msg.dat1 == NTID_GLADIATOR && msg.dat3 == 2));
}

#[test]
fn apply_gladiator_death_ignores_a_non_player_killer() {
    let mut world = World::default();
    assert!(world.spawn_character(gladiator_npc(1), 15, 232));
    let monster = character(2);
    world.add_character(monster);

    world.apply_gladiator_death(CharacterId(1), CharacterId(2));

    let gladiator = world.characters.get(&CharacterId(1)).unwrap();
    assert!(gladiator.driver_messages.is_empty());
}
