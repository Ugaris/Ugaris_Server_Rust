use super::*;
use crate::world::character_value_present;

pub(crate) fn labtorch_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    item.driver_data.resize(2, 0);

    if character.id.0 == 0 {
        item.driver_data[1] = item.modifier_value[0].clamp(0, u8::MAX as i16) as u8;
        return ItemDriverOutcome::Noop;
    }

    if item.driver_data[0] == 0 {
        if character.flags.contains(CharacterFlags::PLAYER) {
            return ItemDriverOutcome::Noop;
        }
        item.sprite += 1;
        item.driver_data[0] = 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = i16::from(item.driver_data[1]);
    } else {
        item.sprite -= 1;
        item.driver_data[0] = 0;
        item.modifier_value[0] = 0;
    }

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: None,
    }
}

/// C `deathfibrin` (`src/area/22/lab1.c:482-590`). The same driver id
/// (`IDR_DEATHFIBRIN = 198`) backs two very different objects
/// distinguished by sprite, exactly like C: `deathfibrin_shrine`
/// (`sprite == 10428`, a fixed map dispenser) and the carried/dropped
/// `deathfibrin` staff itself (`struct deathfibrin_data`, cast onto
/// `it[in].drdata`).
///
/// Deviations/gaps (documented, not silent):
/// - The zero-character passive ticker (`lab1.c:548-588`: light-based
///   `amount` decay while sitting lit on the ground or being carried,
///   auto-vanish after 10 minutes unattended, and the `dat->tickerused`
///   cooldown that pauses decay right after a strike) is not ported -
///   nothing schedules `call_item(IDR_DEATHFIBRIN, in, 0, ...)` for this
///   driver in this port. Without that ticker ever running, C's own
///   lazy `dat->init` (only set the first time the zero-character path
///   runs) would never fire either, so this port instead lazily
///   initializes `amount = 10000` on the *first player strike* instead
///   (byte 4 of `driver_data` doubles as the "already initialized"
///   flag) - the one piece of `dat->init` this port actually needs, so
///   a freshly created staff still starts at 100% charge instead of
///   incorrectly reading as already-spent.
/// - `dat->used`/`dat->tickerused`/`dat->tickervanish` (all only
///   meaningful to the unported ticker) are not represented at all.
pub(crate) fn deathfibrin_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    // C `lab1.c:487-510`: the shrine dispenser.
    if item.sprite == 10428 {
        if character.id.0 == 0 {
            return ItemDriverOutcome::Noop;
        }
        if character.cursor_item.is_some() {
            return ItemDriverOutcome::DeathfibrinShrineOccupied {
                character_id: character.id,
            };
        }
        return ItemDriverOutcome::DeathfibrinShrineGive {
            item_id: item.id,
            character_id: character.id,
        };
    }

    // C `lab1.c:548-588`: the passive ticker is not ported - see the
    // driver's own doc comment.
    if context.timer_call || character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    // C `lab1.c:516-519`.
    if item.carried_by.is_none() {
        return ItemDriverOutcome::DeathfibrinNeedsCarry {
            character_id: character.id,
        };
    }

    // C `lab1.c:521-526`.
    let Some(master_id) = context.deathfibrin_master else {
        return ItemDriverOutcome::DeathfibrinNoMaster {
            character_id: character.id,
            tile_light: context.deathfibrin_tile_light,
        };
    };

    // C `lab1.c:513-543`: lazy `dat->init` substitute (see doc comment)
    // plus the unconditional `dat->amount = max(0, dat->amount - 1000)`.
    item.driver_data.resize(item.driver_data.len().max(5), 0);
    let already_initialized = item.driver_data[4] != 0;
    let amount = if already_initialized {
        u32::from_le_bytes(item.driver_data[0..4].try_into().unwrap_or_default())
    } else {
        item.driver_data[4] = 1;
        10_000
    };
    let amount = amount.saturating_sub(1000);
    item.driver_data[0..4].copy_from_slice(&amount.to_le_bytes());
    let vanished = deathfibrin_check(item, amount);

    ItemDriverOutcome::DeathfibrinStrike {
        item_id: item.id,
        character_id: character.id,
        master_id,
        item_name: outcome_item_name(&item.name),
        vanished,
    }
}

