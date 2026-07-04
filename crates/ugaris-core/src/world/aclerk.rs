//! Arena clerk NPC (`CDR_ACLERK`).
//!
//! Ports the messaging/greeting/idle-chatter slice of
//! `src/module/merchants/merchant.c::aclerk_driver` - the "Cameron Arena"
//! greeter. Store creation is shared with the generic merchant driver via
//! [`World::ensure_merchant_store`]; day/night shop movement is not ported
//! yet (the same known gap documented on `world::merchant`).
//!
//! Two behaviors are intentionally *not* ported because the C source
//! itself never reaches them:
//!
//! - `aclerk_driver`'s `NT_CHAR` handler has three `quiet_say` blocks back
//!   to back, but the first ends with an unconditional
//!   `{ remove_message(cn, msg); continue; }` - the second and third
//!   blocks (an "arena is safe" message and the merchant-style "if you'd
//!   like to trade" greeting) are unreachable dead code. Only the first
//!   message (the arena welcome) is ever sent.
//! - Unlike `merchant_driver`, the `NT_TEXT` "`<name> ... trade`" handler
//!   never sets `ch[co].merchant = cn` - it only reacts to the hardcoded
//!   `abuser()` ID list with a murmur/emote. Saying "<clerk>, trade" to the
//!   arena clerk therefore never actually opens its store in C, and this
//!   port matches that (the store fields it fills via `create_store`/
//!   `add_special_store` go unused for player trading, same as C).

use super::*;
use crate::character_driver::{mem_add_driver, mem_check_driver, mem_erase_driver};
use crate::world::text::hisname;

const ACLERK_GREET_DISTANCE: i32 = 5;
/// C `TICKS * 60` in `aclerk_driver`'s idle-murmur throttle.
pub(crate) const ACLERK_TALK_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 60;
/// C `mem_add_driver(cn, co, 7)`/`mem_check_driver(cn, co, 7)`.
const ACLERK_GREET_MEMORY_SLOT: usize = 7;
const ACLERK_MEMORY_CLEAR_TICKS: u64 = TICKS_PER_SECOND * 60 * 60 * 12;

/// C `abuser(int ID)` (`src/module/merchants/merchant.c`): a hardcoded list
/// of persistent player IDs the arena clerk reacts to when they say
/// "trade". Ported digit-for-digit. Like `TraderDriverData::c1_id`/`c2_id`,
/// this checks the raw runtime `CharacterId` rather than threading the
/// legacy persistent player ID through `World` - the same simplification
/// already established there, since these values will simply never match
/// any current-run character (a purely historical anti-cheat list).
fn is_abuser(id: u32) -> bool {
    matches!(
        id,
        676 | 761
            | 3154
            | 3411
            | 6699
            | 8406
            | 10645
            | 11237
            | 11372
            | 11503
            | 12619
            | 14917
            | 16691
            | 17145
            | 21917
            | 22503
            | 28763
            | 30580
            | 34385
            | 34901
    )
}

impl World {
    /// Arena clerk NPC tick: create the store, welcome nearby players once,
    /// react to abusive trade requests, and idle-chatter. Ports the
    /// reachable core of C `aclerk_driver`.
    pub fn process_aclerk_actions(&mut self) {
        let aclerk_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_ACLERK
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for aclerk_id in aclerk_ids {
            self.ensure_merchant_store(aclerk_id);
            self.process_aclerk_messages(aclerk_id);
            self.greet_nearby_players_aclerk(aclerk_id);
            self.aclerk_idle_chatter(aclerk_id);
            self.clear_expired_aclerk_memory(aclerk_id);
        }
    }

