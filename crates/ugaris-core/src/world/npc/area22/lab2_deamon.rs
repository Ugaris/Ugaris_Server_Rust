//! Lab 2 family-vault guardian (`CDR_LAB2DEAMON`, "Deamon"), the
//! masquerade-detection seek-and-destroy demon Elias's family vault door
//! spawns to test whoever steps on its warning tile.
//!
//! Ports `src/area/22/lab2.c::lab2_deamon_driver` (`:454-771`) and its
//! creation helpers `lab2_deamon_create` (`:368-426`, split between
//! [`World::lab2_deamon_already_tracking`]/[`World::init_lab2_deamon`] -
//! see their own doc comments for why the actual spawn placement lives in
//! `ugaris-server`) and `lab2_deamon_is_elias` (`:428-452`, ported as
//! [`World::lab2_deamon_is_elias`]).
//!
//! Deviations/gaps (documented, not silent):
//! - `fight_driver_set_dist(cn, 2*MAXMAP, 2*MAXMAP, 2*MAXMAP)` (`:471`) is
//!   not reproduced: like `world::gate_fight`'s own module doc comment
//!   explains, this port tracks the single `co` this NPC ever fights
//!   directly (set once from creation, matching `dat->co`) instead of via
//!   C's generic 10-slot `struct fight_driver_data` enemy list, so the
//!   distance limits that call would have configured on that list are
//!   meaningless here.
//! - `fight_driver_add_enemy(cn, co, 1, 1)`/`fight_driver_remove_enemy`
//!   are similarly replaced by direct `attacking`/`pursuing` flag writes
//!   on [`Lab2DeamonDriverData`] - `pursuing` specifically models "is
//!   `co` still present in the (otherwise unmodeled) generic enemy
//!   list", since C's `fight_driver_follow_invisible` giving up on
//!   arrival/unreachability (`drvlib.c:2309-2322`) clears just that
//!   internal enemy slot, not `dat->attacking`/`dat->co` themselves - a
//!   real, intentional C behavior (the daemon stops chasing but still
//!   privately considers itself "attacking" until the "stop attacking
//!   when player tries to run away" `MF_NOMAGIC` check below fires).
//! - `standard_message_driver(cn, msg, 0, 0)` is not reproduced, same
//!   precedent/reasoning as every other file-local NPC's own doc comment
//!   in this module directory (dead code for `agressive=0, helper=0`).
//! - The `MF_NOMAGIC`-gated "stop attacking when player tries to run
//!   away" condition (`:709-716`) is ported exactly as written
//!   (`!(flags & MF_NOMAGIC)` releases the enemy) even though the name
//!   suggests the opposite; `Character::flags`'
//!   [`crate::entity::CharacterFlags::NOMAGIC`] is already kept in sync
//!   with the underlying map tile on every move (`crate::map::MapGrid::
//!   set_char`), so this reads it directly instead of re-deriving it from
//!   `self.map`.

use crate::direction::Direction;
use crate::drvlib::offset2dx;
use crate::item_driver::{
    IID_LAB2_ELIASBELT, IID_LAB2_ELIASBOOTS, IID_LAB2_ELIASCAPE, IID_LAB2_ELIASHAT,
};
use crate::path::pathfinder;
use crate::world::*;

/// C `WN_HEAD` worn-slot index (0-based, matching `crate::zone`'s
/// `NAMES` ordering: `WN_NECK, WN_HEAD, WN_CLOAK, ...`).
const WN_HEAD: usize = 1;
const WN_CLOAK: usize = 2;
const WN_BELT: usize = 5;
const WN_FEET: usize = 10;

/// C `TICKS/8` (`lab2.c:537,589,655,674`): the talk-state-machine's
/// zero-delay "entry" tick before the first line fires.
const LAB2_DEAMON_TALK_ENTRY_TICKS: u64 = TICKS_PER_SECOND / 8;

/// C `lab2_deamon_is_elias`'s three-way return (`lab2.c:428-452`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EliasStatus {
    /// C `0`: none of the four pieces worn.
    None,
    /// C `-1`: 1-3 of the four pieces worn.
    Partial,
    /// C `1`: all four pieces worn.
    Full,
}

/// Per-player facts [`World::process_lab2_deamon_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lab2DeamonPlayerFacts {
    /// `PlayerRuntime::lab2_deamon_checked` (C `lab2_player_data.
    /// deamonchecked`).
    pub deamon_checked: bool,
}

