use super::*;

impl World {
    /// C `get_lq_char` (`lq.c:1935-1946`): the live character currently
    /// spawned for template `slot`, or `None` if it was never spawned, has
    /// already died/despawned, or was respawned into a different
    /// character (serial mismatch, same guard as
    /// [`Self::lq_admin_remove_npc_instance`]).
    pub(super) fn get_lq_char(&self, slot: usize) -> Option<CharacterId> {
        let npc = self.lq_npcs.iter().find(|npc| npc.slot == slot)?;
        let character_id = npc.character_id?;
        let character = self.characters.get(&character_id)?;
        (character.serial == npc.character_serial).then_some(character_id)
    }

    /// C `cmd_nremove` (`lq.c:1898-1932`): despawns (and cancels any
    /// pending respawn for) every NPC matching `<npcID|nick|all>`,
    /// without deleting the template itself (unlike `#npcdelete`).
    pub(super) fn lq_admin_cmd_nremove(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/nremove <npcID|nick|all>";
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
        let slots = self.resolve_lq_npc_slots(&nick, true);
        let count = slots
            .into_iter()
            .filter(|slot| self.lq_admin_remove_npc_instance(*slot))
            .count();
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Removed {count} NPCs."));
        }
    }

    /// C `cmd_nsay` (`lq.c:1953-1988`): makes every live instance of the
    /// matched template(s) `say` `text` (single already-tokenized word
    /// unless quoted, matching every other `cmd_*` text argument in this
    /// table). Only the nick/`all`-scan branch has C's ">10 matches,
    /// cancel" guard - it never triggers on the single-numeric-ID branch
    /// since that can only ever match one slot.
    pub(super) fn lq_admin_cmd_nsay(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/nsay <npcID|nick> <text:str>";
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
        let mut count = 0usize;
        for slot in self.resolve_lq_npc_slots(&nick, false) {
            let Some(target_id) = self.get_lq_char(slot) else {
                continue;
            };
            self.npc_say(target_id, &text);
            count += 1;
            if count > 10 {
                self.queue_lq_error(character_id, "Cancelled, too many NPCs.");
                break;
            }
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        }
    }

    /// C `cmd_nimmortal` (`lq.c:1996-2043`): sets/clears `CF_IMMORTAL|
    /// CF_NOATTACK` on every live instance of the matched template(s).
    pub(super) fn lq_admin_cmd_nimmortal(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/nimmortal <npcID|nick> <0|1>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(onoff) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing 0|1. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let mut count = 0usize;
        for slot in self.resolve_lq_npc_slots(&nick, false) {
            let Some(target_id) = self.get_lq_char(slot) else {
                continue;
            };
            let Some(target) = self.characters.get_mut(&target_id) else {
                continue;
            };
            if onoff != 0 {
                target
                    .flags
                    .insert(CharacterFlags::IMMORTAL | CharacterFlags::NOATTACK);
            } else {
                target
                    .flags
                    .remove(CharacterFlags::IMMORTAL | CharacterFlags::NOATTACK);
            }
            count += 1;
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(
                character_id,
                format!(
                    "Set immortal to {} on {} NPCs",
                    if onoff != 0 { "ON" } else { "OFF" },
                    count
                ),
            );
        }
    }

    /// C `cmd_nemote` (`lq.c:2045-2087`): same shape as
    /// [`Self::lq_admin_cmd_nsay`], but `emote` instead of `say`.
    pub(super) fn lq_admin_cmd_nemote(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/nemote <npcID|nick> <text:str>";
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
        let mut count = 0usize;
        for slot in self.resolve_lq_npc_slots(&nick, false) {
            let Some(target_id) = self.get_lq_char(slot) else {
                continue;
            };
            self.npc_emote(target_id, &text);
            count += 1;
            if count > 10 {
                self.queue_lq_error(character_id, "Cancelled, too many NPCs.");
                break;
            }
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        }
    }

    /// C `cmd_nattack` (`lq.c:2088-2137`): looks up `<player:str>` among
    /// every currently loaded character (`getfirst_char`/`getnext_char`,
    /// no `CF_PLAYER` filter, matching the exact case-insensitive-name
    /// idiom already established by
    /// `world::admin_flag::find_loaded_character_by_name`), then queues
    /// it as a `fight_driver` enemy on every live instance of the matched
    /// NPC template(s). The single-numeric-ID branch passes C's
    /// `hurtme=0` (bypasses `start_dist`/`char_dist` gating in the full
    /// `fight_driver_add_enemy` - not modeled here, see
    /// [`crate::character_driver::add_simple_baddy_enemy_unchecked`]'s own
    /// doc comment); the nick/`all`-scan branch passes `hurtme=1`. Neither
    pub(super) fn lq_admin_cmd_nattack(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/nattack <npcID|nick> <player:str>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(player_name) = reader.take_str() else {
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
        let Some(target_player_id) = self
            .characters
            .values()
            .find(|character| character.name.eq_ignore_ascii_case(&player_name))
            .map(|character| character.id)
        else {
            self.queue_lq_error(character_id, format!("Player {player_name} not found."));
            return;
        };
        let numeric_id_branch = {
            let numeric = legacy_atoi(&nick);
            numeric > 0 && (numeric as usize) < MAX_LQ_NPCS
        };
        let priority = if numeric_id_branch { 0 } else { 1 };
        let tick = self.tick.0 as i32;
        let mut count = 0usize;
        for slot in self.resolve_lq_npc_slots(&nick, false) {
            let Some(target_id) = self.get_lq_char(slot) else {
                continue;
            };
            let Some(target) = self.characters.get_mut(&target_id) else {
                continue;
            };
            let _ = add_simple_baddy_enemy_unchecked(target, target_player_id, priority, tick);
            count += 1;
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("{count} NPCs attacking."));
        }
    }

    /// C `cmd_doorlist` (`lq.c:2443-2452`). Unlike almost every other
    /// `cmd_*` handler in this table, C never validates `ptr` here (no
    /// "Trailing garbage" check) - any extra text after `/doorlist` is
    /// silently ignored, kept exactly.
    pub(super) fn lq_admin_cmd_doorlist(&mut self, character_id: CharacterId, _args: &str) {
        self.discover_lq_doors_once();
        let mut doors: Vec<LqDoorState> = self.lq_doors.clone();
        doors.sort_by_key(|door| door.slot);
        for door in doors {
            let Some(item) = self.items.get(&door.item_id) else {
                continue;
            };
            self.queue_system_text(
                character_id,
                format!(
                    "Door {}, Nick: {}, Pos: {},{}, Key: {}.",
                    door.slot, door.nick, item.x, item.y, door.key_id
                ),
            );
        }
    }

    /// C `cmd_doorlock` (`lq.c:2464-2503`), calling `update_lqdoor`
    /// (`lq.c:2454-2462`) inline via [`write_lq_door_key_id`].
    pub(super) fn lq_admin_cmd_doorlock(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/doorlock <doornick> <keyID:int> (keyID=0 for unlocked)";
        self.discover_lq_doors_once();
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(
                character_id,
                format!("Missing doornick. Usage is: {USAGE}."),
            );
            return;
        };
        let Some(key_id) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing keyID. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let key_id = (key_id as i32) as u32;

        let numeric = legacy_atoi(&nick);
        let slots: Vec<usize> = if numeric > 0
            && (numeric as usize) < MAX_LQ_DOORS
            && self
                .lq_doors
                .iter()
                .any(|door| door.slot == numeric as usize)
        {
            vec![numeric as usize]
        } else {
            self.lq_doors
                .iter()
                .filter(|door| door.nick.eq_ignore_ascii_case(&nick))
                .map(|door| door.slot)
                .collect()
        };

        let mut count = 0usize;
        for slot in slots {
            let Some(door) = self.lq_doors.iter_mut().find(|door| door.slot == slot) else {
                continue;
            };
            door.key_id = key_id;
            let item_id = door.item_id;
            if let Some(item) = self.items.get_mut(&item_id) {
                write_lq_door_key_id(item, key_id);
            }
            count += 1;
        }
        if count == 0 {
            self.queue_lq_error(character_id, "Door not found.");
        } else {
            self.queue_system_text(character_id, format!("Set key for {count} doors."));
        }
    }
}
