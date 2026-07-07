//! Newbie in-window hint system (`tutorial_ppd`/`tutorial()`,
//! `src/system/player_driver.c:374-711`), invoked from `player_driver`
//! (`:961-963`) at most once every 20 realtime seconds per connected
//! player while hints are enabled (`lostcon_ppd.hints`, ported as
//! `PlayerRuntime::hints_disabled` - inverted polarity, already wired for
//! the `/hints` command). At most one hint fires per call, matching every
//! C branch's own `return` immediately after logging.
//!
//! Same `PlayerRuntime`-can't-be-seen-by-`World` split already
//! established for `world::gatekeeper`/`world::lydia`: the caller
//! (`ugaris-server`'s tick loop, `crates/ugaris-server/src/tutorial.rs`)
//! snapshots [`TutorialPlayerFacts`] for every connected player up front
//! and applies the returned [`TutorialOutcome`]s afterwards.
//!
//! Deviations/gaps (documented, not silent):
//! - The mage-only "Lightning Flash" hint (`battle2`, `:497-505`)
//!   compares `ticker` (a raw tick counter, resets on every process
//!   restart) against `ppd->battle2_last` (stamped with `realtime`, a
//!   wall-clock-seconds-since-a-fixed-2001-01-01-epoch value,
//!   `STARTTIME`) - a C unit-mismatch bug. In practice this means the
//!   hint fires **at most once, ever**, per character: it needs ~150
//!   seconds of raw `ticker` accumulation to first pass, and once
//!   `battle2_last` is stamped with the much larger `realtime` value,
//!   `ticker - battle2_last` goes deeply negative and stays there for any
//!   realistic single-process uptime (`ticker` would need over a year of
//!   continuous uptime without a restart to catch up). This codebase has
//!   no equivalent "wall-clock seconds since a fixed distant epoch"
//!   counter distinct from "seconds since world start", so replaying the
//!   literal unit mismatch isn't meaningful here; the practical effect -
//!   fires once ever, then never again - is reproduced directly via
//!   `facts.ppd.battle2_cnt == 0` instead.
//! - `dlog(cn, in, "took torch from tutorial")` (`:552`) is a server-
//!   logfile-only call with no client-visible effect; dropped, matching
//!   the precedent set by every other bare `charlog`/`dlog` call already
//!   dropped elsewhere in this port (e.g. `world/npc/area1/robber.rs`).
//! - C's "no torch found, one is created for the player" sub-branches
//!   (`:536-561`) unconditionally reset `torch_last`/`timer` but only
//!   bump `torch_cnt` if the usual `TF_TIMEOUT` gate has separately
//!   elapsed - unlike every other hint, where all three updates are
//!   gated together. [`TutorialOutcome::fired`] always signals "reset
//!   `last`/`timer` for this hint"; the one exception (only bump `cnt`
//!   when the gate held) is applied by the caller, which still has the
//!   pre-update `torch_last` value to re-check (see
//!   `crates/ugaris-server/src/tutorial.rs::apply_tutorial_outcomes`).

use std::collections::HashMap;

use crate::area_section::section_at;
use crate::direction::Direction;
use crate::drvlib::offset2dx;
use crate::item_driver::{bare_value, drdata, raise_cost, IID_AREA1_WOODPOTION};
use crate::player::TutorialPpd;
use crate::see::check_light;
use crate::world::*;

/// C `#define TF_TIMEOUT (60 * 60)` (`player_driver.c:373`).
const TF_TIMEOUT: u64 = 60 * 60;

/// Which hint fired, for the caller to bump the matching `PlayerRuntime`
/// counter. See the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialHintKind {
    Welcome,
    Lydia,
    Thief,
    Torch,
    Battle,
    Battle2,
    Shop,
    Chest,
    Citem,
    Raise,
    Potion,
    Shift,
    Ctrl,
    Left,
    Chat,
    Chat2,
    Raise2,
}

