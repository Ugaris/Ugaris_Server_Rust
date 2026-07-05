//! Clan-raid catacomb request handling, ported from `src/area/13/
//! dungeon.c`'s `create_dungeon`/`enter_dungeon`/`list_dungeon`/
//! `warn_dungeon` (`dungeon.c:1377-1569`) - part of the "Clan system"
//! task in `PORTING_TODO.md`.
//!
//! This ports the *pure decision* slice only: every validation check,
//! error message shape, catacomb-slot selection/eviction rule, and the
//! guard-roster lookup (`get_clan_dungeon`) that a real `dungeonmaster`
//! NPC driver's `attack`/`enter`/`list` commands would need, matching
//! this codebase's established "pure logic first, wiring later"
//! precedent (see `crate::clan`'s own module doc comment for the guard-
//! count accessors this reuses).
//!
//! Deliberately **not** ported here (left for the future NPC-driver
//! wiring slice, see `PORTING_TODO.md`'s Clan system task): the
//! `CDR_DUNGEONMASTER`/`CDR_DUNGEONFIGHTER` driver constants and
//! `CharacterDriverState` variant, the message-loop entry point itself
//! (`dungeonmaster`), taking the creation fee / teleporting the raider
//! (both real `World` mutations, not pure decisions), the `do`/`while`
//! `create_maze`+loop-over-`build_cell` retry that actually spins up the
//! map (`crates/ugaris-server/src/dungeon.rs` already has every builder
//! this needs), and `dungeonfighter`/`dungeon_potion`/`fighter_dead`
//! (the separate combat-adjacent driver).
//!
//! `destroy_dungeon`'s `build_remove`/`build_empty` map-teardown sweep
//! (`dungeon.c:725-786,1343-1364`) *is* ported below as
//! [`World::destroy_dungeon`] (plus its `build_remove_tile`/
//! `build_empty_tile` per-tile helpers) even though it is a real
//! mutation, not a pure decision, because - unlike the rest of this
//! module's remaining gaps - every primitive it needs
//! (`teleport_character_same_area`/`remove_character`/`destroy_item`/
//! `remove_effect_from_map`/`set_item_expire`/`queue_system_text`)
//! already exists on `World`. One real gap remains: C's
//! `build_remove` falls back from four same-area `teleport_char_driver`
//! attempts to `change_area(cn, ch[cn].resta, ch[cn].restx,
//! ch[cn].resty)` (the player's stored rest/recall point, which may be
//! in a *different* area) before finally giving up with `exit_char(cn)`.
//! This codebase runs one area per server process (see `World::area_id`'s
//! own doc comment) with no cross-area transfer yet (`PORTING_TODO.md`'s
//! "Cross-area transfer" P4 task - every existing cross-area path in
//! `crates/ugaris-server` reports "target area server is down" instead of
//! actually moving the character, see `transport.rs`'s
//! `TransportTravelResult::CrossArea`). `build_remove_tile` mirrors that
//! same precedent: if the evicted player's own `rest_area` happens to
//! equal this running area, their rest point is honored as a same-area
//! teleport (exactly what `change_area` would reduce to in that case);
//! otherwise the cross-area case is unreachable here and the player is
//! evicted via `remove_character` exactly like C's `exit_char(cn)`
//! fallback.
//!
//! `struct master_data`'s 9-slot catacomb-tracking arrays are ported as
//! [`DungeonmasterDriverData`], passed by reference rather than stored
//! on any `Character` yet (no `CharacterDriverState` variant exists for
//! it in this slice).

use super::*;
use crate::clan::score_to_level;

/// C's fixed catacomb-grid size (`dungeon.c` implicitly assumes 9
/// 81x81 catacomb slots laid out 3x3 across the area-13 map).
pub const DUNGEON_SLOT_COUNT: usize = 9;

