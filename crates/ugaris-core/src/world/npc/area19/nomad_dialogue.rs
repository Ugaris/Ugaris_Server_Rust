//! `CDR_NOMAD`'s six persona greeting ladders (`nomad_1`..`nomad_6`), split
//! out of `nomad.rs` to stay under the ~800-line NPC-file guideline (same
//! precedent as `world::npc::area17::guard`/`guard_messages`) - see
//! `nomad.rs`'s own module doc comment for the driver's full behavior.

use crate::world::*;

use super::nomad::{NomadOutcomeEvent, NomadPlayerFacts};

impl World {
    /// C `nomad_1` (`nomad.c:302-361`): Kalanur, the tribe recruiter,
    /// quest 32.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn nomad_1(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        player_name: &str,
        nomad_name: &str,
        facts: &NomadPlayerFacts,
        nr: usize,
        events: &mut Vec<NomadOutcomeEvent>,
    ) -> bool {
        match facts.nomad_state[nr] {
            0 => {
                self.npc_say(
                    nomad_id,
                    &format!("Sul vana ley, {player_name}. I am {nomad_name}."),
                );
                events.push(NomadOutcomeEvent::QuestOpen {
                    player_id,
                    quest_id: 32,
                });
                events.push(NomadOutcomeEvent::UpdateNomadState {
                    player_id,
                    nr,
                    new_state: 1,
                });
                true
            }
            1 => {
                self.npc_say(
                    nomad_id,
                    &format!(
                        "Welcome to the plains of the Vana Laka. Thou wouldst best learn about our \
                         customs, before thou venturest further north, {player_name}."
                    ),
                );
                self.set_nomad_state_event(events, player_id, nr, 2);
                true
            }
            2 => {
                self.npc_say(
                    nomad_id,
                    "We value not what the city-folks call money. We have no use for it. Most of \
                     our trades work in skins or salt for both are important for our survival.",
                );
                self.set_nomad_state_event(events, player_id, nr, 3);
                true
            }
            3 => {
                self.npc_say(
                    nomad_id,
                    "We are fierce warriors, and our shaman's magic is deadly. Each tribe of the \
                     Vana Laka is loyal only to its members, and, to a certain degree, to the \
                     members of the other tribes.",
                );
                self.set_nomad_state_event(events, player_id, nr, 4);
                true
            }
            4 => {
                self.npc_say(
                    nomad_id,
                    "Those who are tribe-less will have a hard time finding trade partners, or \
                     opponents for Llakal Sla.",
                );
                self.set_nomad_state_event(events, player_id, nr, 5);
                true
            }
            5 => {
                self.npc_say(
                    nomad_id,
                    "If thou wishest to earn membership in my tribe, the Vana Kiru, thou must \
                     prove thy worth to us. Collect 100 ounces of salt and hand them to me, and I \
                     will welcome thee to the Vana Kiru.",
                );
                self.set_nomad_state_event(events, player_id, nr, 6);
                true
            }
            6 => {
                self.npc_say(
                    nomad_id,
                    "I will also trade any wolf skins thou might find for salt, in spite of thee \
                     being tribe-less. Thou canst find wolves to the north-east, or thou canst go \
                     north-west, to my tribe.",
                );
                self.set_nomad_state_event(events, player_id, nr, 7);
                true
            }
            7 => {
                self.npc_say(
                    nomad_id,
                    &format!("Kan vana ley, {player_name} - go in peace."),
                );
                self.set_nomad_state_event(events, player_id, nr, 8);
                true
            }
            9 => {
                self.npc_say(
                    nomad_id,
                    "Mayest thy step be light and thy pockets filled with salt.",
                );
                self.set_nomad_state_event(events, player_id, nr, 10);
                true
            }
            _ => false, // 8/10: waiting/done.
        }
    }

    /// C `nomad_2` (`nomad.c:363-388`): Irakar, the dice seller.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn nomad_2(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        player_name: &str,
        nomad_name: &str,
        facts: &NomadPlayerFacts,
        nr: usize,
        events: &mut Vec<NomadOutcomeEvent>,
    ) -> bool {
        match facts.nomad_state[nr] {
            0 => {
                if facts.tribe_member & super::TM_TRIBE1 == 0 {
                    return false;
                }
                self.npc_say(
                    nomad_id,
                    &format!("Sul vana ley, {player_name}. I am {nomad_name}."),
                );
                self.set_nomad_state_event(events, player_id, nr, 1);
                true
            }
            1 => {
                self.npc_say(
                    nomad_id,
                    "I have a nice collection of dice. I'd sell thee a set of cheap dice for 200 \
                     ounces of salt, or a set of mediocre dice for 500 ounces, or a set of  good \
                     dice for 1200 ounces.",
                );
                self.set_nomad_state_event(events, player_id, nr, 2);
                true
            }
            _ => false,
        }
    }

    /// C `nomad_3` (`nomad.c:390-412`): the `Llakal Sla` game host.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn nomad_3(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        player_name: &str,
        nomad_name: &str,
        facts: &NomadPlayerFacts,
        nr: usize,
        events: &mut Vec<NomadOutcomeEvent>,
    ) -> bool {
        match facts.nomad_state[nr] {
            0 => {
                if facts.tribe_member & super::TM_TRIBE1 == 0 {
                    return false;
                }
                self.npc_say(
                    nomad_id,
                    &format!("Sul vana ley, {player_name}. I am {nomad_name}."),
                );
                self.set_nomad_state_event(events, player_id, nr, 1);
                true
            }
            1 => {
                self.npc_say(nomad_id, "Would you like a game of Llakal Sla?");
                self.set_nomad_state_event(events, player_id, nr, 2);
                true
            }
            _ => false,
        }
    }

    /// C `nomad_4` (`nomad.c:414-455`): the Kir monk, Sarkilar's-fate
    /// quest 33.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn nomad_4(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        player: &Character,
        nomad_name: &str,
        facts: &NomadPlayerFacts,
        nr: usize,
        events: &mut Vec<NomadOutcomeEvent>,
    ) -> bool {
        let player_name = player.name.as_str();
        match facts.nomad_state[nr] {
            0 => {
                self.npc_say(
                    nomad_id,
                    &format!(
                        "Welcome to the monastery of Kir Laka, {player_name}. I am {nomad_name}."
                    ),
                );
                events.push(NomadOutcomeEvent::QuestOpen {
                    player_id,
                    quest_id: 33,
                });
                self.set_nomad_state_event(events, player_id, nr, 1);
                true
            }
            1 => {
                let gender = if player.flags.contains(CharacterFlags::MALE) {
                    "men"
                } else {
                    "women"
                };
                self.npc_say(
                    nomad_id,
                    &format!(
                        "It is seldom indeed that we have visitors in these sad times. The \
                         mountains have never been friendly, but only a short while ago all one \
                         had to fear was the cold, or losing one's way. Now one has to fear being \
                         eaten alive by Harpies. Welcome again, {player_name}. It is good to see \
                         that there are {gender} brave enough to visit us."
                    ),
                );
                self.set_nomad_state_event(events, player_id, nr, 2);
                true
            }
            2 => {
                self.npc_say(
                    nomad_id,
                    "Unfortunately, the Harpies are not our only problem. Brother Sarkilar has \
                     left the monastery with a few of the younger brothers. We haven't heard from \
                     him since he left, and I am worried. Neither the nomads nor the valkyries \
                     have seen him, so he must still be in this mountain. If thou couldst find out \
                     what happened to him?",
                );
                self.set_nomad_state_event(events, player_id, nr, 3);
                true
            }
            4 => {
                self.npc_say(
                    nomad_id,
                    &format!(
                        "Oh, Sarkilar! What hast thou done? How couldst thou fall for the silver \
                         tongue of evil? I thank thee, {player_name}, even though thine news is \
                         sad indeed."
                    ),
                );
                self.set_nomad_state_event(events, player_id, nr, 5);
                true
            }
            _ => false, // 3: waiting for news; 5: done.
        }
    }

    /// C `nomad_5` (`nomad.c:457-490`): the Kir monk, life-teacher quest
    /// 34.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn nomad_5(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        player_name: &str,
        nomad_name: &str,
        facts: &NomadPlayerFacts,
        nr: usize,
        events: &mut Vec<NomadOutcomeEvent>,
    ) -> bool {
        match facts.nomad_state[nr] {
            0 => {
                self.npc_say(
                    nomad_id,
                    &format!(
                        "Welcome to the monastery of Kir Laka, {player_name}. I am {nomad_name}, \
                         the teacher."
                    ),
                );
                events.push(NomadOutcomeEvent::QuestOpen {
                    player_id,
                    quest_id: 34,
                });
                self.set_nomad_state_event(events, player_id, nr, 1);
                true
            }
            1 => {
                self.npc_say(
                    nomad_id,
                    "We of the Kir believe in life, and in the mastery of life. The concept of \
                     god-hood is alien to us; and while we value Ishtar's fight against the \
                     demonic hordes, we do not believe he is a god. But enough of Ishtar.",
                );
                self.set_nomad_state_event(events, player_id, nr, 2);
                true
            }
            2 => {
                self.npc_say(
                    nomad_id,
                    "I can teach thee about life, in exchange for a golden statue of Kir. One of \
                     the nomads sells them.",
                );
                self.set_nomad_state_event(events, player_id, nr, 3);
                true
            }
            4 => {
                self.npc_say(
                    nomad_id,
                    "If thou ever needst to regain lost experiences, I can help thee in the \
                     process. I won't be able to bring it all back, but most of it. Do not forget \
                     to bring a golden Kir statue...",
                );
                self.set_nomad_state_event(events, player_id, nr, 5);
                true
            }
            _ => false, // 3/5: waiting for a statue.
        }
    }

    /// C `nomad_6` (`nomad.c:492-516`): the golden-statue seller.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn nomad_6(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        player_name: &str,
        nomad_name: &str,
        facts: &NomadPlayerFacts,
        nr: usize,
        events: &mut Vec<NomadOutcomeEvent>,
    ) -> bool {
        match facts.nomad_state[nr] {
            0 => {
                if facts.tribe_member & super::TM_TRIBE1 == 0 {
                    return false;
                }
                self.npc_say(
                    nomad_id,
                    &format!("Sul vana ley, {player_name}. I am {nomad_name}."),
                );
                self.set_nomad_state_event(events, player_id, nr, 1);
                true
            }
            1 => {
                self.npc_say(
                    nomad_id,
                    "Wouldst thou like to buy a golden statue? I shall give it to thee for only \
                     10000 ounces of salt!",
                );
                self.set_nomad_state_event(events, player_id, nr, 2);
                true
            }
            _ => false,
        }
    }
}
