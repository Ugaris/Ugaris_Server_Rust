//! Palace patrol/reserve-ambush demon sentry NPC (`CDR_PALACEGUARD`).
//!
//! Ports `src/area/11/palace.c::palace_guard` (`:84-353`) - two active
//! patrol demons (`palace_guard1`/`2`) that walk a fixed waypoint loop and
//! shout an area alert on sighting a valid target, a dozen-plus stationary
//! "reserve" demons that snap awake and rush toward that shouted alert
//! position, an unused-in-current-data `alertx`/`alerty` freeze-chokepoint
//! mechanic, and an unused-in-current-data sprite-guided "walk along a
//! line" mode (the `Ice Eye` template, `line=1`). `ch_died_driver`/
//! `ch_respawn_driver`'s own `CDR_PALACEGUARD` cases are both empty no-ops
//! (`palace.c:822-823,835-836`) - no death/respawn hook needed.
//!
//! Deviations/gaps (documented, not silent):
//! - Same single-victim self-defense simplification as every other ported
//!   area NPC (see `world::robber`'s module doc comment): C's generic
//!   10-slot `struct fight_driver_data` narrows to a single tracked
//!   `victim`. Unlike `robber`/`islena` (which only ever call
//!   `standard_message_driver` with `helper=0`, collapsing straight to
//!   `NT_GOTHIT`-only self-defense), most live `palace_guard` instances
//!   run with `scream` unset (`aggressive=1, helper=1`), so this port's
//!   [`World::palace_guard_standard_message`] additionally ports the
//!   `NT_CHAR` aggressive-sighting and `NT_SEEHIT` group-helper branches.
//! - `fight_driver_set_dist(cn, 0, 20, 0)` (`palace.c:157`, on
//!   `NT_CREATE`) is not ported, same precedent as every other
//!   single-victim NPC.
//! - `fight_driver_note_hit(cn)` (inside `standard_message_driver`'s
//!   `NT_GOTHIT` case): server-logfile-only bookkeeping, not ported (same
//!   precedent as `world::islena`).
//! - Coordinates stashed into `dat->dox`/`dat->doy`/`dat->alertx`/
//!   `dat->alerty` are C `unsigned char` fields (`palace.c:87,97`) - ported
//!   as `u8` with the same truncating-cast-from-map-coordinate behavior C
//!   has (map coordinates routinely exceed 255, so this is a real,
//!   preserved C quirk, not a bug introduced by this port).
//! - Confirmed against the live `ugaris_data/zones/11/palace.chr` template
//!   set: no guard instance sets `alertx`/`alerty` (the freeze-chokepoint
//!   branch is dead in current data) and no ground tile in
//!   `zones/11/palace.map` carries the `51050`/`51051`/`51052` overlay
//!   sprites the `line=1` "Ice Eye" branch keys off (also dead in current
//!   data) - both are still ported digit-for-digit below since they are
//!   reachable C behavior, just unexercised by the shipped map.
//! - C `ch[cn].item[30] && (ch[cn].flags & CF_NOBODY)` ->
//!   `CF_ITEMDEATH` transform (`palace.c:159-163`) at `NT_CREATE` is
//!   ported in [`apply_palace_guard_create_message`], the same transform
//!   `apply_simple_baddy_create_message` already carries for
//!   `CDR_SIMPLEBADDY`.

use crate::world::*;