/// C `struct master_data` (`dungeon.c:1366-1375`), minus `dungeonmaster`'s
/// own driver-dispatch bookkeeping (`memcleartimer` is kept since
/// `list_dungeon`/`warn_dungeon` callers may still want it, but nothing
/// in this pure module reads it yet).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DungeonmasterDriverData {
    /// C `target[9]`: the defending clan number for each occupied slot
    /// (`0` = empty).
    pub target: [u16; DUNGEON_SLOT_COUNT],
    /// C `level[9]`: the guard level the catacomb was built at
    /// (`56 + score_to_level(clan_get_training_score(target))`).
    pub level: [i32; DUNGEON_SLOT_COUNT],
    /// C `created[9]`: the tick the catacomb was created (`0` = empty).
    pub created: [u64; DUNGEON_SLOT_COUNT],
    /// C `warning[9]`: the next `warn_dungeon` threshold, in ticks-since-
    /// creation.
    pub warning: [u64; DUNGEON_SLOT_COUNT],
    /// C `owner[9]`: the raider's `ch[].ID` (here, `CharacterId.0`) that
    /// created the catacomb.
    pub owner: [u32; DUNGEON_SLOT_COUNT],
    /// C `created_by_clan[9]`: the raiding clan number.
    pub created_by_clan: [u16; DUNGEON_SLOT_COUNT],
    /// C `memcleartimer`.
    pub memcleartimer: u64,
}

/// C `create_dungeon`'s `say(...)` error branches (`dungeon.c:1384-
/// 1448`), in the same order C checks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DungeonRaidError {
    /// "No clan by that number." (`target < 1 || target >= 32`).
    NoSuchClan,
    /// "You cannot create a clan catacomb, your level is too high (max
    /// 56)."
    LevelTooHigh,
    /// "You are not at war with that clan."
    NotAtWar,
    /// "That clan does not have any jewels you could steal."
    TargetHasNoJewels,
    /// "Your clan does not have enough jewels to mount a raid (your
    /// clan needs to have at least 11 of them)."
    OwnClanLacksJewels,
    /// "This catacomb already exists, please use 'enter %d' instead."
    /// (`slot` is the 1-based catacomb number to report).
    CatacombAlreadyExists { slot: usize },
    /// "Your clan has created a catacomb already, you may not create
    /// another one before the first one has collapsed."
    ClanAlreadyRaiding,
    /// "You have created a catacomb already, you may not create another
    /// one before the first one has collapsed."
    PlayerAlreadyRaiding,
    /// "Sorry, all catacombs are busy. Please try again in %.2f
    /// minutes" (`wait_ticks` is the raw tick count to format).
    AllCatacombsBusy { wait_ticks: i64 },
}

/// A validated `create_dungeon` request, everything a future orchestrator
/// needs to actually spin up the maze via `crates/ugaris-server/src/
/// dungeon.rs`'s `create_maze`/`build_cell` (fee-charging and teleporting
/// the raider still happen at the call site - those are real `World`
/// mutations, not part of this pure decision).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonRaidPlan {
    /// The chosen (possibly evicted) catacomb slot, `0..9`.
    pub slot: usize,
    /// C's hardcoded `fee = 3500` (gold, not centigold).
    pub fee: u32,
    /// `56 + score_to_level(clan_get_training_score(target))`.
    pub level: i32,
    /// The raiding character's own clan (`get_char_clan(co)`).
    pub own_clan: u16,
    /// C's file-scope `xoff`/`yoff` for this slot (`(slot%3)*81+2`,
    /// `(slot/3)*81+2`).
    pub xoff: u16,
    pub yoff: u16,
    /// `get_clan_dungeon(target, 1..=6)`.
    pub warrior: [i32; 6],
    /// `get_clan_dungeon(target, 7..=12)`.
    pub mage: [i32; 6],
    /// `get_clan_dungeon(target, 13..=18)`.
    pub seyan: [i32; 6],
    /// `get_clan_dungeon(target, 19)`.
    pub teleport: i32,
    /// `get_clan_dungeon(target, 20)`.
    pub fake: i32,
    /// `get_clan_dungeon(target, 21)`.
    pub key: i32,
}

/// C `enter_dungeon`'s `say(...)` error branches (`dungeon.c:1517-
/// 1541`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DungeonEnterError {
    /// "Sorry, the target is out of bounds." (`target < 1 || target >
    /// 9`).
    TargetOutOfBounds,
    /// "Sorry, you may not enter this catacomb, it was created for
    /// level %d and below." (`max_level` is the slot's own stored
    /// `level`).
    LevelTooHigh { max_level: i32 },
    /// "You are not at war with that clan."
    NotAtWar,
    /// "Sorry, this catacomb is about to collapse."
    AboutToCollapse,
}

