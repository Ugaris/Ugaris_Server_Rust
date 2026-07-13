//! Krenach NPC (`CDR_KRENACH`), the dwarf grandfather who closes out
//! quest 78 ("The Mysterious Language") once the player has delivered his
//! grandson's regards, and refunds part of the Monk Dictionary's cost.
//!
//! Ports `src/area/37/arkhata.c::krenach_driver` (`:4197-4327`). Unlike
//! every other `arkhata.c` NPC driver, this one has no `analyse_text_
//! driver`/QA table hookup at all - C never wires `NT_TEXT` for it, so
//! there is no [`super::ARKHATA_QA`] use here. Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! ramin`/`thaipan`: the caller supplies a per-player fact snapshot
//! ([`KrenachPlayerFacts`]) up front and applies the returned
//! [`KrenachOutcomeEvent`]s afterwards, since `arkhata_ppd.krenach_state`/
//! `krenach_time` live on `crate::player::PlayerRuntime`, not `World`.
//!
//! `krenach_driver`'s five-state (`0`-`4`) dialogue chain, gated at one
//! point on cross-driver state this file cannot see directly (read via
//! [`KrenachPlayerFacts`]):
//! - `0` needs `arkhata_ppd.monk_state >= 29` (`world::npc::area37::
//!   arkhatamonk`'s own progress) to advance; C's own `case 0` falls
//!   through into `case 1`'s speech/`questlog_done(78)`/advance-to-`2` in
//!   the same tick - collapsed into one `rs == 0` arm here, same
//!   "fallthrough lands on the next case's action" precedent as
//!   `world::npc::area37::ramin`'s own `rs == 0`/`9`/`11` arms. While the
//!   gate is closed, C prints a throttled (`realtime - krenach_time >
//!   300`) grumble line instead - note this branch does *not* set
//!   `didsay` (`arkhata.c:3261-3264`), so it never touches `last_talk`/
//!   `current_victim`, only `krenach_time` (via
//!   [`KrenachOutcomeEvent::UpdateKrenachTime`]).
//!
//! Deviations/gaps (documented, not silent):
//! - `NT_GIVE`'s fallback (`arkhata.c:4300-4308`) is the driver's only
//!   `NT_GIVE` behavior - it never accepts any item, always hands it back
//!   with the usual "Thou hast better use for this than I do" line, same
//!   precedent as `world::npc::area37::trainer`'s own fallback-only
//!   `NT_GIVE`.
//! - C's own `questlog_done(co, 78)` (`arkhata.c:4269`) is quest 78's
//!   completion, opened elsewhere by `arkhatamonk_driver`'s own `nr == 3`
//!   (Johnatan) persona at `monk_state` `21` - a correction of a stray
//!   doc-comment reference in `world::npc::area37::arkhatamonk`'s own
//!   module doc, which named this completion site "the still-unported
//!   `kidnappee_driver`" (a copy-paste slip: `arkhata.c:4269` is inside
//!   `krenach_driver`, not `kidnappee_driver`).
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `krenach_driver` body at all (matching the `rammy`/`ramin`/`thaipan`
//!   "pure talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:4326`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::CDR_LOSTCON;
use crate::drvlib::offset2dx;
use crate::world::*;

/// C `char_dist(cn, co) > 10` (`arkhata.c:4246`, sibling drivers' own
/// identical guard).
const KRENACH_GREET_DISTANCE: i32 = 10;
/// C `TICKS * 5` (`arkhata.c:4229`).
const KRENACH_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:4234`).
const KRENACH_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:4320`): idle "return to post" threshold.
const KRENACH_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `6 * 50` (`arkhata.c:4260`): the "gate still closed" grumble's own
/// wall-clock throttle, in seconds.
const KRENACH_GRUMBLE_COOLDOWN_SECONDS: i32 = 6 * 50;
/// C quest 78, "The Mysterious Language" - completed here, opened by
/// `world::npc::area37::arkhatamonk`.
const QLOG_MONK_DICTIONARY: usize = 78;
/// C `give_money(co, 5000 * 100, "Krenach Dictionary Quest")`
/// (`arkhata.c:4281`).
const KRENACH_REFUND_GOLD: u32 = 5000 * 100;

/// Per-player facts [`World::process_krenach_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KrenachPlayerFacts {
    /// `PlayerRuntime::arkhata_krenach_state()`.
    pub krenach_state: i32,
    /// `PlayerRuntime::arkhata_krenach_time_seconds()`.
    pub krenach_time: i32,
    /// `PlayerRuntime::arkhata_monk_state()` (`ppd->monk_state`,
    /// `arkhata.c:4257`): gates `rs` `0`.
    pub monk_state: i32,
}

