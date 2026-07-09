//! `CDR_LQPARSER`'s `#questsave`/`#questdelete`/`#questload` trio
//! (`cmd_questsave`/`cmd_questdel`/`cmd_questload`, `lq.c:1346-1502`,
//! dispatched at `lq.c:2631-2645`) - the last remaining subcommand family
//! in this table (see `world::lq_admin`'s own doc comment for the rest of
//! it). All three need real file I/O (`open`/`read`/`write`/`unlink` on
//! `quest/<name>.qst`), which only `ugaris-server` can perform - the same
//! `ZoneLoader`-needing split as `LqNspawnDispatch`/`LqThrallDispatch`,
//! except here the external resource is a filesystem path rather than a
//! fresh character. The pure `World` half in this module performs every
//! C-visible check that doesn't need the file's contents (area/prefix/
//! permission gate, argument parsing, the per-character `isalpha` name
//! validation loop, trailing-garbage rejection) and hands back the
//! validated `name`/`password` for `ugaris-server` to act on; `World`
//! itself has no filesystem access (matching every other I/O boundary in
//! this codebase - see `AGENTS.md`'s module layout rules).
//!
//! [`LqQuestSnapshot`] is this port's replacement for C's raw
//! `lq_data`+`lq_npc[]`+`lq_door[]` byte dump (`write(handle, &lq_data,
//! sizeof(lq_data)); write(handle, lq_npc, sizeof(lq_npc)); write(handle,
//! lq_door, sizeof(lq_door));`, `lq.c:1389-1391`) - serialized as JSON by
//! `ugaris-server` instead of C's fixed-size struct layout, since there is
//! no cross-version/cross-process binary-compat requirement to preserve
//! for a save format this port owns end to end (unlike the legacy client
//! wire protocol or the `characters` DB table). [`LqQuestFile`] is the
//! on-disk envelope (`password` alongside the snapshot, mirroring C's own
//! `write(handle, password, 40)` file header).
//!
//! C quirk preserved: `cmd_questload`'s `init_done = 0` (`lq.c:1498`)
//! defers the actual per-door `keyID` restore to the *next* `lq_ticker`
//! rescan tick, because C's `lq_door[]` "identity" is purely a live-map
//! scan-order index (`m`, incremented once per `IDR_DOOR` item with
//! `drdata[10]` set, in map-item-index order) rather than a stable stored
//! key - the loaded `lq_door[]` array's `keyID` fields just sit unused
//! until the next scan re-derives `nick`/`in` for the same slot indices
//! and calls `update_lqdoor(m)`, which pushes whatever `keyID` is *already
//! sitting in that slot* (the just-loaded value) onto the live door item.
//! [`World::apply_lq_quest_snapshot`] performs the equivalent rescan
//! (`World::discover_lq_doors_once`, forced via a fresh
//! `lq_doors_initialized = false`) synchronously instead of waiting a
//! tick - there is no reason to delay a GM command's own visible effect,
//! and the observable end state (each rescanned door slot's `key_id`
//! restored from the file, matched by slot index, and written onto the
//! live item's `driver_data`) is identical either way.

use super::lq_admin::{cmd_word_matches, ArgReader};
use super::*;

/// This port's save-file payload - see the module doc comment for why
/// this is JSON instead of C's raw `lq_data`+`lq_npc[]`+`lq_door[]` byte
/// layout.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LqQuestSnapshot {
    pub data: LqData,
    pub npcs: Vec<LqNpcState>,
    pub doors: Vec<LqDoorState>,
}

/// The on-disk envelope `ugaris-server` reads/writes at `quest/<name>.qst`
/// - C's `write(handle, password, 40)` file header plus the payload C
/// splits across the next three `write` calls (`lq.c:1388-1391`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LqQuestFile {
    pub password: String,
    pub snapshot: LqQuestSnapshot,
}

/// Result of [`World::try_dispatch_lq_quest_file`] - see the module doc
/// comment for why the actual file I/O is deferred to the caller.
pub enum LqQuestFileDispatch {
    /// Not `#questsave`/`#questdelete`/`#questload`, or the caller lacked
    /// area/permission - the caller should try other command dispatch
    /// tables next.
    NotMatched,
    /// Command matched but failed argument validation (missing name,
    /// illegal character, or trailing garbage); the usage error is
    /// already queued, nothing more to do.
    Rejected,
    /// C `cmd_questsave` (`lq.c:1346-1395`): create-or-overwrite
    /// `quest/<name>.qst` with the current LQ state, subject to the
    /// existing file's stored password (if any) matching `password`.
    Save { name: String, password: String },
    /// C `cmd_questdel` (`lq.c:1397-1442`, dispatched as `#questdelete`):
    /// delete `quest/<name>.qst`, subject to the same password check.
    Delete { name: String, password: String },
    /// C `cmd_questload` (`lq.c:1444-1502`): replace the current LQ state
    /// with `quest/<name>.qst`'s contents, subject to the same password
    /// check.
    Load { name: String, password: String },
}

/// C's per-subcommand `usage` string (`lq.c:1350`/`1401`/`1448`) - note
/// `cmd_questdel`'s usage text says `/questdel`, not `/questdelete`, even
/// though the dispatch keyword (`cmdcmp(ptr, "questdelete", 8)`,
/// `lq.c:2635`) is `questdelete`. Preserved verbatim, not a typo fix.
enum QuestFileKind {
    Save,
    Delete,
    Load,
}