/// A validated `enter_dungeon` request: the raider's own teleport
/// destination plus the remaining-time message value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonEnterPlan {
    pub x: u16,
    pub y: u16,
    /// C's `tmp = get_dungeon_time() - ticker + dat->created[target]`,
    /// used by the caller to format "This catacomb will collapse in
    /// %.2f minutes,".
    pub remaining_ticks: i64,
}

impl World {
    /// C `create_dungeon`'s validation and catacomb-slot-selection logic
    /// (`dungeon.c:1377-1500`), minus taking the fee, building the maze,
    /// teleporting the raider, and the two `add_clanlog` calls - all real
    /// side effects the caller applies on `Ok`.
    ///
    /// `target` is the 1-based clan number to raid (C's own `target`
    /// parameter, checked `1..32` before use). `player_id` is C's `co`
    /// (the player who spoke the "attack" command); C's `cn` (the NPC
    /// itself) has no bearing on this pure decision and is omitted.
    pub fn plan_create_dungeon(
        &mut self,
        player_id: CharacterId,
        target: u16,
        dat: &DungeonmasterDriverData,
    ) -> Result<DungeonRaidPlan, DungeonRaidError> {
        if target < 1 || target >= 32 {
            return Err(DungeonRaidError::NoSuchClan);
        }

        let Some(player) = self.characters.get_mut(&player_id) else {
            return Err(DungeonRaidError::NoSuchClan);
        };
        if player.level > 56 {
            return Err(DungeonRaidError::LevelTooHigh);
        }
        let is_god = player.flags.contains(CharacterFlags::GOD);
        let own_clan = self.clan_registry.get_char_clan(player).unwrap_or(0);
        let owner_id = player.id.0;

        if !self
            .clan_registry
            .relations()
            .can_attack_inside(own_clan, target)
            && !is_god
        {
            return Err(DungeonRaidError::NotAtWar);
        }
        if self.clan_registry.jewel_count(target) < 11 {
            return Err(DungeonRaidError::TargetHasNoJewels);
        }
        if self.clan_registry.jewel_count(own_clan) < 12 {
            return Err(DungeonRaidError::OwnClanLacksJewels);
        }

        for (n, &slot_target) in dat.target.iter().enumerate() {
            if slot_target == target {
                return Err(DungeonRaidError::CatacombAlreadyExists { slot: n + 1 });
            }
            if dat.created_by_clan[n] == own_clan {
                return Err(DungeonRaidError::ClanAlreadyRaiding);
            }
            if dat.owner[n] == owner_id {
                return Err(DungeonRaidError::PlayerAlreadyRaiding);
            }
        }

        let dungeon_time = i64::from(self.settings.dungeon_time);
        let ticker = self.tick.0 as i64;
        let mut best = 0i64;
        let mut bestn = 0usize;
        for (n, &created) in dat.created.iter().enumerate() {
            let tmp = if created != 0 {
                ticker - created as i64
            } else {
                dungeon_time.max(ticker - created as i64)
            };
            if tmp > best {
                best = tmp;
                bestn = n;
            }
        }
        if best < dungeon_time {
            return Err(DungeonRaidError::AllCatacombsBusy {
                wait_ticks: dungeon_time - best,
            });
        }

        let level = 56
            + score_to_level(
                self.clan_registry
                    .identity(target)
                    .map(|identity| identity.economy.training_score)
                    .unwrap_or(0),
            );

        let mut warrior = [0i32; 6];
        let mut mage = [0i32; 6];
        let mut seyan = [0i32; 6];
        let mut teleport = 0;
        let mut fake = 0;
        let mut key = 0;
        for n in 1..22 {
            match n {
                1..=6 => warrior[(n - 1) as usize] = self.clan_registry.get_clan_dungeon(target, n),
                7..=12 => mage[(n - 7) as usize] = self.clan_registry.get_clan_dungeon(target, n),
                13..=18 => {
                    seyan[(n - 13) as usize] = self.clan_registry.get_clan_dungeon(target, n)
                }
                19 => teleport = self.clan_registry.get_clan_dungeon(target, n),
                20 => fake = self.clan_registry.get_clan_dungeon(target, n),
                21 => key = self.clan_registry.get_clan_dungeon(target, n),
                _ => {}
            }
        }

        Ok(DungeonRaidPlan {
            slot: bestn,
            fee: 3500,
            level,
            own_clan,
            xoff: ((bestn % 3) * 81 + 2) as u16,
            yoff: ((bestn / 3) * 81 + 2) as u16,
            warrior,
            mage,
            seyan,
            teleport,
            fake,
            key,
        })
    }

