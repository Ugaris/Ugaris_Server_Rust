//! `CDR_TEUFELGAMBLER` (Teufelheim's three-tier dice-and-chips demon
//! gambler), ports `src/area/34/teufel.c::teufelgambler_driver`
//! (`:1233-1435`) plus its three reward tables `give_reward`/
//! `give_reward2`/`give_reward3` (`:635-1204`) and `set_chip_data`
//! (`:1211-1231`).
//!
//! Three zone-file templates (`gambler`/`gambler2`/`gambler3`,
//! `ugaris_data/zones/34/teufel.chr:750-946`) share this one driver,
//! distinguished only by `dat->nr` (`1`/`2`/`3`) parsed once from
//! `arg="N"` at spawn time - same precedent as `world::npc::area31::
//! lostdwarf`'s own module doc comment (`CDR_LOSTDWARF`'s `nr` parsed in
//! `crate::zone`, not re-parsed via `NT_CREATE` here). `nr` selects which
//! chip color the gambler accepts (bronze/silver/gold,
//! [`IID_BRONZECHIP`]/[`IID_SILVERCHIP`]/[`IID_GOLDCHIP`]) and which
//! reward table applies.
//!
//! The dialogue tree (`play`/`play2`/`play3`, `bet one/two/five`, "what's
//! your name") is the shared [`TEUFEL_QA`] table already ported for
//! `teufelquest_driver` - answer codes `2`/`3`/`4` ("bet one/two/five",
//! previously unwired, see `world::npc::area34::mod`'s own doc comment)
//! are this driver's exclusive consumers.
//!
//! Deviations/gaps (documented, not silent):
//! - C's unconditional `do_idle(cn, TICKS)` tail call is not ported,
//!   matching the established `world::npc::area33::gorwin`/
//!   `world::npc::area34::teufelquest` precedent for stationary dialogue
//!   NPCs.
//! - The `(ch[co].flags & CF_GOD) && strstr(msg->dat2, "reward: ")`
//!   debug/cheat branch (`teufel.c:1403-1413`) is ported verbatim,
//!   including running independently of whether the QA table matched
//!   anything that same tick (C's `if ((ptr = strstr(...)))` is a
//!   sibling `if`, not an `else if`, of the QA-driven block) and
//!   including the `Bug #1778`/`Bug #1779` fallback text when the cheat's
//!   free-form roll number doesn't land on a winning `give_rewardN` case
//!   (C's `elog(...)` server-log calls are not ported, matching every
//!   other NPC driver's own precedent of dropping `elog` in favor of the
//!   player-visible `log_char` text only).
//! - `mem_check_driver`/`mem_add_driver`(`, 7)`/`mem_erase_driver` (the
//!   "greet once, forget after 12h" memory slot) uses the same slot `7`
//!   and cadence as `teufelquest_driver`'s own copy - both drivers keep
//!   fully independent `driver_memory`, matching C's per-character
//!   `mem[]` array.

use crate::character_driver::{
    mem_add_driver, mem_check_driver, mem_erase_driver, TextAnalysisOutcome, CDR_TEUFELGAMBLER,
};
use crate::drvlib::offset2dx;
use crate::world::npc::area30::clanclerk::parse_int_atoi;
use crate::world::npc::area34::{is_demon, teufel_analyse_text, TeufelTextOutcome};
use crate::world::*;

/// C `mem_check_driver(cn, co, 7)`/`mem_add_driver(cn, co, 7)`
/// (`teufel.c:1270,1313`): the conventional "greet once" memory slot
/// shared by every ported NPC that uses driver memory this way.
const TEUFELGAMBLER_GREET_MEMORY_SLOT: usize = 7;
/// C `char_dist(cn, co) > 16` (`teufel.c:1264`).
const TEUFELGAMBLER_TALK_DISTANCE: i32 = 16;
/// C `TICKS * 60 * 60 * 12` (`teufel.c:1427`): 12-hour memory-erase
/// cadence.
const TEUFELGAMBLER_MEMORY_CLEAR_TICKS: u64 = TICKS_PER_SECOND * 60 * 60 * 12;

