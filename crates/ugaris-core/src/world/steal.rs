//! `/steal` (C `cmd_steal`, `src/system/prof.c:106-222`): a thief attempts
//! to steal an item from whichever character they are directly facing.

use crate::item_ops::{can_carry, can_use_inventory_slot, remove_item_from_character};

use super::*;

/// Outcome of an attempted [`World::attempt_steal`] call. Each variant maps
/// to one `log_char`/return path in C's `cmd_steal`; the command layer
/// (`crates/ugaris-server`) turns these into the actual player-facing text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StealOutcome {
    /// "You are not a thief, you cannot steal."
    NotAThief,
    /// "You can only steal when standing still."
    NotIdle,
    /// "Please free your hand (mouse cursor) first."
    HandFull,
    /// "Out of map."
    OutOfMap,
    /// "There's no one to steal from."
    NoOneThere,
    /// "You cannot steal from someone you are not allowed to attack."
    CannotAttack,
    /// "You cannot steal inside an arena."
    ArenaOrClan,
    /// "You can only steal from players."
    NotAPlayer,
    /// "You cannot steal from lagging players."
    Lagging,
    /// "You cannot steal in Live Quests."
    LiveQuests,
    /// "You cannot steal from someone if your victim is not standing still."
    VictimBusy,
    /// "You could not find anything to steal."
    NothingToSteal,
    /// "You'd get caught for sure. You decide not to try." (`chance < 10`,
    /// returned before any item is picked or any dice are rolled.)
    WouldBeCaught,
    /// Attempt failed and the victim noticed (`diff < -20`): no item
    /// changes hands, the thief's `endurance` is reset to `1` to
    /// interrupt regen (C `ch[cn].endurance = 1;`).
    Caught {
        victim_id: CharacterId,
        victim_name: String,
    },
    /// C's own `give_char_item` failed after the item was already removed
    /// from the victim (backpack-full race) - the item is destroyed
    /// (C `destroy_item(in); elog(...);`) and, matching C exactly,
    /// **no player-facing message is sent at all**.
    ItemLostSilently,
    /// Stole successfully, but the victim noticed (`-20 <= diff < 0`).
    StolenNoticed {
        victim_id: CharacterId,
        victim_name: String,
        item_name: String,
    },
    /// Stole successfully and completely unnoticed (`diff >= 0`).
    StolenUnnoticed {
        victim_name: String,
        item_name: String,
    },
}

