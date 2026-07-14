//! Lab 5 "Mathor" the mage (`CDR_LAB5MAGE`), the friend Laros mentions:
//! explains the force-summon ritual and, once a player performs it
//! correctly, opens a temporary demon room and drags the Master Demon
//! (plus its guards) into it via [`World::finish_ritual_start`].
//!
//! Ports `src/area/22/lab5.c::lab5_mage_driver` (`:518-836`) plus the
//! shared `ritual_start`/`ritual_hurt` (`:114-243`) it drives once a
//! player shouts the correct real name while standing in the name
//! square. All of the dialogue/ritual state (`ppd->magestate`/
//! `ritualdaemon`/`ritualstate`) lives in the player's own
//! `DRD_LAB5_PLAYER` PPD slot (`crate::player::PlayerRuntime::
//! lab5_mage_state`/`lab5_ritual_daemon`/`lab5_ritual_state` - the same
//! slot `world::npc::area22::lab5_seyan` already uses for
//! `seyanstate`/`seyangot`), so this follows the identical `PlayerFacts`/
//! `OutcomeEvent` split. `dat->cv_co`/`cv_serial`/`lasttalk` (C's
//! `static struct lab5_talk_data datbuf`) is this NPC's own
//! [`Lab5MageDriverData`], same "exactly one Mathor is ever spawned"
//! precedent as `lab5_seyan`'s own module doc comment.
//!
//! `ritual_create_char`'s actual character instantiation needs
//! `ZoneLoader` (it's a brand-new character, not a message on an
//! existing one), so - same "defer the ZoneLoader-needing half to
//! ugaris-server" precedent as `world::pents`/`world::npc::area8::
//! fdemon_army` - a successful [`World::attempt_ritual_start`] (pure: room
//! search, clearing existing occupants, statue placement) only returns a
//! [`Lab5RitualPlan`] wrapped in [`Lab5MageOutcomeEvent::
//! AttemptRitualStart`]; `ugaris-server`'s `area22.rs` spawns the 2-4
//! planned demons via `ZoneLoader::instantiate_character_template` (the
//! exact templates - `lab5_one_servant`/`_master`, etc - the C zone data
//! already ships, their own `arg="type=N;"` already feeding the already-
//! ported `apply_lab5_daemon_create_message`) and then calls
//! [`World::finish_ritual_start`] to do C's own spawn-before-teleport-
//! check tail: `teleport_char_driver` first, success/fail messaging
//! second - preserving C's real quirk that a failed teleport still leaves
//! the just-spawned demons behind (`ritual_start` never undoes its own
//! room setup on failure).
//!
//! Deviations/gaps (documented, not silent):
//! - Same message-driven-sighting/single-victim-adjacent precedents as
//!   every other ported dialogue NPC; `standard_message_driver(cn, msg, 0,
//!   0)` is not reproduced (same reasoning as `lab5_seyan`'s own doc
//!   comment).
//! - C's `else if (strcasestr(str, "DEMONS"))` branch (`lab5.c:758-762`)
//!   is unreachable dead code in C itself: the preceding `else if
//!   (strcasestr(str, "DEMON"))` (`lab5.c:753-757`) already matches any
//!   string containing "DEMON", which "DEMONS" always does, so the
//!   if/else-if chain can never reach the "DEMONS" arm. Not ported (both
//!   branches set `magestate = 20` anyway, so this is a true no-op
//!   omission, not a behavior change).
//! - `ritual_start`'s free-room scan (`lab5.c:176-198`) is ported as a
//!   straightforward "does any tile in the candidate rectangle hold a
//!   player" full scan ([`World::lab5_room_has_player`]) rather than C's
//!   literal nested-`for`-loop-with-early-`break`-then-inspect-`x`/`y`-
//!   sentinels idiom, which has an obscure off-by-one: if the *only*
//!   player found is in the rectangle's last row, C's post-loop `y == ey
//!   + 1` check can look identical to "no player found", incorrectly
//!     treating an occupied room as free. This port always gets that one
//!     case right instead of reproducing the C bug - documented as the one
//!     deliberate correctness deviation in this file.
//! - `IDR_LAB5_ITEM`'s nameplate/realnameplate/entrance/backdoor branches
//!   (`drdata[0]` 5/6/7/8) - the normal, non-god way to progress
//!   `ritualstate`/populate `namecoordx[1..=3]`/override
//!   `daemondoorx`/`daemondoory` - are not yet ported; today the ritual is
//!   only reachable via this driver's own god-only `SET 1/2/3` debug
//!   command, and every candidate room uses C's static door-position
//!   initializers (`daemondoorx`/`daemondoory`, unless/until a backdoor
//!   item overrides them).

use crate::direction::Direction;
use crate::drvlib::offset2dx;
use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_LIGHT_RED, COL_STR_RESET};
use crate::world::*;

