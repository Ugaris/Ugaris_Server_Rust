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
//! already exists on `World`. `build_remove` falls back from four
//! same-area `teleport_char_driver` attempts to `change_area(cn,
//! ch[cn].resta, ch[cn].restx, ch[cn].resty)` (the player's stored
//! rest/recall point, which may be in a *different* area) before
//! finally giving up with `exit_char(cn)`. `build_remove_tile` matches
//! this: if the evicted player's own `rest_area` equals this running
//! area, their rest point is honored as a same-area teleport (exactly
//! what `change_area` would reduce to in that case); otherwise a
//! [`DungeonEvictionTransfer`] is queued for `ugaris-server`'s
//! `world_events.rs::apply_dungeon_eviction_transfers` to hand off to the
//! shared `attempt_cross_area_transfer` helper (`World` has no DB handle
//! or `ServerRuntime` of its own, same reason `world/jail.rs`'s
//! `JailCrossAreaTransfer` is deferred) - matching C's `change_area`
//! call exactly; the player is only evicted via `remove_character`
//! (C's `exit_char(cn)` fallback) if that hand-off itself fails.
//!
//! `struct master_data`'s 9-slot catacomb-tracking arrays are ported as
//! [`DungeonmasterDriverData`], passed by reference rather than stored
//! on any `Character` yet (no `CharacterDriverState` variant exists for
//! it in this slice).

use super::clanclerk::parse_int_atoi;
use super::*;
use crate::character_driver::{
    analyse_text_qa, mem_add_driver, mem_check_driver, mem_erase_driver, TextAnalysisOutcome,
    CDR_DUNGEONMASTER, DUNGEONMASTER_QA, NTID_DUNGEON,
};
use crate::clan::score_to_level;

pub use crate::character_driver::{DungeonmasterDriverData, DUNGEON_SLOT_COUNT};

/// A `build_remove_tile` evicted-player rescue whose `rest_area` differs
/// from this area server's own `area_id` - queued for `ugaris-server`'s
/// `world_events.rs::apply_dungeon_eviction_transfers` since `World` has
/// no DB handle or `ServerRuntime` to perform the `change_area` hand-off
/// itself. See the module doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonEvictionTransfer {
    pub character_id: CharacterId,
    pub target_area: u16,
    pub target_x: u16,
    pub target_y: u16,
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

/// A validated, fee-already-charged `create_dungeon` raid, queued by
/// [`World::dungeonmaster_handle_attack_command`] for `ugaris-server` to
/// resolve via the do-while `create_maze`+`build_cell` retry loop
/// (`dungeon.c:1500-1503`, both builders live in `crates/ugaris-server/
/// src/dungeon.rs`) since that needs `ZoneLoader`/`ServerRuntime` access
/// `World` doesn't have - same "pure decision in `World`, I/O-heavy
/// application in `ugaris-server`" split as every other `*Event`/
/// `*Request` queue in this codebase (see `world::clanmaster::
/// ClanmasterEvent`'s doc comment). By the time this is queued, the fee
/// has already been taken and `DungeonmasterDriverData`'s slot fields
/// have already been updated - only the actual map-building side effect
/// (plus the final "collapse in" message, teleport, and the two
/// `add_clanlog` calls) remains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonRaidBuildRequest {
    /// The `CDR_DUNGEONMASTER` NPC that received the `attack` command -
    /// C's own `say(cn, ...)` caller for the final "collapse in" message.
    pub dungeonmaster_id: CharacterId,
    pub player_id: CharacterId,
    /// The defending clan number (C's own `target` parameter).
    pub target_clan: u16,
    /// The raiding player's own clan (`get_char_clan(co)`).
    pub own_clan: u16,
    pub slot: usize,
    pub xoff: u16,
    pub yoff: u16,
    pub level: i32,
    pub warrior: [i32; 6],
    pub mage: [i32; 6],
    pub seyan: [i32; 6],
    pub teleport: i32,
    pub fake: i32,
    pub key: i32,
}

