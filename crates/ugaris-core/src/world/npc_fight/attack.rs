//! Simple-baddy melee attack passes: per-character attack actions, visibility
//! and follow logic, direct attack drivers and the per-area attack pass.

use super::*;

impl World {
    pub fn process_simple_baddy_attack_action(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> bool {
        let mut seed = self.legacy_random_seed;
        let processed =
            self.process_simple_baddy_attack_action_with_random(character_id, area_id, |below| {
                legacy_random_below_from_seed(&mut seed, below)
            });
        self.legacy_random_seed = seed;
        processed
    }

    pub fn process_simple_baddy_attack_action_with_random(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        mut random: impl FnMut(u32) -> u32,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        // C: `dungeonfighter`'s own tail `char_driver(CDR_SIMPLEBADDY,
        // CDT_DRIVER, cn, ret, lastact)` call (`dungeon.c:2161`) reuses this
        // exact attack logic for `CDR_DUNGEONFIGHTER` guard NPCs too - see
        // `Character::dungeonfighter`'s doc comment. `CDR_PENTER` pentagram
        // demons (`pents.c::demon_character_driver`) do the same tail call
        // (`char_driver(CDR_SIMPLEBADDY, ...)`), same precedent. `CDR_
        // SWAMPMONSTER`'s `ch_driver` dispatch (`swamp.c:807-809`) is the
        // same one-line unconditional tail call too, as is `CDR_
        // FORESTMONSTER`'s (`forest.c:909-911`), `CDR_TWOROBBER`'s
        // (`two.c:3163-3165`), `CDR_SMUGGLELEAD`'s (`staffer.c:932-934`),
        // `CDR_WHITEROBBERBOSS`'s (`brannington_forest.c:684-686`),
        // `CDR_CENTINEL`'s (`brannington.c:2802-2804`), and `CDR_
        // MISSIONFIGHT`'s (`missions.c:1849-1851`). `CDR_TEUFELDEMON`'s
        // (`teufel.c:373-394`) is the same tail call too, though it also
        // needs its own extra `NT_CHAR` handling - see
        // `world::npc::area34::teufeldemon`'s module doc comment.
        // `CDR_TEUFELRAT`'s `teufelrat_driver` (`teufel.c:1610-1626`) is
        // a pure tail call too - its own `NT_CHAR` case body is empty
        // (commented out in C), so unlike `CDR_TEUFELDEMON` it needs no
        // extra per-tick logic of its own at all. `CDR_ARKHATAPRISON`'s
        // `prisoner_driver` (`arkhata.c:4329-4331`) and `CDR_BOOKEATER`'s
        // `bookeater_driver` (`arkhata.c:2083-2085`) are the same pure
        // tail call - see their own doc comments. `CDR_ARKHATASKELLY`'s
        // `arkhataskelly_driver` (`arkhata.c:1587-1609`) is the same pure
        // tail call too - see its own doc comment. `CDR_FORTRESSGUARD`'s
        // `fortressguard_driver` (`arkhata.c:2587-2833`) is *not* a pure
        // tail call (own reimplementation, see its own doc comment for the
        // two real deltas) but reuses `fight_driver_attack_visible`/
        // `fight_driver_follow_invisible` identically, so it needs the
        // same gate widening. `CDR_SHR_WEREWOLF`'s `shr_werewolf_driver`
        // (`shrike.c:379-391`) is a *conditional* tail call (only at full
        // night) - unlike every other driver in this list it is
        // deliberately *not* added to the batch sweep in
        // `process_simple_baddy_attack_actions_with_random` below, only to
        // this per-character gate, so `world::npc::area38::werewolf` can
        // call this function directly once it has already decided it is
        // night - see that module's doc comment.
        if (attacker.driver != CDR_SIMPLEBADDY
            && attacker.driver != CDR_DUNGEONFIGHTER
            && attacker.driver != CDR_PENTER
            && attacker.driver != CDR_SWAMPMONSTER
            && attacker.driver != CDR_FORESTMONSTER
            && attacker.driver != CDR_TWOROBBER
            && attacker.driver != CDR_SMUGGLELEAD
            && attacker.driver != CDR_WHITEROBBERBOSS
            && attacker.driver != CDR_CENTINEL
            && attacker.driver != CDR_MISSIONFIGHT
            && attacker.driver != CDR_TEUFELDEMON
            && attacker.driver != CDR_TEUFELRAT
            && attacker.driver != CDR_CALIGARGUARD2
            && attacker.driver != CDR_CALIGARSKELLY
            && attacker.driver != CDR_ARKHATAPRISON
            && attacker.driver != CDR_BOOKEATER
            && attacker.driver != CDR_ARKHATASKELLY
            && attacker.driver != CDR_FORTRESSGUARD
            && attacker.driver != CDR_SHR_WEREWOLF)
            || attacker.action != 0
            || attacker.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }

        self.fight_driver_attack_visible_and_follow(
            character_id,
            &attacker,
            area_id,
            FightDriverSuppressions::default(),
            true,
            &mut random,
        )
    }