/// C `deathfibrin_check` (`lab1.c:460-480`): updates the staff's sprite/
/// description for its new `amount`, returning whether it should vanish
/// (C's `remove_item`/`destroy_item`, applied by the caller since this
/// pure function has no `World` access).
fn deathfibrin_check(item: &mut Item, amount: u32) -> bool {
    if amount == 0 {
        return true;
    }
    item.sprite = (10428 - 10 * (amount as i32 + 500) / 10000).min(10427);
    item.description = format!("Staff containing {}% Deathfibrin", amount / 100);
    false
}

pub(crate) fn lab2_water_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    item.driver_data.resize(1, 0);

    if character.id.0 == 0 {
        if item.driver_data[0] == 0 {
            item.driver_data[0] = match item.sprite {
                11008..=11010 => 2,
                20793..=20796 => 1,
                11011 => 3,
                11012 => 4,
                11013 => 5,
                _ => 0,
            };
        }
        return ItemDriverOutcome::Noop;
    }

    match item.driver_data[0] {
        1 => {
            if character.cursor_item.is_some() {
                ItemDriverOutcome::Lab2WaterCursorOccupied {
                    item_id: item.id,
                    character_id: character.id,
                }
            } else {
                ItemDriverOutcome::Lab2WaterWell {
                    item_id: item.id,
                    character_id: character.id,
                }
            }
        }
        2 => ItemDriverOutcome::Lab2WaterAltar {
            item_id: item.id,
            character_id: character.id,
        },
        4 | 5 => ItemDriverOutcome::Lab2WaterDrink {
            item_id: item.id,
            character_id: character.id,
        },
        _ => ItemDriverOutcome::Noop,
    }
}

pub(crate) fn lab2_stepaction_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    let step_kind = item.driver_data.first().copied().unwrap_or_default();
    if !matches!(step_kind, 1 | 2) {
        return ItemDriverOutcome::Noop;
    }

    if character.id.0 == 0 {
        item.sprite = 0;
        return ItemDriverOutcome::Lab2StepActionClear { item_id: item.id };
    }

    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    match step_kind {
        1 if character.dir == Direction::Up as u8 => {
            ItemDriverOutcome::Lab2StepActionDaemonWarning {
                item_id: item.id,
                character_id: character.id,
                x: item.x,
                y: item.y.saturating_sub(5),
            }
        }
        2 => ItemDriverOutcome::Lab2StepActionDaemonCheck {
            item_id: item.id,
            character_id: character.id,
        },
        _ => ItemDriverOutcome::Noop,
    }
}

pub(crate) fn lab2_grave_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let grave_item = item.driver_data.first().copied().unwrap_or_default();
    let grave_open_character = item
        .driver_data
        .get(4..8)
        .and_then(|bytes| bytes.try_into().ok())
        .map(i32::from_le_bytes)
        .unwrap_or_default();
    let grave_open_serial = item
        .driver_data
        .get(8..12)
        .and_then(|bytes| bytes.try_into().ok())
        .map(i32::from_le_bytes)
        .unwrap_or_default();

    if (context.timer_call || character.id.0 == 0) && grave_open_character == 0 {
        return ItemDriverOutcome::Noop;
    }

    if (context.timer_call || character.id.0 == 0)
        && grave_open_character != 0
        && grave_open_serial == -1
    {
        return ItemDriverOutcome::Lab2GraveClose { item_id: item.id };
    }

    if (context.timer_call || character.id.0 == 0) && grave_open_character > 0 {
        return ItemDriverOutcome::Lab2GraveCheckOpen {
            item_id: item.id,
            undead_id: CharacterId(grave_open_character as u32),
            undead_serial: grave_open_serial as u32,
            schedule_after_ticks: TICKS_PER_SECOND * 5,
        };
    }

    if character.id.0 != 0 {
        if !character.flags.contains(CharacterFlags::PLAYER) {
            return ItemDriverOutcome::Noop;
        }

        if grave_open_character != 0 {
            return ItemDriverOutcome::Noop;
        }

        if matches!(grave_item, 1..=4) {
            return ItemDriverOutcome::Lab2GraveClueBook {
                item_id: item.id,
                character_id: character.id,
                book: grave_item,
            };
        }

        return ItemDriverOutcome::Lab2GraveOpen {
            item_id: item.id,
            character_id: character.id,
            fixed_item: grave_item,
        };
    }

    ItemDriverOutcome::Noop
}

