use super::*;

pub const CLANSPAWN_DEFAULT_FREQ_HOURS: u8 = 48;

pub const CLANSPAWN_CHECK_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 60;

pub const CLANSPAWN_TIME_GRANULARITY_SECONDS: u32 = 30 * 60;

pub const CLANSPAWN_DEFAULT_MAX_JEWELS: u8 = 2;

pub const CLANJEWEL_CHECK_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 30;

pub const CLANJEWEL_LIFETIME_SECONDS: u32 = 60 * 60;

pub(crate) fn clanspawn_exit_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::ClanSpawnExit {
        item_id: item.id,
        character_id: character.id,
        area_id: character.rest_area,
        x: character.rest_x,
        y: character.rest_y,
    }
}

pub(crate) fn clanspawn_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let freq_hours = match drdata(item, 1) {
        0 => CLANSPAWN_DEFAULT_FREQ_HOURS,
        freq => freq,
    };
    let max_jewel_count = context
        .clanspawn_max_jewel_count
        .unwrap_or(CLANSPAWN_DEFAULT_MAX_JEWELS);

    if context.timer_call && character.id.0 == 0 {
        let current_seconds = context.current_tick / TICKS_PER_SECOND as u32;
        let freq_seconds = u32::from(freq_hours) * 60 * 60;
        let mut next_spawn_seconds = drdata_u32(item, 4);
        if next_spawn_seconds == 0 {
            let random = context
                .clanspawn_random_seconds
                .unwrap_or(0)
                .min(freq_seconds.saturating_div(2).saturating_sub(1));
            next_spawn_seconds = round_down_to_granularity(
                current_seconds + random + freq_seconds / 4,
                CLANSPAWN_TIME_GRANULARITY_SECONDS,
            );
            set_drdata_u32(item, 4, next_spawn_seconds);
            item.max_level = drdata(item, 0);
        }

        let mut jewel_count = drdata(item, 2);
        let mut spawned = false;
        if current_seconds >= next_spawn_seconds && jewel_count <= max_jewel_count {
            if jewel_count == 0 {
                item.sprite += 1;
            }
            jewel_count = jewel_count.saturating_add(1);
            set_drdata(item, 2, jewel_count);
            let random = context
                .clanspawn_random_seconds
                .unwrap_or(0)
                .min(freq_seconds.saturating_sub(1));
            next_spawn_seconds = round_down_to_granularity(
                current_seconds + random + freq_seconds / 2,
                CLANSPAWN_TIME_GRANULARITY_SECONDS,
            );
            set_drdata_u32(item, 4, next_spawn_seconds);
            spawned = true;
        }

        return ItemDriverOutcome::ClanSpawnTimer {
            item_id: item.id,
            spawned,
            jewel_count,
            next_spawn_seconds,
            schedule_after_ticks: CLANSPAWN_CHECK_INTERVAL_TICKS,
        };
    }

    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    if character.level > u32::from(drdata(item, 0)) {
        return ItemDriverOutcome::ClanSpawnLevelTooHigh {
            item_id: item.id,
            character_id: character.id,
        };
    }

    if context.clanspawn_contested {
        return ItemDriverOutcome::ClanSpawnContested {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let current_seconds = context.current_tick / TICKS_PER_SECOND as u32;
    let next_spawn_seconds = drdata_u32(item, 4);
    let mut jewel_count = drdata(item, 2);
    if jewel_count == 0 {
        let god_added = character.flags.contains(CharacterFlags::GOD);
        if god_added {
            item.sprite += 1;
            jewel_count = 1;
            set_drdata(item, 2, jewel_count);
        }
        let remaining_minutes = next_spawn_seconds.saturating_sub(current_seconds) / 60;
        return ItemDriverOutcome::ClanSpawnCountdown {
            item_id: item.id,
            character_id: character.id,
            remaining_minutes,
            freq_hours,
            god_added,
        };
    }

    jewel_count = jewel_count.saturating_sub(1);
    set_drdata(item, 2, jewel_count);
    if jewel_count == 0 {
        item.sprite -= 1;
    }
    ItemDriverOutcome::ClanSpawnAward {
        item_id: item.id,
        character_id: character.id,
        level: item.max_level,
        remaining_jewels: jewel_count,
    }
}

pub(crate) fn clanjewel_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call || character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    let current_seconds = context.current_tick / TICKS_PER_SECOND as u32;
    let mut creation_time = drdata_u32(item, 0);
    if creation_time == 0 {
        creation_time = current_seconds;
        set_drdata_u32(item, 0, creation_time);
    }

    if current_seconds > creation_time.saturating_add(CLANJEWEL_LIFETIME_SECONDS) {
        return ItemDriverOutcome::ClanJewelExpired {
            item_id: item.id,
            character_id: item.carried_by,
            item_name: outcome_item_name(&item.name),
        };
    }

    ItemDriverOutcome::ClanJewelRescheduled {
        item_id: item.id,
        schedule_after_ticks: CLANJEWEL_CHECK_INTERVAL_TICKS,
    }
}
