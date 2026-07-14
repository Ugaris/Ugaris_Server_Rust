use super::*;

impl World {
    /// C `cmd_npc` (`lq.c:357-425`).
    pub(super) fn lq_admin_cmd_npc(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str =
            "/npc <base:str> <level:int> <mode:chr> <respawn:int> [nick1:str] [nick2:str]";
        let mut reader = ArgReader::new(args);
        let Some(basename) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing base. Usage is: {USAGE}."));
            return;
        };
        let Some(level) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing level. Usage is: {USAGE}."));
            return;
        };
        let Some(mode) = reader.take_chr() else {
            self.queue_lq_error(character_id, format!("Missing mode. Usage is: {USAGE}."));
            return;
        };
        let Some(respawn) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing respawn. Usage is: {USAGE}."));
            return;
        };
        let mut nick0 = reader.take_str().unwrap_or_default();
        let mut nick1 = reader.take_str().unwrap_or_default();
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        nick0.truncate(39);
        nick1.truncate(39);

        let Some(caller) = self.characters.get(&character_id) else {
            return;
        };
        let (x, y, dir) = (caller.x, caller.y, caller.dir);

        if let Some(existing) = self.lq_npcs.iter().find(|npc| npc.x == x && npc.y == y) {
            let message = format!(
                " {} {} {} is already at this position",
                existing.slot, existing.nick[0], existing.nick[1]
            );
            self.queue_lq_error(character_id, message);
            return;
        }

        let Some(slot) = self.find_free_lq_npc_slot() else {
            self.queue_system_text(character_id, "No free NPC slots left.");
            return;
        };

        let mut basename = basename;
        basename.truncate(39);
        self.lq_npcs.push(LqNpcState {
            slot,
            basename,
            x,
            y,
            dir,
            level: level.clamp(0, i64::from(u16::MAX)) as u16,
            mode: (mode.to_ascii_lowercase() as u32) as u8,
            respawn_seconds: respawn.clamp(0, i64::from(u32::MAX)) as u32,
            name: String::new(),
            description: String::new(),
            nick: [nick0, nick1],
            character_id: None,
            character_serial: 0,
            sprite: 0,
            greeting: String::new(),
            trigger: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            reply: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            want_key_id: 0,
            reward_item: LqItemSpec::default(),
            reward_mark_id: 0,
            kill_mark_id: 0,
            hurt_mark_id: 0,
            carry_item: LqItemSpec::default(),
            carry_gold: 0,
        });
        self.lq_npcs.sort_by_key(|npc| npc.slot);

        self.queue_system_text(character_id, format!("Added NPC {slot}"));
    }

    /// C `cmd_killthrall` (`lq.c:482-503`): despawns every live
    /// `CDR_LQNPC` character whose `DRD_LQ_NPC_DATA.thrallname`
    /// case-insensitively matches `args` (a `#thrall`-spawned character
    /// only - template-spawned NPCs always have an empty `thrallname`).
    pub(super) fn lq_admin_cmd_killthrall(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/killthrall <thrallname:str>";
        let mut reader = ArgReader::new(args);
        let Some(name) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing name. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }

        let targets: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LQNPC
                    && matches!(
                        character.driver_state.as_ref(),
                        Some(CharacterDriverState::LqNpc(data))
                            if data.thrallname.eq_ignore_ascii_case(&name)
                    )
            })
            .map(|character| character.id)
            .collect();
        let count = targets.len();
        for target_id in targets {
            self.remove_character(target_id);
        }
        self.queue_system_text(character_id, format!("Killed {count} thralls."));
    }

    /// C `cmd_npcname` (`lq.c:512-551`).
    pub(super) fn lq_admin_cmd_npcname(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcname <npcID|nick> <name:str>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(name) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing name. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.name = name.clone();
            npc.name.truncate(39);
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set name of {count} NPCs"));
        }
    }

    /// C `cmd_npcgold` (`lq.c:553-597`).
    pub(super) fn lq_admin_cmd_npcgold(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcgold <npcID|nick> <gold:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(gold) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing gold. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if gold > 2000 {
            self.queue_lq_error(character_id, "Too much gold.");
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.carry_gold = gold.max(0) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set gold of {count} NPCs"));
        }
    }

    /// C `cmd_npcsprite` (`lq.c:599-643`). The `usage`/"Missing gold"
    /// error strings are a verbatim copy-paste of `cmd_npcgold`'s in the
    /// C source (`lq.c:602,609`) - kept exactly, not "fixed".
    pub(super) fn lq_admin_cmd_npcsprite(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcgold <npcID|nick> <sprite:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(sprite) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing gold. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if sprite == 313 || sprite == 305 || sprite == 58 {
            self.queue_system_text(
                character_id,
                "Sorry, Islena is not available for Life Quests.",
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.sprite = sprite as i32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set sprite of {count} NPCs"));
        }
    }

    /// C `cmd_npcdesc` (`lq.c:645-684`).
    pub(super) fn lq_admin_cmd_npcdesc(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcdesc <npcID|nick> <description:str>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(desc) = reader.take_str() else {
            self.queue_lq_error(
                character_id,
                format!("Missing description. Usage is: {USAGE}."),
            );
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.description = desc.clone();
            npc.description.truncate(159);
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set description of {count} NPCs"));
        }
    }

    /// C `cmd_npcgreet` (`lq.c:686-725`).
    pub(super) fn lq_admin_cmd_npcgreet(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcgreet <npcID|nick> <text:str>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(text) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing text. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.greeting = text.clone();
            npc.greeting.truncate(255);
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set greeting of {count} NPCs"));
        }
    }

    /// C `cmd_npckillmark` (`lq.c:727-771`).
    pub(super) fn lq_admin_cmd_npckillmark(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npckillmark <npcID|nick> <mark:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(mark) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing mark. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if mark < 1 || mark >= MAXLQMARK as i64 {
            self.queue_system_text(
                character_id,
                format!("Mark is out of bounds (1-{})", MAXLQMARK - 1),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.kill_mark_id = (mark as i32) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set killmark of {count} NPCs"));
        }
    }

    /// C `cmd_npchurtmark` (`lq.c:773-817`).
    pub(super) fn lq_admin_cmd_npchurtmark(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npchurtmark <npcID|nick> <mark:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(mark) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing mark. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if mark < 1 || mark >= MAXLQMARK as i64 {
            self.queue_system_text(
                character_id,
                format!("Mark is out of bounds (1-{})", MAXLQMARK - 1),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.hurt_mark_id = (mark as i32) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set hurtmark of {count} NPCs"));
        }
    }

    /// C `cmd_npcmodlevel` (`lq.c:819-878`).
    pub(super) fn lq_admin_cmd_npcmodlevel(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcmodlevel <npcID|nick|all> <mod:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(modifier) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing mod. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, true);
        let mut clamp_messages = Vec::new();
        let mut count = 0usize;
        for slot in slots {
            let Some(npc) = self.lq_npcs.iter_mut().find(|npc| npc.slot == slot) else {
                continue;
            };
            count += 1;
            let mut new_level = i64::from(npc.level) + modifier;
            if new_level < 1 {
                new_level = 1;
                clamp_messages.push(format!(
                    "NPC {} ({} {} {}) set to level 1 to avoid negative level.",
                    slot, npc.name, npc.nick[0], npc.nick[1]
                ));
            }
            if new_level > 200 {
                new_level = 200;
                clamp_messages.push(format!(
                    "NPC {} ({} {} {}) set to level 200 to avoid too high levels.",
                    slot, npc.name, npc.nick[0], npc.nick[1]
                ));
            }
            npc.level = new_level as u16;
        }
        for message in clamp_messages {
            self.queue_system_text(character_id, message);
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Changed level of {count} NPCs"));
        }
    }

    /// C `cmd_npcrespawn` (`lq.c:880-919`).
    pub(super) fn lq_admin_cmd_npcrespawn(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcrespawn <npcID|nick|all> <mod:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(modifier) = reader.take_int() else {
            self.queue_lq_error(
                character_id,
                format!("Missing respawn time. Usage is: {USAGE}."),
            );
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, true);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.respawn_seconds = modifier.clamp(0, i64::from(u32::MAX)) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(
                character_id,
                format!("Changed respawn time of {count} NPCs to {modifier}"),
            );
        }
    }

    /// C `cmd_npcpos` (`lq.c:921-982`).
    pub(super) fn lq_admin_cmd_npcpos(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcpos <npcID|nick> [x:int] [y:int]";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let mut x = reader.take_int().unwrap_or(0);
        let mut y = reader.take_int().unwrap_or(0);
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }

        let Some(caller) = self.characters.get(&character_id) else {
            return;
        };
        let (caller_x, caller_y, caller_dir) = (caller.x, caller.y, caller.dir);
        if x == 0 && y == 0 {
            x = i64::from(caller_x);
            y = i64::from(caller_y);
        }
        if x < 1
            || x >= i64::from(MAX_MAP as i32) - 1
            || y < 1
            || y >= i64::from(MAX_MAP as i32) - 1
        {
            self.queue_system_text(character_id, format!("Position {x},{y} is out of bounds."));
            return;
        }

        let numeric = legacy_atoi(&nick);
        let mut target_slot = if numeric >= 1
            && (numeric as usize) < MAX_LQ_NPCS
            && self.lq_npcs.iter().any(|npc| npc.slot == numeric as usize)
        {
            Some(numeric as usize)
        } else {
            None
        };
        if target_slot.is_none() {
            for npc in &self.lq_npcs {
                if npc.nick[0].eq_ignore_ascii_case(&nick)
                    || npc.nick[1].eq_ignore_ascii_case(&nick)
                {
                    if target_slot.is_some() {
                        self.queue_lq_error(
                            character_id,
                            "Cannot set the same position for multiple NPCs.",
                        );
                        return;
                    }
                    target_slot = Some(npc.slot);
                }
            }
        }
        let Some(target_slot) = target_slot else {
            self.queue_lq_error(character_id, "NPC not found.");
            return;
        };

        if let Some(conflict) = self
            .lq_npcs
            .iter()
            .find(|npc| npc.slot != target_slot && i64::from(npc.x) == x && i64::from(npc.y) == y)
        {
            let message = format!(
                " {} {} {} is already at this position",
                conflict.slot, conflict.nick[0], conflict.nick[1]
            );
            self.queue_lq_error(character_id, message);
            return;
        }

        if let Some(npc) = self.lq_npcs.iter_mut().find(|npc| npc.slot == target_slot) {
            npc.x = x as u16;
            npc.y = y as u16;
            npc.dir = caller_dir;
        }
        self.queue_system_text(character_id, format!("Set position to {x},{y}."));
    }

    /// C `cmd_npcreply` (`lq.c:984-1039`).
    pub(super) fn lq_admin_cmd_npcreply(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcreply <npcID|nick> <nr:int> <trigger:str> <reply:str>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(nr) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing nr. Usage is: {USAGE}."));
            return;
        };
        let Some(trigger) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing trigger. Usage is: {USAGE}."));
            return;
        };
        let Some(reply) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing reply. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let idx = nr - 1;
        if !(0..5).contains(&idx) {
            // C typo kept verbatim: "Nr %d it out of bounds." (`lq.c:1012`).
            self.queue_system_text(character_id, format!("Nr {nr} it out of bounds."));
            return;
        }
        let idx = idx as usize;
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            let mut trigger = trigger.clone();
            trigger.truncate(39);
            let mut reply = reply.clone();
            reply.truncate(255);
            npc.trigger[idx] = trigger;
            npc.reply[idx] = reply;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set trigger/reply of {count} NPCs"));
        }
    }

    /// C `cmd_npcwantitem` (`lq.c:1041-1080`).
    pub(super) fn lq_admin_cmd_npcwantitem(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcwantitem <npcID|nick> <ID:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(id) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing ID. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.want_key_id = (id as i32) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set wantitem of {count} NPCs"));
        }
    }

    /// C `cmd_npcitem` (`lq.c:1167-1202`).
    pub(super) fn lq_admin_cmd_npcitem(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str =
            "/npcitem <npcID|nick> <base:str> [keyID:int] [name:str] [description:str]";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(item) = self.lq_admin_parse_item(character_id, &mut reader, USAGE) else {
            return;
        };
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| npc.carry_item = item.clone());
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set item of {count} NPCs"));
        }
    }

    /// C `cmd_npcrewarditem` (`lq.c:1204-1239`). C's own success message
    /// is a verbatim copy-paste of `cmd_npcitem`'s ("Set item of %d
    /// NPCs", not "Set reward item...") - kept exactly.
    pub(super) fn lq_admin_cmd_npcrewarditem(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str =
            "/npcrewarditem <npcID|nick> <base:str> [keyID:int] [name:str] [description:str]";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(item) = self.lq_admin_parse_item(character_id, &mut reader, USAGE) else {
            return;
        };
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| npc.reward_item = item.clone());
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set item of {count} NPCs"));
        }
    }

    /// C `show_npc` (`lq.c:1082-1128`).
    pub(super) fn lq_admin_show_npc(&mut self, character_id: CharacterId, slot: usize) {
        let Some(npc) = self.lq_npcs.iter().find(|npc| npc.slot == slot) else {
            return;
        };
        let npc = npc.clone();
        self.queue_system_text(character_id, format!("Base: {}", npc.basename));
        self.queue_system_text(
            character_id,
            format!("Nicks: {}/{}", npc.nick[0], npc.nick[1]),
        );
        self.queue_system_text(character_id, format!("Level: {}", npc.level));
        self.queue_system_text(character_id, format!("Mode: {}", npc.mode as char));
        self.queue_system_text(character_id, format!("Respawn: {}", npc.respawn_seconds));
        if !npc.name.is_empty() {
            self.queue_system_text(character_id, format!("Name: {}", npc.name));
        }
        if !npc.description.is_empty() {
            self.queue_system_text(character_id, format!("Desc: {}", npc.description));
        }
        if !npc.greeting.is_empty() {
            self.queue_system_text(character_id, format!("Greeting: {}", npc.greeting));
        }
        for i in 0..5 {
            if !npc.trigger[i].is_empty() {
                self.queue_system_text(
                    character_id,
                    format!("Trigger/Reply {}: {}/{}", i, npc.trigger[i], npc.reply[i]),
                );
            }
        }
        if npc.carry_gold != 0 {
            self.queue_system_text(
                character_id,
                format!("Gold: {:.2}G", f64::from(npc.carry_gold) / 100.0),
            );
        }
        if !npc.carry_item.base.is_empty() {
            self.queue_system_text(
                character_id,
                format!(
                    "Carry Item: {} ID: {}",
                    npc.carry_item.base, npc.carry_item.key_id
                ),
            );
        }
        if npc.want_key_id != 0 {
            self.queue_system_text(character_id, format!("Wants ID: {}", npc.want_key_id));
        }
        if !npc.reward_item.base.is_empty() {
            self.queue_system_text(
                character_id,
                format!(
                    "Reward Item: {} ID: {}",
                    npc.reward_item.base, npc.reward_item.key_id
                ),
            );
        }
        if npc.hurt_mark_id != 0 {
            let idx = npc.hurt_mark_id as usize;
            self.queue_system_text(
                character_id,
                format!(
                    "Hurtmark ID: {} ({}), {} exp",
                    self.lq_data.reward_desc[idx], npc.hurt_mark_id, self.lq_data.reward[idx]
                ),
            );
        }
        if npc.kill_mark_id != 0 {
            let idx = npc.kill_mark_id as usize;
            self.queue_system_text(
                character_id,
                format!(
                    "Killmark ID: {} ({}), {} exp",
                    self.lq_data.reward_desc[idx], npc.kill_mark_id, self.lq_data.reward[idx]
                ),
            );
        }
    }

    /// C `cmd_npcshow` (`lq.c:1130-1165`).
    pub(super) fn lq_admin_cmd_npcshow(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcshow <npcID|nick>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = slots.len();
        for slot in slots {
            self.lq_admin_show_npc(character_id, slot);
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Showed {count} NPCs"));
        }
    }

    /// C `cmd_npclist` (`lq.c:1241-1274`).
    pub(super) fn lq_admin_cmd_npclist(&mut self, character_id: CharacterId, args: &str) {
        let mut reader = ArgReader::new(args);
        let mut nick = reader.take_str().unwrap_or_default();
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                "Trailing garbage. Usage is: /npclist <nick|start>.",
            );
            return;
        }
        let start = legacy_atoi(&nick);
        if start != 0 {
            nick.clear();
        }
        let start_slot = start.max(1) as usize;

        let mut slots: Vec<usize> = self.lq_npcs.iter().map(|npc| npc.slot).collect();
        slots.sort_unstable();

        let mut lines = Vec::new();
        let mut count = 0usize;
        for slot in slots {
            if slot < start_slot {
                continue;
            }
            let Some(npc) = self.lq_npcs.iter().find(|npc| npc.slot == slot) else {
                continue;
            };
            if !nick.is_empty()
                && !npc.nick[0].eq_ignore_ascii_case(&nick)
                && !npc.nick[1].eq_ignore_ascii_case(&nick)
            {
                continue;
            }
            lines.push(format!(
                "NPC {:3}: base {}, level {}, nicks: {} {}, pos: {},{}",
                slot, npc.basename, npc.level, npc.nick[0], npc.nick[1], npc.x, npc.y
            ));
            count += 1;
            if count > 99 {
                break;
            }
        }
        for line in lines {
            self.queue_system_text(character_id, line);
        }
        self.queue_system_text(
            character_id,
            format!(
                "{} of {} NPCs ({}%)",
                count,
                MAX_LQ_NPCS - 1,
                100 * count / (MAX_LQ_NPCS - 1)
            ),
        );
    }

    /// C `remove_npc` (`lq.c:1839-1861`), called from both `cmd_npcdel`
    /// (return value ignored - the caller counts template deletions
    /// unconditionally) and `cmd_nremove` (return value is the count).
    /// Returns C's own `flag`/`1`/`0`: `true` if a live instance was
    /// actually destroyed, or if there was no live instance but a
    /// scheduled respawn was pending (and got cancelled); `false` only
    /// when there was neither a live instance nor a pending respawn.
    pub(super) fn lq_admin_remove_npc_instance(&mut self, slot: usize) -> bool {
        let had_scheduled_respawn = self
            .lq_npc_respawns
            .iter()
            .any(|(s, due_tick)| *s == slot && *due_tick > self.tick.0);
        self.lq_npc_respawns.retain(|(s, _)| *s != slot);
        let Some(npc) = self.lq_npcs.iter().find(|npc| npc.slot == slot) else {
            return false;
        };
        let Some(character_id) = npc.character_id else {
            return had_scheduled_respawn;
        };
        let expected_serial = npc.character_serial;
        let live = self
            .characters
            .get(&character_id)
            .is_some_and(|character| character.serial == expected_serial);
        if !live {
            return had_scheduled_respawn;
        }
        self.remove_character(character_id);
        if let Some(npc) = self.lq_npcs.iter_mut().find(|npc| npc.slot == slot) {
            npc.character_id = None;
            npc.character_serial = 0;
        }
        true
    }

    /// C `cmd_npcdel` (`lq.c:1276-1312`).
    pub(super) fn lq_admin_cmd_npcdel(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcdel <npcID|nick>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let mut count = 0usize;
        for slot in slots {
            self.lq_admin_remove_npc_instance(slot);
            self.lq_npcs.retain(|npc| npc.slot != slot);
            count += 1;
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Deleted {count} NPCs."));
        }
    }
}
