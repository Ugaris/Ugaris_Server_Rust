use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FdemonGateSpawnContext {
    pub slot: usize,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FdemonLoaderBlockReason {
    CrystalAlreadyPresent,
    CrystalStuck,
    NeedsCrystal,
    WrongCrystal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FdemonBloodBlockReason {
    BareHands,
    WrongItem,
    ContainerFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FdemonLavaBlockReason {
    BareHands,
    WrongItem,
    EmptyContainer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FdemonCrystalTemplate {
    Small,
    Medium,
    Large,
    Huge,
    Giant,
}

impl FdemonCrystalTemplate {
    pub fn from_farm_size(size: u8) -> Self {
        if size >= 48 {
            Self::Giant
        } else if size >= 40 {
            Self::Huge
        } else if size >= 32 {
            Self::Large
        } else if size >= 24 {
            Self::Medium
        } else {
            Self::Small
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "fdemon_crystal1",
            Self::Medium => "fdemon_crystal2",
            Self::Large => "fdemon_crystal3",
            Self::Huge => "fdemon_crystal4",
            Self::Giant => "fdemon_crystal5",
        }
    }

    pub fn legacy_number(self) -> u8 {
        match self {
            Self::Small => 1,
            Self::Medium => 2,
            Self::Large => 3,
            Self::Huge => 4,
            Self::Giant => 5,
        }
    }

    pub fn foreground_sprite(self) -> u32 {
        match self {
            Self::Small => 59020,
            Self::Medium => 59040,
            Self::Large => 59041,
            Self::Huge => 59042,
            Self::Giant => 59043,
        }
    }
}

pub(crate) fn fdemon_light_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let Some(power) = context.fdemon_loader_power else {
        return ItemDriverOutcome::Noop;
    };
    let (light, sprite) = if power != 0 { (200, 14192) } else { (0, 14189) };

    item.modifier_index[0] = V_LIGHT;
    item.modifier_value[0] = light;
    item.sprite = sprite;

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: Some(TICKS_PER_SECOND),
    }
}

pub(crate) fn fdemon_loader_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(7, 0);

    let mut power = drdata_u16(item, 1);
    let mut animation = item.driver_data[3];
    let mut next_power = drdata_u16(item, 4);
    let mut consumed_cursor_item_id = None;
    let mut sound_type = None;

    if context.timer_call || character.id.0 == 0 {
        if animation != 0 {
            animation = animation.saturating_sub(1);
            if animation == 0 {
                power = next_power;
            }
        }
        if power != 0 {
            power = power.saturating_sub(1);
        }
    } else {
        if power != 0 || animation != 0 {
            if character.flags.contains(CharacterFlags::FDEMON) {
                power = 0;
                animation = 0;
                next_power = 0;
            } else if character.cursor_item.is_some() {
                return ItemDriverOutcome::FdemonLoaderBlocked {
                    item_id: item.id,
                    character_id: character.id,
                    reason: FdemonLoaderBlockReason::CrystalAlreadyPresent,
                };
            } else {
                return ItemDriverOutcome::FdemonLoaderBlocked {
                    item_id: item.id,
                    character_id: character.id,
                    reason: FdemonLoaderBlockReason::CrystalStuck,
                };
            }
        } else {
            let Some(cursor_item_id) = character.cursor_item else {
                return ItemDriverOutcome::FdemonLoaderBlocked {
                    item_id: item.id,
                    character_id: character.id,
                    reason: FdemonLoaderBlockReason::NeedsCrystal,
                };
            };
            if context.cursor_template_id != Some(IID_AREA8_REDCRYSTAL) {
                return ItemDriverOutcome::FdemonLoaderBlocked {
                    item_id: item.id,
                    character_id: character.id,
                    reason: FdemonLoaderBlockReason::WrongCrystal,
                };
            }

            next_power = u16::from(context.cursor_drdata0.unwrap_or_default()).saturating_mul(100);
            animation = 7;
            character.cursor_item = None;
            character.flags.insert(CharacterFlags::ITEMS);
            consumed_cursor_item_id = Some(cursor_item_id);
            sound_type = Some(41);
        }
    }

    if animation == 0 {
        next_power = power;
    }
    set_drdata_u16(item, 4, next_power);
    item.driver_data[3] = animation;
    set_drdata_u16(item, 1, power);

    let overlay = if animation != 0 {
        59028u32.saturating_sub(u32::from(animation))
    } else if next_power != 0 {
        59029
    } else {
        59021
    };

    let old_sprite = item.sprite;
    item.sprite = if next_power != 0 {
        59030 + 9 - (i32::from(next_power.min(2880)) / 320)
    } else {
        14234
    };
    if old_sprite != 14234 && item.sprite == 14234 {
        sound_type = Some(43);
    }

    ItemDriverOutcome::FdemonLoaderChanged {
        item_id: item.id,
        character_id: character.id,
        consumed_cursor_item_id,
        ground_overlay_sprite: overlay,
        sound_type,
        schedule_after_ticks: (context.timer_call || character.id.0 == 0)
            .then_some(TICKS_PER_SECOND),
    }
}

pub(crate) fn fdemon_cannon_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 && !context.timer_call {
        return if context.fdemon_loader_power.unwrap_or_default() == 0 {
            ItemDriverOutcome::FdemonCannonLifeless {
                item_id: item.id,
                character_id: character.id,
            }
        } else {
            ItemDriverOutcome::Noop
        };
    }

    ItemDriverOutcome::FdemonCannonPulse {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: TICKS_PER_SECOND,
    }
}