/// Per-player facts [`World::process_tutorial_hints`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see. See the
/// module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorialPlayerFacts {
    /// `PlayerRuntime::hints_disabled` (C `!lostcon_ppd.hints`).
    pub hints_disabled: bool,
    /// C `ch[cn].login_time`, in the same realtime-seconds clock as
    /// every other field here.
    pub login_realtime_seconds: u64,
    pub ppd: TutorialPpd,
    /// `PlayerRuntime::area1_lydia_state()`.
    pub area1_lydia_state: i32,
    /// `PlayerRuntime::area1_lydia_seen_timer()`.
    pub area1_lydia_seen_timer_realtime_seconds: u64,
}

/// A hint that fired (or a pending `citem_start` update), for the caller
/// to apply back to `PlayerRuntime`. See the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorialOutcome {
    pub character_id: CharacterId,
    pub fired: Option<TutorialHintKind>,
    /// `Some(new_value)` whenever C's `ppd->citem_start` needs to change
    /// (`player_driver.c:565-575`) - independent of whether a hint fired
    /// this call.
    pub citem_start: Option<u64>,
}

/// C `dir1[9]`/`dir2[9]` (`player_driver.c:432-433,446-447`): compass name
/// plus plain-English direction phrase for `offset2dx`'s result. C's
/// index `0` ("unknown") only happens for a zero offset (`offset2dx`
/// returning `None` here).
fn tutorial_direction_text(direction: Option<Direction>) -> (&'static str, &'static str) {
    match direction {
        None => ("unknown", "unknown"),
        Some(Direction::Right) => ("south-east", "down and right"),
        Some(Direction::RightDown) => ("south", "down"),
        Some(Direction::Down) => ("south-west", "down and left"),
        Some(Direction::LeftDown) => ("west", "left"),
        Some(Direction::Left) => ("north-west", "up and left"),
        Some(Direction::LeftUp) => ("north", "up"),
        Some(Direction::Up) => ("north-east", "up and right"),
        Some(Direction::RightUp) => ("east", "right"),
    }
}

/// C's `raise` hint's skill-choice cascade (`player_driver.c:668-712`):
/// the first skill (in this exact order) the player has enough spare
/// experience to raise and is a sensible next raise for.
fn tutorial_raise_skill_name(
    character: &Character,
    items: &HashMap<ItemId, Item>,
) -> Option<&'static str> {
    let exp_available = i64::from(character.exp) - i64::from(character.exp_used);
    let warrior = character.flags.contains(CharacterFlags::WARRIOR);
    let mage = character.flags.contains(CharacterFlags::MAGE);

    let rhand_item = character
        .inventory
        .get(worn_slot::RIGHT_HAND)
        .copied()
        .flatten()
        .and_then(|item_id| items.get(&item_id));

    let sword = bare_value(character, CharacterValue::Sword as usize);
    let two_hand = bare_value(character, CharacterValue::TwoHand as usize);
    let attack = bare_value(character, CharacterValue::Attack as usize);
    let parry = bare_value(character, CharacterValue::Parry as usize);
    let bless = bare_value(character, CharacterValue::Bless as usize);
    let flash = bare_value(character, CharacterValue::Flash as usize);
    let magic_shield = bare_value(character, CharacterValue::MagicShield as usize);

    if warrior
        && exp_available >= i64::from(raise_cost(CharacterValue::Sword as usize, sword, false))
        && rhand_item.is_some_and(|item| item.flags.contains(ItemFlags::SWORD))
        && sword <= attack
        && sword <= parry
    {
        return Some("Sword");
    }
    if warrior
        && exp_available
            >= i64::from(raise_cost(
                CharacterValue::TwoHand as usize,
                two_hand,
                false,
            ))
        && rhand_item.is_some_and(|item| item.flags.contains(ItemFlags::TWOHAND))
        && two_hand <= attack
        && two_hand <= parry
    {
        return Some("Two-Handed");
    }
    if warrior
        && exp_available >= i64::from(raise_cost(CharacterValue::Attack as usize, attack, false))
        && attack <= sword
        && attack <= parry
    {
        return Some("Attack");
    }
    if warrior
        && exp_available >= i64::from(raise_cost(CharacterValue::Parry as usize, parry, false))
        && parry <= attack
        && parry <= sword
    {
        return Some("Parry");
    }
    if mage
        && exp_available >= i64::from(raise_cost(CharacterValue::Bless as usize, bless, false))
        && bless == 1
    {
        return Some("Bless");
    }
    if mage
        && exp_available >= i64::from(raise_cost(CharacterValue::Flash as usize, flash, false))
        && flash <= magic_shield
    {
        return Some("Lightning Flash");
    }
    if mage
        && exp_available
            >= i64::from(raise_cost(
                CharacterValue::MagicShield as usize,
                magic_shield,
                false,
            ))
        && magic_shield <= flash
    {
        return Some("Magic Shield");
    }
    None
}

