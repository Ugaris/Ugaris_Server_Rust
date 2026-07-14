use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_flags_clan(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    let Some(character) = world.characters.get_mut(&character_id) else {
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    };

    let is_lqmaster = character.flags.contains(CharacterFlags::GOD)
        || character.flags.contains(CharacterFlags::EVENTMASTER)
        || (area_id == 20 && character.flags.contains(CharacterFlags::LQMASTER));

    if lower == "noexp" {
        if !character.flags.contains(CharacterFlags::NOEXP)
            && is_gatekeeper_room(area_id, character)
        {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Cannot turn NoExp mode on while in Gatekeeper room.".to_string()],
                ..Default::default()
            }));
        }
        character.flags.toggle(CharacterFlags::NOEXP);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned NoExp mode {}.",
                if character.flags.contains(CharacterFlags::NOEXP) {
                    "on"
                } else {
                    "off"
                }
            )],
            inventory_changed: true,
            ..Default::default()
        }));
    }

    if lower == "nolevel" {
        if !character.flags.contains(CharacterFlags::NOLEVEL)
            && is_gatekeeper_room(area_id, character)
        {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Cannot turn NoLevel mode on while in Gatekeeper room.".to_string()],
                ..Default::default()
            }));
        }
        character.flags.toggle(CharacterFlags::NOLEVEL);
        let enabled = character.flags.contains(CharacterFlags::NOLEVEL);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![if enabled {
                "NoLevel mode enabled. You will not level up until you disable this mode."
                    .to_string()
            } else {
                "NoLevel mode disabled. You will now gain levels normally.".to_string()
            }],
            inventory_changed: true,
            ..Default::default()
        }));
    }

    if lower == "itemmod" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (pos, nr, val) = parse_itemmod_args(rest);
        let Some(item_id) = character.cursor_item else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            }));
        };
        if pos < 0 || pos >= ugaris_core::entity::MAX_MODIFIERS as i64 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Pos out of bounds.".to_string()],
                ..Default::default()
            }));
        }
        if nr < 0 || nr >= CHARACTER_VALUE_NAMES.len() as i64 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Nr out of bounds.".to_string()],
                ..Default::default()
            }));
        }
        if !(0..22).contains(&val) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Val out of bounds.".to_string()],
                ..Default::default()
            }));
        }
        let character_snapshot = character.clone();
        let Some(item) = world.items.get_mut(&item_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            }));
        };
        item.modifier_index[pos as usize] = nr as i16;
        item.modifier_value[pos as usize] = val as i16;
        let mut messages: Vec<String> = legacy_item_look_text(item, &character_snapshot)
            .lines()
            .map(str::to_string)
            .collect();
        messages.push(format!(
            "Item modified: {} (skill {}) at pos {} with value {}",
            value_name(nr as i16),
            nr,
            pos,
            val
        ));
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            inventory_changed: true,
            ..Default::default()
        }));
    }

    if lower == "itemdesc" || lower == "itemname" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let Some(item_id) = character.cursor_item else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            }));
        };
        let trimmed = rest.trim_start();
        let text = legacy_truncate_c_string(trimmed, 79);
        let character_snapshot = character.clone();
        let Some(item) = world.items.get_mut(&item_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Need citem.".to_string()],
                ..Default::default()
            }));
        };
        if lower == "itemdesc" {
            item.description = text;
        } else {
            item.name = text;
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: legacy_item_look_text(item, &character_snapshot)
                .lines()
                .map(str::to_string)
                .collect(),
            inventory_changed: true,
            ..Default::default()
        }));
    }

    if lower.len() >= 4 && "saves".starts_with(lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let saves = legacy_atoi_prefix(rest).clamp(0, i64::from(u8::MAX)) as u8;
        character.saves = saves;
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/saveall` (`command.c:7460-7473`, `cmdcmp(ptr, "saveall", 4)`,
    // `CF_GOD`-gated). Must be checked after the `saves` block above
    // (matching C's own line order, 6278 before 7460): `cmdcmp(ptr,
    // "saves", 4)` matches the literal input "save" first in C, so
    // "/save" is `saves` (a stat setter) not `saveall`, and only
    // "/savea"/"/saveal"/"/saveall" reach this block. See the
    // `save_all_requested` doc comment on `KeyringCommandResult` for what
    // the `main.rs` call site does with the flag.
    if lower.len() >= 4 && "saveall".starts_with(lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![
                "Forcing save of all players...".to_string(),
                "Player data saved".to_string(),
                "Forcing save of merchant inventories...".to_string(),
                "Merchant data saved".to_string(),
            ],
            save_all_requested: true,
            ..Default::default()
        }));
    }

    // C `/shutdown` (`command.c:6068-6086`, `cmdcmp(ptr, "shutdown", 8)`,
    // `CF_GOD`-gated). `minlen` equals the full word length, so unlike most
    // commands here no abbreviation is accepted - only the exact word
    // "shutdown" (case-insensitive) reaches this block.
    if lower == "shutdown" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        // C: `ptr += len; while (isspace(*ptr)) ptr++; diff = atoi(ptr);
        // while (isdigit(*ptr)) ptr++; while (isspace(*ptr)) ptr++; down =
        // atoi(ptr);` - note the `isdigit`-skip does not step over a
        // leading `-` sign, so a negative `diff` leaves `down` parsed from
        // the exact same substring (a real, reproducible C quirk).
        let ptr = rest.trim_start();
        let diff = legacy_atoi_prefix(ptr);
        let after_digits = ptr
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .trim_start();
        let down = legacy_atoi_prefix(after_digits);
        apply_shutdown_command(world, runtime, diff, down);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    if lower == "sprite" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        character.sprite = legacy_atoi_prefix(rest) as i32;
        return ControlFlow::Break(Some(KeyringCommandResult {
            inventory_changed: true,
            name_changed: true,
            ..Default::default()
        }));
    }

    if lower.len() >= 2 && "immortal".starts_with(lower) {
        if !is_lqmaster {
            return ControlFlow::Break(None);
        }
        character.flags.toggle(CharacterFlags::IMMORTAL);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Immortal is {}.",
                if character.flags.contains(CharacterFlags::IMMORTAL) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        }));
    }

    if lower.len() >= 3 && "infrared".starts_with(lower) {
        if !is_lqmaster {
            return ControlFlow::Break(None);
        }
        character.flags.toggle(CharacterFlags::INFRARED);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Infrared is {}.",
                if character.flags.contains(CharacterFlags::INFRARED) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        }));
    }

    if lower.len() >= 3 && "invisible".starts_with(lower) {
        if !is_lqmaster {
            return ControlFlow::Break(None);
        }
        character.flags.toggle(CharacterFlags::INVISIBLE);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Invisible is {}.",
                if character.flags.contains(CharacterFlags::INVISIBLE) {
                    "on"
                } else {
                    "off"
                }
            )],
            ..Default::default()
        }));
    }

    if lower == "xray" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        character.flags.toggle(CharacterFlags::XRAY);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned x-ray mode {}.",
                if character.flags.contains(CharacterFlags::XRAY) {
                    "on"
                } else {
                    "off"
                }
            )],
            inventory_changed: true,
            ..Default::default()
        }));
    }

    if lower.len() >= 3 && "spy".starts_with(lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        character.flags.toggle(CharacterFlags::SPY);
        let enabled = character.flags.contains(CharacterFlags::SPY);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Turned spy mode {}. You will {} see all tells, clan, alliance, club, area, and mirror chat.",
                if enabled { "on" } else { "off" },
                if enabled { "now" } else { "no longer" }
            )],
            ..Default::default()
        }));
    }

    if lower == "setxmas" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let flag = legacy_atoi_prefix(rest.trim_start()) as i32;
        let old_value = runtime_effective_xmas_flag(runtime);
        runtime.xmas_special_override = Some(flag);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages: vec![format!(
                "Setting christmas special to {flag}, old value was {old_value}."
            )],
            ..Default::default()
        }));
    }

    if lower.len() >= 6 && "dlight".starts_with(lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        runtime.dlight_override = legacy_atoi_prefix(rest) as i32;
        let override_value = (runtime.dlight_override != 0).then_some(runtime.dlight_override);
        world.date = GameDate::calculate(
            START_TIME + world.date.realtime,
            area_id as i32,
            override_value,
        );
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    if lower.len() >= 6 && "showattack".starts_with(lower) {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        runtime.show_attack = !runtime.show_attack;
        world.show_attack_debug = runtime.show_attack;
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    if lower == "joinclan" || lower == "joinclub" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let nr = legacy_atoi_prefix(rest.trim_start());
        if lower == "joinclan" {
            if (0..LEGACY_MAX_CLAN).contains(&nr) {
                character.clan = nr as u16;
                character.clan_rank = 4;
                character.clan_serial = world.clan_registry.serial(nr as u16);
            }
        } else if (0..LEGACY_MAX_CLUB).contains(&nr) {
            character.clan = (nr + LEGACY_CLUB_OFFSET) as u16;
            character.clan_rank = 2;
            character.clan_serial = 0;
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            name_changed: true,
            ..Default::default()
        }));
    }

    // C `killclan` (`src/system/command.c:6468-6482`): sets the target
    // clan's debt sky-high (`kill_clan`, `clan.c:1413-1416`) so the next
    // weekly `update_treasure` tick (`clan.c:1154-1160`, `debt >= 2000`)
    // deletes it. `update_treasure`/the whole treasury economy isn't
    // ported (see the clan task's REMAINING notes), so this deletes the
    // clan immediately via [`ClanRegistry::delete_clan`] - the eventual
    // real-world outcome of C's `kill_clan`, without the week-long delay.
    // C emits no player feedback for this command; matched exactly (no
    // messages either way).
    if lower == "killclan" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let nr = legacy_atoi_prefix(rest.trim_start());
        if (1..LEGACY_MAX_CLAN).contains(&nr) {
            world.clan_registry.delete_clan(nr as u16);
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `killclub` (`src/system/command.c:6484-6497`), `CF_GOD`-gated.
    // Genuine C bug kept for fidelity: the bounds check guarding the
    // `kill_club` call compares `nr` against `MAXCLAN` (32,
    // `crate::commands_chat::LEGACY_MAX_CLAN`'s C counterpart), not
    // `MAXCLUB` (16384) - copy-paste leftover from the adjacent
    // `killclan` block above (`club.c`'s own `kill_club(int cnr)` itself
    // correctly bounds-checks against `MAXCLUB`, so this cap only bites
    // at the command layer). `kill_club` (`club.c:132-138`) doesn't clear
    // the club's name - it zeroes `money` and sets `paid = 1` so the next
    // `ClubRegistry::tick_billing` weekly pass deletes it for
    // nonpayment, exactly like `killclan`'s `kill_clan`/`update_treasure`
    // relationship. No player feedback either way, matched exactly.
    if lower == "killclub" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let nr = legacy_atoi_prefix(rest.trim_start());
        if (1..LEGACY_MAX_CLAN).contains(&nr) {
            world.club_registry.kill_club(nr as u16);
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/setclanjewels` (`command.c:7563-7596`), `CF_GOD`-gated. Directly
    // assigns `clan[clan_nr].treasure.jewels`, a distinct storage system
    // from the `GameSettings`-backed `set*` tuning-knob family closed out
    // in an earlier iteration (see this task's REMAINING notes). Args are
    // whitespace-separated `<clan_nr> <jewels> [do_log]`; `do_log`
    // defaults to `1` (log to the clan log) exactly like C's `int do_log =
    // 1; if (*ptr) do_log = atoi(ptr);`. Out-of-range clan numbers,
    // negative jewel counts, or an in-range clan number with no clan
    // actually created there (C's array is preallocated for every
    // in-range slot and would silently write through it anyway - a
    // footgun, not a feature - but this registry has no such slot; see
    // `ClanRegistry::set_jewels`) all report the same "Invalid clan
    // number or jewel count" message C emits only for the former two
    // cases.
    if lower == "setclanjewels" {
        if !character.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let mut tokens = rest.split_whitespace();
        let clan_nr = tokens.next().map(legacy_atoi_prefix).unwrap_or(0);
        let jewels = tokens.next().map(legacy_atoi_prefix).unwrap_or(0);
        let do_log = tokens.next().map(legacy_atoi_prefix).unwrap_or(1);
        let old_jewels = (clan_nr > 0 && clan_nr < LEGACY_MAX_CLAN && jewels >= 0)
            .then(|| {
                world
                    .clan_registry
                    .set_jewels(clan_nr as u16, jewels as i32)
            })
            .flatten();
        let Some(old_jewels) = old_jewels else {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Invalid clan number or jewel count".to_string()],
                ..Default::default()
            }));
        };
        let clan_nr = clan_nr as u16;
        let clan_name = world.clan_registry.name(clan_nr).unwrap_or("").to_string();
        let messages = vec![format!(
            "Clan {clan_nr} ({clan_name}) jewels changed from {old_jewels} to {jewels}"
        )];
        let clan_log_entry = (do_log != 0).then(|| {
            (
                clan_nr,
                world.clan_registry.serial(clan_nr),
                1u8,
                format!(
                    "God {} changed clan jewels from {old_jewels} to {jewels}",
                    character.name
                ),
            )
        });
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            clan_log_entry,
            ..Default::default()
        }));
    }

    // C `cmd_renclan` (`src/system/command.c:4497-4531`), dispatched at
    // `command.c:9646` gated on `CF_STAFF | CF_GOD`. Renames an existing
    // clan; only usable while standing in Aston (`areaID == 3`).
    if lower == "renclan" {
        if !character
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        if area_id != 3 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Sorry, this command only works in Aston.".to_string()],
                ..Default::default()
            }));
        }
        let rest = rest.trim_start();
        let nr = legacy_atoi_prefix(rest);
        let name = rest
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .trim_start();
        if !(1..LEGACY_MAX_CLAN).contains(&nr) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Clan number must be between 1 and {}.",
                    LEGACY_MAX_CLAN - 1
                )],
                ..Default::default()
            }));
        }
        let name: String = name.chars().take(78).collect();
        let messages = match world.clan_registry.set_name(nr as u16, &name) {
            Ok(()) => vec![format!("Clan {nr} name changed to \"{name}\".")],
            Err(_) => vec![format!("No clan by that number ({nr}).")],
        };
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `cmd_renclub` (`src/system/command.c:4548-4585`), dispatched at
    // `command.c:9650` gated on `CF_STAFF | CF_GOD`. Renames an existing
    // club; only usable "nearby a clubmaster" per C's message text, but
    // the actual gate C checks is the same `areaID == 3` as `/renclan`
    // (`club.c` has no clubmaster-proximity concept - the message is
    // aspirational/copy-pasted text, not a real distinct check).
    // `ClubRegistry::rename_club` folds C's three separate failure modes
    // (invalid characters, name too long, name already taken) into one
    // `Err`, matching C's own single combined "didn't work" message for
    // all three (`rename_club` returning `0`).
    if lower == "renclub" {
        if !character
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        if area_id != 3 {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec!["Sorry, this command only works nearby a clubmaster.".to_string()],
                ..Default::default()
            }));
        }
        let rest = rest.trim_start();
        let nr = legacy_atoi_prefix(rest);
        let name = rest
            .trim_start_matches(|ch: char| ch.is_ascii_digit())
            .trim_start();
        if !(1..LEGACY_MAX_CLUB).contains(&nr) {
            return ControlFlow::Break(Some(KeyringCommandResult {
                messages: vec![format!(
                    "Club number must be between 1 and {}.",
                    LEGACY_MAX_CLUB - 1
                )],
                ..Default::default()
            }));
        }
        let name: String = name.chars().take(78).collect();
        let messages = match world.club_registry.rename_club(nr as u16, &name) {
            Ok(()) => vec![format!("Club {nr} name changed to \"{name}\".")],
            Err(_) => {
                vec!["That didn't work. The name is either taken or illegal.".to_string()]
            }
        };
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `cmd_flag` (`command.c:2870-2937`), the shared by-name flag-
    // toggle body of `/god` (`CF_GOD`), `/setsir` (`CF_WON`), `/staff`
    // (`CF_STAFF`), `/emaster` (`CF_EVENTMASTER`), `/devel`
    // (`CF_DEVELOPER`), `/hardcore` (`CF_HARDCORE`), and `/qmaster`
    // (`CF_LQMASTER`) - all dispatched at `command.c:9257-9337`, all
    // `CF_GOD`-gated, all full-word only (`cmdcmp`'s `minlen` equals the
    // command's own length for every one of these seven, so no
    // abbreviation is accepted - matched with `lower == "..."`, not
    // `starts_with`). See `World::apply_cmd_flag_command`'s doc comment
    // for the online/offline message-shape split.
    if let Some((flag, flag_name)) = match lower {
        "god" => Some((CharacterFlags::GOD, "god")),
        "setsir" => Some((CharacterFlags::WON, "sir/lady")),
        "staff" => Some((CharacterFlags::STAFF, "staff")),
        "emaster" => Some((CharacterFlags::EVENTMASTER, "master of events")),
        "devel" => Some((CharacterFlags::DEVELOPER, "developer")),
        "hardcore" => Some((CharacterFlags::HARDCORE, "hardcore")),
        "qmaster" => Some((CharacterFlags::LQMASTER, "qmaster")),
        _ => None,
    } {
        if !world
            .characters
            .get(&character_id)
            .is_some_and(|caller| caller.flags.contains(CharacterFlags::GOD))
        {
            return ControlFlow::Break(None);
        }
        let (name, _remainder) = take_legacy_alpha_name(rest.trim_start());
        let messages = world.apply_cmd_flag_command(character_id, name, flag, flag_name);
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    ControlFlow::Continue(())
}

pub(crate) fn is_gatekeeper_room(area_id: u32, character: &Character) -> bool {
    area_id == 3 && (178..=210).contains(&character.x) && (196..=228).contains(&character.y)
}
