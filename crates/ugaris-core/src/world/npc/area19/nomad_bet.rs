//! `CDR_NOMAD`'s `Llakal Sla` dice-betting minigame plus the salt-currency
//! helpers every file in this module needs, split out of `nomad.rs` to
//! stay under the ~800-line NPC-file guideline (same precedent as
//! `world::npc::area17::guard`/`guard_messages`) - see `nomad.rs`'s own
//! module doc comment for the driver's full behavior.
//!
//! Ports `count_salt`/`remove_salt`/`set_salt_data` (`nomad.c:227-280`),
//! `lucky_die` (`:791-801`, already ported as `crate::item_driver::
//! legacy_lucky_die_from_rolls`), `nomad_bet` (`:803-873`), `nomad_roll`
//! (`:875-925`), and the `NT_NPC`/`NTID_DICE` branch of `nomad`
//! (`:1125-1133`) that receives the player's own dice roll (rolled by the
//! already-ported `IDR_NOMADDICE` item driver, `ugaris-server::
//! tick_item_use_completion`, which broadcasts it via `World::
//! notify_area(x, y, NT_NPC, NTID_DICE, player_id, total)`).
//!
//! A real, deliberately-reproduced C quirk in `nomad_bet`: when reusing a
//! remembered high roll (`ppd->open_bet != 0`), all three dice
//! (`d1`/`d2`/`d3`) are assigned `ppd->open_roll1` - `open_roll2`/
//! `open_roll3` are stored but never actually read back (`nomad.c:846-
//! 848`).

use std::collections::HashMap;

use crate::character_driver::{CharacterDriverMessage, NTID_DICE};
use crate::item_driver::{legacy_lucky_die_from_rolls, IID_AREA19_SALT};
use crate::world::*;

use super::nomad::{NomadDriverData, NomadOutcomeEvent, NomadPlayerFacts};

/// C `TICKS * 60` (`nomad.c:811`): a game is abandoned after a minute of
/// silence.
const NOMAD_BET_TIMEOUT_TICKS: u64 = TICKS_PER_SECOND * 60;

fn read_salt_amount(item: &Item) -> u32 {
    let bytes = &item.driver_data;
    u32::from_le_bytes([
        bytes.first().copied().unwrap_or(0),
        bytes.get(1).copied().unwrap_or(0),
        bytes.get(2).copied().unwrap_or(0),
        bytes.get(3).copied().unwrap_or(0),
    ])
}

fn write_salt_amount(item: &mut Item, value: u32) {
    if item.driver_data.len() < 4 {
        item.driver_data.resize(4, 0);
    }
    item.driver_data[0..4].copy_from_slice(&value.to_le_bytes());
}

impl World {
    /// C `create_item("salt")` immediately followed by `it[in2].value *=
    /// val; *(unsigned int *)(it[in2].drdata) = val; set_salt_data(in2);`
    /// (`nomad_1_give`/`nomad_roll`, `nomad.c:723-725`/`:911-913`): the
    /// freshly-instantiated "salt" template already carries `value` for
    /// one ounce and `drdata` amount `1` (`zones/generic/stack.itm`'s
    /// `salt:` entry), so multiplying `value` by the target amount here
    /// reproduces C's per-unit-value math exactly. `ugaris-server` (which
    /// alone has `ZoneLoader` to create the item) calls this right after
    /// `zone_loader.instantiate_item_template("salt", ...)`.
    pub fn configure_fresh_salt_item(&mut self, item_id: ItemId, amount: u32) {
        if let Some(item) = self.items.get_mut(&item_id) {
            item.value = item.value.saturating_mul(amount);
            write_salt_amount(item, amount);
        }
        self.set_salt_sprite(item_id);
    }

    /// C `set_salt_data` (`nomad.c:227-241`): tiers the sprite by amount
    /// and rewrites the "N ounces of Salt." description.
    pub fn set_salt_sprite(&mut self, item_id: ItemId) {
        let Some(item) = self.items.get_mut(&item_id) else {
            return;
        };
        let amount = read_salt_amount(item);
        item.sprite = if amount >= 10000 {
            13212
        } else if amount >= 1000 {
            13211
        } else if amount >= 100 {
            13210
        } else if amount >= 10 {
            13209
        } else {
            13208
        };
        item.description = format!("{amount} ounces of {}.", item.name);
    }

    /// Reads a carried salt/skin stack's `*(unsigned int *)(it[in].drdata)`
    /// amount; also used by [`super::nomad_give`] for the wolf-skin
    /// trade-in and the tribe-membership threshold check.
    pub fn salt_amount(&self, item_id: ItemId) -> u32 {
        self.items.get(&item_id).map(read_salt_amount).unwrap_or(0)
    }

