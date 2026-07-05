//! Data-driven, hot-reloadable monster loot tables.
//!
//! Ports `src/system/loot/loot.c` (and the schema documented in
//! `src/system/loot/loot.h`): JSON loot tables under `ugaris_data/loot/`
//! (recursively scanned by the server - see `ugaris-server`'s zone/startup
//! wiring), evaluated either at spawn time (`loot_table=`, into a fresh
//! NPC's own inventory - [`World::loot_apply_to_npc`], called right after
//! character creation in `ZoneLoader::apply_map_directives` and every
//! `ugaris-server` NPC spawn/respawn site) or at death time
//! (`loot_table_death=`, `apply_death_loot_for_template` -
//! [`World::loot_apply_to_container`]).
//!
//! File I/O and directory scanning live in `ugaris-server` (matching the
//! `ZoneLoader::load_*_str` split already used for zone/character
//! templates); [`LootRegistry::load_str`] only ever sees already-read JSON
//! text. The `event_drop_rate` (and any future) modifier scalars keep
//! living in `GameSettings::loot_modifiers` (`loot_set_modifier`/
//! `loot_get_modifier`, already ported) rather than duplicating that
//! store here; only the per-counter pity state (`loot_pity_get`/
//! `loot_pity_set`, no equivalent yet) is new, kept on [`LootRegistry`]
//! since it is loot-system-private state with no other consumer.

use serde_json::Value;

use super::*;

/// C `LOOT_MAX_GROUPS_PER_TABLE` (`loot.c:30`).
const LOOT_MAX_GROUPS_PER_TABLE: usize = 8;
/// C `LOOT_MAX_MODS_PER_GRP` (`loot.h:83`).
const LOOT_MAX_MODS_PER_GROUP: usize = 4;
/// C `LOOT_MAX_DEPTH` (`loot.c:34`): sub-table recursion cap.
const LOOT_MAX_DEPTH: u32 = 8;

/// C `enum LootMode` (`loot.c:39-42`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootMode {
    Spawn,
    Death,
}

/// C `enum LootEntryKind` (`loot.c:44-48`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootEntryKind {
    Item,
    Table,
    Nothing,
}

/// C `enum LootCondType` (`loot.c:50-59`), with the associated `arg1`/
/// `arg2` folded directly into each variant instead of the C struct's
/// separate `int arg1; int arg2;` fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootCondition {
    None,
    QuestOpen(u32),
    QuestDone(u32),
    QuestNotDone(u32),
    QuestCountLt(u32, i32),
    QuestCountGe(u32, i32),
    KillerLevelGe(i32),
    KillerLevelLt(i32),
}

/// C `struct LootEntry` (`loot.c:67-74`); `resolved` isn't ported (Rust
/// re-resolves sub-table references by id at roll time instead of caching
/// a pointer, see [`World::roll_loot_table_into_container`]).
#[derive(Debug, Clone)]
pub struct LootEntry {
    pub kind: LootEntryKind,
    pub weight: i32,
    pub count_min: i32,
    pub count_max: i32,
    /// Item template name (`LE_ITEM`) or sub-table id (`LE_TABLE`); empty
    /// for `LE_NOTHING`.
    pub reference: String,
}

/// C `struct LootPity` (`loot.c:76-79`).
#[derive(Debug, Clone, Default)]
pub struct LootPity {
    /// Empty means "no pity gate" (C: `counter[0]` truthiness check).
    pub counter: String,
    pub threshold: i32,
}

/// C `struct LootGroup` (`loot.c:81-90`).
#[derive(Debug, Clone)]
pub struct LootGroup {
    pub rolls: i32,
    pub pity: LootPity,
    pub modifiers: Vec<String>,
    pub condition: LootCondition,
    pub entries: Vec<LootEntry>,
    pub total_weight: i32,
}

/// C `struct LootTable` (`loot.c:92-97`).
#[derive(Debug, Clone)]
pub struct LootTable {
    pub id: String,
    pub mode: LootMode,
    pub groups: Vec<LootGroup>,
}

/// Result of [`LootRegistry::load_str`]: how many tables were added, plus
/// every non-fatal parse warning collected along the way (C's various
/// `elog(...)` calls in `parse_entry`/`parse_condition`/`parse_group`/
/// `parse_table`/`parse_document`) so the server-side file scanner can log
/// them with the source file path attached.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LootLoadReport {
    pub tables_added: usize,
    pub warnings: Vec<String>,
}