    /// C `enter_dungeon` (`dungeon.c:1517-1541`), minus the final
    /// `teleport_char_driver` call (a real `World` mutation the caller
    /// applies on `Ok`). `target` is C's own 1-based `target` parameter.
    pub fn plan_enter_dungeon(
        &mut self,
        player_id: CharacterId,
        target: i32,
        dat: &DungeonmasterDriverData,
    ) -> Result<DungeonEnterPlan, DungeonEnterError> {
        if target < 1 || target > DUNGEON_SLOT_COUNT as i32 {
            return Err(DungeonEnterError::TargetOutOfBounds);
        }
        let slot = (target - 1) as usize;

        let Some(player) = self.characters.get_mut(&player_id) else {
            return Err(DungeonEnterError::TargetOutOfBounds);
        };
        if player.level > 56 {
            return Err(DungeonEnterError::LevelTooHigh {
                max_level: dat.level[slot],
            });
        }
        let own_clan = self.clan_registry.get_char_clan(player).unwrap_or(0);
        if !self
            .clan_registry
            .relations()
            .can_attack_inside(own_clan, dat.target[slot])
        {
            return Err(DungeonEnterError::NotAtWar);
        }

        let dungeon_time = i64::from(self.settings.dungeon_time);
        let ticker = self.tick.0 as i64;
        let tmp = dungeon_time - ticker + dat.created[slot] as i64;
        if tmp < (TICKS_PER_SECOND as i64) * 60 {
            return Err(DungeonEnterError::AboutToCollapse);
        }

        Ok(DungeonEnterPlan {
            x: ((slot % 3) * 81 + 4) as u16,
            y: ((slot / 3) * 81 + 80) as u16,
            remaining_ticks: tmp,
        })
    }

    /// C `list_dungeon` (`dungeon.c:1544-1557`): one formatted line per
    /// occupied catacomb slot, or a single "No catacombs." line if none
    /// are occupied.
    pub fn list_dungeon_lines(&self, dat: &DungeonmasterDriverData) -> Vec<String> {
        let dungeon_time = f64::from(self.settings.dungeon_time);
        let ticker = self.tick.0 as f64;
        let mut lines = Vec::new();
        for (n, &target) in dat.target.iter().enumerate() {
            if target != 0 {
                let remaining = (dungeon_time - ticker + dat.created[n] as f64)
                    / (TICKS_PER_SECOND as f64 * 60.0);
                lines.push(format!(
                    "Catacomb {}: Clan {}, level {}, remaining time: {:.2} minutes.",
                    n + 1,
                    target,
                    dat.level[n],
                    remaining
                ));
            }
        }
        if lines.is_empty() {
            lines.push("No catacombs.".to_string());
        }
        lines
    }

    /// C `warn_dungeon`'s player-selection loop (`dungeon.c:1559-1569`):
    /// every player character currently standing inside catacomb slot
    /// `nr`'s 81x81 area block. The caller formats and delivers "This
    /// catacomb will collapse in %.2f minutes." to each returned id.
    pub fn characters_in_dungeon_slot(&self, slot: usize) -> Vec<CharacterId> {
        self.characters
            .values()
            .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
            .filter(|character| {
                let cx = (i32::from(character.x) - 2) / 81;
                let cy = (i32::from(character.y) - 2) / 81;
                cx >= 0 && cy >= 0 && (cx + cy * 3) as usize == slot
            })
            .map(|character| character.id)
            .collect()
    }

