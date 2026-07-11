//! Warp Fighter driver (`CDR_WARPFIGHTER`), the hired opponent
//! `warptrialdoor_driver` (`crates/ugaris-core/src/item_driver/
//! area25_warped.rs`, `ItemDriverOutcome::WarpTrialDoor`) spawns inside a
//! Warped World trial room.
//!
//! Ports `src/area/25/warped.c::warpfighter` (`:864-969`), `warpfighter_
//! died` (`:971-991`), and the stat-raising helper `warped_raise`
//! (`:487-608`) that scales the fighter's skills to the summoning player's
//! `warped_ppd.base` progress.
//!
//! `warped_raise`'s five "spell of equipment" item attachments
//! (`equip1`/`equip2`/`equip3`/`armor_spell`/`weapon_spell`, carried in
//! non-worn inventory slots 12-16) need `ZoneLoader`, which `World` cannot
//! see - [`apply_warped_raise`] here only ports the pure stat-rescale/exp/
//! profession half; the equipment half lives in `ugaris-server`'s
//! `spawns::spawn_warp_trial_fighter` (same "pure core + ZoneLoader-needing
//! tail in the binary crate" split as `crates/ugaris-server/src/
//! dungeon.rs`'s `build_warrior`/`build_mage`/`build_seyan`, which this
//! module's equipment-item shape closely mirrors).
//!
//! Deviations/gaps (documented, not silent):
//! - `warpfighter`'s `pot_done<1` "drinks a potion of freeze" branch
//!   (`:907-930`) has two `RANDOM(2)` outcomes: a self-buff (pure stat
//!   mutation, ported in [`World::warpfighter_maybe_drink_freeze_potion`])
//!   and a "spoiled potion" self-curse that calls `create_item(
//!   "freeze_spell")` + `create_spell_timer` (needs `ZoneLoader` and a
//!   spell-timer-creation mechanism this codebase has not ported anywhere
//!   yet - see `crates/ugaris-core/src/spell.rs`). The self-buff branch is
//!   ported; the spoiled-potion branch is a documented no-op (the outer
//!   `RANDOM(2)` roll is still drawn, preserving the RNG draw sequence for
//!   anything sharing the seed, but nothing happens on that branch).
//! - `warpfighter_died`'s `dat->co != co` guard means only the *owning
//!   player's own killing blow* teleports them back - ported as
//!   `crate::world_events::death_hooks::apply_warpfighter_death_from_hurt_event`
//!   (`ugaris-server`), since it needs the generic `LegacyHurtEvent`
//!   cause/target pair.

use crate::legacy::profession;
use crate::world::*;

/// C `fight_driver_set_dist(cn, 40, 0, 40)` (`warped.c:880`).
pub const WARPFIGHTER_START_DIST: i32 = 40;
/// C `fight_driver_set_dist(cn, 40, 0, 40)` (`warped.c:880`).
pub const WARPFIGHTER_STOP_DIST: i32 = 40;
/// C `TICKS * 2` (`warped.c:907`): delay before the first potion check.
const WARPFIGHTER_POTION_DELAY: u64 = TICKS_PER_SECOND * 2;

/// C `struct warpfighter_data` (`warped.c:234-240`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WarpFighterDriverData {
    /// C `dat->co`: the summoning player's character id.
    pub owner: CharacterId,
    /// C `dat->cser`: the summoning player's serial, guarding against a
    /// reused character-id slot.
    pub owner_serial: u32,
    /// C `dat->tx`/`dat->ty`: where `warpfighter_died` teleports the owner
    /// back to.
    pub tx: u16,
    pub ty: u16,
    /// C `dat->xs`/`dat->xe`/`dat->ys`/`dat->ye`: the trial room bounds the
    /// owner must stay inside.
    pub xs: u16,
    pub xe: u16,
    pub ys: u16,
    pub ye: u16,
    /// C `dat->creation_time`.
    pub creation_time: u64,
    /// C `dat->pot_done`.
    pub pot_done: i32,
}

