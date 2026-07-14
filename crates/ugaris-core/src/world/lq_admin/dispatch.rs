use super::*;

/// Result of [`World::try_dispatch_lq_nspawn`] - unlike every other
/// command in this table, `#nspawn`'s underlying C `spawn_npc`
/// (`lq.c:1724-1822`) needs a fresh character (`create_char`/`drop_char`),
/// which only `ugaris-server`'s `ZoneLoader`/`ServerRuntime` can provide.
/// This type lets the pure-`World` half (area/permission gate, arg
/// parsing, and the `already there`/`still respawning` eligibility checks
/// `spawn_npc` itself performs before ever touching a character) stay
/// here, while the caller performs the actual instantiation via
/// `ugaris-server::spawns::spawn_lq_npc_character` and reports the result
/// back through [`World::report_lq_nspawn_result`].
pub enum LqNspawnDispatch {
    /// Not `#nspawn`/`/nspawn`, or the caller lacked area/permission -
    /// the caller should try other command dispatch tables next.
    NotMatched,
    /// Command matched but failed argument validation; the usage error is
    /// already queued, nothing more to do.
    Rejected,
    /// Command matched and parsed; these NPC-template slots are eligible
    /// to spawn right now (C's `spawn_npc` per-slot checks already
    /// passed) - possibly empty, in which case the caller should still
    /// call `report_lq_nspawn_result` with `count = 0` to get the
    /// "NPC not found." message.
    Requests(Vec<LqNpcSpawnRequest>),
}

/// Result of [`World::try_dispatch_lq_thrall`] - same split rationale as
/// [`LqNspawnDispatch`]: `#thrall`'s underlying `spawn_npc(.., isthrall=1,
/// ..)` call needs a fresh character, only `ugaris-server` can provide.
pub enum LqThrallDispatch {
    /// Not `#thrall`/`/thrall`, or the caller lacked area/permission - the
    /// caller should try other command dispatch tables next.
    NotMatched,
    /// Command matched but failed argument validation, or the named
    /// template doesn't exist; the message is already queued.
    Rejected,
    /// Command matched and parsed: one [`LqNpcSpawnRequest`] per thrall to
    /// spawn (`count`, already clamped/validated), each with its own
    /// independently-rolled drop position.
    Requests(Vec<LqNpcSpawnRequest>),
}

impl World {
    /// C `cmd_nspawn`'s pure-`World` half (`lq.c:1863-1896`) plus
    /// `spawn_npc`'s own `already there`/`still respawning` guard
    /// (`lq.c:1734-1741`) - see [`LqNspawnDispatch`]'s own doc comment for
    /// why the actual character creation is deferred to the caller.
    pub fn try_dispatch_lq_nspawn(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        command: &str,
    ) -> LqNspawnDispatch {
        if area_id != 20 && area_id != 35 {
            return LqNspawnDispatch::NotMatched;
        }
        let trimmed = command.trim_start();
        let Some(rest) = trimmed
            .strip_prefix('#')
            .or_else(|| trimmed.strip_prefix('/'))
        else {
            return LqNspawnDispatch::NotMatched;
        };
        let mut reader = ArgReader::new(rest);
        let Some(word) = reader.take_str() else {
            return LqNspawnDispatch::NotMatched;
        };
        if !cmd_word_matches(&word, "nspawn", 5) {
            return LqNspawnDispatch::NotMatched;
        }
        let Some(flags) = self
            .characters
            .get(&character_id)
            .map(|character| character.flags)
        else {
            return LqNspawnDispatch::NotMatched;
        };
        if !flags.intersects(CharacterFlags::GOD | CharacterFlags::LQMASTER) {
            return LqNspawnDispatch::NotMatched;
        }

        const USAGE: &str = "/nspawn <npcID|nick|all>";
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return LqNspawnDispatch::Rejected;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return LqNspawnDispatch::Rejected;
        }

