//! Arkhatamonk NPC (`CDR_ARKHATAMONK`), the four monk personas sharing one
//! dialogue state machine in the Arkhata library (Gregor/Johan/Johnatan/
//! Tracy, identified by name like C's own `nr` computation).
//!
//! Ports `src/area/37/arkhata.c::arkhatamonk_driver` (`:1648-2081`) plus
//! the shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`]). Follows the same `World`/`PlayerRuntime` split
//! established by `world::npc::area37::ramin`: the caller supplies a
//! per-player fact snapshot ([`ArkhatamonkPlayerFacts`]) up front and
//! applies the returned [`ArkhatamonkOutcomeEvent`]s afterwards, since
//! `arkhata_ppd.monk_state`/`monk_bits` live on `crate::player::
//! PlayerRuntime`, not `World`.
//!
//! `arkhatamonk_driver`'s thirty-one-state (`0`-`30`) dialogue chain is
//! gated by which of the four monks (`nr` 1-4) the player is speaking to
//! at each step - identical to C's own `if (nr == N)` guards:
//! - `0` needs `arkhata_ppd.ramin_state >= 7` (`world::npc::area37::
//!   ramin`'s own progress) to advance; C's own `case 0` falls through
//!   into `case 1`'s `nr == 1` speech/advance-to-`2` in the same tick -
//!   collapsed into one `monk_state == 0` arm here plus a
//!   `monk_state == 1` arm for both the fallthrough-landed and
//!   directly-re-entered cases, same "fallthrough lands on the next
//!   case's action" precedent as `world::npc::area37::rammy`'s own `rs ==
//!   6`/`13`/`17` arms.
//! - `6`, `8`->`9`->`10`->`11`->`12` (wait), `13`->`14` are Gregor(1)/
//!   Johnatan(3)-only states that need no cross-driver facts.
//! - `12` is a pure wait state: the player must turn in all three
//!   `IID_ARKHATA_MONKPART{1,2,3}` key-parts (this driver's own `NT_GIVE`
//!   handler) before `monk_bits == 7` advances it to `13`.
//! - `19` is a pure wait state: the still-unported `CDR_BOOKEATER`'s own
//!   death hook (`world_events::death_hooks::
//!   apply_arkhata_bookeater_death_from_hurt_event`) advances
//!   `monk_state` from `19` to `20` directly once the player has slain
//!   the huge Book Eater - this file never drives that transition itself.
//! - `28` is a pure wait state: the player must bring back "Corby" (the
//!   still-unported `clerk_driver`'s own dwarf-blacksmith side quest) -
//!   no state here reads any of `clerk_driver`'s fields directly; C's own
//!   `case 28: break; // waiting for corby` has no gating condition at
//!   all besides the player simply not yet being at `monk_state == 29`.
//!
//! Deviations/gaps (documented, not silent):
//! - Unlike `world::npc::area37::ramin`/`rammy` (which both use
//!   `quiet_say` for their own `NT_GIVE` success text), every line of
//!   dialogue in `arkhatamonk_driver` - `NT_CHAR` greetings and the
//!   `NT_GIVE` turn-in acknowledgements alike - is C `say` (`arkhata.c:
//!   1728-2043`, confirmed by grep: no `quiet_say` call exists anywhere
//!   in this function), so every line here uses [`World::npc_say`], not
//!   [`World::npc_quiet_say`].
//! - `NT_GIVE`'s three key-part branches (`monk_state == 12`) are the
//!   only `arkhata_ppd.monk_bits`/`monk_state` writes this driver
//!   performs directly for quest 69; the dictionary branch
//!   (`monk_state == 28`, quest 78's `IID_ARKHATA_DICTIONARY` turn-in)
//!   grants `give_exp(co, 15000)` plus a `log_char` line and advances to
//!   `29`, but quest 78's own `questlog_done(co, 78)` fires elsewhere
//!   (the still-unported `kidnappee_driver`, `arkhata.c:4269`) - a
//!   documented gap, same shape as `world::npc::area37::ramin`'s own
//!   quest-68-completion-lives-elsewhere precedent (there,
//!   `arkhataskelly_dead`; here, `kidnappee_driver`).
//! - `IID_ARKHATA_DICTIONARY` collides byte-for-byte with
//!   `item_driver::IID_ARKHATA_AKEY1` in C itself (`src/common/
//!   item_id.h:265` vs `:267`, both `MAKE_ITEMID(DEV_ID_DB, 0x0000CA)`) -
//!   a genuine pre-existing C item-id bug, not a porting mistake.
//!   Reproduced verbatim as a direct alias so any future `IID_ARKHATA_
//!   AKEY1` comparison against a dictionary item (or vice versa) matches
//!   exactly like C's own `it[in].ID ==` checks would.
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `arkhatamonk_driver` body at all (matching the `rammy`/`jaz`/`ramin`
//!   "pure talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:2080`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::{
    IID_ARKHATA_DICTIONARY, IID_ARKHATA_MONKPART1, IID_ARKHATA_MONKPART2, IID_ARKHATA_MONKPART3,
};
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:1710`, sibling drivers' own
/// identical guard).
const MONK_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const MONK_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:1693`).
const MONK_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:1698`).
const MONK_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:2062`): idle "return to post" threshold.
const MONK_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C quest 69, opened by Johnatan (`nr == 3`) at `monk_state` `8`.
const QLOG_MONK_KEYPARTS: usize = 69;
/// C quest 70, opened by Tracy (`nr == 4`) at `monk_state` `15`; completed
/// by the still-unported `CDR_BOOKEATER`'s own death hook, not here.
const QLOG_MONK_BOOKEATER: usize = 70;
/// C quest 78, opened by Johnatan (`nr == 3`) at `monk_state` `21`;
/// completed by the still-unported `kidnappee_driver` (`arkhata.c:4269`),
/// not here.
const QLOG_MONK_DICTIONARY: usize = 78;
/// C `ppd->monk_bits |= 1/2/4` (`arkhata.c:1995,2007,2020`); `== 7` means
/// all three key-parts were turned in.
const MONK_BIT_GREGOR: i32 = 1;
const MONK_BIT_JOHAN: i32 = 2;
const MONK_BIT_JOHNATAN: i32 = 4;
const MONK_BITS_ALL: i32 = MONK_BIT_GREGOR | MONK_BIT_JOHAN | MONK_BIT_JOHNATAN;
/// C `give_money(co, 200 * 100, "solved Tracy's quest")` (`arkhata.c:1870`).
const MONK_TRACY_REWARD_GOLD: u32 = 200 * 100;
/// C `give_money(co, 3000 * 100, "Monk Dictionary Quest")`
/// (`arkhata.c:1936`).
const MONK_DICTIONARY_REWARD_GOLD: u32 = 3000 * 100;
/// C `give_exp(co, 15000)` (`arkhata.c:2035`).
const MONK_DICTIONARY_REWARD_EXP: i64 = 15000;

