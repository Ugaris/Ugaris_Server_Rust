//! Rouven NPC (`CDR_ROUVEN`), the Imperial Vault guard who runs quests 62
//! ("Tunnel Magics") and 63 ("Chronicles of Seyan") - the direct
//! continuation of `world::npc::area3::carlos`'s ritual quest chain
//! (quest 61).
//!
//! Ports `src/area/26/staffer.c::rouven_driver` (`:681-914`) plus its
//! shared `analyse_text_driver`/`qa[]` table (`:90-101,110-208`), ported
//! as [`super::AREA26_QA`] in `world::npc::area26` (the same table
//! `world::npc::area26::smugglecom` shares). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area3::
//! thomas`/`sir_jones`: the caller supplies a per-player fact snapshot
//! ([`RouvenPlayerFacts`]) up front and applies the returned
//! [`RouvenOutcomeEvent`]s afterwards, since `staffer_ppd.rouven_state`
//! and the `QLOG` 62/63 quest-log entries live on
//! `crate::player::PlayerRuntime`, not `World`.
//!
//! `rouven_driver`'s fourteen-state (`0`-`13`) dialogue chain: greeting
//! gated on `carlos2_state != 0` (i.e. Carlos has already sent the player
//! here) -> "find the source of the curse" -> "it was in the armory" ->
//! "through the left door" -> "good luck" -> (external: `vault_skull`,
//! `IDR_STAFFER` `drdata[0]==4`, already ported - completes quest 62 and
//! advances `rouven_state` to `6`) -> "we'll look into the demons" ->
//! "retrieve the chronicles of Seyan I" -> "stored in the archives" ->
//! (`NT_GIVE`: hand in `IID_MAX_CHRONICLES`, quest 63 done, state jumps to
//! `10`) -> "thank you" -> "now the magical ritual, in the treasury" ->
//! "take this key" (grants `vault_key1`/`IID_MAX_VAULTKEY` if not already
//! carried) -> done (the player returns to `carlos_driver`'s own
//! `carlos2_state` chain to turn in the ritual scroll).
//!
//! Deviations/gaps (documented, not silent):
//! - C's three-way `if`/`if`/`if` "repeat"/"restart" range-reset ladder
//!   (`:842-854`) has no `else`, but the three ranges (`0..=5`, `6..=9`,
//!   `10..=13`) are mutually exclusive, so at most one branch can ever
//!   fire - ported as a plain three-way `match` over the range.
//! - Unlike `world::thomas`/`world::sir_jones`'s `NT_TEXT` branch (but
//!   like `world::npc::area26::smugglecom`'s), this driver's own C body
//!   has no `dat->current_victim` staleness-reset preamble and no victim-
//!   mismatch early-out at all - reproduced verbatim: replies to *any*
//!   nearby player's matched small talk, not just its tracked victim.
//! - `rouven_driver`'s text switch only handles `qa[].answer_code == 2`
//!   (repeat/restart); unlike `smugglecom_driver`, it has no `case 3`
//!   ("reset me") of its own, even though the shared `qa[]` table still
//!   defines that entry - matching every other matched code falling
//!   through to the generic "still counts as `didsay`" catch-all.
//! - No self-defense/regen/spell-self cascade exists in C's `rouven_
//!   driver` body at all (matching `world::astro1`/`world::npc::area26::
//!   smugglecom`'s identical observation for other "pure talker" NPCs) -
//!   this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:913`) is not
//!   ported, matching the established `world::thomas`/`world::sir_jones`
//!   precedent for stationary dialogue NPCs.

use std::collections::HashMap;

use crate::character_driver::{
    analyse_text_qa, mem_add_driver, mem_check_driver, TextAnalysisOutcome,
};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA26_QA;

/// C `char_dist(cn, co) > 10` (`staffer.c:730`).
const ROUVEN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`staffer.c:129`, the shared
/// `analyse_text_driver` copy's own guard).
const ROUVEN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`staffer.c:713`).
const ROUVEN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`staffer.c:718`).
const ROUVEN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`staffer.c:907`): idle "return to post" threshold.
const ROUVEN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `questlog_open(co, 62)` (`staffer.c:754`).
const QLOG_ROUVEN_TUNNEL: usize = 62;
/// C `questlog_open(co, 63)` (`staffer.c:786`).
const QLOG_ROUVEN_CHRONICLES: usize = 63;