/// C's generic "todays hint" potion F-key scan (`player_driver.c:717-
/// 735`): the first usable-potion column (F1-F4, the 4-column inventory
/// grid starting at slot 30) not already claimed by some other usable
/// item earlier in that column.
fn tutorial_potion_fkey(character: &Character, items: &HashMap<ItemId, Item>) -> Option<u8> {
    let mut column_dead = [false; 4];
    for n in INVENTORY_START_INVENTORY..INVENTORY_SIZE {
        let column = (n - INVENTORY_START_INVENTORY) % 4;
        let Some(item) = character
            .inventory
            .get(n)
            .copied()
            .flatten()
            .and_then(|item_id| items.get(&item_id))
        else {
            continue;
        };
        if !column_dead[column] && item.driver == IDR_POTION && drdata(item, 1) != 0 {
            return Some((column + 1) as u8);
        }
        if item.flags.contains(ItemFlags::USE) {
            column_dead[column] = true;
        }
    }
    None
}

impl World {
    /// C `tutorial()`'s per-tick body, run once for every currently-
    /// connected player whose own `ppd->timer` throttle
    /// (`player_driver.c:961`) has elapsed. See the module doc comment.
    pub fn process_tutorial_hints(
        &mut self,
        player_facts: &HashMap<CharacterId, TutorialPlayerFacts>,
        zone_loader: &mut ZoneLoader,
        area_id: u16,
        now: u64,
    ) -> Vec<TutorialOutcome> {
        let mut outcomes = Vec::new();
        for (&character_id, facts) in player_facts {
            if facts.hints_disabled {
                continue;
            }
            if now.saturating_sub(facts.ppd.timer_realtime_seconds) <= 20 {
                continue;
            }
            let Some(outcome) =
                self.process_tutorial_hint(character_id, facts, zone_loader, area_id, now)
            else {
                continue;
            };
            if outcome.fired.is_some() || outcome.citem_start.is_some() {
                outcomes.push(outcome);
            }
        }
        outcomes
    }