/// Which of the four monk personas a live `CDR_ARKHATAMONK` character is,
/// resolved by name (C `if (!strcmp(ch[cn].name, "Gregor"))` etc.,
/// `arkhata.c:1654-1665`).
fn monk_persona_nr(name: &str) -> i32 {
    match name {
        "Gregor" => 1,
        "Johan" => 2,
        "Johnatan" => 3,
        "Tracy" => 4,
        _ => 0,
    }
}

/// Per-player facts [`World::process_arkhatamonk_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArkhatamonkPlayerFacts {
    /// `PlayerRuntime::arkhata_monk_state()`.
    pub monk_state: i32,
    /// `PlayerRuntime::arkhata_monk_bits()`.
    pub monk_bits: i32,
    /// `PlayerRuntime::arkhata_ramin_state()` (`ppd->ramin_state`,
    /// `arkhata.c:1721`): gates `monk_state` `0`.
    pub ramin_state: i32,
}

/// A side effect [`World::process_arkhatamonk_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArkhatamonkOutcomeEvent {
    /// Write the new `arkhata_ppd.monk_state` back.
    UpdateMonkState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->monk_bits |= 1/2/4` (`arkhata.c:1995,2007,2020`).
    UpdateMonkBits {
        player_id: CharacterId,
        new_bits: i32,
    },
    /// C `questlog_open(co, 69)` (`arkhata.c:1784`).
    QuestOpen69 { player_id: CharacterId },
    /// C `questlog_done(co, 69)` (`arkhata.c:1998,2011,2023`).
    QuestDone69 { player_id: CharacterId },
    /// C `questlog_open(co, 70)` (`arkhata.c:1835`).
    QuestOpen70 { player_id: CharacterId },
    /// C `questlog_open(co, 78)` (`arkhata.c:1879`).
    QuestOpen78 { player_id: CharacterId },
}

impl World {
    /// C `arkhatamonk_driver`'s per-tick body (`arkhata.c:1648-2081`).
    pub fn process_arkhatamonk_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ArkhatamonkPlayerFacts>,
        area_id: u16,
    ) -> Vec<ArkhatamonkOutcomeEvent> {
        let monk_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ARKHATAMONK
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for monk_id in monk_ids {
            self.process_arkhatamonk_messages(monk_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_arkhatamonk_messages(
        &mut self,
        monk_id: CharacterId,
        player_facts: &HashMap<CharacterId, ArkhatamonkPlayerFacts>,
        area_id: u16,
        events: &mut Vec<ArkhatamonkOutcomeEvent>,
    ) {
        let Some(monk_name) = self.characters.get(&monk_id).map(|monk| monk.name.clone()) else {
            return;
        };
        let nr = monk_persona_nr(&monk_name);
        let Some(CharacterDriverState::Arkhatamonk(mut data)) = self
            .characters
            .get(&monk_id)
            .and_then(|monk| monk.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&monk_id)
            .map(|monk| std::mem::take(&mut monk.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.monk_handle_char_message(
                    monk_id,
                    nr,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.monk_handle_text_message(
                    monk_id,
                    &monk_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.monk_handle_give_message(monk_id, nr, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(monk) = self.characters.get_mut(&monk_id) {
            monk.driver_state = Some(CharacterDriverState::Arkhatamonk(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:2058-2060`).
        if let (Some(monk), Some((tx, ty))) = (self.characters.get(&monk_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(monk.x), i32::from(monk.y), tx, ty) {
                if let Some(monk_mut) = self.characters.get_mut(&monk_id) {
                    let _ = turn(monk_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if (nr==1||nr==3)
        // secure_move_driver(..., DX_DOWN, ...); if (nr==2)
        // secure_move_driver(..., DX_UP, ...); if (nr==4)
        // secure_move_driver(..., DX_RIGHT, ...); }` (`arkhata.c:2062-
        // 2078`). The NPC's post position (C's `tmpx`/`tmpy`) reuses
        // `rest_x`/`rest_y`, the same substitution every other stationary
        // NPC in this codebase makes.
        let last_talk = if let Some(monk) = self.characters.get(&monk_id) {
            match monk.driver_state.as_ref() {
                Some(CharacterDriverState::Arkhatamonk(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + MONK_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(monk) = self.characters.get(&monk_id) else {
                return;
            };
            let (post_x, post_y) = (monk.rest_x, monk.rest_y);
            let direction = if nr == 2 {
                Direction::Up
            } else if nr == 4 {
                Direction::Right
            } else {
                Direction::Down
            };
            self.secure_move_driver(monk_id, post_x, post_y, direction as u8, 0, 0, area_id);
        }
    }

    /// C `arkhatamonk_driver`'s `NT_CHAR` branch (`arkhata.c:1676-1950`).
    #[allow(clippy::too_many_arguments)]
    fn monk_handle_char_message(
        &mut self,
        monk_id: CharacterId,
        nr: i32,
        data: &mut ArkhatamonkDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ArkhatamonkPlayerFacts>,
        events: &mut Vec<ArkhatamonkOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(monk) = self.characters.get(&monk_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:1681`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:1687`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:1693`).
        if tick < data.last_talk + MONK_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:1698`).
        if tick < data.last_talk + MONK_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:1704`).
        if monk_id == player_id || !char_see_char(&monk, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:1710`).
        if char_dist(&monk, &player) > MONK_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.monk_state;
        match facts.monk_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:
            // 1720-1734`) - see the module doc comment.
            0 if facts.ramin_state >= 7 => {
                if nr == 1 {
                    self.npc_say(
                        monk_id,
                        "Dried leaves of lavender, and rose buttons, cinnamon powder and three drops eucalyptus extract is all you need for a fragrance so refreshing it will open thy air channels even when cough by the worst cold.",
                    );
                    new_state = 2;
                    didsay = true;
                } else {
                    new_state = 1;
                }
            }
            0 => {}
            // C `case 1:` (`arkhata.c:1735-1742`), also reached directly
            // when a prior tick's fallthrough left `monk_state == 1`
            // without a persona-1 speaker present.
            1 => {
                if nr == 1 {
                    self.npc_say(
                        monk_id,
                        "Sleep on a pillow filled with acorns of wheat and drink a glass of milk before going to bed for a good nights sleep with calm dreams.",
                    );
                    new_state = 2;
                    didsay = true;
                }
            }
            // C `case 2:` (`arkhata.c:1743-1750`).
            2 => {
                if nr == 1 {
                    self.npc_say(
                        monk_id,
                        "Twice a week have a bath in water with 1 teaspoon of coconut oil and scrub your skin with a salt water sponge until it turns red to maintain a soft and youthful skin.",
                    );
                    new_state = 3;
                    didsay = true;
                }
            }
            // C `case 3:` (`arkhata.c:1751-1757`).
            3 => {
                if nr == 1 {
                    self.npc_say(
                        monk_id,
                        "Avoid garlic before proposing to your loved one...",
                    );
                    new_state = 4;
                    didsay = true;
                }
            }
            // C `case 4:` (`arkhata.c:1758-1764`).
            4 => {
                if nr == 1 {
                    self.npc_emote(monk_id, "pauses");
                    new_state = 5;
                    didsay = true;
                }
            }
            // C `case 5:` (`arkhata.c:1765-1771`).
            5 => {
                if nr == 1 {
                    self.npc_say(monk_id, "Now that I should have read before.");
                    new_state = 6;
                    didsay = true;
                }
            }
            // C `case 6:` (`arkhata.c:1772-1779`).
            6 => {
                if nr == 2 {
                    self.npc_say(
                        monk_id,
                        "Greetings stranger, Ramin must have sent thee. Please go speak with my brother Johnatan there in the corner. I am busy writing up the family tree of a harpy.",
                    );
                    new_state = 7;
                    didsay = true;
                }
            }
            // C `case 7:` (`arkhata.c:1780-1788`).
            7 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "Greetings! You have come just in time. We usually work here so late at night that we fall asleep by our books right here in the library.",
                    );
                    events.push(ArkhatamonkOutcomeEvent::QuestOpen69 { player_id });
                    new_state = 8;
                    didsay = true;
                }
            }
            // C `case 8:` (`arkhata.c:1789-1796`).
            8 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "Last night someone must have entered the library while we were sleeping. We woke up and the three parts of the key to that closed shelf with books were gone.",
                    );
                    new_state = 9;
                    didsay = true;
                }
            }
            // C `case 9:` (`arkhata.c:1797-1804`).
            9 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "We have always kept one part each. The books in there are old and hold many secrets, some we have not yet been able to understand.",
                    );
                    new_state = 10;
                    didsay = true;
                }
            }
            // C `case 10:` (`arkhata.c:1805-1813`).
            10 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "I believe it must have been some of those monsters who occupy the old building across the bridge and south-west of here. Please find the keyparts and return them, and we will share with you some of the wisdom that is stored in those books.",
                    );
                    new_state = 11;
                    didsay = true;
                }
            }
            // C `case 11:` (`arkhata.c:1814`).
            11 => {}
            // C `case 12: break; // waiting for keyparts` (`arkhata.c:
            // 1815`): advanced by this driver's own `NT_GIVE` handler once
            // `monk_bits == 7`.
            12 => {}
            // C `case 13:` (`arkhata.c:1816-1823`).
            13 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "Thou hast been of great aid to us, let me share with thee some of our wisdom from these books.",
                    );
                    new_state = 14;
                    didsay = true;
                }
            }
            // C `case 14:` (`arkhata.c:1824-1831`).
            14 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "I will now continue my research to uncover the secrets held within these books. Why don't thou go and talk to Tracy in the meantime?",
                    );
                    new_state = 15;
                    didsay = true;
                }
            }
            // C `case 15:` (`arkhata.c:1832-1839`).
            15 => {
                if nr == 4 {
                    self.npc_say(
                        monk_id,
                        "I see you have assisted the monks, perhaps you can help me too?",
                    );
                    events.push(ArkhatamonkOutcomeEvent::QuestOpen70 { player_id });
                    new_state = 16;
                    didsay = true;
                }
            }
            // C `case 16:` (`arkhata.c:1840-1848`).
            16 => {
                if nr == 4 {
                    self.npc_say(
                        monk_id,
                        "There is a hidden backroom down in the southern corner of the library, it seems like a huge book eater has occupied the basement floor there, and is chewing up my most precious novels.",
                    );
                    new_state = 17;
                    didsay = true;
                }
            }
            // C `case 17:` (`arkhata.c:1849-1856`).
            17 => {
                if nr == 4 {
                    self.npc_say(
                        monk_id,
                        "The  teleport system is enough to contain the small book eaters, but this huge one must be smarter as well.",
                    );
                    new_state = 18;
                    didsay = true;
                }
            }
            // C `case 18:` (`arkhata.c:1857-1864`).
            18 => {
                if nr == 4 {
                    self.npc_say(
                        monk_id,
                        "Please go down there and slay it for me, those novels are of great value to me and I can't stand the thought of losing another page! I will reward you handsomely",
                    );
                    new_state = 19;
                    didsay = true;
                }
            }
            // C `case 19: break; // waiting for player to kill the bad
            // book eater` (`arkhata.c:1865-1866`): advanced by the
            // still-unported `CDR_BOOKEATER`'s own death hook
            // (`world_events::death_hooks::
            // apply_arkhata_bookeater_death_from_hurt_event`), not this
            // file.
            19 => {}
            // C `case 20:` (`arkhata.c:1867-1874`).
            20 => {
                if nr == 4 {
                    self.npc_say(
                        monk_id,
                        "Thank you, my novels should be safe now. Here is thy reward.",
                    );
                    self.monk_give_money(monk_id, player_id, MONK_TRACY_REWARD_GOLD);
                    new_state = 21;
                    didsay = true;
                }
            }
            // C `case 21:` (`arkhata.c:1875-1883`).
            21 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "It is nice to see you again my friend. This book I'm attempting to translate is proving to be quite a challenge.",
                    );
                    events.push(ArkhatamonkOutcomeEvent::QuestOpen78 { player_id });
                    new_state = 22;
                    didsay = true;
                }
            }
            // C `case 22:` (`arkhata.c:1884-1891`).
            22 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "It was found down in the caves, and I believe it's written in the Frawd's language. It is cryptic and ancient.",
                    );
                    new_state = 23;
                    didsay = true;
                }
            }
            // C `case 23:` (`arkhata.c:1892-1899`).
            23 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "I believe they are related to the dwarfs, but the languages are less similar then deers and fire snails.",
                    );
                    new_state = 24;
                    didsay = true;
                }
            }
            // C `case 24:` (`arkhata.c:1900-1906`).
            24 => {
                if nr == 3 {
                    self.npc_say(monk_id, "I need someone related to the Frawds who can help translate this language.");
                    new_state = 25;
                    didsay = true;
                }
            }
            // C `case 25:` (`arkhata.c:1907-1914`).
            25 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "Dwarves and their descendants are the only ones who can master the blacksmith craft properly. And they also have a great love for gold.",
                    );
                    new_state = 26;
                    didsay = true;
                }
            }
            // C `case 26:` (`arkhata.c:1915-1922`).
            26 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "If you have seen anyone above ground who forges metal items or weapons for a rather ridiculous salary, he is most likely of dwarf blood.",
                    );
                    new_state = 27;
                    didsay = true;
                }
            }
            // C `case 27:` (`arkhata.c:1923-1929`).
            27 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "If you know such a person seek him out for me please.",
                    );
                    new_state = 28;
                    didsay = true;
                }
            }
            // C `case 28: break; // waiting for corby` (`arkhata.c:1930-
            // 1931`).
            28 => {}
            // C `case 29:` (`arkhata.c:1932-1940`).
            29 => {
                if nr == 3 {
                    self.npc_say(
                        monk_id,
                        "I can not cover all your expenses I'm afraid, but here is 3000g. Let me at least repay some of my debt to you.",
                    );
                    self.monk_give_money(monk_id, player_id, MONK_DICTIONARY_REWARD_GOLD);
                    new_state = 30;
                    didsay = true;
                }
            }
            // C `case 30: break; // all done` (`arkhata.c:1941-1943`).
            30 => {}
            _ => {}
        }

        if new_state != facts.monk_state {
            events.push(ArkhatamonkOutcomeEvent::UpdateMonkState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:1944-1948`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `give_money(cn, val, reason)` (`src/system/tool.c:1460-1474`):
    /// adds straight to `Character::gold`, matching `world::npc::area29::
    /// countbran`'s own `countbran_give_money` precedent - this reward
    /// path needs nothing but `World`.
    fn monk_give_money(&mut self, _monk_id: CharacterId, player_id: CharacterId, amount: u32) {
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.gold = player.gold.saturating_add(amount);
            player.flags.insert(CharacterFlags::ITEMS);
        }
        self.queue_system_text_bytes(player_id, give_money_message(amount));
    }

    /// C `arkhatamonk_driver`'s `NT_TEXT` branch (`arkhata.c:1953-1986`),
    /// wired through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn monk_handle_text_message(
        &mut self,
        monk_id: CharacterId,
        monk_name: &str,
        data: &mut ArkhatamonkDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ArkhatamonkPlayerFacts>,
        events: &mut Vec<ArkhatamonkOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:1956-1958`).
        let tick = self.tick.0;
        if tick > data.last_talk + MONK_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:1960`).
        if data.current_victim.is_some() && data.current_victim != Some(speaker_id) {
            return;
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(monk) = self.characters.get(&monk_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if monk_id == speaker_id {
            return;
        }
        if char_dist(&monk, &speaker) > MONK_QA_DISTANCE
            || !char_see_char(&monk, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let monk_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.monk_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, monk_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_say(monk_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:1965-1981`): rewind to the
            // start of whichever mini-block is in progress.
            TextAnalysisOutcome::Matched(2) => {
                if (8..=12).contains(&monk_state) {
                    data.last_talk = 0;
                    events.push(ArkhatamonkOutcomeEvent::UpdateMonkState {
                        player_id: speaker_id,
                        new_state: 8,
                    });
                }
                if (15..=19).contains(&monk_state) {
                    data.last_talk = 0;
                    events.push(ArkhatamonkOutcomeEvent::UpdateMonkState {
                        player_id: speaker_id,
                        new_state: 15,
                    });
                }
                if (21..=28).contains(&monk_state) {
                    data.last_talk = 0;
                    events.push(ArkhatamonkOutcomeEvent::UpdateMonkState {
                        player_id: speaker_id,
                        new_state: 21,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by the monks'
            // own `switch` but still counts as `didsay` (C: `switch
            // (didsay = analyse_text_driver(...))` - any nonzero return is
            // truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:1982-1985`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit resets above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `arkhatamonk_driver`'s `NT_GIVE` branch (`arkhata.c:1989-2050`).
    fn monk_handle_give_message(
        &mut self,
        monk_id: CharacterId,
        nr: i32,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ArkhatamonkPlayerFacts>,
        events: &mut Vec<ArkhatamonkOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&monk_id)
            .and_then(|monk| monk.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let facts = player_facts.get(&giver_id).copied();
        let monk_state = facts.map(|facts| facts.monk_state).unwrap_or(-1);
        let monk_bits = facts.map(|facts| facts.monk_bits).unwrap_or(0);

        // C `if (ppd && nr==1 && it[in].ID==IID_ARKHATA_MONKPART1 &&
        // ppd->monk_state==12)` (`arkhata.c:1993`).
        if nr == 1 && item.template_id == IID_ARKHATA_MONKPART1 && monk_state == 12 {
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_MONKPART1);
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_say(
                monk_id,
                &format!("Vanilla and... My key-part! I thank thee, {giver_name}."),
            );
            self.monk_give_keypart_bit(monk_id, giver_id, monk_bits, MONK_BIT_GREGOR, events);
            self.destroy_item(item_id);
            return;
        }
        // C `else if (ppd && nr==2 && it[in].ID==IID_ARKHATA_MONKPART3 &&
        // ppd->monk_state==12)` (`arkhata.c:2005`).
        if nr == 2 && item.template_id == IID_ARKHATA_MONKPART3 && monk_state == 12 {
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_MONKPART3);
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_say(
                monk_id,
                &format!(
                    "I shall remember thy herosim. Perhaps I shall write down your family tree next, {giver_name}?"
                ),
            );
            self.monk_give_keypart_bit(monk_id, giver_id, monk_bits, MONK_BIT_JOHAN, events);
            self.destroy_item(item_id);
            return;
        }
        // C `else if (ppd && nr==3 && it[in].ID==IID_ARKHATA_MONKPART2 &&
        // ppd->monk_state==12)` (`arkhata.c:2018`).
        if nr == 3 && item.template_id == IID_ARKHATA_MONKPART2 && monk_state == 12 {
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_MONKPART2);
            let giver_name = self
                .characters
                .get(&giver_id)
                .map(|giver| giver.name.clone())
                .unwrap_or_default();
            self.npc_say(
                monk_id,
                &format!("My key-part! I thank thee, {giver_name}."),
            );
            self.monk_give_keypart_bit(monk_id, giver_id, monk_bits, MONK_BIT_JOHNATAN, events);
            self.destroy_item(item_id);
            return;
        }
        // C `else if (ppd && nr==3 && it[in].ID==IID_ARKHATA_DICTIONARY &&
        // ppd->monk_state==28)` (`arkhata.c:2030`).
        if nr == 3 && item.template_id == IID_ARKHATA_DICTIONARY && monk_state == 28 {
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_DICTIONARY);
            self.npc_say(
                monk_id,
                "This is much more then I had hoped for, I'm now able to learn and translate the language in it's whole. Let us study together and share this knowledge.",
            );
            self.give_exp(
                giver_id,
                MONK_DICTIONARY_REWARD_EXP,
                u32::from(self.area_id),
            );
            self.queue_system_text(
                giver_id,
                "You learn the ancient language and gain some experience.",
            );
            events.push(ArkhatamonkOutcomeEvent::UpdateMonkState {
                player_id: giver_id,
                new_state: monk_state + 1,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:2042-2047`): hand the
        // item back to the giver.
        self.npc_say(
            monk_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }

    /// C `ppd->monk_bits |= N; if (ppd->monk_bits == 7) { questlog_done(co,
    /// 69); ppd->monk_state = 13; }` (`arkhata.c:1995-1999,2007-2012,2020-
    /// 2024`), factored out since all three key-part branches share it.
    fn monk_give_keypart_bit(
        &mut self,
        _monk_id: CharacterId,
        giver_id: CharacterId,
        monk_bits: i32,
        bit: i32,
        events: &mut Vec<ArkhatamonkOutcomeEvent>,
    ) {
        let new_bits = monk_bits | bit;
        events.push(ArkhatamonkOutcomeEvent::UpdateMonkBits {
            player_id: giver_id,
            new_bits,
        });
        if new_bits == MONK_BITS_ALL {
            events.push(ArkhatamonkOutcomeEvent::QuestDone69 {
                player_id: giver_id,
            });
            events.push(ArkhatamonkOutcomeEvent::UpdateMonkState {
                player_id: giver_id,
                new_state: 13,
            });
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_ARKHATAMONK;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `arkhatamonk_driver` itself - no field for it here, same
/// "only port fields the driver actually uses" precedent as `world::npc::
/// area37::rammy`'s `RammyDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArkhatamonkDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_MONK_KEYPARTS`] to `ugaris-server`'s
/// `apply_arkhatamonk_events`.
pub const fn qlog_monk_keyparts() -> usize {
    QLOG_MONK_KEYPARTS
}

/// Exposes [`QLOG_MONK_BOOKEATER`] to `ugaris-server`'s
/// `apply_arkhatamonk_events`.
pub const fn qlog_monk_bookeater() -> usize {
    QLOG_MONK_BOOKEATER
}

/// Exposes [`QLOG_MONK_DICTIONARY`] to `ugaris-server`'s
/// `apply_arkhatamonk_events`.
pub const fn qlog_monk_dictionary() -> usize {
    QLOG_MONK_DICTIONARY
}
