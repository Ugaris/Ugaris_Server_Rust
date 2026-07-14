use super::*;
use ugaris_core::{
    character_driver::{CharacterDriverState, CDR_FDEMON_ARMY},
    entity::CharacterFlags,
    world::npc::area8::fdemon_army::MAXSOLDIER,
};

fn set_soldier_emote(
    world: &mut World,
    soldier_id: CharacterId,
    mutate: impl FnOnce(&mut ugaris_core::world::npc::area8::fdemon_army_emote::SoldierEmote),
) {
    let Some(CharacterDriverState::FdemonArmy(dat)) = world
        .characters
        .get_mut(&soldier_id)
        .and_then(|character| character.driver_state.as_mut())
    else {
        panic!("expected FdemonArmy driver state");
    };
    mutate(&mut dat.emote);
}

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
// `V_DAGGER=3` on `army2s` scales the same way. Tier 6 (`rank=2`,
// `base=51`) is also included for `reequip_soldier_for_promotion` tests.
const ARMY_ITM: &str = r#"
    sleeves5q1: name="Sleeves" ;
    armor5q1: name="Armor" ;
    helmet5q1: name="Helmet" ;
    leggings5q1: name="Leggings" ;
    sword5q1: name="Sword" ;
    dagger5q1: name="Dagger" ;
    sleeves6q1: name="Sleeves" ;
    armor6q1: name="Armor" ;
    helmet6q1: name="Helmet" ;
    leggings6q1: name="Leggings" ;
    sword6q1: name="Sword" ;
    dagger6q1: name="Dagger" ;
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

    assert!(!world.characters.contains_key(&CharacterId(50)));
    let player = runtime.player_for_character(player_id).unwrap();
    assert_eq!(player.farmy_soldier_type(0), 0);
}

// C `update_soldier(co, n, ppd)` (`fdemon.c:394-449`), the promotion
// re-equip half `World::fdemon_platoon_exp` can't reach without
// `ZoneLoader` (see `area8_army.rs`'s own doc comment): rescales
// `value[1]` at the new rank's `base` and swaps every equipped item for
// the new tier, destroying the old ones.
#[tokio::test]
async fn reequip_soldier_for_promotion_rescales_stats_and_swaps_equipment() {
    use ugaris_core::entity::CharacterValue;
    use ugaris_core::world::npc::area8::fdemon_army::SOLDIER_TYPE_WARRIOR;

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

    let soldier_id = CharacterId(50);
    let before = world.characters.get(&soldier_id).unwrap().clone();
    // Rank 1 -> base 47 -> tier 5 (already asserted by the spawn test).
    assert_eq!(before.values[1][CharacterValue::ArmorSkill as usize], 47);
    let old_item_ids: Vec<ItemId> = before.inventory.iter().flatten().copied().collect();
    assert_eq!(old_item_ids.len(), 5); // sleeves/armor/helmet/leggings/sword

    crate::area8_army::reequip_soldier_for_promotion(
        &mut world,
        &mut loader,
        soldier_id,
        SOLDIER_TYPE_WARRIOR,
        2, // base = 43 + 2*4 = 51 -> tier 6
    );

    let after = world.characters.get(&soldier_id).unwrap();
    assert_eq!(after.values[1][CharacterValue::ArmorSkill as usize], 51);
    assert_eq!(after.values[1][CharacterValue::Sword as usize], 51);
    assert_eq!(after.exp, after.exp_used);

    // Every old item is destroyed...
    for item_id in &old_item_ids {
        assert!(!world.items.contains_key(item_id));
    }
    // ...and replaced with 5 fresh ones at the new tier.
    let new_item_ids: Vec<ItemId> = after.inventory.iter().flatten().copied().collect();
    assert_eq!(new_item_ids.len(), 5);
    for item_id in &new_item_ids {
        assert!(world.items.contains_key(item_id));
        assert!(!old_item_ids.contains(item_id));
    }
}

