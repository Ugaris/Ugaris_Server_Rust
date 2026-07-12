use super::*;

impl World {
    pub fn process_simple_baddy_noncombat_action(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> bool {
        self.process_simple_baddy_noncombat_action_with_random_and_context(
            character_id,
            area_id,
            0,
            0,
            |_| 0,
        )
    }

    pub fn process_simple_baddy_noncombat_action_with_random(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        mut random_below: impl FnMut(i32) -> i32,
    ) -> bool {
        self.process_simple_baddy_noncombat_action_with_random_and_context(
            character_id,
            area_id,
            0,
            0,
            &mut random_below,
        )
    }

    pub fn process_simple_baddy_noncombat_action_with_context(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        ret: i32,
        last_action: u16,
    ) -> bool {
        self.process_simple_baddy_noncombat_action_with_random_and_context(
            character_id,
            area_id,
            ret,
            last_action,
            |_| 0,
        )
    }

    pub fn process_simple_baddy_noncombat_action_with_random_and_context(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        ret: i32,
        last_action: u16,
        mut random_below: impl FnMut(i32) -> i32,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_ref() else {
            return false;
        };
        // C: `dungeonfighter`'s own tail `char_driver(CDR_SIMPLEBADDY,
        // CDT_DRIVER, cn, ret, lastact)` call (`dungeon.c:2161`) reuses this
        // exact noncombat logic for `CDR_DUNGEONFIGHTER` guard NPCs too -
        // see `Character::dungeonfighter`'s doc comment. `CDR_PENTER`
        // pentagram demons (`pents.c::demon_character_driver`) do the same
        // tail call (`char_driver(CDR_SIMPLEBADDY, ...)`), same precedent.
        // `CDR_SWAMPMONSTER`'s `ch_driver` dispatch (`swamp.c:807-809`) is
        // the same one-line unconditional tail call too, as is `CDR_
        // FORESTMONSTER`'s (`forest.c:909-911`), `CDR_TWOROBBER`'s
        // (`two.c:3163-3165`), `CDR_SMUGGLELEAD`'s
        // (`staffer.c:932-934`), `CDR_CENTINEL`'s
        // (`brannington.c:2802-2804`), and `CDR_MISSIONFIGHT`'s
        // (`missions.c:1849-1851`). `CDR_TEUFELDEMON`'s (`teufel.c:373-
        // 394`) is the same tail call too, though it also needs its own
        // extra `NT_CHAR` handling - see `world::npc::area34::
        // teufeldemon`'s module doc comment. `CDR_TEUFELRAT`'s
        // `teufelrat_driver` (`teufel.c:1610-1626`) is a pure tail call
        // too - its own `NT_CHAR` case body is empty (commented out in
        // C), so unlike `CDR_TEUFELDEMON` it needs no extra per-tick
        // logic of its own at all.
        if (character.driver != CDR_SIMPLEBADDY
            && character.driver != CDR_DUNGEONFIGHTER
            && character.driver != CDR_PENTER
            && character.driver != CDR_SWAMPMONSTER
            && character.driver != CDR_FORESTMONSTER
            && character.driver != CDR_TWOROBBER
            && character.driver != CDR_SMUGGLELEAD
            && character.driver != CDR_CENTINEL
            && character.driver != CDR_MISSIONFIGHT
            && character.driver != CDR_TEUFELDEMON
            && character.driver != CDR_TEUFELRAT
            && character.driver != CDR_CALIGARGUARD2
            && character.driver != CDR_CALIGARSKELLY
            && character.driver != CDR_ARKHATAPRISON
            && character.driver != CDR_BOOKEATER
            && character.driver != CDR_ARKHATASKELLY)
            || character.action != 0
            || character.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }

        let current_tick = self.tick.0 as i32;
        if current_tick - data.creation_time < TICKS_PER_SECOND as i32 {
            return self
                .characters
                .get_mut(&character_id)
                .is_some_and(|character| {
                    do_idle(character, (TICKS_PER_SECOND / 4) as i32).is_ok()
                });
        }

        if data.scavenger != 0 {
            let Some((target_x, target_y)) = character
                .rest_x
                .ne(&0)
                .then_some((character.rest_x, character.rest_y))
            else {
                return self.idle_simple_baddy(character_id);
            };
            let scavenger_distance = data.scavenger.max(0) as u16;
            if character.x.abs_diff(target_x) >= scavenger_distance
                || character.y.abs_diff(target_y) >= scavenger_distance
            {
                self.clear_simple_baddy_scavenger_direction(character_id);
                if data.notsecure == 0
                    && current_tick - data.lastfight > (TICKS_PER_SECOND * 10) as i32
                {
                    if self.secure_move_driver(
                        character_id,
                        target_x,
                        target_y,
                        Direction::Down as u8,
                        ret,
                        last_action,
                        area_id,
                    ) {
                        return true;
                    }
                } else {
                    let min_dist = if data.notsecure != 0 {
                        data.mindist.max(0) as usize
                    } else {
                        0
                    };
                    if self.setup_walk_toward(
                        character_id,
                        usize::from(target_x),
                        usize::from(target_y),
                        min_dist,
                        area_id,
                        false,
                    ) || self.setup_walk_toward(
                        character_id,
                        usize::from(target_x),
                        usize::from(target_y),
                        min_dist,
                        area_id,
                        true,
                    ) {
                        return true;
                    }
                }
            }
            if self.regenerate_simple_baddy(character_id) {
                return true;
            }
            if self.spell_self_simple_baddy(character_id) {
                return true;
            }
            if self.setup_pending_simple_baddy_friend_bless(character_id) {
                return true;
            }
            if random_below(2) == 0 {
                return self.idle_simple_baddy(character_id);
            }

            let direction = if data.dir != 0 {
                data.dir
            } else {
                random_below(8).clamp(0, 7) + 1
            };
            let Some(direction) = u8::try_from(direction)
                .ok()
                .and_then(|direction| Direction::try_from(direction).ok())
            else {
                self.clear_simple_baddy_scavenger_direction(character_id);
                return self.idle_simple_baddy(character_id);
            };
            let (dx, dy) = direction.delta();
            let next_x = i32::from(character.x) + i32::from(dx);
            let next_y = i32::from(character.y) + i32::from(dy);
            if (next_x - i32::from(target_x)).abs() < i32::from(scavenger_distance)
                && (next_y - i32::from(target_y)).abs() < i32::from(scavenger_distance)
                && self.setup_walk_direction(character_id, direction, area_id)
            {
                let _ = self.set_simple_baddy_home(character_id, character.x, character.y);
                if let Some(CharacterDriverState::SimpleBaddy(data)) = self
                    .characters
                    .get_mut(&character_id)
                    .and_then(|character| character.driver_state.as_mut())
                {
                    data.dir = direction as i32;
                }
                return true;
            }

            self.clear_simple_baddy_scavenger_direction(character_id);
            self.drink_special_poison_simple_baddy(character_id);
            return self.regenerate_simple_baddy(character_id)
                || self.spell_self_simple_baddy(character_id)
                || self.setup_pending_simple_baddy_friend_bless(character_id)
                || self.idle_simple_baddy(character_id);
        }

        let target = if data.dayx != 0 {
            if self.date.hour > 19 || self.date.hour < 6 {
                Some((data.nightx, data.nighty, data.nightdir))
            } else {
                Some((data.dayx, data.dayy, data.daydir))
            }
        } else if character.rest_x != 0 {
            Some((i32::from(character.rest_x), i32::from(character.rest_y), 0))
        } else {
            None
        };

        let Some((target_x, target_y, target_dir)) = target.filter(|(x, y, _)| *x > 0 && *y > 0)
        else {
            self.drink_special_poison_simple_baddy(character_id);
            return self.regenerate_simple_baddy(character_id)
                || self.spell_self_simple_baddy(character_id)
                || self.setup_pending_simple_baddy_friend_bless(character_id)
                || self.idle_simple_baddy(character_id);
        };
        let target_x = target_x as u16;
        let target_y = target_y as u16;
        if character.x == target_x && character.y == target_y {
            let _ = self.set_simple_baddy_home(character_id, target_x, target_y);
            if target_dir != 0 {
                if let Some(character) = self.characters.get_mut(&character_id) {
                    let _ = turn(character, target_dir as u8);
                }
            }
            self.drink_special_poison_simple_baddy(character_id);
            return self.regenerate_simple_baddy(character_id)
                || self.spell_self_simple_baddy(character_id)
                || self.setup_pending_simple_baddy_friend_bless(character_id)
                || self.idle_simple_baddy(character_id);
        }

        if data.teleport != 0 && self.teleport_character(character_id, target_x, target_y, false) {
            let _ = self.set_simple_baddy_home(character_id, target_x, target_y);
            return true;
        }

        if data.notsecure == 0
            && current_tick - data.lastfight > (TICKS_PER_SECOND * 10) as i32
            && self.secure_move_driver(
                character_id,
                target_x,
                target_y,
                target_dir as u8,
                ret,
                last_action,
                area_id,
            )
        {
            return true;
        }

        let min_dist = if data.notsecure != 0 {
            data.mindist.max(0) as usize
        } else {
            0
        };
        let (walk_x, walk_y) = if data.notsecure != 0 && character.rest_x != 0 {
            (character.rest_x, character.rest_y)
        } else {
            (target_x, target_y)
        };
        if self.setup_walk_toward(
            character_id,
            usize::from(walk_x),
            usize::from(walk_y),
            min_dist,
            area_id,
            false,
        ) || self.setup_walk_toward(
            character_id,
            usize::from(walk_x),
            usize::from(walk_y),
            min_dist,
            area_id,
            true,
        ) {
            return true;
        }

        let _ = self.set_simple_baddy_home(character_id, character.x, character.y);
        self.drink_special_poison_simple_baddy(character_id);
        self.regenerate_simple_baddy(character_id)
            || self.spell_self_simple_baddy(character_id)
            || self.setup_pending_simple_baddy_friend_bless(character_id)
            || self.idle_simple_baddy(character_id)
    }

