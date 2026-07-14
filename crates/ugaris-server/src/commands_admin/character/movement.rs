use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_movement(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    character_id: CharacterId,
    area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    // C `/goto` (`src/system/command.c:8453-8567`), gated on
    // `is_lqmaster(cn)` (`command.c:3331-3344`: `CF_GOD`, `CF_EVENTMASTER`,
    // or `CF_LQMASTER` while `areaID == 20`). See [`resolve_goto_jump_args`]
    // for the shared argument-parsing port (numeric `<x> <y> [area]
    // [mirror]`, `n`/`s`/`w`/`e` relative shorthand, `gl[]` shortcut name,
    // or online character name, in that priority order).
    if lower.len() >= 3 && "goto".starts_with(lower) {
        let Some(character) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        let flags = character.flags;
        let (cx, cy) = (character.x, character.y);
        let is_lqmaster = flags.contains(CharacterFlags::GOD)
            || flags.contains(CharacterFlags::EVENTMASTER)
            || (area_id == 20 && flags.contains(CharacterFlags::LQMASTER));
        if !is_lqmaster {
            return ControlFlow::Break(None);
        }
        let is_god = flags.contains(CharacterFlags::GOD);
        let resolved = resolve_goto_jump_args(world, cx, cy, rest);
        let GotoJumpTarget { x, y, mut a, m } = resolved;
        if (1..27).contains(&m) && a == 0 {
            a = area_id as i32;
        }
        if a == area_id as i32 && m == 0 {
            a = 0;
        }
        if !is_god {
            a = 0;
        }
        return ControlFlow::Break(Some(finish_goto_jump(
            world,
            character_id,
            x,
            y,
            a,
            m,
            "goto",
        )));
    }

    // C `/jump` (`command.c:8570-8626`), gated on `CF_STAFF | CF_GOD`. Only
    // resolves a `gl[]` shortcut name (no numeric x/y form, no player-name
    // lookup), with an optional leading `<mirror>` digit token consumed
    // first, and refuses while busy (`ch[cn].action != AC_IDLE`) or within
    // 3 seconds of the last regen tick ("Pant, pant. Too tired."). Unlike
    // `/goto`, cross-area is *not* restricted to `CF_GOD` here (copied
    // as-is from C, which has no such check on this path).
    if lower == "jump" {
        let Some(character) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        let flags = character.flags;
        let (action, regen_ticker) = (character.action, character.regen_ticker);
        if !flags.intersects(CharacterFlags::STAFF | CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        if action != 0
            || world.tick.0.saturating_sub(u64::from(regen_ticker)) < TICKS_PER_SECOND * 3
        {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Pant, pant. Too tired.".to_string()],
                ..Default::default()
            }));
        }

        let mut ptr = rest.trim_start();
        let mut m = 0i32;
        if ptr.starts_with(|ch: char| ch.is_ascii_digit()) {
            m = legacy_atoi_prefix(ptr) as i32;
            ptr = ptr.trim_start_matches(|ch: char| !ch.is_whitespace());
            ptr = ptr.trim_start();
        }
        let (mut x, mut y, mut a) = (0i32, 0i32, 0i32);
        if let Some((gx, gy, ga)) = goto_list_lookup(ptr) {
            x = i32::from(gx);
            y = i32::from(gy);
            a = ga as i32;
        }
        if a == area_id as i32 && m == 0 {
            a = 0;
        }

        if x <= 0 || y <= 0 || !world.map.legacy_inner_bounds(x as usize, y as usize) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["hu?".to_string()],
                ..Default::default()
            }));
        }
        return ControlFlow::Break(Some(finish_goto_jump(
            world,
            character_id,
            x,
            y,
            a,
            m,
            "jump",
        )));
    }

    // C `/gotolist` (`command.c:236-245`, dispatched at `command.c:8815-
    // 8822`), `CF_GOD`-gated. Lists every `gl[]` shortcut with its
    // coordinates and area.
    if lower == "gotolist" {
        if !world
            .characters
            .get(&character_id)
            .is_some_and(|character| character.flags.contains(CharacterFlags::GOD))
        {
            return ControlFlow::Break(None);
        }
        let mut messages = vec!["Available /goto locations:".to_string()];
        messages.extend(
            GOTO_LIST
                .iter()
                .map(|(name, x, y, a)| format!("{name} (x:{x}, y:{y}, area:{a})")),
        );
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `/gotosearch <term>` (`command.c:248-269`, dispatched at
    // `command.c:8823-8829`), `CF_GOD`-gated. Substring search is
    // case-sensitive (C `strstr`, not `strcasestr`) - copied as-is.
    if lower.len() >= 8 && "gotosearch".starts_with(lower) {
        if !world
            .characters
            .get(&character_id)
            .is_some_and(|character| character.flags.contains(CharacterFlags::GOD))
        {
            return ControlFlow::Break(None);
        }
        let term = rest.trim_start();
        if term.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Please provide a search term.".to_string()],
                ..Default::default()
            }));
        }
        let matches: Vec<_> = GOTO_LIST
            .iter()
            .filter(|(name, ..)| name.contains(term))
            .collect();
        let mut messages = vec!["Matching /goto locations:".to_string()];
        messages.extend(
            matches
                .iter()
                .map(|(name, x, y, a)| format!("{name} (x:{x}, y:{y}, area:{a})")),
        );
        if matches.is_empty() {
            messages.push("No matching locations found.".to_string());
        } else {
            messages.push(format!(
                "Found {} matching location{}.",
                matches.len(),
                if matches.len() == 1 { "" } else { "s" }
            ));
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `/summon <name>` (`command.c:8628-8649`), `CF_GOD`-gated. Finds the
    // first character slot (any flags set, not just `CF_PLAYER` - so NPCs
    // can be summoned too) whose name case-insensitively matches the whole
    // remainder of the line, then teleports it next to the caller via
    // `teleport_char_driver` (C `drvlib.c:2651-2673`). No user-visible
    // message on success or failure - only the C `dlog` staff-action log
    // entry, approximated here with a `debug!` trace.
    if lower.len() >= 3 && "summon".starts_with(lower) {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (cx, cy) = (caller.x, caller.y);
        let name = rest.trim_start();
        if let Some(target_id) = find_online_character_by_name(world, name) {
            if world.teleport_char_driver(target_id, cx, cy) {
                if let Some(target) = world.characters.get(&target_id) {
                    debug!(target: "client_log", name = %target.name, id = target_id.0, "summon teleport");
                }
            }
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/kick <name>` (`command.c:8668-8698`), gated on `CF_STAFF|CF_GOD`
    // (no abbreviation, `cmdcmp(ptr, "kick", 4)` requires the exact
    // 4-letter word). Finds the first `CF_PLAYER` character whose name
    // case-insensitively matches the remainder of the line; on a match,
    // tells the caller "Kicked %s." (C `log_char`) and signals the call
    // site (via `kick_target`) to perform the full `exit_char` (save at
    // rest position + despawn) + `player_client_exit` (send `SV_EXIT`
    // with the kick reason, disconnect) teardown on the target - the same
    // deferred side effects as `/logout`, just targeting someone else.
    // On no match, tells the caller "No player by the name %s." The C
    // `dlog` staff-action audit log and `write_scrollback` (which emails
    // the *caller's own* scrollback buffer to game@ugaris.com as
    // moderation evidence - there is no email/CURL infra in this
    // codebase) are both skipped, matching the established convention for
    // untracked audit-only C side effects (see `/summon` above).
    if lower == "kick" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        let name = rest.trim_start();
        let target = world.characters.values().find(|character| {
            character.flags.contains(CharacterFlags::PLAYER)
                && character.name.eq_ignore_ascii_case(name)
        });
        return ControlFlow::Break(Some(match target {
            Some(target) => KeyringCommandResult {
                messages: vec![format!("Kicked {}.", name)],
                kick_target: Some(target.id),
                ..Default::default()
            },
            None => KeyringCommandResult {
                messages: vec![format!("No player by the name {name}.")],
                ..Default::default()
            },
        }));
    }

    // C `/summonall` (`command.c:8653-8667`), `CF_GOD`-gated. Teleports
    // every `CF_PLAYER` character next to the caller, one at a time (the
    // caller themselves is included in the iteration but is a no-op since
    // `teleport_char_driver` refuses moves under Manhattan distance 2).
    if lower == "summonall" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (cx, cy) = (caller.x, caller.y);
        let player_ids: Vec<CharacterId> = world
            .characters
            .values()
            .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
            .map(|character| character.id)
            .collect();
        for target_id in player_ids {
            if world.teleport_char_driver(target_id, cx, cy) {
                if let Some(target) = world.characters.get(&target_id) {
                    debug!(target: "client_log", name = %target.name, id = target_id.0, "summonall teleport");
                }
            }
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/office` (`command.c:9670-9676`), `CF_GOD`-gated, `minlen=6` so
    // the full word must be typed (`cmdcmp(ptr, "office", 6)`, no
    // abbreviation). Teleports to the staff office in Aston (area 3,
    // x:11, y:195): via `change_area` when not already in area 3 (the
    // call site resolves `cross_area_transfer` via
    // `attempt_cross_area_transfer`, falling back to the "Nothing
    // happens" message on failure), or directly via `teleport_char_driver`
    // when already in Aston.
    if lower == "office" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        if area_id != 3 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                cross_area_transfer: Some((3, 11, 195)),
                ..Default::default()
            }));
        }
        if world.teleport_char_driver(character_id, 11, 195) {
            debug!(target: "client_log", "office teleport");
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    ControlFlow::Continue(())
}

