use super::*;
use crate::quest::QuestLog;

fn load_item(loader: &mut ZoneLoader, key: &str) {
    loader
        .load_item_templates_str(&format!(
            "{key}:\nname=\"{key}\"\nsprite=1\nvalue=0\nflag=IF_TAKE\n;\n"
        ))
        .unwrap();
}

struct NoQuests;
impl LootQuestContext for NoQuests {
    fn quest_is_done(&self, _quest: u32) -> bool {
        false
    }
    fn quest_count(&self, _quest: u32) -> u8 {
        0
    }
}

#[test]
fn parses_shorthand_single_group_table_with_pity_and_modifiers() {
    // Real fixture content (`ugaris_data/loot/pents/demons.json`'s
    // `pent_demon_low`), transcribed verbatim.
    let json = r#"{
        "id": "pent_demon_low",
        "rolls": 1,
        "pity": {"counter": "pent_demons", "threshold": 1700},
        "modifiers": ["event_drop_rate"],
        "entries": [
            {"weight": 100, "table": "demon_low_equipment"},
            {"weight": 600, "item": "bronzechip"}
        ]
    }"#;
    let mut registry = LootRegistry::default();
    let report = registry.load_str(json);
    assert_eq!(report.tables_added, 1);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);

    let table = registry.find("pent_demon_low").expect("table parsed");
    assert_eq!(table.mode, LootMode::Spawn, "no \"mode\" key => spawn");
    assert_eq!(table.groups.len(), 1);
    let group = &table.groups[0];
    assert_eq!(group.rolls, 1);
    assert_eq!(group.pity.counter, "pent_demons");
    assert_eq!(group.pity.threshold, 1700);
    assert_eq!(group.modifiers, vec!["event_drop_rate".to_string()]);
    assert_eq!(group.total_weight, 700);
    assert_eq!(group.entries.len(), 2);
    assert!(matches!(group.entries[0].kind, LootEntryKind::Table));
    assert_eq!(group.entries[0].reference, "demon_low_equipment");
    assert!(matches!(group.entries[1].kind, LootEntryKind::Item));
    assert_eq!(group.entries[1].reference, "bronzechip");
}

#[test]
fn parses_full_form_multiple_groups_with_conditions_and_death_mode() {
    let json = r#"{
        "id": "monster_drops",
        "mode": "death",
        "groups": [
            {
                "rolls": 1,
                "entries": [
                    {"weight": 70, "item": "healing_potion"},
                    {"weight": 30, "item": "mana_potion"}
                ]
            },
            {
                "condition": {"quest_open": 5},
                "rolls": 1,
                "entries": [{"weight": 1, "item": "lydia_token"}]
            },
            {
                "condition": {"killer_level_ge": 40},
                "entries": [{"weight": 1, "nothing": true}]
            }
        ]
    }"#;
    let mut registry = LootRegistry::default();
    let report = registry.load_str(json);
    assert_eq!(report.tables_added, 1);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);

    let table = registry.find("monster_drops").expect("table parsed");
    assert_eq!(table.mode, LootMode::Death);
    assert_eq!(table.groups.len(), 3);
    assert_eq!(table.groups[1].condition, LootCondition::QuestOpen(5));
    assert_eq!(table.groups[2].condition, LootCondition::KillerLevelGe(40));
    assert!(matches!(
        table.groups[2].entries[0].kind,
        LootEntryKind::Nothing
    ));
}

#[test]
fn quest_count_conditions_parse_their_array_arguments() {
    let json = r#"{
        "id": "conditional",
        "groups": [
            {"condition": {"quest_count_lt": [3, 5]}, "entries": [{"weight": 1, "nothing": true}]},
            {"condition": {"quest_count_ge": [3, 5]}, "entries": [{"weight": 1, "nothing": true}]},
            {"condition": {"quest_done": 9}, "entries": [{"weight": 1, "nothing": true}]},
            {"condition": {"quest_not_done": 9}, "entries": [{"weight": 1, "nothing": true}]},
            {"condition": {"killer_level_lt": 10}, "entries": [{"weight": 1, "nothing": true}]}
        ]
    }"#;
    let mut registry = LootRegistry::default();
    registry.load_str(json);
    let table = registry.find("conditional").unwrap();
    assert_eq!(table.groups[0].condition, LootCondition::QuestCountLt(3, 5));
    assert_eq!(table.groups[1].condition, LootCondition::QuestCountGe(3, 5));
    assert_eq!(table.groups[2].condition, LootCondition::QuestDone(9));
    assert_eq!(table.groups[3].condition, LootCondition::QuestNotDone(9));
    assert_eq!(table.groups[4].condition, LootCondition::KillerLevelLt(10));
}