        let now = self.tick.0;
        let requests = self
            .resolve_lq_npc_slots(&nick, true)
            .into_iter()
            .filter(|slot| self.lq_nspawn_slot_eligible(*slot, now))
            .filter_map(|slot| self.lq_npcs.iter().find(|npc| npc.slot == slot))
            .map(build_lq_npc_spawn_request)
            .collect();
        LqNspawnDispatch::Requests(requests)
    }

    /// C `spawn_npc`'s own early-out guard (`lq.c:1734-1741`, the
    /// `isthrall == 0` branch only - `#nspawn` never spawns thralls):
    /// skip a slot whose live instance is still there, or whose scheduled
    /// respawn cooldown (`#npcrespawn`/a previous death) hasn't elapsed
    /// yet.
    pub(super) fn lq_nspawn_slot_eligible(&self, slot: usize, now: u64) -> bool {
        if self.get_lq_char(slot).is_some() {
            return false;
        }
        !self
            .lq_npc_respawns
            .iter()
            .any(|(s, due_tick)| *s == slot && *due_tick >= now)
    }

    /// Reports the actual spawn count (`ugaris-server` already attempted
    /// every [`LqNspawnDispatch::Requests`] candidate via `ZoneLoader`) -
    /// C `cmd_nspawn`'s trailing `if (!cnt) ... else ...` (`lq.c:1892-
    /// 1896`).
    pub fn report_lq_nspawn_result(&mut self, character_id: CharacterId, count: usize) {
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Spawned {count} NPCs."));
        }
    }

    /// C `cmd_thrall`'s pure-`World` half (`lq.c:427-475`): like
    /// `#nspawn`, `spawn_npc`'s `isthrall` branch needs a fresh character,
    /// so the caller performs the actual instantiation (once per
    /// `LqNpcSpawnRequest` in [`LqThrallDispatch::Requests`]) via
    /// `ugaris-server::spawns::spawn_lq_npc_character`; no result report
    /// call is needed afterward (unlike `#nspawn`) - C's `cmd_thrall`
    /// never inspects `spawn_npc`'s return value at all.
    pub fn try_dispatch_lq_thrall(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        command: &str,
    ) -> LqThrallDispatch {
        if area_id != 20 && area_id != 35 {
            return LqThrallDispatch::NotMatched;
        }
        let trimmed = command.trim_start();
        let Some(rest) = trimmed
            .strip_prefix('#')
            .or_else(|| trimmed.strip_prefix('/'))
        else {
            return LqThrallDispatch::NotMatched;
        };
        let mut reader = ArgReader::new(rest);
        let Some(word) = reader.take_str() else {
            return LqThrallDispatch::NotMatched;
        };
        if !cmd_word_matches(&word, "thrall", 3) {
            return LqThrallDispatch::NotMatched;
        }
        let Some(flags) = self
            .characters
            .get(&character_id)
            .map(|character| character.flags)
        else {
            return LqThrallDispatch::NotMatched;
        };
        if !flags.intersects(CharacterFlags::GOD | CharacterFlags::LQMASTER) {
            return LqThrallDispatch::NotMatched;
        }

        const USAGE: &str = "/thrall <nick|ID> <count:int> [thrallname:str]";
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing nick. Usage is: {USAGE}."));
            return LqThrallDispatch::Rejected;
        };
        let Some(count) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing count. Usage is: {USAGE}."));
            return LqThrallDispatch::Rejected;
        };
        let mut thrall_name = reader.take_str().unwrap_or_default();
        thrall_name.truncate(39);
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return LqThrallDispatch::Rejected;
        }

        // C `if (MAXCHARS - used_chars < 150) { ... return; }`
        // (`lq.c:448-451`) is not ported - `World`'s character store is an
        // unbounded `HashMap`, not a fixed-capacity array (see e.g.
        // `commands_admin/character.rs`'s `/memstats` doc comment for the
        // established precedent).
        if count > 20 {
            self.queue_system_text(
                character_id,
                "Sorry, maximum number of NPCs you can spawn in one call is 20.",
            );
            return LqThrallDispatch::Rejected;
        }

        // C `n = atoi(nick); if (n>0 && n<MAXLQNPC) { ... } else { for
        // (n=1..MAXLQNPC) if nick[0]/nick[1] match break; }` (`lq.c:454-
        // 469`) - unlike `#nspawn`'s `resolve_lq_npc_slots(.., true)`
        // (which collects every match plus `"all"`), `cmd_thrall` stops
        // at its first (lowest-slot) match and never supports `"all"`.
        let Some(slot) = self.resolve_lq_npc_slots(&nick, false).into_iter().next() else {
            self.queue_system_text(character_id, "Template not found");
            return LqThrallDispatch::Rejected;
        };

        let Some(caller) = self.characters.get(&character_id) else {
            return LqThrallDispatch::Rejected;
        };
        let (caller_x, caller_y) = (i32::from(caller.x), i32::from(caller.y));
        // C `dx2offset(ch[cn].dir, &dx, &dy, NULL);` (`lq.c:470`).
        let (dx, dy) = Direction::try_from(caller.dir)
            .map(Direction::delta)
            .unwrap_or((0, 0));
        let base_x = caller_x + i32::from(dx) * 3;
        let base_y = caller_y + i32::from(dy) * 3;

        let Some(npc) = self.lq_npcs.iter().find(|npc| npc.slot == slot) else {
            self.queue_system_text(character_id, "Template not found");
            return LqThrallDispatch::Rejected;
        };
        let template = build_lq_npc_spawn_request(npc);

        let mut requests = Vec::new();
        for _ in 0..count.max(0) {
            // C `spawn_npc`'s own `isthrall` position roll: `ch[cn].tmpx =
            // tx + 2 - RANDOM(4); ch[cn].tmpy = ty + 2 - RANDOM(4);`
            // (`lq.c:1772-1773`) - rolled independently per spawned
            // thrall, same as C's per-call `spawn_npc` invocation.
            let roll_x = legacy_random_below_from_seed(&mut self.legacy_random_seed, 4) as i32;
            let roll_y = legacy_random_below_from_seed(&mut self.legacy_random_seed, 4) as i32;
            let mut request = template.clone();
            request.x = clamp_world_coordinate(base_x + 2 - roll_x);
            request.y = clamp_world_coordinate(base_y + 2 - roll_y);
            request.is_thrall = true;
            request.thrall_name = thrall_name.clone();
            requests.push(request);
        }
        LqThrallDispatch::Requests(requests)
    }
}
