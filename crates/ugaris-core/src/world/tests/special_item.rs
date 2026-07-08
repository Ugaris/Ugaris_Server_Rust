use super::*;
use crate::character_driver::{MerchantDriverData, CDR_MERCHANT};
use crate::world::special_item::MERCHANT_SPECIAL_REFRESH_TICKS;

/// Registers every fixed-key `ITEM_TYPE_TEMPLATES` entry plus all ten
/// quality tiers of the eight `%dq3`-style families, so any
/// `create_special_item`/`add_special_store` roll (regardless of which of
/// the 21 item types or which `base` tier the RNG picks) resolves to a
/// real template - used by tests that don't want to hand-pin the exact
/// RNG-selected item type.
fn load_all_special_item_type_templates(loader: &mut ZoneLoader) {
    let mut text = String::new();
    for family in [
        "armor",
        "helmet",
        "sleeves",
        "leggings",
        "sword",
        "twohanded",
        "dagger",
        "staff",
    ] {
        for tier in 1..=10 {
            text.push_str(&format!(
                "{family}{tier}q3:\nname=\"{family}{tier}q3\"\nsprite=1\nvalue=0\nflag=IF_TAKE\n;\n"
            ));
        }
    }
    for flat in [
        "plain_gold_ring",
        "green_hat",
        "brown_hat",
        "blue_cape",
        "brown_cape",
        "red_belt",
        "amulet",
        "boots",
        "vest",
        "trousers",
        "bracelet",
        "gloves",
    ] {
        text.push_str(&format!(
            "{flat}:\nname=\"{flat}\"\nsprite=1\nvalue=0\nflag=IF_TAKE\n;\n"
        ));
    }
    loader.load_item_templates_str(&text).unwrap();
}

fn store_ware_total(store: &MerchantStore) -> u32 {
    store.wares.iter().flatten().map(|ware| ware.count).sum()
}

#[test]
fn create_special_item_builds_deterministic_equipment_item() {
    // Seed 0 with strength=5/base=1/potionprob=1/maxchance=1000 walks a
    // fixed C-style-LCG path: potion check always misses (potionprob=1 is
    // 0% per the C doc comment), rolls item-type index 18 ("trousers",
    // ignores `base`), `lowhi_random(5)` collapses to strength 1, and the
    // weighted special-entry roll lands on "Surround Hit" (verified via a
    // Python replica of the exact RNG/table sequence before writing this
    // assertion).
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
            trousers:
                name="Trousers"
                sprite=500
                value=0
                flag=IF_TAKE
            ;
            "#,
        )
        .unwrap();
    let mut world = World::default();
    world.legacy_random_seed = 0;

    let item = world
        .create_special_item(&mut loader, 5, 1, 1, 1000)
        .expect("template exists, roll is deterministic");

    assert_eq!(item.name, "Trousers");
    assert_eq!(item.description, "Trousers of Extremely Weak Surround Hit.");
    assert_eq!(item.value, 200);
    assert_eq!(item.modifier_index[0], CharacterValue::Surround as i16);
    assert_eq!(item.modifier_value[0], 1);
    assert_eq!(
        item.modifier_index[1], 0,
        "only one modifier for a single-stat entry"
    );
    assert_eq!(
        item.min_level, 2,
        "C set_item_requirements_sub: high=1 -> lvl=2"
    );
    assert_eq!(item.template_id, IID_GENERIC_SPECIAL);
}

#[test]
fn create_special_item_potion_branch_returns_unmodified_potion_template() {
    // Seed 0 with potionprob=2 (50%) takes the potion branch: RANDOM(2) on
    // seed 0 is odd, strength 5+2=7 -> potion level 2, and the
    // healing/mana/combo roll picks "mana".
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
            mana_potion2:
                name="Mana Potion"
                sprite=600
                value=0
                flag=IF_TAKE
            ;
            "#,
        )
        .unwrap();
    let mut world = World::default();
    world.legacy_random_seed = 0;

    let item = world
        .create_special_item(&mut loader, 5, 1, 2, 1000)
        .expect("mana_potion2 template exists");

    assert_eq!(item.name, "Mana Potion");
    assert_eq!(
        item.description, "",
        "the potion branch returns the template verbatim, no description rewrite"
    );
    assert_eq!(
        item.modifier_index, [0; MAX_MODIFIERS],
        "the potion branch never touches modifiers"
    );
}

#[test]
fn create_special_item_returns_none_when_template_is_missing() {
    let mut loader = ZoneLoader::new();
    let mut world = World::default();
    world.legacy_random_seed = 0;

    assert!(world
        .create_special_item(&mut loader, 5, 1, 1, 1000)
        .is_none());
}