const MAXPAT: usize = 20;
/// C `WN_HEAD` worn-slot index (0-based, matching `crate::zone`'s
/// `NAMES` ordering: `WN_NECK, WN_HEAD, ...`).
const WN_HEAD: usize = 1;
/// C `TICKS * 20` (`palace.c:195`): scream-alert re-trigger cooldown.
const SCREAM_COOLDOWN_TICKS: u64 = 20;
/// C `TICKS * 30` (`palace.c:255`): scout give-up timeout.
const SCOUT_TIMEOUT_TICKS: u64 = 30;
/// C `TICKS * 5` (`palace.c:263`): scout arrived-but-nothing-there give-up
/// grace period.
const SCOUT_ARRIVED_GRACE_TICKS: u64 = 5;

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_PALACEGUARD`
    /// characters (C `ch_driver`'s `CDR_PALACEGUARD` case, `palace.c:773-
    /// 775`).
    pub fn process_palace_guard_actions(&mut self, area_id: u16) -> usize {
        let guard_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_PALACEGUARD
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for guard_id in guard_ids {
            if self.process_palace_guard_tick(guard_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `palace_guard`'s per-tick body (`palace.c:138-353`).
    fn process_palace_guard_tick(&mut self, guard_id: CharacterId, area_id: u16) -> bool {
        let mut data = match self
            .characters
            .get(&guard_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::PalaceGuard(data)) => data,
            _ => PalaceGuardDriverData::default(),
        };

        let messages = self
            .characters
            .get_mut(&guard_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            let mut skip_standard = false;
            match message.message_type {
                NT_CHAR if message.dat1 > 0 => {
                    let target_id = CharacterId(message.dat1 as u32);
                    if self.palace_guard_check_cap_immunity(target_id, &mut data) {
                        skip_standard = true;
                    } else if self.palace_guard_handle_alert_scream(guard_id, target_id, &mut data)
                    {
                        // C `if (do_freeze(cn)) { return; }` (`palace.c:187-
                        // 189`): queued a freeze action, stop this tick
                        // entirely without processing further messages.
                        self.palace_guard_save(guard_id, data);
                        return true;
                    }
                }
                NT_NPC if data.reserve != 0 && message.dat1 == NTID_PALACE_ALERT => {
                    self.palace_guard_handle_scout_alert(guard_id, message, &mut data);
                }
                _ => {}
            }
            if !skip_standard {
                self.palace_guard_standard_message(guard_id, message, &mut data);
            }
        }

        // C `if (dat->doalert) { ... }` (`palace.c:227-241`).
        if data.doalert {
            if let Some(guard) = self.characters.get(&guard_id).cloned() {
                if guard.x.abs_diff(u16::from(data.alertx)) < 2
                    && guard.y.abs_diff(u16::from(data.alerty)) < 2
                {
                    data.doalert = false;
                    data.docheck = false;
                    self.notify_area_shout(
                        guard.x,
                        guard.y,
                        NT_NPC,
                        NTID_PALACE_ALERT,
                        i32::from(data.dox),
                        i32::from(data.doy),
                    );
                    self.npc_say(guard_id, "Granishni kwalar!");
                } else if self.palace_guard_move(
                    guard_id,
                    u16::from(data.alertx),
                    u16::from(data.alerty),
                    area_id,
                ) {
                    self.palace_guard_save(guard_id, data);
                    return true;
                }
            }
        }

        // C `fight_driver_update(cn)` (`palace.c:243`).
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

        // C `if (fight_driver_attack_visible(cn, 0)) { dat->lastfight =
        // ticker; return; }` (`palace.c:245-248`).
        if data.victim_visible {
            if let Some(victim_id) = data.victim {
                if self.attack_driver_direct(guard_id, victim_id, area_id) {
                    data.lastfight = self.tick.0;
                    self.palace_guard_save(guard_id, data);
                    return true;
                }
            }
        } else if data.victim.is_some() {
            // C `if (fight_driver_follow_invisible(cn)) { dat->lastfight =
            // ticker; return; }` (`palace.c:249-252`).
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
                data.lastfight = self.tick.0;
                self.palace_guard_save(guard_id, data);
                return true;
            }
        }

        // C `if (dat->doscout) { ... }` (`palace.c:254-284`).
        if data.doscout != 0 {
            if self.tick.0.saturating_sub(data.doscout) > SCOUT_TIMEOUT_TICKS * TICKS_PER_SECOND {
                data.doscout = 0;
                if data.patrolx[0] != 0 {
                    data.docheck = true;
                    data.pat = 0;
                    self.npc_say(guard_id, "Olk'ka?");
                }
                // C: no `return;` here - falls through to regenerate/spell/
                // line/patrol below.
            } else {
                let arrived = self.characters.get(&guard_id).is_some_and(|guard| {
                    guard.x.abs_diff(u16::from(data.dox)) < 2
                        && guard.y.abs_diff(u16::from(data.doy)) < 2
                });
                if arrived {
                    if self.tick.0.saturating_sub(data.lastfight)
                        > SCOUT_ARRIVED_GRACE_TICKS * TICKS_PER_SECOND
                    {
                        data.doscout = 0;
                        if data.patrolx[0] != 0 {
                            data.docheck = true;
                            data.pat = 0;
                        }
                        self.npc_say(guard_id, "Nashterk'ka?");
                    }
                    self.palace_guard_save(guard_id, data);
                    return self.palace_guard_idle_half_tick(guard_id);
                }
                let moved = self.palace_guard_move(
                    guard_id,
                    u16::from(data.dox),
                    u16::from(data.doy),
                    area_id,
                );
                self.palace_guard_save(guard_id, data);
                if moved {
                    return true;
                }
                return self.palace_guard_idle_half_tick(guard_id);
            }
        }

        // C `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return;` (`palace.c:287-292`).
        if self.regenerate_simple_baddy(guard_id) {
            self.palace_guard_save(guard_id, data);
            return true;
        }
        if self.spell_self_simple_baddy(guard_id) {
            self.palace_guard_save(guard_id, data);
            return true;
        }

        // C `if (dat->line) { ... }` (`palace.c:294-322`).
        if data.line != 0 && self.palace_guard_walk_line(guard_id, area_id, &mut data) {
            self.palace_guard_save(guard_id, data);
            return true;
        }

        // C `if (dat->patrol || dat->docheck) {...} else {...}`
        // (`palace.c:324-349`).
        if data.patrol != 0 || data.docheck {
            if self.palace_guard_walk_patrol(guard_id, area_id, &mut data) {
                self.palace_guard_save(guard_id, data);
                return true;
            }
        } else if let Some((gx, gy, tmpx, tmpy)) = self
            .characters
            .get(&guard_id)
            .map(|guard| (guard.x, guard.y, guard.rest_x, guard.rest_y))
        {
            if (gx != tmpx || gy != tmpy)
                && (self.setup_walk_toward(
                    guard_id,
                    usize::from(tmpx),
                    usize::from(tmpy),
                    0,
                    area_id,
                    false,
                ) || self.setup_walk_toward(
                    guard_id,
                    usize::from(tmpx),
                    usize::from(tmpy),
                    0,
                    area_id,
                    true,
                ))
            {
                self.palace_guard_save(guard_id, data);
                return true;
            }
        }

        // C `do_idle(cn, TICKS);` (`palace.c:352`).
        self.palace_guard_save(guard_id, data);
        self.idle_simple_baddy(guard_id)
    }

    /// C `palace_guard`'s palace-cap-immunity `NT_CHAR` short-circuit
    /// (`palace.c:169-174`): a target wearing an active `IDR_PALACECAP`
    /// cannot be pursued.
    fn palace_guard_check_cap_immunity(
        &self,
        target_id: CharacterId,
        data: &mut PalaceGuardDriverData,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        let Some(head_item_id) = target.inventory.get(WN_HEAD).copied().flatten() else {
            return false;
        };
        let Some(item) = self.items.get(&head_item_id) else {
            return false;
        };
        if item.driver == IDR_PALACECAP && item.driver_data.first().copied().unwrap_or(0) != 0 {
            if data.victim == Some(target_id) {
                data.victim = None;
            }
            true
        } else {
            false
        }
    }

    /// C's `alertx`/`dofreeze`/`scream` `NT_CHAR` blocks (`palace.c:175-
    /// 200`). Returns `true` if `do_freeze` queued an action (matching
    /// C's `return;`).
    fn palace_guard_handle_alert_scream(
        &mut self,
        guard_id: CharacterId,
        target_id: CharacterId,
        data: &mut PalaceGuardDriverData,
    ) -> bool {
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let daylight = self.date.daylight;

        if data.alertx != 0
            && !data.doalert
            && char_see_char(&guard, &target, &self.map, daylight)
            && can_attack(&guard, &target, &self.map)
        {
            if !data.dofreeze {
                self.npc_say(guard_id, "Granishni?");
            }
            data.dofreeze = true;
            data.dox = target.x as u8;
            data.doy = target.y as u8;
        }

        if data.dofreeze
            && tile_char_dist(&guard, &target) < 4
            && char_see_char(&guard, &target, &self.map, daylight)
            && can_attack(&guard, &target, &self.map)
        {
            if !data.doalert {
                data.doalert = true;
                self.npc_say(guard_id, "Granishni!");
                let weather_movement_percent = self.settings.weather_movement_percent;
                let froze = self.characters.get_mut(&guard_id).is_some_and(|guard_mut| {
                    do_freeze(guard_mut, &self.map, weather_movement_percent).is_ok()
                });
                if froze {
                    return true;
                }
            }
            data.dofreeze = false;
            data.dox = target.x as u8;
            data.doy = target.y as u8;
        }

        // C `if (dat->scream && ...) { ... }` (`palace.c:195-200`).
        if data.scream != 0
            && self.tick.0.saturating_sub(data.lastfight) > SCREAM_COOLDOWN_TICKS * TICKS_PER_SECOND
            && char_dist(&guard, &target) < 16
            && char_see_char(&guard, &target, &self.map, daylight)
            && can_attack(&guard, &target, &self.map)
        {
            self.npc_say(guard_id, "Granishni kwalar!");
            self.notify_area_shout(
                guard.x,
                guard.y,
                NT_NPC,
                NTID_PALACE_ALERT,
                i32::from(target.x),
                i32::from(target.y),
            );
            data.lastfight = self.tick.0;
        }

        false
    }

    /// C's reserve-scout `NT_NPC`/`NTID_PALACE_ALERT` handler
    /// (`palace.c:205-212`).
    fn palace_guard_handle_scout_alert(
        &mut self,
        guard_id: CharacterId,
        message: &CharacterDriverMessage,
        data: &mut PalaceGuardDriverData,
    ) {
        if data.doscout == 0 {
            self.npc_say(guard_id, "Schak'ko");
        }
        data.dox = message.dat2 as u8;
        data.doy = message.dat3 as u8;
        data.doscout = self.tick.0;
    }

    /// C `standard_message_driver` (`src/system/drvlib.c:2466-2540`),
    /// called with `(agressive, helper) = (0, 0)` when `dat->scream` is
    /// set, else `(1, 1)` (`palace.c:215-219`). Narrowed to the
    /// single-victim model - see module doc comment.
    fn palace_guard_standard_message(
        &mut self,
        guard_id: CharacterId,
        message: &CharacterDriverMessage,
        data: &mut PalaceGuardDriverData,
    ) {
        let aggressive = data.scream == 0;
        let helper = data.scream == 0;

        match message.message_type {
            NT_CHAR if message.dat1 > 0 => {
                if !aggressive {
                    return;
                }
                let target_id = CharacterId(message.dat1 as u32);
                if self.palace_guard_is_valid_enemy(guard_id, target_id) {
                    data.victim = Some(target_id);
                }
            }
            NT_SEEHIT if helper && message.dat1 > 0 && message.dat2 > 0 => {
                let attacker_id = CharacterId(message.dat1 as u32);
                let victim_id = CharacterId(message.dat2 as u32);
                let Some(guard) = self.characters.get(&guard_id).cloned() else {
                    return;
                };
                let victim_is_friend = victim_id != guard_id
                    && self
                        .characters
                        .get(&victim_id)
                        .is_some_and(|victim| victim.group == guard.group);
                if victim_is_friend {
                    if self.palace_guard_is_valid_enemy(guard_id, attacker_id) {
                        data.victim = Some(attacker_id);
                    }
                    return;
                }
                let attacker_is_friend = attacker_id != guard_id
                    && self
                        .characters
                        .get(&attacker_id)
                        .is_some_and(|attacker| attacker.group == guard.group);
                if attacker_is_friend && self.palace_guard_is_valid_enemy(guard_id, victim_id) {
                    data.victim = Some(victim_id);
                }
            }
            NT_GOTHIT if message.dat1 > 0 => {
                // C `fight_driver_note_hit(cn)` not ported - see module
                // doc comment.
                let target_id = CharacterId(message.dat1 as u32);
                let Some((guard, target)) = self
                    .characters
                    .get(&guard_id)
                    .cloned()
                    .zip(self.characters.get(&target_id).cloned())
                else {
                    return;
                };
                if guard.group == target.group {
                    return;
                }
                if !can_attack(&guard, &target, &self.map) {
                    return;
                }
                data.victim = Some(target_id);
            }
            _ => {}
        }
    }

    /// C `is_valid_enemy(cn, co, -1)` (`src/system/drvlib.c:897-927`).
    fn palace_guard_is_valid_enemy(&self, guard_id: CharacterId, target_id: CharacterId) -> bool {
        if guard_id == target_id {
            return false;
        }
        let Some(guard) = self.characters.get(&guard_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        guard.group != target.group
            && can_attack(guard, target, &self.map)
            && char_see_char(guard, target, &self.map, self.date.daylight)
    }

    /// C `move_driver(cn, x, y, 1) || tmove_driver(cn, x, y, 1)`: the
    /// fixed mindist-1 pathfind shared by the doalert-walk, doscout-walk,
    /// and patrol-walk call sites.
    fn palace_guard_move(
        &mut self,
        guard_id: CharacterId,
        target_x: u16,
        target_y: u16,
        area_id: u16,
    ) -> bool {
        self.setup_walk_toward(
            guard_id,
            usize::from(target_x),
            usize::from(target_y),
            1,
            area_id,
            false,
        ) || self.setup_walk_toward(
            guard_id,
            usize::from(target_x),
            usize::from(target_y),
            1,
            area_id,
            true,
        )
    }

    /// C `if (dat->line) { ... }` (`palace.c:294-322`): a fixed
    /// sprite-guided walk along `51050`/`51051`/`51052` ground-overlay
    /// tiles once the initial direction is auto-detected from a
    /// neighboring tile - the `Ice Eye` template's own gimmick. Dead in
    /// the currently shipped `zones/11/palace.map` (see module doc
    /// comment) but ported digit-for-digit, including C's own switch
    /// fallthrough (a failed `do_walk` on a line tile falls through to
    /// the same `move_driver` default case a non-line tile takes).
    fn palace_guard_walk_line(
        &mut self,
        guard_id: CharacterId,
        area_id: u16,
        data: &mut PalaceGuardDriverData,
    ) -> bool {
        let Some(guard) = self.characters.get(&guard_id).cloned() else {
            return false;
        };
        let gx = i32::from(guard.x);
        let gy = i32::from(guard.y);
        let here = self.palace_guard_ground_sprite(gx, gy);
        if here == 51050 && self.palace_guard_ground_sprite(gx + 1, gy) == 51052 {
            data.line = Direction::Right as u8;
        }
        if here == 51050 && self.palace_guard_ground_sprite(gx - 1, gy) == 51052 {
            data.line = Direction::Left as u8;
        }
        if here == 51050 && self.palace_guard_ground_sprite(gx, gy + 1) == 51051 {
            data.line = Direction::Down as u8;
        }
        if here == 51050 && self.palace_guard_ground_sprite(gx, gy - 1) == 51051 {
            data.line = Direction::Up as u8;
        }

        if matches!(here, 51050 | 51051 | 51052) {
            if let Ok(direction) = Direction::try_from(data.line) {
                let weather_movement_percent = self.settings.weather_movement_percent;
                let earthmud_extra_cost = self.earthmud_extra_movement_cost(guard_id);
                let walked = self.characters.get_mut(&guard_id).is_some_and(|character| {
                    do_walk(
                        character,
                        &mut self.map,
                        direction as u8,
                        area_id,
                        weather_movement_percent,
                        earthmud_extra_cost,
                    )
                    .is_ok()
                });
                if walked {
                    return true;
                }
            }
        }

        self.setup_walk_toward(
            guard_id,
            usize::from(guard.rest_x),
            usize::from(guard.rest_y),
            0,
            area_id,
            false,
        )
    }

    /// C `map[m].gsprite >> 16` (`palace.c:297` etc.).
    fn palace_guard_ground_sprite(&self, x: i32, y: i32) -> u32 {
        if x < 0 || y < 0 {
            return 0;
        }
        self.map
            .tile(x as usize, y as usize)
            .map(|tile| tile.ground_sprite >> 16)
            .unwrap_or(0)
    }

    /// C `if (dat->patrol || dat->docheck) { ... }` (`palace.c:324-339`).
    fn palace_guard_walk_patrol(
        &mut self,
        guard_id: CharacterId,
        area_id: u16,
        data: &mut PalaceGuardDriverData,
    ) -> bool {
        let Some(guard) = self.characters.get(&guard_id) else {
            return false;
        };
        let (gx, gy) = (guard.x, guard.y);
        let pat = usize::from(data.pat);
        let (target_x, target_y) = (
            data.patrolx.get(pat).copied().unwrap_or(0),
            data.patroly.get(pat).copied().unwrap_or(0),
        );
        if gx.abs_diff(u16::from(target_x)) < 2 && gy.abs_diff(u16::from(target_y)) < 2 {
            data.pat = data.pat.saturating_add(1);
            let next = usize::from(data.pat);
            if usize::from(data.pat) >= MAXPAT || data.patrolx.get(next).copied().unwrap_or(0) == 0
            {
                data.pat = 0;
                data.docheck = false;
            }
            if data.docheck {
                self.npc_say(guard_id, "Nashterk'ka?");
            }
        }
        let pat = usize::from(data.pat);
        let (target_x, target_y) = (
            data.patrolx.get(pat).copied().unwrap_or(0),
            data.patroly.get(pat).copied().unwrap_or(0),
        );
        self.palace_guard_move(guard_id, u16::from(target_x), u16::from(target_y), area_id)
    }

    /// C `do_idle(cn, TICKS / 2); return;` (`palace.c:272,282`).
    fn palace_guard_idle_half_tick(&mut self, guard_id: CharacterId) -> bool {
        self.characters
            .get_mut(&guard_id)
            .is_some_and(|character| do_idle(character, (TICKS_PER_SECOND / 2) as i32).is_ok())
    }

    fn palace_guard_save(&mut self, guard_id: CharacterId, data: PalaceGuardDriverData) {
        if let Some(character) = self.characters.get_mut(&guard_id) {
            character.driver_state = Some(CharacterDriverState::PalaceGuard(data));
        }
    }
}

/// C `palace_guard_parse` (`palace.c:103-136`).
pub fn parse_palace_guard_driver_args(args: &str) -> PalaceGuardDriverData {
    let mut data = PalaceGuardDriverData::default();
    // C `int pat = 0;`: a local index shared across repeated
    // `patrolx=N;patroly=N;` pairs - `patrolx` writes without advancing,
    // `patroly` writes and then advances (`palace.c:108-119`).
    let mut pat: usize = 0;
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "patrolx" => {
                if pat < MAXPAT {
                    data.patrolx[pat] = parsed as u8;
                }
                // C: `elog(...)` only past `MAXPAT` - not ported (log-only).
            }
            "patroly" => {
                if pat < MAXPAT {
                    data.patroly[pat] = parsed as u8;
                    pat += 1;
                }
            }
            "patrol" => data.patrol = parsed as u8,
            "alertx" => data.alertx = parsed as u8,
            "alerty" => data.alerty = parsed as u8,
            "reserve" => data.reserve = parsed as u8,
            "scream" => data.scream = parsed as u8,
            "line" => data.line = parsed as u8,
            _ => {} // C: `elog("unknown arg for %s (%d): %s", ...)` - log-only.
        }
        rest = next;
    }
    data
}

