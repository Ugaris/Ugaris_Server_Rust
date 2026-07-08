//! Two-City guard NPC (`CDR_TWOGUARD`), Exkordon's territory-enforcement
//! patrol.
//!
//! Ports `src/area/17/two.c::guard_driver` (`:325-742`) plus its
//! `guard_dead` death hook (`:744-769`, ported as
//! `crate::world::hurt::apply_two_guard_death_from_hurt_event` in
//! `ugaris-server`'s `world_events::death_hooks`, since it needs
//! `PlayerRuntime`).
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification as every other ported
//!   NPC (see `world::asturin`'s module doc comment) for the generic
//!   `fight_driver_attack_visible`/`fight_driver_follow_invisible`
//!   cascade. Unlike most single-victim NPCs, this driver's own
//!   `dat->current_victim` field already *is* single-victim in C (it
//!   tracks the one player currently being warned/pursued for the leave/
//!   fine dialogue ladder), so this port's `TwoGuardDriverData::victim`
//!   directly represents C's `dat->current_victim`/`dat->last_x`/
//!   `dat->last_y`/`dat->last_co` combined (all four are always written
//!   together at every C call site) - no extra simplification needed
//!   beyond the usual generic-multi-enemy-system collapse.
//! - `player_driver_stop(ch[co].player, 1)` (`two.c:542`, the "pay"
//!   text command's success branch, cancelling the payer's queued
//!   movement) is not ported - no `player_driver_stop` equivalent exists
//!   anywhere in the tree yet (a cross-cutting, not guard-specific, gap).
//! - The "pay" text command's bank-account fallback (`two.c:518-536`) IS
//!   ported (`TwoGuardOutcomeEvent::UpdatePpd::bank_gold_deduction`),
//!   unlike `world::barkeeper`'s "buy pass" (which has no bank fallback
//!   in C to begin with - not a missed precedent).
//! - `fight_driver_note_hit(cn)` (inside `standard_message_driver`'s
//!   `NT_GOTHIT` case): server-logfile-only bookkeeping, not ported (same
//!   precedent as every other ported NPC).
//! - `fight_driver_remove_enemy(cn, co)` (`two.c:540`, the "pay" success
//!   branch, when the payer is now standing somewhere legal): ported as
//!   clearing `data.victim` when it matches the payer.
//! - Every real C quirk called out inline below (the `guard_intro`
//!   one-time-only guest speech gate, the `CS_GUEST`-branch `last_attack`
//!   omission in the `NTID_TWOCITY_PICK` handler, the mismatched-
//!   coordinate `call_guard` alert) is preserved digit-for-digit, not
//!   "fixed".

use std::collections::HashMap;

use crate::character_driver::{next_legacy_name_value, CDR_TWOGUARD};
use crate::item_driver::IDR_TORCH;
use crate::world::*;

/// C `ch[cn].item[WN_LHAND]` slot index, same as `world::robber`'s own
/// `ROBBER_TORCH_SLOT` (`crate::zone`'s `NAMES` ordering).
const TORCH_SLOT: usize = 8;
/// C `#define MAXPAT 8` (`two.c:280`).
const MAXPAT: usize = 8;
/// C `TICKS * 3` (`two.c:666`): drop the tracked victim/warning ladder
/// once nobody has re-triggered it this long.
const VICTIM_TIMEOUT_TICKS: u64 = TICKS_PER_SECOND * 3;
/// C `TICKS * 10` (`two.c:427`): leave-warning repeat interval. Used by
/// `guard_messages.rs`'s `NT_CHAR` handler too, hence `pub(super)`.
pub(super) const LEAVE_WARN_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`two.c:424`): leave-warning escalate timeout for
/// non-guest territory violations.
pub(super) const LEAVE_TIMEOUT_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS * 60` (`two.c:422`): leave-warning escalate timeout for
/// `CS_GUEST`-level territory violations.
pub(super) const LEAVE_TIMEOUT_GUEST_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `TICKS * 15` (`two.c:469`): fine-warning repeat interval.
pub(super) const FINE_WARN_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 15;
/// C `TICKS * 60` (`two.c:474`): fine-warning escalate timeout.
pub(super) const FINE_TIMEOUT_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `TICKS * 30` (`two.c:577`): "help!" alert repeat cooldown.
pub(super) const ALERT_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `TICKS * 3` (`two.c:619,660`): "just resolved, don't re-engage"
/// cooldown gating `standard_message_driver`'s `NT_GOTHIT`/`NT_SEEHIT`
/// enemy-adding.
pub(super) const NOFIGHT_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 3;
/// C `TICKS * 10` (`two.c:710`): give up on a called-to-help destination
/// if stuck this long.
const CALLED_GIVEUP_TICKS: u64 = TICKS_PER_SECOND * 10;

/// Per-player facts [`World::process_two_guard_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoGuardPlayerFacts {
    /// `PlayerRuntime::twocity_legal_status()`.
    pub legal_status: i32,
    /// `PlayerRuntime::twocity_legal_fine()`.
    pub legal_fine: i32,
    /// `PlayerRuntime::twocity_citizen_status()`.
    pub citizen_status: i32,
    /// `PlayerRuntime::twocity_current_guard()` (a `CharacterId.0`, or `0`
    /// for none).
    pub current_guard: i32,
    /// `PlayerRuntime::twocity_current_guard_time()` (wall-clock
    /// `realtime` seconds).
    pub current_guard_time: i32,
    /// `PlayerRuntime::twocity_last_attack()` (wall-clock `realtime`
    /// seconds).
    pub last_attack: i32,
    /// `PlayerRuntime::twocity_guard_intro()`.
    pub guard_intro: i32,
    /// `PlayerRuntime::bank_gold` (C `struct bank_ppd::imperial_gold`),
    /// the "pay" text command's bank-account fallback.
    pub bank_gold: i32,
}