pub(crate) fn lab2_regenerate_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let speed = drdata(item, 0);
    let schedule_after_ticks = u64::from(speed).saturating_mul(TICKS_PER_SECOND) / 24;
    ItemDriverOutcome::Lab2RegenerateTick {
        item_id: item.id,
        target_id: CharacterId(drdata_u32(item, 4)),
        start_tick: drdata_u32(item, 8),
        regen_percent: drdata(item, 1),
        schedule_after_ticks,
    }
}

pub(crate) fn lab3_plant_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call && character.id.0 == 0 && drdata(item, 0) == 10 {
        return ItemDriverOutcome::Lab3WhiteBerryLightTick {
            item_id: item.id,
            destroyed: false,
        };
    }

    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    match drdata(item, 0) {
        5 => {
            const OXYGEN_SECONDS: [u64; 5] = [3, 8, 10, 12, 15];
            let freshness = usize::from(drdata(item, 2).min(4));
            let count = u64::from(drdata(item, 1));
            ItemDriverOutcome::Lab3YellowBerry {
                item_id: item.id,
                character_id: character.id,
                duration_ticks: OXYGEN_SECONDS[freshness] * count * TICKS_PER_SECOND,
                installed: false,
            }
        }
        6 => {
            const LIGHT_POWER: [i16; 5] = [10, 30, 40, 45, 50];
            let freshness = usize::from(drdata(item, 2).min(4));
            let count = i16::from(drdata(item, 1));
            ItemDriverOutcome::Lab3WhiteBerry {
                item_id: item.id,
                character_id: character.id,
                light_power: LIGHT_POWER[freshness].saturating_mul(count),
                started_emit: false,
                installed: false,
            }
        }
        11 => ItemDriverOutcome::Lab3BrownBerry {
            item_id: item.id,
            character_id: character.id,
            duration_ticks: 10 * TICKS_PER_SECOND,
            installed: false,
        },
        _ => ItemDriverOutcome::Noop,
    }
}

/// C `lab3_special` (`src/area/22/lab3.c:897-1068`). `drdata[0]` selects
/// the object flavor: `1` = teleport door, `2` = note-giving skeleton,
/// `3` = readable note (a freshly-created note from case `2` is itself an
/// `IDR_LAB3_SPECIAL` item with `drdata[0]==3`, see `lab3_note_generic`'s
/// zone template). All the actual mutation (teleport, item creation,
/// password assignment) happens outside this pure function - see the
/// three new `Lab3*` outcome variants' own doc comments for where each
/// one resolves.
pub(crate) fn lab3_special_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    match drdata(item, 0) {
        1 => {
            let password_protected = drdata(item, 3) != 0;
            if password_protected && context.lab3_guard_talkstep.unwrap_or(0) < 20 {
                return ItemDriverOutcome::Lab3TeleportDoorLocked {
                    character_id: character.id,
                };
            }
            ItemDriverOutcome::Lab3TeleportDoor {
                item_id: item.id,
                character_id: character.id,
                dx: drdata(item, 1) as i8,
                dy: drdata(item, 2) as i8,
                password_protected,
                extinguished_count: 0,
            }
        }
        2 => {
            if character.cursor_item.is_some() {
                return ItemDriverOutcome::Lab3NoteGivingBlocked {
                    character_id: character.id,
                };
            }
            ItemDriverOutcome::Lab3NoteGivingSkeleton {
                item_id: item.id,
                character_id: character.id,
                note_value: drdata(item, 1),
            }
        }
        3 => ItemDriverOutcome::Lab3NoteRead {
            item_id: item.id,
            character_id: character.id,
            note_value: drdata(item, 1),
        },
        _ => ItemDriverOutcome::Noop,
    }
}