    fn process_aclerk_messages(&mut self, aclerk_id: CharacterId) {
        let Some(aclerk) = self.characters.get_mut(&aclerk_id) else {
            return;
        };
        let aclerk_name = aclerk.name.clone();
        let messages = std::mem::take(&mut aclerk.driver_messages);
        let mut destroy_cursor = false;
        // C: `abuser(ch[co].ID)` reacting speakers who said "<name> ...
        // trade".
        let mut abuser_speakers: Vec<CharacterId> = Vec::new();

        for message in messages {
            match message.message_type {
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3 as u32);
                    if speaker_id != aclerk_id {
                        if let Some(text) = message.text.as_deref() {
                            let lower = text.to_ascii_lowercase();
                            if lower.contains(&aclerk_name.to_ascii_lowercase())
                                && lower.contains("trade")
                                && is_abuser(speaker_id.0)
                            {
                                abuser_speakers.push(speaker_id);
                            }
                        }
                    }
                }
                NT_GIVE => {
                    destroy_cursor = true;
                }
                _ => {}
            }
        }

        if destroy_cursor {
            // C: received items vanish.
            let cursor = self
                .characters
                .get_mut(&aclerk_id)
                .and_then(|aclerk| aclerk.cursor_item.take());
            if let Some(item_id) = cursor {
                self.destroy_item(item_id);
            }
        }
        for speaker_id in abuser_speakers {
            self.aclerk_abuser_reaction(aclerk_id, speaker_id);
        }
    }

    /// C `abuser(ch[co].ID)` hit branch:
    /// `switch (RANDOM(3)) { case 0: murmur; case 1: emote; case 2: murmur; }`.
    fn aclerk_abuser_reaction(&mut self, aclerk_id: CharacterId, speaker_id: CharacterId) {
        let speaker_name = self
            .characters
            .get(&speaker_id)
            .map(|speaker| speaker.name.clone())
            .unwrap_or_default();
        match legacy_random_below_from_seed(&mut self.legacy_random_seed, 3) {
            0 => {
                self.npc_murmur(aclerk_id, "I hate cheaters.");
            }
            1 => {
                self.npc_emote(
                    aclerk_id,
                    &format!("clenches his fists and stares at {speaker_name}."),
                );
            }
            2 => {
                self.npc_murmur(aclerk_id, "I wish the cheaters would leave me alone.");
            }
            _ => {}
        }
    }

    /// C `aclerk_driver`'s `NT_CHAR` handler: only the first `quiet_say`
    /// block is reachable (see the module doc comment). Greets each
    /// visible player within distance 5 exactly once (memory slot 7).
    fn greet_nearby_players_aclerk(&mut self, aclerk_id: CharacterId) {
        let Some(aclerk) = self.characters.get(&aclerk_id).cloned() else {
            return;
        };

        let mut greet_targets: Vec<CharacterId> = Vec::new();
        for character in self.characters.values() {
            if character.id == aclerk_id
                || !character.flags.contains(CharacterFlags::PLAYER)
                || mem_check_driver(
                    &aclerk.driver_memory,
                    ACLERK_GREET_MEMORY_SLOT,
                    character.id.0,
                )
            {
                continue;
            }
            if char_dist(&aclerk, character) > ACLERK_GREET_DISTANCE {
                continue;
            }
            if !char_see_char(&aclerk, character, &self.map, self.date.daylight) {
                continue;
            }
            greet_targets.push(character.id);
        }

        for player_id in &greet_targets {
            // C `quiet_say(cn, "Welcome to the Cameron Arena! ...")`.
            self.npc_quiet_say(
                aclerk_id,
                "Welcome to the Cameron Arena! Step into the sand with another player and you shall enjoy the first personal fight of thy life!",
            );
            if let Some(aclerk) = self.characters.get_mut(&aclerk_id) {
                mem_add_driver(
                    &mut aclerk.driver_memory,
                    ACLERK_GREET_MEMORY_SLOT,
                    player_id.0,
                );
            }
        }
    }

    /// C `aclerk_driver`'s idle-murmur block
    /// (`src/module/merchants/merchant.c`): once per minute, on a
    /// `RANDOM(25)` 1-in-25 hit, `RANDOM(11)` picks one of 11 lines.
    fn aclerk_idle_chatter(&mut self, aclerk_id: CharacterId) {
        let tick = self.tick.0;
        let Some(aclerk) = self.characters.get(&aclerk_id).cloned() else {
            return;
        };
        let last_talk = match aclerk.driver_state.as_ref() {
            Some(CharacterDriverState::Aclerk(data)) => data.last_talk,
            _ => return,
        };
        if tick <= last_talk + ACLERK_TALK_INTERVAL_TICKS {
            return;
        }
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, 25) != 0 {
            return;
        }

        let case = legacy_random_below_from_seed(&mut self.legacy_random_seed, 11);
        let indoors = self
            .map
            .tile(usize::from(aclerk.x), usize::from(aclerk.y))
            .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS));
        let pronoun = hisname(&aclerk);

        match case {
            0 => {
                self.npc_murmur(aclerk_id, "Oh, these sand fleas are a nuisance.");
            }
            1 => {
                self.npc_whisper(
                    aclerk_id,
                    "What I've learned in my life is what doesn't kill thou, makes thou stronger.",
                );
            }
            2 => {
                self.npc_murmur(aclerk_id, "Oh yeah, this forest smells fresher than ever.");
            }
            3 => {
                self.npc_murmur(aclerk_id, "This life get's lonelier by the day.");
            }
            4 => {
                self.npc_murmur(aclerk_id, "Oh my, life is hard but I musn't quit.");
            }
            5 => {
                self.npc_murmur(
                    aclerk_id,
                    "Cheers to the fights one can witness in a lifetime.",
                );
            }
            6 => {
                self.npc_murmur(aclerk_id, "Ishtar! Oh, why is my life so dreadful?");
            }
            7 => {
                self.npc_murmur(
                    aclerk_id,
                    "The demons will get you and you can't stop them!",
                );
            }
            8 => {
                self.npc_emote(aclerk_id, &format!("cracks {pronoun} knuckles"));
            }
            9 => {
                if indoors {
                    // C: `emote(cn, "stares at the ceiling")` - no embedded
                    // period, so the wrapper's own "%s %s." adds the only one.
                    self.npc_emote(aclerk_id, "stares at the ceiling");
                } else {
                    // C: `emote(cn, "eyeballs deep within the forest.")` -
                    // the format string already ends in a period, so the
                    // wrapper's own "%s %s." doubles it up. Copied exactly.
                    self.npc_emote(aclerk_id, "eyeballs deep within the forest.");
                }
            }
            10 => {
                // C: `emote(cn, "slaps %s to wake himself up.", hisname(cn))`
                // - embedded period doubled by the wrapper, same as above.
                self.npc_emote(aclerk_id, &format!("slaps {pronoun} to wake himself up."));
            }
            _ => {}
        }

        if let Some(CharacterDriverState::Aclerk(data)) = self
            .characters
            .get_mut(&aclerk_id)
            .and_then(|aclerk| aclerk.driver_state.as_mut())
        {
            data.last_talk = tick;
        }
    }

    fn clear_expired_aclerk_memory(&mut self, aclerk_id: CharacterId) {
        let tick = self.tick.0;
        if let Some(aclerk) = self.characters.get_mut(&aclerk_id) {
            let memory_clear_tick = match aclerk.driver_state.as_ref() {
                Some(CharacterDriverState::Aclerk(data)) => data.memory_clear_tick,
                _ => return,
            };
            if tick > memory_clear_tick {
                mem_erase_driver(&mut aclerk.driver_memory, ACLERK_GREET_MEMORY_SLOT);
                if let Some(CharacterDriverState::Aclerk(data)) = aclerk.driver_state.as_mut() {
                    data.memory_clear_tick = tick + ACLERK_MEMORY_CLEAR_TICKS;
                }
            }
        }
    }
}
