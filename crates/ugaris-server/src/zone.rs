use super::*;

#[derive(Debug, Clone)]
pub(crate) struct ZoneLoadSummary {
    pub(crate) root: PathBuf,
    pub(crate) map_file: PathBuf,
    pub(crate) item_templates: usize,
    pub(crate) character_templates: usize,
    pub(crate) skipped_template_files: usize,
    pub(crate) placed_items: usize,
    pub(crate) placed_characters: usize,
    pub(crate) ground_tiles: usize,
    pub(crate) blocked_tiles: usize,
    pub(crate) scheduled_light_timers: usize,
}

pub(crate) fn resolve_zone_root(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured {
        return path.exists().then(|| path.to_path_buf());
    }

    [
        PathBuf::from("ugaris_data/zones"),
        PathBuf::from("../ugaris_data/zones"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

pub(crate) fn load_area_zone(
    world: &mut World,
    loader: &mut ZoneLoader,
    zone_root: &Path,
    area_id: u16,
) -> anyhow::Result<ZoneLoadSummary> {
    let area_dir = zone_root.join(area_id.to_string());
    let map_file = first_file_with_extension(&area_dir, "map")?
        .ok_or_else(|| anyhow::anyhow!("no .map file found in {}", area_dir.display()))?;
    let map_text = std::fs::read_to_string(&map_file)?;
    let skipped_template_files = load_zone_templates(loader, zone_root, &area_dir)?;
    loader.apply_map_str(world, &map_text)?;
    let scheduled_light_timers = world.schedule_existing_light_timers();

    let (ground_tiles, blocked_tiles) = map_tile_counts(world);
    Ok(ZoneLoadSummary {
        root: zone_root.to_path_buf(),
        map_file,
        item_templates: loader.item_templates.len(),
        character_templates: loader.character_templates.len(),
        skipped_template_files,
        placed_items: world.items.len(),
        placed_characters: world.characters.len(),
        ground_tiles,
        blocked_tiles,
        scheduled_light_timers,
    })
}

pub(crate) fn next_available_character_id(world: &World) -> u32 {
    world
        .characters
        .keys()
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1)
}

pub(crate) fn load_zone_templates(
    loader: &mut ZoneLoader,
    zone_root: &Path,
    area_dir: &Path,
) -> anyhow::Result<usize> {
    let mut skipped = 0;
    for dir in [zone_root.join("generic"), area_dir.to_path_buf()] {
        skipped += load_zone_template_dir(loader, &dir, "itm")?;
        skipped += load_zone_template_dir(loader, &dir, "chr")?;
    }
    Ok(skipped)
}

pub(crate) fn load_zone_template_dir(
    loader: &mut ZoneLoader,
    dir: &Path,
    extension: &str,
) -> anyhow::Result<usize> {
    let mut skipped = 0;
    for file in files_with_extension(dir, extension)? {
        let text = std::fs::read_to_string(&file)?;
        let result = if extension.eq_ignore_ascii_case("itm") {
            loader.load_item_templates_str(&text)
        } else {
            loader.load_character_templates_str(&text)
        };
        if result.is_err() {
            warn!(file = %file.display(), error = %result.unwrap_err(), "skipping unsupported zone template file");
            skipped += 1;
        }
    }
    Ok(skipped)
}

pub(crate) fn first_file_with_extension(
    dir: &Path,
    extension: &str,
) -> anyhow::Result<Option<PathBuf>> {
    Ok(files_with_extension(dir, extension)?.into_iter().next())
}

pub(crate) fn files_with_extension(dir: &Path, extension: &str) -> anyhow::Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub(crate) fn map_tile_counts(world: &World) -> (usize, usize) {
    let mut ground_tiles = 0;
    let mut blocked_tiles = 0;
    for y in 0..world.map.height() {
        for x in 0..world.map.width() {
            let Some(tile) = world.map.tile(x, y) else {
                continue;
            };
            if tile.ground_sprite != 0 || tile.foreground_sprite != 0 {
                ground_tiles += 1;
            }
            if tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            {
                blocked_tiles += 1;
            }
        }
    }
    (ground_tiles, blocked_tiles)
}

pub(crate) fn choose_spawn_tile(world: &World) -> (usize, usize) {
    if is_spawn_tile_open(world, LOGIN_SPAWN_X, LOGIN_SPAWN_Y) {
        return (LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
    }

    for radius in 1..80 {
        let min_x = LOGIN_SPAWN_X.saturating_sub(radius);
        let max_x = (LOGIN_SPAWN_X + radius).min(world.map.width().saturating_sub(2));
        let min_y = LOGIN_SPAWN_Y.saturating_sub(radius);
        let max_y = (LOGIN_SPAWN_Y + radius).min(world.map.height().saturating_sub(2));
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if is_spawn_tile_open(world, x, y) {
                    return (x, y);
                }
            }
        }
    }

    for y in 1..world.map.height().saturating_sub(1) {
        for x in 1..world.map.width().saturating_sub(1) {
            if is_spawn_tile_open(world, x, y) {
                return (x, y);
            }
        }
    }

    (LOGIN_SPAWN_X, LOGIN_SPAWN_Y)
}

pub(crate) fn is_spawn_tile_open(world: &World, x: usize, y: usize) -> bool {
    world.map.legacy_inner_bounds(x, y)
        && world.map.tile(x, y).is_some_and(|tile| {
            tile.character == 0
                && !tile
                    .flags
                    .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        })
}