/// Parses the `x y area` triple used by `/setjaillocation` and
/// `/setastonlocation` (C `command.c:8036-8050`/`8076-8090`): `atoi` at the
/// current pointer, then skip ascii digits, then skip whitespace, repeated
/// three times.
pub(crate) fn parse_legacy_xyz_triple(rest: &str) -> (i32, i32, i32) {
    let mut ptr = rest.trim_start();
    let x = legacy_atoi_prefix(ptr) as i32;
    ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
    ptr = ptr.trim_start();
    let y = legacy_atoi_prefix(ptr) as i32;
    ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
    ptr = ptr.trim_start();
    let area = legacy_atoi_prefix(ptr) as i32;
    (x, y, area)
}

/// C `gl[]` (`src/system/command.c:132-207`) - the shortcut-destination
/// table shared by `/goto` and `/jump`. Copied name/x/y/area digit for
/// digit.
pub(crate) const GOTO_LIST: &[(&str, u16, u16, u32)] = &[
    ("aston", 167, 188, 3),
    ("elysium", 12, 178, 3),
    ("fort", 126, 179, 1),
    ("zomb1", 5, 5, 2),
    ("zomb2", 3, 86, 2),
    ("skel2", 85, 85, 1),
    ("skel3", 184, 226, 1),
    ("mages", 154, 106, 1),
    ("knights", 163, 82, 1),
    ("trans", 130, 201, 3),
    ("mine", 231, 242, 12),
    ("hole", 236, 176, 3),
    ("lq", 245, 245, 20),
    ("bran", 203, 227, 29),
    ("hole2", 226, 164, 29),
    ("smuggle", 103, 107, 26),
    ("yendor", 41, 250, 14),
    ("grim", 210, 247, 31),
    ("exkor", 67, 108, 17),
    ("job", 228, 228, 32),
    ("tunnel", 250, 250, 33),
    ("teufel", 250, 250, 34),
    ("rds", 245, 250, 3),
    ("swamps", 239, 237, 5),
    ("satp", 229, 94, 3),
    ("creep", 195, 120, 3),
    ("ark", 27, 14, 37),
    ("jail", 186, 234, 3),
    ("lab1", 32, 242, 22),
    ("lab2", 70, 98, 22),
    ("lab3", 230, 250, 22),
    ("lab4", 147, 103, 22),
    ("lab5", 166, 243, 22),
    ("max5s", 26, 26, 30),
    ("max10s", 109, 108, 30),
    ("max15s", 130, 26, 30),
    ("max18s", 181, 16, 30),
    ("max20s", 57, 26, 30),
    ("max24s", 73, 109, 30),
    ("max28s", 78, 16, 30),
    ("max30s", 12, 122, 30),
    ("max34s", 143, 76, 30),
    ("max36s", 212, 6, 30),
    ("max38s", 49, 112, 30),
    ("max40s", 171, 90, 30),
    ("max42s", 150, 57, 30),
    ("max43s", 212, 67, 30),
    ("max44s", 243, 16, 30),
    ("max45s", 231, 65, 30),
    ("max46s", 171, 61, 30),
    ("max48s", 120, 15, 30),
    ("max50s", 211, 47, 30),
    ("max52s", 16, 39, 30),
    ("max60s", 35, 59, 30),
    ("max64s", 233, 54, 30),
    ("max68s", 88, 35, 30),
    ("max76s", 121, 59, 30),
    ("max84s", 28, 90, 30),
    ("max92s", 34, 65, 30),
    ("max100s", 75, 67, 30),
    ("max108s", 109, 78, 30),
    ("max160s", 14, 140, 30),
    ("max200s", 40, 134, 30),
    ("mineshop10", 43, 232, 12),
    ("mineshop20", 43, 203, 12),
    ("mineshop30", 43, 171, 12),
    ("mineshop40", 43, 139, 12),
    ("mineshop50", 43, 107, 12),
    ("mineshop60", 43, 75, 12),
    ("mineshop70", 43, 43, 12),
    ("mineshop80", 43, 11, 12),
    ("mineshop90", 13, 239, 31),
    ("mineshop100", 13, 207, 31),
    ("mineshop110", 13, 175, 31),
    ("mineshop120", 13, 143, 31),
    ("teufeltp", 224, 248, 34),
    ("teufelicegambler", 84, 186, 34),
    ("teufelfiregambler", 123, 227, 34),
    ("teufelearthgambler", 248, 238, 34),
];