    fn process_tutorial_hint(
        &mut self,
        character_id: CharacterId,
        facts: &TutorialPlayerFacts,
        zone_loader: &mut ZoneLoader,
        area_id: u16,
        now: u64,
    ) -> Option<TutorialOutcome> {
        let character = self.characters.get(&character_id).cloned()?;
        let ppd = &facts.ppd;
        let no_fire = |citem_start| TutorialOutcome {
            character_id,
            fired: None,
            citem_start,
        };
        let fired = |hint, citem_start| TutorialOutcome {
            character_id,
            fired: Some(hint),
            citem_start,
        };

        // C `player_driver.c:408-419`: newbie greeting.
        if now.saturating_sub(facts.login_realtime_seconds) < 20
            && ppd.welcome_cnt < 3
            && now.saturating_sub(ppd.welcome_last_realtime_seconds) > TF_TIMEOUT
        {
            self.queue_system_text(
                character_id,
                format!(
                    "#Welcome to Ugaris, {}. This is the help window. To remove it, press \
                     ESCAPE.$$You can access the client help facility by pressing F11.$$Should \
                     you ever require human help, type '/info help' and press RETURN.",
                    character.name
                ),
            );
            return Some(fired(TutorialHintKind::Welcome, None));
        }

        // C `player_driver.c:421-438`: how to find the first quest giver.
        if character.level < 3
            && facts.area1_lydia_state == 0
            && ppd.lydia_cnt < 10
            && now.saturating_sub(ppd.lydia_last_realtime_seconds) > 60
        {
            let (dir1, dir2) = tutorial_direction_text(offset2dx(
                i32::from(character.x),
                i32::from(character.y),
                100,
                129,
            ));
            self.queue_system_text(
                character_id,
                format!(
                    "#Some time ago, James asked you to help Lydia. You can find her {dir1} \
                     ({dir2}) of you.$To walk, left-click on the ground where you wish to \
                     go.$$Should you need human help, type '/info help' and press RETURN. To \
                     remove this window, press ESCAPE."
                ),
            );
            return Some(fired(TutorialHintKind::Lydia, None));
        }

        // C `player_driver.c:440-458`: how to find the quest (thieves).
        if character.level < 4
            && facts.area1_lydia_state == 4
            && now.saturating_sub(facts.area1_lydia_seen_timer_realtime_seconds) > 60
            && section_at(area_id, usize::from(character.x), usize::from(character.y))
                .map(|section| section.id)
                != Some(45)
            && !self.character_has_template_id(character_id, IID_AREA1_WOODPOTION)
            && ppd.thief_cnt < 10
            && now.saturating_sub(ppd.thief_last_realtime_seconds) > 60
        {
            let (dir1, dir2) = tutorial_direction_text(offset2dx(
                i32::from(character.x),
                i32::from(character.y),
                91,
                156,
            ));
            self.queue_system_text(
                character_id,
                format!(
                    "#Lydia asked you to find the thieves who stole her potion. She hinted that \
                     you can find them {dir1} ({dir2}) of you.$To walk, left-click on the \
                     ground where you wish to go.$$Should you need human help, type '/info \
                     help' and press RETURN. To remove this window, press ESCAPE."
                ),
            );
            return Some(fired(TutorialHintKind::Thief, None));
        }

        // C `player_driver.c:460-563`: it's dark, torch hints.
        if let Some(outcome) =
            self.tutorial_torch_hint(character_id, &character, ppd, zone_loader, now)
        {
            return Some(outcome);
        }

        // C `player_driver.c:565-609`: battle-prep hints, area 1 only.
        if area_id == 1 {
            let section_id =
                section_at(area_id, usize::from(character.x), usize::from(character.y))
                    .map(|section| section.id)
                    .unwrap_or(0);
            let outside_village = !(55..=57).contains(&section_id);

            if character.flags.contains(CharacterFlags::WARRIOR) {
                if outside_village
                    && ppd.battle_cnt < 3
                    && now.saturating_sub(ppd.battle_last_realtime_seconds) > TF_TIMEOUT
                {
                    self.queue_system_text(
                        character_id,
                        "#You've left the village, and things might get dangerous. If you get \
                         into a fight, it might be wise to use the skill 'Warcry'. Hold down \
                         ALT and press 8 to do that.",
                    );
                    return Some(fired(TutorialHintKind::Battle, None));
                }
            } else {
                if outside_village
                    && ppd.battle_cnt < 3
                    && now.saturating_sub(ppd.battle_last_realtime_seconds) > TF_TIMEOUT
                    && (may_add_spell(&character, &self.items, IDR_BLESS, self.tick.0 as u32)
                        .is_some()
                        || character.lifeshield < POWERSCALE * 5)
                {
                    self.queue_system_text(
                        character_id,
                        "#You've left the village, and things might get dangerous. You'd better \
                         prepare yourself by casting the spells 'Bless' and 'Magic Shield'. \
                         Hold down ALT and press first 6 and then 5.",
                    );
                    return Some(fired(TutorialHintKind::Battle, None));
                }
                // Documented simplification of C's `ticker`/`realtime`
                // unit-mismatch bug - see the module doc comment.
                if outside_village && ppd.battle2_cnt < 1 {
                    self.queue_system_text(
                        character_id,
                        "#When you get into a fight, remember that mages rely on spells. A \
                         good spell in close ranged combat is 'Lightning Flash' - use ALT-3 to \
                         cast it.",
                    );
                    return Some(fired(TutorialHintKind::Battle2, None));
                }
            }
        }

        // C `player_driver.c:611-620`: shopping tips.
        if character.merchant.is_some()
            && ppd.shop_cnt < 3
            && now.saturating_sub(ppd.shop_last_realtime_seconds) > TF_TIMEOUT
        {
            self.queue_system_text(
                character_id,
                "#You've opened a shop window. The items you can buy are shown in the bottom \
                 left window. To buy anything, left-click on it. To find out what these items \
                 are, right-click on them.$Note that you'll sell any of your items if you \
                 left-click on them now.",
            );
            return Some(fired(TutorialHintKind::Shop, None));
        }

        // C `player_driver.c:622-633`: how to use chests.
        if area_id == 1
            && ((character.x >= 74
                && character.x <= 78
                && character.y >= 148
                && character.y <= 152)
                || (character.x >= 196
                    && character.x <= 201
                    && character.y >= 160
                    && character.y <= 166))
            && ppd.chest_cnt < 3
            && now.saturating_sub(ppd.chest_last_realtime_seconds) > TF_TIMEOUT
        {
            self.queue_system_text(
                character_id,
                "#Do you see that chest? To search it, hold down SHIFT and left-click on it. \
                 If you do not have the right key, go through the building again and be sure \
                 to search all bodies.",
            );
            return Some(fired(TutorialHintKind::Chest, None));
        }

        // C `player_driver.c:635-649`: cursor-item-held-too-long hint.
        // `citem_start` tracking is independent of whether the hint
        // fires, and (unlike every other hint) must survive into every
        // later branch's outcome - see the module doc comment.
        let mut citem_start_update = None;
        if character.cursor_item.is_some() {
            let citem_start = if ppd.citem_start_realtime_seconds == 0 {
                citem_start_update = Some(now);
                now
            } else {
                ppd.citem_start_realtime_seconds
            };
            if now.saturating_sub(citem_start) > 30
                && ppd.citem_cnt < 3
                && now.saturating_sub(ppd.citem_last_realtime_seconds) > TF_TIMEOUT
            {
                self.queue_system_text(
                    character_id,
                    "#You've been carrying that item on your mouse cursor for quite a while \
                     now. Hold down SHIFT and click on the ground to drop it, or hold down \
                     SHIFT and click in your inventory to keep it.",
                );
                return Some(fired(TutorialHintKind::Citem, citem_start_update));
            }
        } else if ppd.citem_start_realtime_seconds != 0 {
            citem_start_update = Some(0);
        }

        // C `player_driver.c:651-712`: skill-raise-available hint.
        if ppd.raise_cnt < 3 && now.saturating_sub(ppd.raise_last_realtime_seconds) > TF_TIMEOUT {
            if let Some(skill_name) = tutorial_raise_skill_name(&character, &self.items) {
                self.queue_system_text(
                    character_id,
                    format!(
                        "#You've accumulated enough experience to raise a skill. You'll find \
                         some blue orbs in the bottom left window. Left-click on the one next \
                         to '{skill_name}' to raise that skill."
                    ),
                );
                return Some(fired(TutorialHintKind::Raise, citem_start_update));
            }
        }

        // C `player_driver.c:714-880`: generic "todays hint" tail, only
        // once nothing has fired for 3 minutes.
        if now.saturating_sub(ppd.timer_realtime_seconds) > 180 {
            if ppd.potion_cnt < 3
                && now.saturating_sub(ppd.potion_last_realtime_seconds) > TF_TIMEOUT
            {
                if let Some(f_key) = tutorial_potion_fkey(&character, &self.items) {
                    self.queue_system_text(
                        character_id,
                        format!(
                            "#Todays Hint:$$You should always watch your Hitpoints by looking \
                             at the small red line below your character's name. If they get \
                             too low, use a healing potion, either by left-clicking on it, or \
                             by pressing F{f_key}.$Note that the F-key is assigned to first \
                             usable item in that column in your inventory, not to the item \
                             itself."
                        ),
                    );
                    return Some(fired(TutorialHintKind::Potion, citem_start_update));
                }
            }
            if ppd.shift_cnt < 3 && now.saturating_sub(ppd.shift_last_realtime_seconds) > TF_TIMEOUT
            {
                self.queue_system_text(
                    character_id,
                    "#Todays Hint:$$As a general rule, anything that deals with items requires \
                     you to hold down SHIFT. But there is one exception: To use an item you \
                     have in your inventory or equipment field, you left-click on it without \
                     holding SHIFT.",
                );
                return Some(fired(TutorialHintKind::Shift, citem_start_update));
            }
            if ppd.ctrl_cnt < 3 && now.saturating_sub(ppd.ctrl_last_realtime_seconds) > TF_TIMEOUT {
                self.queue_system_text(
                    character_id,
                    "#Todays Hint:$$Anything that deals with characters requires you to hold \
                     down CTRL. To look at another character, hold down CTRL and right-click \
                     on that character. To attack him instead, hold down CTRL and left-click.",
                );
                return Some(fired(TutorialHintKind::Ctrl, citem_start_update));
            }
            if ppd.left_cnt < 3 && now.saturating_sub(ppd.left_last_realtime_seconds) > TF_TIMEOUT {
                self.queue_system_text(
                    character_id,
                    "#Todays Hint:$$Clicking the left mouse button always initiates an action, \
                     while right-clicking merely looks at the item or character.",
                );
                return Some(fired(TutorialHintKind::Left, citem_start_update));
            }
            if ppd.chat_cnt < 3 && now.saturating_sub(ppd.chat_last_realtime_seconds) > TF_TIMEOUT {
                self.queue_system_text(
                    character_id,
                    "#Todays Hint:$$To talk with those you see on your screen, just type what \
                     you want to say and press RETURN. Some things you hear contain words in \
                     blue letters. You can click on those to use these texts as an answer.",
                );
                return Some(fired(TutorialHintKind::Chat, citem_start_update));
            }
            if ppd.chat2_cnt < 3 && now.saturating_sub(ppd.chat2_last_realtime_seconds) > TF_TIMEOUT
            {
                self.queue_system_text(
                    character_id,
                    "#Todays Hint:$$If you wish to talk to all players in the game, you have to \
                     use the chat system. It is divided into different channels, numbered 1 to \
                     30. Type /channels to get a list of these channels. Use /join 2, to join, \
                     for example, chat channel 2. To talk in that channel, use /c2 hello.",
                );
                return Some(fired(TutorialHintKind::Chat2, citem_start_update));
            }
            if ppd.raise2_cnt < 3
                && now.saturating_sub(ppd.raise2_last_realtime_seconds) > TF_TIMEOUT
            {
                let message = if character.flags.contains(CharacterFlags::WARRIOR) {
                    "#Todays Hint:$$A good strategy to raise a warrior is to keep your main \
                     weapon skill (Sword or Two-Handed), Attack and Parry close together. Also, \
                     do not neglect Tactics and Immunity."
                } else {
                    "#Todays Hint:$$A good strategy to raise a mage is to concentrate on \
                     Lightning Flash, Magic Shield, Dagger (or Staff) and Immunity at first."
                };
                self.queue_system_text(character_id, message);
                return Some(fired(TutorialHintKind::Raise2, citem_start_update));
            }
        }

        Some(no_fire(citem_start_update))
    }