/// A side effect [`World::process_krenach_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KrenachOutcomeEvent {
    /// Write the new `arkhata_ppd.krenach_state` back.
    UpdateKrenachState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->krenach_time = realtime` (`arkhata.c:4262`), the "gate
    /// still closed" grumble's own throttle stamp - does not touch
    /// `krenach_state`.
    UpdateKrenachTime {
        player_id: CharacterId,
        realtime_seconds: i32,
    },
    /// C `questlog_done(co, 78)` (`arkhata.c:4269`).
    QuestDone78 { player_id: CharacterId },
}

impl World {
    /// C `krenach_driver`'s per-tick body (`arkhata.c:4197-4327`).
    pub fn process_krenach_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, KrenachPlayerFacts>,
        now: i32,
        area_id: u16,
    ) -> Vec<KrenachOutcomeEvent> {
        let krenach_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_KRENACH
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for krenach_id in krenach_ids {
            self.process_krenach_messages(krenach_id, player_facts, now, area_id, &mut events);
        }
        events
    }

    #[allow(clippy::too_many_arguments)]
    fn process_krenach_messages(
        &mut self,
        krenach_id: CharacterId,
        player_facts: &HashMap<CharacterId, KrenachPlayerFacts>,
        now: i32,
        area_id: u16,
        events: &mut Vec<KrenachOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Krenach(mut data)) = self
            .characters
            .get(&krenach_id)
            .and_then(|krenach| krenach.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&krenach_id)
            .map(|krenach| std::mem::take(&mut krenach.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.krenach_handle_char_message(
                    krenach_id,
                    &mut data,
                    message,
                    player_facts,
                    now,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.krenach_handle_give_message(krenach_id, message),
                _ => {}
            }
        }

        if let Some(krenach) = self.characters.get_mut(&krenach_id) {
            krenach.driver_state = Some(CharacterDriverState::Krenach(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:4316-4318`).
        if let (Some(krenach), Some((tx, ty))) =
            (self.characters.get(&krenach_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(krenach.x), i32::from(krenach.y), tx, ty) {
                if let Some(krenach_mut) = self.characters.get_mut(&krenach_id) {
                    let _ = turn(krenach_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:4320-4324`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(krenach) = self.characters.get(&krenach_id) {
            match krenach.driver_state.as_ref() {
                Some(CharacterDriverState::Krenach(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + KRENACH_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(krenach) = self.characters.get(&krenach_id) else {
                return;
            };
            let (post_x, post_y) = (krenach.rest_x, krenach.rest_y);
            self.secure_move_driver(
                krenach_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `krenach_driver`'s `NT_CHAR` branch (`arkhata.c:4213-4297`).
    #[allow(clippy::too_many_arguments)]
    fn krenach_handle_char_message(
        &mut self,
        krenach_id: CharacterId,
        data: &mut KrenachDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, KrenachPlayerFacts>,
        now: i32,
        events: &mut Vec<KrenachOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(krenach) = self.characters.get(&krenach_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:4217`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:4223`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:4229`).
        if tick < data.last_talk + KRENACH_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:4234`).
        if tick < data.last_talk + KRENACH_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:4240`).
        if krenach_id == player_id
            || !char_see_char(&krenach, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:4246`).
        if char_dist(&krenach, &player) > KRENACH_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.krenach_state;
        match facts.krenach_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:
            // 4256-4272`) - see the module doc comment.
            0 if facts.monk_state >= 29 => {
                self.npc_quiet_say(
                    krenach_id,
                    "So you have met my grandson? And he is well? Oh by the pickaxe's tip you are one blessed human.",
                );
                events.push(KrenachOutcomeEvent::QuestDone78 { player_id });
                new_state = 2;
                didsay = true;
            }
            // C `else { if (realtime - ppd->krenach_time > 6*50) { say
            // (...); ppd->krenach_time = realtime; } break; }`
            // (`arkhata.c:4259-4264`) - does not set `didsay`.
            0 => {
                if now.saturating_sub(facts.krenach_time) > KRENACH_GRUMBLE_COOLDOWN_SECONDS {
                    self.npc_quiet_say(krenach_id, "Mrec amil groowah! Giln morg awastu.");
                    events.push(KrenachOutcomeEvent::UpdateKrenachTime {
                        player_id,
                        realtime_seconds: now,
                    });
                }
            }
            // C `case 2:` (`arkhata.c:4273-4277`).
            2 => {
                self.npc_quiet_say(
                    krenach_id,
                    "I am very happy to hear news of him. And to finally have someone to talk with.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:4278-4284`).
            3 => {
                self.npc_quiet_say(
                    krenach_id,
                    "10000g for a book is alot of money. I see you have taken good care of it. Here take these 5000g from me. All in all, the price of your adventure should be more reasonable now.",
                );
                self.krenach_give_refund_gold(player_id);
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`arkhata.c:4285-4289`).
            4 => {
                self.npc_quiet_say(krenach_id, "Go in peace, human.");
                new_state = 5;
                didsay = true;
            }
            _ => {}
        }

        if new_state != facts.krenach_state {
            events.push(KrenachOutcomeEvent::UpdateKrenachState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:4291-4295`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `give_money(cn, val, reason)` (`src/system/tool.c:1460-1474`):
    /// adds straight to `Character::gold`, matching `world::npc::
    /// area37::arkhatamonk`'s own `monk_give_money` precedent - this
    /// reward path needs nothing but `World`.
    fn krenach_give_refund_gold(&mut self, player_id: CharacterId) {
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.gold = player.gold.saturating_add(KRENACH_REFUND_GOLD);
            player.flags.insert(CharacterFlags::ITEMS);
        }
        self.queue_system_text_bytes(player_id, give_money_message(KRENACH_REFUND_GOLD));
    }

    /// C `krenach_driver`'s `NT_GIVE` branch (`arkhata.c:4300-4308`): the
    /// only behavior it has is handing the item straight back.
    fn krenach_handle_give_message(
        &mut self,
        krenach_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&krenach_id)
            .and_then(|krenach| krenach.cursor_item.take())
        else {
            return;
        };
        self.npc_say(
            krenach_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_KRENACH;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `krenach_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::ramin`'s `RaminDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KrenachDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_MONK_DICTIONARY`] to `ugaris-server`'s
/// `apply_krenach_events`.
pub const fn qlog_krenach_dictionary() -> usize {
    QLOG_MONK_DICTIONARY
}