impl Default for WarpFighterDriverData {
    fn default() -> Self {
        Self {
            owner: CharacterId(0),
            owner_serial: 0,
            tx: 0,
            ty: 0,
            xs: 0,
            xe: 0,
            ys: 0,
            ye: 0,
            creation_time: 0,
            pot_done: 0,
        }
    }
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_WARPFIGHTER`
    /// characters (C `ch_driver`'s `CDR_WARPFIGHTER` case,
    /// `warped.c:1164-1166`).
    pub fn process_warpfighter_actions(&mut self, area_id: u16) -> usize {
        let fighter_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_WARPFIGHTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for fighter_id in fighter_ids {
            let mut seed = self.legacy_random_seed;
            let did_act = {
                let mut random = |below: u32| legacy_random_below_from_seed(&mut seed, below);
                self.process_warpfighter_tick(fighter_id, area_id, &mut random)
            };
            self.legacy_random_seed = seed;
            if did_act {
                acted += 1;
            }
        }
        acted
    }

    /// C `warpfighter`'s per-tick body (`warped.c:864-969`).
    fn process_warpfighter_tick(
        &mut self,
        fighter_id: CharacterId,
        area_id: u16,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let mut data = match self
            .characters
            .get(&fighter_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::WarpFighter(data)) => data,
            _ => WarpFighterDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&fighter_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                // C `case NT_TEXT: co = msg->dat3; tabunga(cn, co,
                // (char*)msg->dat2);` (`warped.c:884-887`).
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if let Some(text) = message.text.as_deref() {
                        self.apply_tabunga_text_notification(fighter_id, speaker_id, text);
                    }
                }
                // C `standard_message_driver(cn, msg, 1, 0)`'s `NT_CHAR`
                // branch (`drvlib.c:2467-2473`): `agressive=1` auto-aggro
                // on sight, `require_visible`, no `hurtme`.
                NT_CHAR if message.dat1 > 0 => {
                    let target_id = CharacterId(message.dat1 as u32);
                    self.warpfighter_add_standard_enemy(fighter_id, target_id, 0, true, false);
                }
                // C `standard_message_driver`'s `NT_GOTHIT` branch
                // (`drvlib.c:2507-2528`): unconditional self-defense plus
                // `fight_driver_note_hit`.
                NT_GOTHIT if message.dat1 > 0 => {
                    let tick = self.tick.0 as i32;
                    if let Some(character) = self.characters.get_mut(&fighter_id) {
                        character
                            .fight_driver
                            .get_or_insert_with(FightDriverData::default)
                            .last_hit = tick;
                    }
                    let target_id = CharacterId(message.dat1 as u32);
                    self.warpfighter_add_standard_enemy(fighter_id, target_id, 1, false, true);
                }
                _ => {}
            }
        }

        // C `co = dat->co; if (!ch[co].flags || ch[co].serial != dat->cser
        // || ch[co].x < dat->xs || ch[co].y < dat->ys || ch[co].x > dat->xe
        // || ch[co].y > dat->ye) { remove_char(cn); destroy_char(cn);
        // return; }` (`warped.c:897-905`).
        let owner_valid = self.characters.get(&data.owner).is_some_and(|owner| {
            owner.serial == data.owner_serial
                && owner.x >= data.xs
                && owner.x <= data.xe
                && owner.y >= data.ys
                && owner.y <= data.ye
        });
        if !owner_valid {
            self.remove_character(fighter_id);
            return true;
        }

        self.warpfighter_maybe_drink_freeze_potion(fighter_id, &mut data, random);
        self.warpfighter_maybe_drink_endurance_potion(fighter_id, &mut data, random);
        self.warpfighter_maybe_drink_healing_potion(fighter_id, &mut data, random);

        if let Some(character) = self.characters.get_mut(&fighter_id) {
            character.driver_state = Some(CharacterDriverState::WarpFighter(data));
        }

        // C `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
        // 0)) return; if (fight_driver_follow_invisible(cn)) return;`
        // (`warped.c:949-955`).
        let Some(attacker) = self.characters.get(&fighter_id).cloned() else {
            return false;
        };
        if self.fight_driver_attack_visible_and_follow(
            fighter_id,
            &attacker,
            area_id,
            FightDriverSuppressions::default(),
            true,
            random,
        ) {
            return true;
        }

        // C `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return;` (`warped.c:957-962`).
        if self.regenerate_simple_baddy(fighter_id) {
            return true;
        }
        if self.spell_self_simple_baddy(fighter_id) {
            return true;
        }

        // C `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)` (`warped.c:964-966`): the fighter's post position
        // (C's `tmpx`/`tmpy`, set to its own spawn tile at creation) reuses
        // `rest_x`/`rest_y`, the same substitution every other stationary
        // NPC in this codebase uses.
        let (post_x, post_y) = self
            .characters
            .get(&fighter_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            fighter_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`warped.c:968`).
        self.idle_simple_baddy(fighter_id)
    }

    /// C `standard_message_driver`'s enemy-add half (`drvlib.c:2470-2473,
    /// 2519-2527`), shared by the `NT_CHAR`/`NT_GOTHIT` cases above - same
    /// shape as `SimpleBaddyMessageOutcome::StandardAggro`'s own applier in
    /// `world/npc_messages.rs`, reimplemented directly here since
    /// `CDR_WARPFIGHTER` is not a `CharacterDriverState::SimpleBaddy`.
    fn warpfighter_add_standard_enemy(
        &mut self,
        fighter_id: CharacterId,
        target_id: CharacterId,
        priority: i32,
        require_visible: bool,
        hurtme: bool,
    ) {
        if !self.simple_baddy_can_add_standard_enemy(fighter_id, target_id, require_visible, hurtme)
        {
            return;
        }
        let tick = self.tick.0 as i32;
        let tracking = self.simple_baddy_enemy_tracking(fighter_id, target_id);
        if let Some(character) = self.characters.get_mut(&fighter_id) {
            let _ = add_simple_baddy_enemy_unchecked(character, target_id, priority, tick);
            Self::apply_simple_baddy_enemy_tracking(character, target_id, tracking);
        }
        self.sort_simple_baddy_enemies_like_c(fighter_id);
    }

    /// C `warpfighter`'s `pot_done<1` "drinks a potion of freeze" branch
    /// (`warped.c:907-930`). See module doc comment for the deferred
    /// "spoiled potion" sub-branch.
    fn warpfighter_maybe_drink_freeze_potion(
        &mut self,
        fighter_id: CharacterId,
        data: &mut WarpFighterDriverData,
        random: &mut impl FnMut(u32) -> u32,
    ) {
        if data.pot_done >= 1 || self.tick.0 <= data.creation_time + WARPFIGHTER_POTION_DELAY {
            return;
        }
        data.pot_done += 1;
        let level = self
            .characters
            .get(&fighter_id)
            .map(|character| character.level)
            .unwrap_or(0);
        if level <= 60 || random(6) != 0 {
            return;
        }
        if random(2) != 0 {
            self.npc_emote(fighter_id, "drinks a potion of freeze");
            if let Some(character) = self.characters.get_mut(&fighter_id) {
                let attack = character.values[1][CharacterValue::Attack as usize];
                character.values[1][CharacterValue::Freeze as usize] = attack + attack / 4;
                character.values[1][CharacterValue::Mana as usize] = 10;
            }
            self.update_character(fighter_id);
            if let Some(character) = self.characters.get_mut(&fighter_id) {
                character.mana = POWERSCALE * 10;
            }
        }
        // else: the "spoiled potion" self-curse item - deferred, see
        // module doc comment.
    }

    /// C `warpfighter`'s `pot_done<3` "drinks an endurance potion" branch
    /// (`warped.c:932-939`).
    fn warpfighter_maybe_drink_endurance_potion(
        &mut self,
        fighter_id: CharacterId,
        data: &mut WarpFighterDriverData,
        random: &mut impl FnMut(u32) -> u32,
    ) {
        let Some(character) = self.characters.get(&fighter_id).cloned() else {
            return;
        };
        let warcry_max = character_value(&character, CharacterValue::Warcry) * POWERSCALE / 3;
        if character.lifeshield >= POWERSCALE * 5
            || character.endurance >= warcry_max
            || data.pot_done >= 3
        {
            return;
        }
        data.pot_done += 1;
        if character.level <= 50 || random(4) != 0 {
            return;
        }
        self.npc_emote(fighter_id, "drinks an endurance potion");
        if let Some(character) = self.characters.get_mut(&fighter_id) {
            let max_endurance = character_value(character, CharacterValue::Endurance) * POWERSCALE;
            character.endurance = max_endurance.min(character.endurance + 32 * POWERSCALE);
        }
    }

    /// C `warpfighter`'s `pot_done<5` "drinks a healing potion" branch
    /// (`warped.c:941-947`).
    fn warpfighter_maybe_drink_healing_potion(
        &mut self,
        fighter_id: CharacterId,
        data: &mut WarpFighterDriverData,
        random: &mut impl FnMut(u32) -> u32,
    ) {
        let Some(character) = self.characters.get(&fighter_id).cloned() else {
            return;
        };
        let half_endurance =
            character_value(&character, CharacterValue::Endurance) * POWERSCALE / 2;
        if character.hp >= half_endurance || data.pot_done >= 5 {
            return;
        }
        data.pot_done += 1;
        if character.level <= 40 || random(4) != 0 {
            return;
        }
        self.npc_emote(fighter_id, "drinks a healing potion");
        if let Some(character) = self.characters.get_mut(&fighter_id) {
            let max_hp = character_value(character, CharacterValue::Hp) * POWERSCALE;
            character.hp = max_hp.min(character.hp + 32 * POWERSCALE);
        }
    }
}