/// C's file-scoped `tables[]`/`n_tables` plus `pity_counters[]`/
/// `n_pity_counters` (`loot.c:119-126`). Modifiers (`modifiers[]`/
/// `n_modifiers`) live on [`GameSettings::loot_modifiers`] instead - see
/// the module doc comment.
#[derive(Debug, Default)]
pub struct LootRegistry {
    pub tables: Vec<LootTable>,
    pity_counters: HashMap<String, i32>,
}

impl LootRegistry {
    /// C `loot_find` (`loot.c:196-203`): first match wins on duplicate
    /// ids, matching the linear `for` scan.
    pub fn find(&self, id: &str) -> Option<&LootTable> {
        self.tables.iter().find(|table| table.id == id)
    }

    /// C `loot_table_count` (`loot.c:205`).
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// C `loot_pity_get` (`loot.c:184-187`).
    pub fn pity_get(&self, name: &str) -> i32 {
        self.pity_counters.get(name).copied().unwrap_or(0)
    }

    /// C `loot_pity_set` (`loot.c:189-192`).
    pub fn pity_set(&mut self, name: &str, value: i32) {
        self.pity_counters.insert(name.to_string(), value);
    }

    /// C `clear_tables` (`loot.c:545-552`), the half of `loot_reload`
    /// (`loot.c:600-607`) this crate can perform - preserves pity counters
    /// and (via `GameSettings`) modifiers, exactly like C. Directory
    /// re-scanning is the server's job; the caller follows this with
    /// however many [`Self::load_str`] calls the reload needs.
    pub fn clear_tables(&mut self) {
        self.tables.clear();
    }

    /// C `load_file`+`parse_document` (`loot.c:445-506`) for
    /// already-read JSON text (`ugaris-server` owns `scan_dir`/`fopen`).
    /// Parse failures for individual tables/groups/entries are tolerated
    /// and reported as warnings (matching every C `elog` call skipping
    /// just the offending piece); only a JSON syntax error fails the
    /// whole document, mirroring C's own `cJSON_Parse` failure path.
    pub fn load_str(&mut self, text: &str) -> LootLoadReport {
        let mut warnings = Vec::new();
        let root: Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(err) => {
                warnings.push(format!("loot: parse error: {err}"));
                return LootLoadReport {
                    tables_added: 0,
                    warnings,
                };
            }
        };
        let tables = parse_document(&root, &mut warnings);
        let tables_added = tables.len();
        self.tables.extend(tables);
        LootLoadReport {
            tables_added,
            warnings,
        }
    }
}

fn json_i32(obj: &Value, key: &str, default: i32) -> i32 {
    obj.get(key)
        .and_then(Value::as_f64)
        .map_or(default, |n| n as i32)
}

fn json_str<'a>(obj: &'a Value, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(Value::as_str)
}

/// C `parse_entry` (`loot.c:227-269`).
fn parse_entry(obj: &Value) -> Option<LootEntry> {
    let mut weight = json_i32(obj, "weight", 1);
    if weight < 0 {
        weight = 0;
    }

    let mut count_min = 1;
    let mut count_max = 1;
    match obj.get("count") {
        Some(Value::Number(n)) => {
            let v = n.as_f64().unwrap_or(1.0) as i32;
            count_min = v;
            count_max = v;
        }
        Some(Value::Array(arr)) if arr.len() >= 2 => {
            if let Some(lo) = arr[0].as_f64() {
                count_min = lo as i32;
            }
            if let Some(hi) = arr[1].as_f64() {
                count_max = hi as i32;
            }
        }
        _ => {}
    }
    if count_min < 1 {
        count_min = 1;
    }
    if count_max < count_min {
        count_max = count_min;
    }

    if let Some(item) = json_str(obj, "item") {
        Some(LootEntry {
            kind: LootEntryKind::Item,
            weight,
            count_min,
            count_max,
            reference: item.to_string(),
        })
    } else if let Some(table) = json_str(obj, "table") {
        Some(LootEntry {
            kind: LootEntryKind::Table,
            weight,
            count_min,
            count_max,
            reference: table.to_string(),
        })
    } else if obj.get("nothing").and_then(Value::as_bool) == Some(true) {
        Some(LootEntry {
            kind: LootEntryKind::Nothing,
            weight,
            count_min,
            count_max,
            reference: String::new(),
        })
    } else {
        None
    }
}

