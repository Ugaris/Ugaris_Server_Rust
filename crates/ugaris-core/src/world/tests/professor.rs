use super::*;
use crate::character_driver::{parse_professor_driver_args, CDR_PROFESSOR, NT_CHAR};
use crate::world::npc::professor::ProfessorDriverData;

fn professor_npc(id: u32, nr: i32) -> Character {
    let mut professor = character(id);
    professor.name = "Teacher".into();
    professor.driver = CDR_PROFESSOR;
    professor.driver_state = Some(CharacterDriverState::Professor(ProfessorDriverData {
        dir: 0,
        nr,
        quest: 0,
        quest_option: 600,
        improve_cost: 50,
    }));
    professor.rest_x = professor.x;
    professor.rest_y = professor.y;
    professor
}

fn player_with_profession_skill(id: u32, name: &str, points: i16) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.values[1][CharacterValue::Profession as usize] = points;
    player
}

#[test]
fn parse_professor_driver_args_reads_all_fields() {
    let data = parse_professor_driver_args("nr=3;quest=0;option=600;cost=50;dir=2;");
    assert_eq!(data.nr, 3);
    assert_eq!(data.quest, 0);
    assert_eq!(data.quest_option, 600);
    assert_eq!(data.improve_cost, 50);
    assert_eq!(data.dir, 2);
}

#[test]
fn parse_professor_driver_args_ignores_unknown_keys() {
    let data = parse_professor_driver_args("foo=9;nr=1;");
    assert_eq!(data.nr, 1);
}

#[test]
fn professor_greets_a_visible_player_with_profession_skill_once() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 0), 10, 10));
    assert!(world.spawn_character(player_with_profession_skill(2, "Godmode", 20), 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_professor_actions(0);

    let texts = world.drain_pending_area_text_bytes();
    assert_eq!(texts.len(), 1);
    let greeting = String::from_utf8_lossy(&texts[0].message);
    assert!(greeting.contains("Hello Godmode!"));
    assert!(greeting.contains("professor at Aston University"));
    assert!(greeting.contains("Athlete"));

    // Second sighting: memory suppresses the repeat greeting.
    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_professor_actions(0);
    assert!(world.drain_pending_area_text_bytes().is_empty());
}

#[test]
fn professor_does_not_greet_a_player_without_profession_skill() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 0), 10, 10));
    assert!(world.spawn_character(player_with_profession_skill(2, "Godmode", 0), 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_professor_actions(0);

    assert!(world.drain_pending_area_text_bytes().is_empty());
}

#[test]
fn professor_answers_a_greeting_qa_row() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 0), 10, 10));
    assert!(world.spawn_character(player_with_profession_skill(2, "Godmode", 20), 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_professor_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn professor_describes_its_own_profession_regardless_of_which_word_was_said() {
    // C `case 3:` (`professor.c:410-460`) always describes `dat->nr`, not
    // the profession word the player actually spoke - a real quirk kept
    // verbatim. Here the professor teaches Miner (nr=2) but the player
    // asks about "herbalist".
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 2), 10, 10));
    assert!(world.spawn_character(player_with_profession_skill(2, "Godmode", 20), 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_text_message(CharacterId(2), "herbalist");
    }
    world.process_professor_actions(0);

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("miner")));
}

#[test]
fn professor_teach_explains_cost_using_its_own_profession() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 0), 10, 10));
    assert!(world.spawn_character(player_with_profession_skill(2, "Godmode", 20), 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_text_message(CharacterId(2), "teach");
    }
    world.process_professor_actions(0);

    let texts = world.drain_pending_area_texts();
    let message = texts
        .iter()
        .find(|text| text.message.contains("Athlete"))
        .expect("teach explanation");
    // `quest_option=600` gold, `prof[P_ATHLETE].base=6` profession points.
    assert!(message.message.contains("600 gold coins and 6"));
    // improve fee: `improve_cost(50) * step(3) = 150` gold, `step=3` points.
    assert!(message.message.contains("150 gold coins and 3"));
}

