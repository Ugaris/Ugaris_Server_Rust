//! `CDR_DUNGEONFIGHTER` driver (`src/area/13/dungeon.c`'s `dungeonfighter`/
//! `dungeon_potion`, `dungeon.c:1956-2161`) - the autonomous stat-potion-
//! drinking half of a live clan-raid catacomb's warrior/mage/seyan guard
//! NPCs (`crates/ugaris-server/src/dungeon.rs`'s `build_warrior`/
//! `build_mage`/`build_seyan` spawn them from `dungeon.chr` zone templates
//! whose `driver=52`).
//!
//! Ported this slice: the `NT_DIDHIT`/`NT_GOTHIT` damage-message
//! accumulation, the three simple-potion (mana/hp/combo) drink checks
//! against `ClanEconomy::simple_pot`, and [`World::dungeon_potion`]'s
//! alchemy stat-potion grant against `ClanEconomy::alc_pot`. C's own
//! consumption side effect for both potion kinds is a
//! `server_chat(1028, ...)` cross-node message to a master server that
//! owns the authoritative `struct clan` array (`clan.c`'s
//! `clan_dungeon_chat`, cases `'s'`/`'a'`) - this codebase has no
//! master/slave server split (single area server per process), so -
//! matching `crate::clan::ClanRegistry`'s own `add_alc_potion`/
//! `bump_simple_pot` precedent on the increment side - the decrement is
//! applied directly and locally via
//! [`crate::clan::ClanRegistry::consume_alc_pot`]/
//! [`crate::clan::ClanRegistry::consume_simple_pot`] instead of round-
//! tripping through an unported IPC channel.
//!
//! Also ported (a later slice, see `PORTING_TODO.md`'s Clan system task
//! Progress Log): the tail `char_driver(CDR_SIMPLEBADDY, CDT_DRIVER, cn,
//! ret, lastact)` call (`dungeon.c:2161`) that gives these NPCs their
//! actual idle-wander/auto-attack combat AI. Every existing Rust
//! `process_simple_baddy_*` dispatch function requires a
//! `CharacterDriverState::SimpleBaddy` payload (most also independently
//! check `character.driver == CDR_SIMPLEBADDY`, now widened to also
//! accept `CDR_DUNGEONFIGHTER`); since `Character::driver_state` is a
//! single-variant slot, unlike C's `set_data`, which lets one character
//! hold independent named data blobs for *both* drivers at once
//! (`dungeon.c:2047` vs `simple_baddy.c:164`), this module's own
//! `DungeonfighterDriverData` now lives on the dedicated
//! `Character::dungeonfighter` field instead, freeing `driver_state` to
//! hold a real `SimpleBaddy(SimpleBaddyDriverData)` for these NPCs -
//! parsed from the exact same zone-template `arg="aggressive=1;..."`
//! string the "warrior"/"mage"/"seyan" `dungeon.chr` templates already
//! carry for this purpose (`zone.rs`'s `CDR_DUNGEONFIGHTER` branch, reusing
//! `apply_simple_baddy_create_message`), even though `dungeonfighter`
//! itself never reads that arg string. See `Character::dungeonfighter`'s
//! doc comment for the full precedent/rationale.
//!
//! `fighter_dead` (`dungeon.c:1899-1912`, the death hook wired via C's
//! `ch_died_driver`) is intentionally not ported: its only effect is
//! another `server_chat(1028, ...)` message decrementing `clan[cnr].
//! dungeon.{warrior,mage,seyan}[0][level]` - and a fresh grep of the
//! entire C tree confirms that `[0]` sub-array (as opposed to the `[1]`
//! "configured use" sub-array `get_clan_dungeon`/`set_clan_dungeon_use`
//! already cover) is *never incremented anywhere* in the C source, so the
//! `> 0` guard before the decrement never passes - dead code in the
//! original C, preserved as a no-op rather than invented here.

use crate::character_driver::{DungeonfighterDriverData, CDR_DUNGEONFIGHTER};
use crate::world::*;

impl World {
    /// C `ch_driver`'s `CDR_DUNGEONFIGHTER` dispatch (`dungeon.c:2196-
    /// 2199`): every live `CDR_DUNGEONFIGHTER` NPC's message loop, once
    /// per tick.
    pub fn process_dungeonfighter_actions(&mut self) {
        let fighter_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_DUNGEONFIGHTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for fighter_id in fighter_ids {
            self.process_dungeonfighter_messages(fighter_id);
        }
    }