/// C `warped_raise`'s per-skill `switch (n)` formula body
/// (`warped.c:498-563`), applied before the shared `min(val, 120)` clamp.
/// Notably `V_MANA` is *not* cased (falls to `default: base - 40`) despite
/// being a "power" stat like `V_HP`/`V_ENDURANCE` which *do* get their own
/// case - a real C quirk, preserved verbatim.
fn warped_raise_scaled_value(index: usize, base: i32) -> i32 {
    use crate::entity::CharacterValue::*;
    match index {
        i if i == Hp as usize || i == Endurance as usize => (base - base / 4).max(10),
        i if i == Wisdom as usize => (base - base / 5).max(10),
        i if i == Intelligence as usize || i == Agility as usize || i == Strength as usize => {
            (base - base / 10).max(10)
        }
        i if i == Hand as usize => base.max(1),
        i if i == ArmorSkill as usize => ((base / 10) * 10).max(1),
        i if i == Attack as usize
            || i == Parry as usize
            || i == Immunity as usize
            || i == Warcry as usize =>
        {
            base.max(1)
        }
        i if i == Tactics as usize => (base - 5).max(1),
        i if i == Surround as usize || i == BodyControl as usize => (base - 20).max(1),
        i if i == SpeedSkill as usize || i == Percept as usize => (base - 10).max(1),
        i if i == Rage as usize => (base - 5).max(1),
        i if i == Profession as usize => (base - 5).max(1).min(60),
        _ => (base - 40).max(1),
    }
    .min(120)
}

