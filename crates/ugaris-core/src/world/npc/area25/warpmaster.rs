//! Warpmaster NPC (`CDR_WARPMASTER`), the Warped World's key-for-stone
//! trader.
//!
//! Ports `src/area/25/warped.c::warpmaster` (`:993-1160`) plus the shared
//! `analyse_text_driver` (`:136-232`) via the generic
//! [`crate::character_driver::analyse_text_qa`] helper and [`super::
//! AREA25_QA`].
//!
//! Deviations/gaps (documented, not silent):
//! - C's `NT_CHAR` branch (`:1003-1033`) has no `CF_PLAYER` guard at all
//!   (unlike every other area's greeter NPC) - preserved verbatim, so this
//!   driver will in principle greet any visible, unmet (`mem_check_driver`)
//!   character within 10 tiles, not just players.
//! - `analyse_text_driver`'s own distance check is commented out in this
//!   file's copy (`// if (char_dist(cn,co)>16) return 0;`, `:155`) -
//!   preserved as "no distance limit", unlike the area-3 copy of the same
//!   function (which has a live 12-tile check).
//! - `ppd->supermax_gold`-style player-runtime mutations (the `warped_ppd`
//!   reset on "reset", and the `warped_door_key` item creation on the
//!   stone-for-keys trade) need `PlayerRuntime`/`ZoneLoader`, neither of
//!   which `World` can see - both are reported via
//!   [`WarpmasterOutcomeEvent`] for `ugaris-server` to apply
//!   (`crates/ugaris-server/src/area25.rs`), the same split every other
//!   area-N driver in this codebase already uses.

use crate::character_driver::{
    analyse_text_qa, mem_add_driver, mem_check_driver, mem_erase_driver, TextAnalysisOutcome,
};
use crate::item_driver::drdata;
use crate::item_driver::IID_ALCHEMY_INGREDIENT;
use crate::world::*;

use super::AREA25_QA;

/// C `char_dist(cn, co) > 10` (`warped.c:1013`).
const WARPMASTER_GREET_DISTANCE: i32 = 10;
/// C `ticker % 345600 == 0` (`warped.c:1155`): the periodic
/// `mem_erase_driver` cadence (48 legacy in-game hours' worth of ticks).
const WARPMASTER_MEM_ERASE_INTERVAL: u64 = 345_600;

