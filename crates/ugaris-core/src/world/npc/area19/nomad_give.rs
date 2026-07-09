//! `CDR_NOMAD`'s `NT_GIVE` handling, split out of `nomad.rs` to stay under
//! the ~800-line NPC-file guideline (same precedent as `world::npc::
//! area17::guard`/`guard_messages`) - see `nomad.rs`'s own module doc
//! comment for the driver's full behavior.
//!
//! Ports `nomad`'s `NT_GIVE` branch (`nomad.c:1078-1115`) plus
//! `nomad_1_give` (`:681-737`, the tribe-membership salt turn-in and the
//! wolf-skin-for-salt trade), `nomad_4_give` (`:739-754`, the Sarkilar
//! letter turn-in), and `nomad_5_give` (`:756-789`, the golden Kir statue
//! turn-in). Every other persona (2/3/6) falls through to C's plain
//! `give_char_item`/`destroy_item` default, same as any item a
//! quest-giving persona doesn't recognize.

use std::collections::HashMap;

use crate::character_driver::CharacterDriverMessage;
use crate::item_driver::{
    IID_AREA19_KIR, IID_AREA19_KIRLETTER, IID_AREA19_SALT, IID_AREA19_WOLFSSKIN,
    IID_AREA19_WOLFSSKIN2,
};
use crate::world::*;

use super::nomad::{NomadDriverData, NomadOutcomeEvent, NomadPlayerFacts};
use super::TM_TRIBE1;

impl World {
    /// C `nomad`'s `NT_GIVE` branch (`nomad.c:1078-1115`).
    pub(super) fn nomad_handle_give_message(
        &mut self,
        nomad_id: CharacterId,
        data: &NomadDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, NomadPlayerFacts>,
        events: &mut Vec<NomadOutcomeEvent>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get(&nomad_id)
            .and_then(|nomad| nomad.cursor_item)
        else {
            return;
        };

        // C `if ((ppd = set_data(co, DRD_NOMAD_PPD, ...))) { ... } else {
        // destroy_item(in); ch[cn].citem = 0; }` (`nomad.c:1082,1110-
        // 1113`).
        let Some(facts) = player_facts.get(&player_id).copied() else {
            if let Some(nomad) = self.characters.get_mut(&nomad_id) {
                nomad.cursor_item = None;
            }
            self.destroy_item(item_id);
            return;
        };

        let consumed = match data.nr {
            1 => self.nomad_1_give(nomad_id, player_id, item_id, &facts, data, events),
            4 => self.nomad_4_give(player_id, item_id, &facts, data, events),
            5 => self.nomad_5_give(nomad_id, player_id, item_id, &facts, data, events),
            _ => false,
        };

        if let Some(nomad) = self.characters.get_mut(&nomad_id) {
            nomad.cursor_item = None;
        }
        if !consumed && !self.give_char_item(player_id, item_id) {
            self.destroy_item(item_id);
        }
    }

    /// C `nomad_1_give` (`nomad.c:681-737`).
    fn nomad_1_give(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        item_id: ItemId,
        facts: &NomadPlayerFacts,
        data: &NomadDriverData,
        events: &mut Vec<NomadOutcomeEvent>,
    ) -> bool {
        let nr = data.nr as usize;
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let template_id = item.template_id;

        if template_id == IID_AREA19_SALT {
            let amount = self.salt_amount(item_id);
            if facts.nomad_state[nr] > 8 {
                let player_name = self.player_name(player_id);
                self.npc_say(
                    nomad_id,
                    &format!("Thou art already a member of my tribe, {player_name}."),
                );
                return false;
            }
            if amount < 100 {
                self.npc_say(
                    nomad_id,
                    "This is not enough. Thou needst give me 100 ounces of salt for thy membership.",
                );
                return false;
            }
            if amount > 100 {
                self.npc_say(
                    nomad_id,
                    "This is most generous, but I wilt not accept more than 100 ounces for thy \
                     membership.",
                );
                return false;
            }
            let player_name = self.player_name(player_id);
            self.npc_say(
                nomad_id,
                &format!("Welcome to the tribe of the Vana Kiru, {player_name}."),
            );
            events.push(NomadOutcomeEvent::QuestDone {
                player_id,
                quest_id: 32,
            });
            self.set_nomad_state_event(events, player_id, nr, 9);
            events.push(NomadOutcomeEvent::SetTribeMember {
                player_id,
                flag: TM_TRIBE1,
            });
            self.destroy_item(item_id);
            return true;
        }

        if template_id == IID_AREA19_WOLFSSKIN || template_id == IID_AREA19_WOLFSSKIN2 {
            let count = self.salt_amount(item_id);
            let value = if template_id == IID_AREA19_WOLFSSKIN {
                count * 5
            } else {
                count * 20
            };
            events.push(NomadOutcomeEvent::GiveSaltForSkin {
                nomad_id,
                player_id,
                skin_item_id: item_id,
                amount: value,
            });
            return true;
        }

        false
    }

    /// C `nomad_4_give` (`nomad.c:739-754`).
    fn nomad_4_give(
        &mut self,
        player_id: CharacterId,
        item_id: ItemId,
        facts: &NomadPlayerFacts,
        data: &NomadDriverData,
        events: &mut Vec<NomadOutcomeEvent>,
    ) -> bool {
        let nr = data.nr as usize;
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.template_id == IID_AREA19_KIRLETTER && facts.nomad_state[nr] <= 3 {
            self.set_nomad_state_event(events, player_id, nr, 4);
            events.push(NomadOutcomeEvent::QuestDone {
                player_id,
                quest_id: 33,
            });
            self.destroy_item(item_id);
            self.destroy_items_by_template_id(player_id, IID_AREA19_KIRLETTER);
            return true;
        }
        false
    }

    /// C `nomad_5_give` (`nomad.c:756-789`).
    fn nomad_5_give(
        &mut self,
        nomad_id: CharacterId,
        player_id: CharacterId,
        item_id: ItemId,
        facts: &NomadPlayerFacts,
        data: &NomadDriverData,
        events: &mut Vec<NomadOutcomeEvent>,
    ) -> bool {
        let nr = data.nr as usize;
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.template_id != IID_AREA19_KIR {
            return false;
        }
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return false;
        };

        if facts.nomad_state[nr] > 3 {
            if player.exp > player.exp_used {
                self.npc_say(nomad_id, "But thou dost not have lost any experience.");
                return false;
            }
            self.npc_say(
                nomad_id,
                "There, some of the memories are back. I can work with thee further, if thou \
                 bringst me another of these statues.",
            );
            let diff = i64::from(player.exp_used) - i64::from(player.exp);
            let base_exp = if player.flags.contains(CharacterFlags::HARDCORE) {
                diff / 10
            } else {
                diff / 2
            };
            events.push(NomadOutcomeEvent::GiveExp {
                player_id,
                base_exp,
            });
        } else {
            let player_name = player.name.clone();
            self.npc_say(
                nomad_id,
                &format!(
                    "Isn't it beautiful? Thank thee, {player_name}. Now, let me teach thee..."
                ),
            );
            events.push(NomadOutcomeEvent::QuestDone {
                player_id,
                quest_id: 34,
            });
            self.set_nomad_state_event(events, player_id, nr, 4);
        }
        self.destroy_item(item_id);
        true
    }

    pub(super) fn player_name(&self, character_id: CharacterId) -> String {
        self.characters
            .get(&character_id)
            .map(|character| character.name.clone())
            .unwrap_or_default()
    }
}