// C `take_soldiers`/`drop_soldiers` copy `dat->emote` to/from `ppd->
// soldier[n].emote` (`fdemon.c:559-563,608-612`): a soldier's personality/
// relationship state survives a drop/re-recruit cycle, except the three
// "current need" fields (`boredom`/`fear`/`praise`) which `take_soldiers`
// always resets to `0` on every (re)spawn.
#[tokio::test]
async fn drop_and_retake_carries_emote_state_except_current_needs() {
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
    let soldier_id = CharacterId(50);
    let tendencies = {
        let Some(CharacterDriverState::FdemonArmy(dat)) =
            world.characters.get(&soldier_id).unwrap().driver_state
        else {
            panic!("expected FdemonArmy driver state");
        };
        (
            dat.emote.cuddly,
            dat.emote.angst,
            dat.emote.bore,
            dat.emote.bigmouth,
        )
    };

    // Simulate live gameplay having built up relationship/need state.
    set_soldier_emote(&mut world, soldier_id, |emote| {
        emote.likes[0] = 7;
        emote.talked[1] = 3;
        emote.answer_cn = 99;
        emote.answer_type = 2;
        emote.answer_timer = 123;
        emote.last_emote = 456;
        emote.boredom = 10;
        emote.fear = 20;
        emote.praise = 30;
    });

    crate::area8_army::drop_soldiers(&mut world, &mut runtime, player_id);
    assert!(!world.characters.contains_key(&soldier_id));

    // The PPD now carries the full emote state the live soldier had,
    // including the "current need" fields (drop doesn't reset them - only
    // a later `take_soldiers` respawn does).
    let player = runtime.player_for_character(player_id).unwrap();
    let saved = player.farmy_soldier_emote(0);
    assert_eq!(saved.likes[0], 7);
    assert_eq!(saved.talked[1], 3);
    assert_eq!(saved.answer_cn, 99);
    assert_eq!(saved.answer_type, 2);
    assert_eq!(saved.answer_timer, 123);
    assert_eq!(saved.last_emote, 456);
    assert_eq!(saved.boredom, 10);
    assert_eq!(saved.fear, 20);
    assert_eq!(saved.praise, 30);

    runtime.set_next_character_id(60);
    crate::area8_army::take_soldiers(&mut world, &mut loader, &mut runtime, player_id);
    let rebuilt_id = CharacterId(60);
    let Some(CharacterDriverState::FdemonArmy(dat)) =
        world.characters.get(&rebuilt_id).unwrap().driver_state
    else {
        panic!("expected FdemonArmy driver state");
    };
    // Relationship/tendency/pending-answer state carried over...
    assert_eq!(dat.emote.likes[0], 7);
    assert_eq!(dat.emote.talked[1], 3);
    assert_eq!(dat.emote.answer_cn, 99);
    assert_eq!(dat.emote.answer_type, 2);
    assert_eq!(dat.emote.answer_timer, 123);
    assert_eq!(dat.emote.last_emote, 456);
    assert_eq!(
        (
            dat.emote.cuddly,
            dat.emote.angst,
            dat.emote.bore,
            dat.emote.bigmouth
        ),
        tendencies
    );
    // ...but the three "current need" fields were reset to 0 on respawn.
    assert_eq!(dat.emote.boredom, 0);
    assert_eq!(dat.emote.fear, 0);
    assert_eq!(dat.emote.praise, 0);

    // And the PPD itself reflects that same reset (persisted, not just the
    // live copy), matching C's `ppd->soldier[n].emote.boredom = 0` etc.
    let player = runtime.player_for_character(player_id).unwrap();
    let saved_after_retake = player.farmy_soldier_emote(0);
    assert_eq!(saved_after_retake.boredom, 0);
    assert_eq!(saved_after_retake.fear, 0);
    assert_eq!(saved_after_retake.praise, 0);
    assert_eq!(saved_after_retake.likes[0], 7);
}