pub(crate) fn goto_list_lookup(name: &str) -> Option<(u16, u16, u32)> {
    GOTO_LIST
        .iter()
        .find(|(candidate, ..)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, x, y, a)| (*x, *y, *a))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GotoJumpTarget {
    x: i32,
    y: i32,
    a: i32,
    m: i32,
}

/// C `/goto`'s argument resolution (`command.c:8460-8535`). `ptr` is the
/// text after `"goto "` (already `trim_start`-ed by the caller isn't
/// required; this trims itself, matching C's `while (isspace(*ptr))
/// ptr++;`). Mirrors the exact pointer-stepping quirks of the original,
/// including the fact that a name lookup (`x == atoi(ptr) == 0` branch)
/// compares the *entire remaining string* against `gl[].name`/character
/// names (C `strcasecmp(gl[n].name, ptr)` with the untouched `ptr`) - so a
/// trailing mirror argument after a name is silently ignored (the name
/// simply fails to match anything, since the full remaining text no
/// longer equals just the name). `jump` doesn't call this: it has its own
/// simpler resolution (mirror-prefix, then a single `gl[]` name lookup,
/// no numeric/relative form) ported directly in the dispatcher above.
pub(crate) fn resolve_goto_jump_args(
    world: &World,
    caller_x: u16,
    caller_y: u16,
    args: &str,
) -> GotoJumpTarget {
    let mut ptr = args.trim_start();
    let x_val = legacy_atoi_prefix(ptr) as i32;
    let (mut x, mut y, mut a) = (0i32, 0i32, 0i32);
    if x_val == 0 {
        // Full remaining text (unmodified) is the name candidate - copies
        // the C `strcasecmp(gl[n].name, ptr)`/`strcasecmp(ch[n].name,
        // ptr)` full-string comparison exactly.
        if let Some((gx, gy, ga)) = goto_list_lookup(ptr) {
            x = i32::from(gx);
            y = i32::from(gy);
            a = ga as i32;
        } else if let Some(target_id) = find_online_character_by_name(world, ptr) {
            if let Some(target) = world.characters.get(&target_id) {
                x = i32::from(target.x);
                y = i32::from(target.y);
            }
        }
        // `ptr` is NOT advanced by the name lookup in C (strcasecmp
        // doesn't move the pointer) - the final "consume one token, then
        // parse m" step below still operates on the original `ptr`.
    } else {
        x = x_val;
        ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
        ptr = ptr.trim_start();
        let y_val = legacy_atoi_prefix(ptr) as i32;
        if y_val == 0 {
            match ptr.chars().next().map(|ch| ch.to_ascii_lowercase()) {
                Some('n') => {
                    y = i32::from(caller_y) - x;
                    x = i32::from(caller_x) - x;
                }
                Some('s') => {
                    y = i32::from(caller_y) + x;
                    x += i32::from(caller_x);
                }
                Some('w') => {
                    y = i32::from(caller_y) + x;
                    x = i32::from(caller_x) - x;
                }
                Some('e') => {
                    y = i32::from(caller_y) - x;
                    x += i32::from(caller_x);
                }
                _ => {
                    x = 0;
                    y = 0;
                }
            }
            // `ptr` still points at the direction-letter token (or
            // whatever failed to parse as a direction).
        } else {
            y = y_val;
            ptr = ptr.trim_start_matches(|ch: char| ch.is_ascii_digit());
            ptr = ptr.trim_start();
            a = legacy_atoi_prefix(ptr) as i32;
            // `ptr` still points at `a`'s token.
        }
    }

    // Consume whatever token `ptr` currently points at, then parse `m`
    // from the remainder (C `while (!isspace(*ptr) && *ptr) ptr++; while
    // (isspace(*ptr)) ptr++; m = atoi(ptr);`).
    ptr = ptr.trim_start_matches(|ch: char| !ch.is_whitespace());
    ptr = ptr.trim_start();
    let m = legacy_atoi_prefix(ptr) as i32;

    GotoJumpTarget { x, y, a, m }
}