pub(crate) fn fdemon_waypoint_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let character_id = character.id;
    let character_call = character_id.0 != 0 && !context.timer_call;
    let spotted_enemy = character_call && !character.flags.contains(CharacterFlags::FDEMON);
    ItemDriverOutcome::FdemonWaypoint {
        item_id: item.id,
        character_id,
        spotted_enemy,
        target_character_id: spotted_enemy.then_some(character_id),
        target_serial: spotted_enemy.then_some(character.serial),
        schedule_after_ticks: TICKS_PER_SECOND * 3,
    }
}

pub(crate) fn fdemon_gate_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let level = drdata(item, 0);
    let rate = u64::from(drdata(item, 1));
    let schedule_after_ticks = rate.saturating_mul(TICKS_PER_SECOND);

    let Some(spawn) = context.fdemon_gate_spawn else {
        return ItemDriverOutcome::LightChanged {
            item_id: item.id,
            character_id: CharacterId(0),
            schedule_after_ticks: Some(schedule_after_ticks),
        };
    };

    ItemDriverOutcome::FdemonGateSpawn {
        item_id: item.id,
        character_id: CharacterId(0),
        level,
        slot: spawn.slot,
        x: spawn.x,
        y: spawn.y,
        schedule_after_ticks,
    }
}

pub(crate) fn fdemon_farm_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(3, 0);

    let step = item.driver_data[0];
    let size = item.driver_data[1];
    let mut strength = item.driver_data[2];

    let ready_template = if strength < size {
        strength = strength.wrapping_add(step);
        None
    } else {
        Some(FdemonCrystalTemplate::from_farm_size(size))
    };

    if !context.timer_call && character.id.0 != 0 {
        if character.cursor_item.is_some() {
            return ItemDriverOutcome::FdemonFarmCursorOccupied {
                item_id: item.id,
                character_id: character.id,
            };
        }
        let Some(template) = ready_template else {
            item.driver_data[2] = strength;
            return ItemDriverOutcome::FdemonFarmNotReady {
                item_id: item.id,
                character_id: character.id,
                current: strength,
                required: size,
            };
        };

        strength = 0;
        item.driver_data[2] = strength;
        return ItemDriverOutcome::FdemonFarmHarvest {
            item_id: item.id,
            character_id: character.id,
            template,
            foreground_sprite: 0,
        };
    }

    item.driver_data[2] = strength;
    ItemDriverOutcome::FdemonFarmChanged {
        item_id: item.id,
        character_id: character.id,
        foreground_sprite: ready_template.map_or(0, FdemonCrystalTemplate::foreground_sprite),
        schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
    }
}