/// Per-player facts [`World::process_rouven_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouvenPlayerFacts {
    /// `PlayerRuntime::staffer_rouven_state()`.
    pub rouven_state: i32,
    /// `PlayerRuntime::staffer_carlos2_state()` (C `ppd->carlos2_state`):
    /// gates `case 0`'s greeting on whether Carlos has already sent the
    /// player here.
    pub carlos2_state: i32,
}

/// A side effect [`World::process_rouven_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouvenOutcomeEvent {
    /// Write the new `staffer_ppd.rouven_state` back.
    UpdateRouvenState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, ...)`.
    QuestOpen {
        player_id: CharacterId,
        quest: usize,
    },
    /// C `questlog_done(co, 63)` (`staffer.c:875`) - applied via the
    /// standard `complete_legacy` flow (the real quest-table exp, no
    /// manual item reward). Quest 62's own completion happens inside the
    /// already-ported `vault_skull` item driver, not here.
    QuestDone { player_id: CharacterId },
    /// C `case 12:` (`staffer.c:813-822`): `if (!has_item(co,
    /// IID_MAX_VAULTKEY) && (in2 = create_item("vault_key1"))) { ...
    /// give_char_item ... }` - the eligibility check
    /// (`character_has_item_template`) already ran in `World`; only the
    /// `ZoneLoader`-needing `create_item`/`give_char_item` pair remains
    /// for `ugaris-server`, same precedent as `world::npc::area3::astro2`'s
    /// `QuestDone` money reward.
    GrantVaultKey { player_id: CharacterId },
}

impl World {
    /// C `rouven_driver`'s per-tick body (`staffer.c:681-914`).
    pub fn process_rouven_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, RouvenPlayerFacts>,
        area_id: u16,
    ) -> Vec<RouvenOutcomeEvent> {
        let rouven_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ROUVEN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for rouven_id in rouven_ids {
            self.process_rouven_messages(rouven_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_rouven_messages(
        &mut self,
        rouven_id: CharacterId,
        player_facts: &HashMap<CharacterId, RouvenPlayerFacts>,
        area_id: u16,
        events: &mut Vec<RouvenOutcomeEvent>,
    ) {
        let Some(rouven_name) = self
            .characters
            .get(&rouven_id)
            .map(|rouven| rouven.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Rouven(mut data)) = self
            .characters
            .get(&rouven_id)
            .and_then(|rouven| rouven.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&rouven_id)
            .map(|rouven| std::mem::take(&mut rouven.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.rouven_handle_char_message(
                    rouven_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.rouven_handle_text_message(
                    rouven_id,
                    &rouven_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.rouven_handle_give_message(rouven_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(rouven) = self.characters.get_mut(&rouven_id) {
            rouven.driver_state = Some(CharacterDriverState::Rouven(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`staffer.c:903-905`).
        if let (Some(rouven), Some((tx, ty))) =
            (self.characters.get(&rouven_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(rouven.x), i32::from(rouven.y), tx, ty) {
                if let Some(rouven_mut) = self.characters.get_mut(&rouven_id) {
                    let _ = turn(rouven_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_LEFT, ret,
        // lastact)) return; }` (`staffer.c:907-911`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area26::smugglecom` already uses.
        let last_talk = if let Some(rouven) = self.characters.get(&rouven_id) {
            match rouven.driver_state.as_ref() {
                Some(CharacterDriverState::Rouven(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + ROUVEN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(rouven) = self.characters.get(&rouven_id) else {
                return;
            };
            let (post_x, post_y) = (rouven.rest_x, rouven.rest_y);
            self.secure_move_driver(
                rouven_id,
                post_x,
                post_y,
                Direction::Left as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `rouven_driver`'s `NT_CHAR` branch (`staffer.c:697-833`).
    #[allow(clippy::too_many_arguments)]
    fn rouven_handle_char_message(
        &mut self,
        rouven_id: CharacterId,
        data: &mut RouvenDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, RouvenPlayerFacts>,
        events: &mut Vec<RouvenOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(rouven) = self.characters.get(&rouven_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`staffer.c:700-704`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`staffer.c:706-710`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`staffer.c:712-716`).
        if tick < data.last_talk + ROUVEN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`staffer.c:718-721`).
        if tick < data.last_talk + ROUVEN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`staffer.c:723-727`).
        if rouven_id == player_id || !char_see_char(&rouven, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`staffer.c:729-733`).
        if char_dist(&rouven, &player) > ROUVEN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.rouven_state;
        match facts.rouven_state {
            // C `case 0:` (`staffer.c:739-755`).
            0 => {
                if facts.carlos2_state == 0 {
                    if !mem_check_driver(&rouven.driver_memory, 0, player_id.0) {
                        self.npc_quiet_say(
                            rouven_id,
                            &format!("Hullo {}. Please talk to Carlos first.", player.name),
                        );
                        if let Some(rouven_mut) = self.characters.get_mut(&rouven_id) {
                            mem_add_driver(&mut rouven_mut.driver_memory, 0, player_id.0);
                        }
                    }
                } else {
                    self.npc_quiet_say(
                        rouven_id,
                        &format!(
                            "Hail, {}. Carlos sent you for a ritual? Did he mention the place is cursed? Well, I have two quests of my own for you.",
                            player.name
                        ),
                    );
                    new_state = 1;
                    events.push(RouvenOutcomeEvent::QuestOpen {
                        player_id,
                        quest: QLOG_ROUVEN_TUNNEL,
                    });
                    didsay = true;
                }
            }
            // C `case 1:` (`staffer.c:756-761`).
            1 => {
                self.npc_quiet_say(
                    rouven_id,
                    "First, I beg you to try to locate the source of the curse that has befallen the Imperial Vault.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`staffer.c:762-767`).
            2 => {
                self.npc_quiet_say(
                    rouven_id,
                    "It was in the armory where the guards first started acting strangely and attacking everyone.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`staffer.c:768-772`).
            3 => {
                self.npc_quiet_say(rouven_id, "The way there is through the left door.");
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`staffer.c:773-777`).
            4 => {
                self.npc_quiet_say(rouven_id, "Good luck.");
                new_state = 5;
                didsay = true;
            }
            // C `case 5: break;` (`staffer.c:778-779`): waiting for the
            // player to find the skull (`vault_skull`, already ported).
            5 => {}
            // C `case 6:` (`staffer.c:781-787`).
            6 => {
                self.npc_quiet_say(
                    rouven_id,
                    "You say there's demons and a pile of strange skulls? They must have burrowed in from the underground. We'll look into this immediately.",
                );
                new_state = 7;
                events.push(RouvenOutcomeEvent::QuestOpen {
                    player_id,
                    quest: QLOG_ROUVEN_CHRONICLES,
                });
                didsay = true;
            }
            // C `case 7:` (`staffer.c:788-793`).
            7 => {
                self.npc_quiet_say(
                    rouven_id,
                    "Now I ask you to retrieve the chronicles of Seyan I. He kept a journal detailing many of his plans. Including those for the Aston Empire.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8:` (`staffer.c:794-798`).
            8 => {
                self.npc_quiet_say(rouven_id, "It was stored in the archives to the right.");
                new_state = 9;
                didsay = true;
            }
            // C `case 9: break;` (`staffer.c:799-800`): waiting for the
            // player to hand in the chronicles (`NT_GIVE`).
            9 => {}
            // C `case 10:` (`staffer.c:802-806`).
            10 => {
                self.npc_quiet_say(
                    rouven_id,
                    &format!(
                        "Thank you {}. It will be most useful in our rebuilding efforts.",
                        player.name
                    ),
                );
                new_state = 11;
                didsay = true;
            }
            // C `case 11:` (`staffer.c:807-812`).
            11 => {
                self.npc_quiet_say(
                    rouven_id,
                    "Now we get to the magical ritual. It was kept within the treasury behind me in the emperor's personal vault.",
                );
                new_state = 12;
                didsay = true;
            }
            // C `case 12:` (`staffer.c:813-822`).
            12 => {
                self.npc_quiet_say(
                    rouven_id,
                    "Take this key and when you find the scroll return to Carlos.",
                );
                if !self.character_has_item_template(player_id, IID_MAX_VAULTKEY) {
                    events.push(RouvenOutcomeEvent::GrantVaultKey { player_id });
                }
                new_state = 13;
                didsay = true;
            }
            // C `case 13: break;` (`staffer.c:823-824`): quest chain done.
            13 => {}
            _ => {}
        }

        if new_state != facts.rouven_state {
            events.push(RouvenOutcomeEvent::UpdateRouvenState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; notify_area(...); }`
        // (`staffer.c:826-831`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `rouven_driver`'s `NT_TEXT` branch (`staffer.c:836-862`), wired
    /// through the generic `analyse_text_qa` matcher (same pattern as
    /// `world::npc::area26::smugglecom`'s text handler). This branch has
    /// no victim-staleness-reset preamble and no victim-mismatch early-out
    /// (see the module doc comment).
    #[allow(clippy::too_many_arguments)]
    fn rouven_handle_text_message(
        &mut self,
        rouven_id: CharacterId,
        rouven_name: &str,
        data: &mut RouvenDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, RouvenPlayerFacts>,
        events: &mut Vec<RouvenOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`staffer.c:839`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`staffer.c:121-
        // 135`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if rouven_id == speaker_id {
            return;
        }
        let Some(rouven) = self.characters.get(&rouven_id).cloned() else {
            return;
        };
        if char_dist(&rouven, &speaker) > ROUVEN_QA_DISTANCE
            || !char_see_char(&rouven, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let rouven_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.rouven_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, rouven_name, &speaker.name, AREA26_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(rouven_id, &reply);
                didsay = true;
            }
            // C `case 2:` (repeat/restart), the three mutually exclusive
            // range resets (`staffer.c:842-854`; see the module doc
            // comment for why an `else`-less three-way `if` chain is safe
            // to port as a plain `match`).
            TextAnalysisOutcome::Matched(2) => {
                data.last_talk = 0;
                let new_state = match rouven_state {
                    0..=5 => Some(0),
                    6..=9 => Some(6),
                    10..=13 => Some(10),
                    _ => None,
                };
                if let Some(new_state) = new_state {
                    events.push(RouvenOutcomeEvent::UpdateRouvenState {
                        player_id: speaker_id,
                        new_state,
                    });
                }
                didsay = true;
            }
            // Every other matched code (including `3`, "reset me" -
            // `rouven_driver` has no `case 3` of its own, see the module
            // doc comment) is unhandled by rouven's own C `switch` but
            // still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`staffer.c:857-861`) - note this does *not* touch
        // `dat->last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `rouven_driver`'s `NT_GIVE` branch (`staffer.c:864-895`).
    fn rouven_handle_give_message(
        &mut self,
        rouven_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, RouvenPlayerFacts>,
        events: &mut Vec<RouvenOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&rouven_id)
            .and_then(|rouven| rouven.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let is_player = giver.flags.contains(CharacterFlags::PLAYER);
        let facts = player_facts.get(&giver_id).copied();

        // C `if (it[in].ID == IID_MAX_CHRONICLES && ppd &&
        // ppd->rouven_state >= 6 && ppd->rouven_state <= 9 && (ch[co].
        // flags & CF_PLAYER))` (`staffer.c:872-877`).
        if item.template_id == IID_MAX_CHRONICLES
            && is_player
            && facts.is_some_and(|facts| (6..=9).contains(&facts.rouven_state))
        {
            self.npc_quiet_say(
                rouven_id,
                &format!("Thank you for the book, {}.", giver.name),
            );
            events.push(RouvenOutcomeEvent::QuestDone {
                player_id: giver_id,
            });
            self.destroy_item(item_id);
            self.destroy_items_by_template_id(giver_id, IID_MAX_CHRONICLES);
            events.push(RouvenOutcomeEvent::UpdateRouvenState {
                player_id: giver_id,
                new_state: 10,
            });
            return;
        }

        // C's fallback `else` branch (`staffer.c:878-887`): either point
        // the player back to Carlos for the ritual scroll, or hand any
        // other item straight back.
        if item.template_id == IID_MAX_RITUAL {
            self.npc_quiet_say(rouven_id, "Please take the ritual to Carlos.");
        } else {
            self.npc_quiet_say(
                rouven_id,
                "Thou hast better use for this than I do. Well, if there is use for it at all.",
            );
        }
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_LOSTCON, CDR_ROUVEN};
use crate::item_driver::{IID_MAX_CHRONICLES, IID_MAX_RITUAL, IID_MAX_VAULTKEY};

/// C `struct rouven_data` (`src/area/26/staffer.c:676-679`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RouvenDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