#[test]
fn add_special_store_requires_an_existing_store() {
    let mut loader = ZoneLoader::new();
    load_all_special_item_type_templates(&mut loader);
    let mut world = World::default();

    assert!(
        !world.add_special_store(CharacterId(1), &mut loader),
        "C: add_special_store returns 0 for ERR_ILLEGAL_STORENO when ch[cn].store is unset"
    );
}

#[test]
fn add_special_store_adds_one_ware_to_an_existing_store() {
    let mut loader = ZoneLoader::new();
    load_all_special_item_type_templates(&mut loader);
    let mut world = World::default();
    let mut merchant = character(1);
    merchant.driver = CDR_MERCHANT;
    merchant.driver_state = Some(CharacterDriverState::Merchant(MerchantDriverData::default()));
    assert!(world.spawn_character(merchant, 10, 10));
    assert!(world.ensure_merchant_store(CharacterId(1)));

    assert!(world.add_special_store(CharacterId(1), &mut loader));

    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert_eq!(store_ware_total(store), 1);
}

#[test]
fn refresh_special_stores_seeds_five_then_refreshes_every_twelve_hours() {
    let mut loader = ZoneLoader::new();
    load_all_special_item_type_templates(&mut loader);
    let mut world = World::default();
    let mut merchant = character(1);
    merchant.driver = CDR_MERCHANT;
    merchant.driver_state = Some(CharacterDriverState::Merchant(MerchantDriverData {
        special: 1,
        ..MerchantDriverData::default()
    }));
    assert!(world.spawn_character(merchant, 10, 10));
    assert!(world.ensure_merchant_store(CharacterId(1)));
    world.tick = Tick(1);

    // C `merchant_driver`: `if (dat->special) for (n=0;n<5;n++)
    // add_special_store(cn);` the first time the store exists.
    world.refresh_special_stores(&mut loader);
    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert_eq!(store_ware_total(store), 5);
    let last_special_add = match world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .driver_state
        .as_ref()
    {
        Some(CharacterDriverState::Merchant(data)) => data.last_special_add,
        _ => panic!("merchant driver state"),
    };
    assert_eq!(last_special_add, 1);

    // Re-running the same tick must not add a sixth item.
    world.refresh_special_stores(&mut loader);
    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert_eq!(store_ware_total(store), 5);

    // C: `ticker > dat->lastadd + TICKS*60*60*12`.
    world.tick = Tick(1 + MERCHANT_SPECIAL_REFRESH_TICKS + 1);
    world.refresh_special_stores(&mut loader);
    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert_eq!(store_ware_total(store), 6);
}

fn welding_test_character(level: u32) -> Character {
    let mut ch = character(1);
    ch.level = level;
    ch.flags.insert(CharacterFlags::PAID);
    ch
}

#[test]
fn shrine_welding_rejects_underpowered_characters() {
    // C `shrine_welding` (`random.c:1935`): `ch[cn].level + ch[cn].level/4
    // + 2 < level` - level 1 gives `1 + 0 + 2 = 3`, so a shrine level of 4
    // is out of reach.
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), welding_test_character(1));

    assert_eq!(
        world.apply_random_shrine_welding(CharacterId(1), 4),
        RandomShrineWeldingResult::NotPowerfulEnough
    );
}

#[test]
fn shrine_welding_requires_a_paying_player() {
    let mut world = World::default();
    let mut ch = welding_test_character(50);
    ch.flags.remove(CharacterFlags::PAID);
    world.characters.insert(CharacterId(1), ch);

    assert_eq!(
        world.apply_random_shrine_welding(CharacterId(1), 1),
        RandomShrineWeldingResult::NotPaying
    );
}

#[test]
fn shrine_welding_reports_contempt_with_no_donatable_modifiers() {
    let mut world = World::default();
    world
        .characters
        .insert(CharacterId(1), welding_test_character(50));

    assert_eq!(
        world.apply_random_shrine_welding(CharacterId(1), 1),
        RandomShrineWeldingResult::Contempt,
        "no worn items at all -> no eligible donor"
    );
}

#[test]
fn shrine_welding_reports_regret_when_no_item_can_receive_the_modifier() {
    // A single worn item can never weld with itself (`tmp != in2` in C's
    // second search loop), so one donor and nothing else worn is "regret",
    // not a self-weld.
    let mut world = World::default();
    let mut ch = welding_test_character(50);
    let mut donor = item(10, ItemFlags::empty());
    donor.modifier_index[0] = CharacterValue::Attack as i16;
    donor.modifier_value[0] = 3;
    ch.inventory[0] = Some(ItemId(10));
    world.items.insert(ItemId(10), donor);
    world.characters.insert(CharacterId(1), ch);

    assert_eq!(
        world.apply_random_shrine_welding(CharacterId(1), 1),
        RandomShrineWeldingResult::Regret
    );
}