/// A side effect [`World::process_two_guard_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment. Carries a full new `twocity_ppd` snapshot (rather than one
/// event type per field) since almost every branch of this driver writes
/// more than one of the seven fields at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoGuardOutcomeEvent {
    pub player_id: CharacterId,
    pub legal_status: i32,
    pub legal_fine: i32,
    pub citizen_status: i32,
    pub current_guard: i32,
    pub current_guard_time: i32,
    pub last_attack: i32,
    pub guard_intro: i32,
    /// `Some(need)` if `need` raw gold units should additionally be
    /// deducted from the player's persistent bank balance (`two.c:524-
    /// 532`, the "pay" text command's bank-account fallback);
    /// `Character.gold` itself (already visible to `World`) is always
    /// mutated directly, never through this event.
    pub bank_gold_deduction: Option<i32>,
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_TWOGUARD`
    /// characters (C `ch_driver`'s `CDR_TWOGUARD` case, `two.c:3148-
    /// 3149`).
    pub fn process_two_guard_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TwoGuardPlayerFacts>,
        now: i32,
        zone_loader: &mut ZoneLoader,
        area_id: u16,
    ) -> Vec<TwoGuardOutcomeEvent> {
        let guard_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TWOGUARD
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for guard_id in guard_ids {
            self.process_two_guard_tick(
                guard_id,
                player_facts,
                now,
                zone_loader,
                area_id,
                &mut events,
            );
        }
        events
    }

    /// C `guard_driver`'s per-tick body (`two.c:325-742`).
    #[allow(clippy::too_many_arguments)]
    fn process_two_guard_tick(
        &mut self,
        guard_id: CharacterId,
        player_facts: &HashMap<CharacterId, TwoGuardPlayerFacts>,
        now: i32,
        zone_loader: &mut ZoneLoader,
        area_id: u16,
        events: &mut Vec<TwoGuardOutcomeEvent>,
    ) -> bool {
        let mut data = match self
            .characters
            .get(&guard_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::TwoGuard(data)) => data,
            _ => TwoGuardDriverData::default(),
        };

        // C `two.c:337-376`: torch on/off day-night sensor, unconditional
        // every tick, before the message loop.
        self.two_guard_maintain_torch(guard_id, zone_loader, area_id);

        let messages = self
            .characters
            .get_mut(&guard_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_CHAR if message.dat1 > 0 => {
                    self.two_guard_handle_char(
                        guard_id,
                        message,
                        player_facts,
                        now,
                        events,
                        &mut data,
                    );
                }
                NT_TEXT => {
                    self.two_guard_handle_text(
                        guard_id,
                        message,
                        player_facts,
                        now,
                        events,
                        &mut data,
                    );
                }
                NT_GOTHIT if message.dat1 > 0 => {
                    self.two_guard_handle_gothit(
                        guard_id,
                        message,
                        player_facts,
                        now,
                        events,
                        &mut data,
                    );
                }
                NT_SEEHIT => {
                    self.two_guard_handle_seehit(
                        guard_id,
                        message,
                        player_facts,
                        now,
                        events,
                        &mut data,
                    );
                }
                NT_NPC => {
                    self.two_guard_handle_npc(
                        guard_id,
                        message,
                        player_facts,
                        now,
                        events,
                        &mut data,
                    );
                }
                _ => {}
            }
        }

        // C `two.c:666-668`.
        if data.victim_timeout.saturating_add(VICTIM_TIMEOUT_TICKS) < self.tick.0 {
            data.victim_timeout = 0;
            data.victim = None;
            data.fine_state = 0;
            data.leave_state = 0;
            data.leave_timeout = 0;
        }

        // C `fight_driver_update(cn)`: refresh the tracked victim's
        // visibility/last-seen position, or drop it once it's gone.
        if let Some(victim_id) = data.victim {
            match self
                .characters
                .get(&guard_id)
                .cloned()
                .zip(self.characters.get(&victim_id).cloned())
            {
                Some((guard, victim)) if !victim.flags.contains(CharacterFlags::DEAD) => {
                    if char_see_char(&guard, &victim, &self.map, self.date.daylight) {
                        data.victim_visible = true;
                        data.victim_last_x = victim.x;
                        data.victim_last_y = victim.y;
                    } else {
                        data.victim_visible = false;
                    }
                }
                _ => {
                    data.victim = None;
                    data.victim_visible = false;
                }
            }
        }

        // C `if (fight_driver_attack_visible(cn, 0)) { dat->busy = 1;
        // return; }`.
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(guard_id, victim_id, area_id) {
                    data.busy = true;
                    self.two_guard_save(guard_id, data);
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) { dat->busy = 1;
            // return; }`.
            let arrived = self.characters.get(&guard_id).is_some_and(|guard| {
                guard.x.abs_diff(data.victim_last_x) < 2 && guard.y.abs_diff(data.victim_last_y) < 2
            });
            if arrived {
                data.victim = None;
            } else if self.secure_move_driver(
                guard_id,
                data.victim_last_x,
                data.victim_last_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            ) {
                data.busy = true;
                self.two_guard_save(guard_id, data);
                return true;
            }
        }

        // C `if (spell_self_driver(cn)) { dat->busy = 1; return; }`.
        if self.spell_self_simple_baddy(guard_id) {
            data.busy = true;
            self.two_guard_save(guard_id, data);
            return true;
        }
        // C `if (regenerate_driver(cn)) { dat->busy = 1; return; }`.
        if self.regenerate_simple_baddy(guard_id) {
            data.busy = true;
            self.two_guard_save(guard_id, data);
            return true;
        }

        data.busy = false;

        // C `two.c:692-699`: fallback follow toward the last-known
        // position of a still-tracked victim, then a half-tick idle,
        // unconditionally ending the tick either way.
        if data.victim.is_some() {
            if !data.victim_visible
                && self.setup_walk_toward(
                    guard_id,
                    usize::from(data.victim_last_x),
                    usize::from(data.victim_last_y),
                    1,
                    area_id,
                    false,
                )
            {
                self.two_guard_save(guard_id, data);
                return true;
            }
            let idled = self
                .characters
                .get_mut(&guard_id)
                .is_some_and(|character| do_idle(character, (TICKS_PER_SECOND / 2) as i32).is_ok());
            self.two_guard_save(guard_id, data);
            return idled;
        }

        // C `two.c:701-713`: called-to-help destination.
        if data.tx != 0 {
            let Some(guard) = self.characters.get(&guard_id) else {
                self.two_guard_save(guard_id, data);
                return false;
            };
            if (guard.x.abs_diff(data.tx) as i32) + (guard.y.abs_diff(data.ty) as i32) < 2 {
                data.tx = 0;
                data.ty = 0;
            } else if self.setup_walk_toward(
                guard_id,
                usize::from(data.tx),
                usize::from(data.ty),
                1,
                area_id,
                true,
            ) || self.setup_walk_toward(
                guard_id,
                usize::from(data.tx),
                usize::from(data.ty),
                3,
                area_id,
                true,
            ) {
                data.good_tx_try = self.tick.0;
                self.two_guard_save(guard_id, data);
                return true;
            } else if self.tick.0.saturating_sub(data.good_tx_try) > CALLED_GIVEUP_TICKS {
                data.tx = 0;
                data.ty = 0;
            }
        }

        // C `two.c:715-731`: patrol waypoints. C's `dat->pi` is a plain
        // array index that can momentarily read one past the end of
        // `patx[MAXPAT]` (aliasing into the adjacent `paty[0]` field in
        // memory) before the very next line's `if (!dat->patx[dat->pi])
        // dat->pi = 0;` resets it - an obscure C struct-layout
        // coincidence, not deliberate behavior, so this port simply
        // treats an out-of-range `pi` as "at the end" (`patx[pi] == 0`)
        // instead of reproducing the aliasing read.
        if data.patx[0] != 0 {
            let pi = usize::from(data.pi);
            let (target_x, target_y) = (
                data.patx.get(pi).copied().unwrap_or(0),
                data.paty.get(pi).copied().unwrap_or(0),
            );
            if let Some(guard) = self.characters.get(&guard_id) {
                if guard.x.abs_diff(u16::from(target_x)) < 4
                    && guard.y.abs_diff(u16::from(target_y)) < 4
                {
                    data.pi = data.pi.wrapping_add(1);
                }
            }
            if data.patx.get(usize::from(data.pi)).copied().unwrap_or(0) == 0 {
                data.pi = 0;
            }
            let pi = usize::from(data.pi);
            let (target_x, target_y) = (
                data.patx.get(pi).copied().unwrap_or(0),
                data.paty.get(pi).copied().unwrap_or(0),
            );
            if self.setup_walk_toward(
                guard_id,
                usize::from(target_x),
                usize::from(target_y),
                3,
                area_id,
                true,
            ) {
                self.two_guard_save(guard_id, data);
                return true;
            }
            data.pi = data.pi.wrapping_add(1);
            if data.patx.get(usize::from(data.pi)).copied().unwrap_or(0) == 0 {
                data.pi = 0;
            }
        }

        // C `two.c:733-741`: return to the spawn post (`ch[cn].tmpx/
        // tmpy`, this port's `rest_x`/`rest_y`).
        let Some((gx, gy, post_x, post_y)) = self
            .characters
            .get(&guard_id)
            .map(|guard| (guard.x, guard.y, guard.rest_x, guard.rest_y))
        else {
            self.two_guard_save(guard_id, data);
            return false;
        };
        if gx == post_x && gy == post_y {
            let idled = self
                .characters
                .get_mut(&guard_id)
                .is_some_and(|character| do_idle(character, TICKS_PER_SECOND as i32).is_ok());
            self.two_guard_save(guard_id, data);
            return idled;
        }
        if self.setup_walk_toward(
            guard_id,
            usize::from(post_x),
            usize::from(post_y),
            0,
            area_id,
            true,
        ) {
            self.two_guard_save(guard_id, data);
            return true;
        }
        if self.secure_move_driver(
            guard_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            self.two_guard_save(guard_id, data);
            return true;
        }

        let idled = self
            .characters
            .get_mut(&guard_id)
            .is_some_and(|character| do_idle(character, TICKS_PER_SECOND as i32).is_ok());
        self.two_guard_save(guard_id, data);
        idled
    }

    /// C `two.c:337-376`: keep a hand torch lit at night/in the dark,
    /// unlit in daylight, mirroring `world::robber`'s own torch-upkeep
    /// helper but with the real two-signal on/off sensor instead of
    /// "always relight".
    fn two_guard_maintain_torch(
        &mut self,
        guard_id: CharacterId,
        zone_loader: &mut ZoneLoader,
        area_id: u16,
    ) {
        let Some(torch_slot) = self
            .characters
            .get(&guard_id)
            .and_then(|guard| guard.inventory.get(TORCH_SLOT).copied())
        else {
            return;
        };

        let item_id = match torch_slot {
            None => {
                let Ok(item) = zone_loader.instantiate_item_template("torch", Some(guard_id))
                else {
                    return;
                };
                let item_id = item.id;
                self.items.insert(item_id, item);
                if let Some(guard) = self.characters.get_mut(&guard_id) {
                    guard.inventory[TORCH_SLOT] = Some(item_id);
                }
                // C's own torch-creation snippet (`two.c:337-341`) does
                // NOT call `update_char(cn)` after assigning the new
                // torch to `WN_LHAND`, unlike `world::robber`'s own
                // torch-upkeep helper (`gwendylon.c:3817-3828`, which
                // does) - a real difference between the two files'
                // otherwise near-identical snippets, preserved as-is.
                item_id
            }
            Some(item_id) => item_id,
        };

        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return;
        };
        let tlight = character_value(&guard, CharacterValue::Light);
        let Some(tile) = self.map.tile(usize::from(guard.x), usize::from(guard.y)) else {
            return;
        };

        let mut on = 0;
        let mut off = 0;
        // C `check_dlight(x, y)` (`tool.c:3339-3347`).
        let dlight = (self.date.daylight * i32::from(tile.daylight)) / 256;
        if dlight < 40 {
            on += 1;
        }
        if dlight > 50 {
            off += 1;
        }
        // C `check_light(x, y)` (`tool.c:3349-3356`).
        let light = i32::from(tile.light);
        if light < 10 {
            on += 1;
        }
        if light - tlight > 10 {
            off += 1;
        }

        let lit = self
            .items
            .get(&item_id)
            .is_some_and(|item| item.driver_data.first().copied().unwrap_or(0) != 0);
        if !lit && on == 2 {
            let _ = self.execute_item_driver_request(
                ItemDriverRequest::Driver {
                    driver: IDR_TORCH,
                    item_id,
                    character_id: guard_id,
                    spec: 0,
                },
                area_id,
            );
        }
        if lit && off != 0 {
            let _ = self.execute_item_driver_request(
                ItemDriverRequest::Driver {
                    driver: IDR_TORCH,
                    item_id,
                    character_id: guard_id,
                    spec: 0,
                },
                area_id,
            );
        }

        let now_lit = self
            .items
            .get(&item_id)
            .is_some_and(|item| item.driver_data.first().copied().unwrap_or(0) != 0);
        if let Some(guard) = self.characters.get_mut(&guard_id) {
            guard.sprite = if now_lit { 317 } else { 318 };
        }
    }

    fn two_guard_save(&mut self, guard_id: CharacterId, data: TwoGuardDriverData) {
        if let Some(character) = self.characters.get_mut(&guard_id) {
            character.driver_state = Some(CharacterDriverState::TwoGuard(data));
        }
    }
}

