use crate::world::*;

impl World {
    pub fn open_lab2_grave(
        &mut self,
        item_id: ItemId,
        undead_id: CharacterId,
        undead_serial: u32,
    ) -> bool {
        let (x, y) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            if item.driver_data.len() < 12 {
                item.driver_data.resize(12, 0);
            }
            let open_character = i32::from_le_bytes(item.driver_data[4..8].try_into().unwrap());
            if open_character != 0 {
                return false;
            }
            item.sprite += 1;
            item.driver_data[4..8].copy_from_slice(&(undead_id.0 as i32).to_le_bytes());
            item.driver_data[8..12].copy_from_slice(&(undead_serial as i32).to_le_bytes());
            (item.x, item.y)
        };
        self.mark_dirty_sector(usize::from(x), usize::from(y));
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        true
    }

    pub fn open_empty_lab2_grave(&mut self, item_id: ItemId) -> bool {
        let (x, y) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            if item.driver_data.len() < 12 {
                item.driver_data.resize(12, 0);
            }
            let open_character = i32::from_le_bytes(item.driver_data[4..8].try_into().unwrap());
            if open_character != 0 {
                return false;
            }
            item.sprite += 1;
            item.driver_data[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
            item.driver_data[8..12].copy_from_slice(&(-1_i32).to_le_bytes());
            (item.x, item.y)
        };
        self.mark_dirty_sector(usize::from(x), usize::from(y));
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 5);
        true
    }

    pub(crate) fn close_lab2_grave(&mut self, item_id: ItemId) -> bool {
        let (x, y) = {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            if item.driver_data.len() < 12 {
                item.driver_data.resize(12, 0);
            }
            let open_character = i32::from_le_bytes(item.driver_data[4..8].try_into().unwrap());
            if open_character == 0 {
                return false;
            }
            item.sprite -= 1;
            item.driver_data[4..8].copy_from_slice(&0_i32.to_le_bytes());
            item.driver_data[8..12].copy_from_slice(&0_i32.to_le_bytes());
            (item.x, item.y)
        };
        self.mark_dirty_sector(usize::from(x), usize::from(y));
        true
    }

    pub fn process_lab2_undead_message_actions(&mut self, character_id: CharacterId) -> usize {
        let Some(character) = self.characters.get(&character_id) else {
            return 0;
        };
        if character.action != action::IDLE || character.flags.contains(CharacterFlags::DEAD) {
            return 0;
        }
        if !matches!(
            character.driver_state,
            Some(CharacterDriverState::Lab2Undead(_))
        ) {
            return 0;
        }

        let messages = self
            .characters
            .get_mut(&character_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();
        let mut handled = 0;

        for message in messages {
            if message.message_type == NT_CHAR {
                handled += usize::from(self.remove_lab2_undead_crypt_corridor_enemy(
                    character_id,
                    CharacterId(message.dat1.max(0) as u32),
                ));
                continue;
            }
            if message.message_type != NT_GIVE {
                continue;
            }
            handled += 1;
            let giver_id = CharacterId(message.dat1.max(0) as u32);
            let Some(item_id) = self
                .characters
                .get(&character_id)
                .and_then(|character| character.cursor_item)
            else {
                continue;
            };
            let Some(item) = self.items.get(&item_id).cloned() else {
                continue;
            };

            let holy_water =
                item.driver == IDR_LAB2_WATER && item.driver_data.first().copied() == Some(5);
            self.destroy_item(item_id);
            if !holy_water {
                continue;
            }

            if let Some(giver) = self.characters.get(&giver_id) {
                if giver.flags.bits() != 0 {
                    let name = self
                        .characters
                        .get(&character_id)
                        .map(|character| character.name.clone())
                        .unwrap_or_default();
                    self.queue_system_text(
                        giver_id,
                        format!("You spill the holy water all over the {name}."),
                    );
                }
            }

            let undead = self
                .characters
                .get(&character_id)
                .and_then(|character| match character.driver_state.as_ref() {
                    Some(CharacterDriverState::Lab2Undead(data)) => Some(data.undead != 0),
                    _ => None,
                })
                .unwrap_or(false);
            let protected_by_nomagic =
                self.characters
                    .get(&character_id)
                    .and_then(|character| {
                        self.map
                            .tile(usize::from(character.x), usize::from(character.y))
                            .map(|tile| (character, tile.flags.contains(MapFlags::NOMAGIC)))
                    })
                    .is_some_and(|(character, nomagic)| {
                        nomagic
                            && !self.characters.get(&giver_id).is_some_and(|giver| {
                                giver.flags.contains(CharacterFlags::NONOMAGIC)
                            })
                            && character.flags.bits() != 0
                    });

            if protected_by_nomagic || !undead {
                self.npc_say(character_id, "Mwahahahaha...");
                continue;
            }

            self.npc_say(character_id, "Arrgh!");
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.flags.remove(CharacterFlags::NODEATH);
            }
            if let Some((x, y)) = self
                .characters
                .get(&character_id)
                .map(|character| (character.x, character.y))
            {
                self.create_mist_effect(i32::from(x), i32::from(y));
            }
            if let Some(regen_item_id) = self.characters.get(&character_id).and_then(|character| {
                match character.driver_state.as_ref() {
                    Some(CharacterDriverState::Lab2Undead(data)) => data.regenerate_item_id,
                    _ => None,
                }
            }) {
                if let Some(regen_item) = self.items.get_mut(&regen_item_id) {
                    if regen_item.driver_data.len() < 12 {
                        regen_item.driver_data.resize(12, 0);
                    }
                    let start_tick = (self.tick.0 + TICKS_PER_SECOND * 20) as u32;
                    regen_item.driver_data[8..12].copy_from_slice(&start_tick.to_le_bytes());
                }
            }
            let _ = self.apply_legacy_hurt(character_id, Some(giver_id), 20 * POWERSCALE, 1, 0, 0);
        }

        handled
    }

    pub(crate) fn remove_lab2_undead_crypt_corridor_enemy(
        &mut self,
        character_id: CharacterId,
        target_id: CharacterId,
    ) -> bool {
        const SECOND_CORRIDOR_MIN_X: u16 = 169;
        const SECOND_CORRIDOR_MIN_Y: u16 = 154;
        const SECOND_CORRIDOR_MAX_X: u16 = 188;
        const SECOND_CORRIDOR_MAX_Y: u16 = 158;

        if character_id == target_id {
            return false;
        }
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !matches!(
            character.driver_state,
            Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
                patrol: 2,
                ..
            }))
        ) {
            return false;
        }
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        if target.x < SECOND_CORRIDOR_MIN_X
            || target.y < SECOND_CORRIDOR_MIN_Y
            || target.x > SECOND_CORRIDOR_MAX_X
            || target.y > SECOND_CORRIDOR_MAX_Y
            || !char_see_char(&character, &target, &self.map, self.date.daylight)
        {
            return false;
        }

        let Some(CharacterDriverState::Lab2Undead(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        else {
            return false;
        };
        let previous_len = data.enemies.len();
        data.enemies.retain(|enemy| enemy.target_id != target_id);
        data.enemies.len() != previous_len
    }

    pub fn process_lab2_undead_patrol_action(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character.driver != CDR_LAB2UNDEAD
            || character.action != action::IDLE
            || character.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }
        let Some(CharacterDriverState::Lab2Undead(data)) = character.driver_state.as_ref() else {
            return false;
        };
        if data.patrol == 0 || data.patstep == 0 {
            return false;
        }

        let waypoint_index = usize::from(data.pat.min(data.patstep.saturating_sub(1)));
        let target_x = u16::from(data.patx[waypoint_index]);
        let target_y = u16::from(data.paty[waypoint_index]);
        if target_x == 0 || target_y == 0 {
            return false;
        }

        if self.setup_walk_toward(
            character_id,
            usize::from(target_x),
            usize::from(target_y),
            0,
            area_id,
            false,
        ) || self.setup_walk_toward(
            character_id,
            usize::from(target_x),
            usize::from(target_y),
            0,
            area_id,
            true,
        ) {
            return true;
        }

        if character.x.abs_diff(target_x) >= 3 || character.y.abs_diff(target_y) >= 3 {
            return false;
        }

        let mut idle_ticks = 0;
        let mut say: Option<&'static str> = None;
        if let Some(CharacterDriverState::Lab2Undead(data)) = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            let old_pat = data.pat;
            data.pat = (data.pat + 1) % data.patstep.max(1);
            if data.patrol == 2 {
                match old_pat {
                    0 => idle_ticks = (TICKS_PER_SECOND * 2) as i32,
                    3 => {
                        idle_ticks = (TICKS_PER_SECOND * 2) as i32;
                        say = Some("A gust of wind?");
                    }
                    4 => {
                        idle_ticks = (TICKS_PER_SECOND * 2) as i32;
                        say = Some("Strange.");
                    }
                    _ => {}
                }
            }
        }
        if let Some(message) = say {
            self.npc_say(character_id, message);
        }
        if idle_ticks > 0 {
            if let Some(character) = self.characters.get_mut(&character_id) {
                let _ = do_idle(character, idle_ticks);
            }
        }
        true
    }

    pub fn process_lab2_undead_patrol_actions(&mut self, area_id: u16) -> usize {
        let character_ids: Vec<_> = self
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                (character.driver == CDR_LAB2UNDEAD
                    && matches!(
                        character.driver_state,
                        Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
                            patrol: 1..,
                            ..
                        }))
                    ))
                .then_some(character_id)
            })
            .collect();

        character_ids
            .into_iter()
            .filter(|&character_id| self.process_lab2_undead_patrol_action(character_id, area_id))
            .count()
    }

    pub fn process_lab2_undead_cathedral_self_destruction(
        &mut self,
        character_id: CharacterId,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character.driver != CDR_LAB2UNDEAD
            || character.action != action::IDLE
            || character.flags.contains(CharacterFlags::DEAD)
            || !matches!(
                character.driver_state,
                Some(CharacterDriverState::Lab2Undead(_))
            )
        {
            return false;
        }

        let cathedral_ground = self
            .map
            .tile(usize::from(character.x), usize::from(character.y))
            .map(|tile| matches!(tile.ground_sprite & 0xffff, 20456 | 17062))
            .unwrap_or(false);
        if !cathedral_ground {
            return false;
        }

        self.npc_say(character_id, "Arrgh!");
        self.create_mist_effect(i32::from(character.x), i32::from(character.y));
        if let Some(character) = self.characters.get_mut(&character_id) {
            character
                .flags
                .insert(CharacterFlags::DEAD | CharacterFlags::UPDATE);
            character
                .flags
                .remove(CharacterFlags::ALIVE | CharacterFlags::NODEATH);
            character.hp = 0;
            character.deaths = character.deaths.saturating_add(1);
        }
        true
    }

    pub fn process_lab2_undead_cathedral_self_destructions(&mut self) -> usize {
        let character_ids: Vec<_> = self
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                (character.driver == CDR_LAB2UNDEAD
                    && matches!(
                        character.driver_state,
                        Some(CharacterDriverState::Lab2Undead(_))
                    ))
                .then_some(character_id)
            })
            .collect();

        character_ids
            .into_iter()
            .filter(|&character_id| {
                self.process_lab2_undead_cathedral_self_destruction(character_id)
            })
            .count()
    }

    pub fn process_lab2_undead_crypt_door_action(&mut self, character_id: CharacterId) -> bool {
        const CRYPT_DOOR_X: u16 = 168;
        const CRYPT_DOOR_Y: u16 = 156;

        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character.driver != CDR_LAB2UNDEAD
            || character.action != action::IDLE
            || character.flags.contains(CharacterFlags::DEAD)
        {
            return false;
        }
        if !matches!(
            character.driver_state,
            Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
                patrol: 2,
                ..
            }))
        ) {
            return false;
        }
        if character.x >= CRYPT_DOOR_X
            || character.x.abs_diff(CRYPT_DOOR_X) >= 3
            || character.y.abs_diff(CRYPT_DOOR_Y) >= 3
        {
            return false;
        }

        let Some(door_item_id) = self
            .map
            .tile(usize::from(CRYPT_DOOR_X), usize::from(CRYPT_DOOR_Y))
            .and_then(|tile| (tile.item != 0).then_some(ItemId(tile.item)))
        else {
            return false;
        };
        if !self.items.get(&door_item_id).is_some_and(|item| {
            item.driver == IDR_DOOR && item.driver_data.first().copied().unwrap_or_default() == 1
        }) {
            return false;
        }

        self.toggle_door(door_item_id, character_id) == DoorToggleResult::Toggled
    }

    pub fn process_lab2_undead_crypt_door_actions(&mut self) -> usize {
        let character_ids: Vec<_> = self
            .characters
            .iter()
            .filter_map(|(&character_id, character)| {
                (character.driver == CDR_LAB2UNDEAD
                    && matches!(
                        character.driver_state,
                        Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
                            patrol: 2,
                            ..
                        }))
                    ))
                .then_some(character_id)
            })
            .collect();

        character_ids
            .into_iter()
            .filter(|&character_id| self.process_lab2_undead_crypt_door_action(character_id))
            .count()
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct Lab2UndeadDriverData {
    pub aggressive: i32,
    pub helper: i32,
    pub undead: i32,
    pub patrol: i32,
    pub pat: u8,
    pub patstep: u8,
    pub patx: [u8; 8],
    pub paty: [u8; 8],
    pub grave_item_id: Option<ItemId>,
    pub regenerate_item_id: Option<ItemId>,
    pub opened_by_character_id: Option<CharacterId>,
    pub opened_by_serial: u32,
    pub next_wait_tick: i32,
    #[serde(default)]
    pub enemies: Vec<SimpleBaddyEnemy>,
}

