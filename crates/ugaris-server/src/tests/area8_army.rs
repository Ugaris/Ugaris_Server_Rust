use super::*;
use ugaris_core::{
    character_driver::{CharacterDriverState, CDR_FDEMON_ARMY},
    entity::CharacterFlags,
    world::npc::area8::fdemon_army::MAXSOLDIER,
};

fn connect_player(runtime: &mut ServerRuntime, session_id: u64, character_id: CharacterId) {
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(session_id, commands, 0);
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.character_id = Some(character_id);
    }
}

// Digit-for-digit copies of `ugaris_data/zones/8/fire.chr`'s `army1s`/
// `army2s` templates - only the fields `spawn_army_soldier`/
// `scale_soldier_values`/`soldier_equipment_items` actually read matter
// for these tests (the marker `V_*=n` values double as C's `update_soldier`
// tier markers - see `fdemon_army.rs`'s module doc comment).
const ARMY_CHR: &str = r#"
    army1s:
      name="Josh"
      description="A soldier-warrior."
      sprite=2
      flag=CF_PLAYERLIKE
      flag=CF_ALIVE
      V_HP=2
      V_ENDURANCE=1
      V_MANA=0
      V_SWORD=3
      V_ARMORSKILL=3
      V_DAGGER=0
      driver=44
      group=2
    ;
    army2s:
      name="Bert"
      description="A soldier-mage."
      sprite=3
      flag=CF_PLAYERLIKE
      flag=CF_ALIVE
      V_HP=2
      V_ENDURANCE=1
      V_MANA=2
      V_DAGGER=3
      V_SWORD=0
      V_ARMORSKILL=0
      driver=44
      group=2
    ;
"#;

// `V_ARMORSKILL=3`/`V_SWORD=3` markers with `rank=1` (`base=47`) both scale
// to the "marker 3 -> base" branch, i.e. skill 47 -> tier `47/10+1=5`;
// `V_DAGGER=3` on `army2s` scales the same way.
const ARMY_ITM: &str = r#"
    sleeves5q1: name="Sleeves" ;
    armor5q1: name="Armor" ;
    helmet5q1: name="Helmet" ;
    leggings5q1: name="Leggings" ;
    sword5q1: name="Sword" ;
    dagger5q1: name="Dagger" ;
"#;

fn army_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader.load_character_templates_str(ARMY_CHR).unwrap();
    loader.load_item_templates_str(ARMY_ITM).unwrap();
    loader
}

// C `take_soldiers(cn)` (`fdemon.c:451-590`): a non-warrior male player at
// army rank 1 recruits exactly slot 0 as a warrior (`type=1`), fully
// equipped and positioned at the player's own tile.
#[tokio::test]
async fn take_soldiers_recruits_and_spawns_slot_zero() {
    let area_id: u16 = 8;
    let player_id = CharacterId(1);
    let mut world = World::default();
    world.area_id = area_id;

    let mut player_character = login_character(player_id, &login_block("Hero"), area_id, 10, 10);
    player_character.military_points = 10; // army_rank_for_points(10) = 2 > 0
    player_character.flags.remove(CharacterFlags::WARRIOR);
    player_character.flags.insert(CharacterFlags::MALE);
    player_character.group = 7;
    assert!(world.spawn_character(player_character, 10, 10));

    let mut loader = army_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(50);
    connect_player(&mut runtime, 1, player_id);

    crate::area8_army::take_soldiers(&mut world, &mut loader, &mut runtime, player_id);

    let soldier = world.characters.get(&CharacterId(50)).unwrap();
    assert_eq!(soldier.driver, CDR_FDEMON_ARMY);
    assert_eq!(soldier.group, 7);
    // `drop_char` places the soldier on the nearest free tile - the
    // player already occupies (10, 10), so it lands adjacent.
    assert!(soldier.x.abs_diff(10) <= 1 && soldier.y.abs_diff(10) <= 1);
    assert!(soldier.hp > 0);
    // Non-warrior player -> slot 0 is a warrior (C `if (ch[cn].flags &
    // CF_WARRIOR) type=2; else type=1;`), so it wears the five-piece
    // armor kit plus a sword, not a dagger.
    assert!(soldier.inventory[6].is_some()); // WN_RHAND: sword
    assert!(soldier.inventory[4].is_some()); // WN_BODY: armor
    let Some(CharacterDriverState::FdemonArmy(dat)) = soldier.driver_state else {
        panic!("expected FdemonArmy driver state");
    };
    assert_eq!(dat.leader_cn, player_id);
    assert_eq!(dat.platoon[0], CharacterId(50));
    assert_eq!(dat.platoon[MAXSOLDIER], player_id);

    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(player.farmy_soldier_cn(0), 50);
    assert_eq!(player.farmy_soldier_serial(0), 50);
    assert_eq!(player.farmy_soldier_type(0), 1); // SOLDIER_TYPE_WARRIOR
}

// C `drop_soldiers(cn)` (`fdemon.c:592-625`): destroys the live soldier and
// folds its unspent exp back into the PPD record, but leaves `type`/
// `profile`/`rank` alone (only `serial` resets) so a later `take_soldiers`
// rebuilds the same slot.
#[tokio::test]
async fn drop_soldiers_destroys_character_and_keeps_recruitment_state() {
    let area_id: u16 = 8;
    let player_id = CharacterId(1);
    let mut world = World::default();
    world.area_id = area_id;

    let mut player_character = login_character(player_id, &login_block("Hero"), area_id, 10, 10);
    player_character.military_points = 10;
    player_character.flags.remove(CharacterFlags::WARRIOR);
    player_character.flags.insert(CharacterFlags::MALE);
    assert!(world.spawn_character(player_character, 10, 10));

    let mut loader = army_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(50);
    connect_player(&mut runtime, 1, player_id);

    crate::area8_army::take_soldiers(&mut world, &mut loader, &mut runtime, player_id);
    assert!(world.characters.contains_key(&CharacterId(50)));

    crate::area8_army::drop_soldiers(&mut world, &mut runtime, player_id);
    assert!(!world.characters.contains_key(&CharacterId(50)));

    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(player.farmy_soldier_serial(0), 0);
    // `type`/`profile`/`rank` survive the drop.
    assert_eq!(player.farmy_soldier_type(0), 1);

    // A subsequent `take_soldiers` rebuilds the same slot at the next
    // allocated id, without re-rolling `type`.
    runtime.set_next_character_id(60);
    crate::area8_army::take_soldiers(&mut world, &mut loader, &mut runtime, player_id);
    let rebuilt = world.characters.get(&CharacterId(60)).unwrap();
    assert_eq!(rebuilt.driver, CDR_FDEMON_ARMY);
}

// C `army_rank <= 0` (`ppd->soldier[0].rank`/`type` gate,
// `fdemon.c:468-480`): rank-0 players recruit nobody.
#[tokio::test]
async fn take_soldiers_recruits_nobody_below_rank_one() {
    let area_id: u16 = 8;
    let player_id = CharacterId(1);
    let mut world = World::default();
    world.area_id = area_id;

    let mut player_character = login_character(player_id, &login_block("Hero"), area_id, 10, 10);
    player_character.military_points = 0;
    assert!(world.spawn_character(player_character, 10, 10));

    let mut loader = army_loader();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(50);
    connect_player(&mut runtime, 1, player_id);

    crate::area8_army::take_soldiers(&mut world, &mut loader, &mut runtime, player_id);

    assert!(world.characters.get(&CharacterId(50)).is_none());
    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(player.farmy_soldier_type(0), 0);
}
