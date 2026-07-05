//! Server-side wiring for `ugaris-core`'s data-driven loot tables
//! (`ugaris_core::world::LootRegistry`/`World::loot_apply_to_container`):
//! recursive directory scanning of `ugaris_data/loot/**/*.json` (C
//! `init_loot`/`scan_dir`, `src/system/loot/loot.c:513-598`) and draining
//! `World::drain_pending_death_loot_rolls` (C `apply_death_loot_for_
//! template`, queued by `World::die_character` as a
//! [`PendingDeathLootRoll`] since the killer's quest log lives on the
//! server-owned `PlayerRuntime`, not on the core `Character`).

use super::*;

/// C `LOOT_DATA_DIR "ugaris_data/loot"` (`loot.c:35`), with the same
/// `../` fallback `resolve_zone_root` uses for the equivalent zone data
/// root (this binary's working directory varies between `cargo run` from
/// the workspace root and a packaged deployment one level down).
pub(crate) fn resolve_loot_root(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured {
        return path.exists().then(|| path.to_path_buf());
    }
    [
        PathBuf::from("ugaris_data/loot"),
        PathBuf::from("../ugaris_data/loot"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LootScanSummary {
    pub(crate) files_scanned: usize,
    pub(crate) tables_added: usize,
    pub(crate) warnings: usize,
}

/// C `init_loot`/`loot_reload`'s `scan_dir` half (`loot.c:513-532`,
/// `591-607`): recursively load every `.json` file under `root` into
/// `registry`. Per-file parse warnings are logged here (with the file
/// path attached, matching every C `elog(...)` call's implicit context)
/// rather than propagated, mirroring `load_zone_template_dir`'s
/// warn-and-skip handling of unparsable zone template files.
pub(crate) fn load_loot_tables(registry: &mut LootRegistry, root: &Path) -> LootScanSummary {
    let mut summary = LootScanSummary::default();
    scan_loot_dir(registry, root, &mut summary);
    summary
}

fn scan_loot_dir(registry: &mut LootRegistry, dir: &Path, summary: &mut LootScanSummary) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            scan_loot_dir(registry, &path, summary);
            continue;
        }
        let is_json = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
        if !is_json {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                summary.files_scanned += 1;
                let report = registry.load_str(&text);
                summary.tables_added += report.tables_added;
                summary.warnings += report.warnings.len();
                for warning in report.warnings {
                    warn!(file = %path.display(), warning, "loot table parse warning");
                }
            }
            Err(err) => {
                warn!(file = %path.display(), error = %err, "failed to read loot table file");
            }
        }
    }
}

/// C `die_char`'s `apply_death_loot_for_template(ct, co, tmp)` call
/// (`death.c:741`), resolved server-side once per drained
/// [`PendingDeathLootRoll`]: looks up the dead character's template for
/// its `loot_table_death` id (C `ch_temp[tmp].loot_table_death[0]` early
/// return when unset), builds the killer context from the live `World`
/// character (level) and `PlayerRuntime` (quest log) - `None` whenever
/// either is unavailable, mirroring C `valid_killer`'s `killer_cn > 0 &&
/// CF_PLAYER` gate (a killer without a live session can't have its quest
/// conditions evaluated, so every killer-dependent condition fails, same
/// as a passed-in killer_cn of 0) - and rolls the table into the corpse
/// container. Returns the total items placed across every drained roll,
/// for caller logging.
pub(crate) fn apply_pending_death_loot_rolls(
    world: &mut World,
    runtime: &ServerRuntime,
    zone_loader: &mut ZoneLoader,
    rolls: Vec<PendingDeathLootRoll>,
) -> i32 {
    let mut total_added = 0;
    for roll in rolls {
        let Some(table_id) = zone_loader
            .character_templates
            .get(&roll.template_key)
            .map(|template| template.loot_table_death.clone())
        else {
            continue;
        };
        if table_id.is_empty() {
            continue;
        }

        let killer_facts = roll
            .killer_id
            .and_then(|id| world.characters.get(&id))
            .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
            .map(|character| character.level);
        let killer =
            match (roll.killer_id, killer_facts) {
                (Some(character_id), Some(level)) => runtime
                    .player_for_character(character_id)
                    .map(|player| LootKiller {
                        character_id,
                        level,
                        quest: &player.quest_log,
                    }),
                _ => None,
            };

        let added = world.loot_apply_to_container(
            zone_loader,
            roll.container_id,
            killer.as_ref(),
            &table_id,
        );
        if added > 0 {
            total_added += added;
        }
    }
    total_added
}
