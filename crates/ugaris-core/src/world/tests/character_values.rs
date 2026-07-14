// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
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

/// C `update_char`: with Body Control raised, a *real* weapon (not just
/// `IF_HAND`) in the right hand suppresses the bare-handed Weapon bonus
/// entirely - regression test for the `IF_WEAPON` composite-flag check
/// (must be `intersects`, not `contains`; a real sword only sets
/// `IF_SWORD`, never every weapon-class bit at once).
#[test]
fn body_control_bare_handed_bonus_is_suppressed_by_a_real_weapon_in_hand() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.values[1][CharacterValue::BodyControl as usize] = 20;
    actor.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(10));
    assert!(world.spawn_character(actor, 50, 50));

    let sword = item(10, ItemFlags::WNRHAND | ItemFlags::SWORD | ItemFlags::USED);
    world.add_item(sword);

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    let body_control = actor.values[0][CharacterValue::BodyControl as usize];
    assert_eq!(body_control, 20);
    // Weapon = body_control/4 (=5) only; no bare-handed bonus since a real
    // weapon (SWORD) is in the right hand.
    assert_eq!(actor.values[0][CharacterValue::Weapon as usize], 5);
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

/// C `update_char` (`create.c:1996-2039`): a bare-handed warrior male
/// player gets sprite base 85 + off 0 (nothing in hand).
#[test]
fn sprite_recompute_selects_class_gender_base_when_unarmed() {
    let mut world = World::default();
    let mut actor = character(1);
    actor
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::WARRIOR | CharacterFlags::MALE);
    assert!(world.spawn_character(actor, 50, 50));

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.sprite, 85);
}

/// C `update_char`: a two-handed weapon in the right hand adds `off = 2`
/// on top of the class/gender base sprite.
#[test]
fn sprite_recompute_adds_two_handed_weapon_offset() {
    let mut world = World::default();
    let mut actor = character(1);
    actor
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::WARRIOR | CharacterFlags::MALE);
    actor.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(10));
    assert!(world.spawn_character(actor, 50, 50));

    let weapon = item(
        10,
        ItemFlags::WNRHAND | ItemFlags::SWORD | ItemFlags::WNTWOHANDED | ItemFlags::USED,
    );
    world.add_item(weapon);

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.sprite, 85 + 2);
}

/// C `update_char` (`create.c:2097-2108`): a full six-slot demon-skin-1
/// suit overrides the normal class/weapon sprite selection to 27 with a
/// zero offset, regardless of what is otherwise worn/in-hand.
#[test]
fn sprite_recompute_overrides_to_demonskin_sprite_when_full_suit_worn() {
    let mut world = World::default();
    let mut actor = character(1);
    actor
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::WARRIOR | CharacterFlags::MALE);
    let slots = [
        (worn_slot::HEAD, 11u32),
        (worn_slot::ARMS, 12),
        (worn_slot::LEGS, 13),
        (worn_slot::BODY, 14),
        (worn_slot::CLOAK, 15),
        (worn_slot::FEET, 16),
    ];
    for &(slot, id) in &slots {
        actor.inventory[slot] = Some(ItemId(id));
    }
    assert!(world.spawn_character(actor, 50, 50));

    for &(_, id) in &slots {
        let mut piece = item(id, ItemFlags::USED);
        piece.template_id = (0x01 << 24) | 0x0000A8; // IID_DEMONSKIN1
        world.add_item(piece);
    }

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.sprite, 27);
}

/// C `update_char` (`create.c:1970-1971`): gods keep their custom admin
/// sprite untouched unless it already falls in the player sprite ranges
/// (`60..120`, or the demon sprites 27/157/39).
#[test]
fn sprite_recompute_leaves_god_admin_sprite_untouched() {
    let mut world = World::default();
    let mut actor = character(1);
    actor
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::GOD | CharacterFlags::WARRIOR);
    actor.sprite = 999;
    assert!(world.spawn_character(actor, 50, 50));

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.sprite, 999);
}

