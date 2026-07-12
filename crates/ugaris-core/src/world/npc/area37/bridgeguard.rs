//! Bridge Guard NPCs (`CDR_BRIDGEGUARD`), the pair standing watch over the
//! bridge into Arkhata proper (`zones/37/Bridge_Guards.chr`, `group=2`).
//!
//! Ports `src/area/37/arkhata.c::bridgeguard_driver` (`:1179-1277`). Unlike
//! every other `arkhata.c` NPC ported so far, this driver has real combat
//! self-defense (`fight_driver_add_enemy`/`fight_driver_update`/`fight_
//! driver_attack_visible`/`fight_driver_follow_invisible`/`regenerate_
//! driver`/`spell_self_driver`) - ported the same way `world::npc::area25::
//! warpfighter` reproduces `standard_message_driver`'s semantics directly
//! (`CDR_BRIDGEGUARD` is not a `CharacterDriverState::SimpleBaddy`, so the
//! shared `process_simple_baddy_messages` machinery cannot see its `NT_CHAR`/
//! `NT_NPC` messages).
//!
//! `ch_died_driver`'s `CDR_BRIDGEGUARD` case (`arkhata.c:4651-4652`) is a
//! bare `return 1;` - no death hook exists at all (bridge guards are
//! genuinely killable and simply disappear like any other NPC corpse, no
//! `charlog`/state write on death).
//!
//! Deviations/gaps (documented, not silent):
//! - The `V_BLESS` "only one of them should talk" guard (`arkhata.c:1207`)
//!   reads the guard's *own* present `V_BLESS` value as a data-driven
//!   silence flag - a real C hack exploiting an otherwise-unused stat
//!   field (`zones/37/Bridge_Guards.chr`: the mage-sprite guard has
//!   `V_BLESS=100`, the warrior-sprite guard has `V_BLESS=0`/absent) rather
//!   than a proper per-character "designated talker" flag. Ported verbatim
//!   via `character_value_present(guard, CharacterValue::Bless)`.
//! - The `NT_NPC` branch's `fight_driver_add_enemy(cn, msg->dat3, 1, 0)`
//!   call (`arkhata.c:1240-1242`) has no `dat1`/`NTID_*` filter at all -
//!   unlike every other `NT_NPC`-consuming driver in this codebase, it
//!   reacts to *any* `NT_NPC` message where `msg->dat2`'s group matches its
//!   own. A full C-source cross-reference confirms no `notify_area(...,
//!   NT_NPC, ...)` call site in the entire codebase omits an `NTID_*`
//!   `dat1` value, so this branch's group-based ally-help behavior is
//!   reachable in principle but never actually triggered by any existing C
//!   broadcast in practice - ported verbatim anyway (matching the
//!   established "port dead code faithfully, document it" precedent used
//!   for e.g. `world::npc::area22::lab5_mage`'s `DEMONS` keyword).
//! - No dialogue/quest state exists in C's `bridgeguard_driver` at all (its
//!   only player-facing text is the level-gated one-time greeting) - no
//!   `PlayerRuntime`/`arkhata_ppd` field is touched anywhere in this file.

use crate::drvlib::map_dist;
use crate::world::*;

/// C `char_dist(cn, co) > 10` (implicit via `dist >= 16` below is the real
/// gate; kept for parity with sibling drivers' distance constants even
/// though C's own check here is `map_dist`, not `char_dist`).
/// C `map_dist(...) >= 16` (`arkhata.c:1198`).
const BRIDGEGUARD_SIGHT_DIST: i32 = 16;
/// C `TICKS * 20` (`arkhata.c:1214`).
const BRIDGEGUARD_GREET_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 20;
/// C `ch[co].level < 50` (`arkhata.c:1215`).
const BRIDGEGUARD_MIN_LEVEL: u32 = 50;
/// C `mem_check_driver(cn, co, 7)`/`mem_add_driver(cn, co, 7)`/
/// `mem_erase_driver(cn, 7)` (`arkhata.c:1213,1221,1264`).
const BRIDGEGUARD_MEMORY_SLOT: usize = 7;
/// C `TICKS * 60 * 60` (`arkhata.c:1265`): memory-slot-7 clear cadence.
const BRIDGEGUARD_MEMORY_CLEAR_TICKS: u64 = TICKS_PER_SECOND * 60 * 60;