    /// C `build_remove(x, y)` (`dungeon.c:743-786`): evicts whatever
    /// occupies one map tile - a player (evicted via a same-area
    /// teleport chain, see this module's doc comment for the
    /// `change_area` gap), an NPC (removed outright, matching
    /// `remove_destroy_char`), any item (a takeable item or non-takeable
    /// player body is scattered nearby and destroyed/timed-out, exactly
    /// as C does), and every effect anchored to the tile.
    pub fn build_remove_tile(&mut self, x: usize, y: usize) {
        let character_id = self.map.tile(x, y).and_then(|tile| {
            (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
        });
        if let Some(character_id) = character_id {
            let is_player = self
                .characters
                .get(&character_id)
                .is_some_and(|character| character.flags.contains(CharacterFlags::PLAYER));
            if is_player {
                self.queue_system_text(character_id, "The catacomb collapsed on you.");
                let escaped = self.teleport_character_same_area(character_id, 245, 250, false)
                    || self.teleport_character_same_area(character_id, 240, 250, false)
                    || self.teleport_character_same_area(character_id, 235, 250, false)
                    || self.teleport_character_same_area(character_id, 230, 250, false);
                if !escaped {
                    // C: `change_area(cn, ch[cn].resta, ch[cn].restx,
                    // ch[cn].resty)`, falling back to `exit_char(cn)` on
                    // failure - see this module's doc comment for why
                    // only the same-area case of `change_area` is
                    // reachable here.
                    let rest = self
                        .characters
                        .get(&character_id)
                        .map(|character| (character.rest_area, character.rest_x, character.rest_y));
                    let recalled = matches!(rest, Some((area, _, _)) if area == self.area_id)
                        && rest.is_some_and(|(_, rest_x, rest_y)| {
                            self.teleport_character_same_area(character_id, rest_x, rest_y, false)
                        });
                    if !recalled {
                        self.remove_character(character_id);
                    }
                }
            } else {
                // C `remove_destroy_char(cn)`.
                self.remove_character(character_id);
            }
        }

        let item_id = self
            .map
            .tile(x, y)
            .and_then(|tile| (tile.item != 0).then_some(ItemId(tile.item)));
        if let Some(item_id) = item_id {
            let is_player_body = self
                .items
                .get(&item_id)
                .is_some_and(|item| item.flags.contains(ItemFlags::PLAYERBODY));
            let is_takeable = self
                .items
                .get(&item_id)
                .is_some_and(|item| item.flags.contains(ItemFlags::TAKE));
            if is_player_body {
                let dropped = if let Some(item) = self.items.get_mut(&item_id) {
                    self.map.remove_item_map(item);
                    [(250, 245), (250, 240), (250, 235), (250, 230)]
                        .into_iter()
                        .any(|(dx, dy)| self.map.drop_item(item, dx, dy))
                } else {
                    false
                };
                if dropped {
                    let decay = self.settings.item_decay_time.max(1) as u64;
                    self.set_item_expire(item_id, decay);
                } else {
                    self.destroy_item(item_id);
                }
            } else if is_takeable {
                self.destroy_item(item_id);
            }
        }

        let effect_ids: Vec<u32> = self
            .map
            .tile(x, y)
            .map(|tile| tile.effects)
            .unwrap_or([0; 4])
            .into_iter()
            .filter(|&effect_id| effect_id != 0)
            .map(u32::from)
            .collect();
        for effect_id in effect_ids {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
        }
    }

    /// C `build_empty(x, y)` (`dungeon.c:725-736`): destroys any item
    /// still on the tile (after [`Self::build_remove_tile`] has already
    /// evicted characters/effects) and resets the tile itself to a bare
    /// indoors floor with the catacomb's own 3x3-tiled floor sprite
    /// (`59130 + x%3 + (y%3)*3`).
    pub fn build_empty_tile(&mut self, x: usize, y: usize) {
        let item_id = self
            .map
            .tile(x, y)
            .and_then(|tile| (tile.item != 0).then_some(ItemId(tile.item)));
        if let Some(item_id) = item_id {
            self.destroy_item(item_id);
        }

        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.flags = MapFlags::INDOORS;
            tile.foreground_sprite = 0;
            tile.ground_sprite = 59130 + (x % 3) as u32 + (y % 3) as u32 * 3;
            tile.daylight = 0;
            tile.light = 0;
        }
    }

    /// C `destroy_dungeon(nr)` (`dungeon.c:1343-1364`): tears down the
    /// given 0-based catacomb slot's whole 81x81 map block, first
    /// evicting every occupant/item/effect tile-by-tile
    /// ([`Self::build_remove_tile`]), then resetting every tile to bare
    /// floor ([`Self::build_empty_tile`]) - in that order, exactly
    /// matching C's own two separate sweeps.
    pub fn destroy_dungeon(&mut self, slot: usize) {
        let xf = (slot % 3) * 81 + 2;
        let xt = xf + 80;
        let yf = (slot / 3) * 81 + 2;
        let yt = yf + 80;

        for x in xf..xt {
            for y in yf..yt {
                self.build_remove_tile(x, y);
            }
        }
        for x in xf..xt {
            for y in yf..yt {
                self.build_empty_tile(x, y);
            }
        }
    }
}