pub fn parse_lab2_undead_driver_args(
    args: &str,
) -> (Lab2UndeadDriverData, Vec<UnknownSimpleBaddyArgument>) {
    let mut data = Lab2UndeadDriverData::default();
    let mut unknown = Vec::new();
    let mut rest = args;

    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "aggressive" => data.aggressive = parsed,
            "helper" => data.helper = parsed,
            "patrol" => data.patrol = parsed,
            "undead" => data.undead = parsed,
            _ => unknown.push(UnknownSimpleBaddyArgument {
                name: name.to_string(),
                value: value.to_string(),
            }),
        }
        rest = next;
    }

    (data, unknown)
}

pub fn apply_lab2_undead_create_message(
    character: &mut Character,
    args: Option<&str>,
) -> Vec<UnknownSimpleBaddyArgument> {
    let mut data = match character.driver_state.take() {
        Some(CharacterDriverState::Lab2Undead(data)) => data,
        _ => Lab2UndeadDriverData::default(),
    };

    let unknown = if let Some(args) = args.filter(|args| !args.is_empty()) {
        let parsed = parse_lab2_undead_driver_args(args);
        data = parsed.0;
        parsed.1
    } else {
        Vec::new()
    };

    apply_lab2_undead_patrol_defaults(&mut data);
    character.driver_state = Some(CharacterDriverState::Lab2Undead(data));
    character
        .driver_messages
        .retain(|message| message.message_type != NT_CREATE);
    unknown
}

fn apply_lab2_undead_patrol_defaults(data: &mut Lab2UndeadDriverData) {
    match data.patrol {
        1 => {
            data.patx = [168, 168, 204, 204, 0, 0, 0, 0];
            data.paty = [178, 218, 218, 178, 0, 0, 0, 0];
            data.patstep = 4;
            data.helper = 0;
        }
        2 => {
            data.patx = [171, 138, 138, 165, 167, 138, 138, 171];
            data.paty = [164, 164, 146, 146, 146, 146, 164, 164];
            data.patstep = 8;
            data.helper = 0;
        }
        _ => {}
    }
}