    /// C `fight_driver_attack_visible`+`fight_driver_follow_invisible`
    /// (`src/system/drvlib.c:2222-2320`), player-side wiring: the
    /// `CDR_LOSTCON` self-defense driver (`lostcon_driver`'s own
    /// `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
    /// ppd->nomove)) return; if (!ppd->nomove &&
    /// fight_driver_follow_invisible(cn)) return;` cascade,
    /// `lostcon.c:200-203`) calls this with the lingering `PlayerRuntime`'s
    /// `no*` toggles converted to `suppressions`. Returns `true` if an
    /// action was queued (caller should not also run its idle fallback).
    pub fn process_lostcon_attack_action_with_random(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        suppressions: FightDriverSuppressions,
        mut random: impl FnMut(u32) -> u32,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if attacker.driver != CDR_LOSTCON
            || attacker.action != 0
            || attacker.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }

        self.fight_driver_attack_visible_and_follow(
            character_id,
            &attacker,
            area_id,
            suppressions,
            true,
            &mut random,
        )
    }

    /// Shared body of `fight_driver_attack_visible`+
    /// `fight_driver_follow_invisible` (`src/system/drvlib.c:2222-2320`):
    /// score/attempt every visible enemy in score order (highest `(999 -
    /// dist) * 10 [+5 if facing]` first), falling back to pathfinding
    /// toward the last known position of one invisible enemy when nothing
    /// visible could be attacked, `!suppressions.nomove`, and
    /// `may_follow_invisible` (C's `if (!ppd->nomove &&
    /// fight_driver_follow_invisible(cn))` gate - the always-all-`false`-
    /// suppressions NPC caller never sets `nomove`, so this preserves its
    /// behavior unchanged). `may_follow_invisible` exists because a few C
    /// drivers (e.g. `two.c::thiefguard`, `strategy.c`, `saltmine.c`) call
    /// `fight_driver_attack_visible(cn, 0)` (full movement allowed for the
    /// attack task itself) but never call `fight_driver_follow_invisible`
    /// at all - a real, deliberate C behavior difference from
    /// `simple_baddy_driver`/`lostcon_driver`, not modeled by
    /// `suppressions.nomove` alone (which also suppresses movement *within*
    /// the visible-attack task itself, unlike C's independent nomove
    /// argument to the two separate functions).
    pub(crate) fn fight_driver_attack_visible_and_follow(
        &mut self,
        character_id: CharacterId,
        attacker: &Character,
        area_id: u16,
        suppressions: FightDriverSuppressions,
        may_follow_invisible: bool,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let enemies = self.refresh_simple_baddy_enemy_tracking(attacker);
        if enemies.is_empty() {
            return false;
        }
        let mut visible_enemies: Vec<_> = enemies
            .iter()
            .filter(|enemy| enemy.visible)
            .copied()
            .collect();
        visible_enemies.sort_by(|left, right| {
            self.simple_baddy_visible_enemy_score(attacker, right)
                .cmp(&self.simple_baddy_visible_enemy_score(attacker, left))
        });

        for enemy in visible_enemies {
            let previous_lastfight = self
                .simple_baddy_lastfight(character_id)
                .unwrap_or_default();
            let Some(target) = self.characters.get(&enemy.target_id).cloned() else {
                continue;
            };
            if !can_attack_in_area(attacker, &target, &self.map, area_id) {
                continue;
            }
            if self.setup_weighted_fight_task(character_id, &target, area_id, suppressions, random)
            {
                self.queue_simple_baddy_attack_sound(character_id, previous_lastfight);
                return true;
            }
        }

        if suppressions.nomove || !may_follow_invisible {
            return false;
        }

        for enemy in enemies.into_iter().filter(|enemy| !enemy.visible) {
            if attacker.x.abs_diff(enemy.last_x) < 2 && attacker.y.abs_diff(enemy.last_y) < 2 {
                self.remove_simple_baddy_enemy(character_id, enemy.target_id);
                continue;
            }
            if self.setup_walk_toward(
                character_id,
                usize::from(enemy.last_x),
                usize::from(enemy.last_y),
                0,
                area_id,
                false,
            ) || self.setup_walk_toward(
                character_id,
                usize::from(enemy.last_x),
                usize::from(enemy.last_y),
                0,
                area_id,
                true,
            ) {
                if let Some(character) = self.characters.get_mut(&character_id) {
                    if let Some(CharacterDriverState::SimpleBaddy(data)) =
                        character.driver_state.as_mut()
                    {
                        data.lastfight = self.tick.0 as i32;
                    }
                }
                return true;
            }
            self.remove_simple_baddy_enemy(character_id, enemy.target_id);
        }

        false
    }

    pub(crate) fn simple_baddy_visible_enemy_score(
        &self,
        attacker: &Character,
        enemy: &SimpleBaddyEnemy,
    ) -> i32 {
        let Some(target) = self.characters.get(&enemy.target_id) else {
            return i32::MIN;
        };
        let mut score = (999 - char_dist(attacker, target)) * 10;
        if character_is_facing(attacker, target) {
            score += 5;
        }
        score
    }

    pub(crate) fn setup_simple_baddy_attack_move(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let from_x = usize::from(attacker.x);
        let from_y = usize::from(attacker.y);
        let target_x = usize::from(target.x);
        let target_y = usize::from(target.y);
        let direct = pathfinder(&self.map, from_x, from_y, target_x, target_y, 1, None);
        let moving = (target.tox != 0).then(|| {
            pathfinder(
                &self.map,
                from_x,
                from_y,
                usize::from(target.tox),
                usize::from(target.toy),
                1,
                None,
            )
        });

        let best_partial = moving.unwrap_or(direct);
        let direction = match (direct.direction, moving) {
            (Some(_direct_direction), Some(moving_result))
                if moving_result.direction.is_some() && direct.cost >= moving_result.cost =>
            {
                moving_result.direction.expect("checked above")
            }
            (Some(direct_direction), _) => direct_direction,
            (None, Some(moving_result)) if moving_result.direction.is_some() => {
                moving_result.direction.expect("checked above")
            }
            // The is_some/unwrap pair mirrors the C original's explicit
            // check-then-use sequencing; restructuring the if/else-if
            // chain around if-let would obscure that mapping.
            #[allow(clippy::unnecessary_unwrap)]
            (None, _) => {
                let current_distance = manhattan_distance(from_x, from_y, target_x, target_y);
                if best_partial.best_direction.is_some()
                    && best_partial.best_distance < current_distance
                {
                    best_partial.best_direction.unwrap()
                } else if self.setup_adjacent_use_toward_target(character_id, target_x, target_y) {
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        if let Some(CharacterDriverState::SimpleBaddy(data)) =
                            character.driver_state.as_mut()
                        {
                            data.lastfight = self.tick.0 as i32;
                        }
                    }
                    return true;
                } else {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        return false;
                    };
                    return do_idle(character, (TICKS_PER_SECOND / 4) as i32).is_ok();
                }
            }
        };

        if !self.walk_or_use_driver(character_id, direction, area_id) {
            if !self.setup_adjacent_use_toward_target(character_id, target_x, target_y) {
                return false;
            }
            if let Some(attacker_mut) = self.characters.get_mut(&character_id) {
                if let Some(CharacterDriverState::SimpleBaddy(data)) =
                    attacker_mut.driver_state.as_mut()
                {
                    data.lastfight = self.tick.0 as i32;
                }
            }
            return true;
        }
        if let Some(attacker_mut) = self.characters.get_mut(&character_id) {
            if let Some(CharacterDriverState::SimpleBaddy(data)) =
                attacker_mut.driver_state.as_mut()
            {
                data.lastfight = self.tick.0 as i32;
            }
        }
        true
    }

    pub(crate) fn setup_adjacent_use_toward_target(
        &mut self,
        character_id: CharacterId,
        target_x: usize,
        target_y: usize,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let from_x = usize::from(character.x);
        let from_y = usize::from(character.y);
        let current_distance = manhattan_distance(from_x, from_y, target_x, target_y);

        let mut best: Option<(Direction, ItemId, usize)> = None;
        for direction in [
            Direction::Right,
            Direction::Left,
            Direction::Down,
            Direction::Up,
        ] {
            let (dx, dy) = direction.delta();
            let Some(x) = offset_coordinate(from_x, dx) else {
                continue;
            };
            let Some(y) = offset_coordinate(from_y, dy) else {
                continue;
            };
            let Some(tile) = self.map.tile(x, y) else {
                continue;
            };
            if !tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            {
                continue;
            }
            let item_id = ItemId(tile.item);
            let Some(item) = (tile.item != 0).then(|| self.items.get(&item_id)).flatten() else {
                continue;
            };
            if !item.flags.contains(ItemFlags::USE) {
                continue;
            }
            let distance = manhattan_distance(x, y, target_x, target_y);
            if distance >= current_distance {
                continue;
            }
            if best.is_none_or(|(_, _, best_distance)| distance < best_distance) {
                best = Some((direction, item_id, distance));
            }
        }

        let Some((direction, item_id, _)) = best else {
            return false;
        };
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        do_use(
            character,
            &self.map,
            item,
            direction as u8,
            0,
            self.settings.weather_movement_percent,
        )
        .is_ok()
    }

    pub(crate) fn setup_simple_baddy_attack_driver(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };

        let direction = adjacent_direction(
            attacker.x,
            attacker.y,
            usize::from(target.x),
            usize::from(target.y),
        )
        .or_else(|| {
            (target.tox != 0).then(|| {
                adjacent_direction(
                    attacker.x,
                    attacker.y,
                    usize::from(target.tox),
                    usize::from(target.toy),
                )
            })?
        });
        let Some(direction) = direction else {
            return false;
        };
        let Some(attacker_mut) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_attack(
            attacker_mut,
            &self.map,
            target,
            direction as u8,
            action::ATTACK1,
            self.settings.weather_movement_percent,
        )
        .is_err()
        {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = attacker_mut.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub fn attack_driver_direct(
        &mut self,
        character_id: CharacterId,
        target_id: CharacterId,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        if attacker.id == target.id
            || !char_see_char(&attacker, &target, &self.map, self.date.daylight)
            || !can_attack_in_area(&attacker, &target, &self.map, area_id)
        {
            return false;
        }

        if let Some(direction) = adjacent_direction(
            attacker.x,
            attacker.y,
            usize::from(target.x),
            usize::from(target.y),
        )
        .or_else(|| {
            (target.tox != 0).then(|| {
                adjacent_direction(
                    attacker.x,
                    attacker.y,
                    usize::from(target.tox),
                    usize::from(target.toy),
                )
            })?
        }) {
            let Some(attacker_mut) = self.characters.get_mut(&character_id) else {
                return false;
            };
            return do_attack(
                attacker_mut,
                &self.map,
                &target,
                direction as u8,
                action::ATTACK1,
                self.settings.weather_movement_percent,
            )
            .is_ok();
        }

        let path = pathfinder(
            &self.map,
            usize::from(attacker.x),
            usize::from(attacker.y),
            usize::from(target.x),
            usize::from(target.y),
            1,
            None,
        );
        let Some(direction) = path.direction else {
            return false;
        };
        self.walk_or_use_driver(character_id, direction, area_id)
    }

    pub(crate) fn simple_baddy_lastfight(&self, character_id: CharacterId) -> Option<i32> {
        let character = self.characters.get(&character_id)?;
        match character.driver_state.as_ref()? {
            CharacterDriverState::SimpleBaddy(data) => Some(data.lastfight),
            CharacterDriverState::Clara(_)
            | CharacterDriverState::TwoSkelly(_)
            | CharacterDriverState::Lab2Undead(_)
            | CharacterDriverState::Merchant(_)
            | CharacterDriverState::Aclerk(_)
            | CharacterDriverState::Lostcon(_)
            | CharacterDriverState::Bank(_)
            | CharacterDriverState::Trader(_)
            | CharacterDriverState::Janitor(_)
            | CharacterDriverState::GateWelcome(_)
            | CharacterDriverState::GateFight(_)
            | CharacterDriverState::Clanmaster(_)
            | CharacterDriverState::ClanFound(_)
            | CharacterDriverState::Clanclerk(_)
            | CharacterDriverState::Clubmaster(_)
            | CharacterDriverState::MilitaryMaster(_)
            | CharacterDriverState::MilitaryAdvisor(_)
            | CharacterDriverState::ArenaMaster(_)
            | CharacterDriverState::ArenaFighter(_)
            | CharacterDriverState::ArenaManager(_)
            | CharacterDriverState::Dungeonmaster(_)
            | CharacterDriverState::Dungeonfighter(_)
            | CharacterDriverState::Macro(_)
            | CharacterDriverState::Camhermit(_)
            | CharacterDriverState::Yoakin(_)
            | CharacterDriverState::Terion(_)
            | CharacterDriverState::Gwendylon(_)
            | CharacterDriverState::Greeter(_)
            | CharacterDriverState::Jessica(_)
            | CharacterDriverState::Jiu(_)
            | CharacterDriverState::ForestRanger(_)
            | CharacterDriverState::Brithildie(_)
            | CharacterDriverState::Nook(_)
            | CharacterDriverState::Lydia(_)
            | CharacterDriverState::Robber(_)
            | CharacterDriverState::Sanoa(_)
            | CharacterDriverState::Asturin(_)
            | CharacterDriverState::Reskin(_)
            | CharacterDriverState::Guiwynn(_)
            | CharacterDriverState::James(_)
            | CharacterDriverState::Balltrap(_)
            | CharacterDriverState::Logain(_)
            | CharacterDriverState::Superior(_)
            | CharacterDriverState::Moonie(_)
            | CharacterDriverState::Vampire(_)
            | CharacterDriverState::Vampire2(_)
            | CharacterDriverState::Astro1(_)
            | CharacterDriverState::Astro2(_)
            | CharacterDriverState::Thomas(_)
            | CharacterDriverState::SirJones(_)
            | CharacterDriverState::Seymour(_)
            | CharacterDriverState::Kelly(_)
            | CharacterDriverState::Lampghost(_)
            | CharacterDriverState::Carlos(_)
            | CharacterDriverState::Kassim(_)
            | CharacterDriverState::Supermax(_)
            | CharacterDriverState::Tester(_)
            | CharacterDriverState::Engrave(_)
            | CharacterDriverState::FdemonArmy(_)
            | CharacterDriverState::Islena(_)
            | CharacterDriverState::PalaceGuard(_)
            | CharacterDriverState::GolemKeyhold(_)
            | CharacterDriverState::ForestImp(_)
            | CharacterDriverState::ForestWilliam(_)
            | CharacterDriverState::ForestHermit(_)
            | CharacterDriverState::TwoSanwyn(_)
            | CharacterDriverState::TwoAlchemist(_)
            | CharacterDriverState::TwoBarkeeper(_)
            | CharacterDriverState::TwoServant(_)
            | CharacterDriverState::TwoGuard(_)
            | CharacterDriverState::TwoThiefGuard(_)
            | CharacterDriverState::TwoThiefMaster(_)
            | CharacterDriverState::Nomad(_)
            | CharacterDriverState::Madhermit(_)
            | CharacterDriverState::LqNpc(_)
            | CharacterDriverState::LabGnome(_)
            | CharacterDriverState::Lab2Herald(_)
            | CharacterDriverState::Lab2Deamon(_)
            | CharacterDriverState::Lab3Passguard(_)
            | CharacterDriverState::Lab3Prisoner(_)
            | CharacterDriverState::Lab4Seyan(_)
            | CharacterDriverState::Lab4Gnalb(_)
            | CharacterDriverState::Lab5Seyan(_)
            | CharacterDriverState::Lab5Daemon(_)
            | CharacterDriverState::Lab5Mage(_)
            | CharacterDriverState::StrategyWorker(_)
            | CharacterDriverState::WarpFighter(_)
            | CharacterDriverState::Warpmaster(_)
            | CharacterDriverState::SmuggleCom(_)
            | CharacterDriverState::Rouven(_)
            | CharacterDriverState::Aristocrat(_)
            | CharacterDriverState::Yoatin(_)
            | CharacterDriverState::SpiritBran(_)
            | CharacterDriverState::GuardBran(_)
            | CharacterDriverState::BrennethBran(_)
            | CharacterDriverState::Broklin(_)
            | CharacterDriverState::CountBran(_)
            | CharacterDriverState::CountessaBran(_)
            | CharacterDriverState::DaughterBran(_)
            | CharacterDriverState::ForestBran(_)
            | CharacterDriverState::Grinnich(_)
            | CharacterDriverState::Shanra(_)
            | CharacterDriverState::DwarfChief(_)
            | CharacterDriverState::LostDwarf(_)
            | CharacterDriverState::DwarfShaman(_)
            | CharacterDriverState::DwarfSmith(_)
            | CharacterDriverState::MissionGiver(_)
            | CharacterDriverState::Gorwin(_)
            | CharacterDriverState::TeufelGambler(_)
            | CharacterDriverState::TeufelQuest(_)
            | CharacterDriverState::Nop(_)
            | CharacterDriverState::Rammy(_)
            | CharacterDriverState::Jaz(_)
            | CharacterDriverState::Fiona(_)
            | CharacterDriverState::BridgeGuard(_)
            | CharacterDriverState::Gladiator(_)
            | CharacterDriverState::Ramin(_)
            | CharacterDriverState::Arkhatamonk(_)
            | CharacterDriverState::Captain(_)
            | CharacterDriverState::Judge(_)
            | CharacterDriverState::Jada(_)
            | CharacterDriverState::Potmaker(_)
            | CharacterDriverState::Hunter(_)
            | CharacterDriverState::Thaipan(_)
            | CharacterDriverState::Trainer(_)
            | CharacterDriverState::Kidnappee(_)
            | CharacterDriverState::Clerk(_)
            | CharacterDriverState::Krenach(_)
            | CharacterDriverState::Professor(_) => None,
        }
    }

    pub(crate) fn queue_simple_baddy_attack_sound(
        &mut self,
        character_id: CharacterId,
        previous_lastfight: i32,
    ) {
        let current_tick = self.tick.0 as i32;
        if current_tick - previous_lastfight <= (TICKS_PER_SECOND * 10) as i32 {
            return;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        self.queue_sound_area(usize::from(character.x), usize::from(character.y), 1);
    }

    pub fn process_simple_baddy_attack_actions(&mut self, area_id: u16) -> usize {
        let mut seed = self.legacy_random_seed;
        let count = self.process_simple_baddy_attack_actions_with_random(area_id, |below| {
            legacy_random_below_from_seed(&mut seed, below)
        });
        self.legacy_random_seed = seed;
        count
    }

    pub fn process_simple_baddy_attack_actions_with_random(
        &mut self,
        area_id: u16,
        mut random: impl FnMut(u32) -> u32,
    ) -> usize {
        let character_ids: Vec<_> = self
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                ((character.driver == CDR_SIMPLEBADDY
                    || character.driver == CDR_DUNGEONFIGHTER
                    || character.driver == CDR_PENTER
                    || character.driver == CDR_SWAMPMONSTER
                    || character.driver == CDR_FORESTMONSTER
                    || character.driver == CDR_TWOROBBER
                    || character.driver == CDR_SMUGGLELEAD
                    || character.driver == CDR_WHITEROBBERBOSS
                    || character.driver == CDR_CENTINEL
                    || character.driver == CDR_MISSIONFIGHT
                    || character.driver == CDR_TEUFELDEMON
                    || character.driver == CDR_TEUFELRAT
                    || character.driver == CDR_CALIGARGUARD2
                    || character.driver == CDR_CALIGARSKELLY
                    || character.driver == CDR_ARKHATAPRISON
                    || character.driver == CDR_BOOKEATER
                    || character.driver == CDR_ARKHATASKELLY
                    || character.driver == CDR_FORTRESSGUARD)
                    && matches!(
                        character.driver_state,
                        Some(CharacterDriverState::SimpleBaddy(_))
                    ))
                .then_some(character_id)
            })
            .collect();

        character_ids
            .into_iter()
            .filter(|&character_id| {
                self.process_simple_baddy_attack_action_with_random(
                    character_id,
                    area_id,
                    &mut random,
                )
            })
            .count()
    }
}