/// C `parse_condition` (`loot.c:274-322`). Returns `None` only when `obj`
/// is a JSON object but none of the recognized keys matched (C's
/// "unknown or malformed condition" `elog` + `return 0` path); the caller
/// distinguishes that from "no condition object at all" to decide whether
/// to emit a warning.
fn try_parse_condition(obj: &Value) -> Option<LootCondition> {
    if let Some(n) = obj.get("quest_open").and_then(Value::as_u64) {
        return Some(LootCondition::QuestOpen(n as u32));
    }
    if let Some(n) = obj.get("quest_done").and_then(Value::as_u64) {
        return Some(LootCondition::QuestDone(n as u32));
    }
    if let Some(n) = obj.get("quest_not_done").and_then(Value::as_u64) {
        return Some(LootCondition::QuestNotDone(n as u32));
    }
    if let Some(arr) = obj.get("quest_count_lt").and_then(Value::as_array) {
        if arr.len() >= 2 {
            if let (Some(q), Some(t)) = (arr[0].as_u64(), arr[1].as_i64()) {
                return Some(LootCondition::QuestCountLt(q as u32, t as i32));
            }
        }
    }
    if let Some(arr) = obj.get("quest_count_ge").and_then(Value::as_array) {
        if arr.len() >= 2 {
            if let (Some(q), Some(t)) = (arr[0].as_u64(), arr[1].as_i64()) {
                return Some(LootCondition::QuestCountGe(q as u32, t as i32));
            }
        }
    }
    if let Some(n) = obj.get("killer_level_ge").and_then(Value::as_i64) {
        return Some(LootCondition::KillerLevelGe(n as i32));
    }
    if let Some(n) = obj.get("killer_level_lt").and_then(Value::as_i64) {
        return Some(LootCondition::KillerLevelLt(n as i32));
    }
    None
}

/// C `parse_group` (`loot.c:328-392`). `src` is either the `groups[]`
/// array element or (for the shorthand single-implicit-group schema) the
/// table object itself, matching C's `obj ? obj : fallback_table_obj`.
fn parse_group(src: &Value, table_id: &str, warnings: &mut Vec<String>) -> Option<LootGroup> {
    let mut rolls = json_i32(src, "rolls", 1);
    if rolls < 0 {
        rolls = 0;
    }

    let mut pity = LootPity::default();
    if let Some(p) = src.get("pity").filter(|v| v.is_object()) {
        if let Some(counter) = json_str(p, "counter") {
            pity.counter = counter.to_string();
        }
        pity.threshold = json_i32(p, "threshold", 0).max(0);
    }

    let mut modifiers = Vec::new();
    if let Some(Value::Array(mods)) = src.get("modifiers") {
        for m in mods.iter().take(LOOT_MAX_MODS_PER_GROUP) {
            if let Some(s) = m.as_str() {
                modifiers.push(s.to_string());
            }
        }
    }

    let condition = match src.get("condition") {
        None => LootCondition::None,
        Some(cond_obj) if !cond_obj.is_object() => LootCondition::None,
        Some(cond_obj) => match try_parse_condition(cond_obj) {
            Some(cond) => cond,
            None => {
                warnings.push(format!(
                    "loot \"{table_id}\": unknown or malformed condition"
                ));
                LootCondition::None
            }
        },
    };

    let Some(Value::Array(entries_json)) = src.get("entries") else {
        warnings.push(format!(
            "loot \"{table_id}\": group missing \"entries\" array"
        ));
        return None;
    };
    if entries_json.is_empty() {
        warnings.push(format!("loot \"{table_id}\": group has empty entries"));
        return None;
    }

    let mut entries = Vec::new();
    let mut total_weight: i32 = 0;
    for e in entries_json {
        if !e.is_object() {
            continue;
        }
        match parse_entry(e) {
            Some(entry) => {
                total_weight = total_weight.saturating_add(entry.weight);
                entries.push(entry);
            }
            None => warnings.push(format!(
                "loot \"{table_id}\": entry has no item/table/nothing"
            )),
        }
    }
    if entries.is_empty() || total_weight <= 0 {
        warnings.push(format!(
            "loot \"{table_id}\": group has no usable entries / zero total weight"
        ));
        return None;
    }

    Some(LootGroup {
        rolls,
        pity,
        modifiers,
        condition,
        entries,
        total_weight,
    })
}