    /// C `count_salt` (`nomad.c:270-280`): sums every carried "salt" stack
    /// in the non-worn inventory range (`n=30..INVENTORYSIZE`).
    pub fn count_salt(&self, character_id: CharacterId) -> i32 {
        let Some(character) = self.characters.get(&character_id) else {
            return 0;
        };
        let mut total: i64 = 0;
        for item_id in character.inventory.iter().skip(30).flatten() {
            if let Some(item) = self.items.get(item_id) {
                if item.template_id == IID_AREA19_SALT {
                    total += i64::from(read_salt_amount(item));
                }
            }
        }
        total.clamp(0, i64::from(i32::MAX)) as i32
    }

    /// C `remove_salt` (`nomad.c:243-268`): removes `val` ounces of salt
    /// from the non-worn inventory range, destroying fully-consumed
    /// stacks and shrinking (re-pricing, re-sprite-ing) the first
    /// partially-consumed one.
    pub fn remove_salt(&mut self, character_id: CharacterId, mut val: i32) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let salt_items: Vec<ItemId> = character
            .inventory
            .iter()
            .skip(30)
            .flatten()
            .copied()
            .filter(|item_id| {
                self.items
                    .get(item_id)
                    .is_some_and(|item| item.template_id == IID_AREA19_SALT)
            })
            .collect();