    /// C `player_driver.c:460-563`: the "it's dark" torch sub-system.
    /// Returns `None` if the tile is bright enough or the overall
    /// `torch_cnt < 5` budget is spent (matching C falling through to the
    /// next check without firing).
    fn tutorial_torch_hint(
        &mut self,
        character_id: CharacterId,
        character: &Character,
        ppd: &TutorialPpd,
        zone_loader: &mut ZoneLoader,
        now: u64,
    ) -> Option<TutorialOutcome> {
        let tile = self
            .map
            .tile(usize::from(character.x), usize::from(character.y))?;
        if check_light(tile, self.date.daylight) >= 8 {
            return None;
        }
        if ppd.torch_cnt >= 5 {
            return None;
        }

        let lhand_item = character
            .inventory
            .get(worn_slot::LEFT_HAND)
            .copied()
            .flatten();
        let lhand_is_torch = lhand_item
            .and_then(|item_id| self.items.get(&item_id))
            .is_some_and(|item| item.driver == IDR_TORCH);

        if lhand_is_torch {
            if now.saturating_sub(ppd.torch_last_realtime_seconds) > TF_TIMEOUT {
                self.queue_system_text(
                    character_id,
                    "#It's pretty dark, isn't it? Why don't you light the torch you're holding \
                     by left-clicking on it?$You can use any item by left-clicking on it.",
                );
                self.queue_player_special(character_id, 0, 5, 0);
                return Some(TutorialOutcome {
                    character_id,
                    fired: Some(TutorialHintKind::Torch),
                    citem_start: None,
                });
            }
            return None;
        }

        let has_inventory_torch = character
            .inventory
            .iter()
            .skip(INVENTORY_START_INVENTORY)
            .flatten()
            .any(|&item_id| {
                self.items
                    .get(&item_id)
                    .is_some_and(|item| item.driver == IDR_TORCH)
            });

        if has_inventory_torch {
            if now.saturating_sub(ppd.torch_last_realtime_seconds) > TF_TIMEOUT {
                self.queue_system_text(
                    character_id,
                    "#It's pretty dark, isn't it? Why don't you equip that torch you have in \
                     your inventory and light it?$To equip the torch, hold down SHIFT, \
                     left-click on it, then left-click on the torch slot.$To light the torch, \
                     left-click on it without holding SHIFT.$If you have trouble finding a \
                     torch in your inventory, right-click on the items there to read their \
                     descriptions.",
                );
                self.queue_player_special(character_id, 0, 17, 0);
                return Some(TutorialOutcome {
                    character_id,
                    fired: Some(TutorialHintKind::Torch),
                    citem_start: None,
                });
            }
            return None;
        }

        // C `player_driver.c:536-561`: no torch anywhere - create one.
        // `torch_last`/`timer` reset unconditionally; only `torch_cnt`'s
        // bump is gated by `TF_TIMEOUT` (applied by the caller, which
        // still has the pre-update `torch_last` to re-check).
        let rhand_blocks_lhand = character
            .inventory
            .get(worn_slot::RIGHT_HAND)
            .copied()
            .flatten()
            .and_then(|item_id| self.items.get(&item_id))
            .is_some_and(|item| item.flags.contains(ItemFlags::WNTWOHANDED));

        if lhand_item.is_none() && !rhand_blocks_lhand {
            let item = zone_loader
                .instantiate_item_template("torch", Some(character_id))
                .ok()?;
            let item_id = item.id;
            self.items.insert(item_id, item);
            if let Some(character_mut) = self.characters.get_mut(&character_id) {
                character_mut.inventory[worn_slot::LEFT_HAND] = Some(item_id);
            }
            self.update_character(character_id);
            self.queue_system_text(
                character_id,
                "#There you are, standing in the darkness, and no torch around. Well, I've \
                 just created one for you. It's there. Left-click on it to light it. But \
                 don't expect me to do this all the time.",
            );
            self.queue_player_special(character_id, 0, 5, 0);
            return Some(TutorialOutcome {
                character_id,
                fired: Some(TutorialHintKind::Torch),
                citem_start: None,
            });
        }

        if character.cursor_item.is_none() {
            let item = zone_loader
                .instantiate_item_template("torch", Some(character_id))
                .ok()?;
            let item_id = item.id;
            self.items.insert(item_id, item);
            if let Some(character_mut) = self.characters.get_mut(&character_id) {
                character_mut.cursor_item = Some(item_id);
            }
            self.update_character(character_id);
            self.queue_system_text(
                character_id,
                "#There you are, standing in the darkness, and no torch around. Well, I've \
                 just created one for you. It there, on your mouse cursor. Hold down SHIFT \
                 and left-click on the torch slot. Then left-click on it without holding SHIFT \
                 to light it. But don't expect me to do this all the time.",
            );
            self.queue_player_special(character_id, 0, 5, 0);
            return Some(TutorialOutcome {
                character_id,
                fired: Some(TutorialHintKind::Torch),
                citem_start: None,
            });
        }

        None
    }
}