/// C `lab4_item` (`src/area/22/lab4.c:645-672`). `drdata[0]==1` is the
/// only branch (`gnalb_fireplace_key`'s own `arg="01"`) - C's `switch`
/// falls through to a no-op `return;` for any other `drdata[0]`, so no
/// other case exists to port.
pub(crate) fn lab4_item_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || drdata(item, 0) != 1 {
        return ItemDriverOutcome::Noop;
    }
    // C `if (ch[cn].citem) return;` (`lab4.c:657-659`).
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::Lab4FireplaceKeyBlocked {
            character_id: character.id,
        };
    }
    ItemDriverOutcome::Lab4FireplaceKeyGive {
        item_id: item.id,
        character_id: character.id,
    }
}

/// C `GUNRELOAD` (`lab5.c:1025`): `2 * TICKS / 3` with `TICKS == 24`.
const LAB5_GUNRELOAD_TICKS: u64 = TICKS_PER_SECOND * 2 / 3;

/// C `lab5_item` (`src/area/22/lab5.c:1027-1376`). `drdata[0]` selects the
/// object flavor (see the C file's own comment table, `lab5.c:947-970`).
/// Every branch that only mutates `character`/`item` (heals, drdata/sprite
/// writes) is applied directly here, matching `potion_driver`'s own
/// precedent of mutating through the `&mut Character`/`&mut Item` this
/// function already receives; branches needing `World`/`ZoneLoader`/
/// `PlayerRuntime` (sound, pulseback effects, item creation/destruction,
/// ritual-state persistence) return one of the new `Lab5*` outcomes for
/// `ugaris-server`'s `tick_item_use_lab.rs` to resolve.
///
/// `drdata[0]==2` (fireface) and `drdata[0]==13` (lightface), C's two
/// "shoot a projectile down the corridor forever" perpetual ambient
/// statues, are now also ported (`lab5_face_direction`, reusing the
/// existing `FireballMachineProjectile`/`BallTrapProjectile` outcomes -
/// same precedent as the `drdata[0]==9` gun branch below, which already
/// reuses `FireballMachineProjectile`). Both are `cn==0`-only
/// self-rescheduling timer chains with no player-visible state and no
/// `cn!=0` branch at all; getting their very first `call_item` to fire at
/// all needed closing the generic "nothing primes an always-on ambient
/// item driver's first timer call" gap for this specific driver -
/// `World::schedule_existing_light_timers` (`world/light.rs`) now also
/// matches `IDR_LAB5_ITEM` items with `drdata[0]` `2`/`13`, since both
/// flavors are always placed as static zone `.itm` data (never
/// runtime-`create_item`'d), matching every other entry already in that
/// allow-list. Like the pre-existing `drdata[0]==9` gun branch's own
/// undocumented deviation, the shared `FireballMachineProjectile`
/// resolver unconditionally emits an `NT_SPELL`/`V_FIREBALL`
/// `notify_area` that C's `lab5_item` fireface branch itself does not
/// call (only `area2.c`'s `fireball_machine` does) - left as-is for
/// consistency with that existing precedent rather than special-cased.
/// `drdata[0]==5`/`6`/`7`/`8`'s `cn==0` branches (map-setup writes to
/// `namecoordx/y`/`daemondoorx/y`) are still not ported - they are
/// superseded by `World::lab5_namecoords`' hardcoded
/// `LAB5_NAMECOORD_DEFAULTS`/`lab5.rs`'s `LAB5_DAEMON_DOORS`, already
/// used by the previously-ported ritual system.
pub(crate) fn lab5_item_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        // Timer-tick branches: fireface/lightface, chestbox close, gun
        // reload, pike reset.
        return match drdata(item, 0) {
            2 => {
                // C `lab5.c:1048-1069`.
                let (dx, dy) = lab5_face_direction(item.sprite);
                let item_x = i32::from(item.x);
                let item_y = i32::from(item.y);
                let schedule_after_ticks = if drdata(item, 1) == 0 {
                    set_drdata(item, 1, 1);
                    (((item_x + item_y) % 17 + 1) as u64) * TICKS_PER_SECOND
                } else {
                    5 * TICKS_PER_SECOND
                };
                ItemDriverOutcome::FireballMachineProjectile {
                    item_id: item.id,
                    character_id: character.id,
                    start_x: clamp_legacy_coordinate(item_x + dx),
                    start_y: clamp_legacy_coordinate(item_y + dy),
                    target_x: clamp_legacy_coordinate(item_x + 2 * dx),
                    target_y: clamp_legacy_coordinate(item_y + 2 * dy),
                    power: 50,
                    schedule_after_ticks: Some(schedule_after_ticks),
                }
            }
            13 => {
                // C `lab5.c:1072-1099`.
                let (dx, dy) = lab5_face_direction(item.sprite);
                let item_x = i32::from(item.x);
                let item_y = i32::from(item.y);
                let schedule_after_ticks = if drdata(item, 1) == 0 {
                    set_drdata(item, 1, 1);
                    (((item_x + item_y) % 10 + 1) as u64) * TICKS_PER_SECOND
                } else if drdata(item, 2) == 4 {
                    set_drdata(item, 2, 0);
                    9 * TICKS_PER_SECOND
                } else {
                    set_drdata(item, 2, drdata(item, 2) + 1);
                    7 * TICKS_PER_SECOND / 4
                };
                ItemDriverOutcome::BallTrapProjectile {
                    item_id: item.id,
                    character_id: character.id,
                    start_x: clamp_legacy_coordinate(item_x + dx),
                    start_y: clamp_legacy_coordinate(item_y + dy),
                    target_x: clamp_legacy_coordinate(item_x + 2 * dx),
                    target_y: clamp_legacy_coordinate(item_y + 2 * dy),
                    power: 40,
                    schedule_after_ticks: Some(schedule_after_ticks),
                }
            }
            3 => {
                if drdata(item, 3) == 0 {
                    ItemDriverOutcome::Noop
                } else {
                    set_drdata(item, 3, 0);
                    item.sprite -= 1;
                    ItemDriverOutcome::Lab5ChestboxClose { item_id: item.id }
                }
            }
            9 => {
                let remaining = drdata(item, 1);
                if remaining == 0 {
                    ItemDriverOutcome::Noop
                } else {
                    let remaining = remaining - 1;
                    set_drdata(item, 1, remaining);
                    item.sprite -= 1;
                    ItemDriverOutcome::Lab5GunReloadTick {
                        item_id: item.id,
                        schedule_after_ticks: (remaining != 0).then_some(LAB5_GUNRELOAD_TICKS),
                    }
                }
            }
            10 => {
                if drdata(item, 1) == 0 {
                    ItemDriverOutcome::Noop
                } else {
                    set_drdata(item, 1, 0);
                    item.sprite -= 1;
                    ItemDriverOutcome::Lab5PikeReset { item_id: item.id }
                }
            }
            _ => ItemDriverOutcome::Noop,
        };
    }

    match drdata(item, 0) {
        1 => {
            // C `lab5.c:1148-1154`: full heal, sound resolved by the
            // caller.
            character.hp = max_value(character, CharacterValue::Hp) * POWERSCALE;
            character.mana = max_value(character, CharacterValue::Mana) * POWERSCALE;
            character.endurance = max_value(character, CharacterValue::Endurance) * POWERSCALE;
            character.lifeshield = lab5_lifeshield_max(character) * POWERSCALE;
            ItemDriverOutcome::Lab5Obelisk {
                character_id: character.id,
            }
        }
        3 => {
            // C `lab5.c:1157-1219`.
            if drdata(item, 3) != 0 || character.cursor_item.is_some() {
                return ItemDriverOutcome::Noop;
            }
            if context.lab5_chestbox_already_opened {
                return ItemDriverOutcome::Lab5ChestboxAlreadyOpened {
                    character_id: character.id,
                };
            }
            set_drdata(item, 3, 1);
            item.sprite += 1;
            ItemDriverOutcome::Lab5ChestboxOpen {
                item_id: item.id,
                character_id: character.id,
                reward: drdata(item, 1),
            }
        }
        4 => {
            // C `lab5.c:1222-1232`: combopotion, full heal + destroy.
            character.hp = max_value(character, CharacterValue::Hp) * POWERSCALE;
            character.mana = max_value(character, CharacterValue::Mana) * POWERSCALE;
            character.endurance = max_value(character, CharacterValue::Endurance) * POWERSCALE;
            if character_value_present(character, CharacterValue::MagicShield) != 0 {
                character.lifeshield = lab5_lifeshield_max(character) * POWERSCALE;
            }
            ItemDriverOutcome::Lab5PotionDrunk {
                item_id: item.id,
                character_id: character.id,
            }
        }
        5 => {
            // C `lab5.c:1248-1265`.
            let ritualstate = context.lab5_ritual_state.unwrap_or(0);
            if ritualstate == 0 {
                ItemDriverOutcome::Lab5RitualStart {
                    character_id: character.id,
                    daemon: drdata(item, 1),
                }
            } else {
                ItemDriverOutcome::Lab5RitualHurtAtItem {
                    item_id: item.id,
                    character_id: character.id,
                    stored_daemon: context.lab5_ritual_daemon.unwrap_or(0),
                }
            }
        }
        6 => {
            // C `lab5.c:1268-1289`.
            let ritualstate = context.lab5_ritual_state.unwrap_or(0);
            let ritualdaemon = context.lab5_ritual_daemon.unwrap_or(0);
            if ritualstate == 0 {
                ItemDriverOutcome::Lab5RitualNothing {
                    character_id: character.id,
                }
            } else if ritualstate == 1 && ritualdaemon == drdata(item, 1) {
                ItemDriverOutcome::Lab5RitualProgress {
                    character_id: character.id,
                    daemon: drdata(item, 1),
                    new_state: 2,
                }
            } else {
                ItemDriverOutcome::Lab5RitualHurtAtItem {
                    item_id: item.id,
                    character_id: character.id,
                    stored_daemon: ritualdaemon,
                }
            }
        }
        7 => {
            // C `lab5.c:1292-1319`.
            let ritualstate = context.lab5_ritual_state.unwrap_or(0);
            let ritualdaemon = context.lab5_ritual_daemon.unwrap_or(0);
            if ritualstate == 0 {
                return ItemDriverOutcome::Noop;
            }
            let entrance_index = drdata(item, 1);
            if ritualstate == 2 && ritualdaemon == entrance_index {
                ItemDriverOutcome::Lab5RitualProgress {
                    character_id: character.id,
                    daemon: entrance_index,
                    new_state: 3,
                }
            } else {
                ItemDriverOutcome::Lab5EntranceRitualHurt {
                    character_id: character.id,
                    entrance_index,
                    stored_daemon: ritualdaemon,
                    forced_message: entrance_index == 2,
                }
            }
        }
        8 => ItemDriverOutcome::Lab5Backdoor {
            character_id: character.id,
        },
        9 => {
            // C `lab5.c:1336-1347`.
            if drdata(item, 1) != 0 {
                return ItemDriverOutcome::Lab5GunLocked {
                    character_id: character.id,
                };
            }
            set_drdata(item, 1, 7);
            item.sprite += 7;
            ItemDriverOutcome::FireballMachineProjectile {
                item_id: item.id,
                character_id: character.id,
                start_x: item.x.saturating_add(2),
                start_y: item.y,
                target_x: item.x.saturating_add(60),
                target_y: item.y,
                power: 100,
                schedule_after_ticks: Some(LAB5_GUNRELOAD_TICKS),
            }
        }
        10 => {
            // C `lab5.c:1350-1359`.
            let arming = drdata(item, 1) == 0;
            if arming {
                set_drdata(item, 1, 1);
                item.sprite += 1;
            }
            ItemDriverOutcome::Lab5PikeHurt {
                item_id: item.id,
                character_id: character.id,
                arming,
            }
        }
        11 => {
            // C `lab5.c:1362-1374`.
            let approaching_from_west = character.x < item.x;
            if approaching_from_west && context.has_potion {
                return ItemDriverOutcome::Lab5NoPotionDoorBlocked {
                    character_id: character.id,
                };
            }
            let (target_x, target_y) = if approaching_from_west {
                (item.x.saturating_sub(9), item.y.saturating_sub(7))
            } else {
                (item.x.saturating_add(9), item.y.saturating_add(7))
            };
            ItemDriverOutcome::Lab5NoPotionDoorPass {
                character_id: character.id,
                target_x,
                target_y,
            }
        }
        12 => {
            // C `lab5.c:1235-1245`: manapotion, mana-only heal + destroy.
            character.mana = max_value(character, CharacterValue::Mana) * POWERSCALE;
            if character_value_present(character, CharacterValue::MagicShield) != 0 {
                character.lifeshield = lab5_lifeshield_max(character) * POWERSCALE;
            }
            ItemDriverOutcome::Lab5PotionDrunk {
                item_id: item.id,
                character_id: character.id,
            }
        }
        _ => ItemDriverOutcome::Noop,
    }
}