    pub fn process_simple_baddy_noncombat_actions(&mut self, area_id: u16) -> usize {
        self.process_simple_baddy_noncombat_actions_with_completions(area_id, &[])
    }

    pub fn process_simple_baddy_noncombat_actions_with_completions(
        &mut self,
        area_id: u16,
        completions: &[WorldActionCompletion],
    ) -> usize {
        let mut seed = self.legacy_random_seed;
        let count = self.process_simple_baddy_noncombat_actions_with_random_and_completions(
            area_id,
            completions,
            |below| {
                if below <= 0 {
                    0
                } else {
                    legacy_random_below_from_seed(&mut seed, below as u32) as i32
                }
            },
        );
        self.legacy_random_seed = seed;
        count
    }

    pub fn process_simple_baddy_noncombat_actions_with_random_and_completions(
        &mut self,
        area_id: u16,
        completions: &[WorldActionCompletion],
        mut random_below: impl FnMut(i32) -> i32,
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
                    || character.driver == CDR_CENTINEL
                    || character.driver == CDR_MISSIONFIGHT
                    || character.driver == CDR_TEUFELDEMON
                    || character.driver == CDR_TEUFELRAT
                    || character.driver == CDR_CALIGARGUARD2
                    || character.driver == CDR_CALIGARSKELLY
                    || character.driver == CDR_ARKHATAPRISON
                    || character.driver == CDR_BOOKEATER
                    || character.driver == CDR_ARKHATASKELLY)
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
                let (ret, last_action) = completions
                    .iter()
                    .rev()
                    .find(|completion| completion.character_id == character_id)
                    .map(|completion| (completion.legacy_return_code, completion.action_id))
                    .unwrap_or((0, 0));
                self.process_simple_baddy_noncombat_action_with_random_and_context(
                    character_id,
                    area_id,
                    ret,
                    last_action,
                    &mut random_below,
                )
            })
            .count()
    }

    pub(crate) fn idle_simple_baddy(&mut self, character_id: CharacterId) -> bool {
        self.characters
            .get_mut(&character_id)
            .is_some_and(|character| do_idle(character, TICKS_PER_SECOND as i32).is_ok())
    }

    pub(crate) fn regenerate_simple_baddy(&mut self, character_id: CharacterId) -> bool {
        self.characters
            .get_mut(&character_id)
            .is_some_and(|character| {
                let max_mana = character_value(character, CharacterValue::Mana) * POWERSCALE;
                let max_hp = character_value(character, CharacterValue::Hp) * POWERSCALE;
                if character.mana < max_mana || character.hp < max_hp {
                    do_idle(character, TICKS_PER_SECOND as i32).is_ok()
                } else {
                    false
                }
            })
    }

    pub(crate) fn spell_self_simple_baddy(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let current_tick = self.tick.0 as u32;

        let weather_movement_percent = self.settings.weather_movement_percent;

        if character_value(&character, CharacterValue::Bless) > 0
            && character.mana >= BLESS_COST
            && may_add_spell(&character, &self.items, IDR_BLESS, current_tick).is_some()
        {
            return self
                .characters
                .get_mut(&character_id)
                .is_some_and(|caster| {
                    do_bless(
                        caster,
                        &character,
                        &self.items,
                        current_tick,
                        None,
                        &self.map,
                        weather_movement_percent,
                    )
                    .is_ok()
                });
        }

        if character_value(&character, CharacterValue::MagicShield) * POWERSCALE
            > character.lifeshield
            && character.mana >= POWERSCALE * 3
        {
            return self
                .characters
                .get_mut(&character_id)
                .is_some_and(|character| {
                    do_magicshield(character, &self.map, weather_movement_percent).is_ok()
                });
        }

        if character_value(&character, CharacterValue::Heal) > 0
            && character.hp < character_value(&character, CharacterValue::Hp) * POWERSCALE / 2
            && character.mana >= POWERSCALE * 3
        {
            return self
                .characters
                .get_mut(&character_id)
                .is_some_and(|caster| {
                    do_heal(
                        caster,
                        &character,
                        None,
                        &self.map,
                        weather_movement_percent,
                    )
                    .is_ok()
                });
        }

        false
    }

    pub(crate) fn remember_simple_baddy_bless_friend(
        &mut self,
        character_id: CharacterId,
        target_id: CharacterId,
    ) {
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.pending_bless_friend = Some(target_id);
        }
    }

    pub(crate) fn clear_simple_baddy_bless_friend(&mut self, character_id: CharacterId) {
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.pending_bless_friend = None;
        }
    }

    pub(crate) fn setup_pending_simple_baddy_friend_bless(
        &mut self,
        character_id: CharacterId,
    ) -> bool {
        let target_id = self
            .characters
            .get(&character_id)
            .and_then(|character| character.driver_state.as_ref())
            .and_then(|state| match state {
                CharacterDriverState::SimpleBaddy(data) => data.pending_bless_friend,
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
                | CharacterDriverState::Gladiator(_) => None,
            });
        let Some(target_id) = target_id else {
            return false;
        };

        self.clear_simple_baddy_bless_friend(character_id);
        self.simple_baddy_can_bless_friend(character_id, target_id)
            && self.setup_bless_spell(character_id, target_id)
    }

    pub(crate) fn drink_special_poison_simple_baddy(&mut self, character_id: CharacterId) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_ref() else {
            return;
        };
        if data.drinkspecial == 0 {
            return;
        }
        let has_poison0 = character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
            .iter()
            .flatten()
            .any(|item_id| {
                self.items
                    .get(item_id)
                    .is_some_and(|item| item.driver == IDR_POISON0)
            });
        if has_poison0 {
            // C: `emote(cn, "drinks a potion")`.
            self.npc_emote(character_id, "drinks a potion");
            self.remove_all_poison(character_id);
        }
    }

    pub(crate) fn clear_simple_baddy_scavenger_direction(&mut self, character_id: CharacterId) {
        if let Some(CharacterDriverState::SimpleBaddy(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.dir = 0;
        }
    }

    pub fn secure_move_driver(
        &mut self,
        character_id: CharacterId,
        target_x: u16,
        target_y: u16,
        direction: u8,
        ret: i32,
        last_action: u16,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };

        if character.x != target_x || character.y != target_y {
            if (last_action != action::USE || ret != 2)
                && self.setup_walk_toward(
                    character_id,
                    usize::from(target_x),
                    usize::from(target_y),
                    0,
                    area_id,
                    false,
                )
            {
                return true;
            }
            return self.teleport_character(character_id, target_x, target_y, false);
        }

        if character.dir != direction {
            if let Some(character) = self.characters.get_mut(&character_id) {
                let _ = turn(character, direction);
            }
        }
        false
    }
}