/// C `#define IID_BRONZECHIP MAKE_ITEMID(DEV_ID_DB, 0x0000AC)`
/// (`src/common/item_id.h:231`).
pub const IID_BRONZECHIP: u32 = 0x0100_00AC;
/// C `#define IID_SILVERCHIP MAKE_ITEMID(DEV_ID_DB, 0x0000AD)`
/// (`src/common/item_id.h:232`).
pub const IID_SILVERCHIP: u32 = 0x0100_00AD;
/// C `#define IID_GOLDCHIP MAKE_ITEMID(DEV_ID_DB, 0x0000AE)`
/// (`src/common/item_id.h:233`).
pub const IID_GOLDCHIP: u32 = 0x0100_00AE;

/// A resolved `give_reward`/`give_reward2`/`give_reward3` outcome
/// (`teufel.c:635-1204`): either a direct money grant (the `give_money`
/// `return;` cases) or `cnt` copies of one item template (the
/// `create_item(ptr)` loop, `:807-823`).
enum GambleRewardOutcome {
    Money(u32),
    Items { template: String, count: u32 },
}

fn money(amount: u32) -> Option<GambleRewardOutcome> {
    Some(GambleRewardOutcome::Money(amount))
}

fn items(template: impl Into<String>, count: u32) -> Option<GambleRewardOutcome> {
    Some(GambleRewardOutcome::Items {
        template: template.into(),
        count,
    })
}

/// C's `if (bet == 5) "...3"; else if (bet == 2) "...2"; else "...1";`
/// tier-suffix pattern, repeated for every `demon_*` gear reward across
/// all three tables (e.g. `teufel.c:641-647`).
fn demon_gear_tier(bet: u32) -> &'static str {
    if bet == 5 {
        "3"
    } else if bet == 2 {
        "2"
    } else {
        "1"
    }
}

/// C `give_reward(cn, nr, bet)` (`teufel.c:635-824`): the bronze-chip
/// table (`dat->nr == 1`).
fn give_reward1(roll: i32, bet: u32) -> Option<GambleRewardOutcome> {
    let tier = demon_gear_tier(bet);
    match roll {
        3 => items(format!("demon_warrior{tier}"), 1),
        60 => items(format!("demon_mage{tier}"), 1),
        4 | 59 => money(bet * 2_000_000),
        5 => items(format!("demon_tacoff{tier}"), 1),
        58 => items(format!("demon_magoff{tier}"), 1),
        6 => items(format!("demon_tacdef{tier}"), 1),
        57 => items(format!("demon_magdef{tier}"), 1),
        7 | 56 => items("goldchip", bet),
        8 | 55 => money(bet * 500_000),
        9 => items(format!("demon_sword{tier}"), 1),
        54 => items(format!("demon_flash{tier}"), 1),
        10 => items(format!("demon_twohanded{tier}"), 1),
        53 => items(format!("demon_fire{tier}"), 1),
        11 | 52 => items("silverchip", bet),
        12 | 51 => money(bet * 100_000),
        13 | 50 => money(bet * 50_000),
        14 | 49 => items("mis_combopot", bet * 2),
        15 | 48 => items("mis_combopot", bet),
        16 | 47 => items("combo_potion3", bet),
        17 | 46 => items("combo_potion2", bet),
        18 | 45 => items("combo_potion1", bet),
        19 | 44 => items("green_torch", bet),
        20 | 43 => items("torch", bet),
        _ => None,
    }
}

/// C `give_reward2(cn, nr, bet)` (`teufel.c:826-1015`): the silver-chip
/// table (`dat->nr == 2`).
fn give_reward2(roll: i32, bet: u32) -> Option<GambleRewardOutcome> {
    let tier = demon_gear_tier(bet);
    match roll {
        3 => items(format!("demon_warrior{tier}b"), 1),
        60 => items(format!("demon_mage{tier}b"), 1),
        4 | 59 => money(bet * 3_000_000),
        5 => items(format!("demon_tacoff{tier}b"), 1),
        58 => items(format!("demon_magoff{tier}b"), 1),
        6 => items(format!("demon_tacdef{tier}b"), 1),
        57 => items(format!("demon_magdef{tier}b"), 1),
        7 | 56 => items("goldchip", bet * 2),
        8 | 55 => money(bet * 750_000),
        9 => items(format!("demon_sword{tier}b"), 1),
        54 => items(format!("demon_flash{tier}b"), 1),
        10 => items(format!("demon_twohanded{tier}b"), 1),
        53 => items(format!("demon_fire{tier}b"), 1),
        11 | 52 => items("goldchip", bet),
        12 | 51 => money(bet * 150_000),
        13 | 50 => money(bet * 75_000),
        14 | 49 => items("mis_combopot", bet * 3),
        15 | 48 => items("mis_combopot", bet * 2),
        16 | 47 => items("combo_potion3", bet * 2),
        17 | 46 => items("combo_potion2", bet * 2),
        18 | 45 => items("combo_potion1", bet * 2),
        19 | 44 => items("green_torch", bet * 2),
        20 | 43 => items("torch", bet * 2),
        _ => None,
    }
}