/// C `parse_mode` (`loot.c:394-398`).
fn parse_mode(obj: &Value) -> LootMode {
    match json_str(obj, "mode") {
        Some(s) if s.eq_ignore_ascii_case("death") => LootMode::Death,
        _ => LootMode::Spawn,
    }
}

/// C `parse_table` (`loot.c:400-443`).
fn parse_table(obj: &Value, warnings: &mut Vec<String>) -> Option<LootTable> {
    let Some(id) = json_str(obj, "id").map(str::to_string) else {
        warnings.push("loot: table missing \"id\"".to_string());
        return None;
    };
    let mode = parse_mode(obj);

    let mut groups = Vec::new();
    if let Some(Value::Array(groups_json)) = obj.get("groups") {
        for g in groups_json.iter().take(LOOT_MAX_GROUPS_PER_TABLE) {
            if g.is_object() {
                if let Some(group) = parse_group(g, &id, warnings) {
                    groups.push(group);
                }
            }
        }
    } else if let Some(group) = parse_group(obj, &id, warnings) {
        groups.push(group);
    }

    if groups.is_empty() {
        warnings.push(format!("loot \"{id}\": no usable groups"));
        return None;
    }

    Some(LootTable { id, mode, groups })
}

/// C `parse_document` (`loot.c:445-465`).
fn parse_document(root: &Value, warnings: &mut Vec<String>) -> Vec<LootTable> {
    match root {
        Value::Array(items) => items
            .iter()
            .filter_map(|obj| {
                obj.is_object()
                    .then(|| parse_table(obj, warnings))
                    .flatten()
            })
            .collect(),
        Value::Object(_) => parse_table(root, warnings).into_iter().collect(),
        _ => {
            warnings.push("loot: top-level JSON must be object or array".to_string());
            Vec::new()
        }
    }
}

/// Killer-side quest state needed by [`LootCondition::QuestOpen`]/
/// `QuestDone`/`QuestNotDone`/`QuestCountLt`/`QuestCountGe`
/// (`eval_condition`, `loot.c:615-642`). Implemented directly for
/// [`crate::quest::QuestLog`] below; `ugaris-server` passes
/// `&player.quest_log` through [`LootKiller`] since `QuestLog` lives on
/// the server-owned `PlayerRuntime`, not on the core `Character`.
pub trait LootQuestContext {
    fn quest_is_done(&self, quest: u32) -> bool;
    fn quest_count(&self, quest: u32) -> u8;
}

impl LootQuestContext for crate::quest::QuestLog {
    fn quest_is_done(&self, quest: u32) -> bool {
        self.is_done(quest as usize)
    }
    fn quest_count(&self, quest: u32) -> u8 {
        self.count(quest as usize)
    }
}

/// C `valid_killer` (`loot.c:611-613`) bundled with the per-kill context
/// `eval_condition` needs: `killer_cn`'s level (read straight off
/// `World::characters` by the caller - the core `Character`, unlike quest
/// state, needs no server bridge) and quest log.
pub struct LootKiller<'a> {
    pub character_id: CharacterId,
    pub level: u32,
    pub quest: &'a dyn LootQuestContext,
}

/// C `struct DropSink` (`loot.c:111-115`): "at most one of cn / container
/// is set" - modeled here as an enum instead of two nullable fields.
/// `killer_cn` (death-mode only) is threaded separately as `Option<&
/// LootKiller>` rather than living on the sink, since it is only ever read
/// by [`eval_loot_condition`], not by placement.
#[derive(Debug, Clone, Copy)]
enum LootSink {
    /// Death-mode: corpse/body container (`place_in_container`).
    Container(ItemId),
    /// Spawn-mode: the newly-created NPC's own inventory
    /// (`place_in_npc`).
    Npc(CharacterId),
}