/// C `lab5_item`'s shared fireface/lightface sprite-to-direction table
/// (`lab5.c:1049-1061`, `1074-1086`): the four wall-mounted statue
/// sprites face right/up/left/down respectively. C's final `else`
/// branch assumes `sprite==11138` (down) without an explicit check;
/// reproduced identically here.
fn lab5_face_direction(sprite: i32) -> (i32, i32) {
    match sprite {
        11135 => (1, 0),
        11136 => (0, -1),
        11137 => (-1, 0),
        _ => (0, 1),
    }
}

/// C `get_lifeshield_max` (`src/system/tool.c:3880-3885`).
fn lab5_lifeshield_max(character: &Character) -> i32 {
    let magicshield = max_value(character, CharacterValue::MagicShield);
    if magicshield != 0 {
        magicshield
    } else {
        max_value(character, CharacterValue::Warcry)
    }
}

pub(crate) fn legacy_lab_destination(lab_level: u8) -> Option<(u16, u16, u16, u16)> {
    match lab_level {
        10 => Some((10, 22, 27, 242)),
        15 => Some((12, 22, 69, 105)),
        20 => Some((15, 22, 227, 250)),
        25 => Some((20, 22, 144, 103)),
        30 => Some((25, 22, 163, 243)),
        _ => None,
    }
}