/// C `give_reward3(cn, nr, bet)` (`teufel.c:1017-1204`): the gold-chip
/// table (`dat->nr == 3`).
fn give_reward3(roll: i32, bet: u32) -> Option<GambleRewardOutcome> {
    let tier = demon_gear_tier(bet);
    match roll {
        3 => items(format!("demon_warrior{tier}c"), 1),
        60 => items(format!("demon_mage{tier}c"), 1),
        4 | 59 => money(bet * 4_000_000),
        5 => items(format!("demon_tacoff{tier}c"), 1),
        58 => items(format!("demon_magoff{tier}c"), 1),
        6 => items(format!("demon_tacdef{tier}c"), 1),
        57 => items(format!("demon_magdef{tier}c"), 1),
        7 | 56 => money(bet * 1_250_000),
        8 | 55 => money(bet * 1_000_000),
        9 => items(format!("demon_sword{tier}c"), 1),
        54 => items(format!("demon_flash{tier}c"), 1),
        10 => items(format!("demon_twohanded{tier}c"), 1),
        53 => items(format!("demon_fire{tier}c"), 1),
        11 | 52 => money(bet * 250_000),
        12 | 51 => money(bet * 200_000),
        13 | 50 => money(bet * 100_000),
        14 | 49 => items("mis_combopot", bet * 4),
        15 | 48 => items("mis_combopot", bet * 2),
        16 | 47 => items("combo_potion3", bet * 2),
        17 | 46 => items("combo_potion2", bet * 2),
        18 | 45 => items("combo_potion1", bet * 2),
        19 | 44 => items("green_torch", bet * 2),
        20 | 43 => items("torch", bet * 2),
        _ => None,
    }
}

/// C `set_data(cn, DRD_TEUFELGAMBLE, sizeof(struct gamble_data))`'s
/// `struct gamble_data { int memcleartimer; int nr; }` (`teufel.c:1206-
/// 1209`), fully carried here (unlike the sibling `TeufelQuestDriverData`,
/// which only reads `memcleartimer` - see that module's own doc comment
/// for why `nr` is dropped there).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TeufelGambleDriverData {
    #[serde(default)]
    pub memcleartimer: u64,
    #[serde(default)]
    pub nr: i32,
}

impl World {
    /// C `teufelgambler_driver`'s per-tick body (`teufel.c:1233-1435`).
    pub fn process_teufelgambler_actions(&mut self, loader: &mut ZoneLoader) {
        let gambler_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TEUFELGAMBLER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for gambler_id in gambler_ids {
            self.process_teufelgambler_messages(loader, gambler_id);
        }
    }

    fn process_teufelgambler_messages(&mut self, loader: &mut ZoneLoader, gambler_id: CharacterId) {
        let Some(CharacterDriverState::TeufelGambler(mut data)) = self
            .characters
            .get(&gambler_id)
            .and_then(|character| character.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&gambler_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.teufelgambler_handle_char_message(
                    gambler_id,
                    &data,
                    message,
                    &mut face_target,
                ),
                NT_TEXT => self.teufelgambler_handle_text_message(
                    loader,
                    gambler_id,
                    &data,
                    message,
                    &mut face_target,
                ),
                _ => {}
            }
        }

        // C `if (talkdir) turn(cn, talkdir);` (`teufel.c:1430-1432`).
        if let (Some(gambler), Some((tx, ty))) =
            (self.characters.get(&gambler_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(gambler.x), i32::from(gambler.y), tx, ty) {
                if let Some(gambler_mut) = self.characters.get_mut(&gambler_id) {
                    let _ = turn(gambler_mut, direction as u8);
                }
            }
        }