/// C `guard_parse` (`two.c:302-323`): the same `pat`-index-shared-across-
/// repeated-`patx=N;paty=N;`-pairs parsing shape as `world::npc::area11::
/// palace_guard`'s `parse_palace_guard_driver_args`.
pub fn parse_two_guard_driver_args(args: &str) -> TwoGuardDriverData {
    let mut data = TwoGuardDriverData::default();
    let mut pat: usize = 0;
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "patx" => {
                if pat < MAXPAT {
                    data.patx[pat] = parsed as u8;
                }
            }
            "paty" => {
                if pat < MAXPAT {
                    data.paty[pat] = parsed as u8;
                    pat += 1;
                }
            }
            _ => {} // C: `elog(...)` - log-only.
        }
        rest = next;
    }
    data
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

/// C `struct twoguard_data` (`src/area/17/two.c:281-300`): `dat->last_x`/
/// `dat->last_y`/`dat->last_co` collapse into `victim`/`victim_last_x`/
/// `victim_last_y` (see the module doc comment); `dat->busy` is ported
/// verbatim (it gates the `NT_CHAR` handler using *last* tick's value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoGuardDriverData {
    pub victim: Option<CharacterId>,
    pub victim_timeout: u64,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
    pub fine_state: i32,
    pub fine_timeout: u64,
    pub leave_state: i32,
    pub leave_timeout: u64,
    pub lastsay: u64,
    pub tx: u16,
    pub ty: u16,
    pub lastalert: u64,
    pub good_tx_try: u64,
    pub nofight_timer: u64,
    pub patx: [u8; MAXPAT],
    pub paty: [u8; MAXPAT],
    pub pi: u8,
    pub busy: bool,
}

impl Default for TwoGuardDriverData {
    fn default() -> Self {
        Self {
            victim: None,
            victim_timeout: 0,
            victim_visible: false,
            victim_last_x: 0,
            victim_last_y: 0,
            fine_state: 0,
            fine_timeout: 0,
            leave_state: 0,
            leave_timeout: 0,
            lastsay: 0,
            tx: 0,
            ty: 0,
            lastalert: 0,
            good_tx_try: 0,
            nofight_timer: 0,
            patx: [0; MAXPAT],
            paty: [0; MAXPAT],
            pi: 0,
            busy: false,
        }
    }
}