/// C `char_dist(cn, co) > 7` (`lab5.c:569`): the mage's `NT_CHAR`
/// greeting range (shorter than Laros' own 10-tile range).
const LAB5_MAGE_GREET_DISTANCE: i32 = 7;
/// C `TICKS * 5` (`lab5.c:561`): only talk once the previous line's
/// cooldown has passed.
const LAB5_MAGE_TALK_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 30` (`lab5.c:829`): idle "return to post" threshold.
const LAB5_MAGE_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// C `char *daemonname[4]` (`lab5.c:103`).
const DAEMONNAME: [&str; 4] = ["xxnamexx", "Asfaloth", "Beronath", "Cyradeth"];
/// C `char *daemonreal[4]` (`lab5.c:104`).
const DAEMONREAL: [&str; 4] = ["xxrealxx", "Fao Thals", "Breth Ona", "Ch Dae Tyr"];

/// C `daemonname[daemon]` read, exposed for `ugaris-server`'s
/// `IDR_LAB5_ITEM` nameplate/realnameplate ritual-progress messages
/// (`tick_item_use_lab.rs`), which need the same table this module's own
/// `lab5_ritual_hurt`/`apply_lab5_ritual_hurt_at` use.
pub fn lab5_daemon_name(daemon: u8) -> &'static str {
    DAEMONNAME[usize::from(daemon).min(3)]
}

/// C `daemonreal[daemon]` read, same precedent as [`lab5_daemon_name`].
pub fn lab5_daemon_real_name(daemon: u8) -> &'static str {
    DAEMONREAL[usize::from(daemon).min(3)]
}
/// C `int namecoordx[4]`/`int namecoordy[4]` static initializers
/// (`lab5.c:105-107`). See `World::lab5_namecoord`.
pub(crate) const LAB5_NAMECOORD_DEFAULTS: [(i32, i32); 4] =
    [(85, 33), (90, 28), (85, 23), (80, 28)];
/// C `#define MAXDOOR 4` / `int daemondoorx[MAXDOOR]`/`daemondoory[MAXDOOR]`
/// static initializers (`lab5.c:109-111`). `IDR_LAB5_ITEM`'s `drdata[0]==8`
/// backdoor branch (not yet ported) is the only thing that ever overrides
/// these in C.
const LAB5_DAEMON_DOORS: [(i32, i32); 4] = [(119, 108), (119, 95), (119, 82), (119, 69)];
/// C `static int statue1[4]`/`statue2[4]` (`lab5.c:172-173`), indexed by
/// daemon number (1..=3; index 0 unused).
const LAB5_STATUE1: [i32; 4] = [0, 11165, 11123, 11157];
const LAB5_STATUE2: [i32; 4] = [0, 11167, 11125, 11159];

/// Per-player facts [`World::process_lab5_mage_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lab5MagePlayerFacts {
    /// `PlayerRuntime::lab5_mage_state`.
    pub magestate: u8,
    /// `PlayerRuntime::lab5_ritual_daemon`.
    pub ritualdaemon: u8,
    /// `PlayerRuntime::lab5_ritual_state`.
    pub ritualstate: u8,
}

/// One planned `ritual_create_char` call (`lab5.c:224-235`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lab5RitualDemonSpawn {
    pub template: &'static str,
    pub x: i32,
    pub y: i32,
    pub dir: u8,
    /// Seconds, C's raw `attackstart` argument - `ritual_create_char`
    /// itself multiplies by `TICKS` (`lab5.c:166`).
    pub attackstart_seconds: i32,
}

/// The pure-`World` half of a successful `ritual_start` room search
/// (`lab5.c:170-243`): the room is already cleared/statued by the time
/// this is returned. See module doc comment for why the demon spawns and
/// final teleport are deferred to `ugaris-server`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lab5RitualPlan {
    pub daemon: u8,
    pub door_x: i32,
    pub door_y: i32,
    pub spawns: Vec<Lab5RitualDemonSpawn>,
}

/// A side effect [`World::process_lab5_mage_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lab5MageOutcomeEvent {
    /// Write the new `ppd->magestate` back.
    SetMageState {
        player_id: CharacterId,
        magestate: u8,
    },
    /// Write the new `ppd->ritualdaemon`/`ritualstate` back (the god
    /// `SET` command, or a `ritual_hurt` reset).
    SetRitual {
        player_id: CharacterId,
        ritualdaemon: u8,
        ritualstate: u8,
    },
    /// A free room was found and cleared/statued; `ugaris-server` must
    /// spawn `plan.spawns` via `ZoneLoader` and then call
    /// [`World::finish_ritual_start`] (which itself decides whether to
    /// reset `ritualstate`, reported back via its `bool` return).
    AttemptRitualStart {
        player_id: CharacterId,
        mage_id: CharacterId,
        plan: Lab5RitualPlan,
    },
}