        // C `if (ticker > dat->memcleartimer) { mem_erase_driver(cn, 7);
        // dat->memcleartimer = ticker + TICKS*60*60*12; }`
        // (`teufel.c:1425-1428`).
        let tick = self.tick.0;
        if tick > data.memcleartimer {
            if let Some(gambler) = self.characters.get_mut(&gambler_id) {
                mem_erase_driver(&mut gambler.driver_memory, TEUFELGAMBLER_GREET_MEMORY_SLOT);
            }
            data.memcleartimer = tick + TEUFELGAMBLER_MEMORY_CLEAR_TICKS;
        }

        if let Some(gambler) = self.characters.get_mut(&gambler_id) {
            gambler.driver_state = Some(CharacterDriverState::TeufelGambler(data));
        }
    }

    /// C `teufelgambler_driver`'s `NT_CHAR` branch (`teufel.c:1253-1314`).
    fn teufelgambler_handle_char_message(
        &mut self,
        gambler_id: CharacterId,
        data: &TeufelGambleDriverData,
        message: &CharacterDriverMessage,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let seen_id = CharacterId(message.dat1.max(0) as u32);
        let Some(gambler) = self.characters.get(&gambler_id).cloned() else {
            return;
        };
        let Some(seen) = self.characters.get(&seen_id).cloned() else {
            return;
        };

        // C `if (!char_see_char(cn, co) || cn == co) { remove_message;
        // continue; }` (`teufel.c:1257-1261`).
        if gambler_id == seen_id || !char_see_char(&gambler, &seen, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 16) { remove_message; continue; }`
        // (`teufel.c:1264-1267`).
        if char_dist(&gambler, &seen) > TEUFELGAMBLER_TALK_DISTANCE {
            return;
        }
        // C `if (mem_check_driver(cn, co, 7)) { remove_message; continue;
        // }` (`teufel.c:1270-1273`).
        if mem_check_driver(
            &gambler.driver_memory,
            TEUFELGAMBLER_GREET_MEMORY_SLOT,
            seen_id.0,
        ) {
            return;
        }
        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`teufel.c:1275-1278`).
        if !seen.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `if (dat->nr == 1) { ... } else if (dat->nr == 2) { ... }
        // else if (dat->nr == 3) { ... }` (`teufel.c:1280-1310`).
        let play_word = match data.nr {
            1 => "play",
            2 => "play2",
            3 => "play3",
            _ => "play",
        };
        if !is_demon(seen.sprite) {
            self.npc_say(
                gambler_id,
                &format!(
                    "Oh. A human. Well, no matter I guess. Wanna \u{E0C4}{play_word}\u{E0C0} with me, kid?"
                ),
            );
        } else {
            self.npc_say_bytes(
                gambler_id,
                &format!(
                    "Hello there, {}! Make your bet! Win big! Come on, \u{E0C4}{play_word}\u{E0C0} with me!",
                    seen.name
                ),
            );
        }

        *face_target = Some((i32::from(seen.x), i32::from(seen.y)));
        if let Some(gambler_mut) = self.characters.get_mut(&gambler_id) {
            mem_add_driver(
                &mut gambler_mut.driver_memory,
                TEUFELGAMBLER_GREET_MEMORY_SLOT,
                seen_id.0,
            );
        }
    }

    /// C `teufelgambler_driver`'s `NT_TEXT` branch (`teufel.c:1317-1414`).
    fn teufelgambler_handle_text_message(
        &mut self,
        loader: &mut ZoneLoader,
        gambler_id: CharacterId,
        data: &TeufelGambleDriverData,
        message: &CharacterDriverMessage,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`teufel.c:1320-1323`).
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(gambler) = self.characters.get(&gambler_id).cloned() else {
            return;
        };
        let Some(text) = message.text.as_deref() else {
            return;
        };

        // C `if ((n = analyse_text_driver(...))) { ... }`
        // (`teufel.c:1325-1401`).
        if let TeufelTextOutcome::Recognized(outcome) =
            teufel_analyse_text(self, &gambler, &speaker, text)
        {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            match outcome {
                TextAnalysisOutcome::Said(reply) => {
                    self.npc_quiet_say_bytes(gambler_id, &reply);
                }
                // C's own `case 1:` inside `analyse_text_driver` itself
                // (`teufel.c:338-340`), not `teufelgambler_driver`'s own
                // switch.
                TextAnalysisOutcome::Matched(1) => {
                    self.npc_quiet_say(gambler_id, &format!("I'm {}.", gambler.name));
                }
                // C `if (n > 1) { ... }` (`teufel.c:1327-1400`): codes
                // `2`/`3`/`4` are "bet one/two/five".
                TextAnalysisOutcome::Matched(code @ (2..=4)) => {
                    let bet = match code {
                        2 => 1,
                        3 => 2,
                        _ => 5,
                    };
                    self.teufelgambler_play_round(loader, gambler_id, speaker_id, data.nr, bet);
                }
                TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
            }
        }

        // C `if ((ch[co].flags & CF_GOD) && (ptr = strstr((char
        // *)msg->dat2, "reward: ")) && (n = atoi(ptr + 8))) { ... }`
        // (`teufel.c:1403-1413`) - a sibling `if`, not an `else if`, of
        // the QA-driven block above: it fires independently of whether
        // the QA table matched anything this same tick, matching C
        // exactly (see this module's own doc comment).
        if speaker.flags.contains(CharacterFlags::GOD) {
            if let Some(idx) = text.find("reward: ") {
                let roll = parse_int_atoi(&text[idx + "reward: ".len()..]);
                if roll != 0 {
                    self.teufelgambler_give_reward(
                        loader, gambler_id, speaker_id, data.nr, roll, 5,
                    );
                }
            }
        }
    }

    /// C `teufelgambler_driver`'s betting block (`teufel.c:1336-1400`):
    /// finds and consumes a matching chip stack, rolls three d20s, and
    /// grants a `give_rewardN` prize on a win.
    fn teufelgambler_play_round(
        &mut self,
        loader: &mut ZoneLoader,
        gambler_id: CharacterId,
        player_id: CharacterId,
        gambler_nr: i32,
        cnt: u32,
    ) {
        let chip_template_id = match gambler_nr {
            1 => IID_BRONZECHIP,
            2 => IID_SILVERCHIP,
            _ => IID_GOLDCHIP,
        };
        let chip_offset = match gambler_nr {
            1 => 0,
            3 => 6,
            _ => 12,
        };

        // C `for (n = 30; n < INVENTORYSIZE; n++) { ... if (n <
        // INVENTORYSIZE) { ... } else { say "No chips, no game..."; }`
        // (`teufel.c:1336-1399`).
        let Some(player) = self.characters.get(&player_id) else {
            return;
        };
        let found = player
            .inventory
            .iter()
            .skip(30)
            .filter_map(|slot| *slot)
            .find_map(|item_id| {
                let item = self.items.get(&item_id)?;
                if item.template_id == chip_template_id && chip_count(item) >= cnt {
                    Some(item_id)
                } else {
                    None
                }
            });

        let Some(chip_item_id) = found else {
            self.npc_say(
                gambler_id,
                "No chips, no game (make sure you only have one stack of chips).",
            );
            return;
        };

        let have = self
            .items
            .get(&chip_item_id)
            .map(chip_count)
            .unwrap_or_default();
        if have == cnt {
            self.destroy_item(chip_item_id);
        } else if let Some(item) = self.items.get_mut(&chip_item_id) {
            set_chip_count(item, have - cnt);
            set_chip_data(item, chip_offset);
        }
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.flags.insert(CharacterFlags::ITEMS);
        }

        // C `a = RANDOM(20)+1; b = RANDOM(20)+1; c = RANDOM(20)+1; t = a +
        // b + c;` (`teufel.c:1378-1382`).
        let a = self.roll_legacy_random(20) as i32 + 1;
        let b = self.roll_legacy_random(20) as i32 + 1;
        let c = self.roll_legacy_random(20) as i32 + 1;
        let t = a + b + c;

        if t > 20 && t < 43 {
            self.npc_say(
                gambler_id,
                &format!("Ha! You rolled {a}, {b} and {c}. You lost!"),
            );
            return;
        }
        self.npc_say(
            gambler_id,
            &format!("Oh. You rolled {a}, {b} and {c}. You win!"),
        );
        self.teufelgambler_give_reward(loader, gambler_id, player_id, gambler_nr, t, cnt);
    }

    /// C `give_reward(co, t, cnt)`/`give_reward2(co, t, cnt)`/
    /// `give_reward3(co, t, cnt)` dispatch (`teufel.c:1387-1395,1404-
    /// 1409`) plus the shared item-granting tail every table shares
    /// (`teufel.c:807-823` and its `give_reward2`/`give_reward3` copies).
    fn teufelgambler_give_reward(
        &mut self,
        loader: &mut ZoneLoader,
        gambler_id: CharacterId,
        player_id: CharacterId,
        gambler_nr: i32,
        roll: i32,
        bet: u32,
    ) {
        let outcome = match gambler_nr {
            1 => give_reward1(roll, bet),
            2 => give_reward2(roll, bet),
            _ => give_reward3(roll, bet),
        };
        let Some(outcome) = outcome else {
            // C `if (!ptr || !cnt) { elog(...); log_char(cn, LOG_SYSTEM,
            // 0, "Bug #1778"); return; }` (`teufel.c:802-806`).
            self.queue_system_text(player_id, "Bug #1778");
            return;
        };

        match outcome {
            GambleRewardOutcome::Money(amount) => {
                self.teufelgambler_give_money(player_id, amount);
            }
            GambleRewardOutcome::Items { template, count } => {
                for _ in 0..count.max(1) {
                    let Ok(item) = loader.instantiate_item_template(&template, None) else {
                        // C `if (!in) { elog(...); log_char(cn,
                        // LOG_SYSTEM, 0, "Bug #1779"); return; }`
                        // (`teufel.c:809-813`).
                        self.queue_system_text(player_id, "Bug #1779");
                        return;
                    };
                    let item_name = item.name.clone();
                    let item_id = item.id;
                    self.items.insert(item_id, item);
                    self.recompute_item_requirements(item_id);
                    if !self.give_char_item(player_id, item_id) {
                        self.destroy_item(item_id);
                        self.queue_system_text(
                            player_id,
                            format!(
                                "Seeing that you have no room to store the {item_name}, the Gamber quickly pockets it again. Too bad..."
                            ),
                        );
                    } else {
                        self.npc_say(gambler_id, &format!("The Gambler gives you a {item_name}."));
                    }
                }
            }
        }
    }

    /// C `give_money(cn, val, reason)` (`src/system/tool.c:1460-1474`),
    /// same local-method shape as `world::npc::area34::teufelquest`'s own
    /// `teufelquest_give_money`.
    fn teufelgambler_give_money(&mut self, player_id: CharacterId, amount: u32) {
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.gold = player.gold.saturating_add(amount);
            player.flags.insert(CharacterFlags::ITEMS);
        }
        self.queue_system_text_bytes(player_id, give_money_message(amount));
    }
}