/// A side effect [`World::process_warpmaster_actions`] could not apply
/// directly because it touches `PlayerRuntime` (the `warped_ppd`
/// reset) or needs `ZoneLoader` (creating `warped_door_key` items).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpmasterOutcomeEvent {
    /// C `warpmaster`'s `NT_TEXT` "reset" branch (`warped.c:1045-1052`):
    /// `ppd->points = 0; for (n..MAXWARPBONUS) ppd->bonuslast_used[n] = 0;
    /// ppd->nostepexp = 1;`.
    ResetWarpPpd { player_id: CharacterId },
    /// C `warpmaster`'s `NT_GIVE` alchemy-stone trade
    /// (`warped.c:1061-1133`): create `count` `warped_door_key` items and
    /// give them to `player_id`, destroying `ingredient_item_id` once at
    /// least one key was successfully created and given (C's `flag`
    /// tracking).
    GiveKeys {
        warpmaster_id: CharacterId,
        player_id: CharacterId,
        ingredient_item_id: ItemId,
        count: u8,
    },
}

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_WARPMASTER`
    /// characters (C `ch_driver`'s `CDR_WARPMASTER` case,
    /// `warped.c:1167-1169`).
    pub fn process_warpmaster_actions(&mut self, area_id: u16) -> Vec<WarpmasterOutcomeEvent> {
        let warpmaster_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_WARPMASTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for warpmaster_id in warpmaster_ids {
            self.process_warpmaster_tick(warpmaster_id, area_id, &mut events);
        }
        events
    }

    /// C `warpmaster`'s per-tick body (`warped.c:993-1160`).
    fn process_warpmaster_tick(
        &mut self,
        warpmaster_id: CharacterId,
        area_id: u16,
        events: &mut Vec<WarpmasterOutcomeEvent>,
    ) {
        let messages = self
            .characters
            .get_mut(&warpmaster_id)
            .map(|warpmaster| std::mem::take(&mut warpmaster.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_CHAR if message.dat1 > 0 => {
                    self.warpmaster_handle_char_message(warpmaster_id, message)
                }
                NT_TEXT => self.warpmaster_handle_text_message(warpmaster_id, message, events),
                NT_GIVE => self.warpmaster_handle_give_message(warpmaster_id, message, events),
                _ => {}
            }
        }

        // C `if (spell_self_driver(cn)) return;` (`warped.c:1147-1149`).
        if self.spell_self_simple_baddy(warpmaster_id) {
            return;
        }

        // C `if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT,
        // ret, lastact)) return;` (`warped.c:1151-1153`): the NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase uses.
        let (post_x, post_y) = self
            .characters
            .get(&warpmaster_id)
            .map(|warpmaster| (warpmaster.rest_x, warpmaster.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            warpmaster_id,
            post_x,
            post_y,
            Direction::Right as u8,
            0,
            0,
            area_id,
        ) {
            return;
        }

        // C `if (ticker % 345600 == 0) mem_erase_driver(cn, 7);`
        // (`warped.c:1155-1157`).
        if self.tick.0 % WARPMASTER_MEM_ERASE_INTERVAL == 0 {
            if let Some(warpmaster) = self.characters.get_mut(&warpmaster_id) {
                mem_erase_driver(&mut warpmaster.driver_memory, 7);
            }
        }

        // C `do_idle(cn, TICKS);` (`warped.c:1159`).
        self.idle_simple_baddy(warpmaster_id);
    }

    /// C `warpmaster`'s `NT_CHAR` branch (`warped.c:1003-1033`). No
    /// `CF_PLAYER` guard exists in C here - see module doc comment.
    fn warpmaster_handle_char_message(
        &mut self,
        warpmaster_id: CharacterId,
        message: &crate::character_driver::CharacterDriverMessage,
    ) {
        let seen_id = CharacterId(message.dat1.max(0) as u32);
        let Some(warpmaster) = self.characters.get(&warpmaster_id).cloned() else {
            return;
        };
        let Some(seen) = self.characters.get(&seen_id).cloned() else {
            return;
        };

        // C `if (!char_see_char(cn, co) || cn == co)` (`warped.c:1007`).
        if warpmaster_id == seen_id
            || !char_see_char(&warpmaster, &seen, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`warped.c:1013`).
        if char_dist(&warpmaster, &seen) > WARPMASTER_GREET_DISTANCE {
            return;
        }
        // C `if (mem_check_driver(cn, co, 7))` (`warped.c:1019`).
        if mem_check_driver(&warpmaster.driver_memory, 7, seen_id.0) {
            return;
        }

        if seen.level < 30 {
            self.npc_say(
                warpmaster_id,
                &format!(
                    "Hello {}! You'd better leave this area - it is too dangerous for you.",
                    seen.name
                ),
            );
        } else {
            self.npc_say(
                warpmaster_id,
                &format!(
                    "Hello {}! Welcome to Rodney's \u{E0C4}Warped World\u{E0C0}! Would you like \
                     to buy some \u{E0C4}keys\u{E0C0}?",
                    seen.name
                ),
            );
        }

        if let Some(warpmaster) = self.characters.get_mut(&warpmaster_id) {
            mem_add_driver(&mut warpmaster.driver_memory, 7, seen_id.0);
        }
    }

    /// C `warpmaster`'s `NT_TEXT` branch (`warped.c:1036-1053`) plus the
    /// shared `analyse_text_driver` (`:136-232`, ported as
    /// [`analyse_text_qa`]).
    fn warpmaster_handle_text_message(
        &mut self,
        warpmaster_id: CharacterId,
        message: &crate::character_driver::CharacterDriverMessage,
        events: &mut Vec<WarpmasterOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(warpmaster) = self.characters.get(&warpmaster_id).cloned() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`warped.c:1039`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // `analyse_text_driver`'s own guards: ignore our own talk, and
        // require visibility (`warped.c:146-157`); its distance check is
        // commented out in this file's copy - see module doc comment.
        if warpmaster_id == speaker_id
            || !char_see_char(&warpmaster, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        match analyse_text_qa(text, &warpmaster.name, &speaker.name, AREA25_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(warpmaster_id, &reply);
            }
            // C `if (code == 2 && (ppd = set_data(...)))` (`warped.c:1045`):
            // `set_data` on a player character always succeeds, so this is
            // unconditional once the "reset" qa row matched.
            TextAnalysisOutcome::Matched(2) => {
                self.npc_say(warpmaster_id, "Done.");
                events.push(WarpmasterOutcomeEvent::ResetWarpPpd {
                    player_id: speaker_id,
                });
            }
            TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
        }
    }

    /// C `warpmaster`'s `NT_GIVE` branch (`warped.c:1056-1140`).
    fn warpmaster_handle_give_message(
        &mut self,
        warpmaster_id: CharacterId,
        message: &crate::character_driver::CharacterDriverMessage,
        events: &mut Vec<WarpmasterOutcomeEvent>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get(&warpmaster_id)
            .and_then(|warpmaster| warpmaster.cursor_item)
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };

        // C's four `type == 21/22/23/24` alchemy-stone branches
        // (`warped.c:1061-1133`): the message text/key count pairing is a
        // literal, non-monotonic C table (type 23 -> one key, 21 -> two,
        // 22 -> three, 24 -> four) - preserved digit-for-digit, not
        // "fixed" into a formula.
        let key_trade = if item.template_id == IID_ALCHEMY_INGREDIENT {
            match drdata(&item, 0) {
                23 => Some((1u8, "one key")),
                21 => Some((2u8, "two keys")),
                22 => Some((3u8, "three keys")),
                24 => Some((4u8, "four keys")),
                _ => None,
            }
        } else {
            None
        };

        if let Some(warpmaster) = self.characters.get_mut(&warpmaster_id) {
            warpmaster.cursor_item = None;
        }

        if let Some((count, phrase)) = key_trade {
            self.npc_say(warpmaster_id, &format!("Here you go, {phrase}."));
            events.push(WarpmasterOutcomeEvent::GiveKeys {
                warpmaster_id,
                player_id,
                ingredient_item_id: item_id,
                count,
            });
            return;
        }

        // C `if (flag || !give_char_item(co, in)) destroy_item(ch[cn].citem);`
        // with `flag == 0` here (`warped.c:1135-1137`): hand the item back
        // to the giver, destroying it only if that fails.
        if !self.give_char_item(player_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

/// C `struct warpmaster` has no persistent driver data beyond the shared
/// `DRD_FIGHTDRIVER`-independent `mem_*` slot (`character.driver_memory`,
/// generic across every driver) - this marker type exists only for
/// [`crate::character_driver::CharacterDriverState`]'s "every NPC gets one
/// file, one driver-state variant" convention.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WarpmasterDriverData;

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_WARPMASTER;