impl World {
    /// C `loot_apply_to_container` (`loot.c:777-792`): returns -1 for an
    /// unknown table id or a mode mismatch (mirroring `elog` + early
    /// return), otherwise the number of items placed.
    pub fn loot_apply_to_container(
        &mut self,
        loader: &mut ZoneLoader,
        container_id: ItemId,
        killer: Option<&LootKiller<'_>>,
        table_id: &str,
    ) -> i32 {
        match self.loot_registry.find(table_id) {
            Some(table) if table.mode == LootMode::Death => {}
            _ => return -1,
        }
        self.roll_loot_table(
            loader,
            LootSink::Container(container_id),
            killer,
            table_id,
            0,
        )
    }

    /// C `loot_apply_to_npc` (`loot.c:760-774`): returns -1 for an unknown
    /// table id or a mode mismatch, otherwise the number of items placed
    /// into the NPC's own inventory. Called right after character creation
    /// (C `create.c:1121-1125`'s `if (ch_temp[ctmp].loot_table[0])
    /// loot_apply_to_npc(n, ...)`) - `sink.killer_cn` is always 0 in C's
    /// spawn-mode call site, so every killer-dependent condition fails the
    /// same way `killer: None` already makes [`eval_loot_condition`] fail.
    pub fn loot_apply_to_npc(
        &mut self,
        loader: &mut ZoneLoader,
        character_id: CharacterId,
        table_id: &str,
    ) -> i32 {
        match self.loot_registry.find(table_id) {
            Some(table) if table.mode == LootMode::Spawn => {}
            _ => return -1,
        }
        self.roll_loot_table(loader, LootSink::Npc(character_id), None, table_id, 0)
    }

    /// C `roll_table` (`loot.c:747-758`). Clones the resolved table's
    /// groups out of `self.loot_registry` up front (rather than caching a
    /// resolved pointer like C's `LootEntry::resolved`) so the roll loop
    /// below can freely borrow `self` mutably for RNG draws, pity-counter
    /// updates, and item creation - loot tables are tiny (a handful of
    /// groups/entries each), so this is not a hot path worth the
    /// self-referential-borrow complexity C's raw pointer avoids for free.
    fn roll_loot_table(
        &mut self,
        loader: &mut ZoneLoader,
        sink: LootSink,
        killer: Option<&LootKiller<'_>>,
        table_id: &str,
        depth: u32,
    ) -> i32 {
        if depth >= LOOT_MAX_DEPTH {
            return 0;
        }
        let Some(groups) = self.loot_registry.find(table_id).map(|t| t.groups.clone()) else {
            return 0;
        };
        let mut added = 0;
        for group in &groups {
            added += self.roll_loot_group(loader, sink, killer, group, depth);
        }
        added
    }

    /// C `roll_group` (`loot.c:699-745`).
    fn roll_loot_group(
        &mut self,
        loader: &mut ZoneLoader,
        sink: LootSink,
        killer: Option<&LootKiller<'_>>,
        group: &LootGroup,
        depth: u32,
    ) -> i32 {
        if !eval_loot_condition(&group.condition, killer) {
            return 0;
        }

        let modifier = self.compose_loot_modifier(&group.modifiers);

        if !group.pity.counter.is_empty() {
            let mut value = self.loot_registry.pity_get(&group.pity.counter);
            value += 1;
            let mut eff_threshold = group.pity.threshold;
            if modifier > 0.0 {
                eff_threshold = (f64::from(group.pity.threshold) / modifier) as i32;
                if eff_threshold < 1 {
                    eff_threshold = 1;
                }
            }
            if value <= eff_threshold {
                self.loot_registry.pity_set(&group.pity.counter, value);
                return 0;
            }
            self.loot_registry.pity_set(&group.pity.counter, 0);
        }

        let mut rolls = group.rolls;
        if modifier != 1.0 {
            rolls = (f64::from(rolls) * modifier).ceil() as i32;
        }

        let mut added = 0;
        for _ in 0..rolls.max(0) {
            let Some(entry) = self.pick_weighted_loot_entry(group) else {
                break;
            };
            match entry.kind {
                LootEntryKind::Nothing => {}
                LootEntryKind::Item => {
                    let mut count = entry.count_min;
                    if entry.count_max > entry.count_min {
                        count += legacy_random_below_from_seed(
                            &mut self.legacy_random_seed,
                            (entry.count_max - entry.count_min + 1) as u32,
                        ) as i32;
                    }
                    for _ in 0..count {
                        if self.place_loot_item(loader, sink, &entry.reference) {
                            added += 1;
                        }
                    }
                }
                LootEntryKind::Table => {
                    added +=
                        self.roll_loot_table(loader, sink, killer, &entry.reference, depth + 1);
                }
            }
        }
        added
    }