/// A side effect [`World::process_lab2_deamon_actions`] could not apply
/// directly because it touches `PlayerRuntime`. See the module doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lab2DeamonOutcomeEvent {
    /// C `player_dat->deamonchecked = 1;` (`lab2.c:492`).
    MarkDeamonChecked { player_id: CharacterId },
    /// C `if (ch[dat->co].player) player_driver_halt(ch[dat->co].player);`
    /// (`lab2.c:549,572,583,686`).
    HaltPlayer { player_id: CharacterId },
}

impl World {
    /// C `lab2_deamon_create`'s dedup loop (`lab2.c:376-388`): is there
    /// already a live `CDR_LAB2DEAMON` tracking this exact `(co, serial)`
    /// pair? Called by the `ugaris-server` caller before spawning a new
    /// one.
    pub fn lab2_deamon_already_tracking(&self, co: CharacterId, serial: u32) -> bool {
        self.characters.values().any(|character| {
            character.driver == CDR_LAB2DEAMON
                && character.flags.contains(CharacterFlags::USED)
                && matches!(
                    character.driver_state.as_ref(),
                    Some(CharacterDriverState::Lab2Deamon(data))
                        if data.co == Some(co) && data.serial == serial
                )
        })
    }

    /// C `lab2_deamon_create`'s post-`drop_char` tail
    /// (`ch[cn].dir = DX_DOWN`, `ch[cn].tmpx/tmpy = ch[cn].x/y`,
    /// `dat->co`/`dat->serial`, and the implicit `NT_CREATE` message
    /// `create_char` always queues) - `lab2.c:396-425`. Called by the
    /// `ugaris-server` caller (`tick_item_use_lab.rs`) right after
    /// `World::spawn_character` places the daemon, since only the caller
    /// knows the real spawn coordinates and the triggering player's
    /// `co`/serial. `tmpx`/`tmpy` reuse `rest_x`/`rest_y`, the same
    /// substitution every other stationary/homed NPC in this directory
    /// uses (see `lab2_herald.rs`'s own doc comment).
    pub fn init_lab2_deamon(&mut self, deamon_id: CharacterId, co: CharacterId, serial: u32) {
        let Some(deamon) = self.characters.get_mut(&deamon_id) else {
            return;
        };
        deamon.dir = Direction::Down as u8;
        deamon.rest_x = deamon.x;
        deamon.rest_y = deamon.y;
        deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(Lab2DeamonDriverData {
            co: Some(co),
            serial,
            ..Default::default()
        }));
        deamon.push_driver_message(NT_CREATE, 0, 0, 0);
    }

    /// C `lab2_deamon_is_elias` (`lab2.c:428-452`).
    fn lab2_deamon_is_elias(&self, co: Option<CharacterId>) -> EliasStatus {
        let Some(character) = co.and_then(|id| self.characters.get(&id)) else {
            return EliasStatus::None;
        };
        let mut part = 0;
        if self.lab2_deamon_worn_matches(character, WN_HEAD, IID_LAB2_ELIASHAT) {
            part += 1;
        }
        if self.lab2_deamon_worn_matches(character, WN_CLOAK, IID_LAB2_ELIASCAPE) {
            part += 1;
        }
        if self.lab2_deamon_worn_matches(character, WN_BELT, IID_LAB2_ELIASBELT) {
            part += 1;
        }
        if self.lab2_deamon_worn_matches(character, WN_FEET, IID_LAB2_ELIASBOOTS) {
            part += 1;
        }
        match part {
            4 => EliasStatus::Full,
            0 => EliasStatus::None,
            _ => EliasStatus::Partial,
        }
    }

    fn lab2_deamon_worn_matches(
        &self,
        character: &Character,
        slot: usize,
        template_id: u32,
    ) -> bool {
        character
            .inventory
            .get(slot)
            .copied()
            .flatten()
            .and_then(|item_id| self.items.get(&item_id))
            .is_some_and(|item| item.template_id == template_id)
    }

    /// C `lab2_deamon_driver`'s per-tick body (`lab2.c:454-771`).
    pub fn process_lab2_deamon_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, Lab2DeamonPlayerFacts>,
        area_id: u16,
    ) -> Vec<Lab2DeamonOutcomeEvent> {
        let deamon_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAB2DEAMON
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for deamon_id in deamon_ids {
            self.process_lab2_deamon_tick(deamon_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_lab2_deamon_tick(
        &mut self,
        deamon_id: CharacterId,
        player_facts: &HashMap<CharacterId, Lab2DeamonPlayerFacts>,
        area_id: u16,
        events: &mut Vec<Lab2DeamonOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Lab2Deamon(mut data)) = self
            .characters
            .get(&deamon_id)
            .and_then(|deamon| deamon.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&deamon_id)
            .map(|deamon| std::mem::take(&mut deamon.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_CREATE => self.lab2_deamon_handle_create(&mut data, player_facts, events),
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3.max(0) as u32);
                    if let Some(text) = message.text.as_deref() {
                        self.apply_tabunga_text_notification(deamon_id, speaker_id, text);
                    }
                }
                NT_GIVE => {
                    if let Some(item_id) = self
                        .characters
                        .get(&deamon_id)
                        .and_then(|deamon| deamon.cursor_item)
                    {
                        self.destroy_item(item_id);
                        if let Some(deamon) = self.characters.get_mut(&deamon_id) {
                            deamon.cursor_item = None;
                        }
                    }
                }
                NT_NPC if message.dat1 == NTID_LAB2_DEAMONCHECK => {
                    self.lab2_deamon_handle_deamoncheck(deamon_id, &mut data, message);
                }
                _ => {}
            }
        }

        self.lab2_deamon_talk(deamon_id, &mut data, events);

        // C: "stop attacking when player tries to run away" (`:708-716`).
        if data.attacking && data.pursuing {
            let victim_left_nomagic = data
                .co
                .and_then(|co| self.characters.get(&co))
                .is_some_and(|victim| !victim.flags.contains(CharacterFlags::NOMAGIC));
            if victim_left_nomagic {
                data.attacking = false;
                data.pursuing = false;
                self.npc_say(
                    deamon_id,
                    "Master Elias told me to let them run away, so my work is done.",
                );
            }
        }

        // C: "do we need a teleport?" (`:718-733`).
        if data.attacking {
            if let Some(co) = data.co {
                let valid = self.characters.get(&co).is_some_and(|victim| {
                    victim.flags.contains(CharacterFlags::USED) && victim.serial == data.serial
                });
                if valid {
                    if let Some(moved) = self.lab2_deamon_teleport_if_stuck(deamon_id, co) {
                        if moved {
                            if let Some(deamon) = self.characters.get_mut(&deamon_id) {
                                deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(data));
                            }
                            return;
                        }
                    }
                }
            }
        }

        // C: `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
        // 0)) return; if (fight_driver_follow_invisible(cn)) return;`
        // (`:735-742`), narrowed to the single tracked `co` (see module
        // doc comment).
        if data.pursuing {
            if let Some(co) = data.co {
                let seen = match self
                    .characters
                    .get(&deamon_id)
                    .cloned()
                    .zip(self.characters.get(&co).cloned())
                {
                    Some((deamon, victim)) => {
                        let visible =
                            char_see_char(&deamon, &victim, &self.map, self.date.daylight);
                        if visible {
                            data.victim_last_x = victim.x;
                            data.victim_last_y = victim.y;
                        }
                        visible
                    }
                    None => false,
                };
                data.victim_visible = seen;

                if let Some(deamon) = self.characters.get_mut(&deamon_id) {
                    deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(data));
                }

                if seen {
                    if self.attack_driver_direct(deamon_id, co, area_id) {
                        return;
                    }
                } else {
                    let arrived = self.characters.get(&deamon_id).is_some_and(|deamon| {
                        deamon.x.abs_diff(data.victim_last_x) < 2
                            && deamon.y.abs_diff(data.victim_last_y) < 2
                    });
                    if arrived {
                        data.pursuing = false;
                    } else if self.secure_move_driver(
                        deamon_id,
                        data.victim_last_x,
                        data.victim_last_y,
                        Direction::Down as u8,
                        0,
                        0,
                        area_id,
                    ) {
                        if let Some(deamon) = self.characters.get_mut(&deamon_id) {
                            deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(data));
                        }
                        return;
                    }
                }
                if let Some(deamon) = self.characters.get_mut(&deamon_id) {
                    deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(data));
                }
            }
        }

        // C: `if (regenerate_driver(cn)) return; if (spell_self_driver(cn))
        // return;` (`:745-750`).
        if self.regenerate_simple_baddy(deamon_id) {
            if let Some(deamon) = self.characters.get_mut(&deamon_id) {
                deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(data));
            }
            return;
        }
        if self.spell_self_simple_baddy(deamon_id) {
            if let Some(deamon) = self.characters.get_mut(&deamon_id) {
                deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(data));
            }
            return;
        }

        // C: "remove deamon" (`:752-758`).
        let co_status = data.co.and_then(|co| self.characters.get(&co).cloned());
        let should_remove = match (&data.co, &co_status) {
            (None, _) => true,
            (Some(_), None) => true,
            (Some(_), Some(victim)) => {
                let visible = self
                    .characters
                    .get(&deamon_id)
                    .cloned()
                    .is_some_and(|deamon| {
                        char_see_char(&deamon, victim, &self.map, self.date.daylight)
                    });
                (!data.attacking && !visible)
                    || victim.serial != data.serial
                    || !victim.flags.contains(CharacterFlags::USED)
            }
        };
        if should_remove {
            if let Some(deamon) = self.characters.get(&deamon_id) {
                self.create_mist_effect(i32::from(deamon.x), i32::from(deamon.y));
            }
            self.remove_character(deamon_id);
            return;
        }

        // C: "go home" (`:760-763`).
        let (home_x, home_y) = self
            .characters
            .get(&deamon_id)
            .map(|deamon| (deamon.rest_x, deamon.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            deamon_id,
            home_x,
            home_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            if let Some(deamon) = self.characters.get_mut(&deamon_id) {
                deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(data));
            }
            return;
        }

        // C: "turn to char" (`:765-768`).
        if !data.attacking {
            if let (Some(deamon), Some(victim)) =
                (self.characters.get(&deamon_id).cloned(), co_status.as_ref())
            {
                if let Some(direction) = offset2dx(
                    i32::from(deamon.x),
                    i32::from(deamon.y),
                    i32::from(victim.x),
                    i32::from(victim.y),
                ) {
                    if let Some(deamon) = self.characters.get_mut(&deamon_id) {
                        let _ = turn(deamon, direction as u8);
                    }
                }
            }
        }

        // C `do_idle(cn, TICKS/2)` (`:770`).
        if let Some(deamon) = self.characters.get_mut(&deamon_id) {
            let _ = do_idle(deamon, (TICKS_PER_SECOND / 2) as i32);
            deamon.driver_state = Some(CharacterDriverState::Lab2Deamon(data));
        }
    }

    /// C `lab2_deamon_driver`'s `NT_CREATE` branch (`:470-496`).
    fn lab2_deamon_handle_create(
        &mut self,
        data: &mut Lab2DeamonDriverData,
        player_facts: &HashMap<CharacterId, Lab2DeamonPlayerFacts>,
        events: &mut Vec<Lab2DeamonOutcomeEvent>,
    ) {
        match self.lab2_deamon_is_elias(data.co) {
            EliasStatus::Partial => data.talkstep = 50,
            EliasStatus::Full => {
                data.observing = true;
                data.talkstep = 10;
                let Some(co) = data.co else { return };
                let already_checked = player_facts
                    .get(&co)
                    .is_some_and(|facts| facts.deamon_checked);
                if already_checked {
                    data.talkstep = 20;
                } else {
                    events.push(Lab2DeamonOutcomeEvent::MarkDeamonChecked { player_id: co });
                }
            }
            EliasStatus::None => {}
        }
    }

    /// C `lab2_deamon_driver`'s `NT_NPC`/`NTID_LAB2_DEAMONCHECK` branch
    /// (`:514-528`).
    fn lab2_deamon_handle_deamoncheck(
        &mut self,
        deamon_id: CharacterId,
        data: &mut Lab2DeamonDriverData,
        message: &CharacterDriverMessage,
    ) {
        let co = CharacterId(message.dat2.max(0) as u32);
        let serial_matches = data.co == Some(co)
            && self
                .characters
                .get(&co)
                .is_some_and(|character| character.serial == data.serial);
        if !serial_matches || self.lab2_deamon_is_elias(Some(co)) == EliasStatus::Full {
            return;
        }
        data.attacking = true;
        data.pursuing = true;
        data.talkstep = 255;
        let name = self
            .characters
            .get(&co)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        if data.observing {
            self.npc_shout(
                deamon_id,
                &format!("Hey! Thou are not Elias. Now thou shalt die, {name}!"),
            );
        } else {
            self.npc_shout(
                deamon_id,
                &format!("I warned thee. Now thou shalt die, {name}!"),
            );
        }
    }

    /// C `lab2_deamon_driver`'s talking `switch (dat->talkstep)`
    /// (`:534-706`): four independent dialogue ladders (warn/elias/
    /// quick-elias/masquerade), each keyed by its own entry state.
    fn lab2_deamon_talk(
        &mut self,
        deamon_id: CharacterId,
        data: &mut Lab2DeamonDriverData,
        events: &mut Vec<Lab2DeamonOutcomeEvent>,
    ) {
        let tick = self.tick.0;
        let Some(co) = data.co else {
            return self.lab2_deamon_talk_no_target(data);
        };
        match data.talkstep {
            // -- warn (co has no Elias parts at all) --
            0 => {
                data.talkticker = tick + LAB2_DEAMON_TALK_ENTRY_TICKS;
                data.talkstep = 1;
            }
            1 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(deamon_id, "STOP!");
                data.talkstep = 2;
                data.talkticker = tick + TICKS_PER_SECOND * 5;
                events.push(Lab2DeamonOutcomeEvent::HaltPlayer { player_id: co });
            }
            2 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(
                    deamon_id,
                    "On behalf of Elias, mine Master, I shall not allow anyone but himself to \
                     enter this family vault. So try to reach that door and I will kill thee!",
                );
                data.talkstep = 3;
                data.talkticker = tick + TICKS_PER_SECOND * 8;
            }
            3 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(
                    deamon_id,
                    "Ohh. Excuse me, Master. I am ashamed not to have recognized thee \
                     immediately. So, Elias, if you might want to enter... WAIT!",
                );
                data.talkstep = 4;
                data.talkticker = tick + TICKS_PER_SECOND * 4;
                events.push(Lab2DeamonOutcomeEvent::HaltPlayer { player_id: co });
            }
            4 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(
                    deamon_id,
                    "My eyes tricked me again. Thou art not Elias. I remember excatly his \
                     black hat and cape, and also his belt and boots. So, again, go away or I \
                     will kill thee!",
                );
                events.push(Lab2DeamonOutcomeEvent::HaltPlayer { player_id: co });
                data.talkstep = 255;
            }
            // -- elias (co has all four parts, first time) --
            10 => {
                data.talkticker = tick + LAB2_DEAMON_TALK_ENTRY_TICKS;
                data.talkstep = 11;
            }
            11 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(deamon_id, "Ahhh, Master Elias.");
                data.talkstep = 12;
                data.talkticker = tick + TICKS_PER_SECOND * 2;
            }
            12 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(
                    deamon_id,
                    "Excuse me for coming out of my Dimension. I hadn't recognized thee.",
                );
                data.talkstep = 13;
                data.talkticker = tick + TICKS_PER_SECOND * 6;
            }
            13 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(
                    deamon_id,
                    "But 'tis good to see thee around again. Last time we met thou wert in a \
                     bad condition. Very bad, I must say, very bad indeed.",
                );
                data.talkstep = 14;
                data.talkticker = tick + TICKS_PER_SECOND * 7;
            }
            14 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(
                    deamon_id,
                    "It must have been a couple of years since we last met. Maybe some \
                     couples more. Hahaha.",
                );
                data.talkstep = 15;
                data.talkticker = tick + TICKS_PER_SECOND * 7;
            }
            15 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(
                    deamon_id,
                    "But here I am, talking and talking. Thou sure hast more important things \
                     to do than listening to an old demon like me.",
                );
                data.talkstep = 16;
                data.talkticker = tick + TICKS_PER_SECOND * 9;
            }
            16 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(deamon_id, "So, farewell, Elias.");
                data.talkstep = 17;
                data.talkticker = tick + TICKS_PER_SECOND * 3;
            }
            17 => {
                data.co = None;
                data.talkstep = 255;
            }
            // -- quick elias (co has all four parts, already checked) --
            20 => {
                data.talkticker = tick + LAB2_DEAMON_TALK_ENTRY_TICKS;
                data.talkstep = 21;
            }
            21 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(deamon_id, "Ahh, it's you again, Master Elias. See You.");
                data.talkstep = 22;
                data.talkticker = tick + TICKS_PER_SECOND * 3;
            }
            22 => {
                data.co = None;
                data.talkstep = 255;
            }
            // -- masquerade (co has 1-3 parts) --
            50 => {
                data.talkticker = tick + LAB2_DEAMON_TALK_ENTRY_TICKS;
                data.talkstep = 51;
            }
            51 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(deamon_id, "STOP!");
                data.talkstep = 52;
                data.talkticker = tick + TICKS_PER_SECOND * 5;
                events.push(Lab2DeamonOutcomeEvent::HaltPlayer { player_id: co });
            }
            52 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(
                    deamon_id,
                    "What kind of masquerade is that. Thou are wearing some parts of Elias \
                     stuff, but thou are not Elias.",
                );
                data.talkstep = 53;
                data.talkticker = tick + TICKS_PER_SECOND * 4;
            }
            53 => {
                if tick < data.talkticker {
                    return;
                }
                self.npc_say(
                    deamon_id,
                    "Do not try to get into the family vault, I won't let you in, and thou \
                     will have to die!",
                );
                data.talkstep = 255;
            }
            _ => {}
        }
    }

    /// `dat->co` was cleared by an earlier `talkstep` transition (`17`/
    /// `22`) this same call chain can reach on a re-entrant call; C reads
    /// `ch[dat->co==0]` (the null character) harmlessly in this state,
    /// but every talk case here already only fires while `talkstep` is
    /// `10..=22`/`50..=53`/`0..=4`, none of which are re-entered once
    /// `dat->co` is cleared (`talkstep` is `255` by then) - so this is
    /// unreachable in practice; kept only so the `let Some(co) = data.co`
    /// guard above has a fallback instead of silently dropping a talk
    /// tick.
    fn lab2_deamon_talk_no_target(&mut self, _data: &mut Lab2DeamonDriverData) {}

    /// C: "do we need a teleport?" (`lab2.c:718-733`), given the tracked
    /// enemy is confirmed still valid by the caller. `None` means the
    /// pathfinder checks weren't even attempted (not attacking); `Some`
    /// carries whether the daemon actually teleported.
    fn lab2_deamon_teleport_if_stuck(
        &mut self,
        deamon_id: CharacterId,
        co: CharacterId,
    ) -> Option<bool> {
        let deamon = self.characters.get(&deamon_id).cloned()?;
        let victim = self.characters.get(&co).cloned()?;
        let (fx, fy) = (usize::from(deamon.x), usize::from(deamon.y));
        let (tx, ty) = (usize::from(victim.x), usize::from(victim.y));
        let blocked = pathfinder(&self.map, fx, fy, tx, ty, 2, None)
            .direction
            .is_none()
            && pathfinder(&self.map, fx, fy, tx, ty, 1, None)
                .direction
                .is_none();
        if !blocked {
            return Some(false);
        }
        let moved = self.teleport_char_driver(deamon_id, victim.x, victim.y);
        if moved {
            self.create_mist_effect(i32::from(deamon.x), i32::from(deamon.y));
            if let Some(new_pos) = self.characters.get(&deamon_id) {
                self.create_mist_effect(i32::from(new_pos.x), i32::from(new_pos.y));
            }
        }
        Some(moved)
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct lab2_deamon_driver_data` (`lab2.c:352-359`): the family-vault
/// guardian's own driver memory. `victim_visible`/`victim_last_x`/
/// `victim_last_y`/`pursuing` are this port's narrowed stand-in for C's
/// generic `struct fight_driver_data`/`DRD_FIGHTDRIVER` slot (see the
/// module doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab2DeamonDriverData {
    /// C `dat->co`: the player this daemon was created to guard against.
    #[serde(default)]
    pub co: Option<CharacterId>,
    /// C `dat->serial`: `co`'s serial at creation time, so a departed and
    /// replaced character in the same slot is never mistaken for `co`.
    #[serde(default)]
    pub serial: u32,
    #[serde(default)]
    pub talkstep: u8,
    #[serde(default)]
    pub talkticker: u64,
    /// C `dat->attacking`: seek-and-destroy mode is engaged.
    #[serde(default)]
    pub attacking: bool,
    /// C `dat->observing`: this daemon was created while `co` was already
    /// wearing full Elias gear (changes the shout text on later betrayal).
    #[serde(default)]
    pub observing: bool,
    /// See the module doc comment: whether `co` is still tracked in the
    /// (unmodeled) generic fight-driver enemy list.
    #[serde(default)]
    pub pursuing: bool,
    #[serde(default)]
    pub victim_visible: bool,
    #[serde(default)]
    pub victim_last_x: u16,
    #[serde(default)]
    pub victim_last_y: u16,
}