#[test]
fn professor_learn_succeeds_with_enough_gold_and_profession_points() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 0), 10, 10));
    let mut player = player_with_profession_skill(2, "Godmode", 20);
    player.gold = 60_000; // 600 gold * 100.
    assert!(world.spawn_character(player, 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_text_message(CharacterId(2), "learn");
    }
    world.process_professor_actions(0);

    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.professions[0], 6); // prof[P_ATHLETE].base
    assert_eq!(player.gold, 0);
    assert!(player.flags.contains(CharacterFlags::PROF));
    assert!(player.flags.contains(CharacterFlags::ITEMS));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("learnt the art of Athlete")));

    // C `if (ch[co].flags & CF_PLAYER) achievement_check_profession(...)`.
    let checks = world.drain_pending_professor_achievement_checks();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].player_id, CharacterId(2));
    assert_eq!(checks[0].profession, 0);
    assert_eq!(checks[0].level, 6);
}

#[test]
fn professor_learn_fails_without_enough_gold_and_does_not_change_professions() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 0), 10, 10));
    let mut player = player_with_profession_skill(2, "Godmode", 20);
    player.gold = 100; // Far short of 600 gold.
    assert!(world.spawn_character(player, 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_text_message(CharacterId(2), "learn");
    }
    world.process_professor_actions(0);

    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.professions[0], 0);
    assert_eq!(player.gold, 100);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("cannot afford my fee of 600G")));
    assert!(world
        .drain_pending_professor_achievement_checks()
        .is_empty());
}

#[test]
fn professor_learn_fails_without_enough_free_profession_points() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 0), 10, 10));
    let mut player = player_with_profession_skill(2, "Godmode", 3); // needs 6.
    player.gold = 60_000;
    assert!(world.spawn_character(player, 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_text_message(CharacterId(2), "learn");
    }
    world.process_professor_actions(0);

    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.professions[0], 0);
    // Gold is only deducted after a successful `learn_prof`.
    assert_eq!(player.gold, 60_000);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("not the required profession points")));
}

#[test]
fn professor_improve_raises_an_already_learned_profession() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 0), 10, 10));
    let mut player = player_with_profession_skill(2, "Godmode", 20);
    player.professions[0] = 6; // Already learned Athlete at base level.
    player.gold = 15_000; // improve_cost(50) * step(3) * 100 = 15000.
    assert!(world.spawn_character(player, 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_text_message(CharacterId(2), "improve");
    }
    world.process_professor_actions(0);

    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.professions[0], 9); // 6 + step(3).
    assert_eq!(player.gold, 0);

    let checks = world.drain_pending_professor_achievement_checks();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].level, 9);
}

#[test]
fn professor_improve_fails_when_profession_not_learned_yet() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(professor_npc(1, 0), 10, 10));
    let mut player = player_with_profession_skill(2, "Godmode", 20);
    player.gold = 15_000;
    assert!(world.spawn_character(player, 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_text_message(CharacterId(2), "improve");
    }
    world.process_professor_actions(0);

    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.professions[0], 0);
    assert_eq!(player.gold, 15_000);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("knowest not the ways of the Athlete")));
}

#[test]
fn professor_learn_rejects_light_and_dark_mutual_exclusion() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    // nr=6 -> P_DARK.
    assert!(world.spawn_character(professor_npc(1, 6), 10, 10));
    let mut player = player_with_profession_skill(2, "Godmode", 20);
    player.professions[5] = 6; // Already has P_LIGHT (index 5).
    player.gold = 60_000;
    // C's `!(ch[co].flags & CF_PAID) && count_prof(co) > 0` guard would
    // otherwise fire first (`professor.c:264-267`) - grant `CF_PAID` so
    // this test reaches the Light/Dark mutual-exclusion guard instead.
    player.flags.insert(CharacterFlags::PAID);
    assert!(world.spawn_character(player, 11, 10));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_text_message(CharacterId(2), "learn");
    }
    world.process_professor_actions(0);

    let player = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(player.professions[6], 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Master of Light already")));
}

#[test]
fn professor_destroys_a_given_item() {
    let mut world = World::default();
    let mut professor = professor_npc(1, 0);
    professor.cursor_item = Some(ItemId(900));
    assert!(world.spawn_character(professor, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(professor) = world.characters.get_mut(&CharacterId(1)) {
        professor.push_driver_message(crate::character_driver::NT_GIVE, 2, 0, 0);
    }

    world.process_professor_actions(0);

    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(!world.items.contains_key(&ItemId(900)));
}