/// C `skill[n].cost` (`src/system/skill.c:27-71`) is zero only for
/// `V_ARMOR`(7)/`V_WEAPON`(8)/`V_LIGHT`(9)/`V_SPEED`(10)/`V_DEMON`(38)/
/// `V_COLD`(41) - the non-raisable "derived" values `warped_raise`'s
/// `if (!skill[n].cost) continue;` guard skips.
fn is_warped_raisable_value(index: usize) -> bool {
    !matches!(index, 7 | 8 | 9 | 10 | 38 | 41)
}

/// C `warped_raise(cn, base)`'s pure stat-rescale/exp/level/profession half
/// (`warped.c:487-576`) - the "spell of equipment" item attachments
/// (`:577-608`) need `ZoneLoader`; see module doc comment.
pub fn apply_warped_raise(character: &mut Character, base: i32) {
    for index in 0..CHARACTER_VALUE_COUNT {
        if !is_warped_raisable_value(index) {
            continue;
        }
        if character.values[1][index] == 0 {
            continue;
        }
        character.values[1][index] = warped_raise_scaled_value(index, base) as i16;
    }

    let exp = calc_exp(character);
    character.exp = exp;
    character.exp_used = exp;
    character.level = exp2level(exp);

    // C `ch[cn].prof[P_LIGHT] = min(30, value[1][V_PROFESSION]);
    // ch[cn].prof[P_DARK] = min(30, value[1][V_PROFESSION]); if
    // (value[1][V_PROFESSION] > 30) ch[cn].prof[P_ATHLETE] = min(30,
    // value[1][V_PROFESSION] - 30);` (`warped.c:571-575`).
    let profession_value = i32::from(character.values[1][CharacterValue::Profession as usize]);
    character.professions[profession::LIGHT] = profession_value.min(30) as i16;
    character.professions[profession::DARK] = profession_value.min(30) as i16;
    if profession_value > 30 {
        character.professions[profession::ATHLETE] = (profession_value - 30).min(30) as i16;
    }
}

/// C `it[in].mod_value[n] = 1 + base / 2.75;` (`warped.c:580,587,594`),
/// shared by `equip1`/`equip2`/`equip3`.
pub fn warped_equip_mod_value(base: i32) -> i16 {
    (1.0 + f64::from(base) / 2.75) as i16
}

/// C `it[in].mod_value[0] = max(13, min(123, ch[cn].value[1][V_ARMORSKILL]
/// + 10)) * 20;` (`warped.c:602`).
pub fn warped_armor_spell_mod_value(character: &Character) -> i16 {
    let armor_skill = i32::from(character.values[1][CharacterValue::ArmorSkill as usize]);
    ((armor_skill + 10).clamp(13, 123) * 20) as i16
}

/// C `it[in].mod_value[0] = max(13, min(123, ch[cn].value[1][V_HAND] +
/// 10));` (`warped.c:607`).
pub fn warped_weapon_spell_mod_value(character: &Character) -> i16 {
    let hand = i32::from(character.values[1][CharacterValue::Hand as usize]);
    (hand + 10).clamp(13, 123) as i16
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_WARPFIGHTER;