#[test]
fn shrine_welding_moves_a_modifier_between_two_worn_items() {
    let mut world = World::default();
    world.legacy_random_seed = 0;
    let mut ch = welding_test_character(50);
    let mut donor = item(10, ItemFlags::empty());
    donor.name = "Donor Sword".into();
    donor.modifier_index[0] = CharacterValue::Attack as i16;
    donor.modifier_value[0] = 3;
    let mut receiver = item(11, ItemFlags::empty());
    receiver.name = "Receiver Shield".into();
    ch.inventory[0] = Some(ItemId(10));
    ch.inventory[1] = Some(ItemId(11));
    world.items.insert(ItemId(10), donor);
    world.items.insert(ItemId(11), receiver);
    world.characters.insert(CharacterId(1), ch);

    let result = world.apply_random_shrine_welding(CharacterId(1), 1);

    assert_eq!(
        result,
        RandomShrineWeldingResult::Used {
            item1_name: "Receiver Shield".to_string(),
            item2_name: "Donor Sword".to_string(),
        }
    );
    let receiver = &world.items[&ItemId(11)];
    assert_eq!(receiver.modifier_index[0], CharacterValue::Attack as i16);
    assert_eq!(receiver.modifier_value[0], 3);
    assert_eq!(receiver.description, "Receiver Shield of Welding.");
    let donor = &world.items[&ItemId(10)];
    assert_eq!(donor.modifier_index[0], 0);
    assert_eq!(donor.modifier_value[0], 0);
    assert_eq!(donor.description, "Donor Sword of Unwelding.");
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .flags
        .contains(CharacterFlags::ITEMS));
}

#[test]
fn shrine_welding_never_touches_weapon_armor_speed_demon_light_mods() {
    // C `can_give_mod`: `V_WEAPON`/`V_ARMOR`/`V_SPEED`/`V_DEMON`/`V_LIGHT`
    // modifiers can never be donated away.
    let mut world = World::default();
    world.legacy_random_seed = 0;
    let mut ch = welding_test_character(50);
    let mut only_excluded = item(10, ItemFlags::empty());
    only_excluded.modifier_index[0] = CharacterValue::Weapon as i16;
    only_excluded.modifier_value[0] = 3;
    let receiver = item(11, ItemFlags::empty());
    ch.inventory[0] = Some(ItemId(10));
    ch.inventory[1] = Some(ItemId(11));
    world.items.insert(ItemId(10), only_excluded);
    world.items.insert(ItemId(11), receiver);
    world.characters.insert(CharacterId(1), ch);

    assert_eq!(
        world.apply_random_shrine_welding(CharacterId(1), 1),
        RandomShrineWeldingResult::Contempt
    );
}

#[test]
fn shrine_welding_skips_items_with_noenhance_or_a_driver() {
    let mut world = World::default();
    world.legacy_random_seed = 0;
    let mut ch = welding_test_character(50);
    let mut donor = item(10, ItemFlags::NOENHANCE);
    donor.modifier_index[0] = CharacterValue::Attack as i16;
    donor.modifier_value[0] = 3;
    let mut driven = item(11, ItemFlags::empty());
    driven.driver = 5;
    ch.inventory[0] = Some(ItemId(10));
    ch.inventory[1] = Some(ItemId(11));
    world.items.insert(ItemId(10), donor);
    world.items.insert(ItemId(11), driven);
    world.characters.insert(CharacterId(1), ch);

    assert_eq!(
        world.apply_random_shrine_welding(CharacterId(1), 1),
        RandomShrineWeldingResult::Contempt,
        "NOENHANCE blocks the only possible donor; the receiver has a driver too"
    );
}

#[test]
fn refresh_special_stores_ignores_merchants_without_the_special_flag() {
    let mut loader = ZoneLoader::new();
    load_all_special_item_type_templates(&mut loader);
    let mut world = World::default();
    let mut merchant = character(1);
    merchant.driver = CDR_MERCHANT;
    merchant.driver_state = Some(CharacterDriverState::Merchant(MerchantDriverData::default()));
    assert!(world.spawn_character(merchant, 10, 10));
    assert!(world.ensure_merchant_store(CharacterId(1)));
    world.tick = Tick(1);

    world.refresh_special_stores(&mut loader);

    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert_eq!(store_ware_total(store), 0);
}