#[test]
fn malformed_entry_and_condition_are_skipped_with_warnings_but_table_still_loads() {
    let json = r#"{
        "id": "sloppy",
        "groups": [
            {
                "condition": {"nonsense": true},
                "entries": [
                    {"weight": 1},
                    {"weight": 2, "item": "healing_potion"}
                ]
            }
        ]
    }"#;
    let mut registry = LootRegistry::default();
    let report = registry.load_str(json);
    assert_eq!(report.tables_added, 1);
    assert_eq!(report.warnings.len(), 2);
    assert!(report.warnings[0].contains("unknown or malformed condition"));
    assert!(report.warnings[1].contains("entry has no item/table/nothing"));

    let table = registry.find("sloppy").unwrap();
    assert_eq!(table.groups[0].condition, LootCondition::None);
    assert_eq!(table.groups[0].entries.len(), 1);
    assert_eq!(table.groups[0].total_weight, 2);
}

#[test]
fn table_missing_id_and_group_missing_entries_are_rejected_with_warnings() {
    let mut registry = LootRegistry::default();

    let report = registry.load_str(r#"{"entries": [{"weight": 1, "nothing": true}]}"#);
    assert_eq!(report.tables_added, 0);
    assert!(report.warnings[0].contains("missing \"id\""));

    let report = registry.load_str(r#"{"id": "empty_group"}"#);
    assert_eq!(report.tables_added, 0);
    assert!(report
        .warnings
        .iter()
        .any(|w| w.contains("missing \"entries\"")));
}

#[test]
fn json_syntax_error_is_reported_without_panicking() {
    let mut registry = LootRegistry::default();
    let report = registry.load_str("{ not json");
    assert_eq!(report.tables_added, 0);
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].starts_with("loot: parse error"));
}

#[test]
fn loot_apply_to_container_returns_negative_one_for_unknown_table() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(1), None, "does_not_exist"),
        -1
    );
}

#[test]
fn loot_apply_to_container_returns_negative_one_for_spawn_mode_table() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    world
        .loot_registry
        .load_str(r#"{"id": "spawn_only", "entries": [{"weight": 1, "item": "bronzechip"}]}"#);
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(1), None, "spawn_only"),
        -1
    );
}

#[test]
fn loot_apply_to_container_places_items_and_resolves_sub_tables() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    load_item(&mut loader, "bronzechip");
    load_item(&mut loader, "demon_helmet1");

    world.loot_registry.load_str(
        r#"[
            {
                "id": "demon_low_equipment",
                "rolls": 1,
                "entries": [{"weight": 1, "item": "demon_helmet1"}]
            },
            {
                "id": "pent_demon_low",
                "mode": "death",
                "rolls": 2,
                "entries": [
                    {"weight": 1, "table": "demon_low_equipment"},
                    {"weight": 1, "item": "bronzechip"}
                ]
            }
        ]"#,
    );

    let container = ItemId(500);
    world.legacy_random_seed = 0;
    let added = world.loot_apply_to_container(&mut loader, container, None, "pent_demon_low");
    assert_eq!(added, 2, "two rolls, both entries resolve to a placed item");

    let contained: Vec<&str> = world
        .items
        .values()
        .filter(|item| item.contained_in == Some(container))
        .map(|item| item.name.as_str())
        .collect();
    assert_eq!(contained.len(), 2);
    assert!(contained.contains(&"demon_helmet1") || contained.contains(&"bronzechip"));
}

#[test]
fn roll_group_skips_condition_that_fails_without_a_killer() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    load_item(&mut loader, "healing_potion");
    world.loot_registry.load_str(
        r#"{
            "id": "gated",
            "mode": "death",
            "condition": {"killer_level_ge": 1},
            "entries": [{"weight": 1, "item": "healing_potion"}]
        }"#,
    );

    // C `eval_condition`: no killer (killer_cn=0) fails every
    // killer-dependent condition, regardless of its type.
    let added = world.loot_apply_to_container(&mut loader, ItemId(1), None, "gated");
    assert_eq!(added, 0);
}