    /// C `dungeonfighter(cn, ret, lastact)` (`dungeon.c:2033-2161`), minus
    /// the tail `char_driver(CDR_SIMPLEBADDY, ...)` call - see the module
    /// doc comment.
    fn process_dungeonfighter_messages(&mut self, fighter_id: CharacterId) {
        let Some(mut dat) = self
            .characters
            .get(&fighter_id)
            .and_then(|character| character.dungeonfighter)
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&fighter_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        let mut didhit = false;
        for message in &messages {
            match message.message_type {
                NT_DIDHIT => {
                    dat.damage_done = dat.damage_done.saturating_add(message.dat2);
                    didhit = true;
                }
                NT_GOTHIT => {
                    dat.damage_taken = dat.damage_taken.saturating_add(message.dat2);
                }
                _ => {}
            }
        }

        let Some((cnr, max_mana, max_hp, max_endurance)) =
            self.characters.get(&fighter_id).map(|character| {
                (
                    character.rest_x,
                    i32::from(character.values[0][CharacterValue::Mana as usize]),
                    i32::from(character.values[0][CharacterValue::Hp as usize]),
                    i32::from(character.values[0][CharacterValue::Endurance as usize]),
                )
            })
        else {
            self.store_dungeonfighter_data(fighter_id, dat);
            return;
        };

        // C: `if (dat->damage_done > 10 && dat->damage_done > dat->damage_taken / 16)`,
        // repeated identically ahead of all three simple-potion checks.
        let good_damage = dat.damage_done > 10 && dat.damage_done > dat.damage_taken / 16;
        let mut flag = false;

        // C's mana-potion block (`dungeon.c:2071-2090`).
        if max_mana != 0 && dat.simple_pots_taken < 5 {
            let mana = self.characters.get(&fighter_id).map_or(0, |c| c.mana);
            if mana < max_mana * POWERSCALE / 2 && good_damage {
                let need = max_mana - mana / POWERSCALE;
                if let Some((add, tier)) = self.pick_simple_pot(cnr, 1, need) {
                    if let Some(character) = self.characters.get_mut(&fighter_id) {
                        character.mana =
                            (character.mana + add * POWERSCALE).min(max_mana * POWERSCALE);
                    }
                    self.npc_emote(fighter_id, &format!("drinks a mana potion ({add})"));
                    dat.simple_pots_taken += 1;
                    self.clan_registry.consume_simple_pot(cnr, 1, tier);
                    flag = true;
                }
            }
        }

        // C's hp-potion block (`dungeon.c:2092-2111`).
        if dat.simple_pots_taken < 5 {
            let hp = self.characters.get(&fighter_id).map_or(0, |c| c.hp);
            if hp < max_hp * POWERSCALE / 2 && good_damage {
                let need = max_hp - hp / POWERSCALE;
                if let Some((add, tier)) = self.pick_simple_pot(cnr, 0, need) {
                    if let Some(character) = self.characters.get_mut(&fighter_id) {
                        character.hp = (character.hp + add * POWERSCALE).min(max_hp * POWERSCALE);
                    }
                    self.npc_emote(fighter_id, &format!("drinks a healing potion ({add})"));
                    dat.simple_pots_taken += 1;
                    self.clan_registry.consume_simple_pot(cnr, 0, tier);
                    flag = true;
                }
            }
        }

        // C's combo-potion block (`dungeon.c:2112-2133`), only reached
        // when neither of the two blocks above already drank something
        // this tick (`&& !flag`).
        if !flag && dat.simple_pots_taken < 5 {
            let (hp, mana) = self
                .characters
                .get(&fighter_id)
                .map_or((0, 0), |c| (c.hp, c.mana));
            if (hp < max_hp * POWERSCALE / 2 || mana < max_mana * POWERSCALE / 2) && good_damage {
                let need = (max_hp - hp / POWERSCALE).max(max_mana - mana / POWERSCALE);
                if let Some((add, tier)) = self.pick_simple_pot(cnr, 2, need) {
                    if let Some(character) = self.characters.get_mut(&fighter_id) {
                        character.hp = (character.hp + add * POWERSCALE).min(max_hp * POWERSCALE);
                        character.mana =
                            (character.mana + add * POWERSCALE).min(max_mana * POWERSCALE);
                        character.endurance = (character.endurance + add * POWERSCALE)
                            .min(max_endurance * POWERSCALE);
                    }
                    self.npc_emote(fighter_id, &format!("drinks a combo potion ({add})"));
                    dat.simple_pots_taken += 1;
                    self.clan_registry.consume_simple_pot(cnr, 2, tier);
                }
            }
        }

        // C's alchemy-potion block (`dungeon.c:2135-2141`).
        let hp_after = self.characters.get(&fighter_id).map_or(0, |c| c.hp);
        if didhit
            && dat.alc_pots_taken < 3
            && dat.damage_done > 0
            && hp_after > max_hp * POWERSCALE / 2
        {
            if self.dungeon_potion(fighter_id) {
                dat.alc_pots_taken += 1;
            }
        }

        self.store_dungeonfighter_data(fighter_id, dat);
    }

