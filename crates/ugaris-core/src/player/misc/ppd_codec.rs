// The PPD byte-offset constants and codecs in this module mirror the C
// `struct *_ppd` layouts verbatim as `<field index> * 4` products (so
// `0 * 4`, `1 * 4`, ... line up visually with the C struct order); keep
// clippy from "simplifying" the intentional identity/zero terms.
#![allow(clippy::identity_op, clippy::erasing_op)]

use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_orbspawn_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_ORBSPAWN_PPD_SIZE];
        for (index, entry) in self
            .orb_spawns
            .iter()
            .take(ORBSPAWN_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                ORBSPAWN_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                ORBSPAWN_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_orbspawn_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_ORBSPAWN_PPD_SIZE {
            return false;
        }

        self.orb_spawns.clear();
        for index in 0..ORBSPAWN_MAX_ENTRIES {
            let location_id = read_i32(bytes, ORBSPAWN_PPD_IDS_OFFSET + index * 4);
            let last_used_seconds = read_i32(bytes, ORBSPAWN_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 && last_used_seconds > 0 {
                self.orb_spawns.push(OrbSpawnAccess {
                    location_id: location_id as u32,
                    last_used_seconds: last_used_seconds as u64,
                });
            }
        }
        true
    }

    pub fn encode_legacy_misc_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_MISC_PPD_SIZE];
        let copy_len = self.misc_ppd.len().min(LEGACY_MISC_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.misc_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_misc_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_MISC_PPD_SIZE {
            return false;
        }

        self.misc_ppd = bytes[..LEGACY_MISC_PPD_SIZE].to_vec();
        true
    }

    pub fn encode_legacy_firstkill_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_FIRSTKILL_PPD_SIZE];
        let copy_len = self.first_kill_ppd.len().min(LEGACY_FIRSTKILL_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.first_kill_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_firstkill_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_FIRSTKILL_PPD_SIZE {
            return false;
        }
        self.first_kill_ppd = bytes[..LEGACY_FIRSTKILL_PPD_SIZE].to_vec();
        true
    }

    /// C `give_first_kill`'s bit-test/set (`death.c:196-222`): `index =
    /// ch[co].class / 32; offset = ch[co].class & 31; mask = 1 << offset;
    /// if (ppd->kill[index] & mask) return; ppd->kill[index] |= mask;` -
    /// reworked here as a flat byte/bit-in-byte pair (`class / 8`, `class %
    /// 8`), which addresses the exact same bit in a little-endian `u32[32]`
    /// laid out as 128 raw bytes. Returns `true` the first time `class` is
    /// killed (and records it), `false` on every repeat.
    pub fn mark_first_kill(&mut self, class: i32) -> bool {
        if !(1..=1023).contains(&class) {
            return false;
        }
        if self.first_kill_ppd.len() < LEGACY_FIRSTKILL_PPD_SIZE {
            self.first_kill_ppd.resize(LEGACY_FIRSTKILL_PPD_SIZE, 0);
        }
        let class = class as usize;
        let byte = class / 8;
        let bit = 1u8 << (class % 8);
        if self.first_kill_ppd[byte] & bit != 0 {
            return false;
        }
        self.first_kill_ppd[byte] |= bit;
        true
    }

    /// C `ppd->kill[index] & mask` bit-test (`death.c:196-197`, also
    /// inlined directly in `command.c:1193/1200` by `/pentinfo`'s sibling
    /// `cmd_demonlords`), exposed as a query so callers other than
    /// [`Self::mark_first_kill`] (which also sets the bit) can check
    /// without mutating. Out-of-range classes (matching `mark_first_kill`'s
    /// own guard) are always reported unkilled.
    pub fn has_first_kill(&self, class: i32) -> bool {
        if !(1..=1023).contains(&class) || self.first_kill_ppd.is_empty() {
            return false;
        }
        let class = class as usize;
        let byte = class / 8;
        byte < self.first_kill_ppd.len() && self.first_kill_ppd[byte] & (1 << (class % 8)) != 0
    }

    /// C `count_demon_lord_kills` (`death.c:169-190`): counts unique
    /// first-killed classes in `258..=305` (Earth/Fire/Ice demon lords) and
    /// `404..=411` (Hell demon lords).
    pub fn count_demon_lord_kills(&self) -> u32 {
        if self.first_kill_ppd.is_empty() {
            return 0;
        }
        (258..=305).filter(|&c| self.has_first_kill(c)).count() as u32
            + (404..=411).filter(|&c| self.has_first_kill(c)).count() as u32
    }

    pub fn treasure_dig_last_seconds(&self, dig_index: u8) -> u64 {
        self.treasure_dig_last_seconds
            .get(usize::from(dig_index))
            .copied()
            .unwrap_or_default()
    }

    pub fn mark_treasure_dig(&mut self, dig_index: u8, realtime_seconds: u64) -> bool {
        let Some(last_dig) = self
            .treasure_dig_last_seconds
            .get_mut(usize::from(dig_index))
        else {
            return false;
        };
        *last_dig = realtime_seconds;
        true
    }

    pub fn encode_legacy_treasure_dig_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_TREASURE_DIG_PPD_SIZE];
        for (index, last_dig_seconds) in self.treasure_dig_last_seconds.iter().copied().enumerate()
        {
            write_i32(
                &mut bytes,
                index * 4,
                last_dig_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_treasure_dig_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TREASURE_DIG_PPD_SIZE {
            return false;
        }
        for index in 0..TREASURE_DIG_PPD_ENTRIES {
            self.treasure_dig_last_seconds[index] = read_i32(bytes, index * 4).max(0) as u64;
        }
        true
    }

    pub fn flower_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.flowers
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_flower_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .flowers
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }

        if self.flowers.len() < FLOWER_MAX_ENTRIES {
            self.flowers.push(FlowerAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }

        if let Some(oldest) = self
            .flowers
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = FlowerAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
    }

    pub fn encode_legacy_flower_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_FLOWER_PPD_SIZE];
        for (index, entry) in self.flowers.iter().take(FLOWER_MAX_ENTRIES).enumerate() {
            write_i32(
                &mut bytes,
                FLOWER_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                FLOWER_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_flower_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_FLOWER_PPD_SIZE {
            return false;
        }
        self.flowers.clear();
        for index in 0..FLOWER_MAX_ENTRIES {
            let location_id = read_i32(bytes, FLOWER_PPD_IDS_OFFSET + index * 4);
            let last_used = read_i32(bytes, FLOWER_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 || last_used > 0 {
                self.flowers.push(FlowerAccess {
                    location_id: location_id.max(0) as u32,
                    last_used_seconds: last_used.max(0) as u64,
                });
            }
        }
        true
    }

    pub fn encode_legacy_stats_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_STATS_PPD_SIZE];
        let len = self.stats_ppd.len().min(LEGACY_STATS_PPD_SIZE);
        bytes[..len].copy_from_slice(&self.stats_ppd[..len]);
        bytes
    }

    pub fn decode_legacy_stats_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_STATS_PPD_SIZE {
            return false;
        }
        self.stats_ppd = bytes[..LEGACY_STATS_PPD_SIZE].to_vec();
        true
    }

    /// C `stats_update` (`src/system/statistics.c:23-45`): called once per
    /// real-time minute per connected player (`player_update`, `player.c:
    /// 3460`, `stats_update(cn, 1, 0)`, ported at `award_play_time_minute`'s
    /// call site in `ugaris-server`'s `main.rs`) plus on every store sale
    /// and money-item destruction (`store.c:381`/`do.c:1282`,
    /// `stats_update(cn, 0, price)` - not yet wired, see `PORTING_TODO.md`'s
    /// "Cross-area transfer" task's Progress Log: `.gold`/`.exp` are
    /// write-only fields nothing in this codebase reads yet, unlike
    /// `.online`, which `stats_online_time` sums). Maintains a
    /// `STATS_PPD_MAXSTAT`(365)-day rolling ring buffer of daily
    /// exp/gold/online samples, zeroing every day bucket skipped since the
    /// last update (a player who was offline for more than 365 days wraps
    /// all the way around, clearing the whole buffer - matching C's own
    /// `while (lidx != idx) { lidx = (lidx+1) % MAXSTAT; bzero(...); }`
    /// loop exactly, run against `self.stats_ppd`'s raw legacy bytes
    /// in-place rather than a decoded struct). `now`/`last_update` are
    /// wall-clock unix seconds (the caller's `current_unix_time()`);
    /// `STATS_PPD_STARTTIME` is subtracted first to match C's own
    /// `realtime = time_now - STARTTIME` day-bucketing exactly. Lazily
    /// zero-initializes `self.stats_ppd` on first use, mirroring C's
    /// `set_data` zero-allocating a fresh `stats_ppd` the first time any
    /// character calls this.
    pub fn stats_update(&mut self, character_exp: i32, online_minutes: i32, gold: i32, now: i64) {
        if self.stats_ppd.len() < LEGACY_STATS_PPD_SIZE {
            self.stats_ppd = vec![0; LEGACY_STATS_PPD_SIZE];
        }
        let real_now = now - STATS_PPD_STARTTIME;
        let idx = real_now
            .div_euclid(STATS_PPD_RESOLUTION_SECONDS)
            .rem_euclid(STATS_PPD_MAXSTAT as i64) as usize;
        let last_update = i64::from(read_i32(&self.stats_ppd, STATS_PPD_LAST_UPDATE_OFFSET));
        let mut lidx = last_update
            .div_euclid(STATS_PPD_RESOLUTION_SECONDS)
            .rem_euclid(STATS_PPD_MAXSTAT as i64) as usize;
        while lidx != idx {
            lidx = (lidx + 1) % STATS_PPD_MAXSTAT;
            let offset = stats_ppd_day_offset(lidx);
            write_i32(&mut self.stats_ppd, offset + STATS_PPD_DAY_EXP_OFFSET, 0);
            write_i32(&mut self.stats_ppd, offset + STATS_PPD_DAY_GOLD_OFFSET, 0);
            write_i32(&mut self.stats_ppd, offset + STATS_PPD_DAY_ONLINE_OFFSET, 0);
        }
        write_i32(
            &mut self.stats_ppd,
            STATS_PPD_LAST_UPDATE_OFFSET,
            real_now as i32,
        );
        let offset = stats_ppd_day_offset(idx);
        write_i32(
            &mut self.stats_ppd,
            offset + STATS_PPD_DAY_EXP_OFFSET,
            character_exp,
        );
        let gold_total =
            read_i32(&self.stats_ppd, offset + STATS_PPD_DAY_GOLD_OFFSET).saturating_add(gold);
        write_i32(
            &mut self.stats_ppd,
            offset + STATS_PPD_DAY_GOLD_OFFSET,
            gold_total,
        );
        let online_total = read_i32(&self.stats_ppd, offset + STATS_PPD_DAY_ONLINE_OFFSET)
            .saturating_add(online_minutes);
        write_i32(
            &mut self.stats_ppd,
            offset + STATS_PPD_DAY_ONLINE_OFFSET,
            online_total,
        );
    }

    /// C `stats_online_time` (`src/system/statistics.c:47-58`): sums every
    /// day bucket's `.online` sample across the whole 365-day ring buffer
    /// (`/values`' "Playing for %d hours." line, `tool.c:2917`, divides
    /// this by 60). Returns `0` for a character with no `stats_ppd` yet
    /// (mirrors C's `if (!ppd) return 0;`).
    pub fn stats_online_time(&self) -> i32 {
        if self.stats_ppd.len() < LEGACY_STATS_PPD_SIZE {
            return 0;
        }
        (0..STATS_PPD_MAXSTAT)
            .map(|day| {
                read_i32(
                    &self.stats_ppd,
                    stats_ppd_day_offset(day) + STATS_PPD_DAY_ONLINE_OFFSET,
                )
            })
            .sum()
    }

    pub fn encode_legacy_bank_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_BANK_PPD_SIZE];
        write_i32(
            &mut bytes,
            BANK_PPD_IMPERIAL_GOLD_OFFSET,
            self.bank_gold.min(i32::MAX as u32) as i32,
        );
        bytes
    }

    pub fn decode_legacy_bank_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_BANK_PPD_SIZE {
            return false;
        }
        self.bank_gold = read_i32(bytes, BANK_PPD_IMPERIAL_GOLD_OFFSET).max(0) as u32;
        true
    }

    /// Encodes one `struct item` (`src/system/server.h`) exactly like
    /// `ugaris-server::depot`'s `legacy_account_depot_codec` - both persist
    /// the same C struct, just for a different `DRD_*` id. Kept as an
    /// independent copy (crate-boundary duplication, not code reuse) since
    /// `ugaris-server` depends on `ugaris-core`, not the other way around.
    pub(crate) fn encode_legacy_depot_item(item: &Item) -> [u8; DEPOT_PPD_ITEM_SIZE] {
        let mut bytes = [0u8; DEPOT_PPD_ITEM_SIZE];
        write_u64(&mut bytes, DEPOT_PPD_ITEM_FLAGS_OFFSET, item.flags.bits());
        write_c_string(
            &mut bytes,
            DEPOT_PPD_ITEM_NAME_OFFSET,
            DEPOT_PPD_ITEM_NAME_LEN,
            &item.name,
        );
        write_c_string(
            &mut bytes,
            DEPOT_PPD_ITEM_DESCRIPTION_OFFSET,
            DEPOT_PPD_ITEM_DESCRIPTION_LEN,
            &item.description,
        );
        write_u32(&mut bytes, DEPOT_PPD_ITEM_VALUE_OFFSET, item.value);
        bytes[DEPOT_PPD_ITEM_MIN_LEVEL_OFFSET] = item.min_level;
        bytes[DEPOT_PPD_ITEM_MAX_LEVEL_OFFSET] = item.max_level;
        bytes[DEPOT_PPD_ITEM_NEEDS_CLASS_OFFSET] = item.needs_class;
        write_i32(&mut bytes, DEPOT_PPD_ITEM_OWNER_OFFSET, item.owner_id);
        for index in 0..MAX_MODIFIERS {
            let offset = DEPOT_PPD_ITEM_MOD_INDEX_OFFSET + index * 2;
            bytes[offset..offset + 2].copy_from_slice(&item.modifier_index[index].to_le_bytes());
            let offset = DEPOT_PPD_ITEM_MOD_VALUE_OFFSET + index * 2;
            bytes[offset..offset + 2].copy_from_slice(&item.modifier_value[index].to_le_bytes());
        }
        write_u16(&mut bytes, DEPOT_PPD_ITEM_CONTENT_OFFSET, item.content_id);
        write_u16(&mut bytes, DEPOT_PPD_ITEM_DRIVER_OFFSET, item.driver);
        let drdata_len = item.driver_data.len().min(DEPOT_PPD_ITEM_DRDATA_LEN);
        bytes[DEPOT_PPD_ITEM_DRDATA_OFFSET..DEPOT_PPD_ITEM_DRDATA_OFFSET + drdata_len]
            .copy_from_slice(&item.driver_data[..drdata_len]);
        write_u32(
            &mut bytes,
            DEPOT_PPD_ITEM_TEMPLATE_ID_OFFSET,
            item.template_id,
        );
        write_u32(&mut bytes, DEPOT_PPD_ITEM_SERIAL_OFFSET, item.serial);
        write_i32(&mut bytes, DEPOT_PPD_ITEM_SPRITE_OFFSET, item.sprite);
        bytes
    }

    /// Decodes one `struct item` slot; returns `None` for an empty slot
    /// (`flags == 0`, matching C's `if (ppd->itm[nr].flags)` emptiness
    /// check throughout `depot.c`) rather than `Some` with zeroed fields.
    pub(crate) fn decode_legacy_depot_item(bytes: &[u8], slot: usize) -> Option<Item> {
        if bytes.len() < DEPOT_PPD_ITEM_PERSISTED_PREFIX {
            return None;
        }
        let flags = read_u64(bytes, DEPOT_PPD_ITEM_FLAGS_OFFSET);
        if flags == 0 {
            return None;
        }
        let read_i16 = |offset: usize| i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let mut modifier_index = [0i16; MAX_MODIFIERS];
        let mut modifier_value = [0i16; MAX_MODIFIERS];
        for index in 0..MAX_MODIFIERS {
            modifier_index[index] = read_i16(DEPOT_PPD_ITEM_MOD_INDEX_OFFSET + index * 2);
            modifier_value[index] = read_i16(DEPOT_PPD_ITEM_MOD_VALUE_OFFSET + index * 2);
        }
        Some(Item {
            id: ItemId((slot + 1) as u32),
            name: read_c_string(bytes, DEPOT_PPD_ITEM_NAME_OFFSET, DEPOT_PPD_ITEM_NAME_LEN),
            description: read_c_string(
                bytes,
                DEPOT_PPD_ITEM_DESCRIPTION_OFFSET,
                DEPOT_PPD_ITEM_DESCRIPTION_LEN,
            ),
            flags: ItemFlags::from_bits_retain(flags),
            sprite: read_i32(bytes, DEPOT_PPD_ITEM_SPRITE_OFFSET),
            value: read_u32(bytes, DEPOT_PPD_ITEM_VALUE_OFFSET),
            min_level: bytes[DEPOT_PPD_ITEM_MIN_LEVEL_OFFSET],
            max_level: bytes[DEPOT_PPD_ITEM_MAX_LEVEL_OFFSET],
            needs_class: bytes[DEPOT_PPD_ITEM_NEEDS_CLASS_OFFSET],
            template_id: read_u32(bytes, DEPOT_PPD_ITEM_TEMPLATE_ID_OFFSET),
            owner_id: read_i32(bytes, DEPOT_PPD_ITEM_OWNER_OFFSET),
            modifier_index,
            modifier_value,
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: read_u16(bytes, DEPOT_PPD_ITEM_CONTENT_OFFSET),
            driver: read_u16(bytes, DEPOT_PPD_ITEM_DRIVER_OFFSET),
            driver_data: bytes[DEPOT_PPD_ITEM_DRDATA_OFFSET
                ..DEPOT_PPD_ITEM_DRDATA_OFFSET + DEPOT_PPD_ITEM_DRDATA_LEN]
                .to_vec(),
            serial: read_u32(bytes, DEPOT_PPD_ITEM_SERIAL_OFFSET),
        })
    }

    /// C `struct depot_ppd { struct item itm[MAXDEPOT]; }`: always encodes
    /// all `MAXDEPOT` fixed-size item records (unlike
    /// `ugaris-server::depot`'s `AccountDepotState`, which compacts empty
    /// slots out of its own variable-length subscriber-blob block), so a
    /// slot's index is preserved exactly across save/load.
    pub fn encode_legacy_depot_ppd(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(MAXDEPOT * DEPOT_PPD_ITEM_SIZE);
        for slot in 0..MAXDEPOT {
            match self.depot.get(slot).and_then(Option::as_ref) {
                Some(item) => bytes.extend_from_slice(&Self::encode_legacy_depot_item(item)),
                None => bytes.extend(std::iter::repeat_n(0u8, DEPOT_PPD_ITEM_SIZE)),
            }
        }
        bytes
    }

    pub fn decode_legacy_depot_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < MAXDEPOT * DEPOT_PPD_ITEM_SIZE {
            return false;
        }
        let mut depot = Self::default_depot();
        for (slot, chunk) in bytes
            .chunks_exact(DEPOT_PPD_ITEM_SIZE)
            .take(MAXDEPOT)
            .enumerate()
        {
            depot[slot] = Self::decode_legacy_depot_item(chunk, slot);
        }
        self.depot = depot;
        true
    }

    pub fn touch_xmas_tree(
        &mut self,
        area_id: u16,
        event_year: i32,
        is_xmas: bool,
        has_holiday_treat: bool,
    ) -> XmasTreeResult {
        if !is_xmas {
            return XmasTreeResult::Dormant;
        }
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            self.misc_ppd.resize(LEGACY_MISC_PPD_SIZE, 0);
        }
        if read_i32(&self.misc_ppd, MISC_PPD_GIFT_YEAR_OFFSET) != event_year {
            for byte in &mut self.misc_ppd[MISC_PPD_TREEDONE_OFFSET..MISC_PPD_TREEDONE_OFFSET + 8] {
                *byte = 0;
            }
            write_i32(&mut self.misc_ppd, MISC_PPD_GIFT_YEAR_OFFSET, event_year);
        }

        let idx = usize::from(area_id / 8);
        let bit = 1u8 << (area_id % 8);
        if idx >= 8 || self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] & bit != 0 {
            return XmasTreeResult::AlreadyGranted;
        }
        if !has_holiday_treat {
            return XmasTreeResult::NeedsHolidayTreat;
        }

        self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] |= bit;
        XmasTreeResult::GiftGranted
    }

    pub fn unmark_xmas_tree(&mut self, area_id: u16) {
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            return;
        }
        let idx = usize::from(area_id / 8);
        if idx < 8 {
            self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] &= !(1u8 << (area_id % 8));
        }
    }

    pub fn xmas_tree_marked(&self, area_id: u16) -> bool {
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            return false;
        }
        let idx = usize::from(area_id / 8);
        idx < 8 && self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] & (1u8 << (area_id % 8)) != 0
    }

    pub fn orb_spawn_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.orb_spawns
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_orb_spawn_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .orb_spawns
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }
        if self.orb_spawns.len() < ORBSPAWN_MAX_ENTRIES {
            self.orb_spawns.push(OrbSpawnAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }
        if let Some(oldest) = self
            .orb_spawns
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = OrbSpawnAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
    }
}