impl World {
    /// C `bridgeguard_driver`'s per-tick dispatch loop (C `ch_driver`'s
    /// `CDR_BRIDGEGUARD` case, `arkhata.c:4562-4564`).
    pub fn process_bridgeguard_actions(&mut self, area_id: u16) -> usize {
        let guard_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_BRIDGEGUARD
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for guard_id in guard_ids {
            let mut seed = self.legacy_random_seed;
            let did_act = {
                let mut random = |below: u32| legacy_random_below_from_seed(&mut seed, below);
                self.process_bridgeguard_tick(guard_id, area_id, &mut random)
            };
            self.legacy_random_seed = seed;
            if did_act {
                acted += 1;
            }
        }
        acted
    }

    /// C `bridgeguard_driver`'s per-tick body (`arkhata.c:1179-1277`).
    fn process_bridgeguard_tick(
        &mut self,
        guard_id: CharacterId,
        area_id: u16,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let mut data = match self
            .characters
            .get(&guard_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::BridgeGuard(data)) => data,
            _ => BridgeGuardDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&guard_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if let Some(text) = message.text.as_deref() {
                        self.apply_tabunga_text_notification(guard_id, speaker_id, text);
                    }
                }
                NT_CHAR if message.dat1 > 0 => {
                    self.bridgeguard_handle_char_message(guard_id, &mut data, message);
                }
                // C `case NT_NPC: if ((co=msg->dat2)!=cn &&
                // ch[co].group==ch[cn].group) fight_driver_add_enemy(cn,
                // msg->dat3, 1, 0);` (`arkhata.c:1239-1242`) - see module
                // doc comment for why this is effectively unreachable.
                NT_NPC => {
                    self.bridgeguard_handle_npc_message(guard_id, message);
                }
                _ => {}
            }
        }

        if let Some(guard) = self.characters.get_mut(&guard_id) {
            guard.driver_state = Some(CharacterDriverState::BridgeGuard(data));
        }

        // C `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
        // 0)) return; if (fight_driver_follow_invisible(cn)) return;`
        // (`arkhata.c:1249-1257`).
        let Some(attacker) = self.characters.get(&guard_id).cloned() else {
            return false;
        };
        if self.fight_driver_attack_visible_and_follow(
            guard_id,
            &attacker,
            area_id,
            FightDriverSuppressions::default(),
            true,
            random,
        ) {
            return true;
        }

        // C `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return;` (`arkhata.c:1259-1263`).
        if self.regenerate_simple_baddy(guard_id) {
            return true;
        }
        if self.spell_self_simple_baddy(guard_id) {
            return true;
        }

        // C `if (ticker > dat->misc) { mem_erase_driver(cn, 7); dat->misc =
        // ticker + TICKS*60*60; }` (`arkhata.c:1264-1267`).
        let mut data = match self
            .characters
            .get(&guard_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::BridgeGuard(data)) => data,
            _ => BridgeGuardDriverData::default(),
        };
        if self.tick.0 > data.misc {
            if let Some(guard) = self.characters.get_mut(&guard_id) {
                mem_erase_driver(&mut guard.driver_memory, BRIDGEGUARD_MEMORY_SLOT);
            }
            data.misc = self.tick.0 + BRIDGEGUARD_MEMORY_CLEAR_TICKS;
            if let Some(guard) = self.characters.get_mut(&guard_id) {
                guard.driver_state = Some(CharacterDriverState::BridgeGuard(data));
            }
        }

        // C `do_idle(cn, TICKS);` (`arkhata.c:1275`).
        self.idle_simple_baddy(guard_id)
    }

    /// C `bridgeguard_driver`'s `NT_CHAR` branch (`arkhata.c:1197-1231`).
    fn bridgeguard_handle_char_message(
        &mut self,
        guard_id: CharacterId,
        data: &mut BridgeGuardDriverData,
        message: &CharacterDriverMessage,
    ) {
        let seen_id = CharacterId(message.dat1.max(0) as u32);
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        let Some(seen) = self.characters.get(&seen_id).cloned() else {
            return;
        };

        // C `dist = map_dist(ch[cn].tmpx, ch[cn].tmpy, ch[co].x,
        // ch[co].y); if (dist >= 16) break;` (`arkhata.c:1198-1201`) - the
        // guard's own post position (C's `tmpx`/`tmpy`) reuses `rest_x`/
        // `rest_y`, same substitution every other stationary NPC in this
        // codebase makes.
        let dist = map_dist(guard.rest_x, guard.rest_y, seen.x, seen.y);
        if dist >= BRIDGEGUARD_SIGHT_DIST {
            return;
        }

        // C `if (!(ch[co].flags & CF_PLAYER)) { if (ch[cn].group !=
        // ch[co].group) fight_driver_add_enemy(cn, co, 0, 1); break; }`
        // (`arkhata.c:1202-1207`).
        if !seen.flags.contains(CharacterFlags::PLAYER) {
            if guard.group != seen.group {
                let tick = self.tick.0 as i32;
                if let Some(guard_mut) = self.characters.get_mut(&guard_id) {
                    let _ = add_simple_baddy_enemy_unchecked(guard_mut, seen_id, 0, tick);
                }
            }
            return;
        }
        // C `if (ch[cn].value[0][V_BLESS]) break;` (`arkhata.c:1208-1210`):
        // "only one of them should talk" - see module doc comment.
        if character_value(&guard, CharacterValue::Bless) != 0 {
            return;
        }
        // C `if (!char_see_char(cn, co)) break;` (`arkhata.c:1211-1213`).
        if !char_see_char(&guard, &seen, &self.map, self.date.daylight) {
            return;
        }

        // C `if (dist < 16) { if (!mem_check_driver(cn, co, 7) && ticker -
        // dat->last_talk > TICKS*20) { ... } }` (`arkhata.c:1214-1230`) -
        // `dist < 16` is already guaranteed true here (the `dist >= 16`
        // early-out above already returned), matching C's redundant
        // re-check verbatim.
        let tick = self.tick.0;
        if !mem_check_driver(&guard.driver_memory, BRIDGEGUARD_MEMORY_SLOT, seen_id.0)
            && tick.saturating_sub(data.last_talk) > BRIDGEGUARD_GREET_COOLDOWN_TICKS
        {
            if seen.level < BRIDGEGUARD_MIN_LEVEL {
                self.npc_say(
                    guard_id,
                    &format!(
                        "Hold! This is no place for such inexperienced travellers as thyself. Return here when you are stronger, {}!",
                        seen.name
                    ),
                );
            } else {
                self.npc_say(
                    guard_id,
                    &format!("Greetings {}, thou mayest pass the bridge.", seen.name),
                );
            }
            data.last_talk = tick;
            if let Some(guard_mut) = self.characters.get_mut(&guard_id) {
                mem_add_driver(
                    &mut guard_mut.driver_memory,
                    BRIDGEGUARD_MEMORY_SLOT,
                    seen_id.0,
                );
            }
        }
    }

    /// C `bridgeguard_driver`'s `NT_NPC` branch (`arkhata.c:1239-1242`) -
    /// see module doc comment for why this is effectively unreachable in
    /// practice.
    fn bridgeguard_handle_npc_message(
        &mut self,
        guard_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let ally_id = CharacterId(message.dat2.max(0) as u32);
        if ally_id == guard_id {
            return;
        }
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        let Some(ally) = self.characters.get(&ally_id) else {
            return;
        };
        if ally.group != guard.group {
            return;
        }
        let target_id = CharacterId(message.dat3.max(0) as u32);
        let tick = self.tick.0 as i32;
        if let Some(guard_mut) = self.characters.get_mut(&guard_id) {
            let _ = add_simple_baddy_enemy_unchecked(guard_mut, target_id, 1, tick);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::{
    mem_add_driver, mem_check_driver, mem_erase_driver, CDR_BRIDGEGUARD,
};

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`): only `last_talk`/
/// `misc` are read/written by `bridgeguard_driver` (no `current_victim`
/// use anywhere in this file - unlike every dialogue-only sibling driver,
/// there is no "keep talking to the same victim" tracking here at all).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BridgeGuardDriverData {
    #[serde(default)]
    pub last_talk: u64,
    #[serde(default)]
    pub misc: u64,
}