impl World {
    /// C `namecoordx[i]`/`namecoordy[i]` read (`lab5.c:105-107`): `None`
    /// (not yet overridden by the mage's `NT_CREATE`) falls back to C's
    /// static initializer default. `pub` (not `pub(crate)`) so
    /// `ugaris-server`'s `IDR_LAB5_ITEM` backdoor/entrance-hurt
    /// resolution (`tick_item_use_lab.rs`) can read the same live
    /// coordinates this module's own ritual logic uses.
    pub fn lab5_namecoord(&self, index: usize) -> (i32, i32) {
        self.lab5_namecoords
            .get(index)
            .copied()
            .flatten()
            .unwrap_or_else(|| LAB5_NAMECOORD_DEFAULTS[index.min(3)])
    }

    /// C `lab5_mage_driver`'s per-tick body (`lab5.c:518-836`).
    pub fn process_lab5_mage_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, Lab5MagePlayerFacts>,
        area_id: u16,
    ) -> Vec<Lab5MageOutcomeEvent> {
        let mage_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAB5MAGE
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for mage_id in mage_ids {
            self.process_lab5_mage_messages(mage_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_lab5_mage_messages(
        &mut self,
        mage_id: CharacterId,
        player_facts: &HashMap<CharacterId, Lab5MagePlayerFacts>,
        area_id: u16,
        events: &mut Vec<Lab5MageOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Lab5Mage(mut data)) = self
            .characters
            .get(&mage_id)
            .and_then(|mage| mage.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&mage_id)
            .map(|mage| std::mem::take(&mut mage.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                // C `if (msg->type == NT_CREATE) { namecoordx[0] =
                // ch[cn].x; namecoordy[0] = ch[cn].y; }` (`lab5.c:532-535`).
                NT_CREATE => {
                    if let Some(mage) = self.characters.get(&mage_id) {
                        self.lab5_namecoords[0] = Some((i32::from(mage.x), i32::from(mage.y)));
                    }
                }
                // C `lab5.c:537-547`: unconditional destroy-whatever-was-
                // given, no head-tracking (unlike `lab5_seyan`).
                NT_GIVE => {
                    if let Some(item_id) = self
                        .characters
                        .get(&mage_id)
                        .and_then(|mage| mage.cursor_item)
                    {
                        self.destroy_item(item_id);
                    }
                }
                NT_CHAR => self.lab5_mage_handle_char_message(
                    mage_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.lab5_mage_handle_text_message(
                    mage_id,
                    message,
                    player_facts,
                    area_id,
                    events,
                ),
                _ => {}
            }
        }

        if let Some(mage) = self.characters.get_mut(&mage_id) {
            mage.driver_state = Some(CharacterDriverState::Lab5Mage(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`lab5.c:825-827`).
        if let (Some(mage), Some((tx, ty))) = (self.characters.get(&mage_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(mage.x), i32::from(mage.y), tx, ty) {
                if let Some(mage_mut) = self.characters.get_mut(&mage_id) {
                    let _ = turn(mage_mut, direction as u8);
                }
            }
        }

        // C `if (dat->lasttalk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_UP, ret,
        // lastact)) return; } do_idle(cn, TICKS);` (`lab5.c:829-835`).
        if data.lasttalk + LAB5_MAGE_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(mage) = self.characters.get(&mage_id) else {
                return;
            };
            let (post_x, post_y) = (mage.rest_x, mage.rest_y);
            self.secure_move_driver(mage_id, post_x, post_y, Direction::Up as u8, 0, 0, area_id);
        }
    }

    /// C `lab5_mage_driver`'s `NT_CHAR` branch (`lab5.c:549-717`): the
    /// intro/force/demon/ritual-explanation dialogue ladder, keyed off
    /// the seen player's own `ppd->magestate`.
    #[allow(clippy::too_many_arguments)]
    fn lab5_mage_handle_char_message(
        &mut self,
        mage_id: CharacterId,
        data: &mut Lab5MageDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab5MagePlayerFacts>,
        events: &mut Vec<Lab5MageOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(mage) = self.characters.get(&mage_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        if tick < data.lasttalk + LAB5_MAGE_TALK_COOLDOWN_TICKS {
            return;
        }
        if mage_id == player_id || !char_see_char(&mage, &player, &self.map, self.date.daylight) {
            return;
        }
        if char_dist(&mage, &player) > LAB5_MAGE_GREET_DISTANCE {
            return;
        }

        if let Some(cv_co) = data.cv_co {
            let still_valid = self.characters.get(&cv_co).is_some_and(|cv| {
                cv.serial == data.cv_serial
                    && char_dist(&mage, cv) <= LAB5_MAGE_GREET_DISTANCE
                    && char_see_char(&mage, cv, &self.map, self.date.daylight)
            });
            if !still_valid {
                data.cv_co = None;
            }
        }

        if let Some(cv_co) = data.cv_co {
            if cv_co != player_id {
                return;
            }
        }

        if data.cv_co.is_none() {
            data.cv_co = Some(player_id);
            data.cv_serial = player.serial;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.magestate;
        let mut clear_cv = false;

        match facts.magestate {
            0 => {
                self.npc_say(
                    mage_id,
                    &format!(
                        "Hello {}. My name is Mathor, and I am the friend Laros surely \
                         mentioned.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 1;
            }
            1 => {
                self.npc_say(
                    mage_id,
                    &format!(
                        "It is the entrance room to the Master {COL_STR_LIGHT_BLUE}Demons\
                         {COL_STR_RESET} what thou see here. Those stone plates show their \
                         names. But thou have to find their real names written on similar \
                         plates somewhere behind those doors here."
                    ),
                );
                didsay = true;
                new_state = 2;
            }
            2 => {
                self.npc_say(
                    mage_id,
                    &format!(
                        "Once thou foundst the real name of a Master Demon, Thou can \
                         {COL_STR_LIGHT_BLUE}force{COL_STR_RESET} him to summon thee into his \
                         place, and fight him there. Thou might ask me for more details, if \
                         thou art interested."
                    ),
                );
                didsay = true;
                new_state = 3;
            }
            3 => {
                self.npc_say(mage_id, "And thou shouldst be!");
                didsay = true;
                new_state = 4;
            }
            4 => {
                clear_cv = true;
            }
            10 => {
                self.npc_say(
                    mage_id,
                    &format!(
                        "Well {}. To force a Master {COL_STR_LIGHT_BLUE}Demon{COL_STR_RESET} \
                         to summon thee into his place thou have to perform a certain \
                         {COL_STR_LIGHT_BLUE}ritual{COL_STR_RESET} first. But be very careful, \
                         {0}. If thou makest only one mistake it might kill thee. The powers \
                         that are working here are strong.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 11;
            }
            11 => {
                self.npc_say(mage_id, "Very strong indeed!");
                didsay = true;
                new_state = 12;
            }
            12 => {
                clear_cv = true;
            }
            20 => {
                self.npc_say(
                    mage_id,
                    &format!(
                        "Well {}, unfortunetaly those Master Demons can't be hurt by normal \
                         weapons. So make sure thou art properly equipped with a sacred stone \
                         weapon when fighting the Masters.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 21;
            }
            21 => {
                self.npc_say(
                    mage_id,
                    "I have heard that those weapon might be found somewhere in the section \
                     behind the south western door of this room.",
                );
                didsay = true;
                new_state = 22;
            }
            22 => {
                clear_cv = true;
            }
            30 => {
                self.npc_say(
                    mage_id,
                    &format!(
                        "Oh {}, it's a ritual of mighty powers thou art asking for. So listen \
                         carefully.",
                        player.name
                    ),
                );
                didsay = true;
                new_state = 31;
            }
            31 => {
                self.npc_say(
                    mage_id,
                    "First, thou hast to touch the stone plate of the Demons name.",
                );
                didsay = true;
                new_state = 32;
            }
            32 => {
                self.npc_say(
                    mage_id,
                    "Second, thou hast to touch the correct stone plate of the Demons real \
                     name. Those thou hast to find.",
                );
                didsay = true;
                new_state = 33;
            }
            33 => {
                self.npc_say(
                    mage_id,
                    "Third, thou hast to enter the inner square from the opposite entrance.",
                );
                didsay = true;
                new_state = 34;
            }
            34 => {
                self.npc_say(
                    mage_id,
                    "Then place thineself in the center, and shout the real name of the \
                     Master.",
                );
                didsay = true;
                new_state = 35;
            }
            35 => {
                self.npc_say(mage_id, "Well, thats it. Prepare to fight him then.");
                didsay = true;
                new_state = 36;
            }
            36 => {
                self.npc_say(
                    mage_id,
                    "Ah, and thou couldst do it in any order, but may I suggest doing them \
                     from the east to the west.",
                );
                didsay = true;
                new_state = 37;
            }
            37 => {
                clear_cv = true;
            }
            _ => {}
        }

        if new_state != facts.magestate {
            events.push(Lab5MageOutcomeEvent::SetMageState {
                player_id,
                magestate: new_state,
            });
        }
        if clear_cv {
            data.cv_co = None;
        }

        if didsay {
            data.lasttalk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
        }
    }

    /// C `lab5_mage_driver`'s `NT_TEXT` branch (`lab5.c:719-819`): the
    /// `tabunga` debug echo, the `REPEAT`/`FORCE`/`DEMON`/`RITUAL`
    /// keyword jumps, the god-only `SET 1/2/3` ritual-state debug
    /// command, and the "inside the name square, shouted the real name"
    /// ritual invocation check.
    fn lab5_mage_handle_text_message(
        &mut self,
        mage_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab5MagePlayerFacts>,
        area_id: u16,
        events: &mut Vec<Lab5MageOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };

        self.apply_tabunga_text_notification(mage_id, speaker_id, text);

        if speaker_id == mage_id {
            return;
        }
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(mage) = self.characters.get(&mage_id).cloned() else {
            return;
        };
        if !char_see_char(&mage, &speaker, &self.map, self.date.daylight) {
            return;
        }
        let Some(facts) = player_facts.get(&speaker_id) else {
            return;
        };

        let upper = text.to_ascii_uppercase();
        let is_god = speaker.flags.contains(CharacterFlags::GOD);

        let mut magestate = facts.magestate;
        let mut magestate_changed = false;
        let mut ritualdaemon = facts.ritualdaemon;
        let mut ritualstate = facts.ritualstate;
        let mut ritual_changed = false;

        // C `lab5.c:745-782`: the `REPEAT`/`FORCE`/`DEMON`/`RITUAL`/
        // god-only-`SET` if/else-if chain. `DEMONS` is unreachable dead
        // code in C - see module doc comment.
        if upper.contains("REPEAT") {
            magestate = 0;
            magestate_changed = true;
            self.npc_say(mage_id, &format!("I will repeat, {}", speaker.name));
        } else if upper.contains("FORCE") {
            magestate = 10;
            magestate_changed = true;
            self.lab5_mage_patience_check(mage_id, &speaker, speaker_id);
        } else if upper.contains("DEMON") {
            magestate = 20;
            magestate_changed = true;
            self.lab5_mage_patience_check(mage_id, &speaker, speaker_id);
        } else if upper.contains("RITUAL") {
            magestate = 30;
            magestate_changed = true;
            self.lab5_mage_patience_check(mage_id, &speaker, speaker_id);
        } else if is_god {
            for (marker, daemon) in [("SET 1", 1u8), ("SET 2", 2u8), ("SET 3", 3u8)] {
                if upper.contains(marker) {
                    ritualdaemon = daemon;
                    ritualstate = 3;
                    ritual_changed = true;
                    self.npc_say(
                        mage_id,
                        &format!(
                            "set {ritualdaemon} {ritualstate} ({})",
                            DAEMONREAL[ritualdaemon as usize]
                        ),
                    );
                    break;
                }
            }
        }

        if magestate_changed {
            events.push(Lab5MageOutcomeEvent::SetMageState {
                player_id: speaker_id,
                magestate,
            });
        }

        // C `lab5.c:785-818`: the "inside the name square, wants to
        // shout/SET-call" ritual check - unconditional, independent of
        // which (if any) branch above fired, and reads the *possibly
        // just-updated* `ritualstate`/`ritualdaemon`.
        let (x3, _) = self.lab5_namecoord(3);
        let (x1, y1) = self.lab5_namecoord(1);
        let (x2, y2) = self.lab5_namecoord(2);
        let (_, y0) = self.lab5_namecoord(0);
        let inside_square = ritualstate != 0
            && i32::from(speaker.x) > x3 + 2
            && i32::from(speaker.x) < x1 - 2
            && i32::from(speaker.y) > y2 + 2
            && i32::from(speaker.y) < y0 - 2
            && text.contains(':');

        if inside_square {
            let mut called = 0u8;
            if upper.contains("SHOUTS:") {
                for (candidate, daemon) in [
                    (DAEMONREAL[1], 1u8),
                    (DAEMONREAL[2], 2u8),
                    (DAEMONREAL[3], 3u8),
                ] {
                    if text
                        .to_ascii_uppercase()
                        .contains(&candidate.to_ascii_uppercase())
                    {
                        called = daemon;
                        break;
                    }
                }
            } else if is_god && upper.contains("SET") {
                called = ritualdaemon;
            }

            // C `say(cn, "%d %d %d", pd->ritualdaemon, called,
            // pd->ritualstate);` (`lab5.c:801`) - an unconditional debug
            // leftover, reproduced digit-for-digit.
            self.npc_say(mage_id, &format!("{ritualdaemon} {called} {ritualstate}"));

            if ritualstate == 3
                && ritualdaemon == called
                && i32::from(speaker.x) == x2
                && i32::from(speaker.y) == y1
            {
                match self.attempt_ritual_start(ritualdaemon) {
                    Some(plan) => {
                        events.push(Lab5MageOutcomeEvent::AttemptRitualStart {
                            player_id: speaker_id,
                            mage_id,
                            plan,
                        });
                    }
                    None => {
                        self.lab5_ritual_call_again(speaker_id);
                    }
                }
            } else {
                self.lab5_ritual_hurt(speaker_id, ritualdaemon);
                ritualdaemon = 0;
                ritualstate = 0;
                ritual_changed = true;
            }
        }

        if ritual_changed {
            events.push(Lab5MageOutcomeEvent::SetRitual {
                player_id: speaker_id,
                ritualdaemon,
                ritualstate,
            });
        }
        let _ = area_id;
    }

    /// C's repeated `if (dat->cv_co && (dat->cv_co != co ||
    /// ch[dat->cv_co].serial != dat->cv_serial)) say(cn, "%s, please be
    /// patient while i'm talking to others.", ch[co].name);` snippet
    /// (`lab5.c:750-751,755-756,760-761,765-766`) - identical at all four
    /// call sites, factored out. This only reads `data.cv_co` (not
    /// mutate), so it takes the mage id directly rather than the driver
    /// data (the caller already holds a `&mut Lab5MageDriverData`
    /// borrowed elsewhere in the same message loop iteration in C, but
    /// this port's `NT_TEXT` handling never touches `cv_co`).
    fn lab5_mage_patience_check(
        &mut self,
        mage_id: CharacterId,
        speaker: &Character,
        speaker_id: CharacterId,
    ) {
        let Some(CharacterDriverState::Lab5Mage(data)) = self
            .characters
            .get(&mage_id)
            .and_then(|mage| mage.driver_state.clone())
        else {
            return;
        };
        if let Some(cv_co) = data.cv_co {
            let cv_still_matches = cv_co == speaker_id
                && self
                    .characters
                    .get(&cv_co)
                    .is_some_and(|cv| cv.serial == data.cv_serial);
            if !cv_still_matches {
                self.npc_say(
                    mage_id,
                    &format!(
                        "{}, please be patient while i'm talking to others.",
                        speaker.name
                    ),
                );
            }
        }
    }

    /// C `ritual_hurt` (`lab5.c:114-129`): ends the ritual attempt with a
    /// pulseback effect at the offending name plate's position plus 5
    /// `POWERSCALE` of unarmored self-damage. May kill `player_id` (same
    /// warning C's own comment carries). This is the `lab5_mage_driver`
    /// NT_TEXT call site's own coordinate source (`lab5.c:816`,
    /// `namecoordx/y[pd->ritualdaemon]`); see
    /// [`Self::apply_lab5_ritual_hurt_at`] for the shared explicit-(x,y)
    /// half the `IDR_LAB5_ITEM` item driver's three other call sites
    /// (`lab5.c:1263,1287,1317`) need.
    fn lab5_ritual_hurt(&mut self, player_id: CharacterId, ritualdaemon: u8) {
        let (x, y) = self.lab5_namecoord(usize::from(ritualdaemon));
        self.apply_lab5_ritual_hurt_at(player_id, x, y, ritualdaemon);
    }

    /// Shared body of C `ritual_hurt` (`lab5.c:114-129`), taking an
    /// explicit effect position instead of always resolving it via
    /// `namecoordx/y[ritualdaemon]` - `IDR_LAB5_ITEM`'s nameplate/
    /// realnameplate branches use the touched item's own position
    /// (`it[in].x`/`it[in].y`) and its entrance branch uses
    /// `namecoordx/y[hurttrans[drdata[1]]]`, neither of which is
    /// `namecoordx/y[ritualdaemon]`. `pub` so `ugaris-server`'s
    /// `tick_item_use_lab.rs` can call it once it has resolved the
    /// item-driver-specific (x, y); does *not* touch
    /// `PlayerRuntime::lab5_ritual_daemon`/`_state` (C's `pd->ritualdaemon
    /// = 0; pd->ritualstate = 0;`) since `World` cannot see
    /// `PlayerRuntime` - the caller resets both after this returns,
    /// matching the existing `lab5_mage_driver` NT_TEXT call site's own
    /// post-call reset (`lab5_mage.rs`'s `ritualdaemon = 0; ritualstate =
    /// 0;` right after its own `lab5_ritual_hurt` call).
    pub fn apply_lab5_ritual_hurt_at(
        &mut self,
        player_id: CharacterId,
        x: i32,
        y: i32,
        daemon: u8,
    ) {
        let effect_id = self.create_show_effect(
            EF_PULSEBACK,
            player_id,
            self.tick.0 as u32,
            self.tick.0.saturating_add(17) as u32,
            20,
            42,
        );
        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.x = x;
            effect.y = y;
        }
        self.queue_system_text(
            player_id,
            format!(
                "{COL_STR_LIGHT_RED}The Ritual Of {} ended.{COL_STR_RESET}",
                DAEMONNAME[usize::from(daemon).min(3)]
            ),
        );
        let _ = self.apply_legacy_hurt(player_id, None, 5 * POWERSCALE, 1, 0, 0);
    }

    /// The `ritual_start` "no free room found" fail path
    /// (`lab5.c:811-814`), shared with [`Self::finish_ritual_start`]'s own
    /// failed-teleport branch.
    fn lab5_ritual_call_again(&mut self, player_id: CharacterId) {
        self.queue_system_text(
            player_id,
            "Thou have to call again, but wait a while to do so!".to_string(),
        );
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.endurance =
                i32::from(player.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
        }
    }

    /// C `ritual_start`'s room-search/cleanup/statue half
    /// (`lab5.c:170-243`, up to but not including `ritual_create_char`/
    /// `teleport_char_driver`) - see module doc comment for the
    /// ZoneLoader-needing remainder.
    fn attempt_ritual_start(&mut self, daemon: u8) -> Option<Lab5RitualPlan> {
        if !(1..=3).contains(&daemon) {
            return None;
        }
        for (door_x, door_y) in LAB5_DAEMON_DOORS {
            let sx = door_x;
            let sy = door_y - 6;
            let ex = door_x + 14;
            let ey = door_y + 6;
            if self.lab5_room_has_player(sx, sy, ex, ey) {
                continue;
            }
            self.lab5_clear_room_of_npcs(sx, sy, ex, ey);
            self.lab5_place_ritual_statues(door_x, door_y, daemon);
            return Some(Lab5RitualPlan {
                daemon,
                door_x,
                door_y,
                spawns: lab5_ritual_demon_spawns(daemon, door_x, door_y),
            });
        }
        None
    }

    fn lab5_room_has_player(&self, sx: i32, sy: i32, ex: i32, ey: i32) -> bool {
        for y in sy..=ey {
            let Ok(uy) = usize::try_from(y) else {
                continue;
            };
            for x in sx..=ex {
                let Ok(ux) = usize::try_from(x) else {
                    continue;
                };
                let Some(tile) = self.map.tile(ux, uy) else {
                    continue;
                };
                if tile.character == 0 {
                    continue;
                }
                let character_id = CharacterId(u32::from(tile.character));
                if self
                    .characters
                    .get(&character_id)
                    .is_some_and(|character| character.flags.contains(CharacterFlags::PLAYER))
                {
                    return true;
                }
            }
        }
        false
    }

    fn lab5_clear_room_of_npcs(&mut self, sx: i32, sy: i32, ex: i32, ey: i32) {
        let mut to_remove = Vec::new();
        for y in sy..=ey {
            let Ok(uy) = usize::try_from(y) else {
                continue;
            };
            for x in sx..=ex {
                let Ok(ux) = usize::try_from(x) else {
                    continue;
                };
                let Some(tile) = self.map.tile(ux, uy) else {
                    continue;
                };
                if tile.character == 0 {
                    continue;
                }
                let character_id = CharacterId(u32::from(tile.character));
                if self
                    .characters
                    .get(&character_id)
                    .is_some_and(|character| !character.flags.contains(CharacterFlags::PLAYER))
                {
                    to_remove.push(character_id);
                }
            }
        }
        for character_id in to_remove {
            self.remove_character(character_id);
        }
    }

    fn lab5_place_ritual_statues(&mut self, door_x: i32, door_y: i32, daemon: u8) {
        let statue1 = LAB5_STATUE1[usize::from(daemon)];
        let statue2 = LAB5_STATUE2[usize::from(daemon)];
        for (dx, dy, sprite) in [
            (2, -2, statue1),
            (2, 2, statue1),
            (12, -2, statue2),
            (12, 2, statue2),
        ] {
            let (Ok(x), Ok(y)) = (usize::try_from(door_x + dx), usize::try_from(door_y + dy))
            else {
                continue;
            };
            if let Some(tile) = self.map.tile_mut(x, y) {
                tile.foreground_sprite = sprite as u32;
            }
            self.mark_dirty_sector(x, y);
        }
    }

    /// C `ritual_start`'s tail (`lab5.c:239-242`), run by `ugaris-server`
    /// once it has spawned every demon in the matching [`Lab5RitualPlan`]
    /// (see module doc comment for why C's own spawn-before-teleport-
    /// check order matters). Returns whether the ritual succeeded (the
    /// caller resets `PlayerRuntime::lab5_ritual_state` only on `true`,
    /// matching C's `pd->ritualstate = 0;` being conditional on
    /// `ritual_start`'s return value).
    pub fn finish_ritual_start(
        &mut self,
        player_id: CharacterId,
        mage_id: CharacterId,
        door_x: i32,
        door_y: i32,
        daemon: u8,
    ) -> bool {
        let target_x = (door_x + 1).clamp(0, i32::from(u16::MAX)) as u16;
        let target_y = door_y.clamp(0, i32::from(u16::MAX)) as u16;
        if self.teleport_char_driver(player_id, target_x, target_y) {
            if let Some(mage) = self.characters.get(&mage_id) {
                self.queue_sound_area(usize::from(mage.x), usize::from(mage.y), 41);
            }
            self.queue_system_text(
                player_id,
                format!(
                    "{COL_STR_LIGHT_RED}The Ritual of {} is fulfilled.{COL_STR_RESET}",
                    DAEMONREAL[usize::from(daemon).min(3)]
                ),
            );
            true
        } else {
            self.lab5_ritual_call_again(player_id);
            false
        }
    }
}

/// C `ritual_start`'s `daemon == 1/2/3` `ritual_create_char` calls
/// (`lab5.c:223-236`).
fn lab5_ritual_demon_spawns(daemon: u8, door_x: i32, door_y: i32) -> Vec<Lab5RitualDemonSpawn> {
    match daemon {
        1 => vec![
            Lab5RitualDemonSpawn {
                template: "lab5_one_servant",
                x: door_x + 10,
                y: door_y - 2,
                dir: Direction::Left as u8,
                attackstart_seconds: 7,
            },
            Lab5RitualDemonSpawn {
                template: "lab5_one_servant",
                x: door_x + 10,
                y: door_y + 2,
                dir: Direction::Left as u8,
                attackstart_seconds: 7,
            },
            Lab5RitualDemonSpawn {
                template: "lab5_one_master",
                x: door_x + 12,
                y: door_y,
                dir: Direction::Left as u8,
                attackstart_seconds: 8,
            },
        ],
        2 => vec![
            Lab5RitualDemonSpawn {
                template: "lab5_two_servant",
                x: door_x + 10,
                y: door_y - 2,
                dir: Direction::Left as u8,
                attackstart_seconds: 7,
            },
            Lab5RitualDemonSpawn {
                template: "lab5_two_servant",
                x: door_x + 10,
                y: door_y + 2,
                dir: Direction::Left as u8,
                attackstart_seconds: 7,
            },
            Lab5RitualDemonSpawn {
                template: "lab5_two_master",
                x: door_x + 12,
                y: door_y,
                dir: Direction::Left as u8,
                attackstart_seconds: 8,
            },
        ],
        3 => vec![
            Lab5RitualDemonSpawn {
                template: "lab5_three_servant_mage",
                x: door_x + 6,
                y: door_y - 4,
                dir: Direction::Down as u8,
                attackstart_seconds: 7,
            },
            Lab5RitualDemonSpawn {
                template: "lab5_three_servant_mage",
                x: door_x + 6,
                y: door_y + 4,
                dir: Direction::Up as u8,
                attackstart_seconds: 7,
            },
            Lab5RitualDemonSpawn {
                template: "lab5_three_servant",
                x: door_x + 12,
                y: door_y + 1,
                dir: Direction::Left as u8,
                attackstart_seconds: 8,
            },
            Lab5RitualDemonSpawn {
                template: "lab5_three_master",
                x: door_x + 12,
                y: door_y - 1,
                dir: Direction::Left as u8,
                attackstart_seconds: 8,
            },
        ],
        _ => Vec::new(),
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct lab5_talk_data` reused verbatim by the mage driver
/// (`lab5.c:520-521`) - same "C's function-local static is safe as
/// per-character state since exactly one Mathor is ever spawned"
/// precedent as [`super::lab5_seyan::Lab5SeyanDriverData`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab5MageDriverData {
    #[serde(default)]
    pub cv_co: Option<CharacterId>,
    #[serde(default)]
    pub cv_serial: u32,
    #[serde(default)]
    pub lasttalk: u64,
}

/// C never parses zone-file args for Mathor (`zones/22/lab5.chr`'s
/// `lab5_mage` template has no `arg=`), but C `create_char` generically
/// fires `notify_char(n, NT_CREATE, ticker, 0, 0)` (`create.c:1128`) and
/// `lab5_mage_driver` genuinely reads it (`namecoordx[0] = ch[cn].x;
/// namecoordy[0] = ch[cn].y;`, `lab5.c:532-535`), unlike `CDR_LAB5SEYAN`
/// (which has no `NT_CREATE` handler at all) - so, same "push it because
/// the driver actually consumes it" precedent as `CDR_GATE_FIGHT` above,
/// the message is queued here.
pub fn apply_lab5_mage_create_message(character: &mut Character) {
    character.driver_state = Some(CharacterDriverState::Lab5Mage(Lab5MageDriverData::default()));
    character.push_driver_message(NT_CREATE, 0, 0, 0);
}
