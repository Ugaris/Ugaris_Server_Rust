use super::*;
use crate::character_driver::{CDR_LQNPC, NT_CHAR, NT_GIVE, NT_GOTHIT};
use crate::world::npc::area20::{LqNpcDriverData, LqNpcOutcomeEvent};
use crate::world::{make_lq_item_template_id, LqItemSpec};

fn lqnpc(id: u32, data: LqNpcDriverData) -> Character {
    let mut npc = character(id);
    npc.name = "Quest Guard".into();
    npc.driver = CDR_LQNPC;
    npc.driver_state = Some(CharacterDriverState::LqNpc(data));
    // C `spawn_npc`: `ch[cn].tmpx = lq_npc[n].x; ch[cn].tmpy =
    // lq_npc[n].y;` (`lq.c:1774-1775`) - modeled as `rest_x`/`rest_y`
    // (see `LqNpcDriverData`'s module doc comment). Test spawn positions
    // below always match, so set both here rather than per test.
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

#[test]
fn lqnpc_greets_a_nearby_visible_player_once() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(
        lqnpc(
            1,
            LqNpcDriverData {
                slot: 1,
                greeting: "Halt, who goes there?".to_string(),
                ..Default::default()
            },
        ),
        10,
        10,
    ));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    if let Some(npc) = world.characters.get_mut(&CharacterId(1)) {
        npc.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_lqnpc_actions(20);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Halt, who goes there?")));

    // Second sighting of the same player: already greeted (mem slot 7),
    // no repeat.
    if let Some(npc) = world.characters.get_mut(&CharacterId(1)) {
        npc.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.process_lqnpc_actions(20);
    let texts = world.drain_pending_area_texts();
    assert!(!texts
        .iter()
        .any(|text| text.message.contains("Halt, who goes there?")));
}

#[test]
fn lqnpc_gothit_sets_hurt_mark_and_adds_self_defense_enemy() {
    let mut world = World::default();
    assert!(world.spawn_character(
        lqnpc(
            1,
            LqNpcDriverData {
                slot: 1,
                hurt_mark_id: 3,
                ..Default::default()
            },
        ),
        10,
        10,
    ));
    let mut attacker = player(2, "Godmode");
    attacker.group = 9;
    assert!(world.spawn_character(attacker, 11, 10));

    if let Some(npc) = world.characters.get_mut(&CharacterId(1)) {
        npc.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }
    let events = world.process_lqnpc_actions(20);
    assert_eq!(
        events,
        vec![LqNpcOutcomeEvent::SetPlayerMark {
            player_id: CharacterId(2),
            mark_id: 3,
        }]
    );
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert!(npc
        .fight_driver
        .as_ref()
        .unwrap()
        .enemies
        .iter()
        .any(|enemy| enemy.target_id == CharacterId(2)));
}

#[test]
fn lqnpc_gothit_ignores_a_same_group_attacker_and_out_of_range_mark_id() {
    let mut world = World::default();
    assert!(world.spawn_character(
        lqnpc(
            1,
            LqNpcDriverData {
                slot: 1,
                hurt_mark_id: 0,
                ..Default::default()
            },
        ),
        10,
        10,
    ));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    if let Some(npc) = world.characters.get_mut(&CharacterId(1)) {
        npc.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }
    let events = world.process_lqnpc_actions(20);
    assert!(events.is_empty());
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert!(npc.fight_driver.as_ref().unwrap().enemies.is_empty());
}

#[test]
fn lqnpc_text_matches_trigger_reply_and_starts_following_on_followme() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(
        lqnpc(
            1,
            LqNpcDriverData {
                slot: 1,
                trigger: [
                    "quest".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ],
                reply: [
                    "Aye, I have a quest for thee.".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ],
                ..Default::default()
            },
        ),
        10,
        10,
    ));
    let mut master = player(2, "Godmode");
    master.flags |= CharacterFlags::LQMASTER;
    assert!(world.spawn_character(master, 11, 10));

    if let Some(npc) = world.characters.get_mut(&CharacterId(1)) {
        npc.push_driver_text_message(CharacterId(2), "got a quest for me?");
    }
    world.process_lqnpc_actions(20);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Aye, I have a quest for thee.")));

    if let Some(npc) = world.characters.get_mut(&CharacterId(1)) {
        npc.push_driver_text_message(CharacterId(2), "followme please");
    }
    world.process_lqnpc_actions(20);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(
        npc.driver_state,
        Some(CharacterDriverState::LqNpc(LqNpcDriverData {
            slot: 1,
            trigger: [
                "quest".to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            reply: [
                "Aye, I have a quest for thee.".to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            follow: Some(CharacterId(2)),
            ..Default::default()
        }))
    );

    if let Some(npc) = world.characters.get_mut(&CharacterId(1)) {
        npc.push_driver_text_message(CharacterId(2), "stopfollow");
    }
    world.process_lqnpc_actions(20);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    match npc.driver_state.as_ref() {
        Some(CharacterDriverState::LqNpc(data)) => assert_eq!(data.follow, None),
        other => panic!("expected LqNpc driver state, got {other:?}"),
    }
}

#[test]
fn lqnpc_follows_its_lqmaster_target_when_not_adjacent() {
    let mut world = World::default();
    assert!(world.spawn_character(
        lqnpc(
            1,
            LqNpcDriverData {
                slot: 1,
                follow: Some(CharacterId(2)),
                ..Default::default()
            },
        ),
        10,
        10,
    ));
    let mut master = player(2, "Godmode");
    master.flags |= CharacterFlags::LQMASTER;
    assert!(world.spawn_character(master, 15, 10));

    world.process_lqnpc_actions(20);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    // C `move_driver` (`drvlib.c:76-85`) started a walk toward the
    // follow target rather than leaving the NPC idle - movement itself
    // completes over several ticks (`action::WALK`/`tox`/`toy`), so
    // `x`/`y` don't change within this single tick.
    assert_eq!(npc.action, crate::legacy::action::WALK);
    assert_ne!((npc.tox, npc.toy), (0, 0));
}

#[test]
fn lqnpc_mirrors_its_possessors_movement_when_usurped() {
    let mut world = World::default();
    let possessor_id = CharacterId(2);
    assert!(world.spawn_character(
        lqnpc(
            1,
            LqNpcDriverData {
                slot: 1,
                usurp: Some(possessor_id),
                udx: 0,
                udy: -2,
                ..Default::default()
            },
        ),
        10,
        10,
    ));
    let mut possessor = player(2, "Godmode");
    possessor.flags |= CharacterFlags::GOD;
    possessor.lq_usurp = Some(CharacterId(1));
    // `udx`/`udy` were captured when the possessor stood at (10,8); it
    // has since moved 5 tiles east, so the NPC must mirror that offset.
    assert!(world.spawn_character(possessor, 15, 8));

    world.process_lqnpc_actions(20);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, crate::legacy::action::WALK);
    assert_ne!((npc.tox, npc.toy), (0, 0));
}