        for item_id in salt_items {
            if val <= 0 {
                break;
            }
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            let amount = read_salt_amount(item);
            if i64::from(amount) <= i64::from(val) {
                val -= amount as i32;
                self.destroy_item(item_id);
            } else {
                // C divides unguarded (`price = it[in].value / (*(unsigned
                // int *)(it[in].drdata))`, `nomad.c:256`); this branch only
                // runs with `amount > val >= 1`, so the `== 0` arm is
                // defensive-only dead code kept for safety.
                #[allow(clippy::manual_checked_ops)]
                let price = if amount == 0 { 0 } else { item.value / amount };
                let new_amount = amount - val as u32;
                if let Some(item) = self.items.get_mut(&item_id) {
                    write_salt_amount(item, new_amount);
                    item.value = price * new_amount;
                }
                self.set_salt_sprite(item_id);
                val = 0;
            }
        }
    }

    /// C `nomad_bet` (`nomad.c:803-873`).
    pub(super) fn nomad_bet(
        &mut self,
        nomad_id: CharacterId,
        data: &mut NomadDriverData,
        player_id: CharacterId,
        val: i32,
        facts: &NomadPlayerFacts,
        events: &mut Vec<NomadOutcomeEvent>,
    ) {
        let tick = self.tick.0;
        // C `if (ticker - dat->play_timer > TICKS*60) dat->play_with = 0;`
        // (`nomad.c:811-813`).
        if tick.saturating_sub(data.play_timer) > NOMAD_BET_TIMEOUT_TICKS {
            data.play_with = None;
        }
        // C `if (dat->play_with && !char_see_char(cn, dat->play_with))
        // dat->play_with = 0;` (`nomad.c:814-816`).
        if let Some(current) = data.play_with {
            let still_visible = match (
                self.characters.get(&nomad_id).cloned(),
                self.characters.get(&current).cloned(),
            ) {
                (Some(nomad), Some(other)) => {
                    char_see_char(&nomad, &other, &self.map, self.date.daylight)
                }
                _ => false,
            };
            if !still_visible {
                data.play_with = None;
            }
        }
        if let Some(current) = data.play_with {
            let name = self.player_name(current);
            self.npc_say(
                nomad_id,
                &format!("Sorry, I'm playing with {name} right now."),
            );
            return;
        }

        let nr = data.nr as usize;
        if facts.nomad_win[nr] < -data.max_loss {
            let player_name = self.player_name(player_id);
            self.npc_say(
                nomad_id,
                &format!(
                    "I won't play with thee anymore, {player_name}. Thou art too lucky for my \
                     taste."
                ),
            );
            return;
        }
        if val < data.min_bet {
            self.npc_say(nomad_id, &format!("{val} ounces? That's too cheap."));
            return;
        }
        if val > data.max_bet {
            self.npc_say(nomad_id, &format!("{val} ounces is too much for my taste."));
            return;
        }
        if val > self.count_salt(player_id) {
            let player_name = self.player_name(player_id);
            self.npc_say(
                nomad_id,
                &format!("Thou dost not have {val} ounces of salt to play with, {player_name}."),
            );
            return;
        }

        data.play_with = Some(player_id);
        data.play_timer = tick;
        data.bet = val;

        // C `if (ppd->open_bet) { d1=d2=d3=ppd->open_roll1; } else { d1 =
        // lucky_die(...); d2 = lucky_die(...); d3 = lucky_die(...); }`
        // (`nomad.c:845-853`) - see the module doc comment for the
        // `open_roll2`/`open_roll3`-never-read real C quirk.
        let (d1, d2, d3) = if facts.open_bet != 0 {
            (facts.open_roll.0, facts.open_roll.0, facts.open_roll.0)
        } else {
            let mut seed = self.legacy_random_seed;
            let luck = data.dice_skill.max(0) as u8;
            let roll_die = |seed: &mut u32| -> i32 {
                let rolls = (0..=luck).map(|_| (legacy_random_below_from_seed(seed, 6) + 1) as u8);
                i32::from(legacy_lucky_die_from_rolls(6, luck, rolls))
            };
            let d1 = roll_die(&mut seed);
            let d2 = roll_die(&mut seed);
            let d3 = roll_die(&mut seed);
            self.legacy_random_seed = seed;
            (d1, d2, d3)
        };
        data.my_throw = d1 + d2 + d3;

        let player_name = self.player_name(player_id);
        if data.my_throw < 11 {
            self.npc_say(
                nomad_id,
                &format!("Ack. Well, roll your dice now, {player_name}. (USE the dice)"),
            );
        } else if data.my_throw < 14 {
            self.npc_say(
                nomad_id,
                &format!("Your turn, {player_name}. (USE the dice)"),
            );
        } else {
            self.npc_say(
                nomad_id,
                &format!("Ha! Now it's your turn, {player_name}. (USE the dice)"),
            );
        }

        // C `if (d1+d2+d3 > 13) { ppd->open_bet = val; ppd->open_roll1 =
        // d1; ppd->open_roll2 = d2; ppd->open_roll3 = d3; }`
        // (`nomad.c:867-872`).
        if d1 + d2 + d3 > 13 {
            events.push(NomadOutcomeEvent::SetOpenBet {
                player_id,
                bet: val,
                roll1: d1,
                roll2: d2,
                roll3: d3,
            });
        }
    }

    /// C `nomad_roll` (`nomad.c:875-925`).
    pub(super) fn nomad_roll(
        &mut self,
        nomad_id: CharacterId,
        data: &mut NomadDriverData,
        player_id: CharacterId,
        val: i32,
        facts: &NomadPlayerFacts,
        events: &mut Vec<NomadOutcomeEvent>,
    ) {
        data.play_with = None;
        let nr = data.nr as usize;

        // C `if (dat->bet >= ppd->open_bet) ppd->open_bet = 0; else ppd->
        // open_bet -= dat->bet;` (`nomad.c:885-889`).
        let new_open_bet = if data.bet >= facts.open_bet {
            0
        } else {
            facts.open_bet - data.bet
        };
        events.push(NomadOutcomeEvent::SetOpenBet {
            player_id,
            bet: new_open_bet,
            roll1: facts.open_roll.0,
            roll2: facts.open_roll.1,
            roll3: facts.open_roll.2,
        });

        if data.bet > self.count_salt(player_id) {
            let player_name = self.player_name(player_id);
            self.npc_say(
                nomad_id,
                &format!(
                    "Thou dost not have {val} ounces of salt to play with, {player_name}. The \
                     game's cancelled."
                ),
            );
            return;
        }

        let player_name = self.player_name(player_id);
        if data.my_throw > val {
            self.npc_say(
                nomad_id,
                &format!("It's a pleasure playing with thee, {player_name}."),
            );
            self.remove_salt(player_id, data.bet);
            if let Some(player) = self.characters.get_mut(&player_id) {
                player.flags.insert(CharacterFlags::ITEMS);
            }
            events.push(NomadOutcomeEvent::AdjustNomadWin {
                player_id,
                nr,
                delta: data.bet,
            });
            return;
        }
        if data.my_throw < val {
            self.npc_say(nomad_id, "Dang. Lost again.");
            events.push(NomadOutcomeEvent::PaySaltWinnings {
                nomad_id,
                player_id,
                amount: data.bet,
                nr,
            });
            return;
        }
        self.npc_say(nomad_id, "Oh, a draw. No winner.");
    }

    /// C `nomad`'s `NT_NPC`/`NTID_DICE` branch (`nomad.c:1125-1133`): the
    /// player's own dice roll, delivered by `IDR_NOMADDICE`'s server-side
    /// completion via `World::notify_area`.
    pub(super) fn nomad_handle_npc_message(
        &mut self,
        nomad_id: CharacterId,
        data: &mut NomadDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, NomadPlayerFacts>,
        events: &mut Vec<NomadOutcomeEvent>,
    ) {
        if message.dat1 != NTID_DICE {
            return;
        }
        let player_id = CharacterId(message.dat2.max(0) as u32);
        let val = message.dat3;
        if data.play_with != Some(player_id) {
            return;
        }
        let Some(facts) = player_facts.get(&player_id).copied() else {
            return;
        };
        self.nomad_roll(nomad_id, data, player_id, val, &facts, events);
    }
}