/// A resolved (jewels-actually-changed-hands) catacomb raid, queued by
/// [`World::resolve_dungeon_door_first_solve`] for `ugaris-server` to
/// write the two `add_clanlog` entries (`clan.c:1368-1371`) that need a
/// DB handle `World` doesn't have - same pure-decision/async-I/O split as
/// [`DungeonRaidBuildRequest`]. Only queued when `stolen > 0` (C's own
/// `add_clanlog` calls live inside the `if (cnt > 0)` block).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonJewelStealEvent {
    /// The winning raider (C's own `cID`/`ch[cn].name` log-message
    /// operand).
    pub player_id: CharacterId,
    /// The raided/defending clan (`cnr`).
    pub defender_clan: u16,
    /// The raider's own clan (`onr`).
    pub attacker_clan: u16,
    /// Jewels moved from the defender's to the attacker's treasury.
    pub stolen: i32,
}

impl World {
    /// Drains every [`DungeonRaidBuildRequest`] queued this tick.
    pub fn drain_pending_dungeon_raid_builds(&mut self) -> Vec<DungeonRaidBuildRequest> {
        std::mem::take(&mut self.pending_dungeon_raid_builds)
    }

    /// Drains every [`DungeonJewelStealEvent`] queued this tick.
    pub fn drain_pending_dungeon_jewel_steals(&mut self) -> Vec<DungeonJewelStealEvent> {
        std::mem::take(&mut self.pending_dungeon_jewel_steals)
    }

    /// Drains every cross-area `build_remove_tile` eviction hand-off
    /// queued this tick - see [`DungeonEvictionTransfer`].
    pub fn drain_pending_dungeon_eviction_transfers(&mut self) -> Vec<DungeonEvictionTransfer> {
        std::mem::take(&mut self.pending_dungeon_eviction_transfers)
    }

