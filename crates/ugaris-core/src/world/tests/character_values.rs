use super::*;

/// C `update_char` (`src/system/create.c:1710`): wearing an item adds its
/// positive modifiers to `value[0]` on top of `value[1]` (base/raised).
#[test]
fn wearing_item_adds_modifier_to_effective_value() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::WARRIOR);
    actor.values[1][CharacterValue::Strength as usize] = 100;
    actor.inventory[worn_slot::BODY] = Some(ItemId(10));
    assert!(world.spawn_character(actor, 50, 50));

    let mut armor = item(10, ItemFlags::WNBODY | ItemFlags::USED);
    armor.modifier_index[0] = CharacterValue::Strength as i16;
    armor.modifier_value[0] = 10;
    world.add_item(armor);

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.values[0][CharacterValue::Strength as usize], 110);
}

/// Removing the item drops the modifier back out on the next recompute.
#[test]
fn removing_item_clears_modifier_from_effective_value() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::WARRIOR);
    actor.values[1][CharacterValue::Strength as usize] = 100;
    actor.inventory[worn_slot::BODY] = Some(ItemId(10));
    assert!(world.spawn_character(actor, 50, 50));

    let mut armor = item(10, ItemFlags::WNBODY | ItemFlags::USED);
    armor.modifier_index[0] = CharacterValue::Strength as i16;
    armor.modifier_value[0] = 10;
    world.add_item(armor);
    assert!(world.update_character(CharacterId(1)));

    let actor = world.characters.get_mut(&CharacterId(1)).unwrap();
    actor.inventory[worn_slot::BODY] = None;
    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.values[0][CharacterValue::Strength as usize], 100);
}

/// C `update_char`: `mod[n] = min(value[1][n] * 0.500, mod[n])` for
/// warrior/mage (non-Seyan'Du) characters on powers/attributes and
/// skills/spells - a +200 modifier only grants half of the 100 base.
#[test]
fn item_modifier_is_capped_at_half_of_raised_value_for_single_class() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::WARRIOR);
    actor.values[1][CharacterValue::Strength as usize] = 100;
    actor.inventory[worn_slot::BODY] = Some(ItemId(10));
    assert!(world.spawn_character(actor, 50, 50));

    let mut armor = item(10, ItemFlags::WNBODY | ItemFlags::USED);
    armor.modifier_index[0] = CharacterValue::Strength as i16;
    armor.modifier_value[0] = 200;
    world.add_item(armor);

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    // base(0) + raised(100) + capped mod(50) + bless(0)
    assert_eq!(actor.values[0][CharacterValue::Strength as usize], 150);
}

/// Seyan'Du (warrior+mage) get a looser 72.5% cap instead of 50%.
#[test]
fn item_modifier_cap_is_looser_for_seyan_du() {
    let mut world = World::default();
    let mut actor = character(1);
    actor
        .flags
        .insert(CharacterFlags::WARRIOR | CharacterFlags::MAGE);
    actor.values[1][CharacterValue::Strength as usize] = 100;
    actor.inventory[worn_slot::BODY] = Some(ItemId(10));
    assert!(world.spawn_character(actor, 50, 50));

    let mut armor = item(10, ItemFlags::WNBODY | ItemFlags::USED);
    armor.modifier_index[0] = CharacterValue::Strength as i16;
    armor.modifier_value[0] = 200;
    world.add_item(armor);

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    // base(0) + raised(100) + capped mod(72) + bless(0)
    assert_eq!(actor.values[0][CharacterValue::Strength as usize], 172);
}

