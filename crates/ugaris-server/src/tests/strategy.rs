use super::*;
use ugaris_core::character_driver::CDR_STRATEGY;
use ugaris_core::direction::Direction;
use ugaris_core::world::npc::area23_24::StrategyWorkerDriverData;
use ugaris_core::world::{AiEguardSpawnPlan, AiWorkerSpawnPlan, StrategyWorkerOrder};

fn strategy_npc_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                strategy_npc:
                  name="Worker"
                  description="A recruited worker."
                  sprite=300
                  V_HP=10
                  V_ENDURANCE=8
                  V_MANA=6
                  V_WARCRY=2
                  V_SPEED=3
                  V_WIS=4
                  V_INT=4
                  V_AGI=4
                  V_STR=4
                ;
            "#,
        )
        .unwrap();
    loader
}

// C `ai_main`'s "create new workers" character-creation tail
// (`World::ai_plan_worker_spawn` + `spawn_ai_worker`, `spawner_sub`'s own
// `create_char`/`item_drop_char` half, `strategy.c:1259-1279`).
#[test]
fn spawn_ai_worker_builds_character_near_spawner_item() {
    let mut world = World::default();
    let mut loader = strategy_npc_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(500);

    let mut spawner = test_item_with_driver(ItemId(90), 0);
    spawner.x = 20;
    spawner.y = 20;
    world.add_item(spawner);

    let plan = AiWorkerSpawnPlan {
        spawner_id: ItemId(90),
        group: 0xf001,
        owner_name: "Nasty Nick".to_string(),
        warcry: 5,
        endurance: 6,
        speed: 7,
        trainspeed: 2,
        max_level: 30,
        npc_color: 1,
    };

    let (character_id, x, y) =
        tick_item_use_strategy::spawn_ai_worker(&mut world, &mut loader, &mut runtime, plan)
            .expect("worker should spawn near the free spawner tile");
    assert_eq!(character_id, CharacterId(500));
    assert!((i32::from(x) - 20).abs() <= 1);
    assert!((i32::from(y) - 20).abs() <= 1);

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.driver, CDR_STRATEGY);
    assert_eq!(character.group, 0xf001);
    assert_eq!(character.sprite, 353 + 1);
    assert_eq!(character.dir, Direction::RightDown as u8);
    assert_eq!(character.values[1][CharacterValue::Warcry as usize], 2 + 5);
    assert_eq!(
        character.values[1][CharacterValue::Endurance as usize],
        8 + 6
    );
    assert_eq!(character.values[1][CharacterValue::Speed as usize], 3 + 7);
    assert_eq!(
        character.hp,
        i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE
    );
    assert_eq!(
        character.endurance,
        i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE
    );
    match character.driver_state.as_ref() {
        Some(CharacterDriverState::StrategyWorker(data)) => {
            let data: &StrategyWorkerDriverData = data;
            assert_eq!(data.owner_name, "Nasty Nick");
            assert_eq!(data.trainspeed, 2);
            assert_eq!(data.max_level, 30);
            assert_eq!(data.order, StrategyWorkerOrder::None);
        }
        other => panic!("expected StrategyWorker driver state, got {other:?}"),
    }
}

// C `create_eguard`'s own `create_char`/`drop_char` tail
// (`World::ai_plan_eguard_spawn` + `spawn_ai_eguard`, `strategy.c:2991-3023`).
#[test]
fn spawn_ai_eguard_drops_at_place_and_stamps_fixed_level() {
    let mut world = World::default();
    let mut loader = strategy_npc_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(600);

    let plan = AiEguardSpawnPlan {
        x: 40,
        y: 42,
        group: 0xf002,
        level: 25,
        owner_name: "Grumpy Golem".to_string(),
        warcry: 1,
        endurance: 2,
        speed: 3,
        npc_color: 2,
    };

    let character_id =
        tick_item_use_strategy::spawn_ai_eguard(&mut world, &mut loader, &mut runtime, plan)
            .expect("eguard should drop at the requested tile");
    assert_eq!(character_id, CharacterId(600));

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (40, 42));
    assert_eq!(character.driver, CDR_STRATEGY);
    assert_eq!(character.group, 0xf002);
    assert_eq!(character.level, 25);
    assert_eq!(character.values[1][CharacterValue::Wisdom as usize], 25);
    assert_eq!(
        character.values[1][CharacterValue::Intelligence as usize],
        25
    );
    assert_eq!(character.values[1][CharacterValue::Agility as usize], 25);
    assert_eq!(character.values[1][CharacterValue::Strength as usize], 25);
    assert_eq!(character.values[1][CharacterValue::Warcry as usize], 2 + 1);
    assert_eq!(
        character.values[1][CharacterValue::Endurance as usize],
        8 + 2
    );
    assert_eq!(character.values[1][CharacterValue::Speed as usize], 3 + 3);
    assert_eq!(character.sprite, 353 + 2);
    assert_eq!(character.dir, Direction::RightDown as u8);
    match character.driver_state.as_ref() {
        Some(CharacterDriverState::StrategyWorker(data)) => {
            let data: &StrategyWorkerDriverData = data;
            assert_eq!(data.owner_name, "Grumpy Golem");
            assert_eq!(
                data.order,
                StrategyWorkerOrder::EternalGuard { x: 40, y: 42 }
            );
            assert_eq!(data.trainspeed, 0);
            assert_eq!(data.max_level, 0);
        }
        other => panic!("expected StrategyWorker driver state, got {other:?}"),
    }
}