#[test]
fn roll_group_condition_gates_on_killer_quest_state() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    load_item(&mut loader, "lydia_token");
    world.loot_registry.load_str(
        r#"{
            "id": "quest_gated",
            "mode": "death",
            "condition": {"quest_done": 5},
            "entries": [{"weight": 1, "item": "lydia_token"}]
        }"#,
    );

    let mut quests = QuestLog::default();
    let killer_not_done = LootKiller {
        character_id: CharacterId(2),
        level: 10,
        quest: &quests,
    };
    assert_eq!(
        world.loot_apply_to_container(
            &mut loader,
            ItemId(1),
            Some(&killer_not_done),
            "quest_gated"
        ),
        0,
        "quest_done fails while the quest isn't done"
    );

    quests.mark_done(5);
    let killer_done = LootKiller {
        character_id: CharacterId(2),
        level: 10,
        quest: &quests,
    };
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(2), Some(&killer_done), "quest_gated"),
        1,
        "quest_done passes once the quest is marked done"
    );
}

#[test]
fn quest_open_condition_is_the_permissive_not_done_proxy_not_is_open() {
    // C `LCOND_QUEST_OPEN`'s doc comment (`loot.c:619-625`): it's
    // `!questlog_isdone`, not the questlog's own `is_open` predicate -
    // so a quest that was never opened at all (flags == 0) still counts
    // as "open" here, identical to `quest_not_done`.
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    load_item(&mut loader, "lydia_token");
    world.loot_registry.load_str(
        r#"{
            "id": "quest_open_gated",
            "mode": "death",
            "condition": {"quest_open": 5},
            "entries": [{"weight": 1, "item": "lydia_token"}]
        }"#,
    );
    let quests = QuestLog::default();
    assert!(
        !quests.is_open(5),
        "never opened - questlog_isdone-style predicate differs"
    );
    let killer = LootKiller {
        character_id: CharacterId(2),
        level: 1,
        quest: &quests,
    };
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(1), Some(&killer), "quest_open_gated"),
        1,
        "quest_open passes for a never-done quest (the permissive proxy)"
    );
}

#[test]
fn pity_counter_gates_until_threshold_then_resets() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    load_item(&mut loader, "bronzechip");
    world.loot_registry.load_str(
        r#"{
            "id": "pity_gated",
            "mode": "death",
            "pity": {"counter": "shared_pity", "threshold": 2},
            "entries": [{"weight": 1, "item": "bronzechip"}]
        }"#,
    );

    // Roll 1 and 2: counter reaches 1 then 2, both <= threshold(2) => gated.
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(1), None, "pity_gated"),
        0
    );
    assert_eq!(world.loot_registry.pity_get("shared_pity"), 1);
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(2), None, "pity_gated"),
        0
    );
    assert_eq!(world.loot_registry.pity_get("shared_pity"), 2);
    // Roll 3: counter reaches 3 > threshold(2) => fires and resets to 0.
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(3), None, "pity_gated"),
        1
    );
    assert_eq!(world.loot_registry.pity_get("shared_pity"), 0);
}

#[test]
fn event_drop_rate_modifier_scales_rolls_and_relaxes_pity_threshold() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    load_item(&mut loader, "bronzechip");
    world.loot_registry.load_str(
        r#"{
            "id": "modified",
            "mode": "death",
            "rolls": 2,
            "pity": {"counter": "modified_pity", "threshold": 10},
            "modifiers": ["event_drop_rate"],
            "entries": [{"weight": 1, "item": "bronzechip"}]
        }"#,
    );
    world.settings.set_loot_modifier("event_drop_rate", 4.0);

    // Effective threshold = max(1, 10/4) = 2: first roll pushes the
    // counter to 1 (<=2, gated); the modifier also scales rolls to
    // ceil(2*4)=8, but that doesn't matter while still gated.
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(1), None, "modified"),
        0
    );
    assert_eq!(world.loot_registry.pity_get("modified_pity"), 1);
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(2), None, "modified"),
        0
    );
    assert_eq!(world.loot_registry.pity_get("modified_pity"), 2);
    // Third call: counter -> 3 > eff_threshold(2), fires with
    // ceil(2*4)=8 rolls, one item template so all 8 succeed.
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(3), None, "modified"),
        8
    );
    assert_eq!(world.loot_registry.pity_get("modified_pity"), 0);
}

#[test]
fn recursion_depth_cap_stops_a_self_referencing_sub_table() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    world.loot_registry.load_str(
        r#"{
            "id": "infinite",
            "mode": "death",
            "rolls": 1,
            "entries": [{"weight": 1, "table": "infinite"}]
        }"#,
    );
    // Must terminate (not stack overflow / hang) and add nothing, since
    // every recursive step is itself just another `LE_TABLE` entry.
    let added = world.loot_apply_to_container(&mut loader, ItemId(1), None, "infinite");
    assert_eq!(added, 0);
}