/// Shared tail of `/goto` (`command.c:8537-8567`) and `/jump`
/// (`command.c:8608-8625`): apply the mirror change (if any), then either
/// same-area `teleport_char_driver` or the cross-area `change_area`
/// handoff via `cross_area_transfer` (the `main.rs` call site resolves it
/// through `attempt_cross_area_transfer`, falling back to the "target
/// area server is down" message on failure, matching every other
/// cross-area teleport site in this codebase).
pub(crate) fn finish_goto_jump(
    world: &mut World,
    character_id: CharacterId,
    x: i32,
    y: i32,
    a: i32,
    m: i32,
    verb: &'static str,
) -> KeyringCommandResult {
    let mirror_changed = (1..27).contains(&m).then_some(m as u32);

    if a != 0 {
        return KeyringCommandResult {
            cross_area_transfer: Some((
                a.clamp(0, i32::from(u16::MAX)) as u16,
                x.clamp(0, i32::from(u16::MAX)) as u16,
                y.clamp(0, i32::from(u16::MAX)) as u16,
            )),
            mirror_changed,
            ..Default::default()
        };
    }

    if x <= 0
        || y <= 0
        || !world
            .map
            .legacy_inner_bounds(x.max(0) as usize, y.max(0) as usize)
    {
        return KeyringCommandResult {
            mirror_changed,
            ..Default::default()
        };
    }

    if world.teleport_char_driver(character_id, x as u16, y as u16) {
        debug!(target: "client_log", verb, x, y, "goto/jump teleport");
    }

    KeyringCommandResult {
        mirror_changed,
        ..Default::default()
    }
}