    fn store_dungeonfighter_data(
        &mut self,
        fighter_id: CharacterId,
        dat: DungeonfighterDriverData,
    ) {
        if let Some(character) = self.characters.get_mut(&fighter_id) {
            character.dungeonfighter = Some(dat);
        }
    }

    /// Shared big/medium/small tier-selection logic for all three
    /// `dungeonfighter` simple-potion blocks (`dungeon.c:2075-2090`,
    /// `:2096-2111`, `:2117-2132` - identical `if (need > 24 && ...) {
    /// add = 24; nr = 2; } else if (need > 12 && ...) { add = 16; nr = 1;
    /// } else if (...) { add = 8; nr = 0; }` shape, differing only in
    /// which `simple_pot[kind]` row they read). Returns the chosen
    /// `(add, tier)` pair, or `None` if no stocked tier qualifies.
    fn pick_simple_pot(&self, cnr: u16, kind: usize, need: i32) -> Option<(i32, usize)> {
        let stock = |tier: usize| {
            self.clan_registry
                .identity(cnr)
                .map(|identity| identity.economy.simple_pot[kind][tier])
                .unwrap_or(0)
        };
        if need > 24 && stock(2) > 0 {
            Some((24, 2))
        } else if need > 12 && stock(1) > 0 {
            Some((16, 1))
        } else if stock(0) > 0 {
            Some((8, 0))
        } else {
            None
        }
    }

    /// C `dungeon_potion(cn)` (`dungeon.c:1955-2026`): a `dungeonfighter`
    /// NPC drinks one of its clan's stockpiled alchemy stat potions.
    /// Picks the highest tier the NPC's own `V_INT` qualifies for
    /// (`nr*10 <= V_INT`) that the clan still has in stock, consumes it
    /// (see the module doc comment for the single-server IPC-bypass
    /// simplification), and installs a 10-minute `IDR_POTION_SP` stat-
    /// boost spell: Attack/Parry/Immunity for warriors (`CF_WARRIOR`),
    /// Flash/MagicShield/Immunity for anyone else (mages/seyans - C's own
    /// binary `type` split, not a three-way class check). Returns `false`
    /// for every C early-return case (no free spell slot, no qualifying/
    /// stocked tier, no inventory room).
    pub fn dungeon_potion(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&character, &self.items, IDR_POTION_SP, self.tick.0 as u32)
        else {
            return false;
        };

        let cnr = character.rest_x;
        let kind = if character.flags.contains(CharacterFlags::WARRIOR) {
            0usize
        } else {
            1usize
        };
        let int_value = i32::from(character.values[1][CharacterValue::Intelligence as usize]);

        let mut selected_tier = None;
        for tier in (0..=5i32).rev() {
            if tier * 10 > int_value {
                continue;
            }
            let stock = self
                .clan_registry
                .identity(cnr)
                .map(|identity| identity.economy.alc_pot[kind][tier as usize])
                .unwrap_or(0);
            if stock < 1 {
                continue;
            }
            selected_tier = Some(tier as usize);
            break;
        }
        let Some(tier) = selected_tier else {
            return false;
        };

        let strength = (tier as i32) * 4 + 4;

        let mut modifier_index = [0i16; MAX_MODIFIERS];
        let mut modifier_value = [0i16; MAX_MODIFIERS];
        if kind == 0 {
            modifier_index[0] = CharacterValue::Attack as i16;
            modifier_index[1] = CharacterValue::Parry as i16;
        } else {
            modifier_index[0] = CharacterValue::Flash as i16;
            modifier_index[1] = CharacterValue::MagicShield as i16;
        }
        modifier_index[2] = CharacterValue::Immunity as i16;
        modifier_value[0] = strength as i16;
        modifier_value[1] = strength as i16;
        modifier_value[2] = strength as i16;

        self.clan_registry.consume_alc_pot(cnr, kind, tier);

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let duration_ticks = 10 * 60 * TICKS_PER_SECOND as u32;
        let expire_tick = start_tick.wrapping_add(duration_ticks);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Potion Spell".to_string(),
            description: "A potion spell.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index,
            modifier_value,
            x: 0,
            y: 0,
            carried_by: Some(character_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_POTION_SP,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(character) = self.characters.get_mut(&character_id) {
            if character.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            character.inventory[slot] = Some(item_id);
            character_serial = character.id.0;
            character
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }

        self.update_character(character_id);
        self.schedule_spell_remove_timer(character_id, item_id, slot, character_serial, item_id.0);
        self.npc_emote(
            character_id,
            &format!("drinks a stat potion ({kind},{tier}-{strength})"),
        );
        true
    }
}