impl QuestFileKind {
    fn usage(&self) -> &'static str {
        match self {
            QuestFileKind::Save => "/questsave <name> [password]",
            QuestFileKind::Delete => "/questdel <name> [password]",
            QuestFileKind::Load => "/questload <name> [password]",
        }
    }
}

impl World {
    /// C `special_driver`'s `#questsave`/`#questdelete`/`#questload`
    /// branch (`lq.c:2631-2645`) plus each `cmd_*` handler's shared
    /// argument-parsing prologue (`get_str` name/password, trailing-
    /// garbage check, per-character `isalpha` name validation,
    /// `lq.c:1352-1369`/`1403-1420`/`1450-1467` - identical across all
    /// three). Actual file I/O is left to the caller - see the module doc
    /// comment.
    pub fn try_dispatch_lq_quest_file(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        command: &str,
    ) -> LqQuestFileDispatch {
        if area_id != 20 && area_id != 35 {
            return LqQuestFileDispatch::NotMatched;
        }
        let trimmed = command.trim_start();
        let Some(rest) = trimmed
            .strip_prefix('#')
            .or_else(|| trimmed.strip_prefix('/'))
        else {
            return LqQuestFileDispatch::NotMatched;
        };
        let mut reader = ArgReader::new(rest);
        let Some(word) = reader.take_str() else {
            return LqQuestFileDispatch::NotMatched;
        };
        let kind = if cmd_word_matches(&word, "questsave", 8) {
            QuestFileKind::Save
        } else if cmd_word_matches(&word, "questdelete", 8) {
            QuestFileKind::Delete
        } else if cmd_word_matches(&word, "questload", 8) {
            QuestFileKind::Load
        } else {
            return LqQuestFileDispatch::NotMatched;
        };
        let Some(flags) = self
            .characters
            .get(&character_id)
            .map(|character| character.flags)
        else {
            return LqQuestFileDispatch::NotMatched;
        };
        if !flags.intersects(CharacterFlags::GOD | CharacterFlags::LQMASTER) {
            return LqQuestFileDispatch::NotMatched;
        }

        let usage = kind.usage();
        let Some(name) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing name. Usage is: {usage}."));
            return LqQuestFileDispatch::Rejected;
        };
        let password = reader.take_str().unwrap_or_default();
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {usage}."),
            );
            return LqQuestFileDispatch::Rejected;
        }
        if let Some(bad_char) = name.chars().find(|ch| !ch.is_ascii_alphabetic()) {
            self.queue_lq_error(
                character_id,
                format!("Name contains illegal character {bad_char}."),
            );
            return LqQuestFileDispatch::Rejected;
        }

        match kind {
            QuestFileKind::Save => LqQuestFileDispatch::Save { name, password },
            QuestFileKind::Delete => LqQuestFileDispatch::Delete { name, password },
            QuestFileKind::Load => LqQuestFileDispatch::Load { name, password },
        }
    }

    /// C `cmd_questsave`'s in-memory payload build (`lq.c:1389-1391`):
    /// snapshot the current LQ admin state for `ugaris-server` to write
    /// to disk.
    pub fn lq_quest_snapshot(&self) -> LqQuestSnapshot {
        LqQuestSnapshot {
            data: self.lq_data.clone(),
            npcs: self.lq_npcs.clone(),
            doors: self.lq_doors.clone(),
        }
    }

    /// C `cmd_questload`'s in-memory payload apply (`lq.c:1493-1499`):
    /// replace the current LQ admin state with `snapshot`'s contents. See
    /// the module doc comment for the door-rescan timing simplification.
    pub fn apply_lq_quest_snapshot(&mut self, snapshot: LqQuestSnapshot) {
        self.lq_data = snapshot.data;
        // C `lq_data.open = 0;` (`lq.c:1499`) - always cleared after load,
        // regardless of what the saved file's own `open` flag was.
        self.lq_data.open = false;
        self.lq_npcs = snapshot.npcs;

        let loaded_key_ids: std::collections::HashMap<usize, u32> = snapshot
            .doors
            .iter()
            .map(|door| (door.slot, door.key_id))
            .collect();
        // C `init_done = 0` (`lq.c:1498`): force the next door scan to
        // re-run, matching the live map instead of trusting the loaded
        // `lq_door[]` positions/item references (which may be stale after
        // a map edit between save and load).
        self.lq_doors.clear();
        self.lq_doors_initialized = false;
        self.discover_lq_doors_once();
        for door in &mut self.lq_doors {
            if let Some(&key_id) = loaded_key_ids.get(&door.slot) {
                door.key_id = key_id;
            }
        }
        let restored: Vec<(ItemId, u32)> = self
            .lq_doors
            .iter()
            .filter_map(|door| {
                loaded_key_ids
                    .contains_key(&door.slot)
                    .then_some((door.item_id, door.key_id))
            })
            .collect();
        for (item_id, key_id) in restored {
            if let Some(item) = self.items.get_mut(&item_id) {
                write_lq_door_key_id(item, key_id);
            }
        }
    }
}