pub(crate) fn labentrance_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    for lab_level in 0..64_u8 {
        let bit = 1_u64 << lab_level;
        if context.lab_solved_bits & bit != 0 {
            continue;
        }
        let Some((required_level, area_id, x, y)) = legacy_lab_destination(lab_level) else {
            continue;
        };
        if character.level < u32::from(required_level) {
            return ItemDriverOutcome::LabEntranceTooLow {
                item_id: item.id,
                character_id: character.id,
                required_level,
            };
        }
        return ItemDriverOutcome::Teleport {
            item_id: item.id,
            character_id: character.id,
            x,
            y,
            area_id,
            stop_driver: true,
            quiet: false,
        };
    }

    ItemDriverOutcome::LabEntranceSolvedAll {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn labexit_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call && character.id.0 == 0 {
        let frame = drdata_u32(item, 8);
        if frame < 24 {
            item.sprite = 1060 + (frame % 24) as i32;
        } else if frame < 240 {
            item.sprite = 1060 + (frame % 24) as i32 + 24;
        } else if frame < 240 + 24 {
            item.sprite = 1060 + (frame % 24) as i32 + 48;
        } else {
            return ItemDriverOutcome::LabExitExpired { item_id: item.id };
        }

        let next_frame = frame.saturating_add(1);
        set_drdata_u32(item, 8, next_frame);
        return ItemDriverOutcome::LabExitAnimating {
            item_id: item.id,
            sprite: item.sprite,
            frame: next_frame,
            schedule_after_ticks: 2,
        };
    }

    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let owner_id = drdata_u32(item, 0);
    if character.id.0 != owner_id {
        return ItemDriverOutcome::LabExitWrongOwner {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let frame = drdata_u32(item, 8);
    let close_frame = 240 - 24 + (frame % 24);
    set_drdata_u32(item, 8, close_frame);

    ItemDriverOutcome::LabExitUse {
        item_id: item.id,
        character_id: character.id,
        lab_nr: drdata(item, 4),
        frame: close_frame,
        target_area: 3,
        target_x: 183,
        target_y: 199,
    }
}