impl World {
    /// C `cmd_steal` (`src/system/prof.c:106-222`). Attempts to steal an
    /// item from whichever character `character_id` is directly facing.
    pub fn attempt_steal(&mut self, character_id: CharacterId) -> StealOutcome {
        let Some(character) = self.characters.get(&character_id) else {
            return StealOutcome::NoOneThere;
        };

        if character_profession(character, profession::THIEF) == 0 {
            return StealOutcome::NotAThief;
        }
        if character.action != action::IDLE {
            return StealOutcome::NotIdle;
        }
        if character.cursor_item.is_some() {
            return StealOutcome::HandFull;
        }

        let Ok(direction) = Direction::try_from(character.dir) else {
            return StealOutcome::OutOfMap;
        };
        let (dx, dy) = direction.delta();
        let (Some(x), Some(y)) = (
            offset_coordinate(usize::from(character.x), dx),
            offset_coordinate(usize::from(character.y), dy),
        ) else {
            return StealOutcome::OutOfMap;
        };
        // C: `if (x < 1 || x >= MAXMAP - 1 || y < 1 || y >= MAXMAP - 1)`.
        if x < 1 || x >= MAX_MAP - 1 || y < 1 || y >= MAX_MAP - 1 {
            return StealOutcome::OutOfMap;
        }

        let Some(tile) = self.map.tile(x, y) else {
            return StealOutcome::OutOfMap;
        };
        let tile_flags = tile.flags;
        if tile.character == 0 {
            return StealOutcome::NoOneThere;
        }
        let victim_id = CharacterId(u32::from(tile.character));

        let Some(victim) = self.characters.get(&victim_id) else {
            return StealOutcome::NoOneThere;
        };

        if !can_attack(character, victim, &self.map) {
            return StealOutcome::CannotAttack;
        }
        if tile_flags.intersects(MapFlags::ARENA | MapFlags::CLAN) {
            return StealOutcome::ArenaOrClan;
        }
        if !victim.flags.contains(CharacterFlags::PLAYER) {
            return StealOutcome::NotAPlayer;
        }
        if victim.driver == CDR_LOSTCON {
            return StealOutcome::Lagging;
        }
        if self.area_id == 20 {
            return StealOutcome::LiveQuests;
        }
        if victim.action != action::IDLE
            || self.tick.0.saturating_sub(u64::from(victim.regen_ticker)) < TICKS_PER_SECOND
        {
            return StealOutcome::VictimBusy;
        }

        // Every worn/carried slot is eligible except the spell-slot range
        // (C: `if (n >= 12 && n < 30) continue;`), skipping quest items and
        // anything the thief's own [`can_carry`] check rejects (one-carry
        // drivers already held, or bonded items owned by someone else).
        let stealable: Vec<ItemId> = victim
            .inventory
            .iter()
            .enumerate()
            .filter(|(slot, _)| can_use_inventory_slot(*slot))
            .filter_map(|(_, item_id)| *item_id)
            .filter(|item_id| {
                self.items.get(item_id).is_some_and(|item| {
                    !item.flags.contains(ItemFlags::QUEST)
                        && can_carry(character, item, &self.items)
                })
            })
            .collect();

        if stealable.is_empty() {
            return StealOutcome::NothingToSteal;
        }

        let pick =
            legacy_random_below_from_seed(&mut self.legacy_random_seed, stealable.len() as u32)
                as usize;
        let item_id = stealable[pick];

        let stealth = character_value(character, CharacterValue::Stealth);
        let percept = character_value(victim, CharacterValue::Percept);
        let diff = (stealth - percept) / 2;
        let mut chance = 40 + diff;
        if chance < 10 {
            return StealOutcome::WouldBeCaught;
        }
        chance = chance.min(character_profession(character, profession::THIEF) * 3);

        let dice = legacy_random_below_from_seed(&mut self.legacy_random_seed, 100) as i32;
        let diff = chance - dice;

        let victim_name = victim.name.clone();

        if diff < -20 {
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.endurance = 1;
            }
            return StealOutcome::Caught {
                victim_id,
                victim_name,
            };
        }

        let item_name = self
            .items
            .get(&item_id)
            .map(|item| item.name.clone())
            .unwrap_or_default();

        // C `remove_item_char(in)`, unconditional once we've committed to
        // the theft attempt (the "caught with no attempt" and "would be
        // caught" paths above never reach here).
        if let Some(item) = self.items.get_mut(&item_id) {
            if let Some(victim) = self.characters.get_mut(&victim_id) {
                remove_item_from_character(victim, item);
            }
        }

        let given = match (
            self.characters.get_mut(&character_id),
            self.items.get_mut(&item_id),
        ) {
            (Some(character), Some(item)) => {
                give_item_to_character(character, item, GiveItemFlags::NONE)
            }
            _ => GiveItemResult::Failed,
        };
        if given != GiveItemResult::Ok {
            // C: `destroy_item(in); elog("had to destroy item in
            // cmd_steal()!"); return;` - no player-facing message.
            self.destroy_item(item_id);
            return StealOutcome::ItemLostSilently;
        }

        if diff < 0 {
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.endurance = 1;
            }
            StealOutcome::StolenNoticed {
                victim_id,
                victim_name,
                item_name,
            }
        } else {
            StealOutcome::StolenUnnoticed {
                victim_name,
                item_name,
            }
        }
    }
}