/// Items flagged `IF_BEYONDMAXMOD` bypass the cap entirely (C: added to
/// `beyond[]`, applied after the capped `mod[]`/`bless[]` totals).
#[test]
fn beyond_max_mod_item_bypasses_the_cap() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::WARRIOR);
    actor.values[1][CharacterValue::Strength as usize] = 100;
    actor.inventory[worn_slot::BODY] = Some(ItemId(10));
    assert!(world.spawn_character(actor, 50, 50));

    let mut armor = item(
        10,
        ItemFlags::WNBODY | ItemFlags::USED | ItemFlags::BEYONDMAXMOD,
    );
    armor.modifier_index[0] = CharacterValue::Strength as i16;
    armor.modifier_value[0] = 200;
    world.add_item(armor);

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.values[0][CharacterValue::Strength as usize], 300);
}

/// C `update_char`: `if (!value[1][n] && n >= V_PULSE) value[0][n] = 0;`
/// - a character with zero raised Dagger skill gets no item bonus to it.
#[test]
fn skill_with_no_raised_points_stays_zero_even_with_item_bonus() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(10));
    assert!(world.spawn_character(actor, 50, 50));

    let mut dagger = item(10, ItemFlags::WNRHAND | ItemFlags::DAGGER | ItemFlags::USED);
    dagger.modifier_index[0] = CharacterValue::Dagger as i16;
    dagger.modifier_value[0] = 20;
    world.add_item(dagger);

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.values[0][CharacterValue::Dagger as usize], 0);
}

/// C `update_char`: Speed Skill grants half its value to Speed, and the
/// Athlete profession adds a flat `prof * 3` bonus.
#[test]
fn speed_skill_and_athlete_profession_boost_speed() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.values[1][CharacterValue::SpeedSkill as usize] = 40;
    actor.professions[profession::ATHLETE] = 2;
    assert!(world.spawn_character(actor, 50, 50));

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    // Speed base is 0 (no items), SpeedSkill totals to base(0)+raised(40)=40,
    // so Speed = 40/2 (speed skill bonus) + 2*3 (athlete) = 26.
    assert_eq!(actor.values[0][CharacterValue::Speed as usize], 26);
}

/// C `update_char`: without Body Control, Armor gets a
/// `get_spell_average(cn) * 17.5` bonus instead of the melee bonuses.
#[test]
fn armor_gets_spell_average_bonus_without_body_control() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.values[1][CharacterValue::Bless as usize] = 80;
    assert!(world.spawn_character(actor, 50, 50));

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    // Bless totals to base(0)+raised(80)=80 (no items), spell average = 80/8 = 10,
    // Armor bonus = 10 * 17.5 = 175.
    assert_eq!(actor.values[0][CharacterValue::Armor as usize], 175);
}

/// C `update_char`: Body Control instead grants Armor `*5` and Weapon
/// `/4` bonuses (plus a bare-handed Weapon bonus for players).
#[test]
fn body_control_boosts_armor_and_weapon_for_bare_handed_player() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.values[1][CharacterValue::BodyControl as usize] = 20;
    assert!(world.spawn_character(actor, 50, 50));

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    let body_control = actor.values[0][CharacterValue::BodyControl as usize];
    assert_eq!(body_control, 20);
    assert_eq!(
        actor.values[0][CharacterValue::Armor as usize],
        body_control * 5
    );
    // Weapon = body_control/4 (=5) + bare-handed bonus min(90, body_control/2=10) = 15.
    assert_eq!(actor.values[0][CharacterValue::Weapon as usize], 15);
}

/// C `update_char`: caps the current HP/endurance/mana to the freshly
/// recomputed max whenever gear loss (or any recompute) lowers the max
/// below the current value.
#[test]
fn recompute_clamps_current_hp_to_new_max() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.values[1][CharacterValue::Hp as usize] = 10;
    actor.hp = 50 * POWERSCALE;
    assert!(world.spawn_character(actor, 50, 50));

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.hp, 10 * POWERSCALE);
}

/// C `update_char`: sets `CF_UPDATE` unconditionally so the client resync
/// pass picks up the new values.
#[test]
fn recompute_always_sets_update_flag() {
    let mut world = World::default();
    let actor = character(1);
    assert!(world.spawn_character(actor, 50, 50));

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert!(actor.flags.contains(CharacterFlags::UPDATE));
}