#[test]
fn lqnpc_domirror_turns_to_match_possessors_facing_when_already_aligned() {
    let mut world = World::default();
    let possessor_id = CharacterId(2);
    let mut npc = lqnpc(
        1,
        LqNpcDriverData {
            slot: 1,
            usurp: Some(possessor_id),
            udx: 0,
            udy: 0,
            ..Default::default()
        },
    );
    npc.dir = 1;
    assert!(world.spawn_character(npc, 10, 10));
    let mut possessor = player(2, "Godmode");
    possessor.flags |= CharacterFlags::GOD;
    possessor.lq_usurp = Some(CharacterId(1));
    possessor.dir = 5;
    // Same tile offset as `udx`/`udy` (both zero) - no move is needed,
    // only the facing sync (`turn(cn, ch[co].dir)`, `lq.c:2862-2864`).
    assert!(world.spawn_character(possessor, 10, 10));

    world.process_lqnpc_actions(20);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.dir, 5);
    assert_eq!(npc.action, 0);
}

#[test]
fn lqnpc_does_not_mirror_movement_without_mutual_pairing() {
    let mut world = World::default();
    let possessor_id = CharacterId(2);
    assert!(world.spawn_character(
        lqnpc(
            1,
            LqNpcDriverData {
                slot: 1,
                usurp: Some(possessor_id),
                udx: 0,
                udy: 0,
                ..Default::default()
            },
        ),
        10,
        10,
    ));
    // The possessor's own `lq_usurp` was never set back to this NPC (a
    // stale/racy state) - C's own `pdat->usurp == cn` check
    // (`lq.c:2856`) rejects mirroring here too.
    let mut possessor = player(2, "Godmode");
    possessor.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(possessor, 15, 15));

    world.process_lqnpc_actions(20);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    // No mirroring toward (15,15) happened; the NPC is already at its
    // own rest position (10,10, matching `lqnpc()`'s helper), so the
    // fallback idle-return-home path is also a no-op.
    assert_eq!(npc.action, 0);
    assert_eq!((npc.x, npc.y), (10, 10));
}

#[test]
fn lqnpc_give_matching_key_queues_reward_and_destroys_cursor_item() {
    let mut world = World::default();
    assert!(world.spawn_character(
        lqnpc(
            1,
            LqNpcDriverData {
                slot: 1,
                want_key_id: 7,
                reward_item: LqItemSpec {
                    base: "torch".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        ),
        10,
        10,
    ));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    let mut quest_item = item(50, ItemFlags::empty());
    quest_item.template_id = make_lq_item_template_id(7);
    world.add_item(quest_item);
    if let Some(npc) = world.characters.get_mut(&CharacterId(1)) {
        npc.cursor_item = Some(ItemId(50));
        npc.push_driver_message(NT_GIVE, 2, 0, 0);
    }

    let events = world.process_lqnpc_actions(20);
    assert_eq!(
        events,
        vec![LqNpcOutcomeEvent::GiveRewardItem {
            receiver_id: CharacterId(2),
            item: LqItemSpec {
                base: "torch".to_string(),
                ..Default::default()
            },
        }]
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thanks, that's what I wanted.")));
    assert!(!world.items.contains_key(&ItemId(50)));
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert!(npc.cursor_item.is_none());
}

#[test]
fn lqnpc_give_wrong_key_destroys_item_without_a_reward_event() {
    let mut world = World::default();
    assert!(world.spawn_character(
        lqnpc(
            1,
            LqNpcDriverData {
                slot: 1,
                want_key_id: 7,
                reward_item: LqItemSpec {
                    base: "torch".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        ),
        10,
        10,
    ));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    let mut junk = item(51, ItemFlags::empty());
    junk.template_id = make_lq_item_template_id(999);
    world.add_item(junk);
    if let Some(npc) = world.characters.get_mut(&CharacterId(1)) {
        npc.cursor_item = Some(ItemId(51));
        npc.push_driver_message(NT_GIVE, 2, 0, 0);
    }

    let events = world.process_lqnpc_actions(20);
    assert!(events.is_empty());
    assert!(!world.items.contains_key(&ItemId(51)));
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert!(npc.cursor_item.is_none());
}
