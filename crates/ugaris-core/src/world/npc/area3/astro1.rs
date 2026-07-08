//! Astro1 ambient rambling astronomer NPC (`CDR_ASTRO1`).
//!
//! Ports `src/area/3/area3.c::astro1_driver` (`:1388-1486`): the simplest
//! driver in `area3.c` - it has no dialogue interaction, no quest, and no
//! item logic at all. Every incoming message is drained unconditionally
//! (C's `for (msg = ch[cn].msg; msg; msg = next) { next = msg->next;
//! remove_message(cn, msg); }` never inspects a single message field), and
//! every ten ticks it auto-advances a fifteen-state monologue about
//! watching the moon through a telescope, said via `quiet_say` regardless
//! of whether anyone is listening. State 14 (and any out-of-range state)
//! wraps back to state 0. There is no death hook of its own - `ch_died_
//! driver`'s `CDR_ASTRO1` case (`area3.c:2884-2886`, alongside `CDR_
//! SEYMOUR`/`CDR_KELLY`/`CDR_ASTRO2`/`CDR_THOMAS`/`CDR_SIRJONES`/`CDR_
//! CARLOS`/`CDR_SUPERMAX`/`CDR_KASSIM`) dispatches to the shared `immortal_
//! dead` no-op debug log (`area3.c:2596-2598`), ported as
//! `crate::world_events::death_hooks::apply_area3_immortal_death_from_hurt_event`
//! in `ugaris-server` (currently only listing `CDR_ASTRO1` since it is the
//! only one of that group ported so far; extend the array as the sibling
//! NPCs are ported).
//!
//! Deviations/gaps (documented, not silent):
//! - No self-defense/regen/spell-self cascade exists in C's `astro1_
//!   driver` body at all (unlike every other simple grunt NPC in this
//!   codebase) - this port omits it too, matching the C source exactly.
//! - `secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
//!   lastact)` (`area3.c:1481`): the NPC's post position (C's `tmpx`/
//!   `tmpy`) reuses `rest_x`/`rest_y`, the same substitution every other
//!   stationary NPC in this codebase already uses; `ret`/`lastact` are
//!   passed as `0`/`0`, the same precedent established for every sibling
//!   NPC whose `process_*_actions` entry point has no equivalent of C's
//!   driver-call return code plumbed through.

use crate::world::*;

impl World {
    /// Ports the per-tick dispatch loop over all live `CDR_ASTRO1`
    /// characters (C `ch_driver`'s `CDR_ASTRO1` case, `area3.c:2892-2894`).
    pub fn process_astro1_actions(&mut self, area_id: u16) -> usize {
        let astro1_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ASTRO1
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut acted = 0;
        for astro1_id in astro1_ids {
            if self.process_astro1_tick(astro1_id, area_id) {
                acted += 1;
            }
        }
        acted
    }

    /// C `astro1_driver`'s per-tick body (`area3.c:1388-1486`).
    fn process_astro1_tick(&mut self, astro1_id: CharacterId, area_id: u16) -> bool {
        // C's message loop unconditionally drains every message without
        // inspecting any of them (`area3.c:1397-1402`).
        if let Some(character) = self.characters.get_mut(&astro1_id) {
            character.driver_messages.clear();
        }

        let mut data = match self
            .characters
            .get(&astro1_id)
            .and_then(|character| character.driver_state.clone())
        {
            Some(CharacterDriverState::Astro1(data)) => data,
            _ => Astro1DriverData::default(),
        };

        // C `if (ticker > dat->last_talk + TICKS * 10) { switch
        // (dat->state) { ... } dat->last_talk = ticker; }`
        // (`area3.c:1406-1479`).
        if self.tick.0 > data.last_talk + TICKS_PER_SECOND * 10 {
            let line = ASTRO1_MONOLOGUE.get(data.state as usize).copied();
            match line {
                Some(text) => {
                    self.npc_quiet_say(astro1_id, text);
                    data.state = if data.state as usize + 1 >= ASTRO1_MONOLOGUE.len() {
                        0
                    } else {
                        data.state + 1
                    };
                }
                None => data.state = 0,
            }
            data.last_talk = self.tick.0;
        }

        if let Some(character) = self.characters.get_mut(&astro1_id) {
            character.driver_state = Some(CharacterDriverState::Astro1(data));
        }

        // C `if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN,
        // ret, lastact)) return;` (`area3.c:1481-1483`): return to the
        // post position (`rest_x`/`rest_y` substitution - see module doc).
        let (post_x, post_y) = self
            .characters
            .get(&astro1_id)
            .map(|character| (character.rest_x, character.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(
            astro1_id,
            post_x,
            post_y,
            Direction::Down as u8,
            0,
            0,
            area_id,
        ) {
            return true;
        }

        // C `do_idle(cn, TICKS);` (`area3.c:1485`).
        self.idle_simple_baddy(astro1_id)
    }
}

/// C's fifteen-state moon-telescope monologue (`area3.c:1409-1477`),
/// verbatim - each entry is one `quiet_say(cn, "...")` call's text, in
/// state order (state 14 wraps back to state 0).
const ASTRO1_MONOLOGUE: &[&str] = &[
    "The moon, oh so bright and splendid it seemed. Oh, yes.",
    "From my starting point, I moved the telescope to the south-east.",
    "Then I noticed two triangle-shaped boulders. I continued moving south-east.",
    "Some time later, there were one triangle-shaped boulder and one round like a circle. There, I started moving the telescope south-west.",
    "A triangle-shaped boulder there was, and a square one. I turned the telescope south-east again.",
    "Soon after, the perfectly round boulders caught my eye. Perfect circles. I marvelled at their sight - and got hungry, so I stopped to get some food.",
    "After I got the food, I went back to the triangle-shaped and the square boulder. This time, I moved the telescope south-west.",
    "There, I saw two boulders, both perfectly round, like circles. I continued moving south-west.",
    "I moved the telescope past a triangle-shaped boulder, still going south-west.",
    "At the sight of a round boulder, I started moving the telescope south-east.",
    "After looking some more, I spotted the perfect squares. From there on, I moved north-east.",
    "Some time later, I noticed another square and turned south-east again.",
    "But then I got interrupted by my colleague, who wanted to have a heated discussion about something I don't remember. I offered him some food to quiet him and continued my observations.",
    "And there it was, the very thing I was looking for. Oh, it was so beautiful!",
    "Now what did I say last? I better start over at the beginning!",
];

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::CDR_ASTRO1;

/// C `struct astro1_driver_data` (`src/area/3/area3.c:1383-1386`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Astro1DriverData {
    pub last_talk: u64,
    pub state: i32,
}