#[test]
fn unknown_quest_context_default_used_by_server_never_grants_conditional_drops() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    load_item(&mut loader, "lydia_token");
    world.loot_registry.load_str(
        r#"{
            "id": "quest_gated_no_ctx",
            "mode": "death",
            "condition": {"quest_done": 1},
            "entries": [{"weight": 1, "item": "lydia_token"}]
        }"#,
    );
    let killer = LootKiller {
        character_id: CharacterId(2),
        level: 1,
        quest: &NoQuests,
    };
    assert_eq!(
        world.loot_apply_to_container(&mut loader, ItemId(1), Some(&killer), "quest_gated_no_ctx"),
        0
    );
}

#[test]
fn loot_apply_to_npc_returns_negative_one_for_unknown_table() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    let npc = character(1);
    world.spawn_character(npc, 10, 10);
    assert_eq!(
        world.loot_apply_to_npc(&mut loader, CharacterId(1), "does_not_exist"),
        -1
    );
}

#[test]
fn loot_apply_to_npc_returns_negative_one_for_death_mode_table() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    let npc = character(1);
    world.spawn_character(npc, 10, 10);
    world.loot_registry.load_str(
        r#"{"id": "death_only", "mode": "death", "entries": [{"weight": 1, "item": "bronzechip"}]}"#,
    );
    assert_eq!(
        world.loot_apply_to_npc(&mut loader, CharacterId(1), "death_only"),
        -1
    );
}

#[test]
fn loot_apply_to_npc_places_items_starting_at_carried_slot_thirty() {
    // C `create.c:1121-1125` calling `loot_apply_to_npc` right after
    // character creation, which rolls into `ch[cn].item[30..
    // INVENTORYSIZE]` via `place_in_npc` (`loot.c:665-675`).
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    load_item(&mut loader, "bronzechip");
    load_item(&mut loader, "demon_helmet1");
    world.loot_registry.load_str(
        r#"[
            {
                "id": "demon_low_equipment",
                "rolls": 1,
                "entries": [{"weight": 1, "item": "demon_helmet1"}]
            },
            {
                "id": "pent_demon_low_spawn",
                "rolls": 2,
                "entries": [
                    {"weight": 1, "table": "demon_low_equipment"},
                    {"weight": 1, "item": "bronzechip"}
                ]
            }
        ]"#,
    );

    let mut npc = character(1);
    // Occupy every worn/spell slot (0..30) so a bug that started placement
    // there instead of slot 30 would be immediately caught by an empty
    // sink, not silently succeed in the wrong range.
    for slot in 0..30 {
        npc.inventory[slot] = Some(ItemId(9000 + slot as u32));
    }
    world.spawn_character(npc, 10, 10);

    world.legacy_random_seed = 0;
    let added = world.loot_apply_to_npc(&mut loader, CharacterId(1), "pent_demon_low_spawn");
    assert_eq!(added, 2, "two rolls, both entries resolve to a placed item");

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    // Slots 0..30 are untouched (still the pre-seeded placeholder ids);
    // the two new items land at the first two free carried slots (30, 31).
    for slot in 0..30 {
        assert_eq!(npc.inventory[slot], Some(ItemId(9000 + slot as u32)));
    }
    let carried_names: Vec<&str> = npc.inventory[30..32]
        .iter()
        .flatten()
        .map(|id| world.items.get(id).unwrap().name.as_str())
        .collect();
    assert_eq!(carried_names.len(), 2);
    assert!(carried_names.iter().all(|name| world
        .items
        .values()
        .any(|item| item.name == *name && item.carried_by == Some(CharacterId(1)))));
}

#[test]
fn loot_apply_to_npc_is_a_no_op_when_every_carried_slot_is_full() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    load_item(&mut loader, "bronzechip");
    world.loot_registry.load_str(
        r#"{"id": "full_sink", "rolls": 1, "entries": [{"weight": 1, "item": "bronzechip"}]}"#,
    );

    let mut npc = character(1);
    for slot in 0..INVENTORY_SIZE {
        npc.inventory[slot] = Some(ItemId(9000 + slot as u32));
    }
    world.spawn_character(npc, 10, 10);

    let added = world.loot_apply_to_npc(&mut loader, CharacterId(1), "full_sink");
    assert_eq!(added, 0, "no free carried slot => place_in_npc fails");
}