    /// C `dungeondoor`'s `first_solve` block (`area/13/dungeon.c:1855-
    /// 1891`): the jewel-steal economy mutation (via `clan.c:1343-1372`'s
    /// `'J'` chat-channel handler, applied directly here since this is a
    /// single-process-per-area server with no master/slave IPC split),
    /// the winner's "You won..." feedback, the catacomb-collapsing
    /// broadcast to every player still standing in the slot, and the
    /// `NT_NPC`/`NTID_DUNGEON` notify to every live `CDR_DUNGEONMASTER`
    /// NPC (consumed by [`World::process_dungeonmaster_messages`], which
    /// resets that slot's tracking fields). `cnr` is the door's own
    /// stored defending clan number; `nr` is the 0-based catacomb slot
    /// (both already computed by `item_driver::area13_dungeon::
    /// dungeon_door_driver`). Called once per catacomb, only when
    /// `first_solve` is true.
    ///
    /// C's two early-return guards ("You're not supposed to be here." for
    /// a non-clan-member winner; "You can't steal jewels while your own
    /// clan has less than 12 of them.") skip the broadcast/notify loop
    /// entirely (`dungeon.c:1857-1864` `return`s before reaching the
    /// `for` loop at 1881); both are unreachable in practice since only
    /// clan members can be inside a catacomb raid in the first place, but
    /// are preserved verbatim for parity.
    pub fn resolve_dungeon_door_first_solve(
        &mut self,
        character_id: CharacterId,
        cnr: u32,
        nr: u8,
    ) {
        let cnr = cnr as u16;
        let onr = {
            let Some(character) = self.characters.get_mut(&character_id) else {
                return;
            };
            self.clan_registry.get_char_clan(character)
        };
        let Some(onr) = onr else {
            self.queue_system_text(character_id, "You're not supposed to be here.");
            return;
        };
        if self.clan_registry.jewel_count(onr) < 12 {
            self.queue_system_text(
                character_id,
                "You can't steal jewels while your own clan has less than 12 of them.",
            );
            return;
        }

        let cnt = (self.clan_registry.jewel_count(cnr) - 11).min(3);
        if cnt > 0 {
            self.queue_system_text(
                character_id,
                format!("You won. You stole {cnt} jewels for your clan's storage."),
            );
            self.clan_registry.dungeon_jewel_steal(cnr, onr, cnt);
            self.pending_dungeon_jewel_steals
                .push(DungeonJewelStealEvent {
                    player_id: character_id,
                    defender_clan: cnr,
                    attacker_clan: onr,
                    stolen: cnt,
                });
        } else {
            self.queue_system_text(
                character_id,
                "You won. Unfortunately there's nothing left to steal.",
            );
        }

        let slot = nr as usize;
        for id in self.characters_in_dungeon_slot(slot) {
            self.queue_system_text(id, "This catacomb has been solved and will collapse.");
        }

        let dungeonmaster_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| character.driver == CDR_DUNGEONMASTER)
            .map(|character| character.id)
            .collect();
        let ticker = self.tick.0 as i32;
        for dungeonmaster_id in dungeonmaster_ids {
            if let Some(dungeonmaster) = self.characters.get_mut(&dungeonmaster_id) {
                dungeonmaster.push_driver_message(NT_NPC, NTID_DUNGEON, nr as i32, ticker);
            }
        }
    }
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
                    // failure. Same-area rest points teleport immediately;
                    // cross-area ones are queued for `ugaris-server`'s
                    // `apply_dungeon_eviction_transfers` (see the module
                    // doc comment), which itself falls back to
                    // `remove_character` (C's `exit_char(cn)`) if the
                    // hand-off fails.
                    let rest = self
                        .characters
                        .get(&character_id)
                        .map(|character| (character.rest_area, character.rest_x, character.rest_y));
                    match rest {
                        Some((area, rest_x, rest_y)) if area == self.area_id => {
                            if !self.teleport_character_same_area(
                                character_id,
                                rest_x,
                                rest_y,
                                false,
                            ) {
                                self.remove_character(character_id);
                            }
                        }
                        Some((target_area, target_x, target_y)) => {
                            self.pending_dungeon_eviction_transfers
                                .push(DungeonEvictionTransfer {
                                    character_id,
                                    target_area,
                                    target_x,
                                    target_y,
                                });
                        }
                        None => {
                            self.remove_character(character_id);
                        }
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

    /// C `dungeonmaster`'s top-level per-NPC dispatch (`dungeon.c:1571-
    /// 1731`): iterates every live `CDR_DUNGEONMASTER` NPC, processes its
    /// message queue, then its per-slot expiry/warning tick and 12h
    /// driver-memory clear. C's own `secure_move_driver(cn, ch[cn].tmpx,
    /// ch[cn].tmpy, DX_DOWN, ret, lastact)` call is not ported: the
    /// dungeonmaster NPC has no zone-file waypoints in `ugaris_data`
    /// (unlike `clanmaster`/`clanclerk`, which patrol back to a rest
    /// tile), so it is a dead call in practice - it only ever resumes an
    /// already-in-progress forced move, which nothing in this driver
    /// ever starts.
    pub fn process_dungeonmaster_actions(&mut self) {
        let dungeonmaster_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_DUNGEONMASTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for dungeonmaster_id in dungeonmaster_ids {
            self.process_dungeonmaster_messages(dungeonmaster_id);
            self.dungeonmaster_tick(dungeonmaster_id);
        }
    }

    /// C `dungeonmaster`'s message loop (`dungeon.c:1580-1725`).
    fn process_dungeonmaster_messages(&mut self, dungeonmaster_id: CharacterId) {
        let Some(dungeonmaster_name) = self
            .characters
            .get(&dungeonmaster_id)
            .map(|c| c.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Dungeonmaster(mut dat)) = self
            .characters
            .get(&dungeonmaster_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&dungeonmaster_id)
            .map(|dungeonmaster| std::mem::take(&mut dungeonmaster.driver_messages))
            .unwrap_or_default();

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.dungeonmaster_handle_char_message(dungeonmaster_id, message),
                NT_TEXT => self.dungeonmaster_handle_text_message(
                    dungeonmaster_id,
                    &dungeonmaster_name,
                    &mut dat,
                    message,
                ),
                NT_GIVE => self.dungeonmaster_handle_give_message(dungeonmaster_id),
                NT_NPC if message.dat1 == NTID_DUNGEON => {
                    let slot = message.dat2.max(0) as usize;
                    if slot < DUNGEON_SLOT_COUNT {
                        self.destroy_dungeon(slot);
                        dat.target[slot] = 0;
                        dat.level[slot] = 0;
                        dat.created[slot] = 0;
                        dat.warning[slot] = 0;
                        dat.owner[slot] = 0;
                        dat.created_by_clan[slot] = 0;
                    }
                }
                _ => {}
            }
        }

        if let Some(dungeonmaster) = self.characters.get_mut(&dungeonmaster_id) {
            dungeonmaster.driver_state = Some(CharacterDriverState::Dungeonmaster(dat));
        }
    }

    /// C `dungeonmaster`'s `NT_CHAR` greeting branch (`dungeon.c:1597-
    /// 1620`): "don't talk to someone we can't see, or ourself", "don't
    /// talk to someone far away" (10 tiles), "don't talk to the same
    /// person twice" (`mem_check_driver(cn, co, 7)`).
    fn dungeonmaster_handle_char_message(
        &mut self,
        dungeonmaster_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let speaker_id = CharacterId(message.dat1.max(0) as u32);
        if speaker_id == dungeonmaster_id {
            return;
        }
        let Some(dungeonmaster) = self.characters.get(&dungeonmaster_id).cloned() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !char_see_char(&dungeonmaster, &speaker, &self.map, self.date.daylight) {
            return;
        }
        if char_dist(&dungeonmaster, &speaker) > 10 {
            return;
        }
        if mem_check_driver(&dungeonmaster.driver_memory, 7, speaker_id.0) {
            return;
        }
        self.npc_say(
            dungeonmaster_id,
            &format!(
                "Hello {}! Welcome to the clan catacombs. Be warned, there is a fee of 3500 \
                 gold for attacking now. Say help for details.",
                speaker.name
            ),
        );
        if let Some(dungeonmaster_mut) = self.characters.get_mut(&dungeonmaster_id) {
            mem_add_driver(&mut dungeonmaster_mut.driver_memory, 7, speaker_id.0);
        }
    }

    /// C `dungeonmaster`'s `NT_GIVE` branch (`dungeon.c:1670-1675`): any
    /// gift is unconditionally destroyed (no jewel/item validation,
    /// unlike `clanclerk`'s `NT_GIVE` handler).
    fn dungeonmaster_handle_give_message(&mut self, dungeonmaster_id: CharacterId) {
        if let Some(item_id) = self
            .characters
            .get_mut(&dungeonmaster_id)
            .and_then(|dungeonmaster| dungeonmaster.cursor_item.take())
        {
            self.destroy_item(item_id);
        }
    }

    /// C `dungeonmaster`'s `NT_TEXT` branch (`dungeon.c:1626-1668`): the
    /// shared small-talk qa table (help/list), then the independent
    /// `attack <nr>`/`enter <nr>`/(GM-only) `destroy <nr>` substring
    /// commands - all four run unconditionally regardless of whether the
    /// qa table matched anything, exactly like C's plain (non-`else`)
    /// `if` chain.
    fn dungeonmaster_handle_text_message(
        &mut self,
        dungeonmaster_id: CharacterId,
        dungeonmaster_name: &str,
        dat: &mut DungeonmasterDriverData,
        message: &CharacterDriverMessage,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        if speaker_id == dungeonmaster_id {
            return;
        }
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(text) = message.text.clone() else {
            return;
        };

        let can_see =
            self.characters
                .get(&dungeonmaster_id)
                .cloned()
                .is_some_and(|dungeonmaster| {
                    char_see_char(&dungeonmaster, &speaker, &self.map, self.date.daylight)
                });
        if can_see {
            match analyse_text_qa(&text, dungeonmaster_name, &speaker.name, DUNGEONMASTER_QA) {
                TextAnalysisOutcome::Said(reply) => {
                    self.npc_say(dungeonmaster_id, &reply);
                }
                TextAnalysisOutcome::Matched(2) => {
                    self.npc_say(
                        dungeonmaster_id,
                        "Use: 'attack <nr>' to attack clan <nr>, 'enter <nr>' to enter catacomb \
                         <nr> or 'list' to get a listing of all catacombs.",
                    );
                }
                TextAnalysisOutcome::Matched(3) => {
                    let lines = self.list_dungeon_lines(dat);
                    for line in lines {
                        self.npc_say(dungeonmaster_id, &line);
                    }
                }
                _ => {}
            }
        }

        let lower = text.to_ascii_lowercase();
        if let Some(pos) = lower.find("attack") {
            let target = parse_int_atoi(text.get(pos + 6..).unwrap_or(""));
            self.dungeonmaster_handle_attack_command(dungeonmaster_id, dat, speaker_id, target);
        }
        if let Some(pos) = lower.find("enter") {
            let target = parse_int_atoi(text.get(pos + 5..).unwrap_or(""));
            self.dungeonmaster_handle_enter_command(dungeonmaster_id, dat, speaker_id, target);
        }
        if let Some(pos) = lower.find("destroy") {
            if speaker.flags.contains(CharacterFlags::GOD) {
                let target = parse_int_atoi(text.get(pos + 7..).unwrap_or(""));
                self.dungeonmaster_handle_destroy_command(dat, target);
            }
        }
    }

    /// C `dungeonmaster`'s `attack` handler, calling into `create_dungeon`
    /// (`dungeon.c:1648`): on success, charges the fee, updates the
    /// slot's tracking fields, and queues a [`DungeonRaidBuildRequest`]
    /// for `ugaris-server` to actually build the maze (see that type's
    /// doc comment).
    fn dungeonmaster_handle_attack_command(
        &mut self,
        dungeonmaster_id: CharacterId,
        dat: &mut DungeonmasterDriverData,
        speaker_id: CharacterId,
        target: i32,
    ) {
        let target_clan = target.clamp(0, i32::from(u16::MAX)) as u16;
        match self.plan_create_dungeon(speaker_id, target_clan, dat) {
            Ok(plan) => {
                // C `take_money(co, fee * 100)` - `gate_take_money` is the
                // same shared `take_money(cn, val)` primitive, named for
                // its first caller (`src/system/tool.c:3820-3826`).
                if !self.gate_take_money(speaker_id, plan.fee.saturating_mul(100)) {
                    self.npc_say(
                        dungeonmaster_id,
                        &format!("Sorry, you cannot afford the fee of {}G.", plan.fee),
                    );
                    return;
                }
                self.npc_say(
                    dungeonmaster_id,
                    &format!(
                        "Very well, I have created the catacomb for you. Thank you for paying \
                         {} gold.",
                        plan.fee
                    ),
                );
                dat.level[plan.slot] = plan.level;
                dat.target[plan.slot] = target_clan;
                dat.created[plan.slot] = self.tick.0;
                dat.warning[plan.slot] = 0;
                dat.owner[plan.slot] = speaker_id.0;
                dat.created_by_clan[plan.slot] = plan.own_clan;
                self.pending_dungeon_raid_builds
                    .push(DungeonRaidBuildRequest {
                        dungeonmaster_id,
                        player_id: speaker_id,
                        target_clan,
                        own_clan: plan.own_clan,
                        slot: plan.slot,
                        xoff: plan.xoff,
                        yoff: plan.yoff,
                        level: plan.level,
                        warrior: plan.warrior,
                        mage: plan.mage,
                        seyan: plan.seyan,
                        teleport: plan.teleport,
                        fake: plan.fake,
                        key: plan.key,
                    });
            }
            Err(err) => {
                let message = match err {
                    DungeonRaidError::NoSuchClan => "No clan by that number.".to_string(),
                    DungeonRaidError::LevelTooHigh => {
                        "You cannot create a clan catacomb, your level is too high (max 56)."
                            .to_string()
                    }
                    DungeonRaidError::NotAtWar => "You are not at war with that clan.".to_string(),
                    DungeonRaidError::TargetHasNoJewels => {
                        "That clan does not have any jewels you could steal.".to_string()
                    }
                    DungeonRaidError::OwnClanLacksJewels => {
                        "Your clan does not have enough jewels to mount a raid (your clan needs \
                         to have at least 11 of them)."
                            .to_string()
                    }
                    DungeonRaidError::CatacombAlreadyExists { slot } => {
                        format!("This catacomb already exists, please use 'enter {slot}' instead.")
                    }
                    DungeonRaidError::ClanAlreadyRaiding => {
                        "Your clan has created a catacomb already, you may not create another \
                         one before the first one has collapsed."
                            .to_string()
                    }
                    DungeonRaidError::PlayerAlreadyRaiding => {
                        "You have created a catacomb already, you may not create another one \
                         before the first one has collapsed."
                            .to_string()
                    }
                    DungeonRaidError::AllCatacombsBusy { wait_ticks } => format!(
                        "Sorry, all catacombs are busy. Please try again in {:.2} minutes",
                        wait_ticks as f64 / (TICKS_PER_SECOND as f64 * 60.0)
                    ),
                };
                self.npc_say(dungeonmaster_id, &message);
            }
        }
    }

    /// C `dungeonmaster`'s `enter` handler, calling into `enter_dungeon`
    /// (`dungeon.c:1655`).
    fn dungeonmaster_handle_enter_command(
        &mut self,
        dungeonmaster_id: CharacterId,
        dat: &DungeonmasterDriverData,
        speaker_id: CharacterId,
        target: i32,
    ) {
        match self.plan_enter_dungeon(speaker_id, target, dat) {
            Ok(plan) => {
                self.npc_say(
                    dungeonmaster_id,
                    &format!(
                        "This catacomb will collapse in {:.2} minutes,",
                        plan.remaining_ticks as f64 / (TICKS_PER_SECOND as f64 * 60.0)
                    ),
                );
                self.teleport_character_same_area(speaker_id, plan.x, plan.y, false);
            }
            Err(err) => {
                let message = match err {
                    DungeonEnterError::TargetOutOfBounds => {
                        "Sorry, the target is out of bounds.".to_string()
                    }
                    DungeonEnterError::LevelTooHigh { max_level } => format!(
                        "Sorry, you may not enter this catacomb, it was created for level \
                         {max_level} and below."
                    ),
                    DungeonEnterError::NotAtWar => "You are not at war with that clan.".to_string(),
                    DungeonEnterError::AboutToCollapse => {
                        "Sorry, this catacomb is about to collapse.".to_string()
                    }
                };
                self.npc_say(dungeonmaster_id, &message);
            }
        }
    }

    /// C `dungeonmaster`'s GM-only `destroy` handler (`dungeon.c:1657-
    /// 1668`): the GOD-flag gate is checked by the caller
    /// ([`Self::dungeonmaster_handle_text_message`]) since it inspects
    /// the speaker, not this slot-reset logic.
    fn dungeonmaster_handle_destroy_command(
        &mut self,
        dat: &mut DungeonmasterDriverData,
        target: i32,
    ) {
        if target > 0 && target < 10 {
            let slot = (target - 1) as usize;
            self.destroy_dungeon(slot);
            dat.target[slot] = 0;
            dat.level[slot] = 0;
            dat.created[slot] = 0;
            dat.warning[slot] = 0;
            dat.owner[slot] = 0;
            dat.created_by_clan[slot] = 0;
        }
    }

    /// C `dungeonmaster`'s per-slot expiry/warning tick plus the 12h
    /// driver-memory clear (`dungeon.c:1706-1725`).
    fn dungeonmaster_tick(&mut self, dungeonmaster_id: CharacterId) {
        let Some(CharacterDriverState::Dungeonmaster(mut dat)) = self
            .characters
            .get(&dungeonmaster_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let dungeon_time = i64::from(self.settings.dungeon_time);
        let ticker = self.tick.0 as i64;

        for slot in 0..DUNGEON_SLOT_COUNT {
            if dat.created[slot] == 0 {
                continue;
            }
            let tmp = ticker - dat.created[slot] as i64;
            if tmp > dungeon_time {
                self.destroy_dungeon(slot);
                dat.target[slot] = 0;
                dat.level[slot] = 0;
                dat.created[slot] = 0;
                dat.warning[slot] = 0;
                dat.owner[slot] = 0;
                dat.created_by_clan[slot] = 0;
            }
            if tmp > dat.warning[slot] as i64 {
                self.warn_dungeon(slot, dungeon_time - tmp);
                dat.warning[slot] = dat.warning[slot].saturating_add(TICKS_PER_SECOND * 60 * 5);
            }
        }

        if ticker > dat.memcleartimer as i64 {
            if let Some(dungeonmaster) = self.characters.get_mut(&dungeonmaster_id) {
                mem_erase_driver(&mut dungeonmaster.driver_memory, 7);
            }
            dat.memcleartimer = (ticker as u64).saturating_add(TICKS_PER_SECOND * 60 * 60 * 12);
        }

        if let Some(dungeonmaster) = self.characters.get_mut(&dungeonmaster_id) {
            dungeonmaster.driver_state = Some(CharacterDriverState::Dungeonmaster(dat));
        }
    }

    /// C `warn_dungeon(nr, left)` (`dungeon.c:1559-1569`).
    fn warn_dungeon(&mut self, slot: usize, left_ticks: i64) {
        let minutes = left_ticks as f64 / (TICKS_PER_SECOND as f64 * 60.0);
        let message = format!("This catacomb will collapse in {minutes:.2} minutes.");
        for character_id in self.characters_in_dungeon_slot(slot) {
            self.queue_system_text(character_id, message.clone());
        }
    }
}