    /// C `compose_modifier` (`loot.c:646-652`).
    fn compose_loot_modifier(&self, modifiers: &[String]) -> f64 {
        let mut value = 1.0;
        for name in modifiers {
            value *= self.settings.get_loot_modifier(name);
        }
        value.max(0.0)
    }

    /// C `pick_weighted` (`loot.c:654-663`).
    fn pick_weighted_loot_entry(&mut self, group: &LootGroup) -> Option<LootEntry> {
        if group.total_weight <= 0 {
            return None;
        }
        let roll =
            legacy_random_below_from_seed(&mut self.legacy_random_seed, group.total_weight as u32)
                as i32;
        let mut acc = 0;
        for entry in &group.entries {
            acc += entry.weight;
            if roll < acc {
                return Some(entry.clone());
            }
        }
        group.entries.last().cloned()
    }

    /// C `place_item` (`loot.c:684-695`), dispatching to `place_in_container`
    /// (`loot.c:678-680`) or `place_in_npc` (`loot.c:665-675`) depending on
    /// which half of the `DropSink` is set. Newly created items are
    /// inserted directly (bypassing `World::add_item`'s map-light
    /// bookkeeping, which doesn't apply to contained/carried items at
    /// `x=y=0`) - the same pattern `fill_body_container`'s money item
    /// already uses. Returns `false` (item creation failed, or the sink is
    /// full/missing) without inserting anything, matching C's `free_item`
    /// fallback for a `place_item` that returns 0.
    fn place_loot_item(&mut self, loader: &mut ZoneLoader, sink: LootSink, item_key: &str) -> bool {
        let carried_by = match sink {
            LootSink::Npc(character_id) => Some(character_id),
            LootSink::Container(_) => None,
        };
        let Ok(mut item) = loader.instantiate_item_template(item_key, carried_by) else {
            return false;
        };
        match sink {
            LootSink::Container(container_id) => {
                item.contained_in = Some(container_id);
                self.items.insert(item.id, item);
                true
            }
            LootSink::Npc(character_id) => {
                // C `place_in_npc` (`loot.c:665-675`): first free slot in
                // `ch[cn].item[30..INVENTORYSIZE]` (slots 0-11 worn,
                // 12-29 spells - see `server.h:447`); no-op (sink full)
                // when every carried slot is occupied.
                let Some(character) = self.characters.get_mut(&character_id) else {
                    return false;
                };
                let Some(offset) = character.inventory[30..]
                    .iter()
                    .position(|slot| slot.is_none())
                else {
                    return false;
                };
                character.inventory[30 + offset] = Some(item.id);
                self.items.insert(item.id, item);
                true
            }
        }
    }
}

/// C `eval_condition` (`loot.c:615-642`): killer-dependent conditions
/// (everything but `LCOND_NONE`) fail outright when there is no killer -
/// C's `valid_killer(killer_cn)` gate, folded here into `killer: None`.
fn eval_loot_condition(condition: &LootCondition, killer: Option<&LootKiller<'_>>) -> bool {
    if matches!(condition, LootCondition::None) {
        return true;
    }
    let Some(killer) = killer else {
        return false;
    };
    match *condition {
        LootCondition::None => true,
        // C's `LCOND_QUEST_OPEN` comment (`loot.c:619-625`) is explicit
        // that this is `!questlog_isdone` used as a permissive proxy for
        // "open", identical to `LCOND_QUEST_NOT_DONE` - not the questlog's
        // own (semantically different) `is_open` predicate.
        LootCondition::QuestOpen(q) => !killer.quest.quest_is_done(q),
        LootCondition::QuestDone(q) => killer.quest.quest_is_done(q),
        LootCondition::QuestNotDone(q) => !killer.quest.quest_is_done(q),
        LootCondition::QuestCountLt(q, threshold) => {
            i32::from(killer.quest.quest_count(q)) < threshold
        }
        LootCondition::QuestCountGe(q, threshold) => {
            i32::from(killer.quest.quest_count(q)) >= threshold
        }
        LootCondition::KillerLevelGe(level) => killer.level as i32 >= level,
        LootCondition::KillerLevelLt(level) => (killer.level as i32) < level,
    }
}
