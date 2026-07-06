//! `turn_seyan` (`src/system/tool.c:4278-4389`): the full character re-roll
//! to a plain Seyan'Du that `gate_fight_dead`'s class-8 case
//! (`src/system/gatekeeper.c:742-748`, ported at
//! `World::apply_gate_fight_reward` in `world/gate_fight.rs`) calls
//! unconditionally (unlike classes 5-7, C's own guard comments for class 8
//! are commented out - `// if (ch[co].flags&CF_ARCH) return; // if
//! ((ch[co].flags&CF_WARRIOR) && (ch[co].flags&CF_MAGE)) return;` - so
//! this always runs).
//!
//! This module only covers the `Character`-only half of `turn_seyan`:
//! stat/exp/level reset, profession clear, un-equip/strip items, flag set,
//! and the hp/endurance/mana recompute tail via
//! [`World::update_character`]. `World` cannot look up the `"seyan_m"`
//! template itself (no `ZoneLoader` reference) or touch `PlayerRuntime`
//! (owned by `ugaris-server`), so the caller
//! (`ugaris-server::world_events::apply_gate_fight_death_from_hurt_event`)
//! supplies the template's base `value[1][]` array (looked up once via
//! `ZoneLoader::character_templates.get("seyan_m")`, matching C's
//! `create_char("seyan_m", 0)` + `destroy_char(co)` without ever
//! registering a throwaway character in `World`) and separately calls
//! [`crate::player::PlayerRuntime::clear_turn_seyan_ppd`] for the ~22
//! `del_data` calls this function doesn't reach.
//!
//! Deviations/gaps (documented, not silent):
//! - `destroy_chareffects(cn)` (`tool.c:4325`): no active-spell-effect list
//!   is modeled on `Character` yet, so this is a no-op, same precedent as
//!   `world/gatekeeper.rs`'s `gate_finish_enter_room` and `world/death.rs`.
//! - `DRD_DEPOT_PPD`'s "strip `IF_QUEST` flags from the 80 depot item
//!   slots" (`tool.c:4380-4388`, actually a full slot wipe) is now ported
//!   at `PlayerRuntime::clear_turn_seyan_ppd`, alongside its other
//!   `del_data` calls, now that `depot` has a typed Rust representation.

use super::*;
use crate::legacy::{INVENTORY_LAST_WORN, INVENTORY_START_WORN};

impl World {
    /// C `turn_seyan(int cn)` (`tool.c:4278-4389`), minus the PPD/depot
    /// tail (see module doc comment). `seyan_base_values` is the
    /// `"seyan_m"` template's `value[1][]` (C's `create_char`+copy+
    /// `destroy_char`); returns `false` if `cn` doesn't exist or the
    /// lengths don't line up (defensive - should never happen in
    /// practice, since every template shares the same `V_MAX`-sized
    /// array).
    pub fn apply_turn_seyan(&mut self, cn: CharacterId, seyan_base_values: &[i16]) -> bool {
        let Some(character) = self.characters.get_mut(&cn) else {
            return false;
        };
        if character.values.len() < 2 || character.values[1].len() != seyan_base_values.len() {
            return false;
        }

        // C `for (n = 0; n < V_MAX; n++) ch[cn].value[1][n] =
        // ch[co].value[1][n];` (`tool.c:4288-4291`).
        character.values[1] = seyan_base_values.to_vec();

        // C `ch[cn].exp = 0; ch[cn].exp_used = 0; ch[cn].level = 1;
        // ch[cn].lifeshield = 0;` (`tool.c:4294-4297`).
        character.exp = 0;
        character.exp_used = 0;
        character.level = 1;
        character.lifeshield = 0;

        // C `for (n = 0; n < P_MAX; n++) ch[cn].prof[n] = 0;`
        // (`tool.c:4300-4302`).
        for profession in character.professions.iter_mut() {
            *profession = 0;
        }

        // C's un-equip loop (`tool.c:4305-4318`): worn items (slots
        // `0..12`) are moved into the first free inventory slot at or
        // past `30`, scanning forward with a persistent cursor `m` (never
        // rescanning already-filled slots); items are destroyed instead
        // if inventory is completely full.
        let mut free_slot = INVENTORY_START_INVENTORY;
        let mut to_destroy: Vec<ItemId> = Vec::new();
        for worn_slot in INVENTORY_START_WORN..=INVENTORY_LAST_WORN {
            let Some(item_id) = character.inventory[worn_slot].take() else {
                continue;
            };
            while free_slot < INVENTORY_SIZE && character.inventory[free_slot].is_some() {
                free_slot += 1;
            }
            if free_slot == INVENTORY_SIZE {
                to_destroy.push(item_id);
            } else {
                character.inventory[free_slot] = Some(item_id);
            }
        }

        // C `for (n = 12; n < 30; n++) if ((in = ch[cn].item[n]))
        // { free_item(in); ch[cn].item[n] = 0; }` (`tool.c:4320-4324`).
        to_destroy.extend(
            (INVENTORY_START_SPELLS..=INVENTORY_LAST_SPELLS)
                .filter_map(|slot| character.inventory[slot].take()),
        );

        // C `destroy_chareffects(cn);` (`tool.c:4325`): documented no-op,
        // see module doc comment.

        // C `ch[cn].flags |= CF_MAGE | CF_WARRIOR | CF_ITEMS;`
        // (`tool.c:4328`).
        character
            .flags
            .insert(CharacterFlags::MAGE | CharacterFlags::WARRIOR | CharacterFlags::ITEMS);

        // C `ch[cn].hp = ch[cn].value[0][V_HP] * POWERSCALE;` etc.
        // (`tool.c:4330-4333`): deliberately reads the *stale*
        // `value[0]` (not yet recomputed from the new `value[1]` above -
        // that happens in `update_char`/`update_character` right after),
        // matching C's exact order; `update_character`'s own
        // hp/endurance/mana-exceeds-max clamp then settles the final
        // value, so this line is effectively superseded in practice
        // (a real character's old `value[0][V_HP]` is virtually always
        // above the fresh level-1 Seyan'Du max) but is kept for exact
        // parity since it is still observable if it weren't.
        character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
        character.endurance =
            i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
        character.mana = i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;

        for item_id in to_destroy {
            self.destroy_item(item_id);
        }

        // C `update_char(cn);` (`tool.c:4335`).
        self.update_character(cn);

        // C's quest-item inventory sweep (`tool.c:4356-4365`): `for (n = 0;
        // n < INVENTORYSIZE; n++)` over every slot (worn items already
        // moved/destroyed above, so only spell - already emptied - and
        // regular inventory slots can still hold anything).
        let Some(character) = self.characters.get(&cn) else {
            return true;
        };
        let quest_slots: Vec<usize> = character
            .inventory
            .iter()
            .enumerate()
            .filter_map(|(slot, item_id)| {
                item_id.and_then(|item_id| {
                    self.items
                        .get(&item_id)
                        .is_some_and(|item| item.flags.contains(ItemFlags::QUEST))
                        .then_some(slot)
                })
            })
            .collect();
        let quest_items: Vec<ItemId> = {
            let Some(character) = self.characters.get_mut(&cn) else {
                return true;
            };
            quest_slots
                .into_iter()
                .filter_map(|slot| character.inventory[slot].take())
                .collect()
        };
        for item_id in quest_items {
            self.destroy_item(item_id);
        }

        true
    }
}