/// A sprite change from equipping a weapon marks the character's tile
/// dirty, matching C's `set_sector` call on sprite change.
#[test]
fn sprite_recompute_marks_dirty_sector_on_change() {
    let mut world = World::default();
    let mut actor = character(1);
    actor
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::WARRIOR | CharacterFlags::MALE);
    assert!(world.spawn_character(actor, 50, 50));
    assert!(world.update_character(CharacterId(1)));

    world.tick.0 = 5;
    assert!(world.skip_x_sector(50, 50, world.tick.0) > 0);

    let actor = world.characters.get_mut(&CharacterId(1)).unwrap();
    actor.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(10));
    let weapon = item(
        10,
        ItemFlags::WNRHAND | ItemFlags::SWORD | ItemFlags::WNTWOHANDED | ItemFlags::USED,
    );
    world.add_item(weapon);

    assert!(world.update_character(CharacterId(1)));
    assert_eq!(world.skip_x_sector(50, 50, world.tick.0), 0);
}

/// C `create.c:1856`: `ch[cn].prof[P_CLAN] && n >= V_WIS && n <= V_STR &&
/// (areaID == 13 || (mmf & MF_CLAN))` - a clan master's base-attribute
/// bonus applies in the catacombs (area 13) even on a tile without the
/// `MF_CLAN` map flag.
#[test]
fn clan_profession_bonus_applies_in_area_13_catacombs_without_clan_tile_flag() {
    let mut world = World::default();
    world.area_id = 13;
    let mut actor = character(1);
    actor.professions[profession::CLAN] = 4;
    actor.values[1][CharacterValue::Wisdom as usize] = 50;
    assert!(world.spawn_character(actor, 50, 50));

    // The spawn tile has no `MF_CLAN` flag; only `world.area_id == 13`
    // grants the bonus here.
    assert!(!world
        .map
        .tile(50, 50)
        .unwrap()
        .flags
        .contains(MapFlags::CLAN));

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.values[0][CharacterValue::Wisdom as usize], 54);
}

/// Outside area 13 and off a clan-flagged tile, the same profession grants
/// no bonus.
#[test]
fn clan_profession_bonus_does_not_apply_outside_area_13_or_clan_tile() {
    let mut world = World::default();
    world.area_id = 1;
    let mut actor = character(1);
    actor.professions[profession::CLAN] = 4;
    actor.values[1][CharacterValue::Wisdom as usize] = 50;
    assert!(world.spawn_character(actor, 50, 50));

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.values[0][CharacterValue::Wisdom as usize], 50);
}

/// C `create.c:1785-1797`: `mod[V_LIGHT] += ef[fn].light` for each of the
/// character's attached effects (`ch[cn].ef[0..4]`). A character-attached
/// `EF_MAGICSHIELD` (`create_show_effect(EF_MAGICSHIELD, cn, ..., 25, 0)`,
/// `src/system/act.c:1088`) contributes its light straight into V_LIGHT,
/// uncapped (V_LIGHT sits outside the `n <= V_STR || n >= V_PULSE` mod-cap
/// range in C).
#[test]
fn character_attached_effect_light_contributes_to_v_light() {
    let mut world = World::default();
    let actor = character(1);
    assert!(world.spawn_character(actor, 50, 50));

    let effect_id = world.next_effect_id();
    let mut effect = Effect::new(EF_MAGICSHIELD, effect_id as i32, 0, 3);
    effect.target_character = Some(CharacterId(1));
    effect.light = 25;
    world.effects.insert(effect_id, effect);

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(actor.values[0][CharacterValue::Light as usize], 25);
}

/// C `effect.c:209-238` `add_effect_char`: only the first four attached
/// effects occupy `ch[cn].ef[0..4]`; a fifth attachment attempt fails and
/// contributes no light. `World::character_attached_effect_light`
/// approximates this by summing only the four lowest-id (earliest
/// attached) character-attached effects.
#[test]
fn character_attached_effect_light_caps_at_four_effects_by_creation_order() {
    let mut world = World::default();
    let actor = character(1);
    assert!(world.spawn_character(actor, 50, 50));

    for _ in 0..5 {
        let effect_id = world.next_effect_id();
        let mut effect = Effect::new(EF_FIRERING, effect_id as i32, 0, 100);
        effect.target_character = Some(CharacterId(1));
        effect.light = 20;
        world.effects.insert(effect_id, effect);
    }

    assert!(world.update_character(CharacterId(1)));
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    // Only the first four (of five) attached effects' light counts, matching
    // C's fixed four-slot `ch.ef[]` cap: 4 * 20 = 80, not 5 * 20 = 100.
    assert_eq!(actor.values[0][CharacterValue::Light as usize], 80);
}
