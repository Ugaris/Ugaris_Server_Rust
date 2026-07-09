//! Area 18 (`src/area/18/bones.c`) bone-holder rune-stand mechanics that
//! need `World`-level map/character access beyond a single item+character
//! (the pure item-driver boundary already ported in
//! `item_driver/area18_bones.rs`'s `boneholder_driver`): the sprite/
//! foreground-sprite refresh (`update_holder`), the three-preceding-stand
//! scan an activation holder performs when touched (the `boneholder`
//! function's `it[in].drdata[1] == 2 || 3` branch), and the rune-
//! combination reward table (`exec_rune`).
//!
//! The `used[]` bitmask gate (C `rune_check`/`rune_set`) lives on the
//! session-owned `PlayerRuntime` (`player/areas_misc.rs`), not here -
//! `World` has no access to it, mirroring the `BoneHint`/`bone_hint`
//! split already established for this same C file. The server crate
//! calls `PlayerRuntime::rune_check` before, and `PlayerRuntime::
//! rune_set` after (gated on this module's returned `flag`), calling
//! [`World::exec_rune`] (`crates/ugaris-server/src/tick_item_use_bones.rs`).

use super::*;
use crate::entity::CharacterValue;
use crate::world::values::full_skill_name;

impl World {
    /// C `update_holder(in)` (`src/area/18/bones.c:643-666`): recomputes
    /// the holder item's own sprite and the map tile's foreground sprite
    /// from `drdata[0]` (held rune, 0 if empty) and `drdata[1]` (holder
    /// kind: 0 plain stand, nonzero activation stand).
    pub(crate) fn update_bone_holder_sprite(&mut self, item_id: ItemId) {
        let Some(item) = self.items.get(&item_id) else {
            return;
        };
        let rune = crate::item_driver::drdata(item, 0);
        let kind = crate::item_driver::drdata(item, 1);
        let (x, y) = (usize::from(item.x), usize::from(item.y));
        let (sprite, foreground_sprite) = if rune == 0 {
            (if kind == 0 { 13103 } else { 13104 }, 0u32)
        } else {
            (
                13104 + i32::from(rune),
                if kind == 0 { 13103u32 } else { 13104u32 },
            )
        };
        if let Some(item) = self.items.get_mut(&item_id) {
            item.sprite = sprite;
        }
        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.foreground_sprite = foreground_sprite;
        }
        self.mark_dirty_sector(x, y);
    }

    /// C `boneholder`'s activation-stand branch scan (`bones.c:698-717`,
    /// the three `map[it[in].x + it[in].y * MAXMAP - 3/-2/-1].it` reads)
    /// plus the `remove_rune_from_holder` half that clears each matched
    /// stand's own rune and refreshes its sprite (C `bones.c:678-688`
    /// minus the `create_rune_from_holder`/`give_char_item` item-creation
    /// tail, which needs a `ZoneLoader` template instantiation the server
    /// crate applies from the returned `(holder_item_id, rune)` pairs).
    ///
    /// Returns the concatenated combination number (C's `nr`, built as
    /// `nr = nr * 10 + rune` in stand order: 3-tiles-back first, so it
    /// becomes the hundreds digit) and the list of stands that were
    /// cleared (so the caller can hand each rune item back to the
    /// player).
    pub(crate) fn scan_and_clear_bone_holder_runes(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> (i32, [Option<(ItemId, u8)>; 3]) {
        let mut cleared: [Option<(ItemId, u8)>; 3] = [None; 3];
        let Some(item) = self.items.get(&item_id) else {
            return (0, cleared);
        };
        let linear = usize::from(item.y) * MAX_MAP + usize::from(item.x);
        let mut nr = 0i32;
        for (slot, offset) in [3usize, 2, 1].into_iter().enumerate() {
            let Some(idx) = linear.checked_sub(offset) else {
                continue;
            };
            let (x, y) = (idx % MAX_MAP, idx / MAX_MAP);
            let Some(holder_item_id) = self
                .map
                .tile(x, y)
                .map(|tile| tile.item)
                .filter(|id| *id != 0)
            else {
                continue;
            };
            let holder_item_id = ItemId(holder_item_id);
            let Some(holder) = self.items.get(&holder_item_id) else {
                continue;
            };
            let rune = crate::item_driver::drdata(holder, 0);
            if rune == 0 || crate::item_driver::drdata_u32(holder, 8) != character_id.0 {
                continue;
            }
            nr = nr * 10 + i32::from(rune);
            if let Some(holder) = self.items.get_mut(&holder_item_id) {
                crate::item_driver::set_drdata(holder, 0, 0);
            }
            self.update_bone_holder_sprite(holder_item_id);
            cleared[slot] = Some((holder_item_id, rune));
        }
        (nr, cleared)
    }

    /// C `exec_rune(cn, nr, lastholder)` (`src/area/18/bones.c:327-641`):
    /// the rune-combination reward table. Returns C's `flag` - whether the
    /// caller should mark `nr` used via `PlayerRuntime::rune_set`. The
    /// nine single/double/triple-repeated-digit case families
    /// (`N`/`NN`/`NNN`, one per area-18 sub-level 1-9) are level-
    /// progression area-teleport/bonus-exp/"laugh" navigation combos that
    /// intentionally never set the flag (repeatable every visit, matching
    /// C exactly - only `NN`'s bonus-exp case does); every other reward
    /// (the six literal three-digit skill combos, and every
    /// `special_exec`-table match) sets it unconditionally, even when the
    /// underlying `raise_value_exp` call itself fails (matching C's
    /// `flag = 1;` living outside the `if (raise_value_exp(...))` guard).
    pub fn exec_rune(
        &mut self,
        character_id: CharacterId,
        nr: i32,
        special_exec: &[i32; crate::player::RUNE_SPECIAL_EXEC_COUNT],
        last_holder: bool,
        area_id: u32,
    ) -> bool {
        let level = self
            .characters
            .get(&character_id)
            .map(|character| character.level)
            .unwrap_or(0);
        let mut flag = false;
        const LAUGH: &str = "You hear a sinister laugh echo through the emptiness.";
        match nr {
            // level 1
            1 => {
                self.teleport_char_driver(character_id, 90, 25);
            }
            11 => {
                self.grant_rune_bonus_exp(character_id, level, 50, area_id);
                flag = true;
            }
            111 => {
                self.queue_system_text(character_id, LAUGH);
                self.teleport_char_driver(character_id, 72, 52);
            }

            // level 2
            2 => {
                self.teleport_char_driver(character_id, 175, 4);
            }
            22 => {
                self.grant_rune_bonus_exp(character_id, level, 51, area_id);
                flag = true;
            }
            222 => {
                self.queue_system_text(character_id, LAUGH);
                self.teleport_char_driver(character_id, 105, 40);
            }
            212 => {
                self.raise_rune_skill(character_id, CharacterValue::Endurance, "endurance");
                flag = true;
            }

            // level 3
            3 => {
                self.teleport_char_driver(character_id, 4, 69);
            }
            33 => {
                self.grant_rune_bonus_exp(character_id, level, 52, area_id);
                flag = true;
            }
            333 => {
                self.queue_system_text(character_id, LAUGH);
                self.teleport_char_driver(character_id, 236, 25);
            }
            231 => {
                self.raise_rune_skill(character_id, CharacterValue::Hp, "hitpoints");
                flag = true;
            }
            133 => {
                self.raise_rune_skill(character_id, CharacterValue::Mana, "mana");
                flag = true;
            }

            // level 4
            4 => {
                self.teleport_char_driver(character_id, 90, 95);
            }
            44 => {
                self.grant_rune_bonus_exp(character_id, level, 54, area_id);
                flag = true;
            }
            444 => {
                self.queue_system_text(character_id, LAUGH);
                self.teleport_char_driver(character_id, 67, 85);
            }
            143 => {
                self.raise_rune_skill(character_id, CharacterValue::Parry, "parry");
                flag = true;
            }
            442 => {
                self.raise_rune_skill(character_id, CharacterValue::MagicShield, "magic shield");
                flag = true;
            }
            241 => {
                self.raise_rune_skill(character_id, CharacterValue::Immunity, "immunity");
                flag = true;
            }

            // level 5
            5 => {
                self.teleport_char_driver(character_id, 176, 110);
            }
            55 => {
                self.grant_rune_bonus_exp(character_id, level, 55, area_id);
                flag = true;
            }
            555 => {
                self.queue_system_text(character_id, LAUGH);
                self.teleport_char_driver(character_id, 107, 105);
            }

            // level 6
            6 => {
                self.teleport_char_driver(character_id, 4, 146);
            }
            66 => {
                self.grant_rune_bonus_exp(character_id, level, 57, area_id);
                flag = true;
            }
            666 => {
                self.queue_system_text(character_id, LAUGH);
                self.teleport_char_driver(character_id, 216, 115);
            }

            // level 7
            7 => {
                self.teleport_char_driver(character_id, 91, 171);
            }
            77 => {
                self.grant_rune_bonus_exp(character_id, level, 58, area_id);
                flag = true;
            }
            777 => {
                self.queue_system_text(character_id, LAUGH);
                self.teleport_char_driver(character_id, 9, 172);
            }

            // level 8
            8 => {
                self.teleport_char_driver(character_id, 187, 152);
            }
            88 => {
                self.grant_rune_bonus_exp(character_id, level, 59, area_id);
                flag = true;
            }
            888 => {
                self.queue_system_text(character_id, LAUGH);
                self.teleport_char_driver(character_id, 91, 133);
            }

            // level 9
            9 => {
                self.teleport_char_driver(character_id, 137, 222);
            }
            99 => {
                self.grant_rune_bonus_exp(character_id, level, 61, area_id);
                flag = true;
            }
            999 => {
                self.queue_system_text(character_id, LAUGH);
                self.teleport_char_driver(character_id, 235, 152);
            }

            // special execs and wrong combos
            _ => {
                if let Some(slot) = special_exec.iter().position(|&value| value == nr) {
                    flag = self.exec_rune_special(character_id, slot, last_holder);
                } else {
                    self.queue_system_text(character_id, "Nothing happened.");
                }
            }
        }
        flag
    }

    /// C `give_exp(cn, level_value(min(ch[cn].level + 5, N)) / 6)` plus
    /// its trailing `"You gained experience."` message, shared by all
    /// nine `NN` bonus-exp cases in [`Self::exec_rune`].
    fn grant_rune_bonus_exp(
        &mut self,
        character_id: CharacterId,
        level: u32,
        cap: u32,
        area_id: u32,
    ) {
        let amount = level_value(level.saturating_add(5).min(cap)) / 6;
        self.give_exp(character_id, i64::from(amount), area_id);
        self.queue_system_text(character_id, "You gained experience.");
    }

    /// The six literal three-digit skill-raise cases (`212`/`231`/`133`/
    /// `143`/`442`/`241`): C calls `raise_value_exp` directly (not
    /// `rune_raise` - no warrior/mage skill choice) and logs a fixed
    /// literal message, distinct from `rune_raise`'s dynamic
    /// `skill[].name`-based text used by the `special_exec` table below.
    fn raise_rune_skill(
        &mut self,
        character_id: CharacterId,
        value: CharacterValue,
        message: &str,
    ) {
        let gained = self
            .characters
            .get_mut(&character_id)
            .is_some_and(|character| {
                crate::item_driver::raise_value_exp(character, value as usize).is_some()
            });
        if gained {
            self.queue_system_text(character_id, format!("You gained {message}."));
        }
    }

    /// C `exec_rune`'s `default:` branch inner `switch (n)` (`bones.c:
    /// 511-629`), reached once a `special_exec[n] == nr` match is found.
    /// `n` (`slot`) is the same 0-24 index the generation table
    /// (`ensure_rune_special_execs`) filled, grouped in five-slot bands
    /// per sub-level 5-9. Returns C's per-arm `flag` value (`false` only
    /// for case 20, the bonus-area teleport, which needs a separate
    /// "correct but not usable here" gate on `last_holder`).
    fn exec_rune_special(
        &mut self,
        character_id: CharacterId,
        slot: usize,
        last_holder: bool,
    ) -> bool {
        use CharacterValue::*;
        match slot {
            // level 5
            0 => {
                self.rune_raise(character_id, Attack, Flash);
                true
            }
            1 => {
                self.rune_raise(character_id, Sword, Pulse);
                true
            }
            2 => {
                self.rune_raise(character_id, TwoHand, Fireball);
                true
            }
            3 => {
                self.rune_raise(character_id, Hand, Heal);
                true
            }
            4 => {
                self.rune_raise(character_id, Percept, Percept);
                true
            }

            // level 6
            5 => {
                self.rune_raise(character_id, Profession, Profession);
                true
            }
            6 => {
                self.rune_raise(character_id, Intelligence, Intelligence);
                true
            }
            7 => {
                self.rune_raise(character_id, Strength, Wisdom);
                true
            }
            8 => {
                self.rune_raise(character_id, Hp, Mana);
                true
            }
            9 => {
                self.rune_raise(character_id, Stealth, Stealth);
                true
            }

            // level 7
            10 => {
                self.rune_raise(character_id, Sword, Pulse);
                true
            }
            11 => {
                self.rune_raise(character_id, Attack, Flash);
                true
            }
            12 => {
                self.rune_raise(character_id, Parry, MagicShield);
                true
            }
            13 => {
                self.rune_raise(character_id, Surround, Dagger);
                true
            }
            14 => {
                self.rune_raise(character_id, BodyControl, Staff);
                true
            }

            // level 8
            15 => {
                self.rune_raise(character_id, Tactics, Bless);
                true
            }
            16 => {
                self.rune_raise(character_id, Agility, Agility);
                true
            }
            17 => {
                self.rune_raise(character_id, Hp, Mana);
                true
            }
            18 => {
                self.rune_raise(character_id, Endurance, Endurance);
                true
            }
            19 => {
                self.rune_raise(character_id, Barter, Barter);
                true
            }

            // level 9
            20 => {
                if !last_holder {
                    self.queue_system_text(
                        character_id,
                        "This combination seems to be right, but it does not work here.",
                    );
                } else {
                    self.teleport_char_driver(character_id, 14, 213);
                    self.queue_system_text(character_id, "Uh-oh");
                }
                false
            }
            21 => {
                self.rune_raise(character_id, Intelligence, Intelligence);
                true
            }
            22 => {
                self.rune_raise(character_id, Immunity, Immunity);
                true
            }
            23 => {
                self.rune_raise(character_id, Profession, Profession);
                true
            }
            24 => {
                self.rune_raise(character_id, Strength, Wisdom);
                true
            }

            _ => {
                self.queue_system_text(character_id, "You found bug #1926");
                false
            }
        }
    }

    /// C `rune_raise(cn, war_skill, mage_skill)` (`bones.c:315-325`):
    /// picks the warrior or mage skill by `CF_WARRIOR`, raises it, and on
    /// success logs `"You gained %s."` with the C `skill[].name` table
    /// (`full_skill_name`) - dynamic text, distinct from
    /// [`Self::raise_rune_skill`]'s fixed literal messages.
    fn rune_raise(
        &mut self,
        character_id: CharacterId,
        war_skill: CharacterValue,
        mage_skill: CharacterValue,
    ) {
        let warrior = self
            .characters
            .get(&character_id)
            .is_some_and(|character| character.flags.contains(CharacterFlags::WARRIOR));
        let skill = if warrior { war_skill } else { mage_skill };
        let gained = self
            .characters
            .get_mut(&character_id)
            .is_some_and(|character| {
                crate::item_driver::raise_value_exp(character, skill as usize).is_some()
            });
        if gained {
            self.queue_system_text(
                character_id,
                format!("You gained {}.", full_skill_name(skill)),
            );
        }
    }
}