pub(crate) fn fdemon_blood_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item else {
        return ItemDriverOutcome::FdemonBloodBlocked {
            item_id: item.id,
            character_id: character.id,
            reason: FdemonBloodBlockReason::BareHands,
        };
    };

    if context.cursor_driver == Some(IDR_FLASK) {
        character.cursor_item = None;
        character.flags.insert(CharacterFlags::ITEMS);
        item.sprite = 14348;
        return ItemDriverOutcome::FdemonBloodDestroyedFlask {
            item_id: item.id,
            character_id: character.id,
            flask_item_id: cursor_item_id,
        };
    }

    if context.cursor_template_id != Some(IID_AREA8_BLOOD) {
        return ItemDriverOutcome::FdemonBloodBlocked {
            item_id: item.id,
            character_id: character.id,
            reason: FdemonBloodBlockReason::WrongItem,
        };
    }

    let amount = context.cursor_drdata0.unwrap_or_default();
    if amount > 2 {
        return ItemDriverOutcome::FdemonBloodBlocked {
            item_id: item.id,
            character_id: character.id,
            reason: FdemonBloodBlockReason::ContainerFull,
        };
    }

    let amount = amount.saturating_add(1);
    character.flags.insert(CharacterFlags::ITEMS);
    ItemDriverOutcome::FdemonBloodFilled {
        item_id: item.id,
        character_id: character.id,
        container_item_id: cursor_item_id,
        amount,
    }
}

pub(crate) fn fdemon_lava_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(1, 0);

    if context.timer_call || character.id.0 == 0 {
        if item.driver_data[0] > 0 {
            item.driver_data[0] -= 1;
        }
        let stage = item.driver_data[0];
        let (damage, armor_percent, schedule_after_ticks) = if stage == 0 {
            item.sprite = 14363;
            (1000 * POWERSCALE, 0, None)
        } else if stage < 20 {
            item.sprite = 14364;
            (10 * POWERSCALE, 50, Some(TICKS_PER_SECOND))
        } else if stage < 60 {
            item.sprite = 14365;
            (POWERSCALE, 50, Some(TICKS_PER_SECOND))
        } else {
            (0, 0, Some(TICKS_PER_SECOND))
        };
        return ItemDriverOutcome::FdemonLavaPulse {
            item_id: item.id,
            character_id: character.id,
            stage,
            damage,
            armor_percent,
            schedule_after_ticks,
        };
    }

    let Some(cursor_item_id) = character.cursor_item else {
        return ItemDriverOutcome::FdemonLavaBlocked {
            item_id: item.id,
            character_id: character.id,
            reason: FdemonLavaBlockReason::BareHands,
        };
    };

    if context.cursor_template_id != Some(IID_AREA8_BLOOD) {
        return ItemDriverOutcome::FdemonLavaBlocked {
            item_id: item.id,
            character_id: character.id,
            reason: FdemonLavaBlockReason::WrongItem,
        };
    }

    let amount = context.cursor_drdata0.unwrap_or_default();
    if amount < 1 {
        return ItemDriverOutcome::FdemonLavaBlocked {
            item_id: item.id,
            character_id: character.id,
            reason: FdemonLavaBlockReason::EmptyContainer,
        };
    }

    let amount = amount.saturating_sub(1);
    item.driver_data[0] = 120;
    item.sprite = 14366;
    character.flags.insert(CharacterFlags::ITEMS);
    ItemDriverOutcome::FdemonLavaActivated {
        item_id: item.id,
        character_id: character.id,
        container_item_id: cursor_item_id,
        amount,
        schedule_after_ticks: TICKS_PER_SECOND,
    }
}