/// C `palace_guard`'s `NT_CREATE` handler (`palace.c:152-163`): the
/// one-shot `ch[cn].arg` parse plus the `item[30]`/`CF_NOBODY` ->
/// `CF_ITEMDEATH` transform, same shape as
/// `apply_simple_baddy_create_message`.
pub fn apply_palace_guard_create_message(character: &mut Character, args: Option<&str>) {
    let data = match args.filter(|args| !args.is_empty()) {
        Some(args) => parse_palace_guard_driver_args(args),
        None => PalaceGuardDriverData::default(),
    };
    character.driver_state = Some(CharacterDriverState::PalaceGuard(data));
    character
        .driver_messages
        .retain(|message| message.message_type != NT_CREATE);

    if character.inventory.get(30).and_then(|slot| *slot).is_some()
        && character.flags.contains(CharacterFlags::NOBODY)
    {
        character.flags.remove(CharacterFlags::NOBODY);
        character.flags.insert(CharacterFlags::ITEMDEATH);
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{next_legacy_name_value, CDR_PALACEGUARD, NTID_PALACE_ALERT};
use crate::item_driver::IDR_PALACECAP;

/// C `struct palace_guard_data` (`src/area/11/palace.c:85-101`), plus this
/// port's own single-victim self-defense tracking (see module doc
/// comment).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PalaceGuardDriverData {
    pub patrolx: [u8; MAXPAT],
    pub patroly: [u8; MAXPAT],
    pub alertx: u8,
    pub alerty: u8,
    pub reserve: u8,
    pub patrol: u8,
    pub pat: u8,
    pub scream: u8,
    pub line: u8,
    pub doalert: bool,
    pub docheck: bool,
    pub dofreeze: bool,
    pub dox: u8,
    pub doy: u8,
    pub doscout: u64,
    pub lastfight: u64,
    pub victim: Option<CharacterId>,
    pub victim_visible: bool,
    pub victim_last_x: u16,
    pub victim_last_y: u16,
}

impl Default for PalaceGuardDriverData {
    fn default() -> Self {
        Self {
            patrolx: [0; MAXPAT],
            patroly: [0; MAXPAT],
            alertx: 0,
            alerty: 0,
            reserve: 0,
            patrol: 0,
            pat: 0,
            scream: 0,
            line: 0,
            doalert: false,
            docheck: false,
            dofreeze: false,
            dox: 0,
            doy: 0,
            doscout: 0,
            lastfight: 0,
            victim: None,
            victim_visible: false,
            victim_last_x: 0,
            victim_last_y: 0,
        }
    }
}