/// C `*(unsigned int *)(it[in].drdata + 0)` (`teufel.c:1338` etc.): the
/// chip stack's count, same byte layout as `ugaris-server`'s
/// `stacks::stack_count` for `StackKind::BronzeChip`/`SilverChip`/
/// `GoldChip` (offset `0`, little-endian `u32`) - reimplemented here
/// rather than shared across the crate boundary.
fn chip_count(item: &Item) -> u32 {
    let mut bytes = [0_u8; 4];
    for (index, byte) in item.driver_data.iter().take(4).enumerate() {
        bytes[index] = *byte;
    }
    u32::from_le_bytes(bytes)
}

fn set_chip_count(item: &mut Item, count: u32) {
    if item.driver_data.len() < 4 {
        item.driver_data.resize(4, 0);
    }
    item.driver_data[0..4].copy_from_slice(&count.to_le_bytes());
}

/// C `set_chip_data(in, off)` (`teufel.c:1211-1231`).
fn set_chip_data(item: &mut Item, off: i32) {
    let count = chip_count(item);
    item.sprite = match count {
        0 | 1 => 53007 + off,
        2 => 53008 + off,
        3 => 53009 + off,
        4 => 53010 + off,
        5 => 53011 + off,
        _ => 53012 + off,
    };
    let plural = if count > 1 { "s" } else { "" };
    item.description = format!("{count} {}{plural}.", item.name);
}
