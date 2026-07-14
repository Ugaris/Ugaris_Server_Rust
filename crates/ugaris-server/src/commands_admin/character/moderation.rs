use std::ops::ControlFlow;

use super::*;

pub(super) fn dispatch_moderation(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    character_id: CharacterId,
    _area_id: u32,
    lower: &str,
    rest: &str,
) -> ControlFlow<Option<KeyringCommandResult>> {
    // C `/jail <name>`/`/unjail <name>` (`command.c:8861-8882`/
    // `8839-8858`), `CF_STAFF|CF_GOD`-gated, full-word only (`cmdcmp`'s
    // `minlen` equals each full word's length, no abbreviation accepted).
    // Trims leading whitespace off the argument, then hands it to
    // `World::queue_jail_lookup`, which does all further validation and
    // DB resolution - see that function's and `ugaris-server`'s
    // `apply_jail_events`'s doc comments for the full behavior. Always
    // returns a `default()` result immediately; the real reply arrives
    // later via `World::queue_system_text`, matching C's own fire-and-
    // forget async `lookup_name` DB-worker round-trip.
    if lower == "jail" || lower == "unjail" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        let action = if lower == "jail" {
            ugaris_core::world::JailAction::Jail
        } else {
            ugaris_core::world::JailAction::Unjail
        };
        world.queue_jail_lookup(character_id, rest.trim_start(), action);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/rmdeath <name>` (`command.c:8884-8903` dispatch ->
    // `cmd_removedeath`, `command.c:2006-2019`), `CF_GOD`-gated, full-word
    // only (`cmdcmp`'s `minlen` is 7, the full length of "rmdeath", no
    // abbreviation accepted). Trims leading whitespace off the argument,
    // then hands it to `World::queue_rmdeath_lookup`, which does all
    // further validation and DB resolution - see that function's and
    // `world/rmdeath.rs`'s module doc comment for the full behavior.
    // Always returns a `default()` result immediately; the real reply
    // arrives later via `World::queue_system_text`, matching C's own
    // fire-and-forget async `lookup_name` DB-worker round-trip (same
    // pattern as `/jail`/`/unjail` above).
    if lower == "rmdeath" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        world.queue_rmdeath_lookup(character_id, rest.trim_start());
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/rename <from> <to>` (`command.c:6517-6524` dispatch ->
    // `cmd_rename`, `command.c:2657-2676`), `CF_GOD`-gated, full-word
    // only (`cmdcmp`'s `minlen` is 6, the full length of "rename", no
    // abbreviation accepted). Parses two consecutive `isalpha`-only name
    // tokens (`take_legacy_alpha_name`, mirroring C's own two scan
    // loops, `command.c:2661-2670`), each truncated to the C buffer's
    // 79-byte cap; hands both to `World::queue_rename_command`, which
    // performs all further validation and DB resolution - see that
    // function's and `world/rename.rs`'s module doc comment for the full
    // behavior. Always returns a `default()` result immediately; the
    // real reply arrives later via `World::queue_system_text` (same
    // fire-and-forget async pattern as `/jail`/`/rmdeath` above).
    if lower == "rename" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (from, remainder) = take_legacy_alpha_name(rest.trim_start());
        let from = &from[..from.len().min(79)];
        let (to, _remainder) = take_legacy_alpha_name(remainder.trim_start());
        let to = &to[..to.len().min(79)];
        world.queue_rename_command(character_id, from, to);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/lockname <name>`/`/unlockname <name>` (`command.c:6528-6543`
    // dispatch -> `cmd_lockname`/`cmd_unlockname`, `command.c:2679-2701`),
    // both `CF_GOD`-gated, full-word only (`cmdcmp`'s `minlen` is 8/10,
    // the full word length, no abbreviation accepted). Parses one
    // `isalpha`-only name token, truncated to the C buffer's 79-byte cap;
    // hands it to `World::queue_lockname_command`/
    // `queue_unlockname_command` - see those functions' and
    // `world/lockname.rs`'s module doc comment for the full behavior.
    if lower == "lockname" || lower == "unlockname" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (name, _remainder) = take_legacy_alpha_name(rest.trim_start());
        let name = &name[..name.len().min(79)];
        if lower == "lockname" {
            world.queue_lockname_command(character_id, name);
        } else {
            world.queue_unlockname_command(character_id, name);
        }
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    // C `/punish <name> <level> <reason>` (`command.c:6500-6507` dispatch
    // -> `cmd_punish`, `command.c:2354-2406`), `CF_GOD|CF_STAFF`-gated,
    // full-word only (`cmdcmp`'s `minlen` is 6, the full length of
    // "punish", no abbreviation accepted). Parses an `isalpha`-only name
    // token (`take_legacy_alpha_name`, truncated to the 79-byte buffer
    // cap like `/rename`), then `level = atoi(ptr); while (isdigit(*ptr))
    // ptr++;` (a leading `-`/`+` sign, if any, is *not* skipped by this
    // second loop even though `atoi` itself parsed it - a genuine C quirk
    // only reachable with a malformed negative level, reproduced here by
    // only ever skipping digit characters, never a sign), then the
    // remaining raw bytes (not alpha-filtered, unlike the name) become
    // `reason`, capped at 79 bytes with `reason_overflowed` recording
    // whether the original text was longer - see `World::
    // queue_punish_command`'s doc comment for the validation this hands
    // off to. Always returns a `default()` result immediately; the real
    // reply arrives later via `World::queue_system_text` (same
    // fire-and-forget async pattern as `/jail`/`/rmdeath`/`/rename`
    // above).
    if lower == "punish" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        let (name, after_name) = take_legacy_alpha_name(rest.trim_start());
        let name = &name[..name.len().min(79)];
        let after_level = after_name.trim_start();
        let level = legacy_atoi_prefix(after_level) as i32;
        let digits_end = after_level
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(after_level.len());
        let reason_raw = after_level[digits_end..].trim_start();
        let reason_overflowed = reason_raw.len() > 79;
        let mut reason_end = reason_raw.len().min(79);
        while reason_end > 0 && !reason_raw.is_char_boundary(reason_end) {
            reason_end -= 1;
        }
        let reason = &reason_raw[..reason_end];
        let messages =
            world.queue_punish_command(character_id, name, level, reason, reason_overflowed);
        if messages.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `/unpunish <name> <note id>` (`command.c:6541-6547` dispatch ->
    // `cmd_unpunish`, `command.c:2706-2731`), `CF_GOD`-only-gated,
    // full-word only (`cmdcmp`'s `minlen` is 8, the full length of
    // "unpunish", no abbreviation accepted). Parses an `isalpha`-only name
    // token, truncated to the 79-byte buffer cap, then `atoi`'s the
    // remaining text as the note id. Always returns a `default()` result
    // immediately; the real reply arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/punish` above).
    if lower == "unpunish" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller.flags.contains(CharacterFlags::GOD) {
            return ControlFlow::Break(None);
        }
        let (name, after_name) = take_legacy_alpha_name(rest.trim_start());
        let name = &name[..name.len().min(79)];
        let note_id = legacy_atoi_prefix(after_name.trim_start());
        let messages = world.queue_unpunish_command(character_id, name, note_id);
        if messages.is_empty() {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        }
        return ControlFlow::Break(Some(KeyringCommandResult {
            messages,
            ..Default::default()
        }));
    }

    // C `/exterminate <name>` (`command.c:9657-9662` dispatch ->
    // `cmd_exterminate`, `command.c:2639-2651`), `CF_STAFF|CF_GOD`-gated,
    // full-word only (`cmdcmp`'s `minlen` is 11, the full length of
    // "exterminate", no abbreviation accepted). Parses an `isalpha`-only
    // name token, truncated to the 79-byte buffer cap (C does no other
    // validation before handing off - see `World::queue_exterminate_
    // command`'s doc comment). Always returns a `default()` result
    // immediately; the real reply arrives later via `World::
    // queue_system_text` (same fire-and-forget async pattern as
    // `/punish`/`/unpunish` above) once `ugaris-server`'s `world_events.
    // rs::apply_exterminate_events` resolves the DB round trip.
    if lower == "exterminate" {
        let Some(caller) = world.characters.get(&character_id) else {
            return ControlFlow::Break(Some(KeyringCommandResult::default()));
        };
        if !caller
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
        {
            return ControlFlow::Break(None);
        }
        let (name, _remainder) = take_legacy_alpha_name(rest.trim_start());
        let name = &name[..name.len().min(79)];
        world.queue_exterminate_command(character_id, name);
        return ControlFlow::Break(Some(KeyringCommandResult::default()));
    }

    ControlFlow::Continue(())
}
